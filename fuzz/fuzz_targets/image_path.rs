//! Fuzz target: `rule:packaging/the-argv-lives-in-imagepath`'s encoder must round-trip any argv —
//! `nvs_diagnostics::argv::image_path` → `decode` gives back the binary and every word unchanged.
//!
//! The input is split on NUL into words, since no argument the SCM stores can hold one. The first
//! word is the binary, kept to what a path can be: no quote, and no trailing backslash
//! (`decode`'s doc says why the first word is read that way).
//!
//! Run with `cargo +nightly fuzz run image_path` (needs `cargo-fuzz`; libFuzzer is
//! not supported on Windows, so this only runs where a nightly toolchain
//! with a C compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_diagnostics::argv::{decode, image_path};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut words = text.split('\0').map(str::to_owned);
    let exe = words.next().unwrap_or_default();
    if exe.contains('"') || exe.ends_with('\\') {
        return;
    }
    let argv: Vec<String> = words.collect();

    let line = image_path(&exe, &argv);
    assert!(line.starts_with('"'), "the binary is quoted: {line}");
    let decoded = decode(&line);
    let (image, rest) = decoded.split_first().expect("the binary comes back first");
    assert_eq!(image, &exe, "{line}");
    assert_eq!(rest, argv.as_slice(), "{line}");
});
