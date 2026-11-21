use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use crate::{BookError, Level, Price, Quantity, Side};

/// An aggregated, exchange-agnostic L2 limit order book.
///
/// The book holds a quantity per price per side. It has no notion of an
/// individual order: quantity posted at a price is indistinguishable from any
/// other quantity at that price. Two generic parameters keep the storage
/// independent of the caller's numeric types, defaulting to `i64` prices and
/// `u64` quantities.
///
/// # Contract
///
/// - **The book never matches.** [`post`](Self::post) adds quantity to a level
///   and never consumes the opposite side, so a crossed book is representable
///   and observable through [`is_crossed`](Self::is_crossed). Matching belongs
///   to the caller.
/// - **The only self-initiated mutation** is dropping a level whose quantity
///   reaches zero. No level ever holds zero quantity.
/// - **Prices are never validated.** Negative prices are permitted.
/// - **Reads return `Option`, never `Result`.** An empty or one-sided book is a
///   normal state, not an error.
/// - **Failed mutations leave the book untouched**, so a caller may treat an
///   error as a no-op.
/// - **One writer, totally ordered input.** Mutation takes `&mut self`, so the
///   book cannot be written concurrently. Ordering incoming interest and
///   deciding whether it matches are the caller's responsibilities; a book fed
///   the same ordered stream always reaches the same state.
///
/// # Examples
///
/// ```no_run
/// use orderbook_l2::{Level, OrderBook, Side};
///
/// # fn main() -> Result<(), orderbook_l2::BookError> {
/// let mut book: OrderBook = OrderBook::new();
/// book.post(Side::Bid, 100, 5)?;
/// book.post(Side::Bid, 99, 8)?;
/// book.post(Side::Ask, 101, 4)?;
///
/// assert_eq!(book.best_bid(), Some(Level { price: 100, quantity: 5 }));
/// assert_eq!(book.best_ask(), Some(Level { price: 101, quantity: 4 }));
/// assert_eq!(book.spread(), Some(1));
/// assert_eq!(book.mid(), Some(100));
///
/// let prices: Vec<i64> = book.bids().map(|level| level.price).collect();
/// assert_eq!(prices, [100, 99]);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct OrderBook<P = i64, Q = u64> {
    bids: BTreeMap<Reverse<P>, Q>,
    asks: BTreeMap<P, Q>,
}

impl<P: Price, Q: Quantity> OrderBook<P, Q> {
    /// Creates an empty book.
    ///
    /// # Examples
    ///
    /// ```
    /// use orderbook_l2::OrderBook;
    ///
    /// let book: OrderBook = OrderBook::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    /// Removes every level from both sides.
    pub fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
    }

    /// Adds `quantity` to the level at `price` on `side`.
    ///
    /// Creates the level when it does not exist, otherwise accumulates into it.
    /// This never matches against the opposite side and never rejects a price
    /// that crosses the spread.
    ///
    /// # Errors
    ///
    /// Returns [`BookError::QuantityNotPositive`] when `quantity` is zero or
    /// negative, leaving the book untouched.
    pub fn post(&mut self, side: Side, price: P, quantity: Q) -> Result<(), BookError> {
        if !quantity.is_positive() {
            return Err(BookError::QuantityNotPositive);
        }

        match side {
            Side::Bid => accumulate(self.bids.entry(Reverse(price)).or_default(), quantity),
            Side::Ask => accumulate(self.asks.entry(price).or_default(), quantity),
        }

        Ok(())
    }

    /// Removes `quantity` from the level at `price` on `side`.
    ///
    /// A level whose remaining quantity reaches zero is dropped, so no level
    /// ever holds zero quantity.
    ///
    /// # Errors
    ///
    /// Returns [`BookError::QuantityNotPositive`] when `quantity` is zero or
    /// negative, [`BookError::UnknownLevel`] when no level exists at `price` on
    /// `side`, and [`BookError::InsufficientQuantity`] when the level holds
    /// less than `quantity`. The book is untouched in every error case.
    pub fn remove(&mut self, side: Side, price: P, quantity: Q) -> Result<(), BookError> {
        if !quantity.is_positive() {
            return Err(BookError::QuantityNotPositive);
        }

        match side {
            Side::Bid => decrement(&mut self.bids, Reverse(price), quantity),
            Side::Ask => decrement(&mut self.asks, price, quantity),
        }
    }

    /// Removes the whole level at `price` on `side` and returns it.
    ///
    /// Returns `None` when no level exists there. Because prices are never
    /// validated, this cannot fail.
    pub fn remove_level(&mut self, side: Side, price: P) -> Option<Level<P, Q>> {
        match side {
            Side::Bid => self
                .bids
                .remove_entry(&Reverse(price))
                .map(|(key, quantity)| Level {
                    price: key.0,
                    quantity,
                }),
            Side::Ask => self.asks.remove_entry(&price).map(|(key, quantity)| Level {
                price: key,
                quantity,
            }),
        }
    }

    /// Returns the highest bid level, or `None` when the bid side is empty.
    #[must_use]
    pub fn best_bid(&self) -> Option<Level<P, Q>> {
        self.bids
            .first_key_value()
            .map(|(&Reverse(price), &quantity)| Level { price, quantity })
    }

    /// Returns the lowest ask level, or `None` when the ask side is empty.
    #[must_use]
    pub fn best_ask(&self) -> Option<Level<P, Q>> {
        self.asks
            .first_key_value()
            .map(|(&price, &quantity)| Level { price, quantity })
    }

    /// Returns `best_ask.price - best_bid.price`.
    ///
    /// `None` unless both sides hold at least one level. The result is negative
    /// when the book is crossed; with an unsigned price type that subtraction
    /// underflows, so use a signed price type if crossings are possible.
    #[must_use]
    pub fn spread(&self) -> Option<P> {
        let bid = self.best_bid()?;
        let ask = self.best_ask()?;

        Some(ask.price - bid.price)
    }

    /// Returns the midpoint between the best bid and the best ask, as computed
    /// by [`Price::mid`].
    ///
    /// `None` unless both sides hold at least one level. Integer prices floor
    /// when the two best prices sum to an odd number, so a `100`/`101` book has
    /// a mid of `100`.
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
            Side::Bid => self.bids.get(&Reverse(price)),
            Side::Ask => self.asks.get(&price),
        };

        quantity.map(|&quantity| Level { price, quantity })
    }

    /// Iterates the bid side from the highest price down.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use orderbook_l2::{OrderBook, Side};
    ///
    /// # fn main() -> Result<(), orderbook_l2::BookError> {
    /// let mut book: OrderBook = OrderBook::new();
    /// book.post(Side::Bid, 100, 1)?;
    /// book.post(Side::Bid, 98, 2)?;
    ///
    /// let prices: Vec<i64> = book.bids().map(|level| level.price).collect();
    /// assert_eq!(prices, [100, 98]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn bids(&self) -> impl Iterator<Item = Level<P, Q>> + '_ {
        self.bids
            .iter()
            .map(|(&Reverse(price), &quantity)| Level { price, quantity })
    }

    /// Iterates the ask side from the lowest price up.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use orderbook_l2::{OrderBook, Side};
    ///
    /// # fn main() -> Result<(), orderbook_l2::BookError> {
    /// let mut book: OrderBook = OrderBook::new();
    /// book.post(Side::Ask, 101, 1)?;
    /// book.post(Side::Ask, 103, 2)?;
    ///
    /// let prices: Vec<i64> = book.asks().map(|level| level.price).collect();
    /// assert_eq!(prices, [101, 103]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn asks(&self) -> impl Iterator<Item = Level<P, Q>> + '_ {
        self.asks
            .iter()
            .map(|(&price, &quantity)| Level { price, quantity })
    }

    /// Returns `true` when neither side holds a level.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.depth() == (0, 0)
    }

    /// Returns `true` when the best bid is at or above the best ask.
    ///
    /// Returns `false` when either side is empty. A bid exactly equal to the
    /// ask counts as crossed, because the two sides would have traded.
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

pub(super) fn accumulate<Q: Quantity>(total: &mut Q, quantity: Q) {
    let updated = *total + quantity;
    *total = updated;
}

fn decrement<K: Ord, Q: Quantity>(
    map: &mut BTreeMap<K, Q>,
    key: K,
    quantity: Q,
) -> Result<(), BookError> {
    match map.entry(key) {
        Entry::Occupied(mut entry) => {
            if *entry.get() < quantity {
                return Err(BookError::InsufficientQuantity);
            }

            let remaining = *entry.get() - quantity;

            if remaining.is_zero() {
                entry.remove();
            } else {
                *entry.get_mut() = remaining;
            }

            Ok(())
        }
        Entry::Vacant(_) => Err(BookError::UnknownLevel),
    }
}

impl<P: Price, Q: Quantity> Default for OrderBook<P, Q> {
    fn default() -> Self {
        Self::new()
    }
}
