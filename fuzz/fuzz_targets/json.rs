//! Fuzz target: `Core\Json::decode` and its round trip through `encode`.
//!
//! The check is `nvs_fuzz_checks::json`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz json --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::json(data));
