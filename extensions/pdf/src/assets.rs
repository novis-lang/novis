//! The asset map: the only place a render's bytes come from (`rule:core-classes/pdf-render-has-no-io`).
//!
//! Every reference that loads a resource resolves here: against the map's names, by exact
//! string equality, or as a `data:` URI decoded in place, base64 or percent-encoded. A name may
//! end in `#page=N`, which chooses a page of a PDF asset and is not part of the name. Anything
//! else is refused with [`Error::Invalid`] naming the reference as it was written: a `file:` or
//! `http(s):` URL or any other scheme, an absolute path (`/x`, `\x`, `C:\x`, `\\host\x`), and a
//! name the map lacks. Nothing in this crate fetches, so a refused reference is refused before
//! layout, never drawn as a gap.
//!
//! [`check`] walks the whole document before layout and resolves every reference that loads
//! something, in document order, so the first refused one is the error whether or not the
//! layout would have drawn it. The references are `<img src>`, `<link rel=stylesheet href>`
//! and the `url()`s and `@import`s of the stylesheet it names, `url()`, `src()`, `image-set()`
//! strings and `@import` in `<style>` text and in `style` attributes, and `href` or
//! `xlink:href` on SVG `<image>` and `<use>`, where a `#fragment` names a part of the document
//! itself. A link's `<a href>` loads nothing and is not a reference. CSS is tokenized by
//! `cssparser`, so an escaped function name such as `u\72l(` is read as the `url(` it is.

use std::borrow::Cow;
use std::collections::BTreeMap;

use base64::Engine as _;
use cssparser::{Parser, Token};

use crate::Error;
use crate::parse::{Dom, Kind, Space};

/// The document's assets by name. Two assets of one name keep the first.
pub struct Assets<'a> {
    by_name: BTreeMap<&'a str, &'a [u8]>,
}

impl<'a> Assets<'a> {
    pub fn new(assets: &'a [crate::Asset]) -> Self {
        let mut by_name = BTreeMap::new();
        for asset in assets {
            by_name
                .entry(asset.name.as_str())
                .or_insert(asset.data.as_slice());
        }
        Assets { by_name }
    }

    /// The bytes `reference` names, or the error that refuses it.
    pub fn resolve(&self, reference: &str) -> Result<Cow<'a, [u8]>, Error> {
        let trimmed = reference.trim_matches(is_space);
        if let Some(rest) = strip_prefix_ignore_case(trimmed, "data:") {
            return data_uri(rest).map(Cow::Owned).ok_or_else(|| {
                Error::Invalid(format!(
                    "`{}` is not a valid `data:` URI",
                    abbreviated(reference)
                ))
            });
        }
        let (trimmed, _) = page_fragment(trimmed);
        if !has_scheme(trimmed)
            && !is_absolute(trimmed)
            && let Some(&data) = self.by_name.get(trimmed)
        {
            return Ok(Cow::Borrowed(data));
        }
        Err(refused(reference))
    }
}

/// `reference` without a trailing `#page=N`, and `N`. A reference without one is whole, and its
/// page is `None`.
pub fn page_fragment(reference: &str) -> (&str, Option<u64>) {
    reference
        .rsplit_once("#page=")
        .and_then(|(name, page)| Some((name, page.parse::<u64>().ok()?)))
        .map_or((reference, None), |(name, page)| (name, Some(page)))
}

/// The error for a reference that is not an asset's name or a `data:` URI.
pub fn refused(reference: &str) -> Error {
    Error::Invalid(format!(
        "`{reference}` is not in the asset map: a render reads only its assets and `data:` URIs"
    ))
}

/// HTML's ASCII whitespace, which a URL attribute's value is stripped of.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0c' | '\r')
}

fn strip_prefix_ignore_case<'s>(text: &'s str, prefix: &str) -> Option<&'s str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

/// A `data:` URI's text in an error, cut to its first 64 characters.
fn abbreviated(reference: &str) -> Cow<'_, str> {
    match reference.char_indices().nth(64) {
        Some((at, _)) => Cow::Owned(format!("{}…", &reference[..at])),
        None => Cow::Borrowed(reference),
    }
}

/// Whether `text` starts with a URL scheme of two or more characters: a letter, then letters,
/// digits, `+`, `-` or `.`, then `:`. A single letter before `:` is a drive, which
/// [`is_absolute`] refuses.
fn has_scheme(text: &str) -> bool {
    let Some(colon) = text.find(':') else {
        return false;
    };
    let scheme = &text[..colon];
    scheme.len() >= 2
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// Whether `text` is an absolute path: it starts with `/` or `\`, or with a drive letter and `:`.
fn is_absolute(text: &str) -> bool {
    let bytes = text.as_bytes();
    matches!(bytes.first(), Some(b'/' | b'\\'))
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
}

/// The bytes of a `data:` URI after its `data:`, or `None` when it has no comma or its base64
/// does not decode.
fn data_uri(rest: &str) -> Option<Vec<u8>> {
    let (header, body) = rest.split_once(',')?;
    let base64 = header
        .rsplit(';')
        .next()
        .is_some_and(|last| last.trim().eq_ignore_ascii_case("base64"));
    let bytes = percent_decode(body);
    if base64 {
        let cleaned: Vec<u8> = bytes
            .into_iter()
            .filter(|b| !b.is_ascii_whitespace())
            .collect();
        base64::engine::general_purpose::STANDARD
            .decode(&cleaned)
            .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(&cleaned))
            .ok()
    } else {
        Some(bytes)
    }
}

/// `text` with each `%XX` replaced by its byte. A `%` not followed by two hex digits is kept.
fn percent_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let (Some(hi), Some(lo)) = (
                bytes.get(i + 1).and_then(|b| (*b as char).to_digit(16)),
                bytes.get(i + 2).and_then(|b| (*b as char).to_digit(16)),
            )
        {
            out.push((hi * 16 + lo) as u8);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

/// Resolves every reference in `dom` that loads a resource, in document order, and returns the
/// first refusal.
pub fn check(dom: &Dom, assets: &Assets<'_>) -> Result<(), Error> {
    for at in dom.preorder(0) {
        let node = &dom.nodes[at];
        let Kind::Element { name, space, .. } = &node.kind else {
            continue;
        };
        match (space, name.as_str()) {
            (Space::Html, "img") => {
                if let Some(src) = node.attr("src") {
                    assets.resolve(src)?;
                }
            }
            (Space::Html, "link") if is_stylesheet(node.attr("rel")) => {
                if let Some(href) = node.attr("href") {
                    let css = assets.resolve(href)?;
                    css_references(&String::from_utf8_lossy(&css), assets)?;
                }
            }
            (Space::Html | Space::Svg, "style") => css_references(&dom.text(at), assets)?,
            (Space::Svg, "image" | "use") => {
                for attr in node.attrs().iter().filter(|attr| attr.name == "href") {
                    if !attr.value.trim_matches(is_space).starts_with('#') {
                        assets.resolve(&attr.value)?;
                    }
                }
            }
            _ => {}
        }
        if let Some(style) = node.attr("style") {
            css_references(style, assets)?;
        }
    }
    Ok(())
}

/// Whether a `rel` value has the token `stylesheet`, compared without case.
fn is_stylesheet(rel: Option<&str>) -> bool {
    rel.is_some_and(|rel| {
        rel.split(is_space)
            .any(|token| token.eq_ignore_ascii_case("stylesheet"))
    })
}

/// Resolves every URL `css` loads: `url()` in either form, the string in `src()`, the strings
/// of `image-set()`, and the string after `@import`.
///
/// `cssparser` enters at most 75 nested blocks. Past that it reads a block's tokens as if they
/// were not nested, so from there to the end of the enclosing block every string is resolved
/// as a URL: a reference buried that deep is still checked, at the cost of refusing a
/// stylesheet nested that deep that has an ordinary string in it.
pub fn css_references(css: &str, assets: &Assets<'_>) -> Result<(), Error> {
    let mut parser = Parser::new(css);
    walk_css(&mut parser, assets, Context::Plain)
}

/// What a string token means where the walk is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Context {
    Plain,
    /// Inside `url(` or `src(`: the first string is the URL.
    Url,
    /// Inside `image-set(`: every string is a URL.
    ImageSet,
}

fn walk_css(parser: &mut Parser<'_>, assets: &Assets<'_>, context: Context) -> Result<(), Error> {
    let mut context = context;
    let mut after_import = false;
    let mut first_string = true;
    loop {
        let token = match parser.next_including_whitespace_and_comments() {
            Ok(token) => token.clone(),
            Err(_) => return Ok(()),
        };
        let mut nested = None;
        match &token {
            Token::WhiteSpace(_) | Token::Comment(_) => continue,
            Token::UnquotedUrl(url) | Token::BadUrl(url) => {
                assets.resolve(url)?;
            }
            Token::QuotedString(text) => {
                let is_url = after_import
                    || context == Context::ImageSet
                    || (context == Context::Url && first_string);
                first_string = false;
                if is_url {
                    assets.resolve(text)?;
                }
            }
            Token::AtKeyword(keyword) => {
                after_import = keyword.eq_ignore_ascii_case("import");
                continue;
            }
            Token::Function(name) => {
                nested = Some(
                    if name.eq_ignore_ascii_case("url") || name.eq_ignore_ascii_case("src") {
                        Context::Url
                    } else if name.eq_ignore_ascii_case("image-set")
                        || name.eq_ignore_ascii_case("-webkit-image-set")
                    {
                        Context::ImageSet
                    } else {
                        Context::Plain
                    },
                );
            }
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                nested = Some(if context == Context::ImageSet {
                    Context::ImageSet
                } else {
                    Context::Plain
                });
            }
            _ => {}
        }
        after_import = false;
        if let Some(inner) = nested {
            let mut found = Ok(());
            let entered =
                parser.parse_nested_block(|parser| -> Result<(), cssparser::ParseError<()>> {
                    found = walk_css(parser, assets, inner);
                    Ok(())
                });
            found?;
            if entered.is_err() {
                // Too deep to enter: the block's tokens follow at this level.
                context = Context::ImageSet;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scheme_and_a_drive_are_told_apart() {
        assert!(has_scheme("file:///etc/passwd"));
        assert!(has_scheme("HTTP://example.com"));
        assert!(!has_scheme("C:\\x"));
        assert!(is_absolute("C:\\x"));
        assert!(is_absolute("\\\\host\\x"));
        assert!(is_absolute("/x"));
        assert!(!is_absolute("logo.png"));
    }

    #[test]
    fn a_data_uri_decodes_in_either_encoding() {
        assert_eq!(data_uri("text/plain;base64,aGk=").unwrap(), b"hi");
        assert_eq!(data_uri("text/plain,h%69").unwrap(), b"hi");
        assert!(data_uri("text/plain;base64,!!").is_none());
    }

    #[test]
    fn an_escaped_url_function_is_still_a_url() {
        let assets = Assets::new(&[]);
        let err = css_references("a { background: u\\72l('http://example.com/x') }", &assets)
            .unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m.contains("http://example.com/x")));
    }
}
