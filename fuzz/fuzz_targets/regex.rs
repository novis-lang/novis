//! Fuzz target: compiling a `Core\Regex` pattern must not panic or hang on any
//! text, and the engine a pattern lands on is the same every time it is
//! compiled (`rule:core-classes/regex-two-tiers`).
//!
//! Run with `cargo +nightly fuzz run regex` (needs `cargo-fuzz`; libFuzzer is
//! not supported on Windows, so this only runs where a nightly toolchain with a
//! C compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_stdlib::regex::validate;

fuzz_target!(|data: &[u8]| {
    let Ok(pattern) = std::str::from_utf8(data) else {
        return;
    };
    let once = validate(pattern);
    assert_eq!(once, validate(pattern), "a pattern's tier is fixed");
});
