# thesisgrid

**Same thesis, every instrument, one grid.**

`thesisgrid` takes one market thesis ("BTC to 120k by Dec 31", "NVDA to $200 by Mar 20") and returns a price × date P&L grid for each way to express it: shares or spot, perps, options and prediction-market binaries, sized on the basis you choose (equal dollars, equal max loss or equal exposure). Venue presets cover Kalshi, Polymarket and Hyperliquid.

The math is a Rust core shared with the [ThesisGrid page](https://hunterm777.github.io/ThesisGrid/), so the package returns the same numbers as the page. It needs no network access and never opens a socket.

```python
import thesisgrid as tg

thesis = tg.Thesis(spot=95_000, target=110_000, target_date="2026-12-31",
                   rate=0.04, iv=0.55)
legs = [
    tg.Spot(side="long", stop=85_000),
    tg.Perp.preset("hyperliquid", side="long", leverage=5, funding_8h=0.0001),
    tg.Option.strategy("bull_call_spread", strikes=(100_000, 115_000), expiry="2026-12-31"),
    tg.Binary.preset("kalshi", side="yes", strike=110_000, expiry="2026-12-31", price=0.31),
]
result = tg.compare(thesis, legs, mode="equal_max_loss", budget=500,
                    prices=tg.range_pct(-0.3, 0.3, steps=25),
                    dates=tg.daily("2026-11-24", "2026-12-31"))

result.summary()      # cost, max loss, max gain, breakeven, P&L at target
result.grid("perp")   # NumPy array, price x date
result.to_url()       # open the same scenario on the page
```

Prices in the example are illustrative inputs, not market data. Not financial advice.

Source, docs and license (MIT): https://github.com/hunterm777/ThesisGrid
