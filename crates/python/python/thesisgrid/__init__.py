"""ThesisGrid: same thesis, every instrument, one grid.

Every number comes from the Rust core in ``thesisgrid._thesisgrid``, the same
code the web page runs as WebAssembly. This module only builds the scenario
JSON and shapes the results; it holds no pricing, sizing or fee math, and it
never opens a network connection.
"""

from __future__ import annotations

import datetime as _dt
import json as _json
from typing import Any, Iterable, Sequence

import numpy as _np

from . import _thesisgrid as _core

__version__ = _core.__version__
__all__ = [
    "Thesis", "Spot", "Perp", "Option", "Binary", "Scenario", "Result",
    "parse", "compare", "from_url", "range_pct", "price_range", "price_list",
    "daily", "date_list", "STRATEGIES",
]

STRATEGIES: tuple[str, ...] = tuple(_json.loads(_core.presets())["strategies"])


def _iso(d: Any) -> Any:
    return d.isoformat() if isinstance(d, _dt.date) else d


def _clean(fields: dict[str, Any]) -> dict[str, Any]:
    """Drops unset fields so the core's defaults apply, and dates to ISO."""
    out: dict[str, Any] = {}
    for k, v in fields.items():
        if v is None:
            continue
        if isinstance(v, (list, tuple)):
            v = [_iso(x) if not isinstance(x, dict) else x for x in v]
        out[k] = _iso(v)
    return out


class Thesis:
    """The thesis: underlying, current price, target price and date.

    ``as_of`` defaults to today's date on this machine. Unknown fields are
    rejected by the core when the scenario runs.
    """

    def __init__(self, spot: float, target: float, target_date: str | _dt.date,
                 rate: float | None = None, iv: float | None = None, *,
                 underlying: str | None = None, asset_class: str | None = None,
                 as_of: str | _dt.date | None = None, touch: str | None = None,
                 direction: str | None = None, commodity_day_count: str | None = None) -> None:
        self.fields = _clean({
            "underlying": underlying, "asset_class": asset_class, "spot": spot,
            "target": target, "target_date": target_date,
            "as_of": as_of or _dt.date.today(), "touch": touch, "direction": direction,
            "rate": rate, "iv": iv, "commodity_day_count": commodity_day_count,
        })

    def __repr__(self) -> str:
        return f"Thesis({self.fields})"


class _Instrument:
    kind = ""

    def __init__(self, **fields: Any) -> None:
        self.fields = {"kind": self.kind, **_clean(fields)}

    def __repr__(self) -> str:
        return f"{type(self).__name__}({self.fields})"


class Spot(_Instrument):
    """Shares or spot: side, price, fee_pct, fee_fixed, stop, qty, label."""

    kind = "spot"


class Perp(_Instrument):
    """A perpetual future. ``venue`` picks a preset; every field overrides it."""

    kind = "perp"

    def __init__(self, *, funding_8h: float | None = None, **fields: Any) -> None:
        if funding_8h is not None:
            if "funding_rate" in fields:
                raise ValueError("give funding_8h or funding_rate, not both")
            fields["funding_rate"] = funding_8h
            fields["funding_period_hours"] = 8
        super().__init__(**fields)

    @classmethod
    def preset(cls, venue: str, **fields: Any) -> "Perp":
        """A perp on ``kalshi``, ``polymarket``, ``hyperliquid`` or ``custom``."""
        return cls(venue=venue, **fields)


class Option(_Instrument):
    """An option position: a named strategy or custom ``legs``."""

    kind = "option"

    @classmethod
    def strategy(cls, name: str, strikes: Sequence[float], expiry: str | _dt.date,
                 **fields: Any) -> "Option":
        """One of :data:`STRATEGIES`, with its strikes in ascending order."""
        return cls(strategy=name, strikes=list(strikes), expiry=expiry, **fields)


class Binary(_Instrument):
    """A prediction-market binary: ``above`` or ``range`` contract, YES or NO."""

    kind = "binary"

    @classmethod
    def preset(cls, venue: str, **fields: Any) -> "Binary":
        """A binary with ``kalshi``, ``polymarket``, ``hyperliquid`` (HIP-4) or ``custom`` fees."""
        return cls(venue=venue, **fields)


def range_pct(lo: float = -0.3, hi: float = 0.3, steps: int = 25) -> dict[str, Any]:
    """Rows from ``lo`` to ``hi`` around the current price, as fractions."""
    return {"kind": "range_pct", "lo": lo, "hi": hi, "steps": steps}


def price_range(lo: float, hi: float, steps: int = 25) -> dict[str, Any]:
    """Rows from ``lo`` to ``hi`` in price."""
    return {"kind": "range", "lo": lo, "hi": hi, "steps": steps}


def price_list(values: Iterable[float]) -> dict[str, Any]:
    """Rows at exactly these prices."""
    return {"kind": "list", "values": list(values)}


def daily(start: str | _dt.date | None = None, end: str | _dt.date | None = None,
          step: int = 1) -> dict[str, Any]:
    """Columns every ``step`` counted days (trading days for stocks and indices)."""
    return _clean({"start": start, "end": end, "step": step})


def date_list(dates: Iterable[str | _dt.date]) -> dict[str, Any]:
    """Columns at exactly these dates."""
    return {"list": [_iso(d) for d in dates]}


class Scenario:
    """A full setup: thesis, instruments, comparison mode, budget and grid."""

    def __init__(self, data: dict[str, Any]) -> None:
        self.data = data

    @property
    def thesis(self) -> dict[str, Any]:
        return self.data["thesis"]

    @property
    def instruments(self) -> list[dict[str, Any]]:
        return self.data.get("instruments", [])

    def to_json(self) -> str:
        return _json.dumps(self.data)

    def to_url(self, base: str | None = None) -> str:
        """The scenario as a ThesisGrid page link."""
        return _core.to_url(self.to_json(), base)

    def compare(self) -> "Result":
        return Result(self, _json.loads(_core.compare(self.to_json())))

    def __repr__(self) -> str:
        return f"Scenario({self.data})"


_FIELDS = ("pnl", "pct", "value", "fees", "funding", "delta", "gamma", "theta", "vega", "liquidated")


class Result:
    """Grids and summaries for every instrument, from the core."""

    def __init__(self, scenario: Scenario, raw: dict[str, Any]) -> None:
        self.scenario = scenario
        self.raw = raw
        self.prices = _np.array(raw["prices"], dtype=float)
        self.dates = [_dt.date.fromisoformat(d) for d in raw["dates"]]
        self.target_row: int = raw["target_row"]
        self.target_col: int = raw["target_col"]
        self.sizing_note: str = raw["sizing_note"]
        self.labels = [c["label"] for c in raw["columns"]]

    def _column(self, label: str) -> dict[str, Any]:
        for c in self.raw["columns"]:
            if c["label"] == label:
                return c
        raise KeyError(f"no instrument labelled {label!r}; labels are {self.labels}")

    def errors(self) -> dict[str, str]:
        """Instruments that have no grid, with the reason (e.g. "needs a stop")."""
        return {c["label"]: c["error"] for c in self.raw["columns"] if c["error"]}

    def notes(self, label: str) -> list[str]:
        """Caveats from the venue preset and the model."""
        return list(self._column(label)["notes"])

    def summary(self) -> list[dict[str, Any]]:
        """One row per instrument: cost, max loss, max gain, breakevens, P&L
        and return at target, liquidation price. ``None`` max loss or max gain
        means unbounded."""
        rows = []
        for c in self.raw["columns"]:
            row: dict[str, Any] = {"instrument": c["label"], "kind": c["kind"], "error": c["error"]}
            row.update(c["summary"] or {})
            rows.append(row)
        return rows

    def grid(self, label: str, field: str = "pnl") -> _np.ndarray:
        """A price x date array (rows ascending in price) of one cell field:
        pnl, pct, value, fees, funding, delta, gamma, theta, vega or liquidated."""
        if field not in _FIELDS:
            raise ValueError(f"field must be one of {_FIELDS}")
        c = self._column(label)
        if c["error"]:
            raise ValueError(f"{label}: {c['error']}")
        dtype = bool if field == "liquidated" else float
        return _np.array([[cell[field] for cell in row] for row in c["cells"]], dtype=dtype)

    def to_pandas(self) -> Any:
        """Every cell as a long table (needs pandas)."""
        import pandas as pd

        rows = []
        for c in self.raw["columns"]:
            for p, row in zip(self.prices, c["cells"]):
                for d, cell in zip(self.dates, row):
                    rows.append({"instrument": c["label"], "price": p, "date": d, **cell})
        return pd.DataFrame(rows)

    def to_csv(self) -> str:
        """Grids and summary as CSV, the same file the page exports."""
        return _core.csv(self.scenario.to_json())

    def to_url(self, base: str | None = None) -> str:
        """The same scenario as a ThesisGrid page link."""
        return self.scenario.to_url(base)

    def __repr__(self) -> str:
        return f"Result({self.sizing_note!r}, instruments={self.labels})"


def parse(text: str, spot: float, *, as_of: str | _dt.date | None = None,
          **fields: Any) -> Thesis:
    """A :class:`Thesis` from a quick-entry phrase such as
    ``"NVDA to $200 by Mar 20"``. The current price is always yours to give;
    anything the phrase leaves out must be passed as a keyword."""
    as_of = _iso(as_of or _dt.date.today())
    got = _json.loads(_core.parse(text, as_of))
    read = {k: got[k] for k in ("underlying", "asset_class", "target", "target_date",
                                "direction", "touch") if got.get(k) is not None}
    read.update({k: v for k, v in fields.items() if v is not None})
    missing = [k for k in ("target", "target_date") if k not in read]
    if missing:
        raise ValueError(f"could not read {', '.join(missing)} from {text!r}; pass it as a keyword")
    target = read.pop("target")
    target_date = read.pop("target_date")
    return Thesis(spot, target, target_date, as_of=as_of, **read)


def compare(thesis: Thesis | Scenario, legs: Sequence[_Instrument] = (), *,
            mode: str | None = None, budget: float | None = None,
            prices: dict[str, Any] | None = None, dates: dict[str, Any] | None = None,
            display: str | None = None) -> Result:
    """Sizes every instrument by ``mode`` (equal_dollars, equal_max_loss,
    equal_exposure or manual) and evaluates it on the grid."""
    if isinstance(thesis, Scenario):
        if legs or mode or budget is not None or prices or dates or display:
            raise ValueError("pass either a Scenario or a thesis with options, not both")
        return thesis.compare()
    grid = _clean({"prices": prices, "dates": dates, "display": display})
    data = _clean({"thesis": thesis.fields, "instruments": [leg.fields for leg in legs],
                   "mode": mode, "budget": budget})
    if grid:
        data["grid"] = grid
    return Scenario(data).compare()


def from_url(link: str) -> Scenario:
    """Loads a scenario shared from the page."""
    return Scenario(_json.loads(_core.from_url(link)))
