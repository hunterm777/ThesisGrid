//! Instrument inputs and the resolved positions behind each grid column.

use crate::binary::{Binary, BinarySpec};
use crate::perp::{Perp, PerpSpec};
use crate::spot::{Spot, SpotSpec};
use crate::strategies::{OptionSpec, Strategy};
use crate::thesis::Thesis;
use crate::{Date, Res};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// serde default for `enabled`.
pub fn yes() -> bool {
    true
}

/// A spec with every field at its default.
pub fn empty_spec<T: DeserializeOwned>() -> T {
    serde_json::from_str("{}").expect("every spec field has a default")
}

/// One grid cell: P&L if the position is closed at this price on this date.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Cell {
    /// Dollars.
    pub pnl: f64,
    /// Percent of the position's cost to enter.
    pub pct: f64,
    /// Market value of the position, signed.
    pub value: f64,
    /// Fees paid in and out.
    pub fees: f64,
    /// Funding paid; negative means received.
    pub funding: f64,
    /// Position delta, in units of underlying.
    pub delta: f64,
    /// Position gamma.
    pub gamma: f64,
    /// Dollars per counted day.
    pub theta: f64,
    /// Dollars per volatility point.
    pub vega: f64,
    /// At or beyond the liquidation price.
    pub liquidated: bool,
}

/// One column's inputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum InstrumentSpec {
    /// Shares / spot.
    Spot(SpotSpec),
    /// Perpetual future.
    Perp(PerpSpec),
    /// Option strategy.
    Option(OptionSpec),
    /// Prediction-market binary.
    Binary(BinarySpec),
}

impl InstrumentSpec {
    /// `spot`, `perp`, `option` or `binary`.
    pub fn kind(&self) -> &'static str {
        match self {
            InstrumentSpec::Spot(_) => "spot",
            InstrumentSpec::Perp(_) => "perp",
            InstrumentSpec::Option(_) => "option",
            InstrumentSpec::Binary(_) => "binary",
        }
    }

    /// (label, enabled, qty lock).
    pub fn common(&self) -> (Option<&str>, bool, Option<f64>) {
        match self {
            InstrumentSpec::Spot(s) => (s.label.as_deref(), s.enabled, s.qty),
            InstrumentSpec::Perp(s) => (s.label.as_deref(), s.enabled, s.qty),
            InstrumentSpec::Option(s) => (s.label.as_deref(), s.enabled, s.qty),
            InstrumentSpec::Binary(s) => (s.label.as_deref(), s.enabled, s.qty),
        }
    }

    /// Venue name, if any.
    pub fn venue(&self) -> Option<crate::presets::Venue> {
        match self {
            InstrumentSpec::Perp(s) => Some(s.venue),
            InstrumentSpec::Binary(s) => Some(s.venue),
            _ => None,
        }
    }
}

/// A resolved position, priced per unit of quantity.
#[derive(Clone, Debug, PartialEq)]
pub enum Position {
    /// Shares / spot.
    Spot(Spot),
    /// Perpetual future.
    Perp(Perp),
    /// Option strategy.
    Option(Strategy),
    /// Binary.
    Binary(Binary),
}

impl Position {
    /// Resolves a spec, collecting caveats in `notes`.
    pub fn resolve(spec: &InstrumentSpec, th: &Thesis, notes: &mut Vec<String>) -> Res<Position> {
        Ok(match spec {
            InstrumentSpec::Spot(s) => Position::Spot(Spot::resolve(s, th)?),
            InstrumentSpec::Perp(s) => Position::Perp(Perp::resolve(s, th, notes)?),
            InstrumentSpec::Option(s) => Position::Option(Strategy::resolve(s, th)?),
            InstrumentSpec::Binary(s) => Position::Binary(Binary::resolve(s, th, notes)?),
        })
    }

    /// The cell at price `s` on `date` for quantity `q`, without `pct`.
    pub fn cell(&self, q: f64, s: f64, date: Date) -> Cell {
        match self {
            Position::Spot(p) => p.cell(q, s),
            Position::Perp(p) => p.cell(q, s, date),
            Position::Option(p) => p.cell(q, s, date),
            Position::Binary(p) => p.cell(q, s, date),
        }
    }

    /// Cash to enter.
    pub fn entry_cost(&self, q: f64) -> f64 {
        match self {
            Position::Spot(p) => p.entry_cost(q),
            Position::Perp(p) => p.entry_cost(q),
            Position::Option(p) => p.entry_cost(q),
            Position::Binary(p) => p.entry_cost(q),
        }
    }

    /// Worst-case loss as a positive number; `None` when unbounded.
    pub fn max_loss(&self, q: f64) -> Option<f64> {
        match self {
            Position::Spot(p) => p.max_loss(q),
            Position::Perp(p) => p.max_loss(q),
            Position::Option(p) => p.max_loss(q),
            Position::Binary(p) => p.max_loss(q),
        }
    }

    /// Best case; `None` when unbounded. Spot and perps are held to `horizon`.
    pub fn max_gain(&self, q: f64, horizon: Date) -> Option<f64> {
        match self {
            Position::Spot(p) => p.max_gain(q),
            Position::Perp(p) => p.max_gain(q, horizon),
            Position::Option(p) => p.max_gain(q),
            Position::Binary(p) => p.max_gain(q),
        }
    }

    /// Dollar delta at entry.
    pub fn dollar_delta(&self, q: f64) -> f64 {
        match self {
            Position::Spot(p) => p.dollar_delta(q),
            Position::Perp(p) => p.dollar_delta(q),
            Position::Option(p) => p.dollar_delta(q),
            Position::Binary(p) => p.dollar_delta(q),
        }
    }

    /// Latest expiry, if the position expires.
    pub fn expiry(&self) -> Option<Date> {
        match self {
            Position::Spot(p) => p.expiry(),
            Position::Perp(_) => None,
            Position::Option(p) => Some(p.expiry()),
            Position::Binary(p) => Some(p.expiry),
        }
    }

    /// Whether equal max loss can size this position.
    pub fn needs_stop(&self) -> bool {
        matches!(self, Position::Spot(s) if s.stop.is_none())
    }
}
