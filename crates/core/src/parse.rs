//! Quick entry: reads a plain thesis such as "BTC to 120k by Dec 31".
//!
//! Anything it cannot read is left out and listed in `missing`, never
//! guessed. The current price is never parsed: there is no price feed.

use crate::thesis::{AssetClass, Direction, Touch};
use crate::Date;
use serde::Serialize;

/// What the parser read.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Parsed {
    /// Underlying, upper-cased.
    pub underlying: Option<String>,
    /// Asset class from the underlying's name.
    pub asset_class: Option<AssetClass>,
    /// Target price.
    pub target: Option<f64>,
    /// Target date.
    pub target_date: Option<Date>,
    /// "above" or "below"; "to" leaves it to the current price.
    pub direction: Option<Direction>,
    /// "by" (touch before) or "on"/"at" (on the date).
    pub touch: Option<Touch>,
    /// Fields the text did not give.
    pub missing: Vec<&'static str>,
}

const CRYPTO: [&str; 14] = [
    "BTC", "BITCOIN", "ETH", "ETHER", "ETHEREUM", "SOL", "SOLANA", "XRP", "DOGE", "HYPE", "BNB",
    "ADA", "AVAX", "LINK",
];
const COMMODITY: [&str; 11] = [
    "GOLD",
    "SILVER",
    "OIL",
    "WTI",
    "BRENT",
    "PLATINUM",
    "PALLADIUM",
    "COPPER",
    "NATGAS",
    "XAU",
    "XAG",
];
const INDEX: [&str; 12] = [
    "SPX", "SPY", "QQQ", "NDX", "DIA", "IWM", "USA500", "US100", "US30", "SP500", "NASDAQ", "DOW",
];
const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

/// Asset class from a name: known crypto, commodity and index names, and
/// any other ticker-shaped name (1 to 5 letters) as a stock.
pub fn asset_class(name: &str) -> Option<AssetClass> {
    let n = name.to_ascii_uppercase();
    if CRYPTO.contains(&n.as_str()) {
        Some(AssetClass::Crypto)
    } else if COMMODITY.contains(&n.as_str()) {
        Some(AssetClass::Commodity)
    } else if INDEX.contains(&n.as_str()) {
        Some(AssetClass::Index)
    } else if (1..=5).contains(&n.len()) && n.chars().all(|c| c.is_ascii_alphabetic()) {
        Some(AssetClass::Stock)
    } else {
        None
    }
}

/// Reads "$200", "120k", "3,400", "1.2m".
pub fn price(tok: &str) -> Option<f64> {
    let t = tok
        .trim_start_matches('$')
        .replace(',', "")
        .to_ascii_lowercase();
    let (num, mult) = match t.chars().last()? {
        'k' => (&t[..t.len() - 1], 1e3),
        'm' => (&t[..t.len() - 1], 1e6),
        'b' => (&t[..t.len() - 1], 1e9),
        _ => (&t[..], 1.0),
    };
    let x: f64 = num.parse().ok()?;
    (x > 0.0 && x.is_finite()).then_some(x * mult)
}

fn month(tok: &str) -> Option<u32> {
    let t = tok.to_ascii_lowercase();
    if t.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|m| t.starts_with(m))
        .map(|i| i as u32 + 1)
}

/// Reads "Mar 20", "March 20 2027" or "2027-03-20"; a date without a year is
/// the next one on or after `as_of`.
fn date(toks: &[&str], as_of: Date) -> Option<Date> {
    let first = toks.first()?;
    if let Ok(d) = Date::parse(first) {
        return Some(d);
    }
    let m = month(first)?;
    let day: u32 = toks.get(1)?.trim_end_matches(',').parse().ok()?;
    if let Some(y) = toks.get(2).and_then(|y| y.parse::<i32>().ok()) {
        return Date::from_ymd(y, m, day).ok();
    }
    let (y, _, _) = as_of.ymd();
    let d = Date::from_ymd(y, m, day).ok()?;
    if d >= as_of {
        Some(d)
    } else {
        Date::from_ymd(y + 1, m, day).ok()
    }
}

/// Parses a quick-entry thesis.
pub fn parse(text: &str, as_of: Date) -> Parsed {
    let toks: Vec<&str> = text.split_whitespace().collect();
    let lower: Vec<String> = toks.iter().map(|t| t.to_ascii_lowercase()).collect();
    let mut out = Parsed::default();
    let kw = lower
        .iter()
        .position(|t| matches!(t.as_str(), "to" | "above" | "below"));
    if let Some(k) = kw {
        if k > 0 {
            let name = toks[..k].join(" ").to_ascii_uppercase();
            out.asset_class = asset_class(&name);
            out.underlying = Some(name);
        }
        out.direction = match lower[k].as_str() {
            "above" => Some(Direction::Bullish),
            "below" => Some(Direction::Bearish),
            _ => None,
        };
        out.target = toks.get(k + 1).and_then(|t| price(t));
    }
    if let Some(i) = lower
        .iter()
        .position(|t| matches!(t.as_str(), "by" | "on" | "at"))
    {
        out.target_date = date(&toks[i + 1..], as_of);
        if out.target_date.is_some() {
            out.touch = Some(if lower[i] == "by" {
                Touch::By
            } else {
                Touch::At
            });
        }
    }
    for (missing, name) in [
        (out.underlying.is_none(), "underlying"),
        (out.asset_class.is_none(), "asset_class"),
        (out.target.is_none(), "target"),
        (out.target_date.is_none(), "target_date"),
    ] {
        if missing {
            out.missing.push(name);
        }
    }
    out
}

#[cfg(test)]
mod tests;
