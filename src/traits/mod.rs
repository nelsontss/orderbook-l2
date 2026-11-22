//! Numeric capabilities the book requires of prices and quantities.
//!
//! Prices and quantities are separate generic parameters with separate traits:
//! prices are ordered and subtracted, quantities are totalled and decremented.
//! Keeping them apart prevents expressions that are dimensionally meaningless.

use core::ops::{Add, Sub};

/// A quantity the book can total, decrement, and compare.
///
/// Blanket-implemented for every type that is `Copy + Ord + Default` and
/// supports `Add` and `Sub`, so `u64`, `i64`, and fixed-point decimals need no
/// wiring. [`Default`] is treated as zero, which is why unsigned types work.
///
/// ```
/// use orderbook_l2::Quantity;
///
/// assert!(1_000_u64.is_positive());
/// assert!(0_u64.is_zero());
/// assert!(!(-2_i64).is_positive());
/// ```
pub trait Quantity: Copy + Ord + Default + Add<Output = Self> + Sub<Output = Self> {
    /// Returns `true` when this quantity equals [`Default::default`].
    #[must_use]
    fn is_zero(&self) -> bool {
        *self == Self::default()
    }

    /// Returns `true` when this quantity is strictly greater than
    /// [`Default::default`].
    #[must_use]
    fn is_positive(&self) -> bool {
        *self > Self::default()
    }
}

impl<T> Quantity for T where T: Copy + Ord + Default + Add<Output = T> + Sub<Output = T> {}

/// A price the book can order, subtract, and halve.
///
/// Only [`OrderBook::mid`](crate::OrderBook::mid) requires this trait. Every
/// other operation is bounded by `Copy + Ord + Sub`, so a price type with no
/// midpoint still gets the whole API except `mid`, and types from third-party
/// crates need only the [`Sub`] impl they already have.
///
/// Deliberately not implemented for `f64`: [`Ord`] is required, and `NaN` has no
/// place in a price ladder. Implement this trait for a fixed-point or decimal
/// newtype instead; the only method to supply is [`Price::mid`].
///
/// The method is named `mid` rather than `midpoint` because integers gained an
/// inherent `midpoint` in the standard library, which would silently shadow a
/// trait method of that name.
///
/// Negative prices are permitted. Some real instruments trade below zero, so
/// the book never validates a price's sign.
pub trait Price: Copy + Ord + Sub<Output = Self> {
    /// Returns the point halfway between `self` and `other`.
    ///
    /// The result must lie between the two inputs and must never overflow.
    /// When the exact midpoint is not representable, implementations floor
    /// toward the smaller value, so `100_i64.mid(101)` is `100`.
    ///
    /// ```
    /// use orderbook_l2::Price;
    ///
    /// assert_eq!(100_i64.mid(101), 100);
    /// assert_eq!(i64::MIN.mid(i64::MAX), -1);
    /// assert_eq!(0_u64.mid(u64::MAX), u64::MAX / 2);
    /// ```
    #[must_use]
    fn mid(self, other: Self) -> Self;
}

#[cfg(feature = "decimal")]
impl Price for rust_decimal::Decimal {
    fn mid(self, other: Self) -> Self {
        let (low, high) = if self <= other {
            (self, other)
        } else {
            (other, self)
        };

        low + (high - low) / rust_decimal::Decimal::TWO
    }
}

macro_rules! impl_price {
    ($($price:ty),+ $(,)?) => {
        $(
            impl Price for $price {
                fn mid(self, other: Self) -> Self {
                    (self & other) + ((self ^ other) >> 1)
                }
            }
        )+
    };
}

impl_price!(i64, u64, i128, u128);

#[cfg(test)]
mod tests;
