//! Fuzz target: `Core\Markdown` and `comrak` must render the same document to
//! HTML with the same meaning. comrak is the other engine
//! `rule:core-classes/markdown-to-html` measured, and it is a dependency of
//! this crate alone.
//!
//! Both renderings are sanitized and their whitespace between and around tags
//! is collapsed (`nvs_stdlib::fuzz::html_meaning`), so raw HTML and layout
//! compare as a browser would show them. [`normal`] removes the differences
//! the rule chose, each named there. Any other difference is a bug in one of
//! the two: ours becomes a known-bug catalogue entry and a fix, and a reading
//! of the spec where comrak differs becomes a line in the rule and a case in
//! [`normal`].
//!
//! Run with `cargo +nightly fuzz run markdown_comrak` from the repository root
//! (needs `cargo-fuzz`; libFuzzer is not supported on Windows, so this runs in
//! WSL).

#![no_main]

use libfuzzer_sys::fuzz_target;
use nvs_stdlib::fuzz;

/// comrak's options for the one flavour the rule reads: CommonMark and GFM,
/// with raw HTML written out, since both results are sanitized here.
fn options() -> comrak::Options<'static> {
    let mut options = comrak::Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.render.r#unsafe = true;
    options
}

/// `html`'s meaning with the differences the rule chose removed.
fn normal(html: &str) -> String {
    fuzz::html_meaning(html)
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    // Footnotes are written as the rule writes them: a rule and a list with
    // `md-` ids, where comrak writes a `section`. A document with one is not
    // compared.
    if text.contains("[^") {
        return;
    }
    // CommonMark 0.31.2 counts a Unicode symbol as punctuation beside an
    // emphasis or strikethrough delimiter, and comrak 0.59 counts only
    // punctuation. NUL is
    // U+FFFD, a symbol. Such a document is not compared.
    // comrak 0.59's extended autolinks fail four GFM spec examples, and its
    // domain scan does not count the period of `www.` or take a symbol, as
    // cmark-gfm does. A document that may hold one is not compared.
    let lower = text.to_ascii_lowercase();
    if lower.contains("www.") || text.contains("://") || text.contains('@') {
        return;
    }
    let symbol = |c: char| c == '\0' || (!c.is_ascii() && !c.is_alphanumeric() && !c.is_whitespace());
    if text.contains(['*', '_', '~']) && text.contains(symbol) {
        return;
    }
    // CommonMark's delimiter algorithm runs `~` with `*` and `_`, so a closing
    // `_` removes a `~` opener before it. comrak 0.59 runs `~` apart, so
    // `_:\n~_~` gives it `_: <del>_</del>`. Such a document is not compared.
    if text.contains('~') && text.contains(['*', '_']) {
        return;
    }
    // After a run of backticks that nothing closes, comrak 0.59 reads one more
    // code span and no other: "``a `b` `c`" gives it one `code` element, and
    // CommonMark two. A document with such a run is not compared.
    if text.contains("``") {
        return;
    }
    // CommonMark 0.31.2 reads `<!X>` as a declaration, which is raw HTML.
    // comrak 0.59 follows 0.29, which needs whitespace after the name. A
    // document that may hold one is not compared.
    if text.as_bytes().windows(3).any(|w| w.starts_with(b"<!") && w[2].is_ascii_alphabetic()) {
        return;
    }
    // The end of a document ends its last line as a line feed does, so comrak
    // gets the text with one. comrak 0.59 reads `1.` with no line ending after
    // it as text, where CommonMark reads an empty list item. And CommonMark
    // replaces each NUL with U+FFFD, which comrak does not do in raw HTML, so
    // comrak gets the text with each NUL replaced.
    let replaced = text.replace('\0', "\u{FFFD}");
    let ended = if replaced.ends_with('\n') { replaced } else { replaced + "\n" };
    let ours = normal(&fuzz::markdown(text));
    let theirs = normal(&comrak::markdown_to_html(&ended, &options()));
    assert_eq!(ours, theirs, "{text:?}");
});
