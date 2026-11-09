//! Fuzz target: `Core\Xml::parse`'s strict reader.
//!
//! The check is `nvs_fuzz_checks::xml`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz xml --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::xml(data));
