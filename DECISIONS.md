# Design decisions

## Prices and quantities are integers throughout

Scaled `i64` everywhere, with conversion happening only at parse and at
print. Prices are scaled by 100 (tick size 0.01), quantities by 1000
(step size 0.001).

Floating point was rejected for three reasons: equality comparison on
prices is central to matching an order to a trade and is unreliable under
binary floating point; error accumulates over millions of events; and
integer comparison is cheaper than float comparison.

The two feeds spell the same value differently: `aggTrades` writes
`71455.6`, `bookTicker` writes `71455.50000000`, so the parser has to
normalise regardless of the internal representation.

The scaling factor was initially 10, on the assumption of a 0.10 tick.
That was wrong. Row 22,486 of the 2024-03-15 aggTrades file carries
`71799.36`, which the parser rejected as `TooPrecise`. The assumption
failed loudly on first contact with real data, which is the behaviour the
error was written for. The test named `real_tick_size_is_two_decimals`
pins it.

## Parsing is hand-rolled

No `csv` or `serde`. The files are machine-generated, contain no quoted
fields or embedded commas, and are LF-terminated, so `split(',')` is
sufficient. Using `serde` with a float field would have reintroduced the
float round-trip the integer representation exists to avoid.

## Excess decimal precision is an error, not a rounding case

`bookTicker` pads to eight decimal places with zeros. Truncating those is
lossless. Truncating a non-zero digit would not be, and would mean the
instrument's tick or step size is not what this program assumes.

The parser therefore verifies that every discarded digit is zero, and
fails otherwise. A run that completes while silently absorbing a changed
contract specification is worse than a run that stops.

## Two error variants, because callers treat them differently

`Malformed` means the field is not a valid decimal number: a corrupt row,
which the caller may count and skip.

`TooPrecise` means the field is a valid number that violates the assumed
step size: a broken assumption, which should stop the run.

Policy is set at the top level, not in the parser. The parser classifies;
the caller decides.

Note: the tail of an over-long fraction is checked for being digits
before being checked for being zeros. Without that ordering, any
trailing garbage, a stray byte or a CR from CRLF line endings, would be
classified as a precision violation and abort the run, which is exactly
backwards. This was a real bug, found by tests that asserted the intended
classification rather than the observed one.

## Input is streamed, not loaded

A passive fill simulator is causal: it never needs to look ahead of the
current event. Loading the files into memory would buy random access that
is never used, at a cost proportional to file size.

Each reader holds one parsed record ahead in a `pending` slot, which is
what makes timestamp peeking possible without consuming. The line buffer
is reused across reads rather than allocating a `String` per row.

## One clock, one pass

Events are processed in timestamp order and orders enter the book when
the clock reaches their creation time. There is no need to test each event 
against each order's creation timestamp, because an order cannot see an event 
that preceded it.

Creation timestamps are still recorded, for time-to-fill reporting.

## Trades are processed before book updates at equal timestamps

A `bookTicker` row at time T is a snapshot of the state after the
trades at time T have landed. Processing trades first is therefore not a
stylistic tie-break, it is what makes the residual arithmetic correct:
the depth reduction a tick reports has to be compared against the trade
volume that caused it, which means that volume must already be
accumulated when the tick is handled.

Within a single millisecond the true ordering is unrecoverable, so this
is the best available approximation rather than a guarantee.

## One order per side, held in an `Option`

The current scope quotes one bid and one ask, so the natural container is
`Option<OpenOrder>` per side rather than a collection. `None` means no
live order, and `take()` expresses termination directly.

An earlier draft used a `Vec` with `swap_remove`, justified on the grounds
that nothing depends on ordering. That reasoning was sound but the
container was solving a problem the scope does not have. If stacked
orders at one price are added later, the insertion rule is known: queue
ahead is the level size reported by `bookTicker` plus the sum of
`qty_remaining` across own live orders already resting at that price.

## Orders join at the back of the queue

At insertion, `qty_ahead` is the entire depth reported at that price.
This assumes the simulated order would have landed behind everything
already resting there.

This is but an assumption. It is conservative, in that it never overstates 
fill rate, and it is the only choice consistent with the feed: nothing in 
`bookTicker` indicates where in the queue an arriving order would sit.

## An order can only be inserted at the touch

A bid below the best bid rests at a price the feed never reports, so both
its level depth and its queue position are unknown, and remain unknown
even if the touch later falls to that price, because whatever accumulated
there while it was hidden is unobserved.

A bid above the best bid becomes the best bid immediately, which means it
has moved the touch, and every event replayed afterwards is
counterfactual. It also has a trivial queue position of zero, so there is
nothing to model.

Only a quote at the touch is both observable and non-impacting. The quote
therefore does not carry prices: the simulator takes them from the book at
insertion and reports which ones it used. This is a consequence of
choosing `bookTicker` as the feed. An L2 or L3 depth recording would remove 
the restriction.

## Trade volume is accumulated in full, not just the part ahead

`traded_since_tick` accumulates the entire trade quantity, not the part
absorbed by `qty_ahead`.

The accumulator exists to explain what the depth did, and the depth at a
price level falls by the full traded quantity regardless of where the
simulated order sat in the queue. Recording only the absorbed part would
make the next tick attribute the difference to cancellations that never
happened.

Binance aggregates `aggTrades` rows by taker order and price, so one row
is one price level and a sweep across levels produces separate rows.
There is therefore no row whose quantity spans two levels.

## An outbid order dies, rather than being requoted

When the touch moves off the order's price, the order's level stops being
reported and its queue position stops being trackable. The order is
terminated and reported as unfilled, with the elapsed time.

This models a place-once strategy. A real market maker would cancel and
rejoin at the new touch, so this is a scope decision rather than a
limitation of the data.

Open point: the two directions are currently one branch. The touch moving
away from the order means someone outbid it. The touch moving through
it means the order's own level emptied out, which is a different event
with a well-defined consequence, namely that nothing remains ahead of it.
The second case is currently discarded along with the first.

## Queue models are an enum, rather than a trait object

Three known variants, chosen once at startup. An enum is stack-allocated
and dispatches through a jump table; `Box<dyn QueueModel>` would add a
heap allocation and an indirect call on every event for no flexibility
that is actually needed.

Not yet wired into the event loop.

## Open question: clamping `qty_ahead` against observed depth

`qty_ahead` cannot legitimately exceed the depth reported at the level.
Under the pessimistic model it will, because that model never credits
cancellations to the queue ahead while the real level shrinks through
both trades and cancellations.

Two possible solutions:

- The clamp is the pessimistic update rule. Belief is corrected against
  observation at every tick, and the model stays physically coherent.
- The clamp destroys the measurement. The size of the excess is how far
  the pessimistic assumption has drifted from observable reality, which is
  a number worth reporting, and silently clamping makes pessimistic behave
  like a weaker form of proportional.

Current intent is to record the excess before clamping, so the diagnostic
survives and the state stays coherent. To be decided when `on_tick` is
wired.

## Known limitations

- Only top-of-book orders can be simulated; see README.
- Cancellations are unobservable, which is the entire reason the three
  models exist rather than one.
- Additions to a level are invisible in the same way cancellations are.
  Quantity joining behind the simulated order is never tracked, so the
  residual is a net figure and the models operate on it as though it were
  gross.
- The event loop has no test coverage. All current tests sit below it, at
  the parser and at fill arithmetic.
