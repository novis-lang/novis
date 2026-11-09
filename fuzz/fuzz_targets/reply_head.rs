//! Fuzz target: the reader of an outbound call's reply head.
//!
//! The check is `nvs_fuzz_checks::reply_head`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz reply_head --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::reply_head(data));
