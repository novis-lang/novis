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
///   with no `href` is removed, with its `title`, and its text kept. An `a` never holds
///   another, so each `</a>` closes the last `<a` before it. An image whose
///   URL fails is removed, where comrak's loses its `src`, so an `img` with no
///   `src` is removed.
/// - A tab is a space, and a paragraph or a tight list item starts with no
///   space. comrak 0.59 keeps
///   the spaces and tabs a paragraph starts with when a reference definition
///   before them is removed. CommonMark removes them, and a browser shows both
///   the same. A space between two tags is removed, as `html_meaning` does: in
///   `> \t<div>` comrak keeps the tab in front of the HTML block and ours
///   writes the spaces it stands for.
/// - A `%` with no two hex digits after it is `%25` (`lone_percent_encoded`).
fn normal(html: &str) -> String {
    let meaning = fuzz::html_meaning(&unnested(html));
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
        } else if let Some(end) = rest.starts_with("<img").then(|| tag_end(rest)).flatten() {
            if rest[..end].contains(" src=") {
                out.push_str(&rest[..=end]);
            }
            rest = &rest[end + 1..];
        } else if let Some(end) = rest.starts_with("<a ").then(|| tag_end(rest)).flatten() {
            bare = !rest[..end].contains(" href=");
            if !bare {
                out.push_str(&rest[..=end]);
            }
            rest = &rest[end + 1..];
        } else {
            out.push('<');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    // Spaces collapse once the tags are removed, so the text on both sides
    // of a removed `a` joins as a browser shows it.
    let mut spaced = String::with_capacity(out.len());
    for ch in out.chars() {
        let ch = if ch == '\t' { ' ' } else { ch };
        let opened = spaced.ends_with("<p>") || spaced.ends_with("<li>");
        if !(ch == ' ' && (spaced.ends_with(' ') || opened)) {
            spaced.push(ch);
        }
    }
    lone_percent_encoded(&spaced.replace("> <", "><"))
}

/// `html` with each `a` inside another `a` removed, and its text kept. An
/// autolink inside a link's text whose URL fails the scheme check is text in
/// ours. comrak writes it as a nested `a`, and the HTML parser then closes the
/// outer link there. Both sides are mapped, so a nested link that both write
/// is removed from both.
fn unnested(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut depth = 0usize;
    let mut rest = html;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let open = rest.starts_with("<a ") || rest.starts_with("<a>");
        if let Some(end) = open.then(|| tag_end(rest)).flatten() {
            if depth == 0 {
                out.push_str(&rest[..=end]);
            }
            depth += 1;
            rest = &rest[end + 1..];
        } else if let Some(after) = rest.strip_prefix("</a>") {
            if depth <= 1 {
                out.push_str("</a>");
            }
            depth = depth.saturating_sub(1);
            rest = after;
        } else {
            out.push('<');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

/// `html` with each `%` not followed by two hex digits written as `%25`.
/// cmark writes such a `%` in a URL as it is, and so does ours. comrak 0.59
/// encodes it, so `[a](%u)` gives it `href="%25u"`. Text is the same on both
/// sides, so the whole document is mapped.
fn lone_percent_encoded(html: &str) -> String {
    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len());
    for (at, ch) in html.char_indices() {
        let hex = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_hexdigit);
        if ch == '%' && !(hex(at + 1) && hex(at + 2)) {
            out.push_str("%25");
        } else {
            out.push(ch);
        }
    }
    out
}

/// The index of the `>` that ends the tag `html` starts with. The serializer
/// does not escape `<` or `>` in an attribute value, so a `>` between quotes is
/// part of the value.
fn tag_end(html: &str) -> Option<usize> {
    let mut quoted = false;
    html.char_indices().find_map(|(at, ch)| {
        if ch == '"' {
            quoted = !quoted;
        }
        (ch == '>' && !quoted).then_some(at)
    })
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
    // cmark (issue 607) reads a `\` in a link title as an escape, so `\"`
    // never ends the title. comrak 0.59 follows cmark-gfm's older pattern,
    // where a `\` may be a character of its own and the longest title wins:
    // `[a](/u "\")` is a link for it, and text for cmark. A document with a
    // `\` before a title's closing delimiter is not compared.
    if text.as_bytes().windows(2).any(|w| w[0] == b'\\' && matches!(w[1], b'"' | b'\'' | b')')) {
        return;
    }
    // When a paragraph is only reference definitions, a `---` under it is a
    // thematic break in cmark and CommonMark's reference parser. comrak 0.59
    // copies cmark-gfm's older code, which writes it as text. A document with
    // `]:` and a line of `-` alone is not compared.
    let dashes = |line: &str| {
        let line = line.trim();
        !line.is_empty() && line.bytes().all(|b| b == b'-')
    };
    if text.contains("]:") && text.split(['\n', '\r']).any(dashes) {
        return;
    }
    // A CDATA section ends at the first `]]>`. cmark's pattern, which comrak
    // 0.59 copies, reads `]]` and one more character as content, so in
    // `<![CDATA[x]]]>z` it reads past the end and writes text. A document with
    // a CDATA start and `]]]` is not compared.
    if text.contains("<![CDATA[") && text.contains("]]]") {
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
    // cmark-gfm needs whitespace after a task marker's `]`. comrak 0.59 looks
    // only at the item's first text node, so in `- [x]*a*` the `*` ends that
    // node and the marker counts. A document with a marker followed by a
    // character that starts an inline is not compared.
    let inline = |b: u8| b"!*_<`[&\\~]".contains(&b);
    if text.as_bytes().windows(4).any(|w| {
        w[0] == b'[' && matches!(w[1], b' ' | b'x' | b'X') && w[2] == b']' && inline(w[3])
    }) {
        return;
    }
    // comrak 0.59 opens no container past a depth of 100, and the rule's bound
    // is 512. A line whose container markers and indentation could reach 100
    // is not compared.
    let lead = |line: &str| line.bytes().take_while(|b| b" \t>-+*.)0123456789".contains(b)).count();
    if text.split(['\n', '\r']).any(|line| lead(line) >= 100) {
        return;
    }
    // An HTML block of type 7 cannot interrupt a paragraph, and CommonMark
    // 0.31.2 and cmark hold that for a lazy continuation line too. cmark-gfm
    // and comrak 0.59 do not, so `>a\n<t>` ends their block quote. A document
    // with a `<` line in a paragraph a container started is not compared.
    // comrak 0.59 also keeps the spaces a lazy line starts with, which shows
    // in a code span: `>a \`\n \`c` gives it `<code>  </code>`. On a line its
    // container continues, it removes them, as CommonMark does. A document
    // with a code span and an indented line in such a paragraph is not compared.
    // A `\r\n` ends one line, so it is split first.
    let mut contained = false;
    let code = text.contains('`');
    for line in text.split("\r\n").flat_map(|line| line.split(['\n', '\r'])) {
        let start = line.trim_start_matches([' ', '\t']);
        let marker = start.starts_with(['>', '-', '+', '*'])
            || start.trim_start_matches(|c: char| c.is_ascii_digit()).starts_with(['.', ')']);
        if start.is_empty() {
            contained = false;
        } else if marker {
            contained = true;
        } else if contained && (start.starts_with('<') || (code && start.len() < line.len())) {
            return;
        }
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
