/// `lib.rs` used to make it available to call functions for integration test in `tests/`.
pub mod chunk;
#[cfg(debug_assertions)]
pub mod common;
pub mod file;
pub mod heap;
pub mod object;
pub mod parser;
pub mod tokenizer;
pub mod vm;
