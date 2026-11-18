# orderbook-rs

A generic, exchange-agnostic limit order book in Rust.

Status: **scaffolding only**. No book logic exists yet; the public API is still
to be designed.

## Layout

```
src/lib.rs      crate root
tests/          integration tests (to be added)
benches/        criterion benchmarks (to be added)
```

## Commands

```sh
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
