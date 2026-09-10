use crate::order::Side;

#[derive(Debug, Clone, Copy)]
struct Trade {
    timestamp: i64,
    price: i64,
    qty: i64,
    consumed: Side,
}

#[derive(Debug, Clone, Copy)]
struct BookTicker {
    timestamp: i64,
    best_bid_price: i64,
    best_bid_qty: i64,
    best_ask_price: i64,
    best_ask_qty: i64,
}

#[derive(Debug, PartialEq)]
enum ParseError {
    Malformed,
    TooPrecise,
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


#[cfg(test)]
mod tests {
    use super::*;
    const PRICE_PRECISION: u32 = 1;
    const QTY_PRECISION: u32 = 3;

    #[test]
    fn aggtrades_price_shape() {
        assert_eq!(parse_scaled("71455.6", PRICE_PRECISION), Ok(714556));
        assert_eq!(parse_scaled("71455.7", PRICE_PRECISION), Ok(714557));
    }
    #[test]
    fn bookticker_price_shape() {
        assert_eq!(parse_scaled("71455.50000000", PRICE_PRECISION), Ok(714555));
        assert_eq!(parse_scaled("71455.60000000", PRICE_PRECISION), Ok(714556));
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
        assert_eq!(parse_scaled("50", PRICE_PRECISION), Ok(500));
        assert_eq!(parse_scaled("50.0", PRICE_PRECISION), Ok(500));
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
        assert_eq!(parse_scaled("1.1000", PRICE_PRECISION), Ok(11));
    }
    #[test]
    fn significant_digit_beyond_precision_is_rejected() {
        assert_eq!(parse_scaled("0.000234", QTY_PRECISION), Err(ParseError::TooPrecise));
        assert_eq!(parse_scaled("71455.65", PRICE_PRECISION), Err(ParseError::TooPrecise));
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
        assert_eq!(parse_scaled("999999.9", PRICE_PRECISION), Ok(9999999));
    }

    #[test]
    fn double_decimal_point_is_malformed() {
        assert_eq!(parse_scaled("1.2.3", QTY_PRECISION), Err(ParseError::Malformed));
        assert_eq!(parse_scaled("1.2.3", PRICE_PRECISION), Err(ParseError::Malformed));
    }
}