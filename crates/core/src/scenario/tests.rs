use super::*;

const BTC: &str = r#"{
  "thesis": {"underlying":"BTC","asset_class":"crypto","spot":95000,"target":110000,
             "target_date":"2026-12-31","as_of":"2026-11-24","iv":0.5},
  "mode": "equal_max_loss", "budget": 500,
  "instruments": [
    {"kind":"spot","stop":85000},
    {"kind":"perp","venue":"hyperliquid","leverage":5,"funding_rate":0},
    {"kind":"option","strategy":"bull_call_spread","strikes":[100000,110000],
     "expiry":"2026-12-31","iv":0.5},
    {"kind":"binary","venue":"kalshi","strike":100000,"expiry":"2026-12-31","price":0.4},
    {"kind":"spot","enabled":false}
  ]
}"#;

fn run(json: &str) -> CompareResult {
    compare(&serde_json::from_str(json).unwrap()).unwrap()
}

#[test]
fn btc_hand_worked() {
    let r = run(BTC);
    assert_eq!(r.columns.len(), 4);
    let labels: Vec<&str> = r.columns.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["spot", "perp", "option", "binary"]);
    assert_eq!(r.prices.len(), 26);
    assert_eq!(r.prices[r.target_row], 110000.0);
    assert_eq!(r.dates[r.target_col], Date::parse("2026-12-31").unwrap());
    assert!(r.dates.len() <= 31);
    assert!(r.sizing_note.starts_with("Equal max loss: $500 each."));
    assert!(r
        .sizing_note
        .contains("Perp sized at 5x with liquidation at $"));
    for c in &r.columns {
        let s = c.summary.as_ref().unwrap();
        assert!(c.error.is_none(), "{}", c.label);
        assert!(
            (s.max_loss.unwrap() - 500.0).abs() < 1e-6,
            "{} {:?}",
            c.label,
            s.max_loss
        );
        assert_eq!(c.cells.len(), r.prices.len());
        assert_eq!(c.cells[0].len(), r.dates.len());
    }
    // Spot: long q coins, stop 10,000 below, no fees, so q = 500 / 10,000.
    let spot = &r.columns[0];
    let s = spot.summary.as_ref().unwrap();
    assert!((s.qty - 0.05).abs() < 1e-12);
    assert!((s.pnl_at_target - 0.05 * 15000.0).abs() < 1e-9);
    assert!((s.roc_at_target - 0.05 * 15000.0 / s.cost * 100.0).abs() < 1e-9);
    assert_eq!(s.breakevens, vec![95000.0]);
    let cell = &spot.cells[r.target_row][r.target_col];
    assert!((cell.pnl - s.pnl_at_target).abs() < 1e-9);
    // Bull call spread: max gain at or above 110,000 at expiry.
    let opt = r.columns[2].summary.as_ref().unwrap();
    assert!((opt.pnl_at_target - opt.max_gain.unwrap()).abs() < 1e-6);
    // Binary above 100,000 pays $1 at 110,000: P&L = q(1 - 0.40) minus fees.
    let bin = &r.columns[3];
    let b = bin.summary.as_ref().unwrap();
    assert!(b.pnl_at_target > 0.0 && b.pnl_at_target < b.qty * 0.6 + 1e-9);
    assert!(bin.cells[r.target_row][r.target_col].delta.abs() < 1e-9);
    // Perp: liquidation inside the grid shows a full margin loss.
    let perp = &r.columns[1];
    let liq = perp.summary.as_ref().unwrap().liquidation.unwrap();
    assert!(liq > 75000.0 && liq < 95000.0);
    assert!(perp.cells[0][0].liquidated);
    assert!((perp.cells[0][0].pnl + 500.0).abs() < 1e-6);
}

#[test]
fn errors_stay_in_their_column() {
    let json = BTC.replace(r#""stop":85000"#, r#""side":"long""#);
    let r = run(&json);
    assert_eq!(r.columns[0].error.as_deref(), Some("needs a stop"));
    assert!(r.columns[0].cells.is_empty());
    assert!(r.columns[1].error.is_none());
    let bad = BTC.replace(r#""kind":"perp","#, r#""kind":"perp","qty":-1,"#);
    assert_eq!(
        run(&bad).columns[1].error.as_deref(),
        Some("size must be zero or more")
    );
}

#[test]
fn locks_labels_and_modes() {
    let json = r#"{"thesis":{"spot":100,"target":120,"target_date":"2026-12-31",
        "as_of":"2026-11-24","asset_class":"stock"},"mode":"equal_exposure","budget":1000,
        "instruments":[{"kind":"spot","qty":3},{"kind":"spot"},
        {"kind":"option","strategy":"long_call","strikes":[110],"expiry":"2026-12-31"}]}"#;
    let r = run(json);
    assert_eq!(r.columns[0].label, "spot");
    assert_eq!(r.columns[1].label, "spot 2");
    assert!(r.sizing_note.contains("$1,000 of delta each at entry."));
    assert!(r.sizing_note.contains("Spot locked at 3."));
    assert_eq!(r.columns[0].summary.as_ref().unwrap().qty, 3.0);
    assert!((r.columns[1].summary.as_ref().unwrap().qty - 10.0).abs() < 1e-9);
    assert!(r.columns[2].notes.iter().any(|n| n.contains("drifts")));
    for mode in ["equal_dollars", "manual"] {
        let j = json.replace("equal_exposure", mode);
        let r = run(&j);
        assert!(r.sizing_note.starts_with(Mode::label(r.mode)));
    }
}

#[test]
fn json_round_trip_and_errors() {
    let out = compare_json(BTC).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["columns"].as_array().unwrap().len(), 4);
    assert_eq!(v["presets_as_of"], presets::AS_OF);
    assert!(compare_json("{").is_err());
    assert!(compare_json(r#"{"thesis":{},"oops":1}"#).is_err());
    let bad = BTC.replace(r#""spot":95000"#, r#""spot":-1"#);
    assert!(compare_json(&bad).is_err());
}
