# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
