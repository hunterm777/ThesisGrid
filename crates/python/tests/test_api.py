"""The Python API from the PRD, end to end."""

import datetime as dt

import numpy as np
import pytest

import thesisgrid as tg


def btc():
    thesis = tg.Thesis(spot=95_000, target=110_000, target_date="2026-12-31",
                       rate=0.04, iv=0.55, as_of="2026-11-24")
    legs = [
        tg.Spot(side="long", stop=85_000),
        tg.Perp.preset("hyperliquid", side="long", leverage=5, funding_8h=0.0001),
        tg.Option.strategy("bull_call_spread", strikes=(100_000, 115_000), expiry="2026-12-31"),
        tg.Binary.preset("kalshi", side="yes", strike=110_000, expiry="2026-12-31", price=0.31),
    ]
    return thesis, legs


def test_prd_btc_example():
    thesis, legs = btc()
    result = tg.compare(thesis, legs, mode="equal_max_loss", budget=500,
                        prices=tg.range_pct(-0.3, 0.3, steps=25),
                        dates=tg.daily("2026-11-24", "2026-12-31"))
    assert result.sizing_note.startswith("Equal max loss: $500 each. Perp sized at 5x")
    rows = result.summary()
    assert [r["instrument"] for r in rows] == ["spot", "perp", "option", "binary"]
    for r in rows:
        assert r["max_loss"] == pytest.approx(500, abs=1e-6)
    g = result.grid("perp")
    assert g.shape == (len(result.prices), len(result.dates)) == (26, 38)
    assert result.prices[result.target_row] == 110_000
    assert result.dates[result.target_col] == dt.date(2026, 12, 31)
    assert result.grid("perp", "liquidated")[0, 0]
    assert g[result.target_row, result.target_col] == pytest.approx(rows[1]["pnl_at_target"])
    link = result.to_url()
    assert link.startswith("https://hunterm777.github.io/ThesisGrid/#s=")
    again = tg.compare(tg.from_url(link))
    assert np.array_equal(again.grid("option"), result.grid("option"))
    assert tg.from_url(link).thesis["spot"] == 95_000
    assert result.to_csv().startswith("instrument,price,date")
    df = result.to_pandas()
    assert len(df) == 4 * 26 * 38 and set(df["instrument"]) == set(result.labels)
    assert "Result(" in repr(result) and "Spot(" in repr(legs[0]) and "Thesis(" in repr(thesis)


def test_prd_stock_example():
    thesis = tg.parse("NVDA to $200 by Mar 20", spot=182.40, iv=0.48, as_of="2026-11-24")
    assert thesis.fields["underlying"] == "NVDA"
    assert thesis.fields["asset_class"] == "stock"
    assert thesis.fields["target_date"] == "2027-03-20"
    legs = [
        tg.Spot(side="long"),
        tg.Perp.preset("polymarket", asset="stock", side="long", leverage=5),
        tg.Option.strategy("long_call", strikes=(190,), expiry="2027-03-19"),
        tg.Binary.preset("polymarket", side="yes", strike=200, expiry="2027-03-19", price=0.27),
    ]
    result = tg.compare(thesis, legs, mode="equal_dollars", budget=2_000)
    for r in result.summary():
        assert r["cost"] == pytest.approx(2_000, abs=1e-6)
    assert result.errors() == {}
    assert any("Polymarket" in n or "market" in n for n in result.notes("binary"))


def test_errors_and_inputs():
    thesis, legs = btc()
    r = tg.compare(thesis, [tg.Spot(), legs[1]], mode="equal_max_loss", budget=500)
    assert r.errors() == {"spot": "needs a stop"}
    with pytest.raises(ValueError, match="needs a stop"):
        r.grid("spot")
    with pytest.raises(KeyError):
        r.grid("nope")
    with pytest.raises(ValueError):
        r.grid("perp", "colour")
    with pytest.raises(ValueError):
        tg.compare(thesis, [tg.Spot(oops=1)])
    with pytest.raises(ValueError):
        tg.Perp(funding_8h=0.1, funding_rate=0.1)
    with pytest.raises(ValueError, match="target_date"):
        tg.parse("NVDA to $200 / share", spot=182.4)
    t = tg.parse("NVDA to $200 / share", spot=182.4, target_date=dt.date(2027, 3, 20))
    assert t.fields["target_date"] == "2027-03-20"
    with pytest.raises(ValueError):
        tg.compare(tg.from_url(r.to_url()), legs)
    with pytest.raises(ValueError):
        tg.from_url("https://hunterm777.github.io/ThesisGrid/#s=!!")
    assert tg.Thesis(1, 2, "2099-01-01").fields["as_of"] == dt.date.today().isoformat()


def test_grid_helpers_and_lists():
    thesis, legs = btc()
    r = tg.compare(thesis, legs[:2], mode="manual",
                   prices=tg.price_list([90_000, 100_000]),
                   dates=tg.date_list([dt.date(2026, 12, 1), "2026-12-31"]), display="percent")
    assert list(r.prices) == [90_000, 100_000, 110_000]
    assert r.raw["display"] == "percent"
    r = tg.compare(thesis, legs[:1], prices=tg.price_range(80_000, 120_000, 8))
    assert r.prices[0] == 80_000 and r.prices[-1] == 120_000
    assert "iron_condor" in tg.STRATEGIES
    assert tg.__version__ == "0.1.0"


def test_no_socket_is_ever_opened():
    import socket

    with pytest.raises(AssertionError):
        socket.socket()
    thesis, legs = btc()
    tg.compare(thesis, legs, mode="equal_dollars", budget=1000)
