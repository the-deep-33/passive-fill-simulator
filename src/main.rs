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

#[cfg(test)]
mod tests {
    use super::*;

    const PRICE: i64 = 1084325;

    fn bid(qty_ahead: i64, qty_remaining: i64) -> OpenOrder {
        OpenOrder {
            id: 1,
            side: Side::Bid,
            qty_ahead,
            qty_remaining,
            price: PRICE,
        }
    }

    #[test]
    fn trade_exactly_exhausts_queue() {
        let mut order = bid(2300, 500);

        let filled = order.on_trade(PRICE, 2300, Side::Bid);

        assert_eq!(filled, Some(0));
        assert_eq!(order.qty_ahead, 0);
        assert_eq!(order.qty_remaining, 500);
    }

    #[test]
    fn trade_absorbed_by_queue() {
        let mut order = bid(2120, 500);

        let filled = order.on_trade(PRICE, 1000, Side::Bid);

        assert_eq!(filled, Some(0));
        assert_eq!(order.qty_ahead, 1120);
        assert_eq!(order.qty_remaining, 500);
    }

    #[test]
    fn partial_fill() {
        let mut order = bid(2120, 500);

        let filled = order.on_trade(PRICE, 2300, Side::Bid);

        assert_eq!(filled, Some(180));
        assert_eq!(order.qty_ahead, 0);
        assert_eq!(order.qty_remaining, 320);
    }

    #[test]
    fn full_fill() {
        let mut order = bid(2120, 500);

        let filled = order.on_trade(PRICE, 2620, Side::Bid);

        assert_eq!(filled, Some(500));
        assert_eq!(order.qty_ahead, 0);
        assert_eq!(order.qty_remaining, 0);
    }

    #[test]
    fn oversized_trade_discards_leftover() {
        let mut order = bid(2120, 500);

        let filled = order.on_trade(PRICE, 3500, Side::Bid);

        assert_eq!(filled, Some(500));
        assert_eq!(order.qty_ahead, 0);
        assert_eq!(order.qty_remaining, 0);
    }

    #[test]
    fn wrong_price_is_ignored() {
        let mut order = bid(2120, 500);

        let filled = order.on_trade(PRICE + 10, 3500, Side::Bid);

        assert_eq!(filled, None);
        assert_eq!(order.qty_ahead, 2120);
        assert_eq!(order.qty_remaining, 500);
    }

    #[test]
    fn wrong_side_is_ignored() {
        let mut order = bid(2120, 500);

        let filled = order.on_trade(PRICE, 3500, Side::Ask);

        assert_eq!(filled, None);
        assert_eq!(order.qty_ahead, 2120);
        assert_eq!(order.qty_remaining, 500);
    }

    #[test]
    fn sequential_trades_accumulate() {
        let mut order = bid(2120, 500);

        assert_eq!(order.on_trade(PRICE, 1000, Side::Bid), Some(0));
        assert_eq!(order.qty_ahead, 1120);

        assert_eq!(order.on_trade(PRICE, 2000, Side::Bid), Some(500));
        assert_eq!(order.qty_ahead, 0);
        assert_eq!(order.qty_remaining, 0);
    }
}