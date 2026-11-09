//! Fuzz target: every `Core\Compress` decoder.
//!
//! The check is `nvs_fuzz_checks::decompress`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz decompress --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::decompress(data));
