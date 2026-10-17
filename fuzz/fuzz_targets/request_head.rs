//! Fuzz target: the scanner that reads a connection's request bytes beside
//! `hyper` must not panic, must give the same verdicts however the bytes are
//! split across reads, and must refuse a first head that
//! `rule:errors/http-message-defects` refuses: one with a bare LF ending a
//! line, or with a `Content-Length` given twice.
//!
//! The oracle for the last half is `httparse`, the head reader `hyper` uses.
//! Where it reads a complete first head, that head's bytes and headers say
//! whether the rule refuses it. A refused head is never answered, because the
//! service asks the scanner before anything else (`nvs_server::heads`).
//!
//! The first input byte chooses where the rest is split in two.
//!
//! Run with `cargo +nightly fuzz run request_head` (needs `cargo-fuzz`;
//! libFuzzer is not supported on Windows, so this only runs where a nightly
//! toolchain with a C compiler is available — see AGENTS.md's WSL section).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_server::heads::Scanner;

/// How many requests each run asks the scanner about.
const REQUESTS: u64 = 8;

/// The verdict on each of the first [`REQUESTS`] requests, for bytes that
/// arrive in `reads`.
fn verdicts<'a>(reads: impl IntoIterator<Item = &'a [u8]>) -> Vec<bool> {
    let mut scanner = Scanner::default();
    for read in reads {
        scanner.feed(read);
    }
    let heads = scanner.heads();
    (0..REQUESTS).map(|index| heads.refused(index)).collect()
}

fuzz_target!(|data: &[u8]| {
    let Some((&cut, data)) = data.split_first() else {
        return;
    };
    let whole = verdicts([data]);
    let at = usize::from(cut) * data.len() / 256;
    let (front, back) = data.split_at(at);
    assert_eq!(verdicts([front, back]), whole, "split at {at}");
    assert_eq!(verdicts(data.chunks(1)), whole, "a byte at a time");

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut request = httparse::Request::new(&mut headers);
    let Ok(httparse::Status::Complete(end)) = request.parse(data) else {
        return;
    };
    let head = &data[..end];
    let bare_lf = head
        .iter()
        .enumerate()
        .any(|(at, &byte)| byte == b'\n' && (at == 0 || head[at - 1] != b'\r'));
    let lengths: usize = request
        .headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case("content-length"))
        .map(|header| 1 + usize::from(header.value.contains(&b',')))
        .sum();
    if bare_lf || lengths > 1 {
        assert!(whole[0], "a head the rule refuses was not refused");
    }
});
