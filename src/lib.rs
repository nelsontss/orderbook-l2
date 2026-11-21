//! A generic, exchange-agnostic limit order book.
//!
//! This crate models an **L2** book: a ladder of price levels, each holding an
//! aggregated quantity, with no per-order identity. Posting adds quantity to a
//! level; matching is the caller's responsibility.
//!
//! The book never matches and never rejects a crossing price. See
//! [`OrderBook`] for the full contract.
//!
//! # Choosing a book
//!
//! Three books share the contract and the vocabulary; they differ in price model
//! and cost profile.
//!
//! | Book | Wins | Costs |
//! |---|---|---|
//! | [`OrderBook`] | any [`Price`], no configuration | nothing is fastest, nothing spikes |
//! | [`VecOrderBook`] | depth reads | writing beyond a side's extreme is `O(n)` |
//! | [`TickOrderBook`] | writes, point lookups | 16 B per tick, and a grid you must size |
//!
//! [`OrderBook`] is the default. [`VecOrderBook`] is for read-dominated work
//! over a stable range. [`TickOrderBook`] is for tick-quantised integer feeds
//! whose writes must never spike.
//!
//! ```no_run
//! use orderbook_l2::{OrderBook, Side};
//!
//! # fn main() -> Result<(), orderbook_l2::BookError> {
//! let mut book: OrderBook = OrderBook::new();
//! book.post(Side::Bid, 100, 5)?;
//! book.post(Side::Ask, 101, 3)?;
//!
//! assert_eq!(book.best_bid().map(|level| level.price), Some(100));
//! assert_eq!(book.best_ask().map(|level| level.price), Some(101));
//! assert_eq!(book.mid(), Some(100));
//! assert_eq!(book.spread(), Some(1));
//! # Ok(())
//! # }
//! ```

pub mod book;
pub mod error;
pub mod level;
pub mod side;
pub mod traits;

#[cfg(feature = "experimental-hashmap")]
pub use book::HashOrderBook;
pub use book::{OrderBook, TickOrderBook, VecOrderBook};
pub use error::BookError;
pub use level::Level;
pub use side::Side;
pub use traits::{Price, Quantity};
