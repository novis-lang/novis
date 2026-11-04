//! The XML a workbook's parts are, read one token at a time from a buffer that may end inside a
//! token.
//!
//! [`next`] returns the token at the start of a buffer and its length, or `None` when the buffer
//! ends before the token does, so the caller reads more and asks again. A caller that reads a
//! part a block at a time asks with [`next_from`] and a [`Resume`], so the scan goes on where the
//! last block ended and one token longer than many blocks is scanned once. Names are compared by
//! their local part ([`local`]), so a prefix a file chose does not matter. Text and attribute
//! values are decoded by [`decode`]: the five predefined entities and character references, and
//! nothing else.
//!
//! **No DTD is read and no entity is expanded.** A `<!DOCTYPE` is a [`Error::Parse`], and so is an
//! entity other than the five XML predefines, so a part cannot name a file, a URL or a
//! billion-laughs expansion (`rule:core-classes/spreadsheet-has-no-io`). A comment and a
//! processing instruction are [`Token::Skip`]; CDATA is text.
//!
//! **Every XML part is UTF-8.** [`encoding`] judges the first [`HEAD`] bytes of a part, which
//! [`super::zip::Part`] keeps as it inflates them. A byte order mark of UTF-16 or UTF-32, a NUL,
//! or a declaration naming another encoding is an [`Error::Parse`] naming the part. A
//! declaration of US-ASCII is read, because ASCII is UTF-8. It costs one copy of 256 bytes per
//! XML part.

use crate::Error;

/// How many of a part's first bytes [`encoding`] judges: enough for its byte order mark and
/// its declaration.
pub const HEAD: usize = 256;

/// Whether the part `name` is XML, by its extension, so [`encoding`] judges it.
pub fn is_xml(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".xml") || lower.ends_with(".rels") || lower.ends_with(".vml")
}

/// Throws when the part `name`, whose first bytes are `head`, is not UTF-8.
pub fn encoding(name: &str, head: &[u8]) -> Result<(), Error> {
    let not = |what: &str| {
        Err(Error::Parse(format!(
            "The workbook part `{name}` {what}. Only UTF-8 is supported."
        )))
    };
    if head.starts_with(&[0xFF, 0xFE, 0, 0]) || head.starts_with(&[0, 0, 0xFE, 0xFF]) {
        return not("is in UTF-32");
    }
    if head.starts_with(&[0xFF, 0xFE]) || head.starts_with(&[0xFE, 0xFF]) {
        return not("is in UTF-16");
    }
    if head.contains(&0) {
        return not("is in UTF-16 or UTF-32");
    }
    let text = head.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(head);
    let Some(declaration) = text.strip_prefix(b"<?xml") else {
        return Ok(());
    };
    let Some(end) = find(declaration, 0, b"?>") else {
        return Ok(());
    };
    let Some(named) = attr(&declaration[..end], b"encoding") else {
        return Ok(());
    };
    let named = String::from_utf8_lossy(named);
    let known = ["utf-8", "utf8", "us-ascii", "ascii"];
    if known.iter().any(|ok| named.trim().eq_ignore_ascii_case(ok)) {
        return Ok(());
    }
    not(&format!("declares the encoding `{named}`"))
}

/// One token.
#[derive(Debug, PartialEq, Eq)]
pub enum Token<'a> {
    /// A start tag: its name, its attributes as written, and whether it closes itself.
    Open {
        name: &'a [u8],
        attrs: &'a [u8],
        empty: bool,
    },
    Close(&'a [u8]),
    /// Text as written, entities not yet decoded.
    Text(&'a [u8]),
    /// CDATA, which is never decoded.
    Raw(&'a [u8]),
    Skip,
}

fn find(buf: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    buf.get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| at + from)
}

/// Where the scan of a token that `buf` ended inside stopped, so the call after a refill resumes
/// there and a token longer than many read blocks is scanned once. It belongs to the token at
/// the start of the buffer it was scanned in: the caller passes the same token's start again, with
/// more bytes after it, or a fresh `Resume` for another token. [`next_from`] resets it when it
/// returns a token.
#[derive(Debug, Default, Clone, Copy)]
pub struct Resume {
    /// The bytes of the token already known not to end it.
    at: usize,
    /// The quote a start tag's scan is inside.
    quote: Option<u8>,
}

/// The token at the start of `buf` and its length, or `None` when `buf` ends inside it.
pub fn next(buf: &[u8]) -> Result<Option<(Token<'_>, usize)>, Error> {
    next_from(buf, &mut Resume::default())
}

/// [`next`], resuming the scan where `resume` says the last call on the same token stopped.
pub fn next_from<'a>(
    buf: &'a [u8],
    resume: &mut Resume,
) -> Result<Option<(Token<'a>, usize)>, Error> {
    let found = scan(buf, resume)?;
    if found.is_some() {
        *resume = Resume::default();
    }
    Ok(found)
}

/// `needle` in `buf` from `from`, or from where `resume` stopped when that is later. When it is
/// not there, `resume` keeps the bytes that cannot start it.
fn find_from(buf: &[u8], from: usize, needle: &[u8], resume: &mut Resume) -> Option<usize> {
    let found = find(buf, from.max(resume.at), needle);
    if found.is_none() {
        resume.at = from.max(buf.len().saturating_sub(needle.len() - 1));
    }
    found
}

fn scan<'a>(buf: &'a [u8], resume: &mut Resume) -> Result<Option<(Token<'a>, usize)>, Error> {
    let Some(&first) = buf.first() else {
        return Ok(None);
    };
    if first != b'<' {
        let found = buf[resume.at..].iter().position(|&b| b == b'<');
        let Some(end) = found.map(|end| end + resume.at) else {
            resume.at = buf.len();
            return Ok(None);
        };
        return Ok(Some((Token::Text(&buf[..end]), end)));
    }
    if buf.len() < 2 {
        return Ok(None);
    }
    match buf[1] {
        b'?' => Ok(find_from(buf, 2, b"?>", resume).map(|end| (Token::Skip, end + 2))),
        b'!' => {
            if buf.len() < 9 {
                return Ok(None);
            }
            if buf.starts_with(b"<!--") {
                Ok(find_from(buf, 4, b"-->", resume).map(|end| (Token::Skip, end + 3)))
            } else if buf.starts_with(b"<![CDATA[") {
                Ok(
                    find_from(buf, 9, b"]]>", resume)
                        .map(|end| (Token::Raw(&buf[9..end]), end + 3)),
                )
            } else {
                Err(Error::Parse(
                    "A part of the workbook has a document type declaration. Novis does not read these."
                        .to_owned(),
                ))
            }
        }
        b'/' => {
            let from = resume.at.max(2);
            let Some(end) = buf[from..].iter().position(|&b| b == b'>') else {
                resume.at = buf.len();
                return Ok(None);
            };
            let end = end + from;
            Ok(Some((Token::Close(trim(&buf[2..end])), end + 1)))
        }
        _ => {
            let mut quote = resume.quote;
            for (at, &b) in buf.iter().enumerate().skip(resume.at.max(1)) {
                match (quote, b) {
                    (Some(q), _) if b == q => quote = None,
                    (Some(_), _) => {}
                    (None, b'"' | b'\'') => quote = Some(b),
                    (None, b'>') => {
                        let empty = buf[at - 1] == b'/';
                        let inner = &buf[1..if empty { at - 1 } else { at }];
                        let split = inner
                            .iter()
                            .position(|b| b.is_ascii_whitespace())
                            .unwrap_or(inner.len());
                        return Ok(Some((
                            Token::Open {
                                name: &inner[..split],
                                attrs: &inner[split..],
                                empty,
                            },
                            at + 1,
                        )));
                    }
                    (None, _) => {}
                }
            }
            (resume.at, resume.quote) = (buf.len(), quote);
            Ok(None)
        }
    }
}

fn trim(bytes: &[u8]) -> &[u8] {
    bytes.trim_ascii()
}

/// The part of a name after its prefix.
pub fn local(name: &[u8]) -> &[u8] {
    name.iter()
        .rposition(|&b| b == b':')
        .map_or(name, |at| &name[at + 1..])
}

/// The raw value of the attribute whose local name is `name`.
pub fn attr<'a>(attrs: &'a [u8], name: &[u8]) -> Option<&'a [u8]> {
    let mut at = 0;
    while at < attrs.len() {
        while at < attrs.len() && attrs[at].is_ascii_whitespace() {
            at += 1;
        }
        let eq = at + attrs[at..].iter().position(|&b| b == b'=')?;
        let key = trim(&attrs[at..eq]);
        let mut open = eq + 1;
        while open < attrs.len() && attrs[open].is_ascii_whitespace() {
            open += 1;
        }
        let quote = *attrs.get(open)?;
        let close = open + 1 + attrs[open + 1..].iter().position(|&b| b == quote)?;
        if local(key) == name {
            return Some(&attrs[open + 1..close]);
        }
        at = close + 1;
    }
    None
}

/// The decoded value of the attribute whose local name is `name`.
pub fn attr_text(attrs: &[u8], name: &[u8]) -> Result<Option<String>, Error> {
    attr(attrs, name)
        .map(|raw| {
            let mut out = String::new();
            decode(raw, &mut out)?;
            Ok(out)
        })
        .transpose()
}

/// Appends `raw` to `out` with its entities and character references decoded.
pub fn decode(raw: &[u8], out: &mut String) -> Result<(), Error> {
    let mut start = 0;
    while let Some(amp) = raw[start..].iter().position(|&b| b == b'&') {
        let amp = start + amp;
        push(out, &raw[start..amp]);
        let semi = amp
            + raw[amp..]
                .iter()
                .position(|&b| b == b';')
                .ok_or_else(|| entity(&raw[amp..]))?;
        let name = &raw[amp + 1..semi];
        let c = match name {
            b"lt" => '<',
            b"gt" => '>',
            b"amp" => '&',
            b"quot" => '"',
            b"apos" => '\'',
            [b'#', b'x' | b'X', hex @ ..] => code(hex, 16)?,
            [b'#', dec @ ..] => code(dec, 10)?,
            _ => return Err(entity(name)),
        };
        out.push(c);
        start = semi + 1;
    }
    push(out, &raw[start..]);
    Ok(())
}

fn push(out: &mut String, bytes: &[u8]) {
    out.push_str(&String::from_utf8_lossy(bytes));
}

fn entity(name: &[u8]) -> Error {
    Error::Parse(format!(
        "A part of the workbook uses the entity `{}`. Novis reads only the five entities XML defines.",
        String::from_utf8_lossy(&name[..name.len().min(32)])
    ))
}

fn code(digits: &[u8], radix: u32) -> Result<char, Error> {
    std::str::from_utf8(digits)
        .ok()
        .and_then(|digits| u32::from_str_radix(digits, radix).ok())
        .and_then(char::from_u32)
        .ok_or_else(|| entity(digits))
}

/// Decodes Excel's `_xHHHH_` escapes in cell and note text, which the writer adds for characters
/// XML cannot carry (`crate::xml`'s module doc).
pub fn unescape(text: String) -> String {
    if !text.contains("_x") {
        return text;
    }
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    let mut start = 0;
    while at + 7 <= bytes.len() {
        let hex = &bytes[at + 2..at + 6];
        if bytes[at] == b'_'
            && bytes[at + 1] == b'x'
            && bytes[at + 6] == b'_'
            && hex.iter().all(u8::is_ascii_hexdigit)
            && let Some(c) = std::str::from_utf8(hex)
                .ok()
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .and_then(char::from_u32)
        {
            out.push_str(&text[start..at]);
            out.push(c);
            at += 7;
            start = at;
        } else {
            at += 1;
        }
    }
    out.push_str(&text[start..]);
    out
}
