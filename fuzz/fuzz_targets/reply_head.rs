//! Fuzz target: the reader of an outbound call's reply head must not panic on
//! any bytes, and a head it accepts says one thing about where the body ends.
//!
//! Every accepted header has a lower-cased name and a trimmed value, and
//! neither carries a line break, so no header can hide a second one inside it.
//! A `Content-Length` framing means exactly one `Content-Length` and no
//! `Transfer-Encoding` (`rule:errors/http-message-defects`).
//!
//! Run with `cargo +nightly fuzz run reply_head` (needs `cargo-fuzz`; libFuzzer
//! is not supported on Windows, so this only runs where a nightly toolchain
//! with a C compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_stdlib::fuzz::{Framing, ReplyHead, reply_head};

fuzz_target!(|data: &[u8]| {
    let Some(ReplyHead {
        headers, framing, ..
    }) = reply_head(data)
    else {
        return;
    };
    for (name, value) in &headers {
        assert!(!name.bytes().any(|byte| byte.is_ascii_uppercase()));
        assert!(!name.bytes().any(|byte| byte.is_ascii_control()));
        assert!(!value.contains(['\r', '\n']), "a value carries a line break");
        assert_eq!(value.trim(), value, "a value is trimmed");
    }
    let count = |field: &str| headers.iter().filter(|(name, _)| name == field).count();
    if let Framing::Sized(length) = framing {
        assert_eq!(count("content-length"), 1);
        assert_eq!(count("transfer-encoding"), 0);
        let written = headers
            .iter()
            .find(|(name, _)| name == "content-length")
            .map(|(_, value)| value.as_str());
        assert_eq!(written.and_then(|value| value.parse().ok()), Some(length));
    }
});
