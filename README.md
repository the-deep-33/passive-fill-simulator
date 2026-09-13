# passive-fill-simulator

Simulates passive limit order fills on Binance BTCUSDT perpetual futures,
replaying public `aggTrades` and `bookTicker` data under three queue
position models.

A passive order that rests at the touch is not filled the moment a trade
prints at its price. It is filled only once the queue ahead of it has been
consumed. How much queue sits ahead, and how that changes over time, is
not directly observable from public data, so it has to be modelled. This
tool makes that model explicit and lets you vary it.

## Status

The replay pipeline is complete and runs over the full day: parsing,
chronological merge, order insertion, fill accounting through trades, and
order termination on adverse price movement.

The queue models are specified and partially implemented but are not
yet connected to the event loop. Until they are, queue position moves
only through observed trade volume, which is the optimistic model with
cancellations set to zero rather than a model of cancellations. See
Roadmap.

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

The point of running all three is not to identify the correct one. It is
that the result comes out as a range rather than a number, and the width
of that range separates what the strategy earned from what the queue
assumption granted it. A result that survives only under the optimistic
model has not survived.

## Scope

- One order per side, inserted once, never requoted. An order that is
  outbid is reported as unfilled with a duration, not re-placed at the new
  touch. This models a specific and deliberately simple strategy.
- Both sides are quoted simultaneously, same quantity, same insertion
  timestamp.
- Orders join at the back of the queue at the price they enter at. This is
  a conservative assumption, not an observation: the feed cannot say where
  in the queue an order would have landed.
- No inventory, no position limits, no P&L. Fill timing is the output.

## Usage

```
cargo run --release
```

Paths, quote size, insertion timestamp, and queue model are currently
constants in `main.rs`. Command line arguments are on the roadmap.

Input files are the Binance daily archives, uncompressed, in the data folder.

```
data/BTCUSDT-aggTrades-<date>.csv
data/BTCUSDT-bookTicker-<date>.csv
```

They are not committed. Download them from
<https://data.binance.vision/>.

## Performance

Measured on the 2024-03-15 archives, release profile:

| Feed | Rows | Wall clock | Throughput |
| --- | --- | --- | --- |
| aggTrades | 4,265,593 | ~0.7s | ~6.2M rows/s |
| bookTicker | 26,651,695 | ~6.8s | ~3.9M rows/s |

Debug builds are roughly 5x slower. Cold and warm cache runs differ by
less than run-to-run variance, so replay is CPU bound rather than IO
bound at this row size.

## Tests

```
cargo test
```

Parser tests use values taken directly from the archive files, including
the differing decimal formats between the two feeds. Fill arithmetic is
covered at the `OpenOrder` level.

The event loop itself is not yet covered. See Roadmap.

## Roadmap

Ordered by what unblocks what.

1. Move order insertion into the book update branch of the event loop, so
   orders are sized against the tick that triggers them rather than the
   previous one.
2. Track observed depth at the order's own price, invalidated whenever the
   touch moves off that price, so that consecutive depths being differenced
   always refer to the same price level.
3. Wire `on_tick` into the loop and finish the three models, including the
   consistency check on `qty_ahead` against observed depth.
4. Make the readers generic over `BufRead` so the event loop can be driven
   from in-memory fixtures, then test the merge ordering, insertion, and
   fill sequencing directly.
5. Separate parse failures from IO and simulation failures, and carry the
   file path and row number on the latter.
6. Command line arguments for paths, quote, and model; output in human
   units rather than scaled integers.
7. Fill statistics across a full day and across all three models, which is
   the output the project exists to produce.

## Design notes

See [DECISIONS.md](DECISIONS.md).
