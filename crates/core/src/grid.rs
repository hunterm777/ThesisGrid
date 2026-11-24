//! The price x date grid.

use crate::thesis::Thesis;
use crate::{err, Date, Res};
use serde::{Deserialize, Serialize};

/// Rows: underlying prices.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PriceAxis {
    /// `steps` evenly spaced prices from `spot x (1 + lo)` to `spot x (1 + hi)`.
    RangePct {
        /// Lower bound as a fraction of spot, e.g. -0.3.
        lo: f64,
        /// Upper bound as a fraction of spot, e.g. 0.3.
        hi: f64,
        /// Number of rows.
        steps: u32,
    },
    /// `steps` evenly spaced prices from `lo` to `hi`.
    Range {
        /// Lowest price.
        lo: f64,
        /// Highest price.
        hi: f64,
        /// Number of rows.
        steps: u32,
    },
    /// Explicit prices.
    List {
        /// Prices.
        values: Vec<f64>,
    },
}

impl Default for PriceAxis {
    fn default() -> Self {
        PriceAxis::RangePct {
            lo: -0.3,
            hi: 0.3,
            steps: 25,
        }
    }
}

fn thirty() -> u32 {
    30
}

/// Columns: dates from today to the latest expiry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DateAxis {
    /// First column; today when omitted.
    #[serde(default)]
    pub start: Option<Date>,
    /// Last column; the latest expiry (or the target date) when omitted.
    #[serde(default)]
    pub end: Option<Date>,
    /// Counted days per column (trading days for stocks and indices); chosen
    /// to fit `max_cols` when omitted.
    #[serde(default)]
    pub step: Option<u32>,
    /// Column cap used when `step` is omitted.
    #[serde(default = "thirty")]
    pub max_cols: u32,
    /// Explicit dates, replacing start, end and step.
    #[serde(default)]
    pub list: Option<Vec<Date>>,
}

impl Default for DateAxis {
    fn default() -> Self {
        DateAxis {
            start: None,
            end: None,
            step: None,
            max_cols: 30,
            list: None,
        }
    }
}

/// Cell display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Display {
    /// Dollars.
    #[default]
    Dollars,
    /// Percent of capital.
    Percent,
}

/// Grid layout.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridSpec {
    /// Row prices.
    #[serde(default)]
    pub prices: PriceAxis,
    /// Column dates.
    #[serde(default)]
    pub dates: DateAxis,
    /// Dollars or percent.
    #[serde(default)]
    pub display: Display,
}

fn insert_sorted<T: PartialOrd + Copy>(v: &mut Vec<T>, x: T, same: impl Fn(T, T) -> bool) -> usize {
    if let Some(i) = v.iter().position(|&y| same(x, y)) {
        return i;
    }
    let i = v.iter().position(|&y| y > x).unwrap_or(v.len());
    v.insert(i, x);
    i
}

/// Ascending row prices with the target inserted; returns the target's row.
pub fn price_axis(axis: &PriceAxis, th: &Thesis) -> Res<(Vec<f64>, usize)> {
    let even = |lo: f64, hi: f64, n: u32| -> Res<Vec<f64>> {
        if !(2..=500).contains(&n) || lo.partial_cmp(&hi) != Some(core::cmp::Ordering::Less) {
            return err("price range needs lo < hi and 2 to 500 steps");
        }
        let step = (hi - lo) / f64::from(n - 1);
        Ok((0..n).map(|i| lo + step * f64::from(i)).collect())
    };
    let mut v = match axis {
        PriceAxis::RangePct { lo, hi, steps } => even(*lo, *hi, *steps)?
            .iter()
            .map(|p| th.spot * (1.0 + p))
            .collect(),
        PriceAxis::Range { lo, hi, steps } => even(*lo, *hi, *steps)?,
        PriceAxis::List { values } => {
            let mut v = values.clone();
            v.sort_by(f64::total_cmp);
            v.dedup();
            v
        }
    };
    if v.is_empty() || v.iter().any(|p| !(p.is_finite() && *p > 0.0)) {
        return err("grid prices must be above zero");
    }
    let row = insert_sorted(&mut v, th.target, |a, b| {
        (a - b).abs() <= 1e-9 * a.abs().max(1.0)
    });
    Ok((v, row))
}

/// Ascending column dates with the target date inserted; returns its column.
pub fn date_axis(axis: &DateAxis, th: &Thesis, latest: Date) -> Res<(Vec<Date>, usize)> {
    let mut v = if let Some(list) = &axis.list {
        let mut l = list.clone();
        l.sort();
        l.dedup();
        l
    } else {
        let dc = th.day_count();
        let start = axis.start.unwrap_or(th.as_of);
        let end = axis.end.unwrap_or(latest);
        if end < start {
            return err("grid end date is before its start");
        }
        let total = dc.days(start, end).max(0);
        let step = match axis.step {
            Some(0) => return err("date step must be at least 1"),
            Some(s) => s as i32,
            None => {
                let cols = axis.max_cols.max(2) as i32;
                ((total + cols - 2) / (cols - 1)).max(1)
            }
        };
        let mut v = vec![start];
        let mut n = step;
        while n < total {
            v.push(dc.step(start, n));
            n += step;
            if v.len() > 1000 {
                return err("too many grid columns");
            }
        }
        if end > start {
            v.push(end);
        }
        v
    };
    if v.is_empty() {
        return err("grid needs at least one date");
    }
    let col = insert_sorted(&mut v, th.target_date, |a, b| a == b);
    Ok((v, col))
}

#[cfg(test)]
mod tests;
