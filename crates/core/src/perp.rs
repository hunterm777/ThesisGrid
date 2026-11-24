//! Perpetual futures: linear P&L on notional, fees on notional, funding at
//! the entered rate, and isolated-margin liquidation.
//!
//! Liquidation follows Hyperliquid's published formula
//! `liq = price - side x margin_available / size / (1 - mmr x side)`, with
//! `margin_available = margin - mmr x notional`, extended so funding paid
//! reduces the margin. Funding is charged on entry notional; the venues use
//! the oracle or mark price at each payment, which depends on the path.

use crate::instrument::Cell;
use crate::presets::{self, Venue};
use crate::thesis::{AssetClass, Thesis};
use crate::{err, Date, Liquidity, Res, Side};
use serde::{Deserialize, Serialize};

/// Perp inputs. `None` fields come from the venue preset.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerpSpec {
    /// Column label; the kind when omitted.
    #[serde(default)]
    pub label: Option<String>,
    /// Whether the column is shown.
    #[serde(default = "crate::instrument::yes")]
    pub enabled: bool,
    /// Size locked by hand; overrides the comparison mode.
    #[serde(default)]
    pub qty: Option<f64>,
    /// Long or short.
    #[serde(default)]
    pub side: Side,
    /// Venue preset.
    #[serde(default)]
    pub venue: Venue,
    /// Preset asset class; the thesis's when omitted.
    #[serde(default)]
    pub asset: Option<AssetClass>,
    /// Leverage (default 1).
    #[serde(default)]
    pub leverage: Option<f64>,
    /// Entry price; the thesis's current price when omitted.
    #[serde(default)]
    pub entry: Option<f64>,
    /// Funding rate per `funding_period_hours`; positive means longs pay.
    #[serde(default)]
    pub funding_rate: Option<f64>,
    /// Hours the rate is quoted for.
    #[serde(default)]
    pub funding_period_hours: Option<f64>,
    /// Hours between payments.
    #[serde(default)]
    pub funding_interval_hours: Option<f64>,
    /// Cap per payment; 0 or less means none.
    #[serde(default)]
    pub funding_cap: Option<f64>,
    /// Rates below this per payment are set to zero; 0 or less means none.
    #[serde(default)]
    pub funding_floor: Option<f64>,
    /// Maintenance margin rate on notional.
    #[serde(default)]
    pub maintenance_margin: Option<f64>,
    /// Venue maximum leverage.
    #[serde(default)]
    pub max_leverage: Option<f64>,
    /// Taker fee rate on notional.
    #[serde(default)]
    pub taker_fee: Option<f64>,
    /// Maker fee rate on notional.
    #[serde(default)]
    pub maker_fee: Option<f64>,
    /// Whether entry and exit fill as taker or maker.
    #[serde(default)]
    pub liquidity: Liquidity,
}

impl Default for PerpSpec {
    fn default() -> Self {
        crate::instrument::empty_spec()
    }
}

/// A resolved perp position, per unit of underlying.
#[derive(Clone, Debug, PartialEq)]
pub struct Perp {
    side: Side,
    /// Leverage.
    pub leverage: f64,
    entry: f64,
    /// Effective funding rate per payment after the venue's cap and floor.
    pub funding_per_payment: f64,
    interval_hours: f64,
    /// Maintenance margin rate.
    pub mmr: f64,
    fee: f64,
    as_of: Date,
}

fn positive(x: Option<f64>) -> Option<f64> {
    x.filter(|v| *v > 0.0)
}

impl Perp {
    /// Resolves inputs against the venue preset and the thesis.
    pub fn resolve(s: &PerpSpec, th: &Thesis, notes: &mut Vec<String>) -> Res<Perp> {
        let preset = presets::perp(s.venue, s.asset.unwrap_or(th.asset_class));
        let p = preset.as_ref();
        if let Some(p) = p {
            notes.extend(p.notes.iter().map(|n| n.to_string()));
        }
        let leverage = s.leverage.unwrap_or(1.0);
        let max_lev = s.max_leverage.or(p.and_then(|p| p.max_leverage));
        if !(leverage > 0.0 && leverage.is_finite()) {
            return err("leverage must be above zero");
        }
        if let Some(m) = max_lev.filter(|m| leverage > *m) {
            return err(format!(
                "leverage {leverage}x is above the venue max of {m}x"
            ));
        }
        let mmr = match s.maintenance_margin {
            Some(m) => m,
            None => match (p.and_then(|p| p.mm_factor), max_lev) {
                (Some(f), Some(m)) => f / m,
                _ => {
                    notes.push(
                        "No maintenance margin set: liquidation is shown where the margin is \
                         used up."
                            .into(),
                    );
                    0.0
                }
            },
        };
        if !(0.0..1.0).contains(&mmr) {
            return err("maintenance margin must be at least 0 and below 1");
        }
        let period = s
            .funding_period_hours
            .or(p.map(|p| p.funding_period_hours))
            .unwrap_or(8.0);
        let interval = s
            .funding_interval_hours
            .or(p.map(|p| p.funding_interval_hours));
        let interval = interval.unwrap_or(period);
        if period <= 0.0 || interval <= 0.0 {
            return err("funding period and interval must be above zero");
        }
        let rate = s.funding_rate.or(p.map(|p| p.funding_rate)).unwrap_or(0.0);
        let mut per = rate * interval / period;
        if let Some(cap) = positive(s.funding_cap.or(p.and_then(|p| p.funding_cap))) {
            per = per.clamp(-cap, cap);
        }
        if let Some(floor) = positive(s.funding_floor.or(p.and_then(|p| p.funding_floor))) {
            if per.abs() < floor {
                per = 0.0;
            }
        }
        let fee = match s.liquidity {
            Liquidity::Taker => s.taker_fee.or(p.map(|p| p.taker_fee)),
            Liquidity::Maker => s.maker_fee.or(p.map(|p| p.maker_fee)),
        }
        .unwrap_or(0.0);
        let entry = s.entry.unwrap_or(th.spot);
        if entry <= 0.0 {
            return err("perp entry price must be above zero");
        }
        Ok(Perp {
            side: s.side,
            leverage,
            entry,
            funding_per_payment: per,
            interval_hours: interval,
            mmr,
            fee,
            as_of: th.as_of,
        })
    }

    /// Entry date.
    pub fn as_of(&self) -> Date {
        self.as_of
    }

    /// Funding payments made by `date` (perps fund 24/7 on calendar time).
    pub fn payments(&self, date: Date) -> f64 {
        let hours = f64::from(self.as_of.days_to(date).max(0)) * 24.0;
        libm::floor(hours / self.interval_hours + 1e-9)
    }

    /// Funding paid by `date`; negative means received.
    pub fn funding(&self, q: f64, date: Date) -> f64 {
        self.side.sign() * self.funding_per_payment * self.payments(date) * q * self.entry
    }

    fn margin(&self, q: f64) -> f64 {
        q * self.entry / self.leverage
    }

    fn open_fee(&self, q: f64) -> f64 {
        self.fee * q * self.entry
    }

    /// Liquidation price on `date`, after the funding paid by then.
    pub fn liq_price(&self, date: Date) -> f64 {
        let sign = self.side.sign();
        let paid = self.funding(1.0, date);
        (sign * self.entry - self.entry / self.leverage + paid) / (sign - self.mmr)
    }

    /// P&L if closed at `s` on `date`.
    pub fn cell(&self, q: f64, s: f64, date: Date) -> Cell {
        let sign = self.side.sign();
        let liq = self.liq_price(date);
        let liquidated = q > 0.0 && if sign > 0.0 { s <= liq } else { s >= liq };
        let funding = self.funding(q, date);
        if liquidated {
            return Cell {
                pnl: -(self.margin(q) + self.open_fee(q)),
                fees: self.open_fee(q),
                funding,
                liquidated: true,
                ..Cell::default()
            };
        }
        let fees = self.open_fee(q) + self.fee * q * s;
        Cell {
            pnl: sign * q * (s - self.entry) - fees - funding,
            value: sign * q * s,
            fees,
            funding,
            delta: sign * q,
            ..Cell::default()
        }
    }

    /// Margin plus the opening fee.
    pub fn entry_cost(&self, q: f64) -> f64 {
        self.margin(q) + self.open_fee(q)
    }

    /// Isolated margin plus the opening fee: the most a liquidation can take.
    pub fn max_loss(&self, q: f64) -> Option<f64> {
        Some(self.entry_cost(q))
    }

    /// Unlimited for a long; the price going to zero by `horizon` for a short.
    pub fn max_gain(&self, q: f64, horizon: Date) -> Option<f64> {
        match self.side {
            Side::Long => None,
            Side::Short => Some(self.cell(q, 0.0, horizon).pnl),
        }
    }

    /// Dollar delta at entry.
    pub fn dollar_delta(&self, q: f64) -> f64 {
        self.side.sign() * q * self.entry
    }
}

#[cfg(test)]
mod tests;
