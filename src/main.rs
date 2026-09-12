mod order;
mod parser;

use parser::TradeReader;
use parser::BookTickerReader;

// Binance BTCUSDT perpetual: tick size 0.01, quantity step 0.001.
// Prices and quantities are integers everywhere; scaling happens
// only at parse and at print.
const QTY_MULTIPLIER: i64 = 1000;
const PRICE_MULTIPLIER: i64 = 100;

fn main() -> Result<(), parser::ParseError> {
    let mut reader = parser::TradeReader::open("data/BTCUSDT-aggTrades-2024-03-15.csv")?;
    let mut count: u64 = 0;

    let start = std::time::Instant::now();

    while let Some(_trade) = reader.next_trade()? {
        count += 1;
    }

    println!("{count} trades in {:?}", start.elapsed());

    let mut reader = parser::BookTickerReader::open("data/BTCUSDT-bookTicker-2024-03-15.csv")?;
    let mut count: u64 = 0;

    let start = std::time::Instant::now();

    while let Some(_trade) = reader.next_tick()? {
        count += 1;
    }

    println!("{count} ticks in {:?}", start.elapsed());
    Ok(())
}
