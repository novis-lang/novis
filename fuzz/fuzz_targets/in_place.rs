//! Fuzz target: small programs that replace a value with one made from it,
//! each run with in-place changes and without them, which must print the same.
//!
//! The check is `nvs_fuzz_checks::in_place`, which `cargo test` also runs over
//! a fixed set of seeds. It compiles and runs two programs per input, so no
//! run of `bun nv fuzz --all` takes it. Search with
//! `bun nv fuzz in_place --seconds <n>`.

#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| nvs_fuzz_checks::in_place(data));
