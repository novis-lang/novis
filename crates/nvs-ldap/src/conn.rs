//! One LDAP connection over the parking stream: LDAPS or StartTLS before a password is written, a simple bind, `whoami`, a paged search, a DirSync, the four writes, compare and an unbind
//!
//! The sequencing is this crate's own, as `rule:core-classes/db-crate-boundary`
//! asks. One operation is outstanding at a time, so every answer must carry
//! the ID of the request just sent, and anything else is [`Kind::Protocol`].
//! Every read is [`nvs_host::net`]'s park, so a request waiting on the
//! directory hands its core back rather than holding it, and nothing here
//! needs a second scheduler (`rule:concurrency/one-scheduler`).
//!
//! **Two checks run before a bind writes anything.** An empty password is
//! [`Kind::InvalidCredentials`] (`rule:security/ldap-empty-password-is-refused`):
//! an anonymous session is one that never binds. And a connection with no TLS
//! exists only where the endpoint said [`Tls::None`] *and* the caller found
//! the host on the `cleartext` grant
//! (`rule:security/ldap-cleartext-bind-is-granted-per-host`); without both,
//! [`Connection::open`] refuses an `ldap://` URL before it dials, as
//! [`Kind::EncryptionRequired`]. Reading the grant is the caller's job, because
//! the grant is configuration and this crate reads none: the endpoint carries
//! the answer as [`Endpoint::cleartext_granted`].
//!
//! **A search holds one page.** [`Cursor`] asks for `page_size` entries with
//! the paged results control, reads that page whole, hands it out one entry at
//! a time, and only then asks for the next. The control is sent critical, so
//! a server that does not page refuses the search rather than sending every
//! entry at once. **A continuation reference is data**: it is kept for
//! [`Cursor::references`] and never dialled. A result code the search ends
//! with, a size or time limit included, is an error, and the page that came
//! with it is dropped, so a partial result is never read as a whole one.
//!
//! **The caller owns the cursor**, and advances it with the connection it was
//! started on, so a search can stay open across calls that each borrow the
//! connection for a moment. [`Search`] is the two borrowed together as an
//! iterator. While a cursor has pages left on the server the connection is
//! not [settled](Connection::is_settled), and a cursor dropped before its last
//! page leaves it that way, so a pool closes the connection rather than
//! reusing it with the server still holding the search.
//!
//! **A window is one answer.** A search with a [`proto::Window`] sends the
//! sort and virtual list view controls in place of the paged one, so the
//! server cuts the slice out of the sorted result and the cursor reads it as
//! its one and only page. The count the server sends back with it is
//! [`Cursor::total`]. A sort with no window is sent with every page.
//!
//! **A sync pages by its own cookie.** [`Cursor::changes`] sends AD's DirSync
//! control in place of the paged one, and each answer carries the cookie the
//! next one is asked with. [`Cursor::cookie`] moves to an answer's cookie only
//! once every entry of that answer has been returned, so a program that stores
//! it after stopping early is sent the rest again, never skips it. The control
//! is critical, so a server without DirSync refuses the search with
//! `unavailableCriticalExtension`. Samba matches a DirSync filter against the
//! selected attributes alone, so a sync that selects any adds the ones its
//! filter tests.

use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::time::Instant;

use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;

use crate::ber;
use crate::dn::Dn;
use crate::error::{Error, Kind};
use crate::proto::{self, Control, Entry, Incoming, LdapResult, Op, SearchRequest};

/// Whether a URL is `ldap://` or `ldaps://`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// Plain TCP first, upgraded with StartTLS unless the endpoint says [`Tls::None`].
    Ldap,
    /// TLS from the first byte.
    Ldaps,
}

/// A server's URL: its scheme, host and port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    /// `ldap` or `ldaps`.
    pub scheme: Scheme,
    /// The host as written, without brackets for an IPv6 address. It is the
    /// name the server's certificate is checked against.
    pub host: String,
    /// The port, 389 or 636 when the URL names none.
    pub port: u16,
}

impl Url {
    /// Parses `ldap://host[:port]` or `ldaps://host[:port]`, with an optional
    /// trailing `/`. A DN, attributes or anything else after the host is not
    /// part of an endpoint and is `None`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (scheme, rest) = if let Some(rest) = text.strip_prefix("ldaps://") {
            (Scheme::Ldaps, rest)
        } else {
            (Scheme::Ldap, text.strip_prefix("ldap://")?)
        };
        let rest = rest.strip_suffix('/').unwrap_or(rest);
        let bracketed = rest.starts_with('[');
        let (host, port) = if bracketed {
            let (host, after) = rest[1..].split_once(']')?;
            let port = if after.is_empty() {
                None
            } else {
                Some(after.strip_prefix(':')?)
            };
            (host, port)
        } else {
            match rest.rsplit_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (rest, None),
            }
        };
        let port = match port {
            None => match scheme {
                Scheme::Ldap => 389,
                Scheme::Ldaps => 636,
            },
            Some(port) => port.parse().ok().filter(|port| *port != 0)?,
        };
        let valid = |c: char| {
            c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') || (bracketed && c == ':')
        };
        if host.is_empty() || !host.chars().all(valid) {
            return None;
        }
        Some(Self {
            scheme,
            host: host.to_owned(),
            port,
        })
    }
}

/// Whether a connection uses TLS: `tls` in the `[ldap.<name>]` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tls {
    /// LDAPS, or StartTLS on `ldap://`. The default.
    Required,
    /// No TLS on `ldap://`, which also needs the `cleartext` grant.
    None,
}

/// Everything [`Connection::open`] needs, already resolved by the caller.
#[derive(Debug, Clone, Copy)]
pub struct Endpoint<'a> {
    /// The server's URL.
    pub url: &'a Url,
    /// The address `url.host` resolved to, which the caller has already
    /// checked against the outbound address policy where that applies.
    pub address: SocketAddr,
    /// What the block or `Ldap\Settings` said about TLS.
    pub tls: Tls,
    /// The PEM bundle the server's certificate is checked against, in place
    /// of the compiled-in anchors.
    pub ca_file: Option<&'a Path>,
    /// Whether `url.host` is on `[capabilities.ldap] cleartext`.
    pub cleartext_granted: bool,
    /// When the connect, the TLS handshake and the StartTLS exchange must be
    /// done. It is lifted once the connection is open.
    pub deadline: Option<Instant>,
}

/// The socket, or the TLS session that replaced it.
///
/// `Secured` is about a kilobyte wider than `Plain`, because `rustls` keeps
/// its session state inline. There is one per open connection, so the cost is
/// O(in-flight), and boxing it would add an indirection to every read.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum Wire {
    Plain(NvsTcp),
    Secured(NvsTls),
}

impl Wire {
    fn set_deadline(&mut self, at: Option<Instant>) {
        match self {
            Self::Plain(stream) => stream.set_deadline(at),
            Self::Secured(stream) => stream.set_deadline(at),
        }
    }
}

impl Read for Wire {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(buf),
            Self::Secured(stream) => stream.read(buf),
        }
    }
}

impl Write for Wire {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.write(buf),
            Self::Secured(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Plain(stream) => stream.flush(),
            Self::Secured(stream) => stream.flush(),
        }
    }
}

/// How much one read asks the socket for.
const READ_CHUNK: usize = 16 * 1024;

/// An open LDAP connection.
#[derive(Debug)]
pub struct Connection {
    wire: Wire,
    /// Bytes read past the end of the last message.
    inbox: Vec<u8>,
    /// The ID the next request carries.
    next_id: i32,
    /// Whether a bind may be sent without TLS on this connection.
    cleartext_allowed: bool,
    /// Whether the last operation sent on this connection has its answer.
    settled: bool,
    /// How many cursors have pages left on the server.
    paging: u32,
}

impl Connection {
    /// Dials `endpoint` and makes the connection encrypted, unless it is
    /// allowed not to be.
    ///
    /// # Errors
    ///
    /// [`Kind::EncryptionRequired`] for [`Tls::None`] without the grant, before
    /// anything is dialled, and when the server refuses StartTLS.
    /// [`Kind::Unavailable`] when the socket does not open or the TLS handshake
    /// fails, a certificate that does not verify included. [`Kind::Timeout`]
    /// when the deadline passes.
    pub fn open(endpoint: &Endpoint<'_>) -> Result<Self, Error> {
        let url = endpoint.url;
        let plain = url.scheme == Scheme::Ldap && endpoint.tls == Tls::None;
        if plain && !endpoint.cleartext_granted {
            return Err(Error::new(
                Kind::EncryptionRequired,
                format!(
                    "`tls = \"none\"` needs `{}` on `[capabilities.ldap] cleartext`, \
                     so no connection was opened",
                    url.host
                ),
            ));
        }
        let reach = format!("connecting to {}:{}", url.host, url.port);
        let mut tcp = match endpoint.deadline {
            Some(at) => NvsTcp::connect_timeout(
                endpoint.address,
                at.saturating_duration_since(Instant::now()),
            ),
            None => NvsTcp::connect(endpoint.address),
        }
        .map_err(|why| Error::from_io(&reach, &why))?;
        tcp.set_deadline(endpoint.deadline);

        let mut connection = Self {
            wire: Wire::Plain(tcp),
            inbox: Vec::new(),
            next_id: 1,
            cleartext_allowed: plain,
            settled: true,
            paging: 0,
        };
        let mut connection = match (url.scheme, endpoint.tls) {
            (Scheme::Ldaps, _) => connection.secured(url, endpoint.ca_file)?,
            (Scheme::Ldap, Tls::Required) => {
                connection.start_tls()?;
                connection.secured(url, endpoint.ca_file)?
            }
            (Scheme::Ldap, Tls::None) => connection,
        };
        connection.wire.set_deadline(None);
        Ok(connection)
    }

    /// Sends StartTLS and checks the server agreed and sent nothing after.
    fn start_tls(&mut self) -> Result<(), Error> {
        let answer = self.request(&proto::extended_request(proto::START_TLS), &[], "StartTLS")?;
        let Op::Extended { result, .. } = answer.op else {
            return Err(unexpected("StartTLS"));
        };
        if result.code != 0 {
            let error = Error::from_result("StartTLS", result.code, &result.diagnostic);
            // A server that will not upgrade leaves no encrypted path, which
            // is the condition `EncryptionRequired` names.
            return Err(Error::new(Kind::EncryptionRequired, error.message()));
        }
        // A man in the middle who writes a message after the server's answer
        // and before the handshake would have it read later as if the secured
        // peer had sent it. The server owes silence here, so anything is refused.
        if !self.inbox.is_empty() {
            return Err(Error::new(
                Kind::Protocol,
                format!(
                    "the server sent {} more byte(s) after agreeing to StartTLS",
                    self.inbox.len()
                ),
            ));
        }
        Ok(())
    }

    /// The same connection as a TLS session to `url.host`.
    ///
    /// Taken by value because [`NvsTls::over`] takes the socket by value, which
    /// is what leaves no plaintext stream behind once the session is secured.
    fn secured(self, url: &Url, ca_file: Option<&Path>) -> Result<Self, Error> {
        let Self {
            wire,
            inbox,
            next_id,
            cleartext_allowed,
            settled,
            paging,
        } = self;
        let wire = match wire {
            Wire::Plain(tcp) => Wire::Secured(
                match ca_file {
                    Some(bundle) => NvsTls::over_bundle(tcp, &url.host, bundle),
                    None => NvsTls::over(tcp, &url.host),
                }
                .map_err(|why| {
                    Error::from_io(&format!("the TLS handshake with `{}`", url.host), &why)
                })?,
            ),
            secured @ Wire::Secured(_) => secured,
        };
        Ok(Self {
            wire,
            inbox,
            next_id,
            cleartext_allowed,
            settled,
            paging,
        })
    }

    /// Whether the connection is a TLS session.
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        matches!(self.wire, Wire::Secured(_))
    }

    /// Whether every operation started on this connection has finished, a
    /// paged search included, which is the property a pool checks before it
    /// reuses one.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        self.settled && self.paging == 0
    }

    /// Files a deadline for the reads and writes that follow, or lifts it.
    pub fn set_deadline(&mut self, at: Option<Instant>) {
        self.wire.set_deadline(at);
    }

    /// A simple bind as `name` with `password`.
    ///
    /// # Errors
    ///
    /// [`Kind::InvalidCredentials`] for an empty password, before anything is
    /// sent, and for a password the server refuses. AD's sub-codes give the
    /// other kinds [`Kind::of_result`] lists. [`Kind::EncryptionRequired`] for
    /// a server that will not take a bind without TLS.
    pub fn bind(&mut self, name: &str, password: &[u8]) -> Result<(), Error> {
        if password.is_empty() {
            return Err(Error::new(
                Kind::InvalidCredentials,
                "the bind has an empty password, so nothing was sent",
            ));
        }
        if !self.is_encrypted() && !self.cleartext_allowed {
            return Err(Error::new(
                Kind::EncryptionRequired,
                "the connection is not encrypted, so the password was not sent",
            ));
        }
        let answer = self.request(&proto::bind_request(name, password), &[], "the bind")?;
        let Op::BindResponse(result) = answer.op else {
            return Err(unexpected("the bind"));
        };
        succeeded("the bind", &result)
    }

    /// The identity the connection is bound as, as the server spells it:
    /// `u:DOMAIN\user`, `dn:<DN>`, or empty when it is anonymous.
    ///
    /// # Errors
    ///
    /// [`Kind::Unsupported`] for a server without RFC 4532, and whatever the
    /// socket reports.
    pub fn who_am_i(&mut self) -> Result<String, Error> {
        let answer = self.request(&proto::extended_request(proto::WHO_AM_I), &[], "whoami")?;
        let Op::Extended { result, value, .. } = answer.op else {
            return Err(unexpected("whoami"));
        };
        succeeded("whoami", &result)?;
        String::from_utf8(value.unwrap_or_default())
            .map_err(|_| Error::new(Kind::Protocol, "whoami: the identity is not UTF-8"))
    }

    /// Starts a paged search on this connection. The first page is asked for
    /// when the returned [`Search`] is first read.
    pub fn search(&mut self, request: &SearchRequest<'_>) -> Search<'_> {
        Search {
            connection: self,
            cursor: Cursor::new(request),
        }
    }

    /// Applies `changes` to the entry at `dn` in one Modify request, so the
    /// server applies all of them, in order, or none.
    ///
    /// # Errors
    ///
    /// Every result but success, by [`Kind::of_result`]: [`Kind::NoSuchObject`],
    /// [`Kind::AlreadyExists`] for a value added twice,
    /// [`Kind::ConstraintViolation`] for one the schema does not allow.
    pub fn modify(&mut self, dn: &str, changes: &[proto::Change]) -> Result<(), Error> {
        self.write(&proto::modify_request(dn, changes), "the modify")
    }

    /// Adds a new entry at `dn` with `attributes`.
    ///
    /// # Errors
    ///
    /// [`Kind::AlreadyExists`] for a DN already taken, and every other result
    /// but success.
    pub fn add(&mut self, dn: &str, attributes: &[proto::Attribute]) -> Result<(), Error> {
        self.write(&proto::add_request(dn, attributes), "the add")
    }

    /// Deletes the entry at `dn`, which must have no entries below it.
    ///
    /// # Errors
    ///
    /// [`Kind::NoSuchObject`], and every other result but success.
    pub fn delete(&mut self, dn: &str) -> Result<(), Error> {
        self.write(&proto::delete_request(dn), "the delete")
    }

    /// Renames the entry at `from` to `to`: its first level becomes `to`'s,
    /// and where `to` is below another parent the entry moves there.
    ///
    /// # Errors
    ///
    /// [`Kind::NoSuchObject`], [`Kind::AlreadyExists`] for a name already
    /// taken, and every other result but success.
    pub fn rename(&mut self, from: &Dn, to: &Dn) -> Result<(), Error> {
        let moved = match (from.parent(), to.parent()) {
            (Some(old), Some(new)) if old.same(&new) => None,
            (None, None) => None,
            (_, new) => Some(new.map(|new| new.to_text()).unwrap_or_default()),
        };
        self.write(
            &proto::modify_dn_request(&from.to_text(), &to.rdn().to_text(), moved.as_deref()),
            "the rename",
        )
    }

    /// Resets the password of the account at `dn`, as an administrator does:
    /// one Modify that replaces AD's `unicodePwd` with [`unicode_pwd`]'s form
    /// of `password`.
    ///
    /// # Errors
    ///
    /// [`Kind::EncryptionRequired`] on a connection that is not a TLS
    /// session, before anything is sent and whatever the cleartext grant
    /// says. [`Kind::PasswordPolicy`] for a password the domain's policy
    /// does not accept, and every other result but success.
    pub fn set_password(&mut self, dn: &str, password: &str) -> Result<(), Error> {
        self.password_write("the password reset")?;
        let changes = [proto::Change {
            kind: proto::ChangeKind::Replace,
            attribute: "unicodePwd".to_owned(),
            values: vec![unicode_pwd(password)],
        }];
        self.write(&proto::modify_request(dn, &changes), "the password reset")
    }

    /// Changes the password of the account at `dn`, as its owner does: one
    /// Modify that removes `old` from `unicodePwd` and adds `new`, so the
    /// server checks `old` and the password history.
    ///
    /// # Errors
    ///
    /// As [`Connection::set_password`], and [`Kind::ConstraintViolation`]
    /// or [`Kind::InvalidCredentials`] for an `old` that is not the password.
    pub fn change_password(&mut self, dn: &str, old: &str, new: &str) -> Result<(), Error> {
        self.password_write("the password change")?;
        let changes = [
            proto::Change {
                kind: proto::ChangeKind::Remove,
                attribute: "unicodePwd".to_owned(),
                values: vec![unicode_pwd(old)],
            },
            proto::Change {
                kind: proto::ChangeKind::Add,
                attribute: "unicodePwd".to_owned(),
                values: vec![unicode_pwd(new)],
            },
        ];
        self.write(&proto::modify_request(dn, &changes), "the password change")
    }

    /// Refuses a password write on a connection that is not a TLS session.
    fn password_write(&self, operation: &str) -> Result<(), Error> {
        if self.is_encrypted() {
            return Ok(());
        }
        Err(Error::new(
            Kind::EncryptionRequired,
            format!("{operation} needs an encrypted connection, so the password was not sent"),
        ))
    }

    /// Whether the entry at `dn` has `value` under `attribute`, as the
    /// server's own matching rule for the attribute decides.
    ///
    /// # Errors
    ///
    /// Every result but `compareTrue` and `compareFalse`.
    pub fn compare(&mut self, dn: &str, attribute: &str, value: &[u8]) -> Result<bool, Error> {
        let answer = self.request(
            &proto::compare_request(dn, attribute, value),
            &[],
            "the compare",
        )?;
        let Op::Compared(result) = answer.op else {
            return Err(unexpected("the compare"));
        };
        match result.code {
            5 => Ok(false),
            6 => Ok(true),
            _ => Err(Error::from_result(
                "the compare",
                result.code,
                &result.diagnostic,
            )),
        }
    }

    /// Sends one of the four writes and reads its result.
    fn write(&mut self, op: &[u8], operation: &str) -> Result<(), Error> {
        let answer = self.request(op, &[], operation)?;
        let Op::Written(result) = answer.op else {
            return Err(unexpected(operation));
        };
        succeeded(operation, &result)
    }

    /// Ends the session and closes the socket.
    ///
    /// # Errors
    ///
    /// Whatever the socket reports while the unbind is written.
    pub fn unbind(mut self) -> Result<(), Error> {
        let id = self.take_id();
        self.send(
            &proto::message(id, &proto::unbind_request(), &[]),
            "the unbind",
        )
    }

    fn take_id(&mut self) -> i32 {
        let id = self.next_id;
        self.next_id = if id == i32::MAX { 1 } else { id + 1 };
        id
    }

    fn send(&mut self, bytes: &[u8], operation: &str) -> Result<(), Error> {
        self.wire
            .write_all(bytes)
            .and_then(|()| self.wire.flush())
            .map_err(|why| Error::from_io(operation, &why))
    }

    /// Sends `op` and reads its one answer.
    fn request(
        &mut self,
        op: &[u8],
        controls: &[Control],
        operation: &str,
    ) -> Result<Incoming, Error> {
        let id = self.take_id();
        self.send(&proto::message(id, op, controls), operation)?;
        self.settled = false;
        let answer = self.receive(id, operation)?;
        self.settled = true;
        Ok(answer)
    }

    /// Reads the next message, which must answer request `id`.
    fn receive(&mut self, id: i32, operation: &str) -> Result<Incoming, Error> {
        loop {
            if let Some((head, length)) = ber::header(&self.inbox)? {
                let total = head + length;
                if self.inbox.len() >= total {
                    let incoming = proto::decode(&self.inbox[..total])?;
                    self.inbox.drain(..total);
                    return self.answer(incoming, id, operation);
                }
            }
            let start = self.inbox.len();
            self.inbox.resize(start + READ_CHUNK, 0);
            let read = self.wire.read(&mut self.inbox[start..]);
            let read = match read {
                Ok(read) => read,
                Err(why) => {
                    self.inbox.truncate(start);
                    return Err(Error::from_io(operation, &why));
                }
            };
            self.inbox.truncate(start + read);
            if read == 0 {
                return Err(Error::new(
                    Kind::Unavailable,
                    format!("{operation}: the server closed the connection"),
                ));
            }
        }
    }

    fn answer(&self, incoming: Incoming, id: i32, operation: &str) -> Result<Incoming, Error> {
        if incoming.id == 0 {
            let reason = match &incoming.op {
                Op::Extended { result, .. } => result.diagnostic.clone(),
                _ => String::new(),
            };
            return Err(Error::new(
                Kind::Unavailable,
                format!("{operation}: the server is closing the connection: {reason}"),
            ));
        }
        if incoming.id != i64::from(id) {
            return Err(Error::new(
                Kind::Protocol,
                format!(
                    "{operation}: the server answered request {} while {id} was outstanding",
                    incoming.id
                ),
            ));
        }
        if let Op::Other(tag) = incoming.op {
            return Err(Error::new(
                Kind::Protocol,
                format!("{operation}: the server sent an operation with tag 0x{tag:02x}"),
            ));
        }
        Ok(incoming)
    }
}

fn succeeded(operation: &str, result: &LdapResult) -> Result<(), Error> {
    if result.code == 0 {
        return Ok(());
    }
    Err(Error::from_result(
        operation,
        result.code,
        &result.diagnostic,
    ))
}

fn unexpected(operation: &str) -> Error {
    Error::new(
        Kind::Protocol,
        format!("{operation}: the server answered with another operation"),
    )
}

/// `password` as AD stores it in `unicodePwd`: the text between double
/// quotes, in UTF-16LE.
#[must_use]
pub fn unicode_pwd(password: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity((password.len() + 2) * 2);
    for unit in "\""
        .encode_utf16()
        .chain(password.encode_utf16())
        .chain("\"".encode_utf16())
    {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Paging {
    First,
    More,
    Done,
}

/// How a [`Cursor`] asks for its next answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// With the paged results control and the last page's cookie.
    Paged,
    /// Once, with the virtual list view control.
    Window,
    /// With the DirSync control and the last answer's cookie.
    Sync,
}

/// A search the caller owns: one page of entries held, the next asked for
/// when this one is used up.
///
/// It is advanced with the connection it was started on, and with no other.
#[derive(Debug)]
pub struct Cursor {
    /// The `SearchRequest`, encoded once.
    op: Vec<u8>,
    /// The sort, window and show deleted controls, sent with every page.
    extra: Vec<Control>,
    /// How the search asks for its next answer.
    mode: Mode,
    /// For a sync, the cookie the answer being handed out was asked with,
    /// which [`Cursor::cookie`] returns until its last entry is.
    resume: Vec<u8>,
    /// The count of the whole sorted result a window's answer carried.
    total: Option<u32>,
    page_size: u32,
    cookie: Vec<u8>,
    page: VecDeque<Entry>,
    references: Vec<String>,
    pages: u32,
    state: Paging,
    /// Whether a ranged attribute is fetched to its end before its entry is
    /// returned, which is false only for [`crate::ranged`]'s own follow-ups.
    whole: bool,
}

impl Cursor {
    /// A search for `request` that has sent nothing yet.
    #[must_use]
    pub fn new(request: &SearchRequest<'_>) -> Self {
        let mut extra = Vec::new();
        if let Some(sort) = &request.sort {
            extra.push(Control::sort(sort));
        }
        if let Some(window) = request.window {
            extra.push(Control::window(window));
        }
        if request.show_deleted {
            extra.push(Control::show_deleted());
        }
        Self {
            op: proto::search_request(request),
            extra,
            mode: if request.window.is_some() {
                Mode::Window
            } else {
                Mode::Paged
            },
            resume: Vec::new(),
            total: None,
            page_size: request.page_size.max(1),
            cookie: Vec::new(),
            page: VecDeque::new(),
            references: Vec::new(),
            pages: 0,
            state: Paging::First,
            whole: true,
        }
    }

    /// A DirSync search for `request`'s entries changed since `cookie`, or
    /// for every entry when the cookie is empty. It asks with the cookie in
    /// place of pages, and the server sends a new cookie with each answer.
    #[must_use]
    pub fn changes(request: &SearchRequest<'_>, cookie: &[u8]) -> Self {
        let tested = request.filter.attributes();
        let mut attributes = request.attributes.to_vec();
        // Samba matches a DirSync filter against the selected attributes
        // only, and sends a deletion only when `isDeleted` is selected, so
        // each one the filter tests is selected too, and `isDeleted`.
        if !attributes.is_empty() {
            for name in tested.iter().map(String::as_str).chain(["isDeleted"]) {
                if !attributes.iter().any(|had| had.eq_ignore_ascii_case(name)) {
                    attributes.push(name);
                }
            }
        }
        Self {
            mode: Mode::Sync,
            cookie: cookie.to_vec(),
            resume: cookie.to_vec(),
            ..Self::new(&SearchRequest {
                attributes: &attributes,
                ..*request
            })
        }
    }

    /// The cookie a later sync starts from: the server's cookie for every
    /// answer whose entries were all returned, so a sync that stops early
    /// sends the rest again rather than skipping them. Empty for a search
    /// that is not a sync.
    #[must_use]
    pub fn cookie(&self) -> &[u8] {
        if self.page.is_empty() {
            &self.cookie
        } else {
            &self.resume
        }
    }

    /// [`Cursor::new`] for one step of [`crate::ranged::complete`], which
    /// returns a ranged attribute as the server sent it.
    pub(crate) fn single(request: &SearchRequest<'_>) -> Self {
        Self {
            whole: false,
            ..Self::new(request)
        }
    }

    /// The continuation references the search returned so far. None of them
    /// was followed.
    #[must_use]
    pub fn references(&self) -> &[String] {
        &self.references
    }

    /// How many entries of the current page are held and not yet returned.
    #[must_use]
    pub fn held(&self) -> usize {
        self.page.len()
    }

    /// How many pages the server has sent.
    #[must_use]
    pub fn pages(&self) -> u32 {
        self.pages
    }

    /// Whether the server has sent its last page.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.state == Paging::Done
    }

    /// How many entries the whole sorted result has, which the server sends
    /// with a window's answer, or `None` for a search with no window or one
    /// not started yet.
    #[must_use]
    pub fn total(&self) -> Option<u32> {
        self.total
    }

    /// Asks for the first page now, if it has not been asked for, so a search
    /// the server refuses fails here rather than at the first entry.
    ///
    /// # Errors
    ///
    /// Every failure [`Cursor::next`] reports.
    pub fn start(&mut self, connection: &mut Connection) -> Result<(), Error> {
        if self.state == Paging::First {
            self.fetch_or_stop(connection)?;
        }
        Ok(())
    }

    /// The next entry, asking `connection` for the next page when this one is
    /// used up, or `None` once the last page is. A ranged attribute in it is
    /// fetched to its end first ([`crate::ranged`]).
    pub fn next(&mut self, connection: &mut Connection) -> Option<Result<Entry, Error>> {
        loop {
            if let Some(entry) = self.page.pop_front() {
                if self.whole {
                    return Some(crate::ranged::complete(connection, entry));
                }
                return Some(Ok(entry));
            }
            if self.state == Paging::Done {
                return None;
            }
            if let Err(error) = self.fetch_or_stop(connection) {
                return Some(Err(error));
            }
        }
    }

    /// [`Cursor::fetch`], and on a failure the search ends with nothing held.
    fn fetch_or_stop(&mut self, connection: &mut Connection) -> Result<(), Error> {
        let fetched = self.fetch(connection);
        if fetched.is_err() {
            self.page.clear();
            self.move_to(connection, Paging::Done);
        }
        fetched
    }

    /// Asks for the next page and reads it whole.
    fn fetch(&mut self, connection: &mut Connection) -> Result<(), Error> {
        let id = connection.take_id();
        let mut controls = self.extra.clone();
        match self.mode {
            Mode::Paged => controls.push(Control::paged(self.page_size, &self.cookie)),
            Mode::Window => {}
            Mode::Sync => controls.push(Control::dir_sync(&self.cookie)),
        }
        connection.send(&proto::message(id, &self.op, &controls), "the search")?;
        connection.settled = false;
        loop {
            let incoming = connection.receive(id, "the search")?;
            match incoming.op {
                Op::SearchEntry(entry) => self.page.push_back(entry),
                Op::SearchReference(urls) => self.references.extend(urls),
                Op::SearchDone(result) => {
                    connection.settled = true;
                    self.pages += 1;
                    succeeded("the search", &result)?;
                    if self.mode == Mode::Sync {
                        let (more, cookie) = incoming
                            .controls
                            .iter()
                            .find(|control| control.oid == proto::DIR_SYNC)
                            .ok_or_else(|| unexpected("the search"))?
                            .dir_sync_answer()?;
                        self.resume = std::mem::replace(&mut self.cookie, cookie);
                        let next = if more { Paging::More } else { Paging::Done };
                        self.move_to(connection, next);
                        return Ok(());
                    }
                    if self.mode == Mode::Window {
                        let (total, code) = incoming
                            .controls
                            .iter()
                            .find(|control| control.oid == proto::VLV_RESPONSE)
                            .ok_or_else(|| unexpected("the search"))?
                            .window_total()?;
                        // The window's own result code, which a server may
                        // send beside a search that otherwise succeeded.
                        succeeded(
                            "the search",
                            &LdapResult {
                                code,
                                diagnostic: String::new(),
                                referrals: Vec::new(),
                            },
                        )?;
                        self.total = Some(total);
                        self.move_to(connection, Paging::Done);
                        return Ok(());
                    }
                    let cookie = incoming
                        .controls
                        .iter()
                        .find(|control| control.oid == proto::PAGED_RESULTS)
                        .map(Control::paged_cookie)
                        .transpose()?
                        .unwrap_or_default();
                    let next = if cookie.is_empty() {
                        Paging::Done
                    } else {
                        Paging::More
                    };
                    self.move_to(connection, next);
                    self.cookie = cookie;
                    return Ok(());
                }
                _ => return Err(unexpected("the search")),
            }
        }
    }

    /// Moves to `next`, counting the search on `connection` while the server
    /// holds pages of it.
    fn move_to(&mut self, connection: &mut Connection, next: Paging) {
        match (self.state == Paging::More, next == Paging::More) {
            (false, true) => connection.paging += 1,
            (true, false) => connection.paging -= 1,
            _ => {}
        }
        self.state = next;
    }
}

/// A [`Cursor`] and the connection it runs on, borrowed together as an iterator.
#[derive(Debug)]
pub struct Search<'c> {
    connection: &'c mut Connection,
    cursor: Cursor,
}

impl Search<'_> {
    /// [`Cursor::references`].
    #[must_use]
    pub fn references(&self) -> &[String] {
        self.cursor.references()
    }

    /// [`Cursor::held`].
    #[must_use]
    pub fn held(&self) -> usize {
        self.cursor.held()
    }

    /// [`Cursor::pages`].
    #[must_use]
    pub fn pages(&self) -> u32 {
        self.cursor.pages()
    }
}

impl Iterator for Search<'_> {
    type Item = Result<Entry, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor.next(self.connection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_password_is_quoted_and_utf16le() {
        assert_eq!(unicode_pwd("ab"), b"\"\0a\0b\0\"\0");
        assert_eq!(unicode_pwd("é"), b"\"\0\xe9\0\"\0");
    }

    #[test]
    fn a_url_names_its_scheme_host_and_port() {
        let url = Url::parse("ldaps://dc1.example.test").unwrap();
        assert_eq!(
            (url.scheme, url.host.as_str(), url.port),
            (Scheme::Ldaps, "dc1.example.test", 636)
        );
        let url = Url::parse("ldap://127.0.0.1:16389/").unwrap();
        assert_eq!(
            (url.scheme, url.host.as_str(), url.port),
            (Scheme::Ldap, "127.0.0.1", 16389)
        );
        let url = Url::parse("ldap://[::1]:389").unwrap();
        assert_eq!((url.host.as_str(), url.port), ("::1", 389));
    }

    #[test]
    fn a_url_with_more_than_an_endpoint_is_refused() {
        for text in [
            "http://example.test",
            "ldap://",
            "ldap://example.test:0",
            "ldap://example.test:x",
            "ldap://example.test/DC=example,DC=test",
            "ldap://user@example.test",
        ] {
            assert!(Url::parse(text).is_none(), "{text}");
        }
    }
}
