//! Option strategies: templates that expand to legs, and the resolved
//! multi-leg position.

use crate::instrument::Cell;
use crate::iv::option_iv;
use crate::option::{intrinsic, price, Model, OptType};
use crate::thesis::{AssetClass, DayCount, Thesis};
use crate::{err, Date, Res, Side};
use serde::{Deserialize, Serialize};

/// What a leg holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LegKind {
    /// Call option.
    Call,
    /// Put option.
    Put,
    /// The underlying itself (covered calls).
    Underlying,
}

/// One leg's inputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegSpec {
    /// Call, put or underlying.
    pub kind: LegKind,
    /// Bought or sold.
    #[serde(default)]
    pub side: Side,
    /// Strike (options only).
    #[serde(default)]
    pub strike: Option<f64>,
    /// Expiry; the strategy's when omitted.
    #[serde(default)]
    pub expiry: Option<Date>,
    /// Premium per unit of underlying; priced from volatility when omitted.
    #[serde(default)]
    pub premium: Option<f64>,
    /// Volatility; implied from the premium, or the strategy's, when omitted.
    #[serde(default)]
    pub iv: Option<f64>,
    /// Contracts per strategy unit.
    #[serde(default = "one")]
    pub ratio: f64,
}

fn one() -> f64 {
    1.0
}

fn custom() -> String {
    "custom".into()
}

/// Strategy names the picker offers.
pub const STRATEGIES: [&str; 13] = [
    "long_call",
    "long_put",
    "covered_call",
    "cash_secured_put",
    "bull_call_spread",
    "bear_call_spread",
    "bull_put_spread",
    "bear_put_spread",
    "straddle",
    "strangle",
    "iron_condor",
    "butterfly",
    "custom",
];

/// Expands a named strategy into legs. Strikes are ascending.
pub fn template(name: &str, strikes: &[f64]) -> Res<Vec<LegSpec>> {
    use LegKind::{Call as C, Put as P, Underlying as U};
    use Side::{Long as L, Short as S};
    // (kind, side, strike index, ratio)
    let shape: &[(LegKind, Side, usize, f64)] = match name {
        "long_call" => &[(C, L, 0, 1.0)],
        "long_put" => &[(P, L, 0, 1.0)],
        "covered_call" => &[(U, L, 0, 1.0), (C, S, 0, 1.0)],
        "cash_secured_put" => &[(P, S, 0, 1.0)],
        "bull_call_spread" => &[(C, L, 0, 1.0), (C, S, 1, 1.0)],
        "bear_call_spread" => &[(C, S, 0, 1.0), (C, L, 1, 1.0)],
        "bull_put_spread" => &[(P, L, 0, 1.0), (P, S, 1, 1.0)],
        "bear_put_spread" => &[(P, S, 0, 1.0), (P, L, 1, 1.0)],
        "straddle" => &[(C, L, 0, 1.0), (P, L, 0, 1.0)],
        "strangle" => &[(P, L, 0, 1.0), (C, L, 1, 1.0)],
        "iron_condor" => &[
            (P, L, 0, 1.0),
            (P, S, 1, 1.0),
            (C, S, 2, 1.0),
            (C, L, 3, 1.0),
        ],
        "butterfly" => &[(C, L, 0, 1.0), (C, S, 1, 2.0), (C, L, 2, 1.0)],
        _ => return err(format!("unknown option strategy \"{name}\"")),
    };
    let need = shape.iter().map(|l| l.2).max().unwrap_or(0) + 1;
    if strikes.len() != need {
        return err(format!(
            "{name} takes {need} strike(s), got {}",
            strikes.len()
        ));
    }
    if strikes.windows(2).any(|w| w[0] >= w[1]) || strikes.iter().any(|k| *k <= 0.0) {
        return err("strikes must be positive and in ascending order");
    }
    Ok(shape
        .iter()
        .map(|&(kind, side, i, ratio)| LegSpec {
            kind,
            side,
            strike: (kind != U).then_some(strikes[i]),
            expiry: None,
            premium: None,
            iv: None,
            ratio,
        })
        .collect())
}

/// Option strategy inputs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionSpec {
    /// Column label; the kind when omitted.
    #[serde(default)]
    pub label: Option<String>,
    /// Whether the column is shown.
    #[serde(default = "crate::instrument::yes")]
    pub enabled: bool,
    /// Size locked by hand; overrides the comparison mode.
    #[serde(default)]
    pub qty: Option<f64>,
    /// Strategy name (see [`STRATEGIES`]).
    #[serde(default = "custom")]
    pub strategy: String,
    /// Strikes for the template, ascending.
    #[serde(default)]
    pub strikes: Vec<f64>,
    /// Expiry for every leg that gives none.
    #[serde(default)]
    pub expiry: Option<Date>,
    /// Explicit legs; when given they replace the template's.
    #[serde(default)]
    pub legs: Vec<LegSpec>,
    /// Volatility for legs that give neither premium nor volatility.
    #[serde(default)]
    pub iv: Option<f64>,
    /// Fee per option contract, charged on entry and on an early exit.
    #[serde(default)]
    pub fee_per_contract: f64,
    /// Units of underlying per contract: 100 for stocks and indices, 1 otherwise.
    #[serde(default)]
    pub multiplier: Option<f64>,
    /// Black-Scholes, or Black-76 for options on futures.
    #[serde(default)]
    pub model: Model,
}

#[derive(Clone, Debug, PartialEq)]
struct Leg {
    kind: LegKind,
    sign: f64,
    strike: f64,
    expiry: Date,
    premium: f64,
    iv: f64,
    ratio: f64,
}

/// A resolved strategy, per strategy unit.
#[derive(Clone, Debug, PartialEq)]
pub struct Strategy {
    legs: Vec<Leg>,
    fee: f64,
    mult: f64,
    model: Model,
    rate: f64,
    as_of: Date,
    s0: f64,
    day: DayCount,
}

mod position;
