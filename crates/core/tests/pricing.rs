//! M1 acceptance: closed-form values against independent mpmath references,
//! and put-call parity on every option fixture.

mod common;

use common::fixtures;
use serde_json::Value;
use thesisgrid_core::binary::digital;
use thesisgrid_core::math::exp;
use thesisgrid_core::option::{price, Greeks, Model, OptType};

fn cases() -> Vec<Value> {
    let text = std::fs::read_to_string(fixtures().join("pricing.json")).unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    v["cases"].as_array().unwrap().clone()
}

fn f(c: &Value, k: &str) -> f64 {
    c[k].as_f64().unwrap()
}

fn eval(c: &Value) -> Greeks {
    let (s, k, t, r, v) = (f(c, "s"), f(c, "k"), f(c, "tau"), f(c, "r"), f(c, "sigma"));
    let ty = if c["type"] == "call" {
        OptType::Call
    } else {
        OptType::Put
    };
    match c["model"].as_str().unwrap() {
        "black_scholes" => price(Model::BlackScholes, ty, s, k, t, r, v),
        "black76" => price(Model::Black76, ty, s, k, t, r, v),
        _ => digital(s, k, t, r, v),
    }
}

fn rel(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * b.abs().max(1.0)
}

#[test]
fn values_match_references_to_1e9() {
    let cases = cases();
    assert!(cases.len() >= 40);
    for c in &cases {
        let g = eval(c);
        assert!(
            rel(g.value, f(c, "value"), 1e-9),
            "value {c} got {}",
            g.value
        );
        for (name, got) in [
            ("delta", g.delta),
            ("gamma", g.gamma),
            ("vega", g.vega),
            ("dv_dtau", g.dv_dtau),
        ] {
            // The references are numerical derivatives; the core's are closed form.
            assert!(rel(got, f(c, name), 1e-8), "{name} {c} got {got}");
        }
    }
}

#[test]
fn textbook_cases() {
    let c = cases();
    assert!((f(&c[0], "value") - 4.7594).abs() < 5e-5, "Hull 15.6 call");
    assert!((f(&c[1], "value") - 0.8086).abs() < 5e-5, "Hull 15.6 put");
    assert!((f(&c[2], "value") - 1.1166).abs() < 5e-5, "Hull 18.7 put");
}

#[test]
fn put_call_parity_on_every_fixture() {
    for c in cases().iter().filter(|c| c["model"] != "digital") {
        let (s, k, t, r, v) = (f(c, "s"), f(c, "k"), f(c, "tau"), f(c, "r"), f(c, "sigma"));
        let model = if c["model"] == "black76" {
            Model::Black76
        } else {
            Model::BlackScholes
        };
        let call = price(model, OptType::Call, s, k, t, r, v).value;
        let put = price(model, OptType::Put, s, k, t, r, v).value;
        let fwd = match model {
            Model::BlackScholes => s - k * exp(-r * t),
            Model::Black76 => exp(-r * t) * (s - k),
        };
        assert!(rel(call - put, fwd, 1e-9), "parity {c}");
    }
}

#[test]
fn digital_at_the_money_tends_to_half_discount() {
    let r = 0.05;
    for t in [1e-4, 1e-6, 1e-8] {
        let v = digital(100.0, 100.0, t, r, 0.5).value;
        assert!((v - 0.5 * exp(-r * t)).abs() < 1e-3 * t.sqrt().max(1e-6) * 100.0);
    }
    let v = digital(100.0, 100.0, 1e-12, r, 0.5).value;
    assert!((v - 0.5 * exp(-r * 1e-12)).abs() < 1e-6);
}
