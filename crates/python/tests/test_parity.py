"""M4: the shared fixture suite must match the native build to 1e-9."""

import json
from pathlib import Path

import pytest

from thesisgrid import _thesisgrid as core

FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"
NAMES = sorted(p.name for p in (FIXTURES / "scenarios").glob("*.json"))


def diff(a, b, path):
    if isinstance(a, bool) or isinstance(b, bool):
        return None if a == b else f"{path}: {a} vs {b}"
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return None if abs(a - b) <= 1e-9 * max(1.0, abs(b)) else f"{path}: {a} vs {b}"
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            return f"{path}: length {len(a)} vs {len(b)}"
        return next((d for i, (x, y) in enumerate(zip(a, b)) if (d := diff(x, y, f"{path}[{i}]"))), None)
    if isinstance(a, dict) and isinstance(b, dict):
        if a.keys() != b.keys():
            return f"{path}: keys differ"
        return next((d for k in a if (d := diff(a[k], b[k], f"{path}.{k}"))), None)
    return None if a == b else f"{path}: {a!r} vs {b!r}"


def test_suite_is_present():
    assert len(NAMES) >= 7


@pytest.mark.parametrize("name", NAMES)
def test_scenario_matches_native(name):
    got = json.loads(core.compare((FIXTURES / "scenarios" / name).read_text()))
    want = json.loads((FIXTURES / "expected" / name).read_text())
    assert diff(got, want, name) is None
