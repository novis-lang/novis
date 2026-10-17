//! Fuzz target: the two cookie readers must not panic on any bytes, and each
//! answers only what the bytes really say.
//!
//! `Core\Request::cookie` reads a `Cookie` field. Whatever value it gives for a
//! name must appear in the field as `name=value`, compared byte for byte
//! (`rule:errors/cookie-name-bytes`), and a `__Host-` or `__Secure-` name is
//! never visible over plain HTTP. `Core\Test` reads a `Set-Cookie` line, and
//! the name it gives is non-empty, carries no `;` or `=`, and is trimmed.
//!
//! The input is a flag byte, then the name up to the first NUL, then the field.
//!
//! Run with `cargo +nightly fuzz run cookie` (needs `cargo-fuzz`; libFuzzer is
//! not supported on Windows, so this only runs where a nightly toolchain with a
//! C compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_stdlib::fuzz::{request_cookie, set_cookie_pair};

/// Whether `haystack` contains `needle` anywhere.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || haystack.windows(needle.len()).any(|window| window == needle)
}

fuzz_target!(|data: &[u8]| {
    if let Some((name, value)) = set_cookie_pair(data) {
        assert!(!name.is_empty(), "a `Set-Cookie` name is never empty");
        assert!(!name.contains(&b';') && !name.contains(&b'='));
        assert!(!value.contains(&b';'));
        for edge in [name.first(), name.last(), value.first(), value.last()] {
            assert!(!matches!(edge, Some(b' ' | b'\t')), "a name or value is trimmed");
        }
    }

    let Some((&flag, rest)) = data.split_first() else {
        return;
    };
    let https = flag & 1 == 1;
    let (name, field) = match rest.iter().position(|&byte| byte == 0) {
        Some(at) => (&rest[..at], &rest[at + 1..]),
        None => (rest, &rest[rest.len()..]),
    };
    let Some(value) = request_cookie(field, name, https) else {
        return;
    };
    assert!(
        https || !(name.starts_with(b"__Host-") || name.starts_with(b"__Secure-")),
        "a prefixed cookie is visible over plain HTTP"
    );
    let pair = [name, b"=", &value].concat();
    assert!(contains(field, &pair), "the value is not one the field carries");
});
