//! Calendar dates as whole days since 1970-01-01 (proleptic Gregorian).

use crate::{err, Res};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A calendar date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(pub i32);

impl Date {
    /// Builds a date from year, month (1-12) and day (1-31).
    pub fn from_ymd(y: i32, m: u32, d: u32) -> Res<Date> {
        if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
            return err(format!("invalid date {y:04}-{m:02}-{d:02}"));
        }
        // Howard Hinnant's days_from_civil.
        let y = if m <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let mp = (m as i32 + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d as i32 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Ok(Date(era * 146_097 + doe - 719_468))
    }

    /// Year, month, day.
    pub fn ymd(self) -> (i32, u32, u32) {
        let z = self.0 + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = yoe + era * 400 + i32::from(m <= 2);
        (y, m, d)
    }

    /// Parses `YYYY-MM-DD`.
    pub fn parse(s: &str) -> Res<Date> {
        let parts: Vec<&str> = s.trim().split('-').collect();
        if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
            return err(format!("dates are YYYY-MM-DD, got \"{s}\""));
        }
        let num = |p: &str| {
            p.parse::<u32>()
                .map_err(|_| crate::Error(format!("bad date \"{s}\"")))
        };
        Date::from_ymd(num(parts[0])? as i32, num(parts[1])?, num(parts[2])?)
    }

    /// Day of week, 0 = Monday ... 6 = Sunday.
    pub fn weekday(self) -> u32 {
        // 1970-01-01 was a Thursday (3).
        (self.0 + 3).rem_euclid(7) as u32
    }

    /// Monday to Friday.
    pub fn is_weekday(self) -> bool {
        self.weekday() < 5
    }

    /// This date plus `n` calendar days.
    pub fn add_days(self, n: i32) -> Date {
        Date(self.0 + n)
    }

    /// Calendar days from `self` to `other` (negative if `other` is earlier).
    pub fn days_to(self, other: Date) -> i32 {
        other.0 - self.0
    }

    /// Weekdays in `(self, other]`; negative when `other` is earlier.
    /// Exchange holidays are not modelled.
    pub fn weekdays_to(self, other: Date) -> i32 {
        if other < self {
            return -other.weekdays_to(self);
        }
        let days = other.0 - self.0;
        let mut count = days / 7 * 5;
        let mut d = self.add_days(days / 7 * 7);
        while d < other {
            d = d.add_days(1);
            if d.is_weekday() {
                count += 1;
            }
        }
        count
    }

    /// The `n`-th weekday after this date (`n >= 0`); `n == 0` is the date itself.
    pub fn add_weekdays(self, n: i32) -> Date {
        let mut d = self;
        let mut left = n;
        while left > 0 {
            d = d.add_days(1);
            if d.is_weekday() {
                left -= 1;
            }
        }
        d
    }
}

/// Days in a month.
pub fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (y, m, d) = self.ymd();
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

impl Serialize for Date {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Date::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_weekdays() {
        let d = Date::parse("2026-11-24").unwrap();
        assert_eq!(d.to_string(), "2026-11-24");
        assert_eq!(d.weekday(), 1); // Tuesday
        assert_eq!(Date::parse("1970-01-01").unwrap().0, 0);
        assert_eq!(Date::parse("2000-02-29").unwrap().ymd(), (2000, 2, 29));
        assert!(Date::parse("2026-02-29").is_err());
        assert!(Date::parse("2026/02/01").is_err());
        assert!(Date::parse("2026-xx-01").is_err());
        assert_eq!(Date::parse("1969-12-31").unwrap().0, -1);
    }

    #[test]
    fn counts_weekdays() {
        let fri = Date::parse("2026-11-27").unwrap();
        let mon = Date::parse("2026-11-30").unwrap();
        assert_eq!(fri.weekdays_to(mon), 1);
        assert_eq!(mon.weekdays_to(fri), -1);
        assert_eq!(fri.add_weekdays(1), mon);
        assert_eq!(fri.add_weekdays(0), fri);
        let a = Date::parse("2026-11-24").unwrap();
        assert_eq!(a.weekdays_to(a.add_days(28)), 20);
        assert_eq!(a.days_to(a.add_days(28)), 28);
    }
}
