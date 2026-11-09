//! Fuzz target: `Core\Regex`'s pattern compiler.
//!
//! The check is `nvs_fuzz_checks::regex`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz regex --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::regex(data));
