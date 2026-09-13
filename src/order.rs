#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug)]
pub struct OpenOrder {
    id: u64,
    pub side: Side,
    pub price: i64,
    pub qty_ahead: i64,
    pub qty_remaining: i64,
    pub created_ts: i64,
    pub traded_since_tick: i64,
    pub total_updates: i64,
    pub masked_updates: i64,
}

#[derive(Debug, Clone, Copy)]
pub enum QueueModel {
    Optimistic,
    Pessimistic,
    Proportional,
}

impl OpenOrder {

    pub fn new(id: u64, side: Side, price: i64, qty_ahead: i64, qty_remaining: i64, created_ts: i64) 
        -> Self {

        OpenOrder {
            id, side, price, qty_ahead, qty_remaining, created_ts,
            traded_since_tick: 0,
            total_updates: 0,
            masked_updates: 0,
        }

    }
    
    pub fn on_trade(&mut self, price: i64, qty: i64, consumed: Side) -> Option<i64> {

        if price != self.price || consumed != self.side {
            return None;
        }

        let absorbed = qty.min(self.qty_ahead);
        self.qty_ahead -= absorbed;

        let reaching_me = qty - absorbed;
        let filled = reaching_me.min(self.qty_remaining);
        self.qty_remaining -= filled;
        self.traded_since_tick += qty;

        Some(filled)
    }

    pub fn on_tick(&mut self, old_depth: i64, new_depth: i64, model: QueueModel) {
        let gross = old_depth - new_depth;
        let residual = gross - self.traded_since_tick;
        self.traded_since_tick = 0;
        self.total_updates += 1;

        if residual < 0 {
            self.masked_updates += 1;
            return;
        }
        if residual == 0 {
            return;
        }

        let ahead = match model {
            QueueModel::Optimistic => residual,
            QueueModel::Pessimistic => 0,
            QueueModel::Proportional => todo!(),
        };
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    const PRICE: i64 = 1084325;

    fn bid(qty_ahead: i64, qty_remaining: i64) -> OpenOrder {
        OpenOrder::new(1, Side::Bid, PRICE, qty_ahead, qty_remaining, 0)
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