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
///
/// - A link whose URL fails the scheme check is written as its text, where
///   comrak writes the link and the sanitizer then removes its `href`. An `a`
///   with no attributes is removed and its text kept. An `a` never holds
///   another, so each `</a>` closes the last `<a` before it. An image whose
///   URL fails is removed, where comrak's loses its `src`, so an `img` with no
///   `src` is removed.
/// - A tab is a space, and a paragraph starts with no space. comrak 0.59 keeps
///   the spaces and tabs a paragraph starts with when a reference definition
///   before them is removed. CommonMark removes them, and a browser shows both
///   the same.
fn normal(html: &str) -> String {
    let mut spaced = String::with_capacity(html.len());
    for ch in fuzz::html_meaning(html).chars() {
        let ch = if ch == '\t' { ' ' } else { ch };
        if !(ch == ' ' && (spaced.ends_with(' ') || spaced.ends_with("<p>"))) {
            spaced.push(ch);
        }
    }
    let meaning = spaced;
    let mut out = String::with_capacity(meaning.len());
    let mut bare = false;
    let mut rest = meaning.as_str();
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        if let Some(after) = rest.strip_prefix("<a>") {
            bare = true;
            rest = after;
        } else if let Some(after) = rest.strip_prefix("</a>") {
            if !bare {
                out.push_str("</a>");
            }
            bare = false;
            rest = after;
        } else if let Some(end) = rest.starts_with("<img").then(|| rest.find('>')).flatten() {
            if rest[..end].contains(" src=") {
                out.push_str(&rest[..=end]);
            }
            rest = &rest[end + 1..];
        } else {
            if rest.starts_with("<a ") {
                bare = false;
            }
            out.push('<');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
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
    // CommonMark 0.31.2 counts only spaces and tabs as the whitespace around a
    // link's destination, after a table's delimiter row and in other places.
    // comrak 0.59 follows 0.29, which also counts a vertical tab and a form
    // feed, so `[a](b\u{c})` is a link and `--\u{c}` a delimiter row. A document
    // with either is not compared.
    if text.contains(['\u{b}', '\u{c}']) {
        return;
    }
    // When a `~` closer's nearest opener has the other length, cmark-gfm
    // writes the closer as text and goes on. comrak 0.59 stops, and every
    // delimiter after it in the paragraph is text. A document with a `~` run
    // and a `~~` run is not compared.
    let mut runs = text.split(|c| c != '~').map(str::len);
    if runs.clone().any(|n| n == 1) && runs.any(|n| n == 2) {
        return;
    }
    // A processing instruction ends at the first `?>`. comrak 0.59's scanner
    // reads `?` and the character after it as a pair, so `<?a??>` does not end
    // at `?>` and is text. A document with `??>` is not compared.
    if text.contains("??>") {
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
