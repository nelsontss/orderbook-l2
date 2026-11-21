//! A book stored as a price-indexed array.

use crate::{BookError, Level, Price, Quantity, Side};

/// The price range a [`TickOrderBook`] can address.
///
/// A grid is a coordinate frame: price `base + index * tick` lives at slot
/// `index`, for `index` in `0..len`. Prices that are not an exact multiple of
/// `tick`, or that fall outside the range, are not addressable. It stays
/// crate-private: callers configure a grid by passing its three parameters to
/// [`TickOrderBook::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TickGrid {
    base: i64,
    tick: i64,
    len: usize,
}

impl TickGrid {
    /// Builds a grid of `len` slots starting at `base` with step `tick`.
    ///
    /// Returns `None` when `tick` is not positive or the grid's span would
    /// overflow an `i64`.
    pub(super) fn new(base: i64, tick: i64, len: usize) -> Option<Self> {
        if tick <= 0 {
            return None;
        }

        let slots = i64::try_from(len).ok()?;
        let span = slots.checked_mul(tick)?;
        base.checked_add(span)?;

        Some(Self { base, tick, len })
    }

    pub(super) fn len(&self) -> usize {
        self.len
    }

    /// Returns the slot holding `price`, or `None` when it is not addressable.
    pub(super) fn index_of(&self, price: i64) -> Option<usize> {
        let offset = price.checked_sub(self.base)?;

        if offset < 0 || offset % self.tick != 0 {
            return None;
        }

        let index = usize::try_from(offset / self.tick).ok()?;

        (index < self.len).then_some(index)
    }
}

/// A limit order book storing each side as an array indexed by tick.
///
/// Prices are `i64` ticks of a fixed grid: `base + index * tick` for `index` in
/// `0..len`, declared at construction. Every operation is `O(1)` in the number
/// of levels — `post` and `remove` are a store plus a bitmap bit, [`level`](Self::level)
/// is an indexed load, and `best_bid`/`best_ask` are a handful of bitmap
/// instructions — which makes this the only backend with no cliff at either end
/// of the book.
///
/// The price is memory proportional to the grid's range rather than to the
/// number of levels it holds: 16 bytes per tick per side. Size the grid to the
/// band you actually trade; a grid far larger than the stored depth costs
/// iteration time, since a walk still steps over empty slots 64 at a time.
///
/// Use this when prices are integer ticks, the band is known, and writes need a
/// flat cost. [`OrderBook`](crate::OrderBook) accepts any
/// [`Price`](crate::Price); [`VecOrderBook`](crate::VecOrderBook) trades write
/// spikes for the fastest reads.
///
/// Shares the crate contract: no matching, no zero-quantity levels, failed
/// mutations leave the book untouched, one writer with totally ordered input.
///
/// # Examples
///
/// ```
/// use orderbook_l2::{Side, TickOrderBook};
///
/// let mut book: TickOrderBook = TickOrderBook::new(1_000, 1, 500).unwrap();
/// book.post(Side::Bid, 1_204, 3).unwrap();
/// book.post(Side::Ask, 1_206, 5).unwrap();
///
/// assert_eq!(book.best_bid().map(|level| level.price), Some(1_204));
/// assert_eq!(book.spread(), Some(2));
/// ```
#[derive(Debug, Clone)]
pub struct TickOrderBook<Q = u64> {
    grid: TickGrid,
    bids: Slots<Q>,
    asks: Slots<Q>,
}

impl<Q: Quantity> TickOrderBook<Q> {
    /// Creates an empty book addressing `len` ticks of `tick` starting at
    /// `base`.
    ///
    /// Returns `None` when `tick` is not positive or the grid's span would
    /// overflow an `i64`.
    #[must_use]
    pub fn new(base: i64, tick: i64, len: usize) -> Option<Self> {
        let grid = TickGrid::new(base, tick, len)?;
        let slots = grid.len();

        Some(Self {
            grid,
            bids: Slots::new(base, tick, slots),
            asks: Slots::new(base, tick, slots),
        })
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
    /// negative, and [`BookError::PriceOutsideGrid`] when `price` is not an
    /// exact tick inside the grid.
    pub fn post(&mut self, side: Side, price: i64, quantity: Q) -> Result<(), BookError> {
        if !quantity.is_positive() {
            return Err(BookError::QuantityNotPositive);
        }

        let index = self
            .grid
            .index_of(price)
            .ok_or(BookError::PriceOutsideGrid)?;

        let slots = self.slots_mut(side);
        let updated = slots.quantity(index) + quantity;
        slots.store(index, updated);

        Ok(())
    }

    /// Removes `quantity` from the level at `price` on `side`.
    ///
    /// # Errors
    ///
    /// Returns [`BookError::QuantityNotPositive`] when `quantity` is zero or
    /// negative, [`BookError::PriceOutsideGrid`] when `price` is not an exact
    /// tick inside the grid, [`BookError::UnknownLevel`] when no level exists
    /// at `price`, and [`BookError::InsufficientQuantity`] when the level
    /// holds less than `quantity`.
    pub fn remove(&mut self, side: Side, price: i64, quantity: Q) -> Result<(), BookError> {
        if !quantity.is_positive() {
            return Err(BookError::QuantityNotPositive);
        }

        let index = self
            .grid
            .index_of(price)
            .ok_or(BookError::PriceOutsideGrid)?;

        let slots = self.slots_mut(side);
        let current = slots.quantity(index);

        if current.is_zero() {
            return Err(BookError::UnknownLevel);
        }

        if current < quantity {
            return Err(BookError::InsufficientQuantity);
        }

        slots.store(index, current - quantity);

        Ok(())
    }

    /// Removes the whole level at `price` on `side` and returns it.
    ///
    /// Returns `None` when the price is not addressable or holds no level. Like
    /// [`OrderBook::remove_level`](crate::OrderBook::remove_level), this cannot
    /// fail.
    pub fn remove_level(&mut self, side: Side, price: i64) -> Option<Level<i64, Q>> {
        let index = self.grid.index_of(price)?;

        let slots = self.slots_mut(side);
        let current = slots.level(index);

        if current.quantity.is_zero() {
            return None;
        }

        slots.store(index, Q::default());

        Some(current)
    }

    /// Returns the highest bid level, or `None` when the bid side is empty.
    #[must_use]
    pub fn best_bid(&self) -> Option<Level<i64, Q>> {
        let index = self.bids.best(true)?;

        Some(self.bids.level(index))
    }

    /// Returns the lowest ask level, or `None` when the ask side is empty.
    #[must_use]
    pub fn best_ask(&self) -> Option<Level<i64, Q>> {
        let index = self.asks.best(false)?;

        Some(self.asks.level(index))
    }

    /// Returns `best_ask.price - best_bid.price`.
    #[must_use]
    pub fn spread(&self) -> Option<i64> {
        let bid = self.best_bid()?;
        let ask = self.best_ask()?;

        Some(ask.price - bid.price)
    }

    /// Returns the midpoint between the best bid and the best ask.
    #[must_use]
    pub fn mid(&self) -> Option<i64> {
        let bid = self.best_bid()?;
        let ask = self.best_ask()?;

        Some(bid.price.mid(ask.price))
    }

    /// Returns the level at `price` on `side`, or `None` when it does not exist.
    #[must_use]
    pub fn level(&self, side: Side, price: i64) -> Option<Level<i64, Q>> {
        let index = self.grid.index_of(price)?;
        let level = self.slots(side).level(index);

        (!level.quantity.is_zero()).then_some(level)
    }

    /// Iterates the bid side from the highest price down.
    pub fn bids(&self) -> impl Iterator<Item = Level<i64, Q>> + '_ {
        Occupied::new(&self.bids, true)
    }

    /// Iterates the ask side from the lowest price up.
    pub fn asks(&self) -> impl Iterator<Item = Level<i64, Q>> + '_ {
        Occupied::new(&self.asks, false)
    }

    /// Returns `true` when neither side holds a level.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bids.occupied == 0 && self.asks.occupied == 0
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
        (self.bids.occupied, self.asks.occupied)
    }

    fn slots(&self, side: Side) -> &Slots<Q> {
        match side {
            Side::Bid => &self.bids,
            Side::Ask => &self.asks,
        }
    }

    fn slots_mut(&mut self, side: Side) -> &mut Slots<Q> {
        match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        }
    }
}

#[derive(Debug, Clone)]
struct Slots<Q> {
    levels: Vec<Level<i64, Q>>,
    occupancy: Vec<u64>,
    summary: Vec<u64>,
    occupied: usize,
}

impl<Q: Quantity> Slots<Q> {
    fn new(base: i64, tick: i64, len: usize) -> Self {
        let mut levels = Vec::with_capacity(len);

        for index in 0..len {
            levels.push(Level {
                price: base + index as i64 * tick,
                quantity: Q::default(),
            });
        }

        let words = len.div_ceil(u64::BITS as usize);

        Self {
            levels,
            occupancy: vec![0; words],
            summary: vec![0; words.div_ceil(u64::BITS as usize)],
            occupied: 0,
        }
    }

    fn level(&self, index: usize) -> Level<i64, Q> {
        self.levels[index]
    }

    fn quantity(&self, index: usize) -> Q {
        self.levels[index].quantity
    }

    fn store(&mut self, index: usize, quantity: Q) {
        let was_empty = self.levels[index].quantity.is_zero();
        let is_empty = quantity.is_zero();

        if was_empty != is_empty {
            self.write_bit(index, !is_empty);

            if is_empty {
                self.occupied -= 1;
            } else {
                self.occupied += 1;
            }
        }

        self.levels[index].quantity = quantity;
    }

    fn write_bit(&mut self, index: usize, occupied: bool) {
        let word = index / u64::BITS as usize;
        let bit = 1u64 << (index % u64::BITS as usize);
        let group = word / u64::BITS as usize;
        let flag = 1u64 << (word % u64::BITS as usize);

        if occupied {
            self.occupancy[word] |= bit;
            self.summary[group] |= flag;
        } else {
            self.occupancy[word] &= !bit;

            if self.occupancy[word] == 0 {
                self.summary[group] &= !flag;
            }
        }
    }

    fn clear(&mut self) {
        for level in &mut self.levels {
            level.quantity = Q::default();
        }

        self.occupancy.fill(0);
        self.summary.fill(0);
        self.occupied = 0;
    }

    fn extreme_word(&self, descending: bool) -> Option<usize> {
        let group = if descending {
            self.summary.iter().rposition(|bits| *bits != 0)?
        } else {
            self.summary.iter().position(|bits| *bits != 0)?
        };

        let flags = self.summary[group];
        let flag = if descending {
            flags.ilog2()
        } else {
            flags.trailing_zeros()
        } as usize;

        Some(group * u64::BITS as usize + flag)
    }

    fn best(&self, descending: bool) -> Option<usize> {
        let group = if descending {
            self.summary.iter().rposition(|bits| *bits != 0)?
        } else {
            self.summary.iter().position(|bits| *bits != 0)?
        };

        let flags = self.summary[group];
        let flag = if descending {
            flags.ilog2()
        } else {
            flags.trailing_zeros()
        } as usize;
        let word = group * u64::BITS as usize + flag;

        let bits = self.occupancy[word];
        let bit = if descending {
            bits.ilog2()
        } else {
            bits.trailing_zeros()
        } as usize;

        Some(word * u64::BITS as usize + bit)
    }
}

struct Occupied<'a, Q> {
    slots: &'a Slots<Q>,
    word: usize,
    bits: u64,
    descending: bool,
    run: usize,
    run_left: usize,
}

impl<'a, Q: Quantity> Occupied<'a, Q> {
    fn new(slots: &'a Slots<Q>, descending: bool) -> Self {
        let word = if descending {
            slots.extreme_word(true).map_or(0, |word| word + 1)
        } else {
            slots.extreme_word(false).unwrap_or(slots.occupancy.len())
        };

        Self {
            slots,
            word,
            bits: 0,
            descending,
            run: 0,
            run_left: 0,
        }
    }
}

impl<Q: Quantity> Iterator for Occupied<'_, Q> {
    type Item = Level<i64, Q>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.run_left > 0 {
                let index = self.run;
                self.run_left -= 1;
                self.run = if self.descending {
                    self.run.wrapping_sub(1)
                } else {
                    self.run + 1
                };

                return Some(self.slots.levels[index]);
            }

            if self.bits == 0 {
                if self.descending {
                    if self.word == 0 {
                        return None;
                    }

                    self.word -= 1;
                } else {
                    if self.word >= self.slots.occupancy.len() {
                        return None;
                    }
                }

                self.bits = self.slots.occupancy[self.word];

                if !self.descending {
                    self.word += 1;
                }

                continue;
            }

            if self.bits == u64::MAX {
                let word = if self.descending {
                    self.word
                } else {
                    self.word - 1
                };
                let first = word * u64::BITS as usize;

                self.run = if self.descending {
                    first + (u64::BITS as usize - 1)
                } else {
                    first
                };
                self.run_left = u64::BITS as usize;
                self.bits = 0;

                continue;
            }

            let bit = if self.descending {
                self.bits.ilog2() as usize
            } else {
                self.bits.trailing_zeros() as usize
            };

            self.bits &= !(1u64 << bit);

            let word = if self.descending {
                self.word
            } else {
                self.word - 1
            };
            let index = word * u64::BITS as usize + bit;

            return Some(self.slots.levels[index]);
        }
    }
}
