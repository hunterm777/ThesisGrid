//! A full scenario in, every grid and summary out.

use crate::format::{money, money_round, short};
use crate::grid::{date_axis, price_axis, Display, GridSpec};
use crate::instrument::{Cell, InstrumentSpec, Position};
use crate::presets::{self, Venue};
use crate::sizing::{size, Mode};
use crate::summary::{pct, summarize, Summary};
use crate::thesis::Thesis;
use crate::{Date, Res};
use serde::{Deserialize, Serialize};

fn one() -> u32 {
    1
}

fn thousand() -> f64 {
    1000.0
}

/// Everything the page or the Python package sends to the core.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Format version.
    #[serde(default = "one")]
    pub v: u32,
    /// The thesis.
    pub thesis: Thesis,
    /// One entry per grid column.
    #[serde(default)]
    pub instruments: Vec<InstrumentSpec>,
    /// Comparison basis.
    #[serde(default)]
    pub mode: Mode,
    /// Capital (equal dollars), risk (equal max loss) or dollar delta (equal exposure).
    #[serde(default = "thousand")]
    pub budget: f64,
    /// Grid layout.
    #[serde(default)]
    pub grid: GridSpec,
}

/// One column's output.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ColumnResult {
    /// Unique label, e.g. "perp" or "perp 2".
    pub label: String,
    /// `spot`, `perp`, `option` or `binary`.
    pub kind: &'static str,
    /// Venue preset, where the instrument has one.
    pub venue: Option<Venue>,
    /// Why this column has no grid (e.g. "needs a stop").
    pub error: Option<String>,
    /// Caveats from presets and the model.
    pub notes: Vec<String>,
    /// Summary row.
    pub summary: Option<Summary>,
    /// `cells[row][col]`, rows ascending in price.
    pub cells: Vec<Vec<Cell>>,
}

/// Everything the page or the Python package gets back.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CompareResult {
    /// Row prices, ascending.
    pub prices: Vec<f64>,
    /// Column dates.
    pub dates: Vec<Date>,
    /// Row holding the target price.
    pub target_row: usize,
    /// Column holding the target date.
    pub target_col: usize,
    /// Comparison basis.
    pub mode: Mode,
    /// Budget the mode sizes to.
    pub budget: f64,
    /// Dollars or percent.
    pub display: Display,
    /// The one line shown above the grids.
    pub sizing_note: String,
    /// One per enabled instrument.
    pub columns: Vec<ColumnResult>,
    /// As-of date of the venue presets.
    pub presets_as_of: &'static str,
}

fn labels(specs: &[&InstrumentSpec]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    specs
        .iter()
        .map(|s| {
            let base = s
                .common()
                .0
                .map(str::to_string)
                .unwrap_or_else(|| s.kind().to_string());
            let n = seen.iter().filter(|x| **x == base).count();
            seen.push(base.clone());
            if n == 0 {
                base
            } else {
                format!("{base} {}", n + 1)
            }
        })
        .collect()
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// Runs a scenario.
pub fn compare(sc: &Scenario) -> Res<CompareResult> {
    let th = &sc.thesis;
    th.validate()?;
    let specs: Vec<&InstrumentSpec> = sc.instruments.iter().filter(|s| s.common().1).collect();
    let names = labels(&specs);
    let mut notes: Vec<Vec<String>> = vec![Vec::new(); specs.len()];
    let resolved: Vec<Res<Position>> = specs
        .iter()
        .zip(notes.iter_mut())
        .map(|(s, n)| Position::resolve(s, th, n))
        .collect();
    let latest = resolved
        .iter()
        .filter_map(|p| p.as_ref().ok().and_then(Position::expiry))
        .fold(th.target_date, Date::max);
    let (prices, target_row) = price_axis(&sc.grid.prices, th)?;
    let (dates, target_col) = date_axis(&sc.grid.dates, th, latest)?;
    let mut columns = Vec::with_capacity(specs.len());
    let mut note = format!("{}: ", sc.mode.label());
    note.push_str(&match sc.mode {
        Mode::EqualDollars | Mode::EqualMaxLoss => format!("{} each.", money_round(sc.budget)),
        Mode::EqualExposure => format!("{} of delta each at entry.", money_round(sc.budget)),
        Mode::Manual => "sizes typed by hand.".into(),
    });
    for (((spec, label), pos), mut notes) in specs.iter().zip(names).zip(resolved).zip(notes) {
        let kind = spec.kind();
        let venue = spec.venue();
        let sized = pos.and_then(|p| {
            let q = match spec.common().2 {
                Some(q) if q >= 0.0 => q,
                Some(_) => return crate::err("size must be zero or more"),
                None => size(&p, sc.mode, sc.budget)?,
            };
            Ok((p, q))
        });
        let (p, q) = match sized {
            Ok(x) => x,
            Err(e) => {
                let summary = None;
                let error = Some(e.0);
                columns.push(ColumnResult {
                    label,
                    kind,
                    venue,
                    error,
                    notes,
                    summary,
                    cells: vec![],
                });
                continue;
            }
        };
        if let Some(q) = spec.common().2 {
            note.push_str(&format!(
                " {} locked at {}.",
                capitalize(&label),
                short(q, 4)
            ));
        }
        if let Position::Perp(perp) = &p {
            let liq = perp.liq_price(perp.as_of());
            let liq = if liq > 0.0 {
                format!("with liquidation at {}", money(liq))
            } else {
                "with no liquidation above $0".into()
            };
            let name = capitalize(&label);
            note.push_str(&format!(
                " {name} sized at {}x {liq}.",
                short(perp.leverage, 2)
            ));
        }
        if sc.mode == Mode::EqualExposure && kind != "spot" && kind != "perp" {
            notes.push("Exposure is matched at entry and drifts as price and time move.".into());
        }
        let summary = summarize(&p, q, th.target, th.target_date, th.spot.max(th.target))?;
        let cells = prices
            .iter()
            .map(|&s| {
                dates
                    .iter()
                    .map(|&d| {
                        let mut c = p.cell(q, s, d);
                        c.pct = pct(c.pnl, summary.cost);
                        c
                    })
                    .collect()
            })
            .collect();
        columns.push(ColumnResult {
            label,
            kind,
            venue,
            error: None,
            notes,
            summary: Some(summary),
            cells,
        });
    }
    Ok(CompareResult {
        prices,
        dates,
        target_row,
        target_col,
        mode: sc.mode,
        budget: sc.budget,
        display: sc.grid.display,
        sizing_note: note,
        columns,
        presets_as_of: presets::AS_OF,
    })
}

/// JSON in, JSON out; errors come back as the message.
pub fn compare_json(input: &str) -> Result<String, String> {
    let sc: Scenario = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let out = compare(&sc).map_err(|e| e.0)?;
    serde_json::to_string(&out).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
