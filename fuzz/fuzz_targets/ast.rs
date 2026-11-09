//! Fuzz target: `Core\Ast::parse`'s walk over a parsed tree.
//!
//! The check is `nvs_fuzz_checks::ast`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz ast --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::ast(data));
