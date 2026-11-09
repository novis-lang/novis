//! `rule:core-classes/markdown-chunks`'s block groups: a document written one
//! group of top-level blocks at a time, each group's HTML the same bytes the
//! whole document's HTML holds there.
//!
//! # One walk, cut where nothing is open
//!
//! The engine reads the document in segments ([`super::segments`]), and one
//! renderer state walks the events of each in turn, as `toHtml`'s walks the
//! whole document's. After each event the walk asks
//! [`Renderer::group_ends`](super::Renderer::group_ends), and where that
//! holds it takes the tree built so far out as a group. Everything that spans
//! groups stays in the renderer's state: the added-output budget, the
//! footnote numbers and definitions, and whether the last thing written ends
//! a line. So a group is the slice of the whole tree that lies between two
//! cuts, and footnotes are the last group.
//!
//! # Raw HTML decides where a cut may go
//!
//! A group with raw HTML is rebuilt alone, through the sanitizer's list. That
//! gives the bytes the whole document's rebuild gives only when no element,
//! comment or raw-text element its raw HTML opened is still open at the cut.
//! [`Balance`] reads each piece of raw HTML with a small tokenizer and keeps
//! the elements it opened. It errs one way only: where it cannot tell, it
//! keeps the group open. So a document whose raw HTML never closes is one
//! group, which is decision 7's "correctness first".
//!
//! # A later group is parsed where the whole one was
//!
//! The whole document's parse reads a later group in body mode, after its
//! first group fixed the quirks mode. [`Written`] parses each later group
//! behind the first group's leading doctype and a `<body>` tag, so the parser
//! is in the same mode, and rebuilds it with
//! [`crate::html::rebuilt_in_body`], which keeps its leading whitespace. A
//! group the rebuild leaves empty, such as a lone declaration, does not count
//! as the first: the whole parse drops whitespace up to its first element, so
//! the group after it is written as the document's start.
//!
//! **What it spends:** past `toHtml`'s own state, one group's tree and source,
//! the raw HTML tokenizer's open element names, and the first group's doctype.

use nvs_runtime::Fault;

#[cfg(test)]
use super::callbacks::Callbacks;
use super::callbacks::Trusted;
use crate::html::{settled, source_with};
use crate::xml::Parsed;

/// The elements whose content the HTML tokenizer reads as text up to their
/// own end tag.
const RAW_TEXT: [&str; 9] = [
    "script", "style", "xmp", "iframe", "noembed", "noframes", "noscript", "textarea", "title",
];

/// The elements that have no end tag.
const VOID: [&str; 15] = [
    "area", "base", "basefont", "bgsound", "br", "col", "embed", "frame", "hr", "img", "input",
    "keygen", "link", "meta", "param",
];

/// More void elements, kept apart so each list fits a line.
const VOID_MORE: [&str; 3] = ["source", "track", "wbr"];

/// Where the tokenizer is between two pieces of raw HTML.
#[derive(Debug, Default, PartialEq, Eq)]
enum Inside {
    /// Between tags.
    #[default]
    Data,
    /// Inside `<!--`, up to `-->` or `--!>`.
    Comment,
    /// Inside a raw-text element other than `script`, up to its end tag.
    RawText(String),
    /// Inside a `script`, whose text the tokenizer reads with escape states
    /// of its own.
    Script(Escape),
}

/// The HTML tokenizer's escape state inside a `script`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Escape {
    /// Script data: `</script` ends the element, and `<!--` escapes.
    Plain,
    /// After `<!--`: `</script` still ends the element, `-->` goes back to
    /// plain, and `<script` escapes twice.
    Escaped,
    /// After `<!--` and then `<script`: `</script` goes back to escaped and
    /// `-->` back to plain. Nothing here ends the element.
    Double,
}

/// What the raw HTML of the group being read leaves open.
#[derive(Debug, Default)]
pub(super) struct Balance {
    /// The names of the elements opened and not yet closed, innermost last.
    open: Vec<String>,
    /// How many of [`Self::open`] are `svg` or `math`. A start tag reads it,
    /// so a run of tags that never close costs linear time, not a scan of
    /// [`Self::open`] per tag.
    foreign: usize,
    /// The text not read yet: an unfinished tag, or the end of an unfinished
    /// comment or raw-text element.
    tail: String,
    inside: Inside,
    /// Whether the tokenizer can no longer tell what is open, so no cut is
    /// allowed again.
    lost: bool,
}

/// One token at a `<`, with the bytes it takes.
enum Token {
    /// The text ends before the token does.
    Incomplete,
    /// A `<` that starts no tag.
    Text,
    /// A doctype, a comment, a bogus comment or an end tag with no name.
    Other(usize),
    /// The start of a comment that does not end in the text read.
    CommentOpen,
    Start {
        name: String,
        len: usize,
        self_closing: bool,
    },
    End {
        name: String,
        len: usize,
    },
}

impl Balance {
    /// Whether the raw HTML read so far leaves nothing open.
    pub(super) fn closed(&self) -> bool {
        !self.lost && self.open.is_empty() && self.tail.is_empty() && self.inside == Inside::Data
    }

    /// Output that is not raw HTML is written. If a tag, a comment or a
    /// raw-text element is unfinished, that output becomes part of it, and
    /// the tokenizer cannot follow.
    pub(super) fn interrupt(&mut self) {
        if !self.tail.is_empty() || self.inside != Inside::Data {
            self.lost = true;
        }
    }

    /// `html`, the next piece of raw HTML, read.
    pub(super) fn feed(&mut self, html: &str) {
        if self.lost {
            return;
        }
        let mut text = std::mem::take(&mut self.tail);
        text.push_str(html);
        let at = self.read(&text);
        self.tail = text[at..].to_owned();
    }

    /// Reads `text` and returns how much of it is read. The rest is kept
    /// for the next piece.
    fn read(&mut self, text: &str) -> usize {
        let mut at = 0;
        loop {
            let rest = &text[at..];
            match &self.inside {
                Inside::Comment => {
                    let Some(end) = comment_end(rest) else {
                        return keep_last(text, at, 3);
                    };
                    at += end;
                    self.inside = Inside::Data;
                }
                Inside::RawText(name) => {
                    let Some(end) = raw_text_end(rest, name) else {
                        return keep_last(text, at, name.len() + 2);
                    };
                    at += end;
                    self.inside = Inside::Data;
                }
                Inside::Script(escape) => match script_end(rest, *escape) {
                    Ok(end) => {
                        at += end;
                        self.inside = Inside::Data;
                    }
                    Err((read, escape)) => {
                        self.inside = Inside::Script(escape);
                        return at + read;
                    }
                },
                Inside::Data => {
                    let Some(lt) = rest.find('<') else {
                        return text.len();
                    };
                    at += lt;
                    match token(&text[at..]) {
                        Token::Incomplete => return at,
                        Token::Text => at += 1,
                        Token::Other(len) => at += len,
                        Token::CommentOpen => {
                            at += 4;
                            self.inside = Inside::Comment;
                        }
                        Token::Start {
                            name,
                            len,
                            self_closing,
                        } => {
                            at += len;
                            self.started(name, self_closing);
                            if self.lost {
                                return text.len();
                            }
                        }
                        Token::End { name, len } => {
                            at += len;
                            if self.open.last() == Some(&name) {
                                self.open.pop();
                                if is_foreign(&name) {
                                    self.foreign -= 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// A start tag for `name` read.
    ///
    /// An end tag closes only the innermost open element, and only when it
    /// names it. The parser closes more than that in many cases, such as
    /// implied end tags, so this keeps a group open longer than needed, and
    /// never shorter.
    fn started(&mut self, name: String, self_closing: bool) {
        if VOID.contains(&name.as_str()) || VOID_MORE.contains(&name.as_str()) {
            return;
        }
        if self_closing && (self.foreign > 0 || is_foreign(&name)) {
            return;
        }
        if name == "plaintext" {
            self.lost = true;
            return;
        }
        if name == "script" {
            self.inside = Inside::Script(Escape::Plain);
        } else if RAW_TEXT.contains(&name.as_str()) {
            self.inside = Inside::RawText(name.clone());
        }
        if is_foreign(&name) {
            self.foreign += 1;
        }
        self.open.push(name);
    }
}

/// Whether `name` starts foreign content, where `/>` closes any element.
fn is_foreign(name: &str) -> bool {
    name == "svg" || name == "math"
}

/// The index of `at` moved back so that at most `keep` bytes of `text`
/// after it are kept, on a character boundary.
fn keep_last(text: &str, at: usize, keep: usize) -> usize {
    let mut from = text.len().saturating_sub(keep).max(at);
    while !text.is_char_boundary(from) {
        from += 1;
    }
    from
}

/// The bytes up to and including the end of a comment that `text` is
/// inside: `-->` or `--!>`.
fn comment_end(text: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(dashes) = text[from..].find("--") {
        let after = from + dashes + 2;
        if text[after..].starts_with('>') {
            return Some(after + 1);
        }
        if text[after..].starts_with("!>") {
            return Some(after + 2);
        }
        from = from + dashes + 1;
    }
    None
}

/// Where the end tag of the raw-text element `name` starts in `text`: `</`
/// and the name in any case, followed by whitespace, `/` or `>`.
fn raw_text_end(text: &str, name: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(lt) = text[from..].find("</") {
        let start = from + lt;
        let name_end = start + 2 + name.len();
        if bytes
            .get(start + 2..name_end)
            .is_some_and(|got| got.eq_ignore_ascii_case(name.as_bytes()))
        {
            match bytes.get(name_end) {
                Some(b'\t' | b'\n' | b'\x0C' | b'\r' | b' ' | b'/' | b'>') => return Some(start),
                // The text ends after the name, so the next piece decides.
                None => return None,
                _ => {}
            }
        }
        from = start + 2;
    }
    None
}

/// Where the `</script` that ends a `script` starts in `text`, read from the
/// escape state `escape` as the HTML tokenizer reads it. After `<!--` and
/// `<script`, a `</script` does not end the element, so a cut there would
/// split a script the whole document's parse keeps open.
///
/// When `text` ends first, `Err` gives how many bytes are read and the state
/// there. The rest is at most a `</script` and its next byte, kept for the
/// next piece.
fn script_end(text: &str, mut escape: Escape) -> Result<usize, (usize, Escape)> {
    let bytes = text.as_bytes();
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        let rest = &bytes[at..];
        match byte {
            b'<' => {
                if escape == Escape::Plain {
                    if b"<!--".starts_with(rest) {
                        return Err((at, escape));
                    }
                    if rest.starts_with(b"<!--") {
                        // The dashes are read again, so `<!-->` goes back
                        // to plain as the tokenizer does.
                        escape = Escape::Escaped;
                        at += 2;
                        continue;
                    }
                }
                let closing = rest.get(1) == Some(&b'/');
                match script_tag(rest, if closing { 2 } else { 1 }) {
                    None => return Err((at, escape)),
                    Some(false) => {}
                    Some(true) => match (closing, escape) {
                        (true, Escape::Plain | Escape::Escaped) => return Ok(at),
                        (true, Escape::Double) => escape = Escape::Escaped,
                        (false, Escape::Escaped) => escape = Escape::Double,
                        (false, _) => {}
                    },
                }
                at += 1;
            }
            b'-' if escape != Escape::Plain => {
                if b"-->".starts_with(rest) {
                    return Err((at, escape));
                }
                if rest.starts_with(b"-->") {
                    escape = Escape::Plain;
                    at += 3;
                } else {
                    at += 1;
                }
            }
            _ => at += 1,
        }
    }
    Err((at, escape))
}

/// Whether `text` has the name `script` at `from`, in any case, followed by
/// whitespace, `/` or `>`. `None` when the text ends before that is known.
fn script_tag(text: &[u8], from: usize) -> Option<bool> {
    const NAME: &[u8] = b"script";
    let got = text.get(from..).unwrap_or_default();
    let len = got.len().min(NAME.len());
    if !got[..len].eq_ignore_ascii_case(&NAME[..len]) {
        return Some(false);
    }
    got.get(NAME.len())
        .map(|byte| matches!(byte, b'\t' | b'\n' | b'\x0C' | b'\r' | b' ' | b'/' | b'>'))
}

/// The token `text` starts with. It starts with `<`.
fn token(text: &str) -> Token {
    let bytes = text.as_bytes();
    let Some(&next) = bytes.get(1) else {
        return Token::Incomplete;
    };
    match next {
        b'!' => {
            if let Some(after) = text.strip_prefix("<!--") {
                if after.starts_with('>') {
                    return Token::Other(5);
                }
                if after.starts_with("->") {
                    return Token::Other(6);
                }
                if after.is_empty() || after == "-" {
                    return Token::Incomplete;
                }
                return comment_end(after).map_or(Token::CommentOpen, |end| Token::Other(4 + end));
            }
            if "<!--".starts_with(text) {
                return Token::Incomplete;
            }
            to_close(text)
        }
        b'?' => to_close(text),
        b'/' => match bytes.get(2) {
            None => Token::Incomplete,
            Some(b'>') => Token::Other(3),
            Some(byte) if byte.is_ascii_alphabetic() => match tag(text, 2) {
                Some((name, len, _)) => Token::End { name, len },
                None => Token::Incomplete,
            },
            Some(_) => to_close(text),
        },
        byte if byte.is_ascii_alphabetic() => match tag(text, 1) {
            Some((name, len, self_closing)) => Token::Start {
                name,
                len,
                self_closing,
            },
            None => Token::Incomplete,
        },
        _ => Token::Text,
    }
}

/// A token that ends at the first `>`, such as a doctype.
fn to_close(text: &str) -> Token {
    text.find('>')
        .map_or(Token::Incomplete, |end| Token::Other(end + 1))
}

/// The tag whose name starts at `from`: its name in lower case, its length
/// up to and including its `>`, and whether it ends with `/>`. `None` when
/// the text ends first.
///
/// A quote starts an attribute value only after `=`, as the tokenizer reads
/// it, and a `>` inside the value does not end the tag.
fn tag(text: &str, from: usize) -> Option<(String, usize, bool)> {
    let bytes = text.as_bytes();
    let mut at = from;
    while at < bytes.len() && !matches!(bytes[at], b'\t' | b'\n' | b'\x0C' | b' ' | b'/' | b'>') {
        at += 1;
    }
    let name = text[from..at].to_ascii_lowercase();
    let mut last = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'>' => return Some((name, at + 1, last == b'/')),
            b'=' => {
                at += 1;
                while at < bytes.len() && matches!(bytes[at], b'\t' | b'\n' | b'\x0C' | b' ') {
                    at += 1;
                }
                if let Some(&quote @ (b'"' | b'\'')) = bytes.get(at) {
                    let close = text[at + 1..].find(char::from(quote))?;
                    at += close + 2;
                    last = quote;
                    continue;
                }
                last = b'=';
                continue;
            }
            byte => last = byte,
        }
        at += 1;
    }
    None
}

/// How each group's tree is written: up to the first one that writes
/// anything, as `toHtml` writes a whole document, and each later one behind
/// the first doctype.
#[derive(Clone, Default)]
pub(super) struct Written {
    /// Whether a group has written any HTML.
    started: bool,
    /// The first doctype a group's source starts with, after whitespace and
    /// comments, before any group wrote HTML, or nothing.
    doctype: String,
}

impl Written {
    /// `tree`'s HTML source, rebuilt when `rebuild` says so, with the `code`
    /// results in `trusted` written where their tokens are.
    ///
    /// # Errors
    ///
    /// [`bounded`]'s, naming `member`.
    pub(super) fn group(
        &mut self,
        tree: &Parsed,
        rebuild: bool,
        trusted: Trusted,
        member: &str,
    ) -> Result<String, Fault> {
        let out = source_with(tree, rebuild);
        let first = !self.started;
        if first && self.doctype.is_empty() {
            leading_doctype(&out).clone_into(&mut self.doctype);
        }
        let html = if rebuild {
            let head = (!first).then_some(self.doctype.as_str());
            trusted.spliced(settled(&out, head, member)?)
        } else {
            trusted.spliced(out)
        };
        // A group the rebuild left empty, such as a lone comment, wrote
        // nothing, so the next group still starts the page. The whole
        // document's parse drops the whitespace that follows such a group.
        self.started |= !html.is_empty();
        Ok(html)
    }
}

/// The doctype `source` starts with, after whitespace and comments, which is
/// the one that sets the parser's quirks mode. Empty when there is none.
fn leading_doctype(source: &str) -> &str {
    let mut rest = source;
    loop {
        rest = rest.trim_start_matches(['\t', '\n', '\u{C}', '\r', ' ']);
        if let Some(comment) = rest.strip_prefix("<!--") {
            let Some(end) = comment_end(comment) else {
                return "";
            };
            rest = &comment[end..];
            continue;
        }
        let doctype = rest
            .get(..9)
            .is_some_and(|start| start.eq_ignore_ascii_case("<!doctype"));
        if !doctype {
            return "";
        }
        return rest.find('>').map_or("", |end| &rest[..=end]);
    }
}

/// `markdown` written one block group at a time: each group's HTML, in
/// order, with the footnotes last.
///
/// # Errors
///
/// [`super::rendered_with`]'s.
#[cfg(test)]
pub(super) fn grouped_with(
    markdown: &str,
    member: &'static str,
    hooks: &mut dyn Callbacks,
) -> Result<Vec<String>, Fault> {
    super::segments::grouped_in(
        markdown.as_bytes(),
        crate::byte_source::READ_CHUNK,
        member,
        hooks,
    )
}
