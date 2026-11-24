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

#[cfg(test)]
mod tests {
    use super::*;

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
