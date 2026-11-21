//! An experimental hash-map-backed book, built to be measured against
//! [`OrderBook`](crate::OrderBook).

use std::collections::HashMap;
use std::hash::Hash;

use super::order_book::accumulate;
use crate::{BookError, Level, Price, Quantity, Side};

/// An experimental [`OrderBook`](crate::OrderBook) backed by two hash maps.
///
/// This type exists to answer one question with numbers instead of guesses:
/// does O(1) keyed access beat an ordered tree once the whole API is
/// implemented? It is not covered by the crate's stability promise and should
/// not be depended on.
///
/// Its design differs from [`OrderBook`](crate::OrderBook) in exactly the places
/// a hash map forces:
///
/// - **Best prices are maintained, not derived.** Each side keeps a cached best
///   price, repaired by a full scan when the best level is emptied.
/// - **Depth iteration sorts on every call.** `bids` and `asks` collect and
///   sort, so each call allocates and costs `O(n log n)` regardless of how many
///   levels the caller consumes.
///
/// Both types share the same contract: no matching, no zero-quantity levels,
/// failed mutations leave the book untouched, one writer with totally ordered
/// input.
#[derive(Debug, Clone)]
pub struct HashOrderBook<P = i64, Q = u64> {
    bids: HashMap<P, Q>,
    asks: HashMap<P, Q>,
    best_bid: Option<P>,
    best_ask: Option<P>,
}

impl<P: Price + Hash, Q: Quantity> HashOrderBook<P, Q> {
    /// Creates an empty book.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bids: HashMap::new(),
            asks: HashMap::new(),
            best_bid: None,
            best_ask: None,
        }
    }

    /// Removes every level from both sides.
    pub fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
        self.best_bid = None;
        self.best_ask = None;
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

        match side {
            Side::Bid => {
                accumulate(self.bids.entry(price).or_default(), quantity);

                if self.best_bid.is_none_or(|best| price > best) {
                    self.best_bid = Some(price);
                }
            }
            Side::Ask => {
                accumulate(self.asks.entry(price).or_default(), quantity);

                if self.best_ask.is_none_or(|best| price < best) {
                    self.best_ask = Some(price);
                }
            }
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

        match side {
            Side::Bid => {
                if shrink(&mut self.bids, price, quantity)? && self.best_bid == Some(price) {
                    self.best_bid = self.bids.keys().copied().max();
                }
            }
            Side::Ask => {
                if shrink(&mut self.asks, price, quantity)? && self.best_ask == Some(price) {
                    self.best_ask = self.asks.keys().copied().min();
                }
            }
        }

        Ok(())
    }

    /// Removes the whole level at `price` on `side` and returns it.
    pub fn remove_level(&mut self, side: Side, price: P) -> Option<Level<P, Q>> {
        let removed = match side {
            Side::Bid => {
                let removed = self.bids.remove(&price);

                if removed.is_some() && self.best_bid == Some(price) {
                    self.best_bid = self.bids.keys().copied().max();
                }

                removed
            }
            Side::Ask => {
                let removed = self.asks.remove(&price);

                if removed.is_some() && self.best_ask == Some(price) {
                    self.best_ask = self.asks.keys().copied().min();
                }

                removed
            }
        };

        removed.map(|quantity| Level { price, quantity })
    }

    /// Returns the highest bid level, or `None` when the bid side is empty.
    #[must_use]
    pub fn best_bid(&self) -> Option<Level<P, Q>> {
        let price = self.best_bid?;
        let &quantity = self.bids.get(&price)?;

        Some(Level { price, quantity })
    }

    /// Returns the lowest ask level, or `None` when the ask side is empty.
    #[must_use]
    pub fn best_ask(&self) -> Option<Level<P, Q>> {
        let price = self.best_ask?;
        let &quantity = self.asks.get(&price)?;

        Some(Level { price, quantity })
    }

    /// Returns `best_ask.price - best_bid.price`.
    #[must_use]
    pub fn spread(&self) -> Option<P> {
        let bid = self.best_bid()?;
        let ask = self.best_ask()?;

        Some(ask.price - bid.price)
    }

    /// Returns the midpoint between the best bid and the best ask.
    #[must_use]
    pub fn mid(&self) -> Option<P> {
        let bid = self.best_bid()?;
        let ask = self.best_ask()?;

        Some(bid.price.mid(ask.price))
    }

    /// Returns the level at `price` on `side`, or `None` when it does not exist.
    #[must_use]
    pub fn level(&self, side: Side, price: P) -> Option<Level<P, Q>> {
        let quantity = match side {
            Side::Bid => self.bids.get(&price),
            Side::Ask => self.asks.get(&price),
        };

        quantity.map(|&quantity| Level { price, quantity })
    }

    /// Iterates the bid side from the highest price down, sorting on each call.
    pub fn bids(&self) -> impl Iterator<Item = Level<P, Q>> + '_ {
        let mut levels = self.collect(Side::Bid);
        levels.sort_unstable_by(|left, right| right.price.cmp(&left.price));
        levels.into_iter()
    }

    /// Iterates the ask side from the lowest price up, sorting on each call.
    pub fn asks(&self) -> impl Iterator<Item = Level<P, Q>> + '_ {
        let mut levels = self.collect(Side::Ask);
        levels.sort_unstable_by(|left, right| left.price.cmp(&right.price));
        levels.into_iter()
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

    fn collect(&self, side: Side) -> Vec<Level<P, Q>> {
        let map = match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        };

        map.iter()
            .map(|(&price, &quantity)| Level { price, quantity })
            .collect()
    }
}

impl<P: Price + Hash, Q: Quantity> Default for HashOrderBook<P, Q> {
    fn default() -> Self {
        Self::new()
    }
}

fn shrink<K: Eq + Hash, Q: Quantity>(
    map: &mut HashMap<K, Q>,
    key: K,
    quantity: Q,
) -> Result<bool, BookError> {
    let Some(&current) = map.get(&key) else {
        return Err(BookError::UnknownLevel);
    };

    if current < quantity {
        return Err(BookError::InsufficientQuantity);
    }

    let remaining = current - quantity;

    if remaining.is_zero() {
        map.remove(&key);
        Ok(true)
    } else {
        map.insert(key, remaining);
        Ok(false)
    }
}
