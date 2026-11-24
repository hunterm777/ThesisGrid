//! String-in, string-out entry points shared by the WASM and Python
//! wrappers, so neither wrapper holds any logic of its own.

use crate::presets::{self, Venue};
use crate::scenario::{CompareResult, Scenario};
use crate::{parse, strategies, url, Date};
use serde::Serialize;

fn json(v: &impl Serialize) -> String {
    serde_json::to_string(v).expect("output serializes")
}

fn scenario(text: &str) -> Result<Scenario, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

/// Runs a scenario: JSON [`Scenario`] in, JSON [`CompareResult`] out.
pub fn compare(text: &str) -> Result<String, String> {
    crate::compare_json(text)
}

/// Numbers per cell in [`compare_packed`]'s array, in this order: pnl, pct,
/// value, fees, funding, delta, gamma, theta, vega, liquidated (1 or 0).
pub const CELL_FIELDS: usize = 10;

/// The same result as [`compare`], split for the page: the JSON keeps every
/// field except the cells, which come as one flat array, column by column,
/// then row by row, then date by date, [`CELL_FIELDS`] numbers each. Moving
/// 3,000 cells this way skips formatting and parsing ~30,000 numbers as text.
pub fn compare_packed(text: &str) -> Result<(String, Vec<f64>), String> {
    let mut out = crate::compare(&scenario(text)?).map_err(|e| e.0)?;
    let n: usize = out
        .columns
        .iter()
        .map(|c| c.cells.iter().map(Vec::len).sum::<usize>())
        .sum();
    let mut flat = Vec::with_capacity(n * CELL_FIELDS);
    for col in &mut out.columns {
        for c in std::mem::take(&mut col.cells).iter().flatten() {
            let liq = if c.liquidated { 1.0 } else { 0.0 };
            flat.extend_from_slice(&[
                c.pnl, c.pct, c.value, c.fees, c.funding, c.delta, c.gamma, c.theta, c.vega, liq,
            ]);
        }
    }
    Ok((json(&out), flat))
}

/// Quick entry: the phrase and the as-of date (YYYY-MM-DD) in, JSON out.
pub fn parse(text: &str, as_of: &str) -> Result<String, String> {
    let d = Date::parse(as_of).map_err(|e| e.0)?;
    Ok(json(&parse::parse(text, d)))
}

#[derive(Serialize)]
struct Binaries {
    venue: Venue,
    fees: crate::fees::BinaryFees,
    notes: Vec<&'static str>,
}

#[derive(Serialize)]
struct Presets {
    as_of: &'static str,
    perps: Vec<presets::PerpPreset>,
    binaries: Vec<Binaries>,
    strategies: [&'static str; 13],
}

/// Every venue preset and the strategy names, as JSON.
pub fn presets() -> String {
    let binaries = [
        Venue::Kalshi,
        Venue::Polymarket,
        Venue::Hyperliquid,
        Venue::Custom,
    ]
    .into_iter()
    .map(|venue| {
        let (fees, notes) = presets::binary(venue);
        Binaries { venue, fees, notes }
    })
    .collect();
    json(&Presets {
        as_of: presets::AS_OF,
        perps: presets::all_perps(),
        binaries,
        strategies: strategies::STRATEGIES,
    })
}

/// The legs of a named strategy at the given strikes, as JSON.
pub fn strategy_legs(name: &str, strikes: &[f64]) -> Result<String, String> {
    strategies::template(name, strikes)
        .map(|l| json(&l))
        .map_err(|e| e.0)
}

/// A scenario as a page link.
pub fn to_url(text: &str, base: Option<&str>) -> Result<String, String> {
    Ok(url::to_url(&scenario(text)?, base))
}

/// A page link back to scenario JSON.
pub fn from_url(link: &str) -> Result<String, String> {
    url::from_url(link).map(|s| json(&s)).map_err(|e| e.0)
}

/// A scenario's grids and summary as CSV.
pub fn csv(text: &str) -> Result<String, String> {
    let sc = scenario(text)?;
    let r = crate::compare(&sc).map_err(|e| e.0)?;
    Ok(to_csv(&r))
}

fn opt(x: Option<f64>, none: &str) -> String {
    x.map_or_else(|| none.to_string(), |v| v.to_string())
}

/// CSV text for a result: one row per cell, then the summary rows.
pub fn to_csv(r: &CompareResult) -> String {
    let mut out = String::from(
        "instrument,price,date,pnl,pct_of_capital,value,fees,funding,delta,gamma,theta,vega,liquidated\n",
    );
    for c in &r.columns {
        for (row, &p) in c.cells.iter().zip(&r.prices) {
            for (cell, d) in row.iter().zip(&r.dates) {
                out.push_str(&format!(
                    "{},{p},{d},{},{},{},{},{},{},{},{},{},{}\n",
                    c.label,
                    cell.pnl,
                    cell.pct,
                    cell.value,
                    cell.fees,
                    cell.funding,
                    cell.delta,
                    cell.gamma,
                    cell.theta,
                    cell.vega,
                    cell.liquidated
                ));
            }
        }
    }
    out.push_str(
        "\ninstrument,qty,cost,max_loss,max_gain,breakevens,pnl_at_target,roc_at_target_pct,liquidation,error\n",
    );
    for c in &r.columns {
        match &c.summary {
            Some(s) => {
                let be: Vec<String> = s.breakevens.iter().map(f64::to_string).collect();
                out.push_str(&format!(
                    "{},{},{},{},{},{},{},{},{},\n",
                    c.label,
                    s.qty,
                    s.cost,
                    opt(s.max_loss, "unbounded"),
                    opt(s.max_gain, "unbounded"),
                    be.join(" "),
                    s.pnl_at_target,
                    s.roc_at_target,
                    opt(s.liquidation, "")
                ));
            }
            None => out.push_str(&format!(
                "{},,,,,,,,,{}\n",
                c.label,
                c.error.as_deref().unwrap_or("").replace(',', ";")
            )),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SC: &str = r#"{"thesis":{"spot":100,"target":120,"target_date":"2026-12-31",
        "as_of":"2026-11-24"},"mode":"equal_max_loss","budget":500,
        "instruments":[{"kind":"spot","stop":90},{"kind":"spot","label":"no stop"}]}"#;

    #[test]
    fn every_entry_point() {
        assert!(compare(SC).unwrap().contains("\"columns\""));
        assert!(parse("BTC to 120k by Dec 31", "2026-11-24")
            .unwrap()
            .contains("120000"));
        assert!(parse("x", "bad").is_err());
        let p = presets();
        assert!(p.contains("\"as_of\":\"2026-11-24\"") && p.contains("iron_condor"));
        assert!(strategy_legs("bull_call_spread", &[100.0, 110.0])
            .unwrap()
            .contains("call"));
        assert!(strategy_legs("nope", &[]).is_err());
        let link = to_url(SC, Some("https://x/")).unwrap();
        assert!(link.starts_with("https://x/#s="));
        let back: Scenario = serde_json::from_str(&from_url(&link).unwrap()).unwrap();
        assert_eq!(back, scenario(SC).unwrap());
        assert!(to_url("{", None).is_err());
        assert!(from_url("#s=!!").is_err());
        let csv = csv(SC).unwrap();
        assert!(csv.starts_with("instrument,price,date,pnl"));
        assert!(csv.contains("\nspot,"));
        assert!(csv.contains("no stop,,,,,,,,,needs a stop"));
        assert!(csv.contains("unbounded"));
        assert!(super::csv("{").is_err());
    }

    #[test]
    fn packed_matches_compare() {
        let sc = r#"{"thesis":{"underlying":"BTC","asset_class":"crypto","spot":95000,"target":110000,
            "target_date":"2026-12-31","as_of":"2026-11-24","iv":0.55},
            "instruments":[{"kind":"spot","stop":85000},{"kind":"perp","venue":"hyperliquid","leverage":20},
            {"kind":"binary","venue":"kalshi","strike":110000,"expiry":"2026-12-31","price":0.2}]}"#;
        let full: serde_json::Value = serde_json::from_str(&compare(sc).unwrap()).unwrap();
        let (head, flat) = compare_packed(sc).unwrap();
        let head: serde_json::Value = serde_json::from_str(&head).unwrap();
        let keys = [
            "pnl", "pct", "value", "fees", "funding", "delta", "gamma", "theta", "vega",
        ];
        let mut k = 0;
        for (i, col) in full["columns"].as_array().unwrap().iter().enumerate() {
            assert_eq!(head["columns"][i]["cells"].as_array().unwrap().len(), 0);
            for cell in col["cells"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|r| r.as_array().unwrap())
            {
                for (j, key) in keys.iter().enumerate() {
                    assert_eq!(flat[k + j], cell[key].as_f64().unwrap());
                }
                assert_eq!(flat[k + 9] == 1.0, cell["liquidated"].as_bool().unwrap());
                k += CELL_FIELDS;
            }
        }
        assert_eq!(k, flat.len());
        assert!(flat.iter().skip(9).step_by(CELL_FIELDS).any(|&l| l == 1.0));
    }
}
