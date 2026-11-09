//! Fuzz target: the multipart body reader.
//!
//! The check is `nvs_fuzz_checks::multipart`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz multipart --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::multipart(data));
