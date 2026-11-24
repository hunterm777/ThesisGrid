//! The comparison modes: size every position to the same dollars in, the
//! same max loss or the same dollar delta at entry.

use crate::instrument::Position;
use crate::{err, Res};
use serde::{Deserialize, Serialize};

/// How positions are made comparable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Same cash outlay.
    #[default]
    EqualDollars,
    /// Same worst-case loss.
    EqualMaxLoss,
    /// Same dollar delta at entry.
    EqualExposure,
    /// Sizes typed by hand.
    Manual,
}

impl Mode {
    /// Display name.
    pub fn label(self) -> &'static str {
        match self {
            Mode::EqualDollars => "Equal dollars",
            Mode::EqualMaxLoss => "Equal max loss",
            Mode::EqualExposure => "Equal exposure",
            Mode::Manual => "Manual",
        }
    }
}

/// The quantity the mode measures, for quantity `q`.
pub fn metric(pos: &Position, mode: Mode, q: f64) -> Res<f64> {
    match mode {
        Mode::EqualDollars => Ok(pos.entry_cost(q)),
        Mode::EqualMaxLoss if pos.needs_stop() => err("needs a stop"),
        Mode::EqualMaxLoss => pos
            .max_loss(q)
            .ok_or_else(|| crate::Error("unlimited max loss: size it by hand".into())),
        Mode::EqualExposure => Ok(pos.dollar_delta(q).abs()),
        Mode::Manual => Ok(q),
    }
}

/// The largest quantity whose metric is at most `budget`. Every metric is
/// non-decreasing in quantity; venue fee rounding can make it step, so the
/// result lands within one rounding step (a cent) of the budget.
pub fn size(pos: &Position, mode: Mode, budget: f64) -> Res<f64> {
    if mode == Mode::Manual {
        return Ok(1.0);
    }
    if !(budget > 0.0 && budget.is_finite()) {
        return err("budget must be above zero");
    }
    let per_unit = metric(pos, mode, 1.0)?;
    if !(per_unit > 0.0 && per_unit.is_finite()) {
        return err(match mode {
            Mode::EqualDollars => "collects a credit with no defined cost: size it by hand",
            _ => "has no exposure at entry: size it by hand",
        });
    }
    let f = |q: f64| metric(pos, mode, q).unwrap_or(f64::INFINITY);
    if f(0.0) > budget {
        return err("fixed fees alone exceed the budget");
    }
    let mut hi = 2.0 * budget / per_unit + 1.0;
    while f(hi) <= budget {
        hi *= 2.0;
    }
    let mut lo = 0.0;
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        if f(mid) <= budget {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(lo)
}

#[cfg(test)]
mod tests;
