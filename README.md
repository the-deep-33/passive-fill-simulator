# passive-fill-simulator

Simulates passive limit order fills on Binance BTCUSDT perpetual futures,
replaying public `aggTrades` and `bookTicker` data under three queue
position models.

A passive order that rests at the touch is not filled the moment a trade
prints at its price. It is filled only once the queue ahead of it has been
consumed. How much queue sits ahead, and how that changes over time, is
not directly observable from public data, so it has to be modelled. This
tool makes that model explicit and lets you vary it.

## What the data can and cannot tell you

`bookTicker` is a snapshot of the touch: best bid, best ask, and the size
resting at each. It carries no depth beyond level one.

Two consequences:

- Simulated orders can only rest at the current best bid or best ask.
  An order placed behind the touch is invisible to this feed, so its
  queue position cannot be tracked.
- When the size at a price level shrinks, the feed does not say why.
  `aggTrades` accounts for the part consumed by trades. The residual is
  cancellations, and nothing in the data says whether those cancellations
  sat ahead of the simulated order or behind it.

That residual is what the three models disagree about.

## Queue models

| Model | Treatment of unexplained depth reduction |
| --- | --- |
| Optimistic | All cancellations occurred ahead of the order; queue position improves for free. |
| Pessimistic | All cancellations occurred behind the order; queue position is unchanged. |
| Proportional | Cancellations are split by the order's fractional position in the level. |

Trades are handled identically under all three: quantity consumes the
queue ahead first, and only the remainder reaches the order.

## Usage

```
cargo run --release -- <args>
```

Input files are the Binance daily archives, uncompressed, in the data folder.

```
data/BTCUSDT-aggTrades-<date>.csv
data/BTCUSDT-bookTicker-<date>.csv
```

They are not committed. Download them from
<https://data.binance.vision/>.

## Tests

```
cargo test
```

Parser tests use values taken directly from the archive files, including
the differing decimal formats between the two feeds.

## Design notes

See [DECISIONS.md](DECISIONS.md).