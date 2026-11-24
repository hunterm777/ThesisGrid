use super::*;
use crate::fees::{kalshi::KalshiFees, polymarket::PolymarketFees};

fn th() -> Thesis {
    serde_json::from_str(
        r#"{"spot":95000,"target":110000,"target_date":"2026-12-31","as_of":"2026-11-24",
            "rate":0.04,"iv":0.55}"#,
    )
    .unwrap()
}

fn spec(json: &str) -> BinarySpec {
    serde_json::from_str(json).unwrap()
}

#[test]
fn implied_vol_reprices_entry() {
    let t = th();
    let s = spec(r#"{"venue":"kalshi","strike":110000,"expiry":"2026-12-31","price":0.2}"#);
    let b = Binary::resolve(&s, &t, &mut vec![]).unwrap();
    let c = b.cell(100.0, 95_000.0, t.as_of);
    // Entry cell: model value equals the price paid, so P&L is just fees.
    assert!((c.pnl + c.fees).abs() < 1e-9);
    let open = KalshiFees::default().trade(0.2, 100.0, Liquidity::Taker);
    assert!((b.entry_cost(100.0) - (20.0 + open)).abs() < 1e-12);
    let win = b.cell(100.0, 111_000.0, b.expiry);
    assert!((win.pnl - (100.0 - 20.0 - open)).abs() < 1e-9);
    assert!((b.max_gain(100.0).unwrap() - win.pnl).abs() < 1e-9);
    assert!((b.max_loss(100.0).unwrap() + b.cell(100.0, 1.0, b.expiry).pnl).abs() < 1e-9);
    assert!(b.dollar_delta(100.0) > 0.0);
}

#[test]
fn at_the_money_digital_tends_to_half_discounted() {
    // PRD acceptance: ATM cash-or-nothing, T -> 0: value -> 0.5 e^(-rT).
    for tau in [1e-6, 1e-9, 1e-12] {
        let g = digital(100.0, 100.0, tau, 0.05, 0.4);
        assert!((g.value - 0.5 * exp(-0.05 * tau)).abs() < 1e-4, "{tau}");
    }
    let g = digital(100.0, 100.0, 1e-12, 0.05, 0.4);
    assert!((g.value - 0.5).abs() < 1e-6);
}

#[test]
fn greeks_match_finite_differences() {
    let t = th();
    for side in ["yes", "no"] {
        for shape in [
            r#""strike":100000"#,
            r#""contract":"range","lower":90000,"upper":105000"#,
        ] {
            let s = spec(&format!(
                r#"{{"side":"{side}",{shape},"expiry":"2026-12-31","price":0.4,"iv":0.5}}"#
            ));
            let b = Binary::resolve(&s, &t, &mut vec![]).unwrap();
            let v = |s: f64, tau: f64, sig: f64| b.greeks(s, tau, sig).value;
            let (s0, tau, sig) = (97_000.0, 0.1, 0.5);
            let g = b.greeks(s0, tau, sig);
            let h = 1.0;
            let fd_delta = (v(s0 + h, tau, sig) - v(s0 - h, tau, sig)) / (2.0 * h);
            let fd_gamma =
                (v(s0 + h, tau, sig) - 2.0 * v(s0, tau, sig) + v(s0 - h, tau, sig)) / 1.0;
            let fd_vega = (v(s0, tau, sig + 1e-6) - v(s0, tau, sig - 1e-6)) / 2e-6;
            let fd_tau = (v(s0, tau + 1e-7, sig) - v(s0, tau - 1e-7, sig)) / 2e-7;
            assert!((g.delta - fd_delta).abs() < 1e-10, "delta {side} {shape}");
            assert!((g.gamma - fd_gamma).abs() < 1e-12, "gamma {side} {shape}");
            assert!((g.vega - fd_vega).abs() < 1e-7, "vega {side} {shape}");
            assert!((g.dv_dtau - fd_tau).abs() < 1e-6, "theta {side} {shape}");
        }
    }
}

#[test]
fn settlement_rules_and_errors() {
    let t = th();
    let r = spec(
        r#"{"contract":"range","lower":90000,"upper":100000,"expiry":"2026-12-31",
        "price":0.3,"side":"no","fees":{"model":"polymarket","rate":0.05}}"#,
    );
    let b = Binary::resolve(&r, &t, &mut vec![]).unwrap();
    assert_eq!(b.greeks(90_000.0, 0.0, 0.5).value, 0.0);
    assert_eq!(b.greeks(100_000.0, 0.0, 0.5).value, 1.0);
    let pm = PolymarketFees {
        rate: 0.05,
        exponent: 1.0,
        taker_only: true,
    };
    assert!((b.entry_cost(100.0) - 30.0 - pm.trade(0.3, 100.0, Liquidity::Taker)).abs() < 1e-12);
    let z = digital(0.0, 100.0, 1.0, 0.0, 0.5);
    assert_eq!(z.value, 0.0);
    let bad = |j: &str| Binary::resolve(&spec(j), &t, &mut vec![]).is_err();
    assert!(bad(r#"{"expiry":"2026-12-31","price":0.3}"#));
    assert!(bad(
        r#"{"contract":"range","lower":5,"upper":4,"expiry":"2026-12-31","price":0.3}"#
    ));
    assert!(bad(r#"{"strike":-1,"expiry":"2026-12-31","price":0.3}"#));
    assert!(bad(r#"{"strike":1,"expiry":"2026-12-31","price":1.0}"#));
    assert!(bad(r#"{"strike":1,"expiry":"2026-01-01","price":0.5}"#));
    assert!(bad(
        r#"{"strike":1,"expiry":"2026-12-31","price":0.5,"iv":0}"#
    ));
    let mut notes = vec![];
    let far = spec(r#"{"strike":95000,"expiry":"2026-12-31","price":0.999,"venue":"polymarket"}"#);
    let b = Binary::resolve(&far, &t, &mut notes).unwrap();
    assert_eq!(b.iv, t.iv);
    assert_eq!(notes.len(), 2);
}
