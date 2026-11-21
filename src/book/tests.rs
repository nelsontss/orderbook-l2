use super::OrderBook;
use crate::{BookError, Level, Quantity, Side};

fn book() -> OrderBook {
    OrderBook::new()
}

fn populated(levels: &[(Side, i64, u64)]) -> OrderBook {
    let mut book = book();

    for &(side, price, quantity) in levels {
        book.post(side, price, quantity).unwrap();
    }

    book
}

fn quantity_at<Q: Quantity>(book: &OrderBook<i64, Q>, side: Side, price: i64) -> Option<Q> {
    book.level(side, price).map(|level| level.quantity)
}

#[test]
fn new_book_is_empty() {
    let book = book();

    assert!(book.is_empty());
    assert_eq!(book.depth(), (0, 0));
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.best_ask(), None);
    assert_eq!(book.spread(), None);
    assert_eq!(book.mid(), None);
    assert!(!book.is_crossed());
}

#[test]
fn post_creates_a_level() {
    let book = populated(&[(Side::Bid, 100, 5)]);

    assert_eq!(
        book.level(Side::Bid, 100),
        Some(Level {
            price: 100,
            quantity: 5,
        }),
    );
    assert_eq!(book.depth(), (1, 0));
}

#[test]
fn post_accumulates_at_an_existing_level() {
    let book = populated(&[(Side::Bid, 100, 5), (Side::Bid, 100, 7)]);

    assert_eq!(quantity_at(&book, Side::Bid, 100), Some(12));
    assert_eq!(book.depth(), (1, 0));
}

#[test]
fn post_rejects_zero_quantity() {
    let mut book = book();

    assert_eq!(
        book.post(Side::Bid, 100, 0),
        Err(BookError::QuantityNotPositive),
    );
    assert!(book.is_empty());
}

#[test]
fn post_rejects_negative_quantity() {
    let mut book: OrderBook<i64, i64> = OrderBook::new();

    assert_eq!(
        book.post(Side::Ask, 100, -1),
        Err(BookError::QuantityNotPositive),
    );
    assert!(book.is_empty());
}

#[test]
fn post_keeps_the_two_sides_independent() {
    let book = populated(&[(Side::Bid, 100, 5), (Side::Ask, 100, 9)]);

    assert_eq!(quantity_at(&book, Side::Bid, 100), Some(5));
    assert_eq!(quantity_at(&book, Side::Ask, 100), Some(9));
    assert_eq!(book.depth(), (1, 1));
}

#[test]
fn post_never_matches_the_opposite_side() {
    let book = populated(&[(Side::Ask, 100, 5), (Side::Bid, 100, 3)]);

    assert_eq!(quantity_at(&book, Side::Ask, 100), Some(5));
    assert_eq!(quantity_at(&book, Side::Bid, 100), Some(3));
    assert!(book.is_crossed());
}

#[test]
fn post_permits_negative_prices() {
    let book = populated(&[(Side::Bid, -5, 2)]);

    assert_eq!(book.best_bid().map(|level| level.price), Some(-5));
}

#[test]
fn best_bid_is_the_highest_bid() {
    let book = populated(&[(Side::Bid, 98, 1), (Side::Bid, 102, 2), (Side::Bid, 100, 3)]);

    assert_eq!(
        book.best_bid(),
        Some(Level {
            price: 102,
            quantity: 2,
        }),
    );
}

#[test]
fn best_ask_is_the_lowest_ask() {
    let book = populated(&[
        (Side::Ask, 105, 1),
        (Side::Ask, 101, 2),
        (Side::Ask, 103, 3),
    ]);

    assert_eq!(
        book.best_ask(),
        Some(Level {
            price: 101,
            quantity: 2,
        }),
    );
}

#[test]
fn best_bid_ignores_the_ask_side() {
    let book = populated(&[(Side::Ask, 50, 1)]);

    assert_eq!(book.best_bid(), None);
}

#[test]
fn best_ask_ignores_the_bid_side() {
    let book = populated(&[(Side::Bid, 500, 1)]);

    assert_eq!(book.best_ask(), None);
}

#[test]
fn spread_is_ask_minus_bid() {
    let book = populated(&[(Side::Bid, 100, 1), (Side::Ask, 104, 1)]);

    assert_eq!(book.spread(), Some(4));
}

#[test]
fn spread_is_none_when_a_side_is_empty() {
    assert_eq!(populated(&[(Side::Bid, 100, 1)]).spread(), None);
    assert_eq!(populated(&[(Side::Ask, 100, 1)]).spread(), None);
    assert_eq!(book().spread(), None);
}

#[test]
fn spread_is_negative_when_the_book_is_crossed() {
    let book = populated(&[(Side::Bid, 105, 1), (Side::Ask, 100, 1)]);

    assert_eq!(book.spread(), Some(-5));
}

#[test]
fn mid_of_an_even_spread_is_exact() {
    let book = populated(&[(Side::Bid, 100, 1), (Side::Ask, 104, 1)]);

    assert_eq!(book.mid(), Some(102));
}

#[test]
fn mid_of_an_odd_spread_floors() {
    let book = populated(&[(Side::Bid, 100, 1), (Side::Ask, 101, 1)]);

    assert_eq!(book.mid(), Some(100));
}

#[test]
fn mid_is_none_when_a_side_is_empty() {
    assert_eq!(populated(&[(Side::Bid, 100, 1)]).mid(), None);
    assert_eq!(populated(&[(Side::Ask, 100, 1)]).mid(), None);
    assert_eq!(book().mid(), None);
}

#[test]
fn mid_uses_the_top_of_book_not_the_whole_ladder() {
    let book = populated(&[
        (Side::Bid, 90, 1),
        (Side::Bid, 100, 1),
        (Side::Ask, 110, 1),
        (Side::Ask, 120, 1),
    ]);

    assert_eq!(book.mid(), Some(105));
}

#[test]
fn an_empty_book_is_not_crossed() {
    assert!(!book().is_crossed());
}

#[test]
fn a_one_sided_book_is_not_crossed() {
    assert!(!populated(&[(Side::Bid, 100, 1)]).is_crossed());
    assert!(!populated(&[(Side::Ask, 100, 1)]).is_crossed());
}

#[test]
fn a_normal_spread_is_not_crossed() {
    let book = populated(&[(Side::Bid, 100, 1), (Side::Ask, 101, 1)]);

    assert!(!book.is_crossed());
}

#[test]
fn touching_best_prices_are_crossed() {
    let book = populated(&[(Side::Bid, 100, 1), (Side::Ask, 100, 1)]);

    assert!(book.is_crossed());
}

#[test]
fn an_overlapping_book_is_crossed() {
    let book = populated(&[(Side::Bid, 102, 1), (Side::Ask, 100, 1)]);

    assert!(book.is_crossed());
}

#[test]
fn remove_decrements_a_level() {
    let mut book = populated(&[(Side::Bid, 100, 10)]);

    book.remove(Side::Bid, 100, 4).unwrap();

    assert_eq!(quantity_at(&book, Side::Bid, 100), Some(6));
    assert_eq!(book.depth(), (1, 0));
}

#[test]
fn remove_drops_a_level_that_reaches_zero() {
    let mut book = populated(&[(Side::Bid, 100, 10)]);

    book.remove(Side::Bid, 100, 10).unwrap();

    assert_eq!(book.level(Side::Bid, 100), None);
    assert_eq!(book.best_bid(), None);
    assert!(book.is_empty());
}

#[test]
fn remove_rejects_over_removal_without_mutating() {
    let mut book = populated(&[(Side::Bid, 100, 10)]);

    assert_eq!(
        book.remove(Side::Bid, 100, 11),
        Err(BookError::InsufficientQuantity),
    );
    assert_eq!(quantity_at(&book, Side::Bid, 100), Some(10));
}

#[test]
fn remove_rejects_an_unknown_level() {
    let mut book = populated(&[(Side::Bid, 100, 10)]);

    assert_eq!(book.remove(Side::Bid, 99, 1), Err(BookError::UnknownLevel),);
    assert_eq!(book.depth(), (1, 0));
}

#[test]
fn remove_distinguishes_sides_at_the_same_price() {
    let mut book = populated(&[(Side::Bid, 100, 10), (Side::Ask, 100, 10)]);

    book.remove(Side::Bid, 100, 10).unwrap();

    assert_eq!(quantity_at(&book, Side::Bid, 100), None);
    assert_eq!(quantity_at(&book, Side::Ask, 100), Some(10));
}

#[test]
fn remove_rejects_zero_quantity() {
    let mut book = populated(&[(Side::Bid, 100, 10)]);

    assert_eq!(
        book.remove(Side::Bid, 100, 0),
        Err(BookError::QuantityNotPositive),
    );
    assert_eq!(quantity_at(&book, Side::Bid, 100), Some(10));
}

#[test]
fn remove_rejects_negative_quantity() {
    let mut book: OrderBook<i64, i64> = OrderBook::new();
    book.post(Side::Bid, 100, 10).unwrap();

    assert_eq!(
        book.remove(Side::Bid, 100, -3),
        Err(BookError::QuantityNotPositive),
    );
    assert_eq!(quantity_at(&book, Side::Bid, 100), Some(10));
}

#[test]
fn remove_level_returns_the_level_it_removed() {
    let mut book = populated(&[(Side::Ask, 101, 4)]);

    assert_eq!(
        book.remove_level(Side::Ask, 101),
        Some(Level {
            price: 101,
            quantity: 4,
        }),
    );
    assert_eq!(book.level(Side::Ask, 101), None);
    assert!(book.is_empty());
}

#[test]
fn remove_level_on_an_absent_price_is_none() {
    let mut book = populated(&[(Side::Ask, 101, 4)]);

    assert_eq!(book.remove_level(Side::Ask, 102), None);
    assert_eq!(book.depth(), (0, 1));
}

#[test]
fn remove_level_is_infallible_for_any_price() {
    let mut book = book();

    assert_eq!(book.remove_level(Side::Bid, -1), None);
    assert_eq!(book.remove_level(Side::Bid, i64::MAX), None);
    assert!(book.is_empty());
}

#[test]
fn bids_iterate_from_high_to_low() {
    let book = populated(&[(Side::Bid, 100, 1), (Side::Bid, 102, 1), (Side::Bid, 98, 1)]);

    let prices: Vec<i64> = book.bids().map(|level| level.price).collect();

    assert_eq!(prices, [102, 100, 98]);
}

#[test]
fn asks_iterate_from_low_to_high() {
    let book = populated(&[
        (Side::Ask, 103, 1),
        (Side::Ask, 101, 1),
        (Side::Ask, 105, 1),
    ]);

    let prices: Vec<i64> = book.asks().map(|level| level.price).collect();

    assert_eq!(prices, [101, 103, 105]);
}

#[test]
fn iteration_yields_quantities_too() {
    let book = populated(&[(Side::Bid, 100, 3), (Side::Bid, 99, 8)]);

    let levels: Vec<Level<i64, u64>> = book.bids().collect();

    assert_eq!(
        levels,
        [
            Level {
                price: 100,
                quantity: 3,
            },
            Level {
                price: 99,
                quantity: 8,
            },
        ],
    );
}

#[test]
fn iteration_over_an_empty_side_yields_nothing() {
    let book = populated(&[(Side::Bid, 100, 1)]);

    assert_eq!(book.bids().count(), 1);
    assert_eq!(book.asks().count(), 0);
}

#[test]
fn iteration_visits_every_level_once() {
    let mut book = book();

    for price in 1..=50_i64 {
        book.post(Side::Bid, price, 1).unwrap();
    }

    assert_eq!(book.bids().count(), 50);
    assert_eq!(book.bids().map(|level| level.price).sum::<i64>(), 1_275);
}

#[test]
fn depth_counts_levels_per_side() {
    let book = populated(&[(Side::Bid, 100, 1), (Side::Bid, 99, 1), (Side::Ask, 101, 1)]);

    assert_eq!(book.depth(), (2, 1));
}

#[test]
fn depth_does_not_count_quantity() {
    let book = populated(&[(Side::Bid, 100, 1_000_000)]);

    assert_eq!(book.depth(), (1, 0));
}

#[test]
fn clear_empties_both_sides() {
    let mut book = populated(&[(Side::Bid, 100, 1), (Side::Ask, 101, 1)]);

    book.clear();

    assert!(book.is_empty());
    assert_eq!(book.depth(), (0, 0));
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.best_ask(), None);
}

#[test]
fn clear_leaves_the_book_usable() {
    let mut book = populated(&[(Side::Bid, 100, 1)]);

    book.clear();
    book.post(Side::Bid, 42, 2).unwrap();

    assert_eq!(book.best_bid().map(|level| level.price), Some(42));
}

#[test]
fn level_returns_none_for_an_absent_price() {
    let book = populated(&[(Side::Bid, 100, 1)]);

    assert_eq!(book.level(Side::Bid, 101), None);
    assert_eq!(book.level(Side::Ask, 100), None);
}

#[test]
fn default_and_new_are_the_same() {
    let from_new: OrderBook = OrderBook::new();
    let from_default: OrderBook = OrderBook::default();

    assert_eq!(from_new.depth(), from_default.depth());
    assert!(from_default.is_empty());
}

#[test]
fn price_and_quantity_types_are_independent() {
    let mut book: OrderBook<i64, i128> = OrderBook::new();

    book.post(Side::Bid, 100, 10).unwrap();

    assert_eq!(book.best_bid().map(|level| level.quantity), Some(10));
}

#[test]
fn book_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<OrderBook>();
    assert_send_sync::<OrderBook<i128, i128>>();
}

macro_rules! differential_test {
    ($name:ident, $book:ty, $new:expr) => {
        #[test]
        fn $name() {
            struct Lcg(u64);

            impl Lcg {
                fn next(&mut self) -> u64 {
                    self.0 = self
                        .0
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407);
                    self.0 >> 33
                }
            }

            let mut rng = Lcg(0x5eed);
            let mut tree: OrderBook = OrderBook::new();
            let mut experimental: $book = $new;

            for _ in 0..2_000 {
                let side = if rng.next() % 2 == 0 {
                    Side::Bid
                } else {
                    Side::Ask
                };
                let price = 90 + (rng.next() % 20) as i64;
                let quantity = 1 + rng.next() % 10;

                if rng.next() % 4 == 0 {
                    let from_tree = tree.remove(side, price, quantity);
                    let from_experimental = experimental.remove(side, price, quantity);
                    assert_eq!(from_tree, from_experimental);
                } else {
                    let from_tree = tree.post(side, price, quantity);
                    let from_experimental = experimental.post(side, price, quantity);
                    assert_eq!(from_tree, from_experimental);
                }

                let tree_levels: Vec<Level<i64, u64>> = tree.bids().chain(tree.asks()).collect();
                let experimental_levels: Vec<Level<i64, u64>> =
                    experimental.bids().chain(experimental.asks()).collect();

                assert_eq!(tree_levels, experimental_levels);
                assert_eq!(tree.best_bid(), experimental.best_bid());
                assert_eq!(tree.best_ask(), experimental.best_ask());
                assert_eq!(tree.depth(), experimental.depth());
            }
        }
    };
}

#[cfg(feature = "experimental-hashmap")]
differential_test!(
    hash_book_matches_tree_book_on_the_same_stream,
    crate::book::HashOrderBook,
    crate::book::HashOrderBook::new()
);

differential_test!(
    tick_book_matches_tree_book_on_the_same_stream,
    crate::book::TickOrderBook,
    crate::book::TickOrderBook::new(0, 1, 1_000).unwrap()
);

differential_test!(
    vec_book_matches_tree_book_on_the_same_stream,
    crate::book::VecOrderBook,
    crate::book::VecOrderBook::new()
);

#[test]
fn tick_grid_rejects_invalid_grids() {
    use super::tick_book::TickGrid;

    assert_eq!(TickGrid::new(0, 0, 10), None);
    assert_eq!(TickGrid::new(0, -1, 10), None);
    assert_eq!(TickGrid::new(i64::MAX, 4, 4), None);
    assert!(TickGrid::new(100, 5, 8).is_some());
    assert!(TickGrid::new(-100, 1, 500).is_some());
    assert!(TickGrid::new(i64::MIN, 1, 4).is_some());
}

#[test]
fn tick_grid_maps_prices_and_slots() {
    use super::tick_book::TickGrid;

    let grid = TickGrid::new(100, 5, 8).unwrap();

    assert_eq!(grid.len(), 8);

    assert_eq!(grid.index_of(100), Some(0));
    assert_eq!(grid.index_of(135), Some(7));
    assert_eq!(grid.index_of(103), None);
    assert_eq!(grid.index_of(140), None);
    assert_eq!(grid.index_of(99), None);
}

#[test]
fn tick_book_rejects_off_grid_prices() {
    use crate::book::TickOrderBook;

    let mut book: TickOrderBook = TickOrderBook::new(100, 5, 10).unwrap();

    assert_eq!(
        book.post(Side::Bid, 103, 1),
        Err(BookError::PriceOutsideGrid),
    );
    assert_eq!(
        book.post(Side::Bid, 99, 1),
        Err(BookError::PriceOutsideGrid),
    );
    assert_eq!(
        book.post(Side::Bid, 1_000, 1),
        Err(BookError::PriceOutsideGrid),
    );

    assert_eq!(
        book.remove(Side::Bid, 103, 1),
        Err(BookError::PriceOutsideGrid),
    );
    assert_eq!(book.remove_level(Side::Bid, 103), None);
    assert_eq!(book.level(Side::Bid, 103), None);

    book.post(Side::Bid, 105, 3).unwrap();

    assert_eq!(
        book.best_bid(),
        Some(Level {
            price: 105,
            quantity: 3,
        }),
    );
}

#[test]
fn tick_book_supports_negative_prices() {
    use crate::book::TickOrderBook;

    let mut book: TickOrderBook = TickOrderBook::new(-100, 1, 500).unwrap();

    book.post(Side::Bid, -50, 2).unwrap();

    assert_eq!(
        book.best_bid(),
        Some(Level {
            price: -50,
            quantity: 2,
        }),
    );
    assert_eq!(book.mid(), None);
}

#[test]
fn tick_book_iteration_skips_empty_regions() {
    use crate::book::TickOrderBook;

    let mut book: TickOrderBook = TickOrderBook::new(0, 1, 1_000).unwrap();

    for price in [3_i64, 70, 64, 999, 500] {
        book.post(Side::Bid, price, 1).unwrap();
    }

    for price in [1_i64, 128, 700] {
        book.post(Side::Ask, price, 2).unwrap();
    }

    let bids: Vec<(i64, u64)> = book
        .bids()
        .map(|level| (level.price, level.quantity))
        .collect();
    let asks: Vec<(i64, u64)> = book
        .asks()
        .map(|level| (level.price, level.quantity))
        .collect();

    assert_eq!(bids, [(999, 1), (500, 1), (70, 1), (64, 1), (3, 1)]);
    assert_eq!(asks, [(1, 2), (128, 2), (700, 2)]);
}

#[test]
fn an_identical_ordered_stream_produces_an_identical_book() {
    let ops = [
        (Side::Bid, 100_i64, Some(5_u64)),
        (Side::Ask, 101, Some(4)),
        (Side::Bid, 99, Some(8)),
        (Side::Bid, 100, Some(3)),
        (Side::Ask, 101, None),
        (Side::Ask, 102, Some(6)),
    ];

    let mut left = book();
    let mut right = book();

    for &(side, price, quantity) in &ops {
        for book in [&mut left, &mut right] {
            match quantity {
                Some(quantity) => book.post(side, price, quantity).unwrap(),
                None => book.remove(side, price, 1).unwrap(),
            }
        }
    }

    let left_levels: Vec<Level<i64, u64>> = left.bids().chain(left.asks()).collect();
    let right_levels: Vec<Level<i64, u64>> = right.bids().chain(right.asks()).collect();

    assert_eq!(left_levels, right_levels);
    assert_eq!(left.depth(), (2, 2));
}
