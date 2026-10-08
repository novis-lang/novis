//! LDAPv3's messages as this client sends and reads them: bind, unbind, search, the extended operation, controls, and the filter, encoded straight to BER
//!
//! RFC 4511 § 4 is the grammar. Only the operations [`crate::Connection`]
//! performs are encoded, and an incoming message whose operation this client
//! never asked for is [`Op::Other`], which the connection refuses.
//!
//! **A [`Filter`] is encoded from its tree, with no text step.** RFC 4515's
//! string form exists for people typing filters; a client that renders one and
//! has the server parse it back is where filter injection comes from. A value
//! is an octet string inside the element its tree position names, so no value
//! can change the filter's structure whatever bytes it holds. [`Filter::to_text`]
//! renders that form for a log, from the tree, and nothing sends it.

use std::fmt;

use crate::ber::{self, Reader, Writer, tag};
use crate::error::{Error, Kind};

/// The StartTLS extended operation (RFC 4511 § 4.14).
pub const START_TLS: &str = "1.3.6.1.4.1.1466.20037";
/// The "Who am I?" extended operation (RFC 4532).
pub const WHO_AM_I: &str = "1.3.6.1.4.1.4203.1.11.3";
/// The notice of disconnection a server sends with message ID 0 (RFC 4511 § 4.4.1).
pub const NOTICE_OF_DISCONNECTION: &str = "1.3.6.1.4.1.1466.20036";
/// The simple paged results control (RFC 2696).
pub const PAGED_RESULTS: &str = "1.2.840.113556.1.4.319";

const BIND_REQUEST: u8 = 0x60;
const BIND_RESPONSE: u8 = 0x61;
const UNBIND_REQUEST: u8 = 0x42;
const SEARCH_REQUEST: u8 = 0x63;
const SEARCH_ENTRY: u8 = 0x64;
const SEARCH_DONE: u8 = 0x65;
const SEARCH_REFERENCE: u8 = 0x73;
const EXTENDED_REQUEST: u8 = 0x77;
const EXTENDED_RESPONSE: u8 = 0x78;
const CONTROLS: u8 = 0xa0;
const REFERRAL: u8 = 0xa3;

/// How far below its base a search reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The base entry alone.
    Base,
    /// The base's direct children.
    One,
    /// The base and everything below it.
    Subtree,
}

/// A search filter, as a tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filter {
    /// Every filter in the list matches.
    And(Vec<Filter>),
    /// At least one filter in the list matches.
    Or(Vec<Filter>),
    /// The filter does not match.
    Not(Box<Filter>),
    /// The attribute has this value.
    Equal(String, Vec<u8>),
    /// The attribute has a value with these parts, in order.
    Substrings {
        /// The attribute.
        attribute: String,
        /// What the value starts with.
        initial: Option<Vec<u8>>,
        /// What it contains, in order, between the start and the end.
        any: Vec<Vec<u8>>,
        /// What it ends with.
        last: Option<Vec<u8>>,
    },
    /// The attribute has a value at or above this one.
    GreaterOrEqual(String, Vec<u8>),
    /// The attribute has a value at or below this one.
    LessOrEqual(String, Vec<u8>),
    /// The attribute has any value.
    Present(String),
    /// The attribute has a value approximately equal to this one.
    Approx(String, Vec<u8>),
    /// A matching rule applied to an attribute, such as AD's in-chain rule.
    Extensible {
        /// The matching rule's OID.
        rule: Option<String>,
        /// The attribute.
        attribute: Option<String>,
        /// The value the rule compares with.
        value: Vec<u8>,
        /// Whether the entry's DN components are matched too.
        dn_attributes: bool,
    },
    /// A filter already encoded by [`Filter::encode`], written out as it is.
    /// `Core\Ldap\Filter` keeps its filter this way, so a search sends the
    /// bytes it was built into.
    Encoded(Vec<u8>),
}

impl Filter {
    /// Writes the filter's BER encoding.
    pub fn encode(&self, out: &mut Writer) {
        match self {
            Self::And(all) => out.constructed(0xa0, |set| all.iter().for_each(|f| f.encode(set))),
            Self::Or(any) => out.constructed(0xa1, |set| any.iter().for_each(|f| f.encode(set))),
            Self::Not(inner) => out.constructed(0xa2, |not| inner.encode(not)),
            Self::Equal(attribute, value) => assertion(out, 0xa3, attribute, value),
            Self::Substrings {
                attribute,
                initial,
                any,
                last,
            } => out.constructed(0xa4, |sub| {
                sub.octets(tag::OCTET_STRING, attribute.as_bytes());
                sub.constructed(tag::SEQUENCE, |parts| {
                    if let Some(initial) = initial {
                        parts.octets(0x80, initial);
                    }
                    for middle in any {
                        parts.octets(0x81, middle);
                    }
                    if let Some(last) = last {
                        parts.octets(0x82, last);
                    }
                });
            }),
            Self::GreaterOrEqual(attribute, value) => assertion(out, 0xa5, attribute, value),
            Self::LessOrEqual(attribute, value) => assertion(out, 0xa6, attribute, value),
            Self::Present(attribute) => out.octets(0x87, attribute.as_bytes()),
            Self::Approx(attribute, value) => assertion(out, 0xa8, attribute, value),
            Self::Extensible {
                rule,
                attribute,
                value,
                dn_attributes,
            } => out.constructed(0xa9, |ext| {
                if let Some(rule) = rule {
                    ext.octets(0x81, rule.as_bytes());
                }
                if let Some(attribute) = attribute {
                    ext.octets(0x82, attribute.as_bytes());
                }
                ext.octets(0x83, value);
                if *dn_attributes {
                    ext.boolean(0x84, true);
                }
            }),
            Self::Encoded(encoded) => out.raw(encoded),
        }
    }

    /// The filter's BER encoding, as [`Filter::Encoded`] keeps it.
    #[must_use]
    pub fn to_ber(&self) -> Vec<u8> {
        let mut out = Writer::new();
        self.encode(&mut out);
        out.into_bytes()
    }

    /// The filter one [`Filter::to_ber`] encoding holds, read back as a tree.
    /// An [`Filter::Encoded`] inside it comes back as the tree it encodes.
    ///
    /// # Errors
    ///
    /// [`Kind::Protocol`] for bytes that are not exactly one filter element.
    pub fn from_ber(bytes: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader::new(bytes);
        let filter = Self::from_element(reader.element()?)?;
        if !reader.is_empty() {
            return Err(bad_filter("bytes after the filter"));
        }
        Ok(filter)
    }

    fn from_element(element: ber::Tlv<'_>) -> Result<Self, Error> {
        let list = |body: &[u8]| -> Result<Vec<Self>, Error> {
            let mut reader = Reader::new(body);
            let mut out = Vec::new();
            while !reader.is_empty() {
                out.push(Self::from_element(reader.element()?)?);
            }
            Ok(out)
        };
        let pair = |body: &[u8]| -> Result<(String, Vec<u8>), Error> {
            let mut ava = Reader::new(body);
            let attribute = text(ava.expect(tag::OCTET_STRING)?)?;
            let value = ava.expect(tag::OCTET_STRING)?.to_vec();
            Ok((attribute, value))
        };
        let body = element.body;
        Ok(match element.tag {
            0xa0 => Self::And(list(body)?),
            0xa1 => Self::Or(list(body)?),
            0xa2 => {
                let mut inner = list(body)?;
                match (inner.pop(), inner.is_empty()) {
                    (Some(one), true) => Self::Not(Box::new(one)),
                    _ => return Err(bad_filter("a `not` without exactly one filter")),
                }
            }
            0xa3 => pair(body).map(|(a, v)| Self::Equal(a, v))?,
            0xa4 => {
                let mut sub = Reader::new(body);
                let attribute = text(sub.expect(tag::OCTET_STRING)?)?;
                let mut parts = Reader::new(sub.expect(tag::SEQUENCE)?);
                let (mut initial, mut any, mut last) = (None, Vec::new(), None);
                while !parts.is_empty() {
                    let part = parts.element()?;
                    match part.tag {
                        0x80 => initial = Some(part.body.to_vec()),
                        0x81 => any.push(part.body.to_vec()),
                        0x82 => last = Some(part.body.to_vec()),
                        _ => return Err(bad_filter("a substring part with an unknown tag")),
                    }
                }
                Self::Substrings {
                    attribute,
                    initial,
                    any,
                    last,
                }
            }
            0xa5 => pair(body).map(|(a, v)| Self::GreaterOrEqual(a, v))?,
            0xa6 => pair(body).map(|(a, v)| Self::LessOrEqual(a, v))?,
            0x87 => Self::Present(text(body)?),
            0xa8 => pair(body).map(|(a, v)| Self::Approx(a, v))?,
            0xa9 => {
                let mut ext = Reader::new(body);
                let rule = ext.optional(0x81)?.map(text).transpose()?;
                let attribute = ext.optional(0x82)?.map(text).transpose()?;
                let value = ext.expect(0x83)?.to_vec();
                let dn_attributes = ext.optional(0x84)?.map(ber::boolean).transpose()?;
                Self::Extensible {
                    rule,
                    attribute,
                    value,
                    dn_attributes: dn_attributes.unwrap_or(false),
                }
            }
            _ => return Err(bad_filter("an element with an unknown tag")),
        })
    }

    /// The filter in RFC 4515's text form, such as `(&(objectClass=user)(cn=a\2a))`.
    /// A value's `*`, `(`, `)`, `\`, control characters and bytes that are not
    /// UTF-8 are written as `\` and two hex digits, so the text is one line and
    /// reads back as the same filter.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        self.write_text(&mut out);
        out
    }

    /// The filter RFC 4515's text form writes, the way [`Filter::to_text`]
    /// renders it: `parse` of that text is the same filter. Every attribute
    /// and matching rule is checked by [`is_attribute_description`], and a
    /// value's `\` and two hex digits are the byte they name.
    ///
    /// # Errors
    ///
    /// A [`TextError`] naming the character the text stops being a filter at.
    pub fn parse(text: &str) -> Result<Self, TextError> {
        let mut parser = TextParser { text, at: 0 };
        let filter = parser.filter()?;
        if parser.at < text.len() {
            return Err(parser.error("there is text after the filter's last `)`"));
        }
        Ok(filter)
    }

    fn write_text(&self, out: &mut String) {
        let each = |open: &str, all: &[Self], out: &mut String| {
            out.push_str(open);
            all.iter().for_each(|f| f.write_text(out));
            out.push(')');
        };
        match self {
            Self::And(all) => each("(&", all, out),
            Self::Or(any) => each("(|", any, out),
            Self::Not(inner) => each("(!", std::slice::from_ref(&**inner), out),
            Self::Equal(attribute, value) => text_assertion(out, attribute, "=", value),
            Self::Substrings {
                attribute,
                initial,
                any,
                last,
            } => {
                out.push('(');
                out.push_str(attribute);
                out.push('=');
                if let Some(initial) = initial {
                    escaped(out, initial);
                }
                out.push('*');
                for middle in any {
                    escaped(out, middle);
                    out.push('*');
                }
                if let Some(last) = last {
                    escaped(out, last);
                }
                out.push(')');
            }
            Self::GreaterOrEqual(attribute, value) => text_assertion(out, attribute, ">=", value),
            Self::LessOrEqual(attribute, value) => text_assertion(out, attribute, "<=", value),
            Self::Present(attribute) => {
                out.push('(');
                out.push_str(attribute);
                out.push_str("=*)");
            }
            Self::Approx(attribute, value) => text_assertion(out, attribute, "~=", value),
            Self::Extensible {
                rule,
                attribute,
                value,
                dn_attributes,
            } => {
                out.push('(');
                if let Some(attribute) = attribute {
                    out.push_str(attribute);
                }
                if *dn_attributes {
                    out.push_str(":dn");
                }
                if let Some(rule) = rule {
                    out.push(':');
                    out.push_str(rule);
                }
                out.push_str(":=");
                escaped(out, value);
                out.push(')');
            }
            // Only bytes [`Filter::to_ber`] wrote reach here, and those read
            // back. Bytes that do not are written as `(?)`, which no filter is.
            Self::Encoded(encoded) => match Self::from_ber(encoded) {
                Ok(filter) => filter.write_text(out),
                Err(_) => out.push_str("(?)"),
            },
        }
    }
}

/// Why [`Filter::parse`] could not read a filter, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextError {
    /// The character the text stops being a filter at, counted from 1.
    pub position: usize,
    /// What is wrong there.
    pub reason: &'static str,
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at character {}, {}", self.position, self.reason)
    }
}

impl std::error::Error for TextError {}

/// RFC 4515 § 3's grammar, read left to right with no backtracking.
struct TextParser<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> TextParser<'a> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn error(&self, reason: &'static str) -> TextError {
        self.error_at(self.at, reason)
    }

    fn error_at(&self, at: usize, reason: &'static str) -> TextError {
        let before = self.text.get(..at).unwrap_or(self.text);
        TextError {
            position: before.chars().count() + 1,
            reason,
        }
    }

    fn expect(&mut self, byte: u8, reason: &'static str) -> Result<(), TextError> {
        if self.peek() == Some(byte) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.error(reason))
        }
    }

    /// `filter = "(" filtercomp ")"`.
    fn filter(&mut self) -> Result<Filter, TextError> {
        self.expect(b'(', "a filter starts with `(`")?;
        let filter = match self.peek() {
            Some(b'&') => {
                self.at += 1;
                Filter::And(self.list()?)
            }
            Some(b'|') => {
                self.at += 1;
                Filter::Or(self.list()?)
            }
            Some(b'!') => {
                self.at += 1;
                Filter::Not(Box::new(self.filter()?))
            }
            _ => self.item()?,
        };
        self.expect(b')', "a filter ends with `)`")?;
        Ok(filter)
    }

    /// `filterlist = 1*filter`.
    fn list(&mut self) -> Result<Vec<Filter>, TextError> {
        let mut all = Vec::new();
        while self.peek() == Some(b'(') {
            all.push(self.filter()?);
        }
        if all.is_empty() {
            return Err(self.error("`&` and `|` are followed by one filter or more"));
        }
        Ok(all)
    }

    /// Everything up to the next character that ends a name.
    fn word(&mut self) -> &'a str {
        let start = self.at;
        while let Some(b) = self.peek() {
            if matches!(b, b'=' | b'~' | b'<' | b'>' | b':' | b'(' | b')' | b'*') {
                break;
            }
            self.at += 1;
        }
        &self.text[start..self.at]
    }

    fn attribute(&self, at: usize, name: &str) -> Result<String, TextError> {
        if is_attribute_description(name) {
            Ok(name.to_owned())
        } else {
            Err(self.error_at(at, "this is not an attribute name"))
        }
    }

    /// `item = simple / present / substring / extensible`.
    fn item(&mut self) -> Result<Filter, TextError> {
        let start = self.at;
        let name = self.word();
        if self.peek() == Some(b':') {
            let attribute = if name.is_empty() {
                None
            } else {
                Some(self.attribute(start, name)?)
            };
            return self.extensible(attribute);
        }
        let attribute = self.attribute(start, name)?;
        let make: fn(String, Vec<u8>) -> Filter = match self.peek() {
            Some(b'=') => {
                self.at += 1;
                return self.equality(attribute);
            }
            Some(b'~') => Filter::Approx,
            Some(b'>') => Filter::GreaterOrEqual,
            Some(b'<') => Filter::LessOrEqual,
            _ => {
                return Err(self.error("an attribute is followed by `=`, `~=`, `>=`, `<=` or `:`"));
            }
        };
        self.at += 1;
        self.expect(b'=', "`~`, `>` and `<` are followed by `=`")?;
        Ok(make(attribute, self.whole_value()?))
    }

    /// What follows `attribute=`: a value, `*`, or parts joined by `*`.
    fn equality(&mut self, attribute: String) -> Result<Filter, TextError> {
        let mut parts = vec![self.value()?];
        while self.peek() == Some(b'*') {
            if parts.len() > 1 && parts.last().is_some_and(Vec::is_empty) {
                return Err(self.error("two `*` have nothing between them"));
            }
            self.at += 1;
            parts.push(self.value()?);
        }
        if parts.len() == 1 {
            return Ok(Filter::Equal(attribute, parts.remove(0)));
        }
        if parts.len() == 2 && parts.iter().all(Vec::is_empty) {
            return Ok(Filter::Present(attribute));
        }
        let last = parts.pop().filter(|part| !part.is_empty());
        let initial = Some(parts.remove(0)).filter(|part| !part.is_empty());
        Ok(Filter::Substrings {
            attribute,
            initial,
            any: parts,
            last,
        })
    }

    /// `extensible`, from its first `:` to the end of its value.
    fn extensible(&mut self, attribute: Option<String>) -> Result<Filter, TextError> {
        let mut dn_attributes = false;
        let mut rule = None;
        loop {
            self.expect(b':', "a matching rule is followed by `:=`")?;
            if self.peek() == Some(b'=') {
                self.at += 1;
                break;
            }
            let start = self.at;
            let word = self.word();
            if word.eq_ignore_ascii_case("dn") && !dn_attributes && rule.is_none() {
                dn_attributes = true;
            } else if rule.is_none() && !word.contains(';') && is_attribute_description(word) {
                rule = Some(word.to_owned());
            } else {
                return Err(self.error_at(start, "this is not `dn` or a matching rule"));
            }
        }
        if attribute.is_none() && rule.is_none() {
            return Err(self.error("a match with no attribute needs a matching rule"));
        }
        Ok(Filter::Extensible {
            rule,
            attribute,
            value: self.whole_value()?,
            dn_attributes,
        })
    }

    /// A value that may not hold a `*`.
    fn whole_value(&mut self) -> Result<Vec<u8>, TextError> {
        let value = self.value()?;
        if self.peek() == Some(b'*') {
            return Err(self.error("a `*` in this value is written `\\2a`"));
        }
        Ok(value)
    }

    /// `valueencoding`, up to the `)` or `*` that ends it.
    fn value(&mut self) -> Result<Vec<u8>, TextError> {
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None | Some(b')' | b'*') => return Ok(out),
                Some(b'(') => return Err(self.error("a `(` in a value is written `\\28`")),
                Some(0) => return Err(self.error("a NUL byte in a value is written `\\00`")),
                Some(b'\\') => {
                    let hex = self
                        .text
                        .get(self.at + 1..self.at + 3)
                        .filter(|hex| hex.bytes().all(|b| b.is_ascii_hexdigit()))
                        .and_then(|hex| u8::from_str_radix(hex, 16).ok());
                    let Some(byte) = hex else {
                        return Err(self.error("a `\\` in a value is followed by two hex digits"));
                    };
                    out.push(byte);
                    self.at += 3;
                }
                Some(byte) => {
                    out.push(byte);
                    self.at += 1;
                }
            }
        }
    }
}

fn bad_filter(what: &str) -> Error {
    Error::new(
        Kind::Protocol,
        format!("a filter's encoding is malformed: {what}"),
    )
}

fn text_assertion(out: &mut String, attribute: &str, operator: &str, value: &[u8]) {
    out.push('(');
    out.push_str(attribute);
    out.push_str(operator);
    escaped(out, value);
    out.push(')');
}

/// `value` as RFC 4515 § 3 writes an assertion value: UTF-8 as it is, and
/// every byte that would end or open a filter, every control character and
/// every byte that is not UTF-8 as `\` and two hex digits.
fn escaped(out: &mut String, value: &[u8]) {
    use std::fmt::Write as _;
    for chunk in value.utf8_chunks() {
        for c in chunk.valid().chars() {
            if matches!(c, '*' | '(' | ')' | '\\') || c.is_control() {
                let mut buf = [0; 4];
                for byte in c.encode_utf8(&mut buf).bytes() {
                    let _ = write!(out, "\\{byte:02x}");
                }
            } else {
                out.push(c);
            }
        }
        for byte in chunk.invalid() {
            let _ = write!(out, "\\{byte:02x}");
        }
    }
}

/// Whether `name` is an attribute description as RFC 4512 § 2.5 writes one:
/// a name (`cn`, `msDS-User-Account-Control-Computed`) or a numeric OID
/// (`2.5.4.3`), then any number of `;option`s (`cn;lang-en`). An attribute
/// name is sent as the program wrote it, so nothing else may reach the wire.
#[must_use]
pub fn is_attribute_description(name: &str) -> bool {
    let mut parts = name.split(';');
    let kind = parts.next().unwrap_or_default();
    let keystring = |text: &str| {
        let mut chars = text.chars();
        chars.next().is_some_and(|c| c.is_ascii_alphabetic())
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
    };
    let number = |text: &str| {
        !text.is_empty()
            && text.bytes().all(|b| b.is_ascii_digit())
            && (text.len() == 1 || !text.starts_with('0'))
    };
    let numeric_oid = |text: &str| text.contains('.') && text.split('.').all(number);
    (keystring(kind) || numeric_oid(kind))
        && parts.all(|option| {
            !option.is_empty()
                && option
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

fn assertion(out: &mut Writer, tag: u8, attribute: &str, value: &[u8]) {
    out.constructed(tag, |ava| {
        ava.octets(tag::OCTET_STRING, attribute.as_bytes());
        ava.octets(tag::OCTET_STRING, value);
    });
}

/// A control attached to a request or sent back with a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Control {
    /// The control's OID.
    pub oid: String,
    /// Whether the server must refuse the operation if it does not support this control.
    pub critical: bool,
    /// The control's value, which its OID defines.
    pub value: Option<Vec<u8>>,
}

impl Control {
    /// The paged results control asking for `size` entries after `cookie`.
    #[must_use]
    pub fn paged(size: u32, cookie: &[u8]) -> Self {
        let mut value = Writer::new();
        value.constructed(tag::SEQUENCE, |seq| {
            seq.integer(tag::INTEGER, i64::from(size));
            seq.octets(tag::OCTET_STRING, cookie);
        });
        Self {
            oid: PAGED_RESULTS.to_owned(),
            critical: true,
            value: Some(value.into_bytes()),
        }
    }

    /// The cookie of a paged results control a server sent back, which is
    /// empty on the last page.
    ///
    /// # Errors
    ///
    /// [`Kind::Protocol`] for a value that is not the RFC 2696 shape.
    pub fn paged_cookie(&self) -> Result<Vec<u8>, Error> {
        let value = self.value.as_deref().unwrap_or_default();
        let mut outer = Reader::new(value);
        let mut seq = Reader::new(outer.expect(tag::SEQUENCE)?);
        seq.expect(tag::INTEGER)?;
        Ok(seq.expect(tag::OCTET_STRING)?.to_vec())
    }
}

/// An `LDAPMessage` with `op` as its operation, already encoded.
#[must_use]
pub fn message(id: i32, op: &[u8], controls: &[Control]) -> Vec<u8> {
    let mut out = Writer::new();
    out.constructed(tag::SEQUENCE, |msg| {
        msg.integer(tag::INTEGER, i64::from(id));
        msg.raw(op);
        if !controls.is_empty() {
            msg.constructed(CONTROLS, |list| {
                for control in controls {
                    list.constructed(tag::SEQUENCE, |one| {
                        one.octets(tag::OCTET_STRING, control.oid.as_bytes());
                        if control.critical {
                            one.boolean(tag::BOOLEAN, true);
                        }
                        if let Some(value) = &control.value {
                            one.octets(tag::OCTET_STRING, value);
                        }
                    });
                }
            });
        }
    });
    out.into_bytes()
}

/// A simple `BindRequest`, LDAP version 3.
#[must_use]
pub fn bind_request(name: &str, password: &[u8]) -> Vec<u8> {
    let mut out = Writer::new();
    out.constructed(BIND_REQUEST, |bind| {
        bind.integer(tag::INTEGER, 3);
        bind.octets(tag::OCTET_STRING, name.as_bytes());
        bind.octets(0x80, password);
    });
    out.into_bytes()
}

/// An `UnbindRequest`.
#[must_use]
pub fn unbind_request() -> Vec<u8> {
    let mut out = Writer::new();
    out.octets(UNBIND_REQUEST, &[]);
    out.into_bytes()
}

/// An `ExtendedRequest` naming `oid`, with no value.
#[must_use]
pub fn extended_request(oid: &str) -> Vec<u8> {
    let mut out = Writer::new();
    out.constructed(EXTENDED_REQUEST, |ext| ext.octets(0x80, oid.as_bytes()));
    out.into_bytes()
}

/// What a `SearchRequest` asks for.
#[derive(Debug, Clone, Copy)]
pub struct SearchRequest<'a> {
    /// The DN the search starts from.
    pub base: &'a str,
    /// How far below the base it reaches.
    pub scope: Scope,
    /// Which entries it returns.
    pub filter: &'a Filter,
    /// The attributes each entry carries. Empty asks for every user attribute,
    /// and `1.1` alone asks for none.
    pub attributes: &'a [&'a str],
    /// The most entries one page holds.
    pub page_size: u32,
    /// The most entries the whole search returns, or 0 for the server's own limit.
    pub size_limit: u32,
    /// The most seconds the server spends, or 0 for the server's own limit.
    pub time_limit: u32,
}

/// The `SearchRequest` operation, encoded once and sent again for every page.
#[must_use]
pub fn search_request(request: &SearchRequest<'_>) -> Vec<u8> {
    let mut out = Writer::new();
    out.constructed(SEARCH_REQUEST, |search| {
        search.octets(tag::OCTET_STRING, request.base.as_bytes());
        search.integer(
            tag::ENUMERATED,
            match request.scope {
                Scope::Base => 0,
                Scope::One => 1,
                Scope::Subtree => 2,
            },
        );
        // `neverDerefAliases`: AD has no aliases, and following one is a
        // second lookup the program did not write.
        search.integer(tag::ENUMERATED, 0);
        search.integer(tag::INTEGER, i64::from(request.size_limit));
        search.integer(tag::INTEGER, i64::from(request.time_limit));
        search.boolean(tag::BOOLEAN, false);
        request.filter.encode(search);
        search.constructed(tag::SEQUENCE, |list| {
            for attribute in request.attributes {
                list.octets(tag::OCTET_STRING, attribute.as_bytes());
            }
        });
    });
    out.into_bytes()
}

/// One entry a search returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The entry's DN.
    pub dn: String,
    /// Its attributes, in the order the server sent them.
    pub attributes: Vec<Attribute>,
}

impl Entry {
    /// The values of `name`, matched without case, or `None` when the entry
    /// does not carry it.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[Vec<u8>]> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
            .map(|attribute| attribute.values.as_slice())
    }
}

/// One attribute of an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// The attribute's name as the server spelled it.
    pub name: String,
    /// Its values, as the server sent them.
    pub values: Vec<Vec<u8>>,
}

/// The `LDAPResult` every response carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LdapResult {
    /// The result code: 0 is success.
    pub code: u32,
    /// The server's diagnostic message.
    pub diagnostic: String,
    /// The referral URLs, when the code is `referral`.
    pub referrals: Vec<String>,
}

/// The operation an incoming message carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// The answer to a bind.
    BindResponse(LdapResult),
    /// One entry of a search.
    SearchEntry(Entry),
    /// A continuation reference of a search: the URLs of another server.
    SearchReference(Vec<String>),
    /// The end of a search, or of one page of it.
    SearchDone(LdapResult),
    /// The answer to an extended operation.
    Extended {
        /// The result.
        result: LdapResult,
        /// The response's OID.
        name: Option<String>,
        /// The response's value.
        value: Option<Vec<u8>>,
    },
    /// An operation this client never asks for, by its tag.
    Other(u8),
}

/// An incoming `LDAPMessage`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incoming {
    /// The ID of the request it answers, or 0 for an unsolicited notice.
    pub id: i64,
    /// What it carries.
    pub op: Op,
    /// The controls the server sent with it.
    pub controls: Vec<Control>,
}

/// Decodes one whole `LDAPMessage`, header included.
///
/// # Errors
///
/// [`Kind::Protocol`] for anything that is not a well-formed message.
pub fn decode(bytes: &[u8]) -> Result<Incoming, Error> {
    let mut outer = Reader::new(bytes);
    let mut msg = Reader::new(outer.expect(tag::SEQUENCE)?);
    let id = ber::integer(msg.expect(tag::INTEGER)?)?;
    let op = msg.element()?;
    let op = match op.tag {
        BIND_RESPONSE => Op::BindResponse(result(&mut Reader::new(op.body))?),
        SEARCH_ENTRY => Op::SearchEntry(entry(op.body)?),
        SEARCH_REFERENCE => {
            let mut urls = Reader::new(op.body);
            let mut list = Vec::new();
            while !urls.is_empty() {
                list.push(text(urls.expect(tag::OCTET_STRING)?)?);
            }
            Op::SearchReference(list)
        }
        SEARCH_DONE => Op::SearchDone(result(&mut Reader::new(op.body))?),
        EXTENDED_RESPONSE => {
            let mut body = Reader::new(op.body);
            let result = result(&mut body)?;
            let name = body.optional(0x8a)?.map(text).transpose()?;
            let value = body.optional(0x8b)?.map(<[u8]>::to_vec);
            Op::Extended {
                result,
                name,
                value,
            }
        }
        other => Op::Other(other),
    };
    let mut controls = Vec::new();
    if let Some(list) = msg.optional(CONTROLS)? {
        let mut list = Reader::new(list);
        while !list.is_empty() {
            let mut one = Reader::new(list.expect(tag::SEQUENCE)?);
            let oid = text(one.expect(tag::OCTET_STRING)?)?;
            let critical = one
                .optional(tag::BOOLEAN)?
                .map(ber::boolean)
                .transpose()?
                .unwrap_or(false);
            let value = one.optional(tag::OCTET_STRING)?.map(<[u8]>::to_vec);
            controls.push(Control {
                oid,
                critical,
                value,
            });
        }
    }
    Ok(Incoming { id, op, controls })
}

fn result(body: &mut Reader<'_>) -> Result<LdapResult, Error> {
    let code = ber::integer(body.expect(tag::ENUMERATED)?)?;
    let code = u32::try_from(code)
        .map_err(|_| Error::new(Kind::Protocol, "the server sent a negative result code"))?;
    body.expect(tag::OCTET_STRING)?;
    let diagnostic = String::from_utf8_lossy(body.expect(tag::OCTET_STRING)?).into_owned();
    let mut referrals = Vec::new();
    if let Some(list) = body.optional(REFERRAL)? {
        let mut list = Reader::new(list);
        while !list.is_empty() {
            referrals.push(text(list.expect(tag::OCTET_STRING)?)?);
        }
    }
    Ok(LdapResult {
        code,
        diagnostic,
        referrals,
    })
}

fn entry(body: &[u8]) -> Result<Entry, Error> {
    let mut body = Reader::new(body);
    let dn = text(body.expect(tag::OCTET_STRING)?)?;
    let mut list = Reader::new(body.expect(tag::SEQUENCE)?);
    let mut attributes = Vec::new();
    while !list.is_empty() {
        let mut one = Reader::new(list.expect(tag::SEQUENCE)?);
        let name = text(one.expect(tag::OCTET_STRING)?)?;
        let mut set = Reader::new(one.expect(tag::SET)?);
        let mut values = Vec::new();
        while !set.is_empty() {
            values.push(set.expect(tag::OCTET_STRING)?.to_vec());
        }
        attributes.push(Attribute { name, values });
    }
    Ok(Entry { dn, attributes })
}

/// An `LDAPString`, which RFC 4511 § 4.1.2 makes UTF-8.
fn text(bytes: &[u8]) -> Result<String, Error> {
    String::from_utf8(bytes.to_vec())
        .map_err(|_| Error::new(Kind::Protocol, "the server sent a string that is not UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filter_value_stays_inside_its_element_whatever_bytes_it_holds() {
        // `*)(uid=*` would close the filter and open another in RFC 4515's
        // text form. Encoded from the tree it is one octet string.
        let filter = Filter::And(vec![
            Filter::Equal("objectClass".into(), b"user".to_vec()),
            Filter::Equal("cn".into(), b"*)(uid=*".to_vec()),
        ]);
        let mut out = Writer::new();
        filter.encode(&mut out);
        let bytes = out.into_bytes();
        let mut outer = Reader::new(&bytes);
        let mut and = Reader::new(outer.expect(0xa0).unwrap());
        and.expect(0xa3).unwrap();
        let mut second = Reader::new(and.expect(0xa3).unwrap());
        assert_eq!(second.expect(tag::OCTET_STRING).unwrap(), b"cn");
        assert_eq!(second.expect(tag::OCTET_STRING).unwrap(), b"*)(uid=*");
        assert!(and.is_empty() && outer.is_empty());
    }

    #[test]
    fn a_bind_request_is_the_rfc_4511_shape() {
        let bytes = message(1, &bind_request("cn=a", b"pw"), &[]);
        assert_eq!(
            bytes,
            [
                0x30, 0x12, 0x02, 0x01, 0x01, 0x60, 0x0d, 0x02, 0x01, 0x03, 0x04, 0x04, b'c', b'n',
                b'=', b'a', 0x80, 0x02, b'p', b'w'
            ]
        );
    }

    #[test]
    fn a_paged_control_round_trips_its_cookie() {
        let control = Control::paged(1000, b"next");
        assert_eq!(control.paged_cookie().unwrap(), b"next");
    }

    #[test]
    fn a_search_done_with_a_paged_control_decodes() {
        let mut out = Writer::new();
        out.constructed(tag::SEQUENCE, |msg| {
            msg.integer(tag::INTEGER, 7);
            msg.constructed(SEARCH_DONE, |done| {
                done.integer(tag::ENUMERATED, 0);
                done.octets(tag::OCTET_STRING, b"");
                done.octets(tag::OCTET_STRING, b"");
            });
            msg.constructed(CONTROLS, |list| {
                let paged = Control::paged(0, b"c");
                list.constructed(tag::SEQUENCE, |one| {
                    one.octets(tag::OCTET_STRING, paged.oid.as_bytes());
                    one.octets(tag::OCTET_STRING, paged.value.as_deref().unwrap());
                });
            });
        });
        let incoming = decode(&out.into_bytes()).unwrap();
        assert_eq!(incoming.id, 7);
        assert!(matches!(
            incoming.op,
            Op::SearchDone(LdapResult { code: 0, .. })
        ));
        assert_eq!(incoming.controls[0].paged_cookie().unwrap(), b"c");
    }

    #[test]
    fn an_attribute_description_is_a_name_or_an_oid_with_options() {
        for good in [
            "cn",
            "msDS-User-Account-Control-Computed",
            "2.5.4.3",
            "cn;lang-en",
            "member;range=0-1499".split('=').next().unwrap(),
        ] {
            assert!(is_attribute_description(good), "{good}");
        }
        for bad in [
            "", "1cn", "-cn", "c n", "cn)", "cn=*", "2.5..3", "2.05.4", "2", "cn;", "cn;a b",
        ] {
            assert!(!is_attribute_description(bad), "{bad}");
        }
    }

    #[test]
    fn every_filter_reads_back_from_its_encoding_and_renders_as_rfc_4515() {
        let filter = Filter::And(vec![
            Filter::Or(vec![
                Filter::Substrings {
                    attribute: "cn".into(),
                    initial: Some(b"a*".to_vec()),
                    any: vec![b"(b)".to_vec()],
                    last: Some(b"c\\".to_vec()),
                },
                Filter::Not(Box::new(Filter::Present("mail".into()))),
            ]),
            Filter::GreaterOrEqual("uSNChanged".into(), b"10".to_vec()),
            Filter::LessOrEqual("uSNChanged".into(), b"20".to_vec()),
            Filter::Approx("sn".into(), b"Sm\xc3\xafth\n\xff".to_vec()),
            Filter::Extensible {
                rule: Some("1.2.840.113556.1.4.1941".into()),
                attribute: Some("memberOf".into()),
                value: b"CN=Staff".to_vec(),
                dn_attributes: true,
            },
        ]);
        let read = Filter::from_ber(&filter.to_ber()).unwrap();
        assert_eq!(read, filter);
        assert_eq!(
            Filter::Encoded(filter.to_ber()).to_text(),
            "(&(|(cn=a\\2a*\\28b\\29*c\\5c)(!(mail=*)))(uSNChanged>=10)(uSNChanged<=20)\
             (sn~=Sm\u{ef}th\\0a\\ff)(memberOf:dn:1.2.840.113556.1.4.1941:=CN=Staff))"
        );
        assert!(Filter::from_ber(b"\x87\x02cn\x00").is_err());
        assert_eq!(Filter::parse(&filter.to_text()).unwrap(), filter);
    }

    #[test]
    fn filter_text_parses_to_the_tree_it_writes() {
        assert_eq!(
            Filter::parse("(cn=a*b**)"),
            Err(TextError {
                position: 9,
                reason: "two `*` have nothing between them"
            })
        );
        assert_eq!(
            Filter::parse("(cn=*)").unwrap(),
            Filter::Present("cn".into())
        );
        assert_eq!(
            Filter::parse("(cn=)").unwrap(),
            Filter::Equal("cn".into(), Vec::new())
        );
        assert_eq!(
            Filter::parse("(:DN:2.5.13.5:=x)").unwrap(),
            Filter::Extensible {
                rule: Some("2.5.13.5".into()),
                attribute: None,
                value: b"x".to_vec(),
                dn_attributes: true,
            }
        );
        for (bad, position) in [
            ("", 1),
            ("cn=a", 1),
            ("(cn=a", 6),
            ("(cn=a))", 7),
            ("(&)", 3),
            ("(c n=a)", 2),
            ("(cn=a(b)", 6),
            ("(cn~=a*)", 7),
            ("(cn=\\2)", 5),
            ("(cn=\\zz)", 5),
            ("(:=a)", 4),
            ("(cn:x y:=a)", 5),
            ("(cn<a)", 5),
        ] {
            let error = Filter::parse(bad).unwrap_err();
            assert_eq!(error.position, position, "{bad}: {error}");
        }
    }

    #[test]
    fn an_encoded_filter_writes_the_bytes_it_was_built_into() {
        let filter = Filter::Equal("cn".into(), b"*)(uid=*".to_vec());
        let encoded = Filter::Encoded(filter.to_ber());
        assert_eq!(encoded.to_ber(), filter.to_ber());
    }
}
