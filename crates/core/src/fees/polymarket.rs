//! Polymarket binary fees.
//!
//! From Polymarket's fee docs: `fee = C x feeRate x p x (1 - p)`, "rounded to
//! 5 decimal places"; "the smallest fee charged is 0.00001 USDC". The rate,
//! exponent and taker-only flag are set per market (`feeSchedule` in the
//! market details). The exponent applies to the price term `p x (1 - p)`.
//! The docs give no rounding direction; ThesisGrid rounds down, which is
//! what "anything smaller rounds to zero" implies.

use crate::math::floor_to;
use crate::Liquidity;
use serde::{Deserialize, Serialize};

fn one() -> f64 {
    1.0
}

fn yes() -> bool {
    true
}

/// One market's fee schedule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolymarketFees {
    /// `feeSchedule.rate`, set per market.
    #[serde(default)]
    pub rate: f64,
    /// `feeSchedule.exponent` (1 in the docs' formula).
    #[serde(default = "one")]
    pub exponent: f64,
    /// `feeSchedule.takerOnly`: makers pay nothing.
    #[serde(default = "yes")]
    pub taker_only: bool,
}

impl PolymarketFees {
    /// Fee for `contracts` shares at `price`.
    pub fn trade(&self, price: f64, contracts: f64, liq: Liquidity) -> f64 {
        if self.taker_only && liq == Liquidity::Maker {
            return 0.0;
        }
        let curve = libm::pow(price * (1.0 - price), self.exponent);
        floor_to(contracts * self.rate * curve, 0.000_01)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prd_acceptance_case() {
        let f = PolymarketFees {
            rate: 0.05,
            exponent: 1.0,
            taker_only: true,
        };
        assert_eq!(f.trade(0.5, 100.0, Liquidity::Taker), 1.25);
        assert_eq!(f.trade(0.5, 100.0, Liquidity::Maker), 0.0);
        let tiny = PolymarketFees {
            rate: 0.0001,
            exponent: 1.0,
            taker_only: false,
        };
        assert_eq!(tiny.trade(0.01, 1.0, Liquidity::Maker), 0.0);
    }
}
