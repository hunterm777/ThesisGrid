//! Venue fee models.
//!
//! Perp fees on all three venues are `rate x |notional|` with no published
//! rounding, so they live in [`crate::perp`]. This module holds the binary
//! (prediction-market) fee models, which differ per venue.

pub mod custom;
pub mod hyperliquid;
pub mod kalshi;
pub mod polymarket;

use crate::Liquidity;
use serde::{Deserialize, Serialize};

/// A binary contract's fee model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "model", rename_all = "lowercase", deny_unknown_fields)]
pub enum BinaryFees {
    /// Kalshi trading fee.
    Kalshi(kalshi::KalshiFees),
    /// Polymarket per-market fee schedule.
    Polymarket(polymarket::PolymarketFees),
    /// Hyperliquid HIP-4 outcome markets.
    Hip4(hyperliquid::Hip4Fees),
    /// User-defined.
    Custom(custom::CustomFees),
}

impl BinaryFees {
    /// Fee to open `contracts` at `price` (dollars per contract).
    pub fn open(&self, price: f64, contracts: f64, liq: Liquidity) -> f64 {
        match self {
            BinaryFees::Kalshi(f) => f.trade(price, contracts, liq),
            BinaryFees::Polymarket(f) => f.trade(price, contracts, liq),
            BinaryFees::Hip4(_) => 0.0,
            BinaryFees::Custom(f) => f.trade(price, contracts),
        }
    }

    /// Fee to close `contracts` at `price` before expiry.
    pub fn close(&self, price: f64, contracts: f64, liq: Liquidity) -> f64 {
        match self {
            BinaryFees::Kalshi(f) => f.trade(price, contracts, liq),
            BinaryFees::Polymarket(f) => f.trade(price, contracts, liq),
            BinaryFees::Hip4(f) => f.close(price, contracts, liq),
            BinaryFees::Custom(f) => f.trade(price, contracts),
        }
    }

    /// Fee at settlement, when the contract pays `value` (0 or 1).
    pub fn settle(&self, value: f64, contracts: f64) -> f64 {
        match self {
            BinaryFees::Hip4(f) => f.settle(value, contracts),
            _ => 0.0,
        }
    }
}
