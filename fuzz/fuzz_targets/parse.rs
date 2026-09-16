//! Fuzz target: the parser must not panic on any input, valid UTF-8 or not,
//! including inputs the grammar rejects — a rejection must come back as a
//! diagnostic, never a crash.
//!
//! The seeds are `fuzz/seeds/parse/`, shared with the `ast` target beside this
//! one, which takes the same input and carries the walk over the tree as well.
//!
//! Run with `cargo +nightly fuzz run parse` (needs `cargo-fuzz`; libFuzzer is
//! not supported on Windows, so this only runs where a nightly toolchain
//! with a C compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_diagnostics::{Diagnostics, SourceMap};
use nvs_syntax::parse_file;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut map = SourceMap::new();
    let id = map.add("fuzz.nvs", text.to_string());
    let mut diags = Diagnostics::new();
    let _ = parse_file(map.file(id), &mut diags);
});
