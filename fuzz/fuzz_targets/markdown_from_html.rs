//! Fuzz target: `Core\Markdown::fromHtml` must not panic on any HTML, and its
//! Markdown must hold `rule:core-classes/markdown-from-html`'s properties:
//!
//! - writing the Markdown's own HTML again gives the same Markdown;
//! - the input read as Markdown with no raw HTML, rendered, written back and
//!   rendered again, has the same meaning as the first rendering: the same
//!   sanitized HTML once whitespace between and around tags is collapsed.
//!
//! Raw HTML in Markdown is not round-tripped. `fromHtml` writes a `div`'s
//! children as blocks of the container around it, and an empty paragraph or
//! list as nothing, as `rule:core-classes/markdown-from-html` says, so its
//! sanitized HTML does not come back tag for tag. The first property covers
//! that HTML, since it holds over any input.
//!
//! Run with `cargo +nightly fuzz run markdown_from_html` from the repository
//! root (needs `cargo-fuzz`; libFuzzer is not supported on Windows, so this
//! runs in WSL).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_stdlib::fuzz;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let first = fuzz::from_html(text);
    let second = fuzz::from_html(&fuzz::markdown(&first));
    assert_eq!(second, first, "written from its own HTML");

    if fuzz::markdown_has_raw_html(text) {
        return;
    }
    let html = fuzz::markdown(text);
    let again = fuzz::markdown(&fuzz::from_html(&html));
    assert_eq!(fuzz::html_meaning(&again), fuzz::html_meaning(&html), "the round trip");
});
