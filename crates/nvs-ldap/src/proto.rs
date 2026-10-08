//! LDAPv3's messages as this client sends and reads them: bind, unbind, search, the extended operation, controls, and the filter, encoded straight to BER
//!
//! RFC 4511 § 4 is the grammar. Only the operations [`crate::Connection`]
//! performs are encoded, and an incoming message whose operation this client
//! never asked for is [`Op::Other`], which the connection refuses.
//!
//! **A [`Filter`] is encoded from its tree, with no text step.** RFC 4515's
//! string form exists for people typing filters; a client that renders one and
//! has the server parse it back is where filter injection comes from, and
//! nothing here produces or reads that form. A value is an octet string inside
//! the element its tree position names, so no value can change the filter's
//! structure whatever bytes it holds.

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
        }
    }
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
}
