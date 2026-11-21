//! Head-to-head comparison of the `BTreeMap` book against the experimental
//! backends. Each backend is added by enabling its feature.

use criterion::Criterion;

const MID: i64 = 100_000;
const DEPTHS: [i64; 2] = [1_000, 10_000];
const BATCH: u32 = 64;

fn main() {
    let mut criterion = Criterion::default().configure_from_args();

    tree_benches::reads(&mut criterion);
    tree_benches::writes(&mut criterion);
    tree_benches::removes(&mut criterion);

    #[cfg(feature = "experimental-hashmap")]
    {
        hashmap_benches::reads(&mut criterion);
        hashmap_benches::writes(&mut criterion);
        hashmap_benches::removes(&mut criterion);
    }

    tick_benches::reads(&mut criterion);
    tick_benches::writes(&mut criterion);
    tick_benches::removes(&mut criterion);
    tick_oversized_benches::reads(&mut criterion);
    tick_oversized_benches::writes(&mut criterion);
    tick_oversized_benches::removes(&mut criterion);

    vec_benches::reads(&mut criterion);
    vec_benches::writes(&mut criterion);
    vec_benches::removes(&mut criterion);

    criterion.final_summary();
}

macro_rules! comparison_benches {
    ($module:ident, $label:literal, $book:ty, $new:expr) => {
        mod $module {
            use std::hint::black_box;
            use std::time::{Duration, Instant};

            use criterion::{Bencher, BenchmarkId, Criterion};
            use orderbook_l2::Side;

            use super::{BATCH, DEPTHS, MID};

            const LABEL: &str = $label;

            fn build(depth: i64) -> $book {
                let mut book: $book = $new(depth);

                for offset in 1..=depth {
                    book.post(Side::Bid, MID - offset, 1).unwrap();
                    book.post(Side::Ask, MID + offset, 1).unwrap();
                }

                book
            }

            fn id(operation: &str, depth: i64) -> BenchmarkId {
                BenchmarkId::new(format!("{LABEL}/{operation}"), depth)
            }

            fn batch<F>(bencher: &mut Bencher<'_>, depth: i64, mut routine: F)
            where
                F: FnMut(&mut $book, usize),
            {
                bencher.iter_custom(|iterations| {
                    let mut total = Duration::ZERO;

                    for _ in 0..iterations {
                        let mut book = build(depth);
                        let book = black_box(&mut book);
                        let start = Instant::now();

                        for step in 0..BATCH as usize {
                            routine(book, step);
                        }

                        total += start.elapsed() / BATCH;
                    }

                    total
                });
            }

            pub fn reads(c: &mut Criterion) {
                let mut group = c.benchmark_group("read");

                for depth in DEPTHS {
                    let book = build(depth);
                    let book = black_box(&book);

                    group.bench_function(id("best_bid", depth), |bencher| {
                        bencher.iter(|| book.best_bid());
                    });

                    group.bench_function(id("mid", depth), |bencher| {
                        bencher.iter(|| book.mid());
                    });

                    group.bench_function(id("level_hit", depth), |bencher| {
                        bencher.iter(|| book.level(Side::Bid, MID - depth / 2));
                    });

                    group.bench_function(id("top_10_bids", depth), |bencher| {
                        bencher.iter(|| book.bids().take(10).map(|level| level.price).sum::<i64>());
                    });

                    group.bench_function(id("walk_bids", depth), |bencher| {
                        bencher.iter(|| book.bids().map(|level| level.quantity).sum::<u64>());
                    });
                }

                group.finish();
            }

            pub fn writes(c: &mut Criterion) {
                let mut group = c.benchmark_group("write");

                for depth in DEPTHS {
                    group.bench_function(id("accumulate_existing_level", depth), |bencher| {
                        batch(bencher, depth, |book, step| {
                            book.post(Side::Bid, MID - 1 - step as i64, 1).unwrap();
                        });
                    });

                    group.bench_function(id("new_best_level", depth), |bencher| {
                        batch(bencher, depth, |book, step| {
                            book.post(Side::Bid, MID + 1 + step as i64, 1).unwrap();
                        });
                    });

                    group.bench_function(id("new_deep_level", depth), |bencher| {
                        batch(bencher, depth, |book, step| {
                            book.post(Side::Bid, MID - depth - 1 - step as i64, 1)
                                .unwrap();
                        });
                    });
                }

                group.finish();
            }

            pub fn removes(c: &mut Criterion) {
                let mut group = c.benchmark_group("remove");

                for depth in DEPTHS {
                    group.bench_function(id("top_level", depth), |bencher| {
                        batch(bencher, depth, |book, step| {
                            book.remove(Side::Bid, MID - 1 - step as i64, 1).unwrap();
                        });
                    });

                    group.bench_function(id("emptying_the_best_level", depth), |bencher| {
                        batch(bencher, depth, |book, step| {
                            book.remove_level(Side::Bid, MID - 1 - step as i64);
                        });
                    });
                }

                group.finish();
            }
        }
    };
}

comparison_benches!(tree_benches, "btree", orderbook_l2::OrderBook, |_depth| {
    orderbook_l2::OrderBook::new()
});

#[cfg(feature = "experimental-hashmap")]
comparison_benches!(
    hashmap_benches,
    "hashmap",
    orderbook_l2::HashOrderBook,
    |_depth| orderbook_l2::HashOrderBook::new()
);

comparison_benches!(
    tick_benches,
    "tick",
    orderbook_l2::TickOrderBook,
    |depth: i64| orderbook_l2::TickOrderBook::new(
        MID - depth - BATCH as i64 - 1,
        1,
        (2 * depth + 2 * BATCH as i64 + 3) as usize
    )
    .unwrap()
);

comparison_benches!(
    tick_oversized_benches,
    "tick-oversized",
    orderbook_l2::TickOrderBook,
    |_depth| orderbook_l2::TickOrderBook::new(89_000, 1, 22_000).unwrap()
);

comparison_benches!(vec_benches, "vec", orderbook_l2::VecOrderBook, |_depth| {
    orderbook_l2::VecOrderBook::new()
});
