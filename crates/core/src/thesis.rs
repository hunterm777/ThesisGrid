//! The thesis: one underlying, one target, one date.

use crate::{err, Date, Res};
use serde::{Deserialize, Serialize};

/// What the underlying is. Sets the grid's time axis, the pricing clock and
/// the venue presets on offer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetClass {
    /// Crypto: 365 calendar days, trades 24/7.
    #[default]
    Crypto,
    /// Single stock: grid steps in trading days; options and binaries are
    /// priced on 365 calendar days, as listed-option IVs are quoted.
    Stock,
    /// Index or ETF: same as a single stock.
    Index,
    /// Commodity: day count per the user's setting.
    Commodity,
}

/// How time to expiry is counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DayCount {
    /// Calendar days over a 365-day year.
    #[default]
    Calendar365,
    /// Weekdays over a 252-day year (exchange holidays not modelled).
    Trading252,
}

impl DayCount {
    /// Days in the year for this count.
    pub fn year_days(self) -> f64 {
        match self {
            DayCount::Calendar365 => 365.0,
            DayCount::Trading252 => 252.0,
        }
    }

    /// Counted days from `a` to `b`.
    pub fn days(self, a: Date, b: Date) -> i32 {
        match self {
            DayCount::Calendar365 => a.days_to(b),
            DayCount::Trading252 => a.weekdays_to(b),
        }
    }

    /// Year fraction from `a` to `b`, floored at zero.
    pub fn year_frac(self, a: Date, b: Date) -> f64 {
        f64::from(self.days(a, b).max(0)) / self.year_days()
    }

    /// The date `n` counted days after `a`.
    pub fn step(self, a: Date, n: i32) -> Date {
        match self {
            DayCount::Calendar365 => a.add_days(n),
            DayCount::Trading252 => a.add_weekdays(n),
        }
    }
}

/// Whether the target must be reached by the date or on it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Touch {
    /// Touches the target any time before the date.
    By,
    /// Is at the target on the date.
    #[default]
    At,
}

/// Bullish or bearish.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Target above the current price.
    Bullish,
    /// Target below the current price.
    Bearish,
}

fn default_iv() -> f64 {
    0.5
}

/// One thesis. Every number is typed in by the user.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thesis {
    /// Free-text underlying name, e.g. "BTC" or "NVDA".
    #[serde(default)]
    pub underlying: String,
    /// Asset class.
    #[serde(default)]
    pub asset_class: AssetClass,
    /// Current price, typed by the user.
    pub spot: f64,
    /// Target price.
    pub target: f64,
    /// Target date.
    pub target_date: Date,
    /// Valuation date ("today").
    pub as_of: Date,
    /// By or at the target date.
    #[serde(default)]
    pub touch: Touch,
    /// Bullish or bearish; derived from target vs spot when omitted.
    #[serde(default)]
    pub direction: Option<Direction>,
    /// Continuously compounded risk-free rate.
    #[serde(default)]
    pub rate: f64,
    /// Implied volatility used where an instrument gives none.
    #[serde(default = "default_iv")]
    pub iv: f64,
    /// Day count for commodities (stocks, indices and crypto are fixed).
    #[serde(default)]
    pub commodity_day_count: DayCount,
}

impl Thesis {
    /// Day count for the grid's date axis: trading days (weekdays) for stocks
    /// and indices, calendar days for crypto, the user's choice for commodities.
    pub fn day_count(&self) -> DayCount {
        match self.asset_class {
            AssetClass::Stock | AssetClass::Index => DayCount::Trading252,
            AssetClass::Crypto => DayCount::Calendar365,
            AssetClass::Commodity => self.commodity_day_count,
        }
    }

    /// Clock for option and binary pricing (time to expiry and theta):
    /// 365 calendar days for stocks, indices and crypto, matching how
    /// exchanges and data vendors quote implied volatility; the user's
    /// choice for commodities.
    pub fn vol_clock(&self) -> DayCount {
        match self.asset_class {
            AssetClass::Stock | AssetClass::Index | AssetClass::Crypto => DayCount::Calendar365,
            AssetClass::Commodity => self.commodity_day_count,
        }
    }

    /// Stated or derived direction.
    pub fn direction(&self) -> Direction {
        self.direction.unwrap_or(if self.target >= self.spot {
            Direction::Bullish
        } else {
            Direction::Bearish
        })
    }

    /// Checks the inputs.
    pub fn validate(&self) -> Res<()> {
        let finite = [self.spot, self.target, self.rate, self.iv]
            .iter()
            .all(|x| x.is_finite());
        if !finite {
            return err("thesis numbers must be finite");
        }
        if self.spot <= 0.0 || self.target <= 0.0 {
            return err("current price and target must be above zero");
        }
        if self.iv <= 0.0 {
            return err("implied volatility must be above zero");
        }
        if self.target_date < self.as_of {
            return err("target date is before today");
        }
        Ok(())
    }

    /// Year fraction from `a` to `b` on this thesis's pricing clock.
    pub fn tau(&self, a: Date, b: Date) -> f64 {
        self.vol_clock().year_frac(a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn btc() -> Thesis {
        serde_json::from_str(
            r#"{"spot":95000,"target":110000,"target_date":"2026-12-31","as_of":"2026-11-24"}"#,
        )
        .unwrap()
    }

    #[test]
    fn defaults_and_day_counts() {
        let mut t = btc();
        assert_eq!(t.iv, 0.5);
        assert_eq!(t.day_count(), DayCount::Calendar365);
        assert_eq!(t.direction(), Direction::Bullish);
        assert_eq!(t.vol_clock(), DayCount::Calendar365);
        t.asset_class = AssetClass::Stock;
        assert_eq!(t.day_count(), DayCount::Trading252);
        assert_eq!(t.vol_clock(), DayCount::Calendar365);
        t.asset_class = AssetClass::Index;
        assert_eq!(t.day_count(), DayCount::Trading252);
        assert_eq!(t.vol_clock(), DayCount::Calendar365);
        t.asset_class = AssetClass::Commodity;
        t.commodity_day_count = DayCount::Trading252;
        assert_eq!(t.day_count(), DayCount::Trading252);
        assert_eq!(t.vol_clock(), DayCount::Trading252);
        t.target = 80_000.0;
        assert_eq!(t.direction(), Direction::Bearish);
        assert!(t.validate().is_ok());
        t.iv = 0.0;
        assert!(t.validate().is_err());
        t.iv = 0.5;
        t.spot = f64::NAN;
        assert!(t.validate().is_err());
        t.spot = -1.0;
        assert!(t.validate().is_err());
        t.spot = 1.0;
        t.target_date = Date::parse("2026-01-01").unwrap();
        assert!(t.validate().is_err());
    }

    #[test]
    fn year_fractions() {
        let a = Date::parse("2026-11-24").unwrap();
        let b = Date::parse("2026-12-24").unwrap();
        assert_eq!(DayCount::Calendar365.year_frac(a, b), 30.0 / 365.0);
        assert_eq!(DayCount::Trading252.year_frac(a, b), 22.0 / 252.0);
        assert_eq!(DayCount::Trading252.year_frac(b, a), 0.0);
        assert_eq!(
            DayCount::Trading252.step(a, 4),
            Date::parse("2026-11-30").unwrap()
        );
    }
}
