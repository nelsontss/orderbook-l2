//! The aggregated limit order book and its query surface.

mod order_book;

#[cfg(feature = "experimental-hashmap")]
mod hash_order_book;

mod tick_book;
mod vec_order_book;

#[cfg(test)]
mod tests;

pub use order_book::OrderBook;

#[cfg(feature = "experimental-hashmap")]
pub use hash_order_book::HashOrderBook;

pub use tick_book::TickOrderBook;
pub use vec_order_book::VecOrderBook;
