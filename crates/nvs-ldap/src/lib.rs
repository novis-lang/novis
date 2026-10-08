//! The LDAP wire below `Core\Ldap`: a BER codec of LDAP's own subset, and a connection over `nvs-host`'s parking stream
//!
//! `rule:core-classes/db-crate-boundary` is the shape this crate copies from
//! `nvs-db`: the wire lives in a crate below `nvs-stdlib`, `nvs-stdlib` depends
//! on it and never the reverse, and the protocol's sequencing is written here.
//! ADR 0278 is the design `Core\Ldap` is built to, and this crate holds the
//! parts of it that are protocol: § 3's bind and its two client-side checks,
//! § 5's paged search and its unfollowed references, and § 10's error kinds.
//!
//! - [`ber`] encodes and reads the BER subset RFC 4511 § 5.1 allows, and its
//!   module doc says why it is written here rather than borrowed.
//! - [`proto`] is the messages, the controls and the filter tree.
//! - [`dn`] is RFC 4514's distinguished name, escaped as it is built.
//! - [`conn`] is one connection: LDAPS or StartTLS, bind, `whoami`, search,
//!   unbind.
//! - [`error`] is the one error type, with ADR 0278 § 10's kinds.
//! - [`value`] reads the value forms AD sends that are not text: a GUID, a
//!   SID, a FILETIME, an interval and a GeneralizedTime.
//! - [`ranged`] fetches a ranged attribute (`member;range=0-1499`) to its end.
//! - [`schema`] reads the syntax of every attribute type the server declares.
//!
//! Nothing here reads configuration or a capability grant. The caller resolves
//! the address, checks it against the outbound policy and the grants, and
//! hands [`Connection::open`] an [`Endpoint`] carrying the answers.

pub mod ber;
pub mod conn;
pub mod dn;
pub mod error;
pub mod proto;
pub mod ranged;
pub mod schema;
pub mod value;

pub use conn::{Connection, Cursor, Endpoint, Scheme, Search, Tls, Url};
pub use dn::{Ava, Dn, PartError, Rdn};
pub use error::{Error, Kind};
pub use proto::{
    Attribute, Entry, Filter, Scope, SearchRequest, TextError, is_attribute_description,
};
pub use schema::{Schema, Syntax};
pub use value::{GeneralizedTime, Sid, SidTextError, ValueError};
