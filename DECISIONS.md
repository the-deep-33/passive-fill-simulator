# Design decisions

## Prices and quantities are integers throughout

Scaled `i64` everywhere, with conversion happening only at parse and at
print. Prices are scaled by 10 (tick size 0.10), quantities by 1000
(step size 0.001).

Floating point was rejected for three reasons: equality comparison on
prices is central to matching an order to a trade and is unreliable under
binary floating point; error accumulates over millions of events; and
integer comparison is cheaper than float comparison.

The two feeds spell the same value differently: `aggTrades` writes
`71455.6`, `bookTicker` writes `71455.50000000`, so the parser has to
normalise regardless of the internal representation.

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
trailing garbage — a stray byte, a CR from CRLF line endings — would be
classified as a precision violation and abort the run, which is exactly
backwards.

## Input is streamed, not loaded

A passive fill simulator is causal: it never needs to look ahead of the
current event. Loading the files into memory would buy random access that
is never used, at a cost proportional to file size.

## One clock, one pass

Events are processed in timestamp order and orders enter the book when
the clock reaches their creation time. Causality is therefore structural:
there is no need to test each event against each order's creation
timestamp, because an order cannot see an event that preceded it.

Creation timestamps are still recorded, for time-to-fill reporting.

## Open orders are stored in a `Vec`

Cardinality is low: a handful of resting orders at most, so the
container choice is not performance-relevant. A `Vec` was chosen for
simplicity; a `HashMap` would add hashing cost and an indirection that
buy nothing at this size.

Nothing depends on the order of that `Vec`: fills are determined by
price and queue position, and each order carries its own state from its
own insertion moment. `swap_remove` is therefore safe and O(1), where
`remove` would be O(n) for no benefit.

## Queue models are an enum, not a trait object

Three known variants, chosen once at startup. An enum is stack-allocated
and dispatches through a jump table; `Box<dyn QueueModel>` would add a
heap allocation and an indirect call on every event for no flexibility
that is actually needed.

## Known limitations

- Only top-of-book orders can be simulated; see README.
- Cancellations are unobservable, which is the entire reason the three
  models exist rather than one.