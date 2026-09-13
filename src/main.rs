mod order;
mod parser;
mod sim;

use parser::TradeReader;
use parser::BookTickerReader;

use crate::sim::Quote;
use crate::sim::Simulator;

// Binance BTCUSDT perpetual: tick size 0.01, quantity step 0.001.
// Prices and quantities are integers everywhere; scaling happens
// only at parse and at print.
const QTY_MULTIPLIER: i64 = 1000;
const PRICE_MULTIPLIER: i64 = 100;

fn main() -> Result<(), parser::ParseError> {
    let quote = Quote {
        bid_price: 7157810,
        ask_price: 7157820,
        qty: 500,
        timestamp: 1710461204280,
    };
    let mut sim = Simulator::new(
        "data/BTCUSDT-aggTrades-2024-03-15.csv",
        "data/BTCUSDT-bookTicker-2024-03-15.csv",
        quote,
    )?;
    sim.run()?;
    Ok(())
}
