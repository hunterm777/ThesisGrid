use super::*;

fn th(asset: &str) -> Thesis {
    serde_json::from_str(&format!(
        r#"{{"spot":100,"target":112,"target_date":"2026-12-10","as_of":"2026-11-24",
            "asset_class":"{asset}"}}"#
    ))
    .unwrap()
}

#[test]
fn default_rows_include_target() {
    let (p, row) = price_axis(&PriceAxis::default(), &th("crypto")).unwrap();
    assert_eq!(p.len(), 26);
    assert_eq!(p[row], 112.0);
    assert!((p[0] - 70.0).abs() < 1e-12 && (p[25] - 130.0).abs() < 1e-12);
    let on = PriceAxis::Range {
        lo: 100.0,
        hi: 124.0,
        steps: 13,
    };
    let (p, row) = price_axis(&on, &th("crypto")).unwrap();
    assert_eq!((p.len(), p[row]), (13, 112.0));
    let list = PriceAxis::List {
        values: vec![120.0, 90.0, 90.0],
    };
    assert_eq!(
        price_axis(&list, &th("crypto")).unwrap().0,
        vec![90.0, 112.0, 120.0]
    );
    for bad in [
        PriceAxis::Range {
            lo: 1.0,
            hi: 1.0,
            steps: 5,
        },
        PriceAxis::RangePct {
            lo: -1.5,
            hi: 0.3,
            steps: 5,
        },
        PriceAxis::List { values: vec![] },
    ] {
        assert!(price_axis(&bad, &th("crypto")).is_err());
    }
}

#[test]
fn columns_fit_thirty_and_use_trading_days_for_stocks() {
    let t = th("crypto");
    let end = Date::parse("2027-03-19").unwrap();
    let (d, col) = date_axis(&DateAxis::default(), &t, end).unwrap();
    assert!(d.len() <= 31);
    assert_eq!(
        (d[0], *d.last().unwrap(), d[col]),
        (t.as_of, end, t.target_date)
    );
    let daily = DateAxis {
        step: Some(1),
        ..Default::default()
    };
    let (d, _) = date_axis(&daily, &t, Date::parse("2026-12-31").unwrap()).unwrap();
    assert_eq!(d.len(), 38);
    let s = th("stock");
    let (d, _) = date_axis(&daily, &s, Date::parse("2026-12-10").unwrap()).unwrap();
    assert!(d.iter().skip(1).all(|x| x.is_weekday()));
    assert_eq!(d.len(), 13);
    let list = DateAxis {
        list: Some(vec![t.as_of]),
        ..Default::default()
    };
    assert_eq!(date_axis(&list, &t, end).unwrap().0.len(), 2);
    let bad = DateAxis {
        step: Some(0),
        ..Default::default()
    };
    assert!(date_axis(&bad, &t, end).is_err());
    let back = DateAxis {
        end: Some(Date::parse("2026-01-01").unwrap()),
        ..Default::default()
    };
    assert!(date_axis(&back, &t, end).is_err());
    let empty = DateAxis {
        list: Some(vec![]),
        ..Default::default()
    };
    assert!(date_axis(&empty, &t, end).is_err());
}
