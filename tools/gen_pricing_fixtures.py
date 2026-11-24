"""Writes fixtures/pricing.json: Black-Scholes, Black-76 and digital reference
values computed with mpmath at 50 digits, independently of the Rust core.

Greeks are taken by high-precision numerical differentiation of the closed
form, so they do not reuse the core's Greek formulas.

Run: python tools/gen_pricing_fixtures.py
"""

import json
from pathlib import Path

import mpmath as mp

mp.mp.dps = 50


def N(x):
    return mp.ncdf(x)


def bs(ty, s, k, t, r, v):
    d1 = (mp.log(s / k) + (r + v * v / 2) * t) / (v * mp.sqrt(t))
    d2 = d1 - v * mp.sqrt(t)
    if ty == "call":
        return s * N(d1) - k * mp.e ** (-r * t) * N(d2)
    return k * mp.e ** (-r * t) * N(-d2) - s * N(-d1)


def b76(ty, f, k, t, r, v):
    d1 = (mp.log(f / k) + v * v * t / 2) / (v * mp.sqrt(t))
    d2 = d1 - v * mp.sqrt(t)
    df = mp.e ** (-r * t)
    if ty == "call":
        return df * (f * N(d1) - k * N(d2))
    return df * (k * N(-d2) - f * N(-d1))


def digital(_ty, s, k, t, r, v):
    d2 = (mp.log(s / k) + (r - v * v / 2) * t) / (v * mp.sqrt(t))
    return mp.e ** (-r * t) * N(d2)


MODELS = {"black_scholes": bs, "black76": b76, "digital": digital}

# (model, type, s, k, tau, r, sigma); the first rows are textbook cases:
# Hull, Options, Futures and Other Derivatives, Example 15.6 (S=42, K=40,
# r=10%, sigma=20%, T=0.5: call 4.76, put 0.81) and Example 18.7 for Black-76
# (F=20, K=20, r=9%, sigma=25%, T=4/12: put 1.12).
CASES = [
    ("black_scholes", "call", 42, 40, 0.5, 0.10, 0.20),
    ("black_scholes", "put", 42, 40, 0.5, 0.10, 0.20),
    ("black76", "put", 20, 20, mp.mpf(4) / 12, 0.09, 0.25),
    ("black76", "call", 20, 20, mp.mpf(4) / 12, 0.09, 0.25),
]
for s, k, t, r, v in [
    (100, 100, 1.0, 0.05, 0.2),
    (100, 120, 0.25, 0.03, 0.35),
    (95000, 110000, 37 / 365, 0.04, 0.55),
    (182.4, 200, 79 / 252, 0.04, 0.48),
    (3400, 3200, 52 / 365, 0.0, 0.18),
    (50, 40, 2.0, 0.08, 1.2),
    (10, 30, 0.05, 0.01, 0.9),
]:
    for model in ("black_scholes", "black76"):
        for ty in ("call", "put"):
            CASES.append((model, ty, s, k, t, r, v))
    CASES.append(("digital", "call", s, k, t, r, v))
CASES.append(("digital", "call", 100, 100, 1e-6, 0.05, 0.5))


def row(model, ty, s, k, t, r, v):
    f = MODELS[model]
    s, k, t, r, v = (mp.mpf(x) for x in (s, k, t, r, v))
    val = f(ty, s, k, t, r, v)
    return {
        "model": model,
        "type": ty,
        "s": float(s),
        "k": float(k),
        "tau": float(t),
        "r": float(r),
        "sigma": float(v),
        "value": float(val),
        "delta": float(mp.diff(lambda x: f(ty, x, k, t, r, v), s)),
        "gamma": float(mp.diff(lambda x: f(ty, x, k, t, r, v), s, 2)),
        "vega": float(mp.diff(lambda x: f(ty, s, k, t, r, x), v)),
        "dv_dtau": float(mp.diff(lambda x: f(ty, s, k, x, r, v), t)),
    }


out = {
    "about": "Reference values from mpmath at 50 digits; see tools/gen_pricing_fixtures.py.",
    "cases": [row(*c) for c in CASES],
}
path = Path(__file__).resolve().parent.parent / "fixtures" / "pricing.json"
path.write_text(json.dumps(out, indent=1) + "\n")
print(f"wrote {len(out['cases'])} cases to {path}")
