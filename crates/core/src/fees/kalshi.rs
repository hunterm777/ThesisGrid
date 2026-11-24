//! Kalshi binary fees.
//!
//! From Kalshi's fee schedule (effective July 7, 2026):
//! taker `round up(M x 0.07 x C x P x (1 - P))`, maker
//! `round up(M x 0.0175 x C x P x (1 - P))`, with the series multiplier M
//! defaulting to 1 for takers and 0 for makers. Fees round up to the cent.
//! There is no settlement fee.

use crate::math::ceil_to;
use crate::Liquidity;
use serde::{Deserialize, Serialize};

/// Taker coefficient.
pub const TAKER_COEF: f64 = 0.07;
/// Maker coefficient.
pub const MAKER_COEF: f64 = 0.0175;

fn one() -> f64 {
    1.0
}

/// Kalshi fee parameters: the series' taker and maker multipliers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KalshiFees {
    /// Taker multiplier M (default 1).
    #[serde(default = "one")]
    pub taker_multiplier: f64,
    /// Maker multiplier M (default 0: makers pay nothing).
    #[serde(default)]
    pub maker_multiplier: f64,
}

impl Default for KalshiFees {
    fn default() -> Self {
        KalshiFees {
            taker_multiplier: 1.0,
            maker_multiplier: 0.0,
        }
    }
}

impl KalshiFees {
    /// Fee for one order of `contracts` at `price`, rounded up to the cent.
    pub fn trade(&self, price: f64, contracts: f64, liq: Liquidity) -> f64 {
        let coef = match liq {
            Liquidity::Taker => TAKER_COEF * self.taker_multiplier,
            Liquidity::Maker => MAKER_COEF * self.maker_multiplier,
        };
        ceil_to(coef * contracts * price * (1.0 - price), 0.01)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prd_acceptance_case() {
        // 100 contracts at $0.50: 0.07 x 100 x 0.5 x 0.5 = $1.75.
        assert_eq!(
            KalshiFees::default().trade(0.5, 100.0, Liquidity::Taker),
            1.75
        );
        assert_eq!(
            KalshiFees::default().trade(0.5, 100.0, Liquidity::Maker),
            0.0
        );
        let m = KalshiFees {
            taker_multiplier: 1.0,
            maker_multiplier: 1.0,
        };
        // 0.0175 x 100 x 0.25 = 0.4375 -> $0.44
        assert_eq!(m.trade(0.5, 100.0, Liquidity::Maker), 0.44);
        // 0.07 x 1 x 0.31 x 0.69 = 0.014973 -> $0.02
        assert_eq!(
            KalshiFees::default().trade(0.31, 1.0, Liquidity::Taker),
            0.02
        );
    }
}
