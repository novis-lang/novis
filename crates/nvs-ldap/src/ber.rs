//! The subset of BER that LDAP uses: definite lengths, single-byte tags, and nothing a general ASN.1 codec carries beyond that
//!
//! RFC 4511 § 5.1 restricts LDAP's encoding to BER with three rules a general
//! decoder does not keep: only the definite form of length, OCTET STRING
//! values in the primitive form only, and BOOLEAN `TRUE` as `0xFF` when sent.
//! What is left is a tag byte, a length of at most four bytes, and a body, and
//! that is all [`Writer`] produces and all [`Reader`] accepts. An indefinite
//! length, a multi-byte tag or a length past [`MAX_LENGTH`] is
//! [`Kind::Protocol`] rather than something parsed around: none of them is a
//! message an LDAP server may send.
//!
//! This is written here rather than borrowed, against
//! `rule:core-classes/db-crate-boundary`'s "a codec is borrowed". The one
//! crate carrying LDAP's types, `rasn-ldap`, sits on `rasn`, which is ten
//! encodings (PER, OER, XER, JER and more) and `bitvec`, `chrono`, `nom`,
//! `num-bigint`, `serde_json`, an XML parser and a derive macro behind them.
//! None of that runs an async runtime, and all of it is audit surface for the
//! three hundred lines below.

use crate::error::{Error, Kind};

/// The universal tags LDAP's messages are built from.
pub mod tag {
    /// `BOOLEAN`.
    pub const BOOLEAN: u8 = 0x01;
    /// `INTEGER`.
    pub const INTEGER: u8 = 0x02;
    /// `OCTET STRING`, which is also every `LDAPString` and `LDAPDN`.
    pub const OCTET_STRING: u8 = 0x04;
    /// `ENUMERATED`.
    pub const ENUMERATED: u8 = 0x0a;
    /// `SEQUENCE` and `SEQUENCE OF`, constructed.
    pub const SEQUENCE: u8 = 0x30;
    /// `SET` and `SET OF`, constructed.
    pub const SET: u8 = 0x31;
}

/// The longest body this codec reads, in bytes.
///
/// One LDAP message is one entry or one result, and the whole message is held
/// before it is decoded, so this is the memory one read can claim. 64 MiB is
/// past any entry a directory serves — AD caps a value at a few megabytes —
/// and a length past it is a broken or hostile peer, refused before a byte of
/// the body is buffered. The request's own memory cap still applies under it.
pub const MAX_LENGTH: usize = 64 << 20;

/// Encodes one message, outermost element first.
///
/// A constructed element's length is not known until its body is written, so
/// [`Writer::constructed`] writes the body in place and inserts the header
/// before it afterwards. That moves the body once per nesting level, which is
/// a few hundred bytes for every request this crate sends.
#[derive(Debug, Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    /// An empty writer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The encoded bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    /// A primitive element with `body` as its contents.
    pub fn octets(&mut self, tag: u8, body: &[u8]) {
        self.buf.push(tag);
        push_length(&mut self.buf, body.len());
        self.buf.extend_from_slice(body);
    }

    /// An `INTEGER` or `ENUMERATED` in its shortest two's-complement form.
    pub fn integer(&mut self, tag: u8, value: i64) {
        let bytes = value.to_be_bytes();
        let mut start = 0;
        // A leading byte is redundant when it is only sign extension of the
        // next one: 0x00 before a byte under 0x80, 0xFF before one at or over it.
        while start < bytes.len() - 1 {
            let (lead, next) = (bytes[start], bytes[start + 1]);
            if (lead == 0x00 && next & 0x80 == 0) || (lead == 0xFF && next & 0x80 != 0) {
                start += 1;
            } else {
                break;
            }
        }
        self.octets(tag, &bytes[start..]);
    }

    /// A `BOOLEAN`, with `TRUE` as `0xFF` as RFC 4511 § 5.1 requires.
    pub fn boolean(&mut self, tag: u8, value: bool) {
        self.octets(tag, &[if value { 0xFF } else { 0x00 }]);
    }

    /// A constructed element whose body is what `body` writes.
    pub fn constructed(&mut self, tag: u8, body: impl FnOnce(&mut Self)) {
        let start = self.buf.len();
        body(self);
        let length = self.buf.len() - start;
        let mut header = vec![tag];
        push_length(&mut header, length);
        self.buf.splice(start..start, header);
    }

    /// Bytes already encoded elsewhere, appended as they are.
    pub fn raw(&mut self, encoded: &[u8]) {
        self.buf.extend_from_slice(encoded);
    }
}

fn push_length(buf: &mut Vec<u8>, length: usize) {
    if length < 0x80 {
        // Checked by the branch: a length under 0x80 is one byte.
        #[allow(clippy::cast_possible_truncation)]
        buf.push(length as u8);
        return;
    }
    let bytes = length.to_be_bytes();
    let skip = bytes.iter().take_while(|byte| **byte == 0).count();
    let significant = &bytes[skip..];
    // At most eight bytes, so the count fits the low seven bits.
    #[allow(clippy::cast_possible_truncation)]
    buf.push(0x80 | significant.len() as u8);
    buf.extend_from_slice(significant);
}

/// The size of the element at the start of `buf`: its header and its body.
///
/// `Ok(None)` means `buf` does not yet hold the whole header, which is how a
/// reader of a socket knows to read more before it can know how much more.
///
/// # Errors
///
/// [`Kind::Protocol`] for a multi-byte tag, an indefinite length, a length
/// field over four bytes, and a length over [`MAX_LENGTH`].
pub fn header(buf: &[u8]) -> Result<Option<(usize, usize)>, Error> {
    let Some(&tag) = buf.first() else {
        return Ok(None);
    };
    if tag & 0x1F == 0x1F {
        return Err(malformed("a multi-byte tag"));
    }
    let Some(&first) = buf.get(1) else {
        return Ok(None);
    };
    if first < 0x80 {
        return Ok(Some((2, usize::from(first))));
    }
    let count = usize::from(first & 0x7F);
    if count == 0 {
        return Err(malformed("an indefinite length"));
    }
    if count > 4 {
        return Err(malformed("a length field over four bytes"));
    }
    let Some(field) = buf.get(2..2 + count) else {
        return Ok(None);
    };
    let length = field
        .iter()
        .fold(0usize, |acc, byte| (acc << 8) | usize::from(*byte));
    if length > MAX_LENGTH {
        return Err(malformed(
            "a message longer than the 64 MiB this client reads",
        ));
    }
    Ok(Some((2 + count, length)))
}

/// One element: its tag and its contents.
#[derive(Debug, Clone, Copy)]
pub struct Tlv<'a> {
    /// The identifier byte, class and constructed bit included.
    pub tag: u8,
    /// The contents, without the header.
    pub body: &'a [u8],
}

/// Reads the elements of one body in order.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    /// A reader over `body`.
    #[must_use]
    pub fn new(body: &'a [u8]) -> Self {
        Self { rest: body }
    }

    /// Whether every element has been read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }

    /// The tag of the next element, without reading it.
    #[must_use]
    pub fn peek_tag(&self) -> Option<u8> {
        self.rest.first().copied()
    }

    /// The next element.
    ///
    /// # Errors
    ///
    /// [`Kind::Protocol`] when no element is left, or the next one is
    /// malformed or runs past the end of the body.
    pub fn element(&mut self) -> Result<Tlv<'a>, Error> {
        let Some((head, length)) = header(self.rest)? else {
            return Err(malformed("an element cut off in its header"));
        };
        let Some(body) = self.rest.get(head..head + length) else {
            return Err(malformed("an element longer than the message holding it"));
        };
        let tag = self.rest[0];
        self.rest = &self.rest[head + length..];
        Ok(Tlv { tag, body })
    }

    /// The next element's contents, which must carry `tag`.
    ///
    /// # Errors
    ///
    /// [`Kind::Protocol`] for another tag, and whatever [`Reader::element`] throws.
    pub fn expect(&mut self, tag: u8) -> Result<&'a [u8], Error> {
        let element = self.element()?;
        if element.tag != tag {
            return Err(Error::new(
                Kind::Protocol,
                format!(
                    "the server sent tag 0x{:02x} where 0x{tag:02x} belongs",
                    element.tag
                ),
            ));
        }
        Ok(element.body)
    }

    /// The next element's contents when it carries `tag`, and nothing read
    /// otherwise.
    ///
    /// # Errors
    ///
    /// Whatever [`Reader::element`] throws.
    pub fn optional(&mut self, tag: u8) -> Result<Option<&'a [u8]>, Error> {
        if self.peek_tag() == Some(tag) {
            return Ok(Some(self.element()?.body));
        }
        Ok(None)
    }
}

/// An `INTEGER` or `ENUMERATED` body.
///
/// # Errors
///
/// [`Kind::Protocol`] for an empty body or one over eight bytes.
pub fn integer(body: &[u8]) -> Result<i64, Error> {
    if body.is_empty() || body.len() > 8 {
        return Err(malformed("an integer of no bytes or of more than eight"));
    }
    let negative = body[0] & 0x80 != 0;
    let mut value: i64 = if negative { -1 } else { 0 };
    for byte in body {
        value = (value << 8) | i64::from(*byte);
    }
    Ok(value)
}

/// A `BOOLEAN` body: any byte but zero is `TRUE`, as BER allows on receipt.
///
/// # Errors
///
/// [`Kind::Protocol`] for a body that is not one byte.
pub fn boolean(body: &[u8]) -> Result<bool, Error> {
    match body {
        [byte] => Ok(*byte != 0),
        _ => Err(malformed("a boolean that is not one byte")),
    }
}

fn malformed(what: &str) -> Error {
    Error::new(Kind::Protocol, format!("the server sent {what}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_integer_is_written_in_its_shortest_form_and_read_back() {
        for (value, encoded) in [
            (0_i64, &[0x02, 0x01, 0x00][..]),
            (127, &[0x02, 0x01, 0x7F]),
            (128, &[0x02, 0x02, 0x00, 0x80]),
            (-1, &[0x02, 0x01, 0xFF]),
            (-129, &[0x02, 0x02, 0xFF, 0x7F]),
            (1000, &[0x02, 0x02, 0x03, 0xE8]),
        ] {
            let mut writer = Writer::new();
            writer.integer(tag::INTEGER, value);
            let bytes = writer.into_bytes();
            assert_eq!(bytes, encoded, "{value}");
            assert_eq!(integer(&bytes[2..]).unwrap(), value);
        }
    }

    #[test]
    fn a_long_body_gets_a_long_length_and_reads_back() {
        let body = vec![7u8; 300];
        let mut writer = Writer::new();
        writer.constructed(tag::SEQUENCE, |inner| {
            inner.octets(tag::OCTET_STRING, &body)
        });
        let bytes = writer.into_bytes();
        assert_eq!(&bytes[..2], &[0x30, 0x82]);
        let mut outer = Reader::new(&bytes);
        let sequence = outer.expect(tag::SEQUENCE).unwrap();
        assert!(outer.is_empty());
        assert_eq!(
            Reader::new(sequence).expect(tag::OCTET_STRING).unwrap(),
            &body[..]
        );
    }

    #[test]
    fn a_header_not_yet_received_asks_for_more() {
        assert!(header(&[]).unwrap().is_none());
        assert!(header(&[0x30]).unwrap().is_none());
        assert!(header(&[0x30, 0x82, 0x01]).unwrap().is_none());
        assert_eq!(header(&[0x30, 0x82, 0x01, 0x2C]).unwrap(), Some((4, 300)));
    }

    #[test]
    fn what_ldap_does_not_allow_is_a_protocol_error() {
        for bytes in [
            &[0x30, 0x80][..],
            &[0x1F, 0x01, 0x00],
            &[0x30, 0x85, 1, 0, 0, 0, 0],
            &[0x30, 0x84, 0x7F, 0xFF, 0xFF, 0xFF],
        ] {
            let error = header(bytes).unwrap_err();
            assert_eq!(error.kind(), Kind::Protocol, "{bytes:02x?}");
        }
        let error = Reader::new(&[0x04, 0x05, 1, 2]).element().unwrap_err();
        assert_eq!(error.kind(), Kind::Protocol);
    }
}
