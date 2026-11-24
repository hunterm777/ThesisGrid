use super::*;

fn today() -> Date {
    Date::parse("2026-11-24").unwrap()
}

#[test]
fn prd_phrases() {
    let p = parse("NVDA to $200 / share", today());
    assert_eq!(p.underlying.as_deref(), Some("NVDA"));
    assert_eq!(p.asset_class, Some(AssetClass::Stock));
    assert_eq!(p.target, Some(200.0));
    assert_eq!(p.target_date, None);
    assert_eq!(p.missing, vec!["target_date"]);

    let p = parse("NVDA to $200 by Mar 20", today());
    assert_eq!(p.target_date, Some(Date::parse("2027-03-20").unwrap()));
    assert_eq!(p.touch, Some(Touch::By));

    let p = parse("BTC to 120k by Dec 31", today());
    assert_eq!(p.asset_class, Some(AssetClass::Crypto));
    assert_eq!(p.target, Some(120_000.0));
    assert_eq!(p.target_date, Some(Date::parse("2026-12-31").unwrap()));

    let p = parse("gold below 3,400 by Jan 15", today());
    assert_eq!(p.underlying.as_deref(), Some("GOLD"));
    assert_eq!(p.asset_class, Some(AssetClass::Commodity));
    assert_eq!(p.direction, Some(Direction::Bearish));
    assert_eq!(p.target, Some(3400.0));
    assert_eq!(p.target_date, Some(Date::parse("2027-01-15").unwrap()));
    assert!(p.missing.is_empty());
}

#[test]
fn other_forms_and_blanks() {
    let p = parse("SPY above 700 on 2027-01-15", today());
    assert_eq!(p.asset_class, Some(AssetClass::Index));
    assert_eq!(p.direction, Some(Direction::Bullish));
    assert_eq!(p.touch, Some(Touch::At));
    let p = parse("eth to 1.2m by March 20 2028", today());
    assert_eq!(p.target, Some(1.2e6));
    assert_eq!(p.target_date, Some(Date::parse("2028-03-20").unwrap()));
    let p = parse("something vague", today());
    assert_eq!(
        p.missing,
        vec!["underlying", "asset_class", "target", "target_date"]
    );
    let p = parse("to the moon by never", today());
    assert_eq!(
        p.missing,
        vec!["underlying", "asset_class", "target", "target_date"]
    );
    let p = parse("Some Long Name to 5b by Feb 30", today());
    assert_eq!(p.asset_class, None);
    assert_eq!(p.target, Some(5e9));
    assert_eq!(p.target_date, None);
    assert_eq!(price("abc"), None);
    assert_eq!(price("-5"), None);
    assert_eq!(month("ma"), None);
    let p = parse("BTC to 100k by Nov 24", today());
    assert_eq!(p.target_date, Some(today()));
}
