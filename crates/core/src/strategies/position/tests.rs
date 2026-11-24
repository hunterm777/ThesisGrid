use super::*;
use crate::math::exp;

fn th(asset: &str) -> Thesis {
    serde_json::from_str(&format!(
        r#"{{"spot":100,"target":120,"target_date":"2026-12-31","as_of":"2026-11-24",
            "rate":0.04,"iv":0.4,"asset_class":"{asset}"}}"#
    ))
    .unwrap()
}

fn strat(json: &str, asset: &str) -> Res<Strategy> {
    Strategy::resolve(&serde_json::from_str(json).unwrap(), &th(asset))
}

const EXP: &str = r#""expiry":"2026-12-31""#;

#[test]
fn long_call_costs_and_limits() {
    let s = strat(
        &format!(r#"{{"strategy":"long_call","strikes":[110],{EXP},"fee_per_contract":0.65}}"#),
        "stock",
    )
    .unwrap();
    let prem = s.legs[0].premium;
    assert!((s.entry_cost(2.0) - (2.0 * 100.0 * prem + 1.3)).abs() < 1e-9);
    assert!((s.max_loss(2.0).unwrap() - s.entry_cost(2.0)).abs() < 1e-9);
    assert_eq!(s.max_gain(2.0), None);
    // Entry cell: only the round-trip fees.
    let c = s.cell(1.0, 100.0, th("stock").as_of);
    assert!((c.pnl + 1.3).abs() < 1e-9);
    // At expiry no exit fee.
    let e = s.cell(1.0, 130.0, s.expiry());
    assert!((e.pnl - (100.0 * (20.0 - prem) - 0.65)).abs() < 1e-9);
    assert!(s.dollar_delta(1.0) > 0.0);
}

#[test]
fn spreads_condors_and_flies() {
    let bcs = strat(
        &format!(r#"{{"strategy":"bull_call_spread","strikes":[100,115],{EXP}}}"#),
        "crypto",
    )
    .unwrap();
    let debit = bcs.legs[0].premium - bcs.legs[1].premium;
    assert!((bcs.max_loss(1.0).unwrap() - debit).abs() < 1e-12);
    assert!((bcs.max_gain(1.0).unwrap() - (15.0 - debit)).abs() < 1e-12);
    let ic = strat(
        &format!(r#"{{"strategy":"iron_condor","strikes":[80,90,110,120],{EXP}}}"#),
        "crypto",
    )
    .unwrap();
    let credit = -ic.net_debit(1.0);
    assert!(credit > 0.0);
    assert!((ic.max_gain(1.0).unwrap() - credit).abs() < 1e-12);
    assert!((ic.entry_cost(1.0) - (10.0 - credit)).abs() < 1e-12);
    let fly = strat(
        &format!(r#"{{"strategy":"butterfly","strikes":[90,100,110],{EXP}}}"#),
        "crypto",
    )
    .unwrap();
    let paid = fly.net_debit(1.0);
    assert!((fly.max_gain(1.0).unwrap() - (10.0 - paid)).abs() < 1e-12);
    for name in [
        "bear_call_spread",
        "bull_put_spread",
        "bear_put_spread",
        "strangle",
    ] {
        let s = strat(
            &format!(r#"{{"strategy":"{name}","strikes":[95,105],{EXP}}}"#),
            "crypto",
        )
        .unwrap();
        assert!(s.max_loss(1.0).unwrap() > 0.0, "{name}");
    }
    let st = strat(
        &format!(r#"{{"strategy":"straddle","strikes":[100],{EXP}}}"#),
        "crypto",
    )
    .unwrap();
    assert_eq!(st.max_gain(1.0), None);
    let lp = strat(
        &format!(r#"{{"strategy":"long_put","strikes":[100],{EXP}}}"#),
        "crypto",
    )
    .unwrap();
    assert!((lp.max_gain(1.0).unwrap() - (100.0 - lp.legs[0].premium)).abs() < 1e-12);
}

#[test]
fn covered_call_and_cash_secured_put() {
    let cc = strat(
        &format!(r#"{{"strategy":"covered_call","strikes":[110],{EXP}}}"#),
        "stock",
    )
    .unwrap();
    let prem = cc.legs[1].premium;
    assert!((cc.entry_cost(1.0) - 100.0 * (100.0 - prem)).abs() < 1e-9);
    assert!((cc.max_gain(1.0).unwrap() - 100.0 * (10.0 + prem)).abs() < 1e-9);
    let csp = strat(
        &format!(r#"{{"strategy":"cash_secured_put","strikes":[95],{EXP}}}"#),
        "stock",
    )
    .unwrap();
    let p = csp.legs[0].premium;
    assert!((csp.entry_cost(1.0) - 100.0 * (95.0 - p)).abs() < 1e-9);
    let naked = strat(
        &format!(r#"{{"legs":[{{"kind":"call","side":"short","strike":110}}],{EXP}}}"#),
        "crypto",
    )
    .unwrap();
    assert_eq!(naked.max_loss(1.0), None);
    assert!(naked.entry_cost(1.0) < 0.0);
}

#[test]
fn premium_implies_vol_and_parity_holds() {
    let s = strat(
        &format!(
            r#"{{"legs":[{{"kind":"call","strike":105,"premium":4.0}},
        {{"kind":"put","strike":105,"premium":9.0,"side":"short"}}],{EXP}}}"#
        ),
        "crypto",
    )
    .unwrap();
    assert!(s.legs[0].iv > 0.0 && s.legs[1].iv > 0.0);
    let c = s.cell(1.0, 100.0, th("crypto").as_of);
    assert!(c.pnl.abs() < 1e-9);
    for model in ["black_scholes", "black76"] {
        let s = strat(
            &format!(
                r#"{{"legs":[{{"kind":"call","strike":105}},{{"kind":"put","strike":105,
            "side":"short"}}],{EXP},"model":"{model}"}}"#
            ),
            "crypto",
        )
        .unwrap();
        let t = th("crypto");
        let tau = t.tau(t.as_of, s.expiry());
        let synth = s.cell(1.0, 100.0, t.as_of).value;
        let fwd = if model == "black76" {
            exp(-0.04 * tau) * (100.0 - 105.0)
        } else {
            100.0 - 105.0 * exp(-0.04 * tau)
        };
        assert!((synth - fwd).abs() < 1e-9, "{model}");
    }
}

#[test]
fn rejects_bad_inputs() {
    let bad = |j: &str| strat(j, "crypto").is_err();
    assert!(bad(&format!(
        r#"{{"strategy":"long_call","strikes":[],{EXP}}}"#
    )));
    assert!(bad(&format!(
        r#"{{"strategy":"bull_call_spread","strikes":[110,100],{EXP}}}"#
    )));
    assert!(bad(&format!(
        r#"{{"strategy":"wheel","strikes":[110],{EXP}}}"#
    )));
    assert!(bad(r#"{"strategy":"long_call","strikes":[110]}"#));
    assert!(bad(
        r#"{"strategy":"long_call","strikes":[110],"expiry":"2026-01-01"}"#
    ));
    assert!(bad(&format!(r#"{{"legs":[],"strategy":"custom",{EXP}}}"#)));
    assert!(bad(&format!(r#"{{"legs":[{{"kind":"call"}}],{EXP}}}"#)));
    assert!(bad(&format!(
        r#"{{"legs":[{{"kind":"call","strike":1,"ratio":0}}],{EXP}}}"#
    )));
    assert!(bad(&format!(
        r#"{{"legs":[{{"kind":"call","strike":1,"iv":-1}}],{EXP}}}"#
    )));
    assert!(bad(&format!(
        r#"{{"legs":[{{"kind":"call","strike":100,"premium":500}}],{EXP}}}"#
    )));
    assert!(bad(&format!(
        r#"{{"strategy":"long_call","strikes":[110],{EXP},"multiplier":0}}"#
    )));
    let given = strat(
        &format!(r#"{{"legs":[{{"kind":"call","strike":100,"premium":5,"iv":0.3}}],{EXP}}}"#),
        "crypto",
    )
    .unwrap();
    assert_eq!(given.legs[0].iv, 0.3);
    assert_eq!(STRATEGIES.len(), 13);
}
