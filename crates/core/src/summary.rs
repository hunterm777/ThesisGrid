//! The summary row: cost, max loss, max gain, breakevens and P&L at target.

use crate::instrument::Position;
use crate::math::bisect;
use crate::{Date, Res};
use serde::Serialize;

/// One instrument's summary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Summary {
    /// Quantity held (shares, coins, strategy units or contracts).
    pub qty: f64,
    /// Cash to enter.
    pub cost: f64,
    /// Worst case as a positive number; `null` when unlimited.
    pub max_loss: Option<f64>,
    /// Best case; `null` when unlimited.
    pub max_gain: Option<f64>,
    /// Prices where P&L on the target date is zero.
    pub breakevens: Vec<f64>,
    /// P&L at the target price on the target date.
    pub pnl_at_target: f64,
    /// That P&L as a percent of the cost to enter.
    pub roc_at_target: f64,
    /// Liquidation price at entry (perps only).
    pub liquidation: Option<f64>,
    /// Dollar delta at entry.
    pub dollar_delta: f64,
}

/// Percent of `cost`, 0 when there is no cost.
pub fn pct(pnl: f64, cost: f64) -> f64 {
    if cost > 0.0 {
        100.0 * pnl / cost
    } else {
        0.0
    }
}

/// Zero crossings of `f` on `(0, hi]`, scanned on a log grid then bisected.
pub fn roots(f: impl Fn(f64) -> f64, lo: f64, hi: f64) -> Vec<f64> {
    let n = 4000;
    let ratio = libm::pow(hi / lo, 1.0 / f64::from(n));
    let mut out = Vec::new();
    let mut a = lo;
    let mut fa = f(a);
    for _ in 0..n {
        let b = a * ratio;
        let fb = f(b);
        if fa == 0.0 {
            out.push(a);
        } else if (fa < 0.0) != (fb < 0.0) && fb != 0.0 {
            let r = bisect(a, b, &f);
            // A jump (liquidation) is not a breakeven.
            if f(r).abs() < 1e-6 * (1.0 + fa.abs().max(fb.abs())) {
                out.push(r);
            }
        }
        a = b;
        fa = fb;
    }
    out
}

/// Builds the summary for `pos` held at quantity `q`.
pub fn summarize(
    pos: &Position,
    q: f64,
    target: f64,
    target_date: Date,
    scale: f64,
) -> Res<Summary> {
    let cost = pos.entry_cost(q);
    let at = pos.cell(q, target, target_date).pnl;
    let f = |s: f64| pos.cell(q, s, target_date).pnl;
    Ok(Summary {
        qty: q,
        cost,
        max_loss: pos.max_loss(q),
        max_gain: pos.max_gain(q, target_date),
        breakevens: roots(f, scale * 1e-4, scale * 10.0),
        pnl_at_target: at,
        roc_at_target: pct(at, cost),
        liquidation: match pos {
            Position::Perp(p) => Some(p.liq_price(p.as_of())),
            _ => None,
        },
        dollar_delta: pos.dollar_delta(q),
    })
}
