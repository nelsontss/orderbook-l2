//! A book stored as one sorted `Vec` per side.

use std::ops::Sub;

use crate::{BookError, Level, Price, Quantity, Side};

/// A limit order book storing each side as a sorted `Vec`.
///
/// Both sides are held worst-price-first — bids ascending, asks descending — so
/// the best level is the last element and `push`/`pop` serve the touch. Reads
/// are contiguous scans over [`Level`]s, which makes this the fastest backend
/// for depth queries: a full walk is roughly nine times faster than
/// [`OrderBook`](crate::OrderBook)'s and truncated reads about seven times.
///
/// The trade is the far end. Creating a level beyond a side's current extreme
/// shifts every element, `O(n)`; at ten thousand levels that is a
/// multi-microsecond spike against a twelve-nanosecond push at the other end.
/// Use this book when reads dominate and the traded range is stable — prefer
/// [`OrderBook`](crate::OrderBook) when writes can land anywhere, and
/// [`TickOrderBook`](crate::TickOrderBook) when every write must be constant
/// time.
///
/// Shares the crate contract: no matching, no zero-quantity levels, failed
/// mutations leave the book untouched, one writer with totally ordered input.
#[derive(Debug, Clone)]
pub struct VecOrderBook<P = i64, Q = u64> {
    bids: Vec<Level<P, Q>>,
    asks: Vec<Level<P, Q>>,
}

impl<P: Copy + Ord + Sub<Output = P>, Q: Quantity> VecOrderBook<P, Q> {
    /// Creates an empty book.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }

    /// Removes every level from both sides.
    pub fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
    }

    /// Adds `quantity` to the level at `price` on `side`.
    ///
    /// # Errors
    ///
    /// Returns [`BookError::QuantityNotPositive`] when `quantity` is zero or
    /// negative.
    pub fn post(&mut self, side: Side, price: P, quantity: Q) -> Result<(), BookError> {
        if !quantity.is_positive() {
            return Err(BookError::QuantityNotPositive);
        }

        let levels = match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        match position(levels, side, price) {
            Ok(index) => {
                let updated = levels[index].quantity + quantity;
                levels[index].quantity = updated;
            }
            Err(index) => levels.insert(index, Level { price, quantity }),
        }

        Ok(())
    }

    /// Removes `quantity` from the level at `price` on `side`.
    ///
    /// # Errors
    ///
    /// Returns [`BookError::QuantityNotPositive`] when `quantity` is zero or
    /// negative, [`BookError::UnknownLevel`] when no level exists at `price`,
    /// and [`BookError::InsufficientQuantity`] when the level holds less than
    /// `quantity`.
    pub fn remove(&mut self, side: Side, price: P, quantity: Q) -> Result<(), BookError> {
        if !quantity.is_positive() {
            return Err(BookError::QuantityNotPositive);
        }

        let levels = match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        let index = position(levels, side, price).map_err(|_| BookError::UnknownLevel)?;
        let current = levels[index].quantity;

        if current < quantity {
            return Err(BookError::InsufficientQuantity);
        }

        let remaining = current - quantity;

        if remaining.is_zero() {
            levels.remove(index);
        } else {
            levels[index].quantity = remaining;
        }

        Ok(())
    }

    /// Removes the whole level at `price` on `side` and returns it.
    pub fn remove_level(&mut self, side: Side, price: P) -> Option<Level<P, Q>> {
        let levels = match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        let index = position(levels, side, price).ok()?;

        Some(levels.remove(index))
    }

    /// Replaces the level at `price` on `side` with `quantity`.
    ///
    /// A quantity of zero removes the level. This is the shape exchange feeds
    /// publish when they carry absolute size rather than deltas, so a feed
    /// handler does not have to pair `remove_level` with `post`.
    ///
    /// # Errors
    ///
    /// Returns [`BookError::QuantityNotPositive`] when `quantity` is negative.
    pub fn set_level(&mut self, side: Side, price: P, quantity: Q) -> Result<(), BookError> {
        if quantity < Q::default() {
            return Err(BookError::QuantityNotPositive);
        }

        let levels = match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        match position(levels, side, price) {
            Ok(index) => {
                if quantity.is_zero() {
                    levels.remove(index);
                } else {
                    levels[index].quantity = quantity;
                }
            }
            Err(index) => {
                if !quantity.is_zero() {
                    levels.insert(index, Level { price, quantity });
                }
            }
        }

        Ok(())
    }

    /// Returns the highest bid level, or `None` when the bid side is empty.
    #[must_use]
    pub fn best_bid(&self) -> Option<Level<P, Q>> {
        self.bids.last().copied()
    }

    /// Returns the lowest ask level, or `None` when the ask side is empty.
    #[must_use]
    pub fn best_ask(&self) -> Option<Level<P, Q>> {
        self.asks.last().copied()
    }

    /// Returns `best_ask.price - best_bid.price`.
    #[must_use]
    pub fn spread(&self) -> Option<P> {
        let bid = self.best_bid()?;
        let ask = self.best_ask()?;

        Some(ask.price - bid.price)
    }

    /// Returns the level at `price` on `side`, or `None` when it does not exist.
    #[must_use]
    pub fn level(&self, side: Side, price: P) -> Option<Level<P, Q>> {
        let levels = match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        };

        let index = position(levels, side, price).ok()?;

        levels.get(index).copied()
    }

    /// Iterates the bid side from the highest price down.
    pub fn bids(&self) -> impl Iterator<Item = Level<P, Q>> + '_ {
        self.bids.iter().rev().copied()
    }

    /// Iterates the ask side from the lowest price up.
    pub fn asks(&self) -> impl Iterator<Item = Level<P, Q>> + '_ {
        self.asks.iter().rev().copied()
    }

    /// Returns `true` when neither side holds a level.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bids.is_empty() && self.asks.is_empty()
    }

    /// Returns `true` when the best bid is at or above the best ask.
    #[must_use]
    pub fn is_crossed(&self) -> bool {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => bid.price >= ask.price,
            _ => false,
        }
    }

    /// Returns `(bid levels, ask levels)`.
    #[must_use]
    pub fn depth(&self) -> (usize, usize) {
        (self.bids.len(), self.asks.len())
    }
}

impl<P: Price, Q: Quantity> VecOrderBook<P, Q> {
    /// Returns the midpoint between the best bid and the best ask.
    ///
    /// This is the only method that requires [`Price`]; every other operation
    /// needs just `Copy + Ord + Sub`.
    #[must_use]
    pub fn mid(&self) -> Option<P> {
        let bid = self.best_bid()?;
        let ask = self.best_ask()?;

        Some(bid.price.mid(ask.price))
    }
}

impl<P: Copy + Ord + Sub<Output = P>, Q: Quantity> Default for VecOrderBook<P, Q> {
    fn default() -> Self {
        Self::new()
    }
}

fn position<P: Ord, Q>(levels: &[Level<P, Q>], side: Side, price: P) -> Result<usize, usize> {
    match side {
        Side::Bid => levels.binary_search_by(|level| level.price.cmp(&price)),
        Side::Ask => levels.binary_search_by(|level| price.cmp(&level.price)),
    }
}
