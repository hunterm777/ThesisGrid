//! Hyperliquid HIP-4 outcome-market fees.
//!
//! From Hyperliquid's fee docs ("Outcome tokens"): fees are charged only when
//! closing or settling, never when opening; outcome trading has no rebates,
//! and users who would get a maker rebate pay zero. The HIP-4 deployer page
//! (marked testnet-only) says users pay the base outcome rate times
//! `scale + max(scale, 1)`. The HIP-4 page says fees are currently zero for
//! initial testing, so the preset rates are 0. Fee-paying volume is
//! `px x sz`; settlement is charged at the taker rate on the settlement
//! value (not published; ThesisGrid's reading). No rounding is published.

use crate::Liquidity;
use serde::{Deserialize, Serialize};

fn one() -> f64 {
    1.0
}

/// HIP-4 fee parameters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hip4Fees {
    /// Base taker rate.
    #[serde(default)]
    pub taker_rate: f64,
    /// Base maker rate; a negative (rebate) rate pays zero.
    #[serde(default)]
    pub maker_rate: f64,
    /// Deployer fee scale.
    #[serde(default = "one")]
    pub deployer_scale: f64,
}

impl Default for Hip4Fees {
    fn default() -> Self {
        Hip4Fees {
            taker_rate: 0.0,
            maker_rate: 0.0,
            deployer_scale: 1.0,
        }
    }
}

impl Hip4Fees {
    /// Multiple of the base rate a user pays.
    pub fn factor(&self) -> f64 {
        self.deployer_scale + self.deployer_scale.max(1.0)
    }

    fn rate(&self, liq: Liquidity) -> f64 {
        let base = match liq {
            Liquidity::Taker => self.taker_rate,
            Liquidity::Maker => self.maker_rate,
        };
        base.max(0.0) * self.factor()
    }

    /// Fee to close `contracts` at `price`.
    pub fn close(&self, price: f64, contracts: f64, liq: Liquidity) -> f64 {
        self.rate(liq) * price * contracts
    }

    /// Fee at settlement on the settlement value.
    pub fn settle(&self, value: f64, contracts: f64) -> f64 {
        self.rate(Liquidity::Taker) * value * contracts
    }
}

/// All-in fee rate on a HIP-3 builder perp, from a validator-operated perp
/// base rate. From Hyperliquid's fee docs ("Fee formula for developers"):
/// the base rate is multiplied by `scale + 1` when the deployer fee scale is
/// below 1 and by `2 x scale` otherwise. Growth mode gives "a >=90%
/// reduction on the all-in fees"; ThesisGrid applies exactly 90%.
pub fn hip3_rate(base: f64, deployer_scale: f64, growth_mode: bool) -> f64 {
    let mult = if deployer_scale < 1.0 {
        deployer_scale + 1.0
    } else {
        2.0 * deployer_scale
    };
    let growth = if growth_mode { 0.1 } else { 1.0 };
    base * mult * growth
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hip3_multiplier_and_growth_mode() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-15;
        assert!(close(hip3_rate(0.000_45, 0.5, false), 0.000_675));
        assert!(close(hip3_rate(0.000_45, 1.0, false), 0.0009));
        assert!(close(hip3_rate(0.000_45, 3.0, false), 0.0027));
        // Docs: growth-mode all-in taker is 0.0045% to 0.009%.
        assert!(close(hip3_rate(0.000_45, 0.0, true), 0.000_045));
        assert!(close(hip3_rate(0.000_45, 1.0, true), 0.000_09));
        // xyz:NVDA's page on Nov 24 2026: 0.0090% taker, 0.0030% maker.
        assert!(close(hip3_rate(0.000_15, 1.0, true), 0.000_03));
    }

    #[test]
    fn close_and_settle_only() {
        let f = Hip4Fees {
            taker_rate: 0.001,
            maker_rate: -0.0001,
            deployer_scale: 0.5,
        };
        assert_eq!(f.factor(), 1.5);
        assert!((f.close(0.4, 100.0, Liquidity::Taker) - 0.06).abs() < 1e-15);
        assert_eq!(f.close(0.4, 100.0, Liquidity::Maker), 0.0);
        assert!((f.settle(1.0, 100.0) - 0.15).abs() < 1e-15);
        assert_eq!(Hip4Fees::default().settle(1.0, 100.0), 0.0);
    }
}
