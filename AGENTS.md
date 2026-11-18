# AGENTS.md

A generic, exchange-agnostic limit order book library.

## Rules

1. **No inline comments.** Code explains itself through names and structure.
   Crate- and item-level doc comments are welcome; narrating comments are not.
2. **Tests live beside the code.** Each module declares `#[cfg(test)] mod tests;`
   pointing at a sibling `tests.rs` (or `tests/` folder with `mod.rs`).
   Integration tests in `tests/` only when they exercise the public API end to end.
3. **Every module folder is registered manually.** A new folder needs its own
   `mod.rs`, declared by its parent; keep `mod.rs` to declarations and re-exports.
4. **Gates.** `cargo fmt` and
   `cargo clippy --all-targets --all-features -- -D warnings` must pass, plus
   `cargo test`.
5. **No public API by accident.** Anything exported is documented and covered by
   a test that shows how it is meant to be used.
