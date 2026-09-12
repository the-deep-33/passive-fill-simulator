use crate::order::Side;
use std::fs::File;
use std::io::{BufRead, BufReader};

const PRICE_PRECISION:u32 = 2;
const QTY_PRECISION:u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trade {
    timestamp: i64,
    price: i64,
    qty: i64,
    consumed: Side,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BookTicker {
    timestamp: i64,
    best_bid_price: i64,
    best_bid_qty: i64,
    best_ask_price: i64,
    best_ask_qty: i64,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Malformed,
    TooPrecise,
    FieldCount,
    Io(std::io::ErrorKind),
}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        ParseError::Io(e.kind())
    }
}

#[derive(Debug)]
pub struct TradeReader {
    reader: BufReader<File>,
    buf: String,
    pending: Option<Trade>,
}

impl TradeReader {
    fn fill(&mut self) -> Result<(), ParseError> {
        self.buf.clear();
        let n = self.reader.read_line(&mut self.buf)?;
        self.pending = if n == 0 {
            None
        } else {
            Some(parse_trade(self.buf.trim_end())?)
        };
        Ok(())
    }
    pub fn open(filepath: &str) -> Result<Self, ParseError> {
        let file = File::open(filepath)?;
        let mut reader = BufReader::new(file);
        let mut buf = String::with_capacity(128);
        reader.read_line(&mut buf)?; // header, discarded

        let mut r = TradeReader { reader, buf, pending: None };
        r.fill()?;
        Ok(r)
    }
    pub fn peek_ts(&self) -> Option<i64> {
        let pending = self.pending?;
        let timestamp = pending.timestamp;
        Some(timestamp)
    }
    pub fn next_trade(&mut self) -> Result<Option<Trade>, ParseError> {
        let trade = self.pending.take();

        self.fill()?;

        Ok(trade)
    }
}

#[derive(Debug)]
pub struct BookTickerReader {
    reader: BufReader<File>,
    buf: String,
    pending: Option<BookTicker>,
}

impl BookTickerReader {
    fn fill(&mut self) -> Result<(), ParseError> {
        self.buf.clear();
        let n = self.reader.read_line(&mut self.buf)?;
        self.pending = if n == 0 {
            None
        } else {
            Some(parse_book_ticker(self.buf.trim_end())?)
        };
        Ok(())
    }
    pub fn open(filepath: &str) -> Result<Self, ParseError> {
        let file = File::open(filepath)?;
        let mut reader = BufReader::new(file);
        let mut buf = String::with_capacity(128);
        reader.read_line(&mut buf)?; // header, discarded

        let mut r = BookTickerReader { reader, buf, pending: None };
        r.fill()?;
        Ok(r)
    }
    pub fn peek_ts(&self) -> Option<i64> {
        let pending = self.pending?;
        let timestamp = pending.timestamp;
        Some(timestamp)
    }
    pub fn next_tick(&mut self) -> Result<Option<BookTicker>, ParseError> {
        let tick = self.pending.take();

        self.fill()?;

        Ok(tick)
    }
}

/// Converts a fixed-point decimal string into an integer scaled by
/// 10^precision. The integral and the fractional parts are parsed
/// separately, meaning there are no floating point errors at stake.
///
/// The fractional part is right-padded to "precision" digits, so at
/// precision 3, "0.5" yields 500.
///
/// Fails if the field is not a decimal number, or if it carries more
/// fractional digits than the precision allows, in other words,
/// the function fails if the input violates the standard rules that the
/// CSV files follow, in terms of formatting.
fn parse_scaled(field: &str, precision: u32) -> Result<i64, ParseError> {

    if field.starts_with('-') {
        return Err(ParseError::Malformed);
    }

    let number = field.split_once('.') ;

    match number {
        None => {
            let integral = field.parse::<i64>().map_err(|_| ParseError::Malformed)?;
            Ok(integral * 10i64.pow(precision))
        }
        Some((integral, fraction)) => {
            if fraction.is_empty() {
                return Err(ParseError::Malformed);
            }
            if !fraction.bytes().all(|b| b.is_ascii_digit()) {
                return Err(ParseError::Malformed);
            }
            let precision = precision as usize;
            let fraction = if precision >= fraction.len() {
                let deficit = precision - fraction.len();
                fraction.parse::<u64>().map_err(|_| ParseError::Malformed)?
                    * 10u64.pow(deficit as u32)
            } else {
                let (significant, tail) = fraction.split_at(precision);
                if !tail.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(ParseError::Malformed);
                }
                if !tail.bytes().all(|b| b == b'0') {
                    return Err(ParseError::TooPrecise);
                }
                significant.parse::<u64>().map_err(|_| ParseError::Malformed)?
            };

            let whole = integral.parse::<i64>().map_err(|_| ParseError::Malformed)?
                * 10i64.pow(precision as u32);

            let number = whole + (fraction as i64);

            Ok(number)
        }
    }

}

fn parse_trade(line: &str) -> Result<Trade, ParseError> {

    let mut it = line.split(',');

    let _agg_trade_id   = it.next().ok_or(ParseError::FieldCount)?;
    let price           = it.next().ok_or(ParseError::FieldCount)?;
    let quantity        = it.next().ok_or(ParseError::FieldCount)?;
    let _first_trade_id = it.next().ok_or(ParseError::FieldCount)?;
    let _last_trade_id  = it.next().ok_or(ParseError::FieldCount)?;
    let transact_time   = it.next().ok_or(ParseError::FieldCount)?;
    let is_buyer_maker  = it.next().ok_or(ParseError::FieldCount)?;

    if it.next().is_some() {
        return Err(ParseError::FieldCount);
    }

    let price = parse_scaled(price, PRICE_PRECISION)?;
    let quantity = parse_scaled(quantity, QTY_PRECISION)?;
    let transact_time = transact_time.parse::<i64>().map_err(|_| ParseError::Malformed)?;

    // is_buyer_maker: the buyer was resting in the book, so an aggressive
    // seller consumed liquidity from the bid side.
    let side = match is_buyer_maker {
        "true"  => Side::Bid,
        "false" => Side::Ask,
        _ => return Err(ParseError::Malformed),
    };

    Ok(
        Trade { 
            timestamp: transact_time, 
            price, 
            qty: quantity, 
            consumed: side 
        }
    )
}

fn parse_book_ticker(line: &str) -> Result<BookTicker, ParseError> {
    let mut it = line.split(',');

    let _update_id = it.next().ok_or(ParseError::FieldCount)?;
    let best_bid_price = it.next().ok_or(ParseError::FieldCount)?;
    let best_bid_qty = it.next().ok_or(ParseError::FieldCount)?;
    let best_ask_price = it.next().ok_or(ParseError::FieldCount)?;
    let best_ask_qty = it.next().ok_or(ParseError::FieldCount)?;
    let transact_time = it.next().ok_or(ParseError::FieldCount)?;
    let _event_time = it.next().ok_or(ParseError::FieldCount)?;

    if it.next().is_some() {
        return Err(ParseError::FieldCount);
    }

    let best_bid_price = parse_scaled(best_bid_price, PRICE_PRECISION)?;
    let best_bid_qty = parse_scaled(best_bid_qty, QTY_PRECISION)?;
    let best_ask_price = parse_scaled(best_ask_price, PRICE_PRECISION)?;
    let best_ask_qty = parse_scaled(best_ask_qty, QTY_PRECISION)?;
    let transact_time = transact_time.parse::<i64>().map_err(|_| ParseError::Malformed)?;

    Ok(
        BookTicker { 
            timestamp: transact_time, 
            best_bid_price: best_bid_price, 
            best_bid_qty: best_bid_qty, 
            best_ask_price: best_ask_price, 
            best_ask_qty: best_ask_qty, 
        }
    )
}


#[cfg(test)]
mod tests {
    use super::*;

    // Tests for parse_scaled
    #[test]
    fn aggtrades_price_shape() {
        assert_eq!(parse_scaled("71455.6", PRICE_PRECISION), Ok(7145560));
        assert_eq!(parse_scaled("71455.7", PRICE_PRECISION), Ok(7145570));
    }
    #[test]
    fn real_tick_size_is_two_decimals() {
        // Row 22486 of BTCUSDT-aggTrades-2024-03-15: the row that caught
        // an incorrect tick-size assumption on first contact with real data.
        assert_eq!(parse_scaled("71799.36", PRICE_PRECISION), Ok(7179936));
    }
    #[test]
    fn bookticker_price_shape() {
        assert_eq!(parse_scaled("71455.50000000", PRICE_PRECISION), Ok(7145550));
        assert_eq!(parse_scaled("71455.60000000", PRICE_PRECISION), Ok(7145560));
    }
    #[test]
    fn aggtrades_quantity_shape() {
        assert_eq!(parse_scaled("0.148", QTY_PRECISION), Ok(148));
        assert_eq!(parse_scaled("0.002", QTY_PRECISION), Ok(2));
    }
    #[test]
    fn bookticker_quantity_shape() {
        assert_eq!(parse_scaled("2.12900000", QTY_PRECISION), Ok(2129));
        assert_eq!(parse_scaled("2.26000000", QTY_PRECISION), Ok(2260));
    }
    #[test]
    fn both_files_agree_on_the_same_value() {
        assert_eq!(
            parse_scaled("71455.6", PRICE_PRECISION),
            parse_scaled("71455.60000000", PRICE_PRECISION)
        );
    }
    #[test]
    fn whole_number_is_still_scaled() {
        assert_eq!(parse_scaled("50", PRICE_PRECISION), Ok(5000));
        assert_eq!(parse_scaled("50.0", PRICE_PRECISION), Ok(5000));
    }
    #[test]
    fn short_fraction_is_padded_not_left_aligned() {
        assert_eq!(parse_scaled("0.5", QTY_PRECISION), Ok(500));
        assert_eq!(parse_scaled("0.05", QTY_PRECISION), Ok(50));
        assert_eq!(parse_scaled("0.005", QTY_PRECISION), Ok(5));
    }
    #[test]
    fn zero_is_zero_in_every_spelling() {
        assert_eq!(parse_scaled("0", QTY_PRECISION), Ok(0));
        assert_eq!(parse_scaled("0.0", QTY_PRECISION), Ok(0));
        assert_eq!(parse_scaled("0.00000000", QTY_PRECISION), Ok(0));
    }
    #[test]
    fn trailing_zeros_beyond_precision_are_discarded() {
        assert_eq!(parse_scaled("1.1000", PRICE_PRECISION), Ok(110));
    }
    #[test]
    fn significant_digit_beyond_precision_is_rejected() {
        assert_eq!(parse_scaled("0.000234", QTY_PRECISION), Err(ParseError::TooPrecise));
        assert_eq!(parse_scaled("71455.655", PRICE_PRECISION), Err(ParseError::TooPrecise));
    }
    #[test]
    fn missing_fraction_after_point_is_malformed() {
        assert_eq!(parse_scaled("50.", PRICE_PRECISION), Err(ParseError::Malformed));
        assert_eq!(parse_scaled(".5", QTY_PRECISION), Err(ParseError::Malformed));
    }
    #[test]
    fn negatives_are_rejected_in_both_arms() {
        assert_eq!(parse_scaled("-50", PRICE_PRECISION), Err(ParseError::Malformed));
        assert_eq!(parse_scaled("-1.5", PRICE_PRECISION), Err(ParseError::Malformed));
    }
    #[test]
    fn non_numeric_input_is_malformed() {
        assert_eq!(parse_scaled("", PRICE_PRECISION), Err(ParseError::Malformed));
        assert_eq!(parse_scaled("abc", PRICE_PRECISION), Err(ParseError::Malformed));
        assert_eq!(parse_scaled("71455.6x", PRICE_PRECISION), Err(ParseError::Malformed));
        assert_eq!(parse_scaled("71455.6\r", PRICE_PRECISION), Err(ParseError::Malformed));
    }
    #[test]
    fn whitespace_is_not_silently_accepted() {
        assert_eq!(parse_scaled(" 71455.6", PRICE_PRECISION), Err(ParseError::Malformed));
        assert_eq!(parse_scaled("71455.6 ", PRICE_PRECISION), Err(ParseError::Malformed));
    }
    #[test]
    fn magnitude_headroom() {
        assert_eq!(parse_scaled("999999.9", PRICE_PRECISION), Ok(99999990));
    }
    #[test]
    fn double_decimal_point_is_malformed() {
        assert_eq!(parse_scaled("1.2.3", QTY_PRECISION), Err(ParseError::Malformed));
        assert_eq!(parse_scaled("1.2.3", PRICE_PRECISION), Err(ParseError::Malformed));
    }

    // Tests for parse_trade
    #[test]
    fn buyer_maker_test() {
        let buyer_maker: &str = "2072657703,71455.6,0.148,4735739403,4735739407,1710460800043,true";
        let trade = parse_trade(buyer_maker).unwrap();
        let trade_comp = Trade {
            timestamp: 1710460800043,
            price: 7145560,
            qty: 148,
            consumed: Side::Bid,
        };
        assert_eq!(trade, trade_comp);
    }
    #[test]
    fn seller_maker_test() {
        let seller_maker = "2072657704,71455.7,0.002,4735739408,4735739408,1710460800043,false";
        let trade = parse_trade(seller_maker).unwrap();
        let trade_comp = Trade {
            timestamp: 1710460800043,
            price: 7145570,
            qty: 2,
            consumed: Side::Ask,
        };
        assert_eq!(trade, trade_comp);
    }
    #[test]
    fn two_decimal_price_row_parses() {
        let line = "2072680187,71799.36,0.001,4735795907,4735795907,1710462181496,false";
        let trade = parse_trade(line).unwrap();
        assert_eq!(trade.price, 7179936);
        assert_eq!(trade.qty, 1);
        assert_eq!(trade.consumed, Side::Ask);
    }
    #[test]
    fn too_many_fields() {
        let malformed_line = "2072657704,71455.7,0.002,4735739408,4735739408,1710460800043,false,true";
        assert_eq!(parse_trade(malformed_line), Err(ParseError::FieldCount));
    }
    #[test]
    fn too_little_fields() {
        let malformed_line = "2072657704,4735739408,4735739408,1710460800043";
        assert_eq!(parse_trade(malformed_line), Err(ParseError::FieldCount));
    }
    #[test]
    fn no_fields() {
        let malformed_line = "";
        assert_eq!(parse_trade(malformed_line), Err(ParseError::FieldCount));
    }
    #[test]
    fn non_existent_side() {
        let malformed_line = "2072657704,71455.7,0.002,4735739408,4735739408,1710460800043,maybe";
        assert_eq!(parse_trade(malformed_line), Err(ParseError::Malformed));
    }
    #[test]
    fn malformed_timestamp() {
        let malformed_line = "2072657704,71455.7,0.002,4735739408,4735739408,abcd,true";
        assert_eq!(parse_trade(malformed_line), Err(ParseError::Malformed));
    }
    #[test]
    fn too_much_precision() {
        let buyer_maker: &str = "2072657703,71455.00000003453,0.148,4735739403,4735739407,1710460800043,true";
        assert_eq!(parse_trade(buyer_maker), Err(ParseError::TooPrecise));
    }
    #[test]
    fn header_parsing_error() {
        let buyer_maker: &str = "agg_trade_id,price,quantity,first_trade_id,last_trade_id,transact_time,is_buyer_maker";
        assert_eq!(parse_trade(buyer_maker), Err(ParseError::Malformed));
    }

    // Tests for parse_book_ticker
    #[test]
    fn book_ticker_real_line() {
        let line = "4183452554160,71455.50000000,2.12900000,71455.60000000,2.26000000,1710460800006,1710460800012";
        let expected = BookTicker {
            timestamp: 1710460800006,
            best_bid_price: 7145550,
            best_bid_qty: 2129,
            best_ask_price: 7145560,
            best_ask_qty: 2260,
        };
        assert_eq!(parse_book_ticker(line), Ok(expected));
    }
    #[test]
    fn book_ticker_uses_transaction_time_not_event_time() {
        let line = "4183452554171,71455.50000000,2.13900000,71455.60000000,2.26000000,1710460800006,1710460800013";
        let ticker = parse_book_ticker(line).unwrap();
        assert_eq!(ticker.timestamp, 1710460800006);
    }
    #[test]
    fn book_ticker_bid_and_ask_are_not_swapped() {
        let line = "4183452554412,71455.50000000,4.35500000,71455.60000000,2.25200000,1710460800009,1710460800016";
        let ticker = parse_book_ticker(line).unwrap();
        assert!(ticker.best_bid_price < ticker.best_ask_price);
        assert_eq!(ticker.best_bid_qty, 4355);
        assert_eq!(ticker.best_ask_qty, 2252);
    }
    #[test]
    fn book_ticker_too_many_fields() {
        let line = "4183452554160,71455.50000000,2.12900000,71455.60000000,2.26000000,1710460800006,1710460800012,9";
        assert_eq!(parse_book_ticker(line), Err(ParseError::FieldCount));
    }
    #[test]
    fn book_ticker_too_few_fields() {
        let line = "4183452554160,71455.50000000,2.12900000,71455.60000000";
        assert_eq!(parse_book_ticker(line), Err(ParseError::FieldCount));
    }
    #[test]
    fn book_ticker_empty_line() {
        assert_eq!(parse_book_ticker(""), Err(ParseError::FieldCount));
    }
    #[test]
    fn book_ticker_header_is_rejected() {
        let line = "update_id,best_bid_price,best_bid_qty,best_ask_price,best_ask_qty,transaction_time,event_time";
        assert_eq!(parse_book_ticker(line), Err(ParseError::Malformed));
    }
    #[test]
    fn book_ticker_malformed_timestamp() {
        let line = "4183452554160,71455.50000000,2.12900000,71455.60000000,2.26000000,abcd,1710460800012";
        assert_eq!(parse_book_ticker(line), Err(ParseError::Malformed));
    }
    #[test]
    fn book_ticker_excess_precision_propagates() {
        let line = "4183452554160,71455.555,2.12900000,71455.60000000,2.26000000,1710460800006,1710460800012";
        assert_eq!(parse_book_ticker(line), Err(ParseError::TooPrecise));
    }
}