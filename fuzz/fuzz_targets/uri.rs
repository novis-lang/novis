//! Fuzz target: `Core\Uri::parse` and its round trip through `toString`.
//!
//! The check is `nvs_fuzz_checks::uri`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz uri --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::uri(data));
