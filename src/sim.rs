use crate::order::OpenOrder;
use crate::order::Side;
use crate::parser::Trade;
use crate::parser::TradeReader;
use crate::parser::BookTicker;
use crate::parser::BookTickerReader;
use crate::parser::ParseError;

pub struct Quote {
    pub bid_price: i64,
    pub ask_price: i64,
    pub qty: i64,
    pub timestamp: i64,
}

pub struct Simulator {
    trades: TradeReader,
    ticks: BookTickerReader,
    book: Option<BookTicker>,
    inserted: bool,
    quote: Quote,
    bid: Option<OpenOrder>,
    ask: Option<OpenOrder>,
}

pub enum Advance {
    Trades,
    Tick,
}

fn apply_trade(slot: &mut Option<OpenOrder>, t: &Trade, label: &str) {
    let Some(order) = slot else { return };
    let Some(filled) = order.on_trade(t.price, t.qty, t.consumed) else { return };
    if filled > 0 {
        println!("{label} filled {filled} at timestamp: {}", t.timestamp);
    }
    if order.qty_remaining == 0 {
        println!("Total time for the {label} side of the order to fill: {}", (t.timestamp - order.created_ts));
        *slot = None;
    }
}

fn apply_tick(slot: &mut Option<OpenOrder>, b: &BookTicker) {
    let Some(order) = slot else { return };
    let best_price = match order.side {
        Side::Ask => b.best_ask_price,
        Side::Bid => b.best_bid_price,
    };

    if best_price != order.price {
        println!("Price changed: the quote price is {} and the new best price is {} for side {:?}", order.price, best_price, order.side);
        *slot = None;
    }
}

impl Simulator {
    pub fn new(trades_path: &str, ticks_path: &str, quote: Quote) -> Result<Self, ParseError> {
        Ok(Simulator {
            trades: TradeReader::open(trades_path)?,
            ticks: BookTickerReader::open(ticks_path)?,
            book: None,
            inserted: false,
            quote,
            bid: None,
            ask: None,
        })
    }

    pub fn run(&mut self) -> Result<(), ParseError>{
        loop {
            let trade_time = self.trades.peek_ts();
            let tick_time = self.ticks.peek_ts();

            let (advance, now) = match (trade_time, tick_time) {
                (None, None) => break,
                (None, Some(k)) => (Advance::Tick, k),
                (Some(t), None) => (Advance::Trades, t),
                (Some(t), Some(k)) => {
                    if t <= k { (Advance::Trades, t) } else { (Advance::Tick, k) }
                }
            };

            if !self.inserted && now >= self.quote.timestamp {
                let bt = match self.book {
                    Some(bt) => bt,
                    None => return Err(ParseError::NoOrder),
                };

                self.bid = Some(OpenOrder::new(
                    0,
                    Side::Bid,
                    self.quote.bid_price,
                    bt.best_bid_qty,
                    self.quote.qty,
                    now,
                ));

                self.ask = Some(OpenOrder::new(
                    1,
                    Side::Ask,
                    self.quote.ask_price,
                    bt.best_ask_qty,
                    self.quote.qty,
                    now,
                ));

                self.inserted = true;
            }

            match advance {
                Advance::Trades => {
                 if let Some(t) = self.trades.next_trade()? {
                    apply_trade(&mut self.bid, &t, "Bid");
                    apply_trade(&mut self.ask, &t, "Ask");
                }
                },
                Advance::Tick => {
                    if let Some(k) = self.ticks.next_tick()? {
                        apply_tick(&mut self.bid, &k);
                        apply_tick(&mut self.ask, &k);
                        self.book = Some(k);
                    }
                },
            }

        }

        Ok(())

    }
}