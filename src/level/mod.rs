//! A single aggregated price level.

/// The total quantity resting at one price on one side of the book.
///
/// A level is the unit every read of [`OrderBook`](crate::OrderBook) hands back.
/// It carries no order identity: two orders at the same price are one level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Level<P, Q> {
    /// The price all quantity at this level rests at.
    pub price: P,
    /// The total quantity resting at [`Level::price`].
    pub quantity: Q,
}

#[cfg(test)]
mod tests;
