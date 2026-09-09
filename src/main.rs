use std::cmp;

// Binance BTCUSDT perpetual: tick size 0.10, quantity step 0.001.
// Prices and quantities are integers everywhere; scaling happens
// only at parse and at print.
const QTY_MULTIPLIER: i64 = 1000;
const PRICE_MULTIPLIER: i64 = 10;

#[derive(Debug, PartialEq, Clone, Copy)]
enum Side {
    Bid,
    Ask,
}

#[derive(Debug)]
struct OpenOrder {
    id: u64,
    side: Side,
    qty_ahead: i64,
    qty_remaining: i64,
    price: i64,
}

impl OpenOrder {
    
    fn on_trade(&mut self, price: i64, qty: i64, consumed: Side) -> Option<i64> {

        if price != self.price || consumed != self.side {
            return None;
        }

        let absorbed = qty.min(self.qty_ahead);
        self.qty_ahead -= absorbed;

        let reaching_me = qty - absorbed;
        let filled = reaching_me.min(self.qty_remaining);
        self.qty_remaining -= filled;

        Some(filled)
    }

}
fn main() {
    println!("Hello, world!");
}
