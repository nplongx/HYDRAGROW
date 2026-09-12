//! API-layer test modules (compiled under `cfg(test)` only).
//!
//! NOTE: this module was previously orphaned (no `mod tests;` declaration in
//! `api/mod.rs`), so `cargo test` silently skipped every test in this
//! directory. It is now wired in; keep it that way.

mod test_scope_definitions;
