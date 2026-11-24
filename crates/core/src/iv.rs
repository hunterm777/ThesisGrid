//! Implied volatility from an entered price.

use crate::option::{price, Model, OptType};
use crate::{err, Res};

/// Volatility bounds searched, as fractions per year.
pub const VOL_MIN: f64 = 1e-4;
/// Upper bound.
pub const VOL_MAX: f64 = 10.0;

/// Implied volatility of a vanilla option. Option value rises monotonically
/// with volatility, so the root is unique; Newton steps with a bisection
/// fallback converge to 1e-12 in price.
pub fn option_iv(
    model: Model,
    ty: OptType,
    s: f64,
    k: f64,
    tau: f64,
    r: f64,
    target: f64,
) -> Res<f64> {
    if tau <= 0.0 {
        return err("cannot imply volatility at or after expiry");
    }
    let f = |v: f64| price(model, ty, s, k, tau, r, v).value - target;
    let (flo, fhi) = (f(VOL_MIN), f(VOL_MAX));
    if flo > 0.0 || fhi < 0.0 {
        return err(format!(
            "premium {target} is outside the model's range for strike {k}"
        ));
    }
    let (mut lo, mut hi) = (VOL_MIN, VOL_MAX);
    let mut v = 0.5;
    for _ in 0..100 {
        let g = price(model, ty, s, k, tau, r, v);
        let diff = g.value - target;
        if diff.abs() < 1e-12 {
            return Ok(v);
        }
        if diff > 0.0 {
            hi = v;
        } else {
            lo = v;
        }
        let step = if g.vega > 1e-12 {
            v - diff / g.vega
        } else {
            f64::NAN
        };
        v = if step > lo && step < hi {
            step
        } else {
            0.5 * (lo + hi)
        };
        if hi - lo < 1e-15 {
            break;
        }
    }
    Ok(v)
}

/// Implied volatility of a cash-or-nothing digital. A digital's value is not
/// monotone in volatility, so the range is scanned and the lowest root is
/// returned. `value(sigma)` prices the contract.
pub fn digital_iv(value: impl Fn(f64) -> f64, target: f64) -> Res<f64> {
    let g = |v: f64| value(v) - target;
    let steps = 2000;
    let ratio = (VOL_MAX / VOL_MIN).powf(1.0 / f64::from(steps));
    let mut a = VOL_MIN;
    let mut ga = g(a);
    for _ in 0..steps {
        let b = a * ratio;
        let gb = g(b);
        if ga == 0.0 {
            return Ok(a);
        }
        if (ga < 0.0) != (gb < 0.0) {
            return Ok(crate::math::bisect(a, b, g));
        }
        a = b;
        ga = gb;
    }
    err(format!("no volatility gives a contract price of {target}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_vol() {
        for ty in [OptType::Call, OptType::Put] {
            for model in [Model::BlackScholes, Model::Black76] {
                for v in [0.05, 0.3, 1.2, 3.0] {
                    let p = price(model, ty, 100.0, 110.0, 0.4, 0.03, v).value;
                    let got = option_iv(model, ty, 100.0, 110.0, 0.4, 0.03, p).unwrap();
                    assert!((got - v).abs() < 1e-8, "{ty:?} {model:?} {v} -> {got}");
                }
            }
        }
    }

    #[test]
    fn rejects_bad_inputs() {
        let bs = Model::BlackScholes;
        assert!(option_iv(bs, OptType::Call, 100.0, 90.0, 0.4, 0.0, 5.0).is_err());
        assert!(option_iv(bs, OptType::Call, 100.0, 90.0, 0.4, 0.0, 200.0).is_err());
        assert!(option_iv(bs, OptType::Call, 100.0, 90.0, 0.0, 0.0, 10.0).is_err());
        assert!(digital_iv(|_| 0.2, 0.9).is_err());
        assert_eq!(digital_iv(|v| v, VOL_MIN).unwrap(), VOL_MIN);
    }
}
