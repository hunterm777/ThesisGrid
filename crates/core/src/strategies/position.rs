//! Resolution and evaluation of a multi-leg option position.

use super::*;

impl Strategy {
    /// Resolves inputs against the thesis.
    pub fn resolve(s: &OptionSpec, th: &Thesis) -> Res<Strategy> {
        let specs = if s.legs.is_empty() {
            template(&s.strategy, &s.strikes)?
        } else {
            s.legs.clone()
        };
        if specs.is_empty() {
            return err("a custom strategy needs at least one leg");
        }
        let mult = s.multiplier.unwrap_or(match th.asset_class {
            AssetClass::Stock | AssetClass::Index => 100.0,
            _ => 1.0,
        });
        if mult <= 0.0 || s.fee_per_contract < 0.0 {
            return err("multiplier must be above zero and fees non-negative");
        }
        let base_iv = s.iv.unwrap_or(th.iv);
        let mut legs = Vec::with_capacity(specs.len());
        for l in &specs {
            legs.push(Self::leg(l, s, th, base_iv)?);
        }
        let fee = s.fee_per_contract;
        let (model, rate, as_of, s0, day) = (s.model, th.rate, th.as_of, th.spot, th.day_count());
        Ok(Strategy {
            legs,
            fee,
            mult,
            model,
            rate,
            as_of,
            s0,
            day,
        })
    }

    fn leg(l: &LegSpec, s: &OptionSpec, th: &Thesis, base_iv: f64) -> Res<Leg> {
        let sign = l.side.sign();
        if l.ratio.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater) {
            return err("leg ratio must be above zero");
        }
        let expiry = l
            .expiry
            .or(s.expiry)
            .ok_or_else(|| crate::Error("option needs an expiry".into()))?;
        if expiry < th.as_of {
            return err("option expiry is before today");
        }
        let ty = match l.kind {
            LegKind::Underlying => {
                let premium = l.premium.unwrap_or(th.spot);
                return Ok(Leg {
                    kind: l.kind,
                    sign,
                    strike: 0.0,
                    expiry,
                    premium,
                    iv: 0.0,
                    ratio: l.ratio,
                });
            }
            LegKind::Call => OptType::Call,
            LegKind::Put => OptType::Put,
        };
        let strike = l
            .strike
            .filter(|k| *k > 0.0)
            .ok_or_else(|| crate::Error("option leg needs a positive strike".into()))?;
        let tau = th.tau(th.as_of, expiry);
        let (premium, iv) = match (l.premium, l.iv) {
            (_, Some(v)) if v <= 0.0 => return err("leg volatility must be above zero"),
            (Some(p), None) => (p, option_iv(s.model, ty, th.spot, strike, tau, th.rate, p)?),
            (Some(p), Some(v)) => (p, v),
            (None, v) => {
                let v = v.unwrap_or(base_iv);
                (
                    price(s.model, ty, th.spot, strike, tau, th.rate, v).value,
                    v,
                )
            }
        };
        Ok(Leg {
            kind: l.kind,
            sign,
            strike,
            expiry,
            premium,
            iv,
            ratio: l.ratio,
        })
    }

    fn opt_type(kind: LegKind) -> OptType {
        if kind == LegKind::Put {
            OptType::Put
        } else {
            OptType::Call
        }
    }

    fn contracts(&self, q: f64) -> f64 {
        self.legs
            .iter()
            .filter(|l| l.kind != LegKind::Underlying)
            .map(|l| l.ratio * q)
            .sum()
    }

    /// P&L if closed at `s` on `date`; legs past expiry are worth intrinsic.
    pub fn cell(&self, q: f64, s: f64, date: Date) -> Cell {
        let days = self.day.year_days();
        let mut c = Cell {
            fees: self.fee * self.contracts(q),
            ..Cell::default()
        };
        for l in &self.legs {
            let n = l.sign * l.ratio * self.mult * q;
            let (value, delta, gamma, vega, dv_dtau) = if l.kind == LegKind::Underlying {
                (s, 1.0, 0.0, 0.0, 0.0)
            } else {
                let tau = self.day.year_frac(date, l.expiry);
                if date < l.expiry {
                    c.fees += self.fee * l.ratio * q;
                }
                let g = price(
                    self.model,
                    Self::opt_type(l.kind),
                    s,
                    l.strike,
                    tau,
                    self.rate,
                    l.iv,
                );
                (g.value, g.delta, g.gamma, g.vega, g.dv_dtau)
            };
            c.pnl += n * (value - l.premium);
            c.value += n * value;
            c.delta += n * delta;
            c.gamma += n * gamma;
            c.vega += n * vega / 100.0;
            c.theta -= n * dv_dtau / days;
        }
        c.pnl -= c.fees;
        c
    }

    fn net_debit(&self, q: f64) -> f64 {
        self.legs
            .iter()
            .map(|l| l.sign * l.ratio * self.mult * q * l.premium)
            .sum()
    }

    fn expiry_pnl(&self, q: f64, s: f64) -> f64 {
        let payoff = |l: &Leg| match l.kind {
            LegKind::Underlying => s,
            k => intrinsic(Self::opt_type(k), s, l.strike),
        };
        let held: f64 = self
            .legs
            .iter()
            .map(|l| l.sign * l.ratio * self.mult * q * payoff(l))
            .sum();
        held - self.net_debit(q) - self.fee * self.contracts(q)
    }

    /// Lowest and highest P&L with every leg held to its expiry, `None`
    /// where unbounded. Payoffs are piecewise linear with kinks at strikes.
    fn expiry_range(&self, q: f64) -> (Option<f64>, Option<f64>) {
        let mut pts: Vec<f64> = vec![0.0];
        pts.extend(
            self.legs
                .iter()
                .filter(|l| l.kind != LegKind::Underlying)
                .map(|l| l.strike),
        );
        let vals: Vec<f64> = pts.iter().map(|&s| self.expiry_pnl(q, s)).collect();
        let slope: f64 = self
            .legs
            .iter()
            .filter(|l| l.kind != LegKind::Put)
            .map(|l| l.sign * l.ratio * self.mult * q)
            .sum();
        let lo = vals.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let tol = 1e-12 * self.mult.max(1.0);
        ((slope >= -tol).then_some(lo), (slope <= tol).then_some(hi))
    }

    /// Worst case at expiry, as a positive number; `None` when unbounded.
    pub fn max_loss(&self, q: f64) -> Option<f64> {
        self.expiry_range(q).0.map(|lo| (-lo).max(0.0))
    }

    /// Best case at expiry; `None` when unbounded.
    pub fn max_gain(&self, q: f64) -> Option<f64> {
        self.expiry_range(q).1
    }

    /// Net premium plus fees, or the max loss when that is larger (credit
    /// spreads, covered calls and cash-secured puts tie up their max loss).
    pub fn entry_cost(&self, q: f64) -> f64 {
        let paid = self.net_debit(q) + self.fee * self.contracts(q);
        self.max_loss(q).map_or(paid, |m| paid.max(m))
    }

    /// Dollar delta at entry.
    pub fn dollar_delta(&self, q: f64) -> f64 {
        self.cell(q, self.s0, self.as_of).delta * self.s0
    }

    /// Latest leg expiry.
    pub fn expiry(&self) -> Date {
        self.legs
            .iter()
            .map(|l| l.expiry)
            .max()
            .unwrap_or(self.as_of)
    }
}

#[cfg(test)]
mod tests;
