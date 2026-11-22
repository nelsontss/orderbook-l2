# orderbook-l2

[![CI](https://github.com/nelsontss/orderbook-l2/actions/workflows/ci.yml/badge.svg)](https://github.com/nelsontss/orderbook-l2/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/orderbook-l2.svg)](https://crates.io/crates/orderbook-l2)
[![docs.rs](https://docs.rs/orderbook-l2/badge.svg)](https://docs.rs/orderbook-l2)
[![license](https://img.shields.io/crates/l/orderbook-l2.svg)](#license)

A generic, exchange-agnostic **L2 limit order book** in Rust.

Quantities are aggregated per price level: there is no per-order identity and no
matching. Posting adds liquidity at a price, removing takes it away, and the book
answers the questions a market-data consumer asks — best bid and ask, spread,
mid, one level, or the whole ladder in order.

```rust
use orderbook_l2::{Level, OrderBook, Side};

let mut book: OrderBook = OrderBook::new();

book.post(Side::Bid, 100, 5).unwrap();
book.post(Side::Bid, 99, 8).unwrap();
book.post(Side::Ask, 101, 4).unwrap();

assert_eq!(book.best_bid(), Some(Level { price: 100, quantity: 5 }));
assert_eq!(book.best_ask(), Some(Level { price: 101, quantity: 4 }));
assert_eq!(book.spread(), Some(1));
assert_eq!(book.mid(), Some(100));
assert_eq!(book.depth(), (2, 1));

let ladder: Vec<i64> = book.bids().map(|level| level.price).collect();
assert_eq!(ladder, [100, 99]);
```

Status: pre-release. The API is settled enough to build on; expect additions, not
reshaping.

```toml
[dependencies]
orderbook-l2 = "0.1"   # once 0.1.0 is on crates.io
```

Licensed under either of MIT or Apache-2.0, at your option.

## What it is not

- **Not a matching engine.** The book never consumes the opposite side. A crossed
  book is representable, and `is_crossed()` reports it; deciding when liquidity
  trades belongs to the caller.
- **Not order-level.** There is no `OrderId`, no queue position, no per-order
  cancellation. If you need price-time priority inside a level, you need an L3
  structure, which is a different type of thing.
- **Not a concurrency coordinator.** Mutation takes `&mut self`, so exactly one
  writer. Ordering incoming interest and deciding what matches is upstream work.

## The API

Every book exposes the same surface.

| Method | Returns |
|---|---|
| `new()` | an empty book (`OrderBook`, `VecOrderBook`) |
| `new(base, tick, len)` | an empty book over a fixed grid (`TickOrderBook`) |
| `post(side, price, quantity)` | `Result<(), BookError>` |
| `remove(side, price, quantity)` | `Result<(), BookError>` |
| `remove_level(side, price)` | `Option<Level>` |
| `set_level(side, price, quantity)` | `Result<(), BookError>` — replaces, removes on zero |
| `clear()` | — |
| `best_bid()` / `best_ask()` | `Option<Level>` |
| `spread()` | `Option<P>` |
| `mid()` | `Option<P>` |
| `level(side, price)` | `Option<Level>` |
| `bids()` / `asks()` | iterator of `Level`, best first |
| `depth()` | `(bid levels, ask levels)` |
| `is_empty()` / `is_crossed()` | `bool` |

Reads return `Option`, never `Result`: an empty or one-sided book is a normal
state. A failed mutation leaves the book untouched, so an error is always a no-op.

```rust
use orderbook_l2::BookError;

assert_eq!(book.post(Side::Bid, 100, 0), Err(BookError::QuantityNotPositive));
assert_eq!(book.remove(Side::Bid, 42, 1), Err(BookError::UnknownLevel));
assert_eq!(book.remove(Side::Bid, 100, 9), Err(BookError::InsufficientQuantity));
```

## Choosing a book

Three books ship. They share the contract and the vocabulary; only the price
model and the cost profile differ.

| | `OrderBook` | `VecOrderBook` | `TickOrderBook` |
|---|---|---|---|
| prices | any `Price` | any `Price` | `i64` ticks on a fixed grid |
| constructor | `new()` | `new()` | `new(base, tick, len)?` |
| `post` / `remove` | `O(log n)` | `O(1)` at the touch, `O(n)` at the far end | `O(1)` everywhere |
| `best_*` / `mid` | `O(log n)` | `O(1)` | `O(1)` |
| `bids()` / `asks()` | `O(k)` | `O(k)`, fastest | `O(range/64 + k)` |
| `level()` | `O(log n)` | `O(log n)` | `O(1)` |
| memory | ~20 B per level | 16 B per level | 16 B per **tick** |

Pick by what your workload does most, using the measured numbers in
[Performance](#performance):

- **`OrderBook` is the default.** Any price type, no configuration, and no
  operation is ever catastrophically slow. Choose it when you don't have a
  specific reason to choose something else.
- **`VecOrderBook` when reads dominate.** A depth walk is ~10× faster than the
  tree's and a top-of-book read ~7×. The cost lands on writes that extend the
  book outward — 2.13 µs against 12.6 ns for a write at the touch.
- **`TickOrderBook` when writes must never spike.** Flat 1.1–1.7 ns per write
  regardless of depth or which end of the book you hit. Requires integer ticks
  and a price band you can declare up front.

## `OrderBook` — the default

Generic over price and quantity, with sensible defaults:

```rust
use orderbook_l2::{BookError, OrderBook, Side};

let mut book: OrderBook<i64, u64> = OrderBook::new();
// `OrderBook` alone is the same type.
let mut shorthand: OrderBook = OrderBook::new();

book.post(Side::Bid, 98_500, 3).unwrap();
book.post(Side::Ask, 98_510, 2).unwrap();
```

Prices may be negative, and are never validated:

```rust
book.post(Side::Bid, -37, 1).unwrap();
assert_eq!(book.level(Side::Bid, -37).map(|level| level.quantity), Some(1));
```

Backed by a `BTreeMap` per side, the bid side keyed by `Reverse<P>` so both sides
iterate best-first without reverse traversal.

## `VecOrderBook` — for read-heavy books

Same API, no arguments to the constructor:

```rust
use orderbook_l2::{Side, VecOrderBook};

let mut book: VecOrderBook = VecOrderBook::new();

for price in 1..=1_000 {
    book.post(Side::Bid, 100_000 - price, 1).unwrap();
}

let volume: u64 = book.bids().take(10).map(|level| level.quantity).sum();
```

Each side is a sorted `Vec` held worst-price-first, so the best level is the last
element and the touch is served by `push`/`pop`. Reads are contiguous scans,
which is why this book has the fastest depth queries.

The trade is the far end of each side: creating a level beyond the current
extreme shifts every element. Grow the book outward from the touch and this is
cheap; replay a descending snapshot into it and it is not.

## `TickOrderBook` — flat writes

A tick array: one slot per price on a declared grid, `base + index * tick`.

```rust
use orderbook_l2::{BookError, Side, TickOrderBook};

// 500 slots of 1 tick each, covering 1_000..1_499.
let mut book: TickOrderBook = TickOrderBook::new(1_000, 1, 500).unwrap();

book.post(Side::Bid, 1_204, 3).unwrap();
book.post(Side::Ask, 1_206, 5).unwrap();

assert_eq!(book.spread(), Some(2));

// Prices off the grid are rejected rather than silently ignored.
assert_eq!(
    book.post(Side::Bid, 1_205, 1),
    Ok(()),
);
assert_eq!(
    book.post(Side::Bid, 2_000, 1),
    Err(BookError::PriceOutsideGrid),
);
```

Every operation is constant time, and no write moves data. The grid must be wide
enough for the instrument's range and fine enough for its tick:

```rust
// Prices scaled by 100: 1,000 ticks of one cent, covering 100.00 .. 109.99.
let book: TickOrderBook = TickOrderBook::new(10_000, 1, 1_000).unwrap();
```

Memory is 16 bytes per tick per side, occupied or not, and construction touches
the whole grid. Size it to the band you trade: a grid far larger than the depth
stored costs iteration time, since a walk steps over empty slots 64 at a time.

## Feeding it from a market data stream

Exchange deltas usually carry the **new absolute quantity** for a price, not the
change. The replace pattern is `remove_level` followed by `post`:

```rust
use orderbook_l2::{OrderBook, Side};

fn set_level(book: &mut OrderBook, side: Side, price: i64, quantity: u64) {
    book.remove_level(side, price);

    if quantity > 0 {
        book.post(side, price, quantity).unwrap();
    }
}
```

If your feed is incremental instead, use a signed quantity type and call `post`
or `remove` with the magnitude:

```rust
use orderbook_l2::{OrderBook, Side};

let mut book: OrderBook<i64, i64> = OrderBook::new();

let updates = [(100, 5), (100, -2), (99, 3)];
for (price, delta) in updates {
    if delta > 0 {
        book.post(Side::Bid, price, delta).unwrap();
    } else {
        book.remove(Side::Bid, price, -delta).unwrap();
    }
}
```

## Custom numeric types

`Quantity` is blanket-implemented for anything that is `Copy + Ord + Default`
with `Add` and `Sub`, so fixed-point and decimal crates satisfy it with no work.

`Price` adds one thing on top of `Copy + Ord + Sub`: a midpoint. Only `mid()`
requires it. Every other operation — post, remove, best bid and ask, spread,
depth, iteration — is bounded by `Copy + Ord + Sub` alone, so a price type with
no meaningful midpoint still gets the whole API minus `mid()`.

```rust
use orderbook_l2::Price;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Ticks(i64);

impl std::ops::Sub for Ticks {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl Price for Ticks {
    fn mid(self, other: Self) -> Self {
        Self((self.0 & other.0) + ((self.0 ^ other.0) >> 1))
    }
}
```

### Decimals

`rust_decimal::Decimal` is `Copy + Ord + Default` with `Add` and `Sub`, so it
works as a quantity immediately and as a price for everything except `mid()`.
To get `mid()` too, enable the `decimal` feature:

```toml
orderbook-l2 = { version = "0.1", features = ["decimal"] }
```

```rust
use orderbook_l2::{OrderBook, Side};
use rust_decimal::Decimal;

let mut book: OrderBook<Decimal, Decimal> = OrderBook::new();
book.post(Side::Bid, Decimal::new(10_025, 2), Decimal::new(5, 0)).unwrap();
book.post(Side::Ask, Decimal::new(10_030, 2), Decimal::new(3, 0)).unwrap();

assert_eq!(book.mid(), Some(Decimal::new(100_275, 3)));
```

The feature is off by default and pulls only `rust_decimal` and its arithmetic
dependencies.

### Floats

`f64` is deliberately not supported: it is `PartialOrd`, not `Ord`, and a
NaN-descending price ladder is not something worth debugging.

## Performance

Measured at 10,000 levels per side on an Apple Silicon laptop, `cargo bench`.
Absolute numbers are machine-specific; the ratios are the point.

| Operation | `OrderBook` | `TickOrderBook` | `VecOrderBook` |
|---|---|---|---|
| `best_bid` | 1.00 ns | 1.85 ns | **0.32 ns** |
| `mid` | 1.74 ns | 3.49 ns | **0.29 ns** |
| `level()` hit | 5.70 ns | **0.25 ns** | 9.05 ns |
| `bids().take(10)` | 10.0 ns | 25.1 ns | **1.38 ns** |
| full walk | 14.7 µs | 20.5 µs | **1.52 µs** |
| `post`, existing level | 10.1 ns | **1.08 ns** | 9.81 ns |
| `post`, new best level | 18.2 ns | **1.46 ns** | 12.6 ns |
| `post`, new deepest level | 23.1 ns | **1.54 ns** | 2.13 µs |
| `remove`, top level | 22.9 ns | **1.68 ns** | 11.8 ns |

Reading the table:

- **`VecOrderBook` owns reads** because a slice of `Level` is copied as a
  vectorised loop. Its single cliff is the far end of a side, where creating a
  level shifts the whole array.
- **`TickOrderBook` owns writes** because index arithmetic on independent slots
  pipelines, while a tree descent is a chain of dependent loads. It pays on
  iteration, where its occupancy bitmap must be decoded.
- **`OrderBook` never wins a row and never spikes** — 10–23 ns for every write,
  at every depth. That is why it is the default.

Methodology: read benchmarks call one operation per iteration against a fixed
book; write and remove benchmarks run 64 operations per built fixture and report
per-operation time, because a 1 ns operation cannot be measured against a ~1 ms
fixture. Both are steady-state throughput with a hot working set, not cold-start
latency. A fourth backend, `HashOrderBook`, exists behind the
`experimental-hashmap` feature purely as a measured dead end.

## Repository layout

```text
src/
  lib.rs              crate root and re-exports
  book/
    order_book.rs     OrderBook
    vec_order_book.rs VecOrderBook
    tick_book.rs      TickOrderBook
    hash_order_book.rs  HashOrderBook (experimental-hashmap)
    tests.rs          unit and differential tests
  level/              Level
  side/               Side
  traits/             Price, Quantity
  error/              BookError
tests/public_api.rs   end-to-end tests through the public API
benches/book.rs       benchmarks for OrderBook
benches/comparison.rs four-way backend comparison
```

## Development

MSRV is **1.85**, the edition-2024 floor, and CI runs the full suite on it.

```sh
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --features experimental-hashmap
cargo bench --bench book
cargo bench --all-features --bench comparison
```

The three shipped books are validated against each other: a differential test
applies 2,000 pseudo-random posts and removes to every backend and to
`OrderBook` in lockstep, asserting identical ladders, best prices, depth and
error results after every step.
