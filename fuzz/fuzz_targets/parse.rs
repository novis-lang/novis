//! Fuzz target: the parser.
//!
//! The check is `nvs_fuzz_checks::parse`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz parse --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::parse(data));
