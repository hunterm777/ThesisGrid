use super::*;

fn th(asset: &str) -> Thesis {
    serde_json::from_str(&format!(
        r#"{{"spot":100000,"target":120000,"target_date":"2026-12-31","as_of":"2026-11-24",
            "asset_class":"{asset}"}}"#
    ))
    .unwrap()
}

fn spec(venue: Venue) -> PerpSpec {
    PerpSpec {
        venue,
        leverage: Some(5.0),
        ..Default::default()
    }
}

#[test]
fn hyperliquid_taker_fee_and_hourly_funding() {
    let mut notes = vec![];
    let s = PerpSpec {
        funding_rate: Some(0.0001),
        ..spec(Venue::Hyperliquid)
    };
    let p = Perp::resolve(&s, &th("crypto"), &mut notes).unwrap();
    // PRD acceptance: $10,000 notional at 0.045% = $4.50 per side.
    let q = 0.1;
    assert!((p.cell(q, 100_000.0, th("crypto").as_of).fees - 9.0).abs() < 1e-9);
    assert!((p.entry_cost(q) - (2000.0 + 4.5)).abs() < 1e-9);
    // Paid hourly at one eighth of the 8-hour rate.
    assert_eq!(p.funding_per_payment, 0.0001 / 8.0);
    let d1 = th("crypto").as_of.add_days(1);
    assert_eq!(p.payments(d1), 24.0);
    assert!((p.funding(q, d1) - 0.0001 * 3.0 * 10_000.0).abs() < 1e-9);
    // MM = half of the initial margin at 40x max = 1.25%.
    assert_eq!(p.mmr, 0.0125);
    let liq = p.liq_price(th("crypto").as_of);
    assert!((liq - 100_000.0 * (1.0 - 0.2) / (1.0 - 0.0125)).abs() < 1e-6);
}

#[test]
fn caps_and_floors() {
    let mut n = vec![];
    let k = PerpSpec {
        funding_rate: Some(0.00009),
        ..spec(Venue::Kalshi)
    };
    let p = Perp::resolve(&k, &th("crypto"), &mut n).unwrap();
    // PRD acceptance: Kalshi funding below 0.01% absolute rounds to zero.
    assert_eq!(p.funding_per_payment, 0.0);
    let k = PerpSpec {
        funding_rate: Some(0.05),
        ..spec(Venue::Kalshi)
    };
    assert_eq!(
        Perp::resolve(&k, &th("crypto"), &mut n)
            .unwrap()
            .funding_per_payment,
        0.02
    );
    let pm = PerpSpec {
        funding_rate: Some(-1.0),
        ..spec(Venue::Polymarket)
    };
    let p = Perp::resolve(&pm, &th("crypto"), &mut n).unwrap();
    assert_eq!(p.funding_per_payment, -0.04);
    assert_eq!(p.mmr, 0.025);
    let no_cap = PerpSpec {
        funding_cap: Some(0.0),
        ..pm
    };
    assert_eq!(
        Perp::resolve(&no_cap, &th("crypto"), &mut n)
            .unwrap()
            .funding_per_payment,
        -0.125
    );
    let stock = Perp::resolve(&spec(Venue::Kalshi), &th("stock"), &mut n).unwrap();
    assert_eq!(stock.payments(th("stock").as_of.add_days(3)), 3.0);
    assert_eq!(stock.mmr, 0.0);
}

#[test]
fn liquidation_and_short() {
    let mut n = vec![];
    let s = PerpSpec {
        side: Side::Short,
        maintenance_margin: Some(0.05),
        ..spec(Venue::Custom)
    };
    let p = Perp::resolve(&s, &th("crypto"), &mut n).unwrap();
    let d = th("crypto").as_of;
    let liq = p.liq_price(d);
    assert!((liq - 100_000.0 * 1.2 / 1.05).abs() < 1e-6);
    let c = p.cell(1.0, liq + 1.0, d);
    assert!(c.liquidated && (c.pnl + 20_000.0).abs() < 1e-9);
    assert!(!p.cell(1.0, liq - 1.0, d).liquidated);
    assert_eq!(p.max_loss(1.0), Some(20_000.0));
    assert!((p.max_gain(1.0, d).unwrap() - 100_000.0).abs() < 1e-9);
    assert_eq!(p.dollar_delta(2.0), -200_000.0);
    let long = Perp::resolve(&spec(Venue::Custom), &th("crypto"), &mut n).unwrap();
    assert_eq!(long.max_gain(1.0, d), None);
    // Equity at the liquidation price equals maintenance margin.
    let eq = |p: &Perp, s: f64| p.entry_cost(1.0) + p.side.sign() * (s - p.entry);
    assert!((eq(&p, liq) - 0.05 * liq).abs() < 1e-6);
}

#[test]
fn rejects_bad_inputs() {
    let mut n = vec![];
    let t = th("crypto");
    let bad = |s: PerpSpec| Perp::resolve(&s, &t, &mut vec![]).is_err();
    assert!(bad(PerpSpec {
        leverage: Some(7.0),
        ..spec(Venue::Kalshi)
    }));
    assert!(bad(PerpSpec {
        leverage: Some(0.0),
        ..spec(Venue::Custom)
    }));
    assert!(bad(PerpSpec {
        maintenance_margin: Some(1.0),
        ..spec(Venue::Custom)
    }));
    assert!(bad(PerpSpec {
        funding_interval_hours: Some(0.0),
        ..spec(Venue::Custom)
    }));
    assert!(bad(PerpSpec {
        entry: Some(-1.0),
        ..spec(Venue::Custom)
    }));
    let maker = PerpSpec {
        liquidity: Liquidity::Maker,
        ..spec(Venue::Polymarket)
    };
    let p = Perp::resolve(&maker, &t, &mut n).unwrap();
    assert!((p.entry_cost(1.0) - (20_000.0 + 12.5)).abs() < 1e-9);
}
