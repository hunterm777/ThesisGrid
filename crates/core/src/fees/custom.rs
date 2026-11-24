//! User-defined binary fees: a flat fee per contract plus a percentage of
//! the traded value, charged on entry and on an early exit, not at settlement.

use serde::{Deserialize, Serialize};

/// Custom fee parameters.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomFees {
    /// Dollars per contract.
    #[serde(default)]
    pub per_contract: f64,
    /// Fraction of `price x contracts`.
    #[serde(default)]
    pub pct_notional: f64,
}

impl CustomFees {
    /// Fee for `contracts` at `price`.
    pub fn trade(&self, price: f64, contracts: f64) -> f64 {
        self.per_contract * contracts + self.pct_notional * price * contracts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_plus_percent() {
        let f = CustomFees {
            per_contract: 0.01,
            pct_notional: 0.02,
        };
        assert!((f.trade(0.5, 100.0) - 2.0).abs() < 1e-12);
    }
}
