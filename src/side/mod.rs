//! The bought and sold sides of the book.

/// Which side of the book a quantity rests on.
///
/// [`Side::Bid`] is the buy side and [`Side::Ask`] the sell side. An L2 book
/// keeps the two sides independent: the same price may carry quantity on both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
    /// Buy side: resting interest that wants to buy.
    Bid,
    /// Sell side: resting interest that wants to sell.
    Ask,
}

impl Side {
    /// Returns the other side.
    ///
    /// ```
    /// use orderbook_l2::Side;
    ///
    /// assert_eq!(Side::Bid.opposite(), Side::Ask);
    /// assert_eq!(Side::Ask.opposite(), Side::Bid);
    /// ```
    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Side::Bid => Side::Ask,
            Side::Ask => Side::Bid,
        }
    }
}

#[cfg(test)]
mod tests;
