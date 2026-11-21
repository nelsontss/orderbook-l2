//! Rejections returned by [`OrderBook`](crate::OrderBook) mutations.

use core::fmt;

/// Why a mutation of [`OrderBook`](crate::OrderBook) was rejected.
///
/// The variants are intentionally payload-free: a generic error carrying the
/// offending price or quantity would force `P` and `Q` into every signature in
/// the crate. Callers already hold the values they passed in.
///
/// ```
/// use orderbook_l2::{BookError, OrderBook, Side};
///
/// let mut book: OrderBook = OrderBook::new();
///
/// assert_eq!(
///     book.post(Side::Bid, 100, 0),
///     Err(BookError::QuantityNotPositive),
/// );
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BookError {
    /// The supplied quantity was zero or negative.
    QuantityNotPositive,
    /// No level exists at the supplied price on the supplied side.
    UnknownLevel,
    /// The level holds less quantity than the removal asked for.
    InsufficientQuantity,
    /// The price is not an exact tick inside the book's grid.
    PriceOutsideGrid,
}

impl fmt::Display for BookError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            BookError::QuantityNotPositive => "quantity must be strictly positive",
            BookError::UnknownLevel => "no level exists at the given price",
            BookError::InsufficientQuantity => "level holds less quantity than requested",
            BookError::PriceOutsideGrid => "price is not an exact tick inside the grid",
        };

        formatter.write_str(message)
    }
}

impl core::error::Error for BookError {}

#[cfg(test)]
mod tests;
