//! Cash-or-nothing binaries: `e^(-r tau) N(d2)` before expiry, $1 or $0 at
//! expiry, resolved on the closing price (not an intraday touch).
//!
//! "Above" YES pays when the price closes above the strike; "range" YES pays
//! when `lower <= price < upper`. NO pays the opposite.

use crate::fees::BinaryFees;
use crate::instrument::Cell;
use crate::iv::digital_iv;
use crate::math::{exp, ln, norm_cdf, norm_pdf, sqrt};
use crate::option::Greeks;
use crate::presets::{self, Venue};
use crate::thesis::{DayCount, Thesis};
use crate::{err, Date, Liquidity, Res};
use serde::{Deserialize, Serialize};

/// YES or NO.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// Pays $1 when the event happens.
    #[default]
    Yes,
    /// Pays $1 when it does not.
    No,
}

/// Contract shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Contract {
    /// Closes above a strike.
    #[default]
    Above,
    /// Closes inside `[lower, upper)`.
    Range,
}

/// Binary inputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BinarySpec {
    /// Column label; the kind when omitted.
    #[serde(default)]
    pub label: Option<String>,
    /// Whether the column is shown.
    #[serde(default = "crate::instrument::yes")]
    pub enabled: bool,
    /// Size locked by hand; overrides the comparison mode.
    #[serde(default)]
    pub qty: Option<f64>,
    /// Venue preset for the fee model.
    #[serde(default)]
    pub venue: Venue,
    /// Side bought.
    #[serde(default)]
    pub side: Outcome,
    /// Above or range.
    #[serde(default)]
    pub contract: Contract,
    /// Strike of an "above" contract.
    #[serde(default)]
    pub strike: Option<f64>,
    /// Lower bound of a range contract.
    #[serde(default)]
    pub lower: Option<f64>,
    /// Upper bound of a range contract.
    #[serde(default)]
    pub upper: Option<f64>,
    /// Expiry (resolution) date.
    pub expiry: Date,
    /// Price paid per contract for the side bought, in dollars.
    pub price: f64,
    /// Fee model; the venue preset's when omitted.
    #[serde(default)]
    pub fees: Option<BinaryFees>,
    /// Taker or maker.
    #[serde(default)]
    pub liquidity: Liquidity,
    /// Volatility for repricing; implied from `price` when omitted.
    #[serde(default)]
    pub iv: Option<f64>,
}

/// A resolved binary, per contract.
#[derive(Clone, Debug, PartialEq)]
pub struct Binary {
    side: Outcome,
    /// Strikes: `[strike]` for above, `[lower, upper]` for range.
    strikes: Vec<f64>,
    /// Expiry date.
    pub expiry: Date,
    price: f64,
    fees: BinaryFees,
    liquidity: Liquidity,
    /// Volatility used for repricing.
    pub iv: f64,
    rate: f64,
    s0: f64,
    tau0: f64,
    day: DayCount,
}

/// Cash-or-nothing digital paying $1 if the price ends at or above `k`: value
/// e^(-r tau) N(d2) and its Greeks.
pub fn digital(s: f64, k: f64, tau: f64, r: f64, sigma: f64) -> Greeks {
    let disc = exp(-r * tau);
    let vol_t = sigma * sqrt(tau);
    if tau <= 0.0 || s <= 0.0 || vol_t < 1e-12 {
        let itm = if tau <= 0.0 {
            s >= k
        } else {
            s * exp(r * tau) >= k
        };
        let value = if itm { disc } else { 0.0 };
        return Greeks {
            value,
            dv_dtau: -r * value,
            ..Greeks::default()
        };
    }
    let d2 = (ln(s / k) + (r - 0.5 * sigma * sigma) * tau) / vol_t;
    let d1 = d2 + vol_t;
    let n = disc * norm_pdf(d2);
    let value = disc * norm_cdf(d2);
    let dd2_dtau = ((r - 0.5 * sigma * sigma) * tau - ln(s / k)) / (2.0 * sigma * tau * sqrt(tau));
    Greeks {
        value,
        delta: n / (s * vol_t),
        gamma: -n * d1 / (s * s * sigma * sigma * tau),
        vega: -n * d1 / sigma,
        dv_dtau: -r * value + n * dd2_dtau,
    }
}

fn sub(a: Greeks, b: Greeks) -> Greeks {
    Greeks {
        value: a.value - b.value,
        delta: a.delta - b.delta,
        gamma: a.gamma - b.gamma,
        vega: a.vega - b.vega,
        dv_dtau: a.dv_dtau - b.dv_dtau,
    }
}

impl Binary {
    /// Resolves inputs against the venue preset and the thesis.
    pub fn resolve(s: &BinarySpec, th: &Thesis, notes: &mut Vec<String>) -> Res<Binary> {
        let strikes = match s.contract {
            Contract::Above => vec![s
                .strike
                .ok_or_else(|| crate::Error("binary needs a strike".into()))?],
            Contract::Range => match (s.lower, s.upper) {
                (Some(l), Some(u)) if l < u => vec![l, u],
                _ => return err("a range binary needs lower < upper"),
            },
        };
        if strikes.iter().any(|k| *k <= 0.0) {
            return err("binary strikes must be above zero");
        }
        if !(s.price > 0.0 && s.price < 1.0) {
            return err("binary price must be between $0 and $1");
        }
        if s.expiry < th.as_of {
            return err("binary expiry is before today");
        }
        let (preset_fees, preset_notes) = presets::binary(s.venue);
        if s.fees.is_none() {
            notes.extend(preset_notes.iter().map(|n| n.to_string()));
        }
        let mut b = Binary {
            side: s.side,
            strikes,
            expiry: s.expiry,
            price: s.price,
            fees: s.fees.clone().unwrap_or(preset_fees),
            liquidity: s.liquidity,
            iv: th.iv,
            rate: th.rate,
            s0: th.spot,
            tau0: th.tau(th.as_of, s.expiry),
            day: th.day_count(),
        };
        b.iv = match s.iv {
            Some(v) if v > 0.0 => v,
            Some(_) => return err("binary volatility must be above zero"),
            None => match digital_iv(|v| b.greeks(b.s0, b.tau0, v).value, s.price) {
                Ok(v) => v,
                Err(_) => {
                    notes.push(format!(
                        "No volatility reproduces the ${} price; the thesis volatility is used.",
                        crate::format::price_text(s.price)
                    ));
                    th.iv
                }
            },
        };
        Ok(b)
    }

    /// Value and Greeks of the side held, per contract.
    pub fn greeks(&self, s: f64, tau: f64, sigma: f64) -> Greeks {
        let r = self.rate;
        let yes = match self.strikes[..] {
            [k] => digital(s, k, tau, r, sigma),
            [l, u, ..] => sub(digital(s, l, tau, r, sigma), digital(s, u, tau, r, sigma)),
            [] => Greeks::default(),
        };
        if tau <= 0.0 {
            let pays = match self.strikes[..] {
                [k] => s >= k,
                [l, u, ..] => l <= s && s < u,
                [] => false,
            };
            let yes_pays = f64::from(u8::from(pays));
            let v = if self.side == Outcome::Yes {
                yes_pays
            } else {
                1.0 - yes_pays
            };
            return Greeks {
                value: v,
                ..Greeks::default()
            };
        }
        match self.side {
            Outcome::Yes => yes,
            Outcome::No => {
                let disc = exp(-r * tau);
                Greeks {
                    value: disc - yes.value,
                    delta: -yes.delta,
                    gamma: -yes.gamma,
                    vega: -yes.vega,
                    dv_dtau: -r * disc - yes.dv_dtau,
                }
            }
        }
    }

    /// P&L if closed at `s` on `date`, or settled at expiry.
    pub fn cell(&self, q: f64, s: f64, date: Date) -> Cell {
        let tau = self.day.year_frac(date, self.expiry);
        let g = self.greeks(s, tau, self.iv);
        let open = self.fees.open(self.price, q, self.liquidity);
        let exit = if date >= self.expiry {
            self.fees.settle(g.value, q)
        } else {
            self.fees.close(g.value, q, self.liquidity)
        };
        let days = self.day.year_days();
        Cell {
            pnl: q * (g.value - self.price) - open - exit,
            value: q * g.value,
            fees: open + exit,
            delta: q * g.delta,
            gamma: q * g.gamma,
            theta: -q * g.dv_dtau / days,
            vega: q * g.vega / 100.0,
            ..Cell::default()
        }
    }

    /// Contracts times price plus the entry fee.
    pub fn entry_cost(&self, q: f64) -> f64 {
        q * self.price + self.fees.open(self.price, q, self.liquidity)
    }

    /// The whole cost when the contract settles at $0.
    pub fn max_loss(&self, q: f64) -> Option<f64> {
        Some(self.entry_cost(q) + self.fees.settle(0.0, q))
    }

    /// $1 per contract less the cost and any settlement fee.
    pub fn max_gain(&self, q: f64) -> Option<f64> {
        Some(q - self.entry_cost(q) - self.fees.settle(1.0, q))
    }

    /// Dollar delta at entry.
    pub fn dollar_delta(&self, q: f64) -> f64 {
        q * self.greeks(self.s0, self.tau0, self.iv).delta * self.s0
    }
}

#[cfg(test)]
mod tests;
