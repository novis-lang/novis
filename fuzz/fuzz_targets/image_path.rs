//! Fuzz target: the encoder that writes an argv into a Windows service's `ImagePath`.
//!
//! The check is `nvs_fuzz_checks::image_path`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/`. Search with
//! `bun nv fuzz image_path --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::image_path(data));
