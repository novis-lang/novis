//! Fuzz target: the lexer must not panic on any input, valid UTF-8 or not.
//!
//! Run with `cargo +nightly fuzz run lex` (needs `cargo-fuzz`; libFuzzer is
//! not supported on Windows, so this only runs where a nightly toolchain
//! with a C compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_syntax::tokenize;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut map = SourceMap::new();
    let id = map.add("fuzz.nvs", text.to_string());
    let mut diags = Diagnostics::new();
    let _ = tokenize(map.file(id), &mut diags);
});
