//! Fuzz target: the `Cookie` and `Set-Cookie` readers.
//!
//! The check is `nvs_fuzz_checks::cookie`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz cookie --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::cookie(data));
