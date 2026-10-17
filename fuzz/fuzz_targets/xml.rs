//! Fuzz target: `Core\Xml::parse`'s strict reader must not panic, overflow its
//! stack or hang on any text. It either builds a tree or refuses the whole
//! document (`rule:errors/ambiguous-input-refused`).
//!
//! Run with `cargo +nightly fuzz run xml` (needs `cargo-fuzz`; libFuzzer is not
//! supported on Windows, so this only runs where a nightly toolchain with a C
//! compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = nvs_stdlib::fuzz::xml(text);
    }
});
