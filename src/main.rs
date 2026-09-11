mod order;
mod parser;

// Binance BTCUSDT perpetual: tick size 0.10, quantity step 0.001.
// Prices and quantities are integers everywhere; scaling happens
// only at parse and at print.
const QTY_MULTIPLIER: i64 = 1000;
const PRICE_MULTIPLIER: i64 = 10;

fn main() {
}
