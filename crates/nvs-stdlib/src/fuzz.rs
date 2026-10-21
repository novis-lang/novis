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
