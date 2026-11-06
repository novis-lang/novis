//! GFM's extended autolinks, which `pulldown-cmark` does not read:
//! `www.` links, `http://` and `https://` and `ftp://` links, and email
//! addresses written in text with no `<` and `>` around them.
//!
//! [`spans`] finds them in one run of a paragraph's text, and the renderer
//! writes each span as a link. The scan is one pass over the text: each
//! position is read once as a candidate start, and a span's own scan moves
//! the position past its end. Only the email form looks back, and never past
//! the end of the span before it.
//!
//! Two bounds keep the pass linear. A domain is read for at most
//! [`MAX_DOMAIN`] bytes, so a failed candidate inside a long run of domain
//! characters costs a bounded scan and not the rest of the run. And the
//! parentheses of a span are counted once, before its end is trimmed.

use std::ops::Range;

/// One autolink: the text it covers and the URL it links to.
pub(super) struct Span {
    pub(super) range: Range<usize>,
    pub(super) url: String,
}

/// The schemes an extended URL autolink starts with.
const SCHEMES: &[&str] = &["http://", "https://", "ftp://"];

/// Every extended autolink in `text`, in order and not overlapping.
pub(super) fn spans(text: &str) -> Vec<Span> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut done = 0;
    let mut at = 0;
    while at < bytes.len() {
        let before = at.checked_sub(1).map(|i| bytes[i]);
        let span = if bytes[at] == b'@' {
            email(text, done, at)
        } else if text[at..].starts_with("www.")
            && before
                .is_none_or(|b| b.is_ascii_whitespace() || matches!(b, b'*' | b'_' | b'~' | b'('))
        {
            www(text, at)
        } else if before.is_none_or(|b| !b.is_ascii_alphanumeric()) {
            SCHEMES
                .iter()
                .find(|scheme| text[at..].starts_with(*scheme))
                .and_then(|scheme| url(text, at, scheme.len()))
        } else {
            None
        };
        match span {
            Some(span) => {
                at = span.range.end;
                done = at;
                out.push(span);
            }
            None => at += text[at..].chars().next().map_or(1, char::len_utf8),
        }
    }
    out
}

/// Whether `byte` may appear in a domain: a letter, a digit, `-`, `_` or
/// `.`. A byte of a non-ASCII character counts, as cmark-gfm counts it.
fn domain_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') || byte >= 0x80
}

/// The longest domain an autolink reads, in bytes. A DNS name is at most 253.
const MAX_DOMAIN: usize = 256;

/// The length of the valid domain at the start of `text`, or `None`.
///
/// A valid domain is segments of letters, digits, `_` and `-` between
/// periods, with no `_` in its last two segments, and no longer than
/// [`MAX_DOMAIN`]. `period` says whether it must hold one.
fn domain(text: &str, period: bool) -> Option<usize> {
    let run = text
        .bytes()
        .take(MAX_DOMAIN + 1)
        .take_while(|&b| domain_byte(b))
        .count();
    if run > MAX_DOMAIN || !text.is_char_boundary(run) {
        return None;
    }
    let core = text[..run].trim_end_matches('.');
    if core.is_empty() || (period && !core.contains('.')) {
        return None;
    }
    let underscore = core
        .rsplit('.')
        .take(2)
        .any(|segment| segment.contains('_'));
    (!underscore).then_some(run)
}

/// `end` moved back past the punctuation GFM leaves out of an autolink's end.
fn trimmed(text: &str, start: usize, mut end: usize) -> usize {
    let opens = text[start..end].matches('(').count();
    let mut closes = text[start..end].matches(')').count();
    loop {
        let link = &text[start..end];
        let Some(last) = link.bytes().last() else {
            return end;
        };
        match last {
            b'?' | b'!' | b'.' | b',' | b':' | b'*' | b'_' | b'~' => end -= 1,
            b')' if closes > opens => {
                closes -= 1;
                end -= 1;
            }
            b';' => {
                let name = link[..link.len() - 1]
                    .bytes()
                    .rev()
                    .take_while(u8::is_ascii_alphanumeric)
                    .count();
                let amp = link.len() - 1 - name;
                if name > 0 && amp > 0 && link.as_bytes()[amp - 1] == b'&' {
                    // An entity's name holds no parenthesis, so the counts
                    // stay as they are.
                    end = start + amp - 1;
                } else {
                    return end;
                }
            }
            _ => return end,
        }
    }
}

/// The end of the path after a domain ending at `at`: every character up to
/// whitespace or `<`.
fn path_end(text: &str, at: usize) -> usize {
    at + text[at..]
        .bytes()
        .take_while(|&b| !b.is_ascii_whitespace() && b != b'<')
        .count()
}

/// The `www.` autolink starting at `start`.
fn www(text: &str, start: usize) -> Option<Span> {
    let domain = domain(&text[start..], true)?;
    let end = trimmed(text, start, path_end(text, start + domain));
    (end > start + "www.".len()).then(|| Span {
        range: start..end,
        url: format!("http://{}", &text[start..end]),
    })
}

/// The URL autolink starting at `start`, whose scheme is `scheme` bytes long.
fn url(text: &str, start: usize, scheme: usize) -> Option<Span> {
    let domain = domain(&text[start + scheme..], false)?;
    let end = trimmed(text, start, path_end(text, start + scheme + domain));
    (end > start + scheme).then(|| Span {
        range: start..end,
        url: text[start..end].to_owned(),
    })
}

/// The email autolink whose `@` is at `at`, its local part starting no
/// earlier than `done`.
fn email(text: &str, done: usize, at: usize) -> Option<Span> {
    let bytes = text.as_bytes();
    let local = bytes[done..at]
        .iter()
        .rev()
        .take_while(|&&b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'+'))
        .count();
    if local == 0 {
        return None;
    }
    let start = at - local;
    let run = bytes[at + 1..]
        .iter()
        .take_while(|&&b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        .count();
    let host = text[at + 1..at + 1 + run].trim_end_matches('.');
    if !host.contains('.') || host.ends_with(['-', '_']) {
        return None;
    }
    let end = at + 1 + host.len();
    Some(Span {
        range: start..end,
        url: format!("mailto:{}", &text[start..end]),
    })
}
