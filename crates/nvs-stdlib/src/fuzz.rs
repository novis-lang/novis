//! The fuzzing seam: thin wrappers over the parsers that read outside bytes and
//! are private to this crate, for the targets under `fuzz/fuzz_targets/`
//! (`rule:testing/the-deep-lane`).
//!
//! **One hidden module rather than a wider visibility on each parser.** The
//! multipart, cookie, reply-head, XML and decompression readers stay
//! `pub(crate)` where they live, so nothing outside this crate builds on them,
//! and the fuzz crate reaches all of them through the one door here. Each
//! wrapper takes plain bytes and answers plain data, so a target needs no
//! knowledge of `Ctx`, `Fault` or a module's internal types. Nothing here is
//! called on a request path, and it spends nothing until a target calls it.

use nvs_runtime::{Inbound, RequestBody, Scheme};

/// One multipart body read to its end: the file parts it yielded and the
/// fields it buffered, or the refusal.
#[derive(Debug, PartialEq, Eq)]
pub struct Form {
    /// Each file part's field name, filename, declared type and bytes, in order.
    pub files: Vec<FilePart>,
    /// Each field part's name and value, in arrival order.
    pub fields: Vec<(Vec<u8>, Vec<u8>)>,
}

/// One file part of a [`Form`].
#[derive(Debug, PartialEq, Eq)]
pub struct FilePart {
    /// The `name` its `Content-Disposition` gave.
    pub name: Vec<u8>,
    /// The `filename` its `Content-Disposition` gave.
    pub filename: Vec<u8>,
    /// Its `Content-Type`, where it declared one.
    pub content_type: Option<Vec<u8>>,
    /// Every byte of its body.
    pub bytes: Vec<u8>,
}

/// A body that hands out `bytes` in pieces of at most `chunk` octets.
struct Pieces<'a> {
    bytes: &'a [u8],
    chunk: usize,
}

impl RequestBody for Pieces<'_> {
    fn next_chunk(&mut self) -> Result<Option<&[u8]>, Box<str>> {
        if self.bytes.is_empty() {
            return Ok(None);
        }
        let (head, tail) = self.bytes.split_at(self.chunk.min(self.bytes.len()));
        self.bytes = tail;
        Ok(Some(head))
    }
}

/// The `boundary` a multipart `Content-Type` value declares, or the refusal.
///
/// # Errors
///
/// What `crate::multipart`'s own reader refuses.
pub fn multipart_boundary(content_type: &[u8]) -> Result<Vec<u8>, Box<str>> {
    crate::multipart::boundary_of(content_type)
}

/// `body` read as a multipart body under `boundary`, arriving in pieces of at
/// most `chunk` octets, with form fields capped at `cap` bytes.
///
/// Every file part's bytes are read, so the whole body is walked once.
///
/// # Errors
///
/// What `crate::multipart`'s reader refuses.
pub fn multipart(boundary: &[u8], body: &[u8], chunk: usize, cap: usize) -> Result<Form, Box<str>> {
    let mut wire = Pieces {
        bytes: body,
        chunk: chunk.max(1),
    };
    let mut parse = crate::multipart::Multipart::new(
        boundary,
        cap,
        nvs_config::defaults::LIMITS_MAX_MULTIPART_PARTS,
    );
    let mut files = Vec::new();
    while let Some(head) = parse.next_part(&mut wire)? {
        let mut bytes = Vec::new();
        while let Some(piece) = parse.next_chunk(&mut wire)? {
            bytes.extend_from_slice(piece);
        }
        files.push(FilePart {
            name: head.name,
            filename: head.filename,
            content_type: head.content_type,
            bytes,
        });
    }
    Ok(Form {
        files,
        fields: parse.fields().to_vec(),
    })
}

/// The cookie `name` a request carrying `cookie` as its one `Cookie` field
/// sees, over HTTPS where `https` is set.
#[must_use]
pub fn request_cookie(cookie: &[u8], name: &[u8], https: bool) -> Option<Vec<u8>> {
    let mut inbound = Inbound::new("GET", "/", "");
    inbound.push_header("cookie", cookie);
    let scheme = if https { Scheme::Https } else { Scheme::Http };
    inbound.set_peer(None, scheme);
    crate::request::cookie_of(&inbound, name).map(<[u8]>::to_vec)
}

/// The name and value one `Set-Cookie` line sets, as `Core\Test` reads a
/// response, or `None` where it sets nothing.
#[must_use]
pub fn set_cookie_pair(line: &[u8]) -> Option<(&[u8], &[u8])> {
    crate::test::cookie_pair(line)
}

/// How a reply head delimits the body under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framing {
    /// `Transfer-Encoding` ending in `chunked`.
    Chunked,
    /// A `Content-Length` of this many octets.
    Sized(u64),
    /// The body ends with the connection.
    UntilClose,
}

/// An outbound call's reply head, as the transport reads it.
#[derive(Debug)]
pub struct ReplyHead {
    /// The status code.
    pub status: i64,
    /// Each header as its lower-cased name and its trimmed value, in order.
    pub headers: Vec<(String, String)>,
    /// Where the body under the head ends.
    pub framing: Framing,
}

/// `head` read as an outbound call's reply head, or `None` where it is refused.
#[must_use]
pub fn reply_head(head: &[u8]) -> Option<ReplyHead> {
    crate::http::transport::reply_head(head)
}

/// `document` parsed by `Core\Xml`'s strict reader, or the sentence saying what
/// is not well-formed about it.
///
/// # Errors
///
/// A document that is not well-formed.
pub fn xml(document: &str) -> Result<(), String> {
    crate::xml::parse(document).map(drop)
}

/// How many codecs `Core\Compress` decodes, which [`decompress`] numbers from 0.
pub const CODECS: u8 = 5;

/// `data` decoded under codec number `codec` (modulo [`CODECS`]), refused past
/// `bytes` octets of output or `ratio` octets per input octet.
#[must_use]
pub fn decompress(codec: u8, data: &[u8], bytes: u64, ratio: u64) -> Option<Vec<u8>> {
    use crate::compress::{Bound, Codec, decompress_within};
    let codec = match codec % CODECS {
        0 => Codec::Gzip,
        1 => Codec::Zlib,
        2 => Codec::Deflate,
        3 => Codec::Brotli,
        _ => Codec::Zstd,
    };
    decompress_within(codec, data, Bound { bytes, ratio }, "fuzz").ok()
}

/// `text` rendered to HTML as `Core\Markdown::toHtml` renders it.
///
/// # Panics
///
/// Never outside a request: only a request's memory limit stops a render.
#[must_use]
pub fn markdown(text: &str) -> String {
    crate::markdown::rendered(text, r"Core\Markdown::toHtml").expect("no memory limit")
}

/// `html` written as Markdown, as `Core\Markdown::fromHtml` writes it.
///
/// # Panics
///
/// Never outside a request: only a request's memory limit stops it.
#[must_use]
pub fn from_html(html: &str) -> String {
    crate::markdown::from_html_text(html).expect("no memory limit")
}

/// Whether `text` holds raw HTML, which `Core\Markdown::toHtml` passes
/// through the sanitizer and `Core\Markdown::fromHtml` may write another way.
#[must_use]
pub fn markdown_has_raw_html(text: &str) -> bool {
    crate::markdown::has_raw_html(text)
}

/// How many bytes of output a Markdown document may add beyond its own
/// length (`rule:core-classes/markdown-cost`).
pub const MARKDOWN_ADDED_OUTPUT: usize = crate::markdown::ADDED_OUTPUT_SLACK;

/// `text` rendered as `Core\Markdown::toHtmlEach` renders a file read
/// `piece` bytes at a time: one HTML string per block group.
///
/// # Panics
///
/// Never outside a request: `text` is UTF-8, and only a request's memory
/// limit stops a render.
#[must_use]
pub fn markdown_groups(text: &str, piece: usize) -> Vec<String> {
    crate::markdown::grouped_in(text.as_bytes(), piece).expect("UTF-8 and no memory limit")
}

/// `text` pushed into a `Core\Markdown\Live` `piece` bytes at a time: each
/// push's finished part and open part, and what `finish()` returns.
///
/// # Panics
///
/// Never outside a request: only a request's memory limit stops a render.
#[must_use]
pub fn markdown_live(text: &str, piece: usize) -> (Vec<(String, String)>, String) {
    crate::markdown::live_in(text, piece).expect("no memory limit")
}

/// Whether `Core\Markdown` writes `url` into a link or an image: the
/// sanitizer's check over the URL as it is and over it percent-decoded.
#[must_use]
pub fn markdown_url_allowed(url: &str) -> bool {
    crate::markdown::url_allowed(url)
}

/// `html` sanitized, with each element's attributes in order of name, each run
/// of CSS white space (spaces, tabs and line breaks) as one space, and none
/// between two tags. A `<br>` at either end of an inline element is moved out
/// of it, and an inline element other than a link left empty is dropped,
/// since a browser draws both the same. The space at either end of a line
/// goes, as CSS drops it, also where inline tags stand between it and the
/// `<br>` or block tag. Two renderings that differ only in how
/// they lay out their whitespace, attributes and breaks give the same string.
/// The sanitized answer is the one
/// [`html_sanitized`] gives, which a second parse builds again, so an `li`
/// that foster parenting moved into another is compared where it ends up.
#[must_use]
pub fn html_meaning(html: &str) -> String {
    use crate::html::{parse_in_page, rebuilt, source};
    let mut tree = rebuilt(&parse_in_page(&html_sanitized(html)));
    breaks_hoisted(&mut tree);
    let mut work = vec![&mut tree];
    while let Some(node) = work.pop() {
        node.attributes.sort();
        work.extend(node.children.iter_mut());
    }
    let sanitized = source(&tree);
    let mut out = String::with_capacity(sanitized.len());
    for ch in sanitized.chars() {
        let ch = if matches!(ch, '\t' | '\n' | '\r') {
            ' '
        } else {
            ch
        };
        if !(ch == ' ' && out.ends_with(' ')) {
            out.push(ch);
        }
    }
    line_ends_trimmed(&out.trim().replace("> <", "><"))
}

/// The name of the tag `piece` is, or `None` when it is text.
fn tag_name(piece: &str) -> Option<&str> {
    let inner = piece.strip_prefix('<')?.trim_start_matches('/');
    let end = inner
        .find(|ch: char| !ch.is_ascii_alphanumeric())
        .unwrap_or(inner.len());
    Some(&inner[..end])
}

/// The elements whose tags start or end a line. `pre` is not one, because a
/// browser keeps the white space in it.
const LINE_EDGES: [&str; 21] = [
    "br", "p", "div", "li", "ul", "ol", "blockquote", "h1", "h2", "h3", "h4", "h5", "h6", "table",
    "tr", "td", "th", "dl", "dt", "dd", "hr",
];

/// Whether `pieces`, walked in their order, reach the edge of a line across
/// the tags of inline elements alone.
fn reaches_break<'a>(mut pieces: impl Iterator<Item = &'a &'a str>) -> bool {
    pieces
        .find(|piece| tag_name(piece).is_none_or(|name| !INLINE.contains(&name)))
        .and_then(|piece| tag_name(piece))
        .is_some_and(|name| LINE_EDGES.contains(&name))
}

/// `html` with the space at either end of a line removed. A line ends at a
/// `<br>` or a tag in [`LINE_EDGES`], and CSS drops the white space at either
/// end of a line, also where an inline element's tag stands between the space
/// and that tag. `html` is
/// [`source`](crate::html::source)'s output with each run of white space one
/// space, so a `<` outside an attribute's quotes always starts a tag.
fn line_ends_trimmed(html: &str) -> String {
    // Each piece is a tag or the text between two tags.
    let mut pieces: Vec<&str> = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    for (at, ch) in html.char_indices() {
        match ch {
            '<' if !quoted && at > start => {
                pieces.push(&html[start..at]);
                start = at;
            }
            '"' if html[start..].starts_with('<') => quoted = !quoted,
            '>' if !quoted && html[start..].starts_with('<') => {
                pieces.push(&html[start..=at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    if start < html.len() {
        pieces.push(&html[start..]);
    }
    let mut out = String::with_capacity(html.len());
    for (at, piece) in pieces.iter().enumerate() {
        if piece.starts_with('<') {
            out.push_str(piece);
            continue;
        }
        let mut text: &str = piece;
        if reaches_break(pieces[..at].iter().rev()) {
            text = text.trim_start_matches(' ');
        }
        if reaches_break(pieces[at + 1..].iter()) {
            text = text.trim_end_matches(' ');
        }
        out.push_str(text);
    }
    out
}

/// The inline elements [`breaks_hoisted`] moves a `<br>` out of.
const INLINE: [&str; 14] = [
    "a", "em", "i", "strong", "b", "del", "s", "sup", "sub", "kbd", "mark", "ins", "u", "span",
];

/// Moves each `<br>` at either end of an inline element in `node` out of the
/// element, children first, and drops an inline element other than a link
/// left empty. The tree is at most 512 elements deep, which bounds the
/// recursion.
fn breaks_hoisted(node: &mut crate::xml::Parsed) {
    use crate::xml::Kind;
    let br = |node: &crate::xml::Parsed| node.kind == Kind::Element && node.name == "br";
    for child in &mut node.children {
        breaks_hoisted(child);
    }
    let mut out = Vec::with_capacity(node.children.len());
    for mut child in std::mem::take(&mut node.children) {
        if child.kind != Kind::Element || !INLINE.contains(&child.name.as_str()) {
            out.push(child);
            continue;
        }
        let lead = child.children.iter().take_while(|node| br(node)).count();
        out.extend(child.children.drain(..lead));
        let trail = child.children.iter().rev().take_while(|node| br(node)).count();
        let after: Vec<_> = child.children.drain(child.children.len() - trail..).collect();
        if !child.children.is_empty() || child.name == "a" {
            out.push(child);
        }
        out.extend(after);
    }
    node.children = out;
}

/// [`markdown`]'s HTML, and the work it cost: the engine's steps
/// (`pulldown_cmark::steps`) plus one per byte of HTML written. The count is
/// `None` in a build without debug assertions, which counts nothing.
#[must_use]
pub fn markdown_counting_steps(text: &str) -> (String, Option<u64>) {
    let before = pulldown_cmark::steps::taken();
    let html = markdown(text);
    let steps = pulldown_cmark::steps::taken()
        .zip(before)
        .map(|(after, before)| after - before + html.len() as u64);
    (html, steps)
}

/// `html` parsed and rebuilt as `Core\Html::sanitize` does it, and written out.
#[must_use]
pub fn html_sanitized(html: &str) -> String {
    crate::html::settled(html, None, "html_sanitized").expect("no memory limit outside a request")
}

/// `html` parsed and written out with every text node removed, so only its
/// tags and their attributes are left. A search of it finds markup and never
/// the text a document shows. An attribute's value is text too, and the
/// serializer writes its `<` and `>` as they are, so they are written as `&lt;`
/// and `&gt;` here: `alt="<b>"` holds no `<b>` to find.
#[must_use]
pub fn html_tags(html: &str) -> String {
    use crate::html::{parse, source};
    use crate::xml::Kind;
    let mut tree = parse(html);
    let mut work = vec![&mut tree];
    while let Some(node) = work.pop() {
        node.children.retain(|child| child.kind != Kind::Text);
        for (_, value) in &mut node.attributes {
            *value = value.replace('<', "\u{1}lt;").replace('>', "\u{1}gt;");
        }
        work.extend(node.children.iter_mut());
    }
    source(&tree).replace('\u{1}', "&")
}
