//! Fuzz target: `Core\Markdown`'s renderer must not panic on any text, and its
//! output must hold the properties `rule:core-classes/markdown-cost` and
//! `rule:core-classes/markdown-chunks` state:
//!
//! - sanitizing the HTML again changes nothing;
//! - the HTML grows at most 32 bytes per input byte, plus the added-output
//!   budget, as the known-bug catalogue's `output = "bounded"` entries say;
//! - `toHtmlEach`'s groups, read in pieces of the first byte's size, join to
//!   the whole document's HTML;
//! - every `open` part `Core\Markdown\Live` gives is a sanitize fixed point, and
//!   over a document with no reference definition the `done` parts and
//!   `finish()` join to the whole document's HTML.
//!
//! Run with `cargo +nightly fuzz run markdown` from the repository root (needs
//! `cargo-fuzz`; libFuzzer is not supported on Windows, so this runs in WSL).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_stdlib::fuzz;

fuzz_target!(|data: &[u8]| {
    let Some((&cut, rest)) = data.split_first() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(rest) else {
        return;
    };
    let html = fuzz::markdown(text);
    assert_eq!(fuzz::html_sanitized(&html), html, "sanitized again");
    assert!(
        html.len() <= 32 * text.len() + fuzz::MARKDOWN_ADDED_OUTPUT,
        "{} bytes of HTML for {} bytes of Markdown",
        html.len(),
        text.len()
    );

    let piece = usize::from(cut % 16) + 1;
    assert_eq!(fuzz::markdown_groups(text, piece).concat(), html, "groups in pieces of {piece}");

    let (parts, rest) = fuzz::markdown_live(text, piece);
    for (_, open) in &parts {
        assert_eq!(&fuzz::html_sanitized(open), open, "Live's open part sanitized again");
    }
    // A definition changes only what comes after it in Live, so the parts join
    // to the whole document only where there is none.
    if !text.contains("]:") {
        let joined: String = parts.iter().map(|(done, _)| done.as_str()).collect::<String>() + &rest;
        assert_eq!(joined, html, "Live in pieces of {piece}");
    }
});
