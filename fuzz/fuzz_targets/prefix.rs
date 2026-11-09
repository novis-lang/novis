//! Fuzz target: the parser over a document cut in two.
//!
//! The check is `nvs_fuzz_checks::prefix`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz prefix --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::prefix(data));
