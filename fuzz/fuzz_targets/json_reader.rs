//! Fuzz target: `Core\Json`'s own reader, against `serde_json` as its oracle.
//!
//! The check is `nvs_fuzz_checks::json_reader`, which `cargo test` also runs
//! over every saved input under `fuzz/seeds/json/`. Search with
//! `bun nv fuzz json_reader --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::json_reader(data));
