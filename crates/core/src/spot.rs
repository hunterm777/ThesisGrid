//! Shares or spot: linear P&L minus fees.

use crate::instrument::Cell;
use crate::thesis::Thesis;
use crate::{err, Date, Res, Side};
use serde::{Deserialize, Serialize};

/// Shares / spot inputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpotSpec {
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
    /// Entry price; the thesis's current price when omitted.
    #[serde(default)]
    pub price: Option<f64>,
    /// Fee as a fraction of traded value, per trade.
    #[serde(default)]
    pub fee_pct: f64,
    /// Flat dollars per trade.
    #[serde(default)]
    pub fee_fixed: f64,
    /// Stop price, used to size by equal max loss.
    #[serde(default)]
    pub stop: Option<f64>,
}

impl Default for SpotSpec {
    fn default() -> Self {
        crate::instrument::empty_spec()
    }
}

/// A resolved spot position, per unit of quantity (one share or coin).
#[derive(Clone, Debug, PartialEq)]
pub struct Spot {
    side: Side,
    entry: f64,
    fee_pct: f64,
    fee_fixed: f64,
    /// Stop price, if any.
    pub stop: Option<f64>,
}

impl Spot {
    /// Resolves inputs against the thesis.
    pub fn resolve(s: &SpotSpec, th: &Thesis) -> Res<Spot> {
        let entry = s.price.unwrap_or(th.spot);
        if entry <= 0.0 || s.fee_pct < 0.0 || s.fee_fixed < 0.0 {
            return err("shares need a positive price and non-negative fees");
        }
        if let Some(stop) = s.stop {
            let ok = match s.side {
                Side::Long => stop < entry && stop >= 0.0,
                Side::Short => stop > entry,
            };
            if !ok {
                return err("the stop must be on the losing side of the entry price");
            }
        }
        Ok(Spot {
            side: s.side,
            entry,
            fee_pct: s.fee_pct,
            fee_fixed: s.fee_fixed,
            stop: s.stop,
        })
    }

    fn fee(&self, px: f64, q: f64) -> f64 {
        self.fee_pct * q * px + self.fee_fixed
    }

    /// P&L if closed at `s`.
    pub fn cell(&self, q: f64, s: f64) -> Cell {
        let sign = self.side.sign();
        let fees = self.fee(self.entry, q) + self.fee(s, q);
        Cell {
            pnl: sign * q * (s - self.entry) - fees,
            value: sign * q * s,
            fees,
            delta: sign * q,
            ..Cell::default()
        }
    }

    /// Cash to enter.
    pub fn entry_cost(&self, q: f64) -> f64 {
        q * self.entry + self.fee(self.entry, q)
    }

    /// Worst case: the stop, or the price going to zero for a long without one.
    pub fn max_loss(&self, q: f64) -> Option<f64> {
        match (self.stop, self.side) {
            (Some(stop), _) => Some(-self.cell(q, stop).pnl),
            (None, Side::Long) => Some(-self.cell(q, 0.0).pnl),
            (None, Side::Short) => None,
        }
    }

    /// Best case: unlimited for a long, the price going to zero for a short.
    pub fn max_gain(&self, q: f64) -> Option<f64> {
        match self.side {
            Side::Long => None,
            Side::Short => Some(self.cell(q, 0.0).pnl),
        }
    }

    /// Dollar delta at entry.
    pub fn dollar_delta(&self, q: f64) -> f64 {
        self.side.sign() * q * self.entry
    }

    /// Spot positions do not expire.
    pub fn expiry(&self) -> Option<Date> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn th() -> Thesis {
        serde_json::from_str(
            r#"{"spot":100,"target":120,"target_date":"2026-12-31","as_of":"2026-11-24"}"#,
        )
        .unwrap()
    }

    #[test]
    fn long_and_short() {
        let spec = SpotSpec {
            fee_pct: 0.001,
            fee_fixed: 1.0,
            ..Default::default()
        };
        let s = Spot::resolve(&spec, &th()).unwrap();
        let c = s.cell(10.0, 120.0);
        assert!((c.pnl - (200.0 - 1.0 - 1.0 - 1.2 - 1.0)).abs() < 1e-12);
        assert!((s.entry_cost(10.0) - 1002.0).abs() < 1e-12);
        assert!((s.max_loss(10.0).unwrap() - 1003.0).abs() < 1e-12);
        assert_eq!(s.max_gain(10.0), None);
        let short = Spot::resolve(
            &SpotSpec {
                side: Side::Short,
                ..spec.clone()
            },
            &th(),
        )
        .unwrap();
        assert_eq!(short.max_loss(1.0), None);
        assert!((short.max_gain(1.0).unwrap() - 97.9).abs() < 1e-12);
        assert_eq!(short.dollar_delta(2.0), -200.0);
        let stopped = SpotSpec {
            stop: Some(90.0),
            ..spec.clone()
        };
        let st = Spot::resolve(&stopped, &th()).unwrap();
        assert!((st.max_loss(10.0).unwrap() - (100.0 + 2.0 + 1.9)).abs() < 1e-12);
        assert!(Spot::resolve(
            &SpotSpec {
                stop: Some(110.0),
                ..spec.clone()
            },
            &th()
        )
        .is_err());
        let bad_short = SpotSpec {
            side: Side::Short,
            stop: Some(90.0),
            ..spec
        };
        assert!(Spot::resolve(&bad_short, &th()).is_err());
        assert!(Spot::resolve(
            &SpotSpec {
                price: Some(0.0),
                ..Default::default()
            },
            &th()
        )
        .is_err());
        assert_eq!(st.expiry(), None);
    }
}
