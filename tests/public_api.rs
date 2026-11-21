//! End-to-end usage of the crate's public API.

use orderbook_l2::{BookError, Level, OrderBook, Side};

fn seeded() -> OrderBook {
    let mut book: OrderBook = OrderBook::new();

    for (price, quantity) in [(100_i64, 5_u64), (99, 8), (98, 12)] {
        book.post(Side::Bid, price, quantity).unwrap();
    }

    for (price, quantity) in [(101_i64, 4_u64), (102, 6), (103, 9)] {
        book.post(Side::Ask, price, quantity).unwrap();
    }

    book
}

#[test]
fn read_the_top_of_book() {
    let book = seeded();

    assert_eq!(
        book.best_bid(),
        Some(Level {
            price: 100,
            quantity: 5,
        }),
    );
    assert_eq!(
        book.best_ask(),
        Some(Level {
            price: 101,
            quantity: 4,
        }),
    );
    assert_eq!(book.spread(), Some(1));
    assert_eq!(book.mid(), Some(100));
    assert!(!book.is_crossed());
    assert_eq!(book.depth(), (3, 3));
}

#[test]
fn walk_the_ladder_to_size_an_order() {
    let book = seeded();

    let asks: Vec<Level<i64, u64>> = book.asks().collect();
    let mut remaining = 9_u64;
    let mut cost = 0_i64;
    let mut filled = 0_u64;

    for level in asks {
        let take = remaining.min(level.quantity);
        cost += level.price * i64::try_from(take).unwrap();
        filled += take;
        remaining -= take;

        if remaining == 0 {
            break;
        }
    }

    assert_eq!(filled, 9);
    assert_eq!(cost, 4 * 101 + 5 * 102);
    assert_eq!(remaining, 0);
}

#[test]
fn adding_liquidity_moves_the_top_of_book() {
    let mut book = seeded();

    book.post(Side::Bid, 100, 5).unwrap();

    assert_eq!(
        book.best_bid(),
        Some(Level {
            price: 100,
            quantity: 10,
        }),
    );
}

#[test]
fn consuming_a_level_moves_the_top_of_book() {
    let mut book = seeded();

    book.remove(Side::Bid, 100, 5).unwrap();

    assert_eq!(
        book.best_bid(),
        Some(Level {
            price: 99,
            quantity: 8,
        }),
    );
    assert_eq!(book.spread(), Some(2));
    assert_eq!(book.mid(), Some(100));
}

#[test]
fn a_taker_sweeping_a_level_updates_depth() {
    let mut book = seeded();

    book.remove(Side::Ask, 101, 4).unwrap();

    assert_eq!(
        book.best_ask(),
        Some(Level {
            price: 102,
            quantity: 6,
        }),
    );
    assert_eq!(book.depth(), (3, 2));
}

#[test]
fn cancelling_a_level_removes_it_entirely() {
    let mut book = seeded();

    let removed = book.remove_level(Side::Ask, 102);

    assert_eq!(
        removed,
        Some(Level {
            price: 102,
            quantity: 6,
        }),
    );
    assert_eq!(book.level(Side::Ask, 102), None);
    assert_eq!(book.depth(), (3, 2));
}

#[test]
fn rejecting_bad_input_is_a_no_op() {
    let mut book = seeded();

    assert_eq!(
        book.post(Side::Bid, 100, 0),
        Err(BookError::QuantityNotPositive),
    );
    assert_eq!(
        book.remove(Side::Bid, 98, 1_000),
        Err(BookError::InsufficientQuantity),
    );
    assert_eq!(book.remove(Side::Ask, 500, 1), Err(BookError::UnknownLevel),);
    assert_eq!(book.depth(), (3, 3));
}

#[test]
fn a_quantity_type_wider_than_the_price_type_works() {
    let mut book: OrderBook<i64, i128> = OrderBook::new();

    book.post(Side::Bid, 100, 10_i128).unwrap();
    book.post(Side::Bid, 100, 5_i128).unwrap();

    assert_eq!(book.best_bid().map(|level| level.quantity), Some(15));
}
