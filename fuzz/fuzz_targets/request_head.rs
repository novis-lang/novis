//! Fuzz target: the server's request head scanner.
//!
//! The check is `nvs_fuzz_checks::request_head`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz request_head --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::request_head(data));
