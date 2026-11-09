//! Fuzz target: `Core\Markdown`'s renderer.
//!
//! The check is `nvs_fuzz_checks::markdown`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz markdown --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::markdown(data));
