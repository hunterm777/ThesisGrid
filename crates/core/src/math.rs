//! Numeric helpers. Everything goes through `libm` for cross-target determinism.

/// 1 / sqrt(2 pi).
pub const INV_SQRT_2PI: f64 = 0.398_942_280_401_432_7;

/// Standard normal cumulative distribution, `0.5 * erfc(-x / sqrt 2)`.
pub fn norm_cdf(x: f64) -> f64 {
    0.5 * libm::erfc(-x * core::f64::consts::FRAC_1_SQRT_2)
}

/// Standard normal density.
pub fn norm_pdf(x: f64) -> f64 {
    INV_SQRT_2PI * libm::exp(-0.5 * x * x)
}

/// `e^x`.
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

/// Natural log.
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

/// Square root.
pub fn sqrt(x: f64) -> f64 {
    libm::sqrt(x)
}

/// Rounds `x` up to a multiple of `step` (e.g. 0.01 for cents).
///
/// A relative guard of 1e-9 steps absorbs binary floating-point noise, so
/// `ceil_to(1.75, 0.01)` is 1.75 and not 1.76.
pub fn ceil_to(x: f64, step: f64) -> f64 {
    let n = x / step;
    let r = libm::round(n);
    let k = if (n - r).abs() < 1e-9 {
        r
    } else {
        libm::ceil(n)
    };
    scale(k, step)
}

/// Rounds `x` down to a multiple of `step`, with the same noise guard.
pub fn floor_to(x: f64, step: f64) -> f64 {
    let n = x / step;
    let r = libm::round(n);
    let k = if (n - r).abs() < 1e-9 {
        r
    } else {
        libm::floor(n)
    };
    scale(k, step)
}

/// `k * step`, divided by the whole number `1 / step` when there is one, so
/// 125999 steps of 0.00001 is exactly the double nearest 1.25999.
fn scale(k: f64, step: f64) -> f64 {
    let inv = libm::round(1.0 / step);
    if inv >= 1.0 && (1.0 / step - inv).abs() < 1e-9 {
        k / inv
    } else {
        k * step
    }
}

/// Finds the root of `f` on `[lo, hi]` by bisection. `f(lo)` and `f(hi)`
/// must differ in sign.
pub fn bisect(mut lo: f64, mut hi: f64, f: impl Fn(f64) -> f64) -> f64 {
    let mut flo = f(lo);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if mid <= lo || mid >= hi {
            break;
        }
        let fm = f(mid);
        if fm == 0.0 {
            return mid;
        }
        if (fm < 0.0) == (flo < 0.0) {
            lo = mid;
            flo = fm;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdf_reference_points() {
        assert_eq!(norm_cdf(0.0), 0.5);
        assert!((norm_cdf(1.0) - 0.841_344_746_068_542_9).abs() < 1e-15);
        assert!((norm_cdf(-1.959_963_984_540_054) - 0.025).abs() < 1e-15);
        assert!((norm_cdf(-8.0) - 6.220_960_574_271_785e-16).abs() < 1e-28);
    }

    #[test]
    fn rounding_guards_float_noise() {
        assert_eq!(ceil_to(0.07 * 100.0 * 0.25, 0.01), 1.75);
        assert_eq!(ceil_to(1.751, 0.01), 1.76);
        assert_eq!(floor_to(1.259_999, 0.000_01), 1.259_99);
    }

    #[test]
    fn bisect_finds_root() {
        let r = bisect(0.0, 2.0, |x| x * x - 2.0);
        assert!((r - core::f64::consts::SQRT_2).abs() < 1e-12);
    }
}
