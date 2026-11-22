# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - unreleased

### Added

- `set_level(side, price, quantity)` on all three books: replaces a level, and
  removes it when the quantity is zero, for feeds that publish absolute size
  rather than deltas.
- `decimal` feature: implements `Price` for `rust_decimal::Decimal`, so `mid()`
  is available on decimal books. Optional and off by default.

### Changed

- `mid()` is now the only method that requires `Price`. Everything else is
  bounded by `Copy + Ord + Sub`, so `OrderBook<usize, u64>`,
  `OrderBook<Decimal, Decimal>` and any other ordered, subtractable price type
  work without a newtype or an extra impl.

## [0.1.0] - 2026-11-22

### Added

- `OrderBook`, an L2 book backed by one `BTreeMap` per side, generic over any
  `Price` and `Quantity` with `i64`/`u64` defaults.
- `VecOrderBook`, storing each side as a sorted `Vec` for the fastest depth
  reads, at the cost of an `O(n)` write beyond a side's extreme.
- `TickOrderBook`, a price-indexed array over a declared grid: every operation
  constant time, memory proportional to the price range.
- `Level`, `Side`, `Price`, `Quantity` and `BookError` as the shared vocabulary.
- Differential tests asserting that every backend produces identical books for
  the same ordered stream.
- Criterion benchmarks for the default book and a four-way backend comparison.

[Unreleased]: https://github.com/nelsontss/orderbook-l2/commits/main
