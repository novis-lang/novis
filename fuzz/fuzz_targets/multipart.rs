//! Fuzz target: the multipart reader behind `Core\Request::files()` and
//! `post()` must not panic on any body, and how the body is cut into wire
//! chunks must not change what it reads.
//!
//! The first byte picks a chunk size from 1 to 16 and the rest is the body,
//! under the fixed boundary `B` so the fuzzer reaches delimiters quickly. The
//! same body is read again as one chunk, and both reads must give the same
//! parts and fields, or both refuse: a parse whose answer depends on where the
//! packets broke is two readers of one message
//! (`rule:errors/ambiguous-input-refused`). The whole input is also offered to
//! the `Content-Type` boundary reader.
//!
//! Run with `cargo +nightly fuzz run multipart` (needs `cargo-fuzz`; libFuzzer
//! is not supported on Windows, so this only runs where a nightly toolchain
//! with a C compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_stdlib::fuzz::{multipart, multipart_boundary};

/// The form-field cap each read runs under, far above anything libFuzzer
/// writes by default, so a refusal is the parser's and never the cap's.
const CAP: usize = 1 << 20;

fuzz_target!(|data: &[u8]| {
    let _ = multipart_boundary(data);
    let Some((&first, body)) = data.split_first() else {
        return;
    };
    let chunk = usize::from(first % 16) + 1;
    let pieces = multipart(b"B", body, chunk, CAP);
    let whole = multipart(b"B", body, body.len(), CAP);
    match (pieces, whole) {
        (Ok(pieces), Ok(whole)) => assert_eq!(pieces, whole, "chunking changed the parts"),
        (Err(_), Err(_)) => {}
        (pieces, whole) => panic!(
            "chunks of {chunk} and one chunk disagree: {:?} against {:?}",
            pieces.map(drop),
            whole.map(drop)
        ),
    }
});
