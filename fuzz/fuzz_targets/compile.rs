//! Fuzz target: the whole compiler, from the parser to code generation.
//!
//! The check is `nvs_fuzz_checks::compile`, which `cargo test` also runs over
//! every saved input under `fuzz/seeds/compile/`. It is slow per input, so no
//! run of `bun nv fuzz --all` takes it. Search with
//! `bun nv fuzz compile --seconds <n>` from Windows.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::compile(data));
