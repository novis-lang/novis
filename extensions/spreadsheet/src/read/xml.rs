//! The XML a workbook's parts are, read one token at a time from a buffer that may end inside a
//! token.
//!
//! [`next`] returns the token at the start of a buffer and its length, or `None` when the buffer
//! ends before the token does, so the caller reads more and asks again. Names are compared by
//! their local part ([`local`]), so a prefix a file chose does not matter. Text and attribute
//! values are decoded by [`decode`]: the five predefined entities and character references, and
//! nothing else.
//!
//! **No DTD is read and no entity is expanded.** A `<!DOCTYPE` is a [`Error::Parse`], and so is an
//! entity other than the five XML predefines, so a part cannot name a file, a URL or a
//! billion-laughs expansion (`rule:core-classes/spreadsheet-has-no-io`). A comment and a
//! processing instruction are [`Token::Skip`]; CDATA is text.

use crate::Error;

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

/// The token at the start of `buf` and its length, or `None` when `buf` ends inside it.
pub fn next(buf: &[u8]) -> Result<Option<(Token<'_>, usize)>, Error> {
    let Some(&first) = buf.first() else {
        return Ok(None);
    };
    if first != b'<' {
        return Ok(buf
            .iter()
            .position(|&b| b == b'<')
            .map(|end| (Token::Text(&buf[..end]), end)));
    }
    if buf.len() < 2 {
        return Ok(None);
    }
    match buf[1] {
        b'?' => Ok(find(buf, 2, b"?>").map(|end| (Token::Skip, end + 2))),
        b'!' => {
            if buf.len() < 9 {
                return Ok(None);
            }
            if buf.starts_with(b"<!--") {
                Ok(find(buf, 4, b"-->").map(|end| (Token::Skip, end + 3)))
            } else if buf.starts_with(b"<![CDATA[") {
                Ok(find(buf, 9, b"]]>").map(|end| (Token::Raw(&buf[9..end]), end + 3)))
            } else {
                Err(Error::Parse(
                    "A part of the workbook has a document type declaration. Novis does not read these."
                        .to_owned(),
                ))
            }
        }
        b'/' => Ok(buf
            .iter()
            .position(|&b| b == b'>')
            .map(|end| (Token::Close(trim(&buf[2..end])), end + 1))),
        _ => {
            let mut quote = None;
            for (at, &b) in buf.iter().enumerate().skip(1) {
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
