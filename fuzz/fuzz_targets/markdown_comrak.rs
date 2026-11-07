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
///   `src` is removed. A URL that fails only percent-decoded (`allowed`) fails
///   too.
/// - A tab is a space, and a paragraph or a tight list item starts with no
///   space. comrak 0.59 keeps
///   the spaces and tabs a paragraph starts with when a reference definition
///   before them is removed. CommonMark removes them, and a browser shows both
///   the same. A space between two tags is removed, as `html_meaning` does: in
///   `> \t<div>` comrak keeps the tab in front of the HTML block and ours
///   writes the spaces it stands for.
/// - A `%` with no two hex digits after it is `%25` (`lone_percent_encoded`).
fn normal(html: &str) -> String {
    let meaning = fuzz::html_meaning(&unlinked(&unnested(html)));
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
            if allowed(&rest[..end], "src") {
                out.push_str(&rest[..=end]);
            }
            rest = &rest[end + 1..];
        } else if let Some(end) = rest.starts_with("<a ").then(|| tag_end(rest)).flatten() {
            bare = !allowed(&rest[..end], "href");
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

/// Whether the start tag `tag` has the attribute `name` with a URL that
/// `Core\Markdown` writes. The sanitizer reads a URL as a browser does, and the
/// renderer also checks it percent-decoded, so `%5C%5C/x` passes the first and
/// fails the second. Ours writes no URL that fails, so the check maps only
/// comrak's.
fn allowed(tag: &str, name: &str) -> bool {
    let Some(at) = tag.find(&format!(" {name}=\"")) else {
        return false;
    };
    let value = &tag[at + name.len() + 3..];
    let value = &value[..value.find('"').unwrap_or(value.len())];
    let url = value
        .replace("&quot;", "\"")
        .replace("&nbsp;", "\u{a0}")
        .replace("&amp;", "&");
    fuzz::markdown_url_allowed(&url)
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

/// `html` with each `a` whose `href` fails the check removed, and its text
/// kept. This runs before `html_meaning`, which removes a space between two
/// tags. In `*b*  <x:y>`, ours writes the spaces in front of the autolink's
/// text, and comrak writes them between `</em>` and the `a`. `html` has no
/// `a` inside another (`unnested`).
fn unlinked(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut removed = false;
    let mut rest = html;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        if let Some(end) = rest.starts_with("<a ").then(|| tag_end(rest)).flatten() {
            removed = !allowed(&rest[..end], "href");
            if !removed {
                out.push_str(&rest[..=end]);
            }
            rest = &rest[end + 1..];
        } else if let Some(after) = rest.strip_prefix("</a>").filter(|_| removed) {
            removed = false;
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
    // `unnested` removes an `a` inside another `a`. A raw `<a>` tag in the
    // document opens an `a` that comrak writes as it is, and a link after it
    // is then nested and removed. Ours is already parsed, and the parser
    // closes the raw `a` where the link opens, so the link stays. A document
    // with a raw `<a` tag is not compared.
    if lower.as_bytes().windows(3).any(|w| w[0] == b'<' && w[1] == b'a' && b" \t\r\n/>".contains(&w[2])) {
        return;
    }
    // A link whose text has raw HTML is rebuilt from its source in ours, and
    // an autolink in that text then closes the link, as a browser reads
    // cmark's output. `unnested` removes the autolink from comrak's output
    // instead. A document with `[`, an angle autolink and another tag is not
    // compared.
    let bytes = text.as_bytes();
    let autolink = |at: usize| {
        let scheme = bytes[at + 1..].iter().take_while(|b| b.is_ascii_alphanumeric() || b"+.-".contains(b)).count();
        bytes[at + 1].is_ascii_alphabetic() && (2..=32).contains(&scheme) && bytes.get(at + 1 + scheme) == Some(&b':')
    };
    let tags: Vec<usize> = (0..bytes.len().saturating_sub(1))
        .filter(|&at| bytes[at] == b'<' && (bytes[at + 1].is_ascii_alphabetic() || b"/!?".contains(&bytes[at + 1])))
        .collect();
    if text.contains('[') && tags.iter().any(|&at| autolink(at)) && tags.iter().any(|&at| !autolink(at)) {
        return;
    }
    // In a table cell, cmark-gfm removes the `\` in front of every `|` before
    // the cell's inline parse, so `\\\\|b` gives `\|b`. comrak 0.59 steps over
    // `\\` pairs first and keeps that `\`, which gives `\\|b`. A document with
    // `\\|` is not compared.
    if text.contains("\\\\|") {
        return;
    }
    // An escaped `&` is a plain character, so `\&amp;` in a link destination
    // is the text `&amp;`, as commonmark.js reads it. cmark and comrak 0.59
    // decode entities before backslash escapes, so they read `&`. A document
    // with `\&` is not compared.
    if text.contains("\\&") {
        return;
    }
    // A link label needs a character that is not whitespace, so in `[a][ ]`
    // the `[ ]` is text after a shortcut reference. cmark trims the second
    // label and reads an empty one as a collapsed reference, and comrak 0.59
    // copies it. A document with `][` and only whitespace before the next `]`
    // is not compared.
    if text.match_indices("][").any(|(at, _)| {
        let rest = &text[at + 2..];
        rest.find(']').is_some_and(|end| end > 0 && rest[..end].trim().is_empty())
    }) {
        return;
    }
    // A definition's title must end its line. When text follows it, cmark
    // reads the definition again with no title. comrak 0.59 copies cmark's
    // older code, which keeps the title: `[a]: /u\n't'x` gives `a` the title
    // `t`. A document with `]:` and a line with a title delimiter that does
    // not end with one is not compared.
    let delimiter = |c: char| matches!(c, '"' | '\'' | '(' | ')');
    if text.contains("]:")
        && text.split(['\n', '\r']).any(|line| {
            line.contains(delimiter) && !line.trim_end().ends_with(delimiter)
        })
    {
        return;
    }
    // CommonMark 0.31.2 ends an HTML comment at the first `-->`, so
    // `<!-- --->` is a comment. comrak 0.59 follows 0.29, where the text of a
    // comment cannot end with `-`, and writes it as text. A document with a
    // comment start and `--->` is not compared.
    if text.contains("<!--") && text.contains("--->") {
        return;
    }
    // A line ending in a code span is one space, and cmark writes `\r\n` as
    // one. comrak 0.59 writes a space for the `\r` and one for the `\n`, so
    // the span keeps a space that CommonMark strips. A document with a
    // backtick and `\r\n` is not compared.
    if text.contains('`') && text.contains("\r\n") {
        return;
    }
    // A CDATA section ends at the first `]]>`. cmark's pattern, which comrak
    // 0.59 copies, reads `]]` and one more character as content, so in
    // `<![CDATA[x]]]>z` it reads past the end and writes text. A document with
    // a CDATA start and `]]]` is not compared.
    if text.contains("<![CDATA[") && text.contains("]]]") {
        return;
    }
    // `<![CDATA[` starts an HTML block or a CDATA section only in capitals,
    // as in cmark. comrak 0.59 reads it in any case, so `<![CDAtA[` is HTML
    // for it. A document with another case of it is not compared.
    if lower.matches("<![cdata[").count() != text.matches("<![CDATA[").count() {
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
    let raw = comrak::markdown_to_html(&ended, &options());
    // A link cannot hold a link, so once a link forms, cmark deactivates every
    // `[` before it. comrak 0.59 opens the outer link again after a later `[b]`
    // that is not a link, so `[[a](u)[b]](v)` gives it an `a` inside an `a`.
    // With no `<` in the document, only that can nest an `a`, and such a
    // document is not compared.
    if !text.contains('<') && unnested(&raw) != raw {
        return;
    }
    let mut theirs = normal(&raw);
    // cmark lets references add the document's length in bytes, and at least
    // 100,000. comrak 0.59 swaps `max` for `min`, so it allows the length and
    // at most 100,000, and a third use of a long URL in a short document is
    // text. On a difference in a document with a definition, comrak gets the
    // text again after a comment line that makes the document 100,000 bytes
    // long, and the comment is removed from its HTML.
    if ours != theirs && text.contains("]:") && !text.starts_with('\u{feff}') {
        let comment = format!("<!--{}-->\n", "x".repeat(100_000));
        let padded = comrak::markdown_to_html(&format!("{comment}{ended}"), &options());
        if let Some(html) = padded.strip_prefix(comment.as_str()) {
            theirs = normal(html);
        }
    }
    assert_eq!(ours, theirs, "{text:?}");
});
