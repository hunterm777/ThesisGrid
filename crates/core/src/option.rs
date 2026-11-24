//! Black-Scholes (options on spot) and Black-76 (options on futures), with Greeks.
//!
//! No dividends. Listed equity options are American-style; ThesisGrid prices
//! them as European and does not model early exercise.

use crate::math::{exp, ln, norm_cdf, norm_pdf, sqrt};
use serde::{Deserialize, Serialize};

/// Call or put.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OptType {
    /// Right to buy.
    Call,
    /// Right to sell.
    Put,
}

/// Pricing model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Model {
    /// Black-Scholes on the spot price.
    #[default]
    BlackScholes,
    /// Black-76 on a futures price.
    Black76,
}

/// Value and sensitivities of one option on one unit of underlying.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Greeks {
    /// Option value.
    pub value: f64,
    /// dV/dS (dV/dF for Black-76).
    pub delta: f64,
    /// d2V/dS2.
    pub gamma: f64,
    /// dV/d(sigma), per 1.00 of volatility.
    pub vega: f64,
    /// dV/d(tau), per year of time to expiry. Theta is the negative of this.
    pub dv_dtau: f64,
}

/// Payoff at expiry.
pub fn intrinsic(ty: OptType, s: f64, k: f64) -> f64 {
    match ty {
        OptType::Call => (s - k).max(0.0),
        OptType::Put => (k - s).max(0.0),
    }
}

fn expired(ty: OptType, s: f64, k: f64, disc: f64) -> Greeks {
    let itm = match ty {
        OptType::Call => s > k,
        OptType::Put => s < k,
    };
    let sign = if ty == OptType::Call { 1.0 } else { -1.0 };
    Greeks {
        value: disc * intrinsic(ty, s, k),
        delta: if itm { sign * disc } else { 0.0 },
        ..Greeks::default()
    }
}

/// Prices one option. `s` is spot (Black-Scholes) or the futures price
/// (Black-76); `tau` is in years; `r` is continuously compounded.
pub fn price(model: Model, ty: OptType, s: f64, k: f64, tau: f64, r: f64, sigma: f64) -> Greeks {
    let s = s.max(0.0);
    if tau <= 0.0 {
        return expired(ty, s, k, 1.0);
    }
    let disc = exp(-r * tau);
    let vol_t = sigma * sqrt(tau);
    if s == 0.0 || vol_t < 1e-12 {
        return match model {
            Model::BlackScholes => {
                let mut g = expired(ty, s, k * disc, 1.0);
                g.dv_dtau = match (ty, g.value > 0.0) {
                    (OptType::Call, true) => r * k * disc,
                    (OptType::Put, true) => -r * k * disc,
                    _ => 0.0,
                };
                g
            }
            Model::Black76 => {
                let mut g = expired(ty, s, k, disc);
                g.dv_dtau = -r * g.value;
                g
            }
        };
    }
    match model {
        Model::BlackScholes => black_scholes(ty, s, k, tau, r, sigma, disc, vol_t),
        Model::Black76 => black76(ty, s, k, tau, r, sigma, disc, vol_t),
    }
}

#[allow(clippy::too_many_arguments)]
fn black_scholes(
    ty: OptType,
    s: f64,
    k: f64,
    tau: f64,
    r: f64,
    sigma: f64,
    disc: f64,
    vol_t: f64,
) -> Greeks {
    let d1 = (ln(s / k) + (r + 0.5 * sigma * sigma) * tau) / vol_t;
    let d2 = d1 - vol_t;
    let pdf = norm_pdf(d1);
    let gamma = pdf / (s * vol_t);
    let vega = s * pdf * sqrt(tau);
    let decay = s * pdf * sigma / (2.0 * sqrt(tau));
    match ty {
        OptType::Call => Greeks {
            value: s * norm_cdf(d1) - k * disc * norm_cdf(d2),
            delta: norm_cdf(d1),
            gamma,
            vega,
            dv_dtau: decay + r * k * disc * norm_cdf(d2),
        },
        OptType::Put => Greeks {
            value: k * disc * norm_cdf(-d2) - s * norm_cdf(-d1),
            delta: norm_cdf(d1) - 1.0,
            gamma,
            vega,
            dv_dtau: decay - r * k * disc * norm_cdf(-d2),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn black76(
    ty: OptType,
    f: f64,
    k: f64,
    tau: f64,
    r: f64,
    sigma: f64,
    disc: f64,
    vol_t: f64,
) -> Greeks {
    let d1 = (ln(f / k) + 0.5 * sigma * sigma * tau) / vol_t;
    let d2 = d1 - vol_t;
    let pdf = norm_pdf(d1);
    let gamma = disc * pdf / (f * vol_t);
    let vega = disc * f * pdf * sqrt(tau);
    let (value, delta) = match ty {
        OptType::Call => (
            disc * (f * norm_cdf(d1) - k * norm_cdf(d2)),
            disc * norm_cdf(d1),
        ),
        OptType::Put => (
            disc * (k * norm_cdf(-d2) - f * norm_cdf(-d1)),
            -disc * norm_cdf(-d1),
        ),
    };
    let dv_dtau = -r * value + disc * f * pdf * sigma / (2.0 * sqrt(tau));
    Greeks {
        value,
        delta,
        gamma,
        vega,
        dv_dtau,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BS: Model = Model::BlackScholes;
    const B76: Model = Model::Black76;

    fn fd(model: Model, ty: OptType, s: f64, k: f64, t: f64, r: f64, v: f64) -> [f64; 4] {
        let p = |s: f64, t: f64, v: f64| price(model, ty, s, k, t, r, v).value;
        let h = s * 1e-4;
        [
            (p(s + h, t, v) - p(s - h, t, v)) / (2.0 * h),
            (p(s + h, t, v) - 2.0 * p(s, t, v) + p(s - h, t, v)) / (h * h),
            (p(s, t, v + 1e-5) - p(s, t, v - 1e-5)) / 2e-5,
            (p(s, t + 1e-6, v) - p(s, t - 1e-6, v)) / 2e-6,
        ]
    }

    #[test]
    fn greeks_match_finite_differences() {
        for model in [BS, B76] {
            for ty in [OptType::Call, OptType::Put] {
                for (s, k) in [(42.0, 40.0), (95_000.0, 110_000.0), (100.0, 100.0)] {
                    let g = price(model, ty, s, k, 0.5, 0.05, 0.3);
                    let n = fd(model, ty, s, k, 0.5, 0.05, 0.3);
                    let scale = s.max(1.0);
                    assert!((g.delta - n[0]).abs() < 1e-6, "delta {model:?} {ty:?}");
                    assert!(
                        (g.gamma - n[1]).abs() * scale < 1e-3,
                        "gamma {model:?} {ty:?}"
                    );
                    assert!(
                        (g.vega - n[2]).abs() / scale < 1e-6,
                        "vega {model:?} {ty:?}"
                    );
                    assert!(
                        (g.dv_dtau - n[3]).abs() / scale < 1e-5,
                        "theta {model:?} {ty:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn edge_cases() {
        let c = price(BS, OptType::Call, 110.0, 100.0, 0.0, 0.05, 0.3);
        assert_eq!((c.value, c.delta), (10.0, 1.0));
        let p = price(BS, OptType::Put, 110.0, 100.0, 0.0, 0.05, 0.3);
        assert_eq!((p.value, p.delta), (0.0, 0.0));
        let z = price(BS, OptType::Call, 110.0, 100.0, 1.0, 0.05, 0.0);
        assert!((z.value - (110.0 - 100.0 * exp(-0.05))).abs() < 1e-12);
        assert!((z.dv_dtau - 5.0 * exp(-0.05)).abs() < 1e-12);
        let zp = price(BS, OptType::Put, 90.0, 100.0, 1.0, 0.05, 0.0);
        assert!((zp.dv_dtau + 5.0 * exp(-0.05)).abs() < 1e-12);
        let zo = price(BS, OptType::Put, 110.0, 100.0, 1.0, 0.05, 0.0);
        assert_eq!(zo.dv_dtau, 0.0);
        let f = price(B76, OptType::Put, 90.0, 100.0, 1.0, 0.05, 0.0);
        assert!((f.value - 10.0 * exp(-0.05)).abs() < 1e-12);
        assert!((f.delta + exp(-0.05)).abs() < 1e-12);
        let zero = price(BS, OptType::Put, 0.0, 100.0, 1.0, 0.05, 0.2);
        assert!((zero.value - 100.0 * exp(-0.05)).abs() < 1e-12);
        assert_eq!(intrinsic(OptType::Put, 90.0, 100.0), 10.0);
    }
}
