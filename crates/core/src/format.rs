//! Number formatting for the one-line sizing note, shared by every build.

/// A contract price with at least two decimals ("0.30", "0.125").
pub fn price_text(p: f64) -> String {
    let two = format!("{p:.2}");
    if two.parse::<f64>() == Ok(p) {
        two
    } else {
        p.to_string()
    }
}

/// Like [`money`] but drops ".00" from whole-dollar amounts ("$500").
pub fn money_round(x: f64) -> String {
    let m = money(x);
    m.strip_suffix(".00").map(str::to_string).unwrap_or(m)
}

/// Dollars: cents below $1,000, whole dollars with commas from there.
pub fn money(x: f64) -> String {
    let neg = x < 0.0;
    let a = x.abs();
    let c = libm::round(a * 100.0) as u64;
    let (whole, cents) = if c >= 100_000 {
        (libm::round(a) as u64, None)
    } else {
        (c / 100, Some(c % 100))
    };
    let digits = whole.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    let sign = if neg && (whole > 0 || cents.unwrap_or(0) > 0) {
        "-"
    } else {
        ""
    };
    match cents {
        Some(c) => format!("{sign}${out}.{c:02}"),
        None => format!("{sign}${out}"),
    }
}

/// A short number: up to `dp` decimals, trailing zeros dropped.
pub fn short(x: f64, dp: usize) -> String {
    let s = format!("{x:.dp$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(money(84_210.4), "$84,210");
        assert_eq!(money(500.0), "$500.00");
        assert_eq!(money(-1_234_567.0), "-$1,234,567");
        assert_eq!(money(-0.001), "$0.00");
        assert_eq!(money(999.999), "$1,000");
        assert_eq!(short(3.10, 1), "3.1");
        assert_eq!(short(5.0, 2), "5");
        assert_eq!(short(12.0, 0), "12");
    }
}
