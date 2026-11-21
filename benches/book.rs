//! Criterion benchmarks for the default `BTreeMap` book.

#![allow(missing_docs)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::{Bencher, Criterion, criterion_group, criterion_main};
use orderbook_l2::{OrderBook, Side};

const MID: i64 = 100_000;
const DEPTHS: [i64; 3] = [100, 1_000, 10_000];
const BATCH: u32 = 64;

fn populated(depth: i64) -> OrderBook {
    let mut book: OrderBook = OrderBook::new();

    for offset in 1..=depth {
        book.post(Side::Bid, MID - offset, 1).unwrap();
        book.post(Side::Ask, MID + offset, 1).unwrap();
    }

    book
}

fn batch<F>(bencher: &mut Bencher<'_>, depth: i64, mut routine: F)
where
    F: FnMut(&mut OrderBook, usize),
{
    bencher.iter_custom(|iterations| {
        let mut total = Duration::ZERO;

        for _ in 0..iterations {
            let mut book = populated(depth);
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

fn post(c: &mut Criterion) {
    let mut group = c.benchmark_group("post");

    for depth in DEPTHS {
        group.bench_function(format!("accumulate_existing_level/{depth}"), |bencher| {
            batch(bencher, depth, |book, step| {
                book.post(Side::Bid, MID - 1 - step as i64, 1).unwrap();
            });
        });

        group.bench_function(format!("new_best_level/{depth}"), |bencher| {
            batch(bencher, depth, |book, step| {
                book.post(Side::Bid, MID + 1 + step as i64, 1).unwrap();
            });
        });

        group.bench_function(format!("new_deep_level/{depth}"), |bencher| {
            batch(bencher, depth, |book, step| {
                book.post(Side::Bid, MID - depth - 1 - step as i64, 1)
                    .unwrap();
            });
        });
    }

    group.finish();
}

fn reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("read");

    for depth in DEPTHS {
        let book = populated(depth);
        let book = black_box(&book);

        group.bench_function(format!("best_bid/{depth}"), |bencher| {
            bencher.iter(|| book.best_bid());
        });

        group.bench_function(format!("best_ask/{depth}"), |bencher| {
            bencher.iter(|| book.best_ask());
        });

        group.bench_function(format!("spread/{depth}"), |bencher| {
            bencher.iter(|| book.spread());
        });

        group.bench_function(format!("mid/{depth}"), |bencher| {
            bencher.iter(|| book.mid());
        });

        group.bench_function(format!("level_hit/{depth}"), |bencher| {
            bencher.iter(|| book.level(Side::Bid, MID - depth / 2));
        });

        group.bench_function(format!("level_miss/{depth}"), |bencher| {
            bencher.iter(|| book.level(Side::Bid, MID + depth));
        });

        group.bench_function(format!("walk_bids/{depth}"), |bencher| {
            bencher.iter(|| book.bids().map(|level| level.quantity).sum::<u64>());
        });

        group.bench_function(format!("walk_asks/{depth}"), |bencher| {
            bencher.iter(|| book.asks().map(|level| level.quantity).sum::<u64>());
        });

        group.bench_function(format!("top_10_bids/{depth}"), |bencher| {
            bencher.iter(|| book.bids().take(10).map(|level| level.price).sum::<i64>());
        });

        group.bench_function(format!("top_10_imbalance/{depth}"), |bencher| {
            bencher.iter(|| {
                let bids: u64 = book.bids().take(10).map(|level| level.quantity).sum();
                let asks: u64 = book.asks().take(10).map(|level| level.quantity).sum();
                (bids, asks)
            });
        });
    }

    group.finish();
}

fn remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("remove");

    for depth in DEPTHS {
        group.bench_function(format!("top_level/{depth}"), |bencher| {
            batch(bencher, depth, |book, step| {
                book.remove(Side::Bid, MID - 1 - step as i64, 1).unwrap();
            });
        });

        group.bench_function(format!("emptying_the_best_level/{depth}"), |bencher| {
            batch(bencher, depth, |book, step| {
                book.remove_level(Side::Bid, MID - 1 - step as i64);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, post, reads, remove);
criterion_main!(benches);
