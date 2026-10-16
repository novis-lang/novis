//! RFC 4514's distinguished name: a [`Dn`] built from parts, each value escaped as it is written, and read back from text
//!
//! `rule:core-classes/ldap-dn-is-the-launderer` is why this exists. A DN is
//! text the server parses into a path in the tree, so a value placed into one
//! by string concatenation can add a level or end the name early. [`Dn::of`]
//! and [`Dn::child`] take the value as a value, and [`Dn::to_text`] escapes
//! every character RFC 4514 § 2.4 says changes the structure.
//!
//! [`Dn::parse`] reads RFC 4514 § 3's grammar with two allowances servers and
//! people both rely on: spaces around `,`, `+` and `=` are dropped, and an
//! unescaped space at either end of a value is not part of it. It refuses the
//! `#` form of a value, the BER encoding in hex, which no directory this crate
//! is tested against writes, and an empty value. A parsed DN is held as its
//! parts, so `Dn::parse(&dn.to_text()) == dn` for every DN.
//!
//! [`Dn::is_within`] compares names and values without case, as Active
//! Directory's attributes do. It reads no schema, so `2.5.4.3` and `cn` are
//! two different names to it.

use std::fmt::{self, Write as _};

use crate::proto::{TextError, is_attribute_description};

/// One `attribute=value` pair of an [`Rdn`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ava {
    /// The attribute type, such as `CN`, as it was written.
    pub attribute: String,
    /// The value, with every escape read.
    pub value: String,
}

impl Ava {
    fn new(attribute: &str, value: &str) -> Result<Self, PartError> {
        if !is_attribute_type(attribute) {
            return Err(PartError::Attribute);
        }
        if value.is_empty() {
            return Err(PartError::EmptyValue);
        }
        Ok(Self {
            attribute: attribute.to_owned(),
            value: value.to_owned(),
        })
    }

    fn same(&self, other: &Self) -> bool {
        self.attribute.eq_ignore_ascii_case(&other.attribute)
            && self.value.to_lowercase() == other.value.to_lowercase()
    }
}

/// One level of a [`Dn`]: a pair, or several joined by `+`. Never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rdn(Vec<Ava>);

impl Rdn {
    /// The pairs, in the order they were written.
    #[must_use]
    pub fn parts(&self) -> &[Ava] {
        &self.0
    }

    /// The first pair, which is the only one outside a multi-valued RDN.
    #[must_use]
    pub fn first(&self) -> &Ava {
        &self.0[0]
    }

    /// The level as RFC 4514 text, every value escaped, as a rename sends it.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        self.write_text(&mut out);
        out
    }

    fn write_text(&self, out: &mut String) {
        for (j, ava) in self.0.iter().enumerate() {
            if j > 0 {
                out.push('+');
            }
            out.push_str(&ava.attribute);
            out.push('=');
            escaped(out, &ava.value);
        }
    }

    /// Whether both have the same pairs, in any order.
    fn same(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self
                .0
                .iter()
                .all(|ava| other.0.iter().any(|theirs| ava.same(theirs)))
    }
}

/// A distinguished name, leaf first. Never empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dn(Vec<Rdn>);

/// Why [`Dn::of`] or [`Dn::child`] could not build a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartError {
    /// The attribute is not an RFC 4512 name or numeric OID.
    Attribute,
    /// The value is empty.
    EmptyValue,
}

impl fmt::Display for PartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Attribute => "the attribute is not an attribute name",
            Self::EmptyValue => "the value is empty",
        })
    }
}

impl std::error::Error for PartError {}

impl Dn {
    /// The one-level DN `attribute=value`.
    ///
    /// # Errors
    ///
    /// [`PartError`] for an attribute that is not a name or an empty value.
    pub fn of(attribute: &str, value: &str) -> Result<Self, PartError> {
        Ok(Self(vec![Rdn(vec![Ava::new(attribute, value)?])]))
    }

    /// This DN with `attribute=value` added below it as its new first level.
    ///
    /// # Errors
    ///
    /// [`Dn::of`]'s.
    pub fn child(&self, attribute: &str, value: &str) -> Result<Self, PartError> {
        let mut rdns = Vec::with_capacity(self.0.len() + 1);
        rdns.push(Rdn(vec![Ava::new(attribute, value)?]));
        rdns.extend(self.0.iter().cloned());
        Ok(Self(rdns))
    }

    /// The DN one level up, or `None` for a DN with one level.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        (self.0.len() > 1).then(|| Self(self.0[1..].to_vec()))
    }

    /// The first level, the entry's own name.
    #[must_use]
    pub fn rdn(&self) -> &Rdn {
        &self.0[0]
    }

    /// Every level, leaf first.
    #[must_use]
    pub fn rdns(&self) -> &[Rdn] {
        &self.0
    }

    /// Whether this DN is `other` or below it.
    #[must_use]
    pub fn is_within(&self, other: &Self) -> bool {
        let Some(skip) = self.0.len().checked_sub(other.0.len()) else {
            return false;
        };
        self.0[skip..]
            .iter()
            .zip(&other.0)
            .all(|(mine, theirs)| mine.same(theirs))
    }

    /// The DN as RFC 4514 text, every value escaped.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (i, rdn) in self.0.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            rdn.write_text(&mut out);
        }
        out
    }

    /// Whether this DN names the same entry as `other`: every level the same,
    /// with attribute names matched without case.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        self.0.len() == other.0.len() && self.is_within(other)
    }

    /// Reads RFC 4514 text, as the module doc describes.
    ///
    /// # Errors
    ///
    /// [`TextError`] at the first character that is not part of a DN.
    pub fn parse(text: &str) -> Result<Self, TextError> {
        let mut parser = Parser { text, at: 0 };
        parser.skip_spaces();
        if parser.peek().is_none() {
            return Err(parser.error("a DN has at least one part, such as `DC=example`"));
        }
        let mut rdns = vec![parser.rdn()?];
        while parser.peek() == Some(b',') {
            parser.at += 1;
            rdns.push(parser.rdn()?);
        }
        Ok(Self(rdns))
    }
}

/// Whether `name` is an RFC 4514 `attributeType`: a description with no options.
fn is_attribute_type(name: &str) -> bool {
    !name.contains(';') && is_attribute_description(name)
}

/// `value` as RFC 4514 § 2.4 writes it. `=` is escaped too, as Active
/// Directory writes it, and a control character is written as hex.
fn escaped(out: &mut String, value: &str) {
    for (i, c) in value.char_indices() {
        let first = i == 0;
        let last = i + c.len_utf8() == value.len();
        match c {
            '"' | '+' | ',' | ';' | '<' | '>' | '\\' | '=' => {
                out.push('\\');
                out.push(c);
            }
            ' ' if first || last => out.push_str("\\ "),
            '#' if first => out.push_str("\\#"),
            c if c.is_control() => {
                let mut buf = [0; 4];
                for byte in c.encode_utf8(&mut buf).bytes() {
                    let _ = write!(out, "\\{byte:02x}");
                }
            }
            c => out.push(c),
        }
    }
}

/// RFC 4514 § 3's grammar, read left to right with no backtracking.
struct Parser<'a> {
    text: &'a str,
    at: usize,
}

impl Parser<'_> {
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

    fn skip_spaces(&mut self) {
        while self.peek() == Some(b' ') {
            self.at += 1;
        }
    }

    /// `relativeDistinguishedName = attributeTypeAndValue *( "+" attributeTypeAndValue )`.
    fn rdn(&mut self) -> Result<Rdn, TextError> {
        let mut parts = vec![self.ava()?];
        while self.peek() == Some(b'+') {
            self.at += 1;
            parts.push(self.ava()?);
        }
        Ok(Rdn(parts))
    }

    /// `attributeTypeAndValue = attributeType "=" attributeValue`.
    fn ava(&mut self) -> Result<Ava, TextError> {
        self.skip_spaces();
        let start = self.at;
        while let Some(b) = self.peek() {
            if matches!(b, b'=' | b',' | b'+' | b' ') {
                break;
            }
            self.at += 1;
        }
        let attribute = &self.text[start..self.at];
        if !is_attribute_type(attribute) {
            return Err(self.error_at(start, "this is not an attribute name"));
        }
        self.skip_spaces();
        if self.peek() != Some(b'=') {
            return Err(self.error("an attribute name is followed by `=`"));
        }
        self.at += 1;
        self.skip_spaces();
        Ok(Ava {
            attribute: attribute.to_owned(),
            value: self.value()?,
        })
    }

    /// `attributeValue`, up to the `,` or `+` that ends it. An unescaped
    /// space at its end is dropped; the one at its start already was.
    fn value(&mut self) -> Result<String, TextError> {
        let start = self.at;
        if self.peek() == Some(b'#') {
            return Err(self.error("a `#` at the start of a value is written `\\#`"));
        }
        let bytes = self.text.as_bytes();
        let mut out = Vec::new();
        let mut kept = 0;
        loop {
            match self.peek() {
                None | Some(b',' | b'+') => break,
                Some(b'\\') => {
                    let hex = self
                        .text
                        .get(self.at + 1..self.at + 3)
                        .filter(|hex| hex.bytes().all(|b| b.is_ascii_hexdigit()))
                        .and_then(|hex| u8::from_str_radix(hex, 16).ok());
                    if let Some(byte) = hex {
                        out.push(byte);
                        self.at += 3;
                    } else if let Some(&special) = bytes.get(self.at + 1).filter(|b| {
                        matches!(
                            b,
                            b'"' | b'+' | b',' | b';' | b'<' | b'>' | b' ' | b'#' | b'=' | b'\\'
                        )
                    }) {
                        out.push(special);
                        self.at += 2;
                    } else {
                        return Err(self.error(
                            "a `\\` in a value is followed by a special character or two hex digits",
                        ));
                    }
                    kept = out.len();
                }
                Some(b'"') => return Err(self.error("a `\"` in a value is written `\\\"`")),
                Some(b';') => return Err(self.error("a `;` in a value is written `\\;`")),
                Some(b'<') => return Err(self.error("a `<` in a value is written `\\<`")),
                Some(b'>') => return Err(self.error("a `>` in a value is written `\\>`")),
                Some(0) => return Err(self.error("a NUL byte in a value is written `\\00`")),
                Some(byte) => {
                    out.push(byte);
                    self.at += 1;
                    if byte != b' ' {
                        kept = out.len();
                    }
                }
            }
        }
        out.truncate(kept);
        if out.is_empty() {
            return Err(self.error_at(start, "the value is empty"));
        }
        String::from_utf8(out).map_err(|_| self.error_at(start, "the value is not UTF-8 text"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_special_character_is_escaped_and_reads_back() {
        let dn = Dn::parse("OU=Staff,DC=example,DC=test")
            .expect("parses")
            .child("CN", " #Lee, Ann+1 <a=b>; \"x\" \\ ")
            .expect("builds");
        assert_eq!(
            dn.to_text(),
            "CN=\\ #Lee\\, Ann\\+1 \\<a\\=b\\>\\; \\\"x\\\" \\\\\\ ,OU=Staff,DC=example,DC=test"
        );
        assert_eq!(Dn::parse(&dn.to_text()), Ok(dn));
    }

    #[test]
    fn spaces_around_separators_are_dropped() {
        assert_eq!(
            Dn::parse(" CN = Ann Lee , DC=test "),
            Dn::parse("CN=Ann Lee,DC=test")
        );
    }

    #[test]
    fn a_malformed_dn_names_where_it_stops() {
        let cases = [
            ("", 1),
            ("CN=Ann,", 8),
            ("CN=a;b", 5),
            ("1CN=Ann", 1),
            ("CN=#04", 4),
            ("CN=\\q", 4),
            ("CN", 3),
            ("CN=\\ff", 4),
        ];
        for (text, position) in cases {
            let error = Dn::parse(text).expect_err(text);
            assert_eq!(error.position, position, "{text}: {}", error.reason);
        }
    }

    #[test]
    fn within_compares_without_case() {
        let user = Dn::parse("CN=Ann,OU=Staff,DC=Example,DC=test").expect("parses");
        let base = Dn::parse("dc=example,dc=TEST").expect("parses");
        assert!(user.is_within(&base));
        assert!(base.is_within(&base));
        assert!(!base.is_within(&user));
        assert!(!user.is_within(&Dn::of("DC", "other").expect("builds")));
        assert_eq!(
            user.parent().and_then(|p| p.parent()),
            Some(Dn::parse("DC=Example,DC=test").expect("parses"))
        );
        assert_eq!(Dn::of("DC", "test").expect("builds").parent(), None);
    }
}
