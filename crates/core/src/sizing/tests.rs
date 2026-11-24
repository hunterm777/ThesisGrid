use super::*;
use crate::instrument::InstrumentSpec;
use crate::thesis::Thesis;

fn th() -> Thesis {
    serde_json::from_str(
        r#"{"spot":95000,"target":110000,"target_date":"2026-12-31","as_of":"2026-11-24",
            "rate":0.04,"iv":0.55}"#,
    )
    .unwrap()
}

fn pos(json: &str) -> Position {
    let spec: InstrumentSpec = serde_json::from_str(json).unwrap();
    Position::resolve(&spec, &th(), &mut vec![]).unwrap()
}

#[test]
fn every_mode_hits_the_budget_to_the_cent() {
    let positions = [
        pos(r#"{"kind":"spot","fee_pct":0.001,"stop":85000}"#),
        pos(r#"{"kind":"perp","venue":"hyperliquid","leverage":5}"#),
        pos(
            r#"{"kind":"option","strategy":"bull_call_spread","strikes":[100000,115000],
                "expiry":"2026-12-31","fee_per_contract":0.5}"#,
        ),
        pos(
            r#"{"kind":"binary","venue":"kalshi","strike":110000,"expiry":"2026-12-31","price":0.31}"#,
        ),
    ];
    for mode in [Mode::EqualDollars, Mode::EqualMaxLoss, Mode::EqualExposure] {
        for p in &positions {
            let q = size(p, mode, 500.0).unwrap();
            let m = metric(p, mode, q).unwrap();
            assert!(
                m <= 500.0 + 1e-9 && 500.0 - m < 0.01,
                "{mode:?} {p:?} -> {m}"
            );
        }
    }
}

#[test]
fn equal_max_loss_long_call() {
    // PRD acceptance: contracts x premium + fees = $500.
    let p = pos(
        r#"{"kind":"option","strategy":"long_call","strikes":[100000],
        "expiry":"2026-12-31","fee_per_contract":0.65}"#,
    );
    let q = size(&p, Mode::EqualMaxLoss, 500.0).unwrap();
    assert!((p.entry_cost(q) - 500.0).abs() < 1e-9);
    assert!((p.max_loss(q).unwrap() - 500.0).abs() < 1e-9);
}

#[test]
fn modes_that_cannot_size() {
    let s = pos(r#"{"kind":"spot"}"#);
    assert_eq!(
        size(&s, Mode::EqualMaxLoss, 500.0).unwrap_err().0,
        "needs a stop"
    );
    let naked = pos(
        r#"{"kind":"option","legs":[{"kind":"call","side":"short","strike":100000}],
        "expiry":"2026-12-31"}"#,
    );
    assert!(size(&naked, Mode::EqualMaxLoss, 500.0).is_err());
    assert!(size(&naked, Mode::EqualDollars, 500.0).is_err());
    assert!(size(&s, Mode::EqualDollars, 0.0).is_err());
    let fixed = pos(r#"{"kind":"spot","fee_fixed":600}"#);
    assert!(size(&fixed, Mode::EqualDollars, 500.0).is_err());
    assert_eq!(size(&s, Mode::Manual, 500.0).unwrap(), 1.0);
    assert_eq!(metric(&s, Mode::Manual, 3.0).unwrap(), 3.0);
    for m in [
        Mode::EqualDollars,
        Mode::EqualMaxLoss,
        Mode::EqualExposure,
        Mode::Manual,
    ] {
        assert!(!m.label().is_empty());
    }
}
