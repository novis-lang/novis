//! One SMTP session: the connection, implicit TLS or `STARTTLS`, `AUTH PLAIN`, `AUTH LOGIN` or
//! `AUTH XOAUTH2`, and the envelope, over [`nvs_host`]'s parking stream.
//!
//! A session has two halves. [`open`] connects, secures and signs in, and [`transact`] sends one
//! message over the result. [`deliver`] is both, for a block with `pool = false`, and
//! [`super::pool`] runs the second half again over a session it kept. The module doc of [`super`]
//! says why a credential needs TLS, and why a block with no credential and no `tls` stays in the
//! clear.
//!
//! **Nothing is sent that the server will refuse part of.** A message over the `SIZE` the server
//! advertised gets no `MAIL FROM`, and a rejected recipient ends the transaction with `RSET`
//! before `DATA`, naming every address the server rejected. Sending to the rest is declined: a
//! program that wants that sends one message per recipient.

use std::io::{Read, Write};
use std::net::SocketAddr;
use std::time::Instant;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use nvs_config::mail::{Auth, Tls};
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;
use nvs_runtime::{Fault, ThrownClass};

use super::Endpoint;

/// Who the message is from and to, as the server sees it: `MAIL FROM` and every `RCPT TO`.
pub(super) struct Envelope<'a> {
    /// The envelope sender — the block's `return_path`, with the call's bounce tag when it has one.
    pub(super) sender: &'a str,
    /// Every `to`, `cc` and `bcc` address, in that order.
    pub(super) recipients: &'a [String],
}

/// The longest bounce tag, which keeps the tagged local part inside RFC 5321's 64 octets for an
/// ordinary `return_path`.
const BOUNCE_TAG_CEILING: usize = 64;

/// The envelope sender for `return_path` and the call's `tag`: `return_path` itself without one,
/// and `local+tag@domain` with one.
///
/// The program chooses only the tag. The local part and the domain stay the operator's, so the
/// envelope identity cannot be moved to another domain.
///
/// # Errors
///
/// A catchable `RuntimeError` for a tag that is empty, longer than [`BOUNCE_TAG_CEILING`], or holds
/// anything but `A-Z a-z 0-9 . _ -`.
pub(super) fn bounce_sender(
    return_path: &str,
    tag: Option<&str>,
    member: &str,
) -> Result<String, Fault> {
    let Some(tag) = tag else {
        return Ok(return_path.to_owned());
    };
    let allowed = |byte: &u8| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-');
    if tag.is_empty() || tag.len() > BOUNCE_TAG_CEILING || !tag.bytes().all(|byte| allowed(&byte)) {
        return Err(Fault::thrown(format!(
            "{member}: `bounceTag` is `{}`. A bounce tag has 1 to {BOUNCE_TAG_CEILING} letters, \
             digits, dots, underscores or hyphens.",
            tag.escape_debug()
        )));
    }
    // `return_path` came through `address_of`, so it has exactly one `@`.
    let (local, domain) = return_path.rsplit_once('@').unwrap_or((return_path, ""));
    Ok(format!("{local}+{tag}@{domain}"))
}

/// One SMTP conversation over [`nvs_host`]'s parking stream.
///
/// Held as a struct for the read buffer: a reply is line-oriented and a
/// multiline one is only finished when a line's fourth byte is a space, so a
/// reader that did not keep what it over-read would lose the head of the next
/// reply. It also keeps the extensions of its last `EHLO`, which every message
/// sent over it reads.
pub(super) struct Session {
    /// The socket, or the TLS session `STARTTLS` replaced it with — either way
    /// it hands the core back rather than blocking it.
    stream: Wire,
    /// Whatever the last read took past the end of the reply it was completing.
    held: Vec<u8>,
    /// The lines of the last `EHLO` reply, which [`advertised`] reads.
    extensions: Vec<String>,
}

/// The stream is not printed: a pooled session is debugged by its place in the pool.
impl std::fmt::Debug for Session {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Session")
            .field("secured", &self.secured())
            .field("extensions", &self.extensions.len())
            .finish_non_exhaustive()
    }
}

/// What a [`Session`] is talking over.
///
/// An enum rather than a boxed trait object because there are two variants and
/// there will not be a third, and because the upgrade *consumes* the socket:
/// [`NvsTls::over`] takes an [`NvsTcp`] by value, which is what leaves nobody
/// holding a plaintext stream to an endpoint that has been secured.
///
/// `Secured` is about a kilobyte wider than `Plain` — `rustls`'s session state
/// is held inline — and that is the trade AGENTS.md's priority ordering asks
/// for, spent deliberately: there is one `Wire` per in-flight `send`, so the
/// cost is O(in-flight) rather than O(messages sent), and boxing it would buy an
/// allocation and an indirection on every record read to save a kilobyte
/// of memory footprint, which the ordering says not to chase.
#[allow(clippy::large_enum_variant)]
enum Wire {
    /// Before `STARTTLS`, and for the whole of a session with `tls = "none"`.
    Plain(NvsTcp),
    /// After it. Every wait inside is still [`nvs_host::net`]'s park, which
    /// [`nvs_host::tls`]'s module doc is the home of.
    Secured(NvsTls),
}

impl Read for Wire {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(buf),
            Self::Secured(stream) => stream.read(buf),
        }
    }
}

impl Write for Wire {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.write(buf),
            Self::Secured(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(stream) => stream.flush(),
            Self::Secured(stream) => stream.flush(),
        }
    }
}

/// A TLS session over `stream`, verified against the name the operator wrote
/// as `host` and against the block's `tls_ca_file` when it names one.
///
/// The certificate is checked against that name, the same rule
/// `crate::http::transport` states: the address was derived from it and
/// answers for nothing on its own. A `tls_ca_file` replaces the compiled-in
/// anchors for this block alone ([`nvs_host::tls`]'s module doc is why it
/// replaces rather than adds).
///
/// # Errors
///
/// A catchable `IOError` when the handshake does not complete, including a
/// certificate that does not verify.
fn handshake(stream: NvsTcp, endpoint: &Endpoint, member: &str) -> Result<NvsTls, Fault> {
    let name = endpoint.host.as_str();
    match &endpoint.ca_file {
        Some(bundle) => NvsTls::over_bundle(stream, name, bundle),
        None => NvsTls::over(stream, name),
    }
    .map_err(|why| {
        let message = if nvs_host::tls::unknown_issuer(&why) {
            format!(
                "{member}: the TLS handshake with `{name}` did not complete. Its certificate is \
                 signed by a CA this host does not trust. If the server uses a private or company \
                 CA, set `[mail.{}] tls_ca_file` to a PEM file of that CA.",
                endpoint.name
            )
        } else {
            format!("{member}: the TLS handshake with `{name}` did not complete — {why}")
        };
        Fault::thrown_as(ThrownClass::Io, message)
    })
}

impl Session {
    /// The connected session, with the server's banner already accepted, and
    /// over TLS from the first byte when the block's `tls` is `implicit`.
    ///
    /// Every wait of the exchange ends at `until`: the connect, the handshake,
    /// the banner, each command and the `DATA` body.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` for a socket that will not open, a handshake that
    /// does not complete, and a banner that is not a 220.
    fn open(
        address: SocketAddr,
        endpoint: &Endpoint,
        until: Instant,
        member: &str,
    ) -> Result<Self, Fault> {
        let budget = until.saturating_duration_since(Instant::now());
        let mut stream = NvsTcp::connect_timeout(address, budget).map_err(|why| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!("{member} could not reach {address}: {why}"),
            )
        })?;
        stream.set_deadline(Some(until));
        let stream = if endpoint.tls == Tls::Implicit {
            Wire::Secured(handshake(stream, endpoint, member)?)
        } else {
            Wire::Plain(stream)
        };
        let mut session = Self {
            stream,
            held: Vec::new(),
            extensions: Vec::new(),
        };
        session.expect(220, "the greeting", member)?;
        Ok(session)
    }

    /// Bounds every later wait of the session by `until`, for the message about to be sent.
    pub(super) fn set_deadline(&mut self, until: Instant) {
        match &mut self.stream {
            Wire::Plain(stream) => stream.set_deadline(Some(until)),
            Wire::Secured(stream) => stream.set_deadline(Some(until)),
        }
    }

    /// Ends the session politely. A `QUIT` the server never answers changes nothing, so its
    /// reply is not read.
    pub(super) fn quit(&mut self) {
        let _ = self.stream.write_all(b"QUIT\r\n");
    }

    /// Whether the session runs over TLS, from the first byte or after `STARTTLS`.
    fn secured(&self) -> bool {
        matches!(self.stream, Wire::Secured(_))
    }

    /// The same conversation over TLS: `STARTTLS`, a handshake verified by
    /// [`handshake`], and the session that answers on the other side of it.
    ///
    /// Taken by value, which is the property rather than a style choice — the
    /// plaintext session is gone by the time this returns, so no later step can
    /// reach for it.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the endpoint refuses `STARTTLS`, when it sends
    /// anything between agreeing to it and the handshake, and when the handshake
    /// does not complete — including a certificate that does not verify.
    fn secure(mut self, endpoint: &Endpoint, member: &str) -> Result<Self, Fault> {
        self.command("STARTTLS", 220, "the upgrade", member)?;
        // The whole reason `held` is inspected anywhere. A man in the middle
        // who writes commands *after* the endpoint's 220 and before the
        // handshake has them buffered here, and a session that carried the
        // buffer across would replay them as though the secured peer had sent
        // them. The endpoint owes silence between the two, so anything at all is
        // refused rather than discarded — discarding is the same defence with
        // the evidence thrown away.
        if !self.held.is_empty() {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member}: the endpoint sent {} more byte(s) after agreeing to `STARTTLS`, \
                     which is how a plaintext command is smuggled into a secured session",
                    self.held.len()
                ),
            ));
        }
        let Wire::Plain(stream) = self.stream else {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!("{member}: the session is already secured"),
            ));
        };
        Ok(Self {
            stream: Wire::Secured(handshake(stream, endpoint, member)?),
            held: Vec::new(),
            extensions: Vec::new(),
        })
    }

    /// One command, and the lines of the reply it was answered with.
    ///
    /// `line` never reaches a diagnostic — `what` names the step instead —
    /// because some of the commands this sends carry the credential.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the write fails, when the connection ends
    /// mid-reply, or when the code is not `want`.
    fn command(
        &mut self,
        line: &str,
        want: i64,
        what: &str,
        member: &str,
    ) -> Result<Vec<String>, Fault> {
        self.send(line, what, member)?;
        self.expect(want, what, member)
    }

    /// One command, and the code and lines of whatever the server replied.
    ///
    /// # Errors
    ///
    /// [`command`](Self::command)'s, except that no code is an error.
    fn ask(&mut self, line: &str, what: &str, member: &str) -> Result<(i64, Vec<String>), Fault> {
        self.send(line, what, member)?;
        self.reply(what, member)
    }

    /// Writes `line` and its terminator.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the write fails.
    fn send(&mut self, line: &str, what: &str, member: &str) -> Result<(), Fault> {
        self.stream
            .write_all(format!("{line}\r\n").as_bytes())
            .and_then(|()| self.stream.flush())
            .map_err(|why| {
                Fault::thrown_as(
                    ThrownClass::Io,
                    format!("{member} could not send {what}: {why}"),
                )
            })
    }

    /// Reads one whole reply, holds it to `want`, and answers with its lines —
    /// which for `EHLO` are the extension list [`advertised`] reads.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` for a closed connection, a reply that is not
    /// three digits, or a code other than `want` — the last carrying the
    /// server's own sentence, because that is the only part of the failure the
    /// operator did not already write.
    fn expect(&mut self, want: i64, what: &str, member: &str) -> Result<Vec<String>, Fault> {
        let (code, reply) = self.reply(what, member)?;
        if code != want {
            let line = reply.last().map_or("", String::as_str);
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!("{member}: the endpoint refused {what} — {line}"),
            ));
        }
        Ok(reply)
    }

    /// Reads one whole reply, and answers with its code and its lines.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` for a closed connection, a reply that is not
    /// three digits, and a reply past [`REPLY_LINES`].
    fn reply(&mut self, what: &str, member: &str) -> Result<(i64, Vec<String>), Fault> {
        let mut reply = Vec::new();
        loop {
            if reply.len() == REPLY_LINES {
                return Err(Fault::thrown_as(
                    ThrownClass::Io,
                    format!(
                        "{member}: the reply to {what} has more than the {REPLY_LINES} lines \
                         an SMTP reply may have"
                    ),
                ));
            }
            let line = self.line(what, member)?;
            let code = line
                .get(..3)
                .and_then(|digits| digits.parse::<i64>().ok())
                .ok_or_else(|| {
                    Fault::thrown_as(
                        ThrownClass::Io,
                        format!(
                            "{member}: the endpoint answered {what} with `{line}`, which is \
                                 not an SMTP reply"
                        ),
                    )
                })?;
            // A continuation line writes `250-`; the last one writes `250 `.
            if line.as_bytes().get(3) != Some(&b'-') {
                reply.push(line);
                return Ok((code, reply));
            }
            reply.push(line);
        }
    }

    /// The next CRLF-terminated line, without its terminator.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the connection closes before one arrives.
    fn line(&mut self, what: &str, member: &str) -> Result<String, Fault> {
        loop {
            let found = self.held.iter().position(|byte| *byte == b'\n');
            if found.unwrap_or(self.held.len()) > REPLY_LINE_CEILING {
                return Err(Fault::thrown_as(
                    ThrownClass::Io,
                    format!(
                        "{member}: a line of the reply to {what} is longer than the \
                         {REPLY_LINE_CEILING} bytes an SMTP reply line may be"
                    ),
                ));
            }
            if let Some(end) = found {
                let mut line: Vec<u8> = self.held.drain(..=end).collect();
                while line
                    .last()
                    .is_some_and(|byte| *byte == b'\n' || *byte == b'\r')
                {
                    line.pop();
                }
                return Ok(String::from_utf8_lossy(&line).into_owned());
            }
            let mut chunk = [0_u8; 512];
            let read = self.stream.read(&mut chunk).map_err(|why| {
                Fault::thrown_as(
                    ThrownClass::Io,
                    format!("{member} could not read the reply to {what}: {why}"),
                )
            })?;
            if read == 0 {
                return Err(Fault::thrown_as(
                    ThrownClass::Io,
                    format!("{member}: the endpoint closed the connection during {what}"),
                ));
            }
            self.held.extend_from_slice(&chunk[..read]);
        }
    }
}

/// The longest reply line [`Session::line`] reads, terminator excluded.
///
/// RFC 5321 § 4.5.3.1.5 allows 512 octets with the terminator. The cap is
/// eight times that, so an endpoint that is a little over the limit still
/// works, and one that never ends a line is refused at 4 KiB.
const REPLY_LINE_CEILING: usize = 4096;

/// The most lines one reply may have.
///
/// An `EHLO` reply is one line per extension, and endpoints advertise about
/// twenty. With [`REPLY_LINE_CEILING`] this caps a reply at 1 MiB, which an
/// endpoint that sends continuation lines forever runs into.
const REPLY_LINES: usize = 256;

/// The parameters `keyword` was advertised with in an `EHLO` reply, or `None`
/// where the endpoint did not advertise it at all.
///
/// The reply's first line is the endpoint's own greeting rather than a keyword,
/// so it is skipped; every other line is `250-KEYWORD PARAM PARAM`, and
/// RFC 5321 § 2.4 makes the keyword case-insensitive.
fn advertised<'a>(reply: &'a [String], keyword: &str) -> Option<&'a str> {
    reply.iter().skip(1).find_map(|line| {
        let rest = line.get(4..)?;
        let (word, params) = rest.split_once(' ').unwrap_or((rest, ""));
        word.eq_ignore_ascii_case(keyword).then_some(params)
    })
}

/// RFC 4616's `PLAIN` message, base64'd: an empty authorization identity — so
/// the endpoint uses the authentication one — then the user and the password,
/// NUL-separated.
fn plain(user: &str, password: &str) -> String {
    STANDARD.encode(format!("\0{user}\0{password}"))
}

/// Whether `mechanism` is among those the `AUTH` line of `extensions` offers.
fn offers(extensions: &[String], mechanism: &str) -> bool {
    advertised(extensions, "AUTH").is_some_and(|mechanisms| {
        mechanisms
            .split_ascii_whitespace()
            .any(|offered| offered.eq_ignore_ascii_case(mechanism))
    })
}

/// Sends the credential with the block's `auth`, or with `PLAIN` when the server offers it and
/// `LOGIN` when it offers only that.
///
/// # Errors
///
/// A catchable `IOError` when the server does not offer the mechanism the block names, offers
/// neither, or refuses the credential. The credential never reaches the message.
fn authenticate(
    session: &mut Session,
    endpoint: &Endpoint,
    extensions: &[String],
    (user, password): (&str, &str),
    member: &str,
) -> Result<(), Fault> {
    let host = &endpoint.host;
    let mechanism = match endpoint.auth {
        Some(Auth::Plain) if offers(extensions, "PLAIN") => Auth::Plain,
        Some(Auth::Login) if offers(extensions, "LOGIN") => Auth::Login,
        Some(named) => {
            let word = named.word();
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member}: `[mail]` sends the credential with `AUTH {word}`, and {host} does \
                     not offer it. Nothing was sent."
                ),
            ));
        }
        None if offers(extensions, "PLAIN") => Auth::Plain,
        None if offers(extensions, "LOGIN") => Auth::Login,
        None => {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member}: {host} offers neither `AUTH PLAIN` nor `AUTH LOGIN` over TLS. \
                     Nothing was sent."
                ),
            ));
        }
    };
    match mechanism {
        Auth::Plain => {
            session.command(
                &format!("AUTH PLAIN {}", plain(user, password)),
                235,
                "the credential",
                member,
            )?;
        }
        Auth::Login | Auth::XOAuth2 => {
            session.command("AUTH LOGIN", 334, "the login", member)?;
            session.command(&STANDARD.encode(user), 334, "the user", member)?;
            session.command(&STANDARD.encode(password), 235, "the credential", member)?;
        }
    }
    Ok(())
}

/// The `XOAUTH2` message, base64'd: the user and the bearer token, each ended by a `\x01`, and one
/// more `\x01` at the end.
fn xoauth2(user: &str, token: &str) -> String {
    STANDARD.encode(format!("user={user}\x01auth=Bearer {token}\x01\x01"))
}

/// Signs `user` in with `AUTH XOAUTH2` and the block's `token`.
///
/// A token the server refuses is dropped from [`super::oauth`]'s cache, so the next send fetches
/// a new one. The session that failed is closed, never kept.
///
/// # Errors
///
/// A catchable `IOError` when the server does not offer `XOAUTH2` or refuses the token. The token
/// never reaches the message.
fn bearer(
    session: &mut Session,
    endpoint: &Endpoint,
    extensions: &[String],
    (user, token): (&str, &str),
    member: &str,
) -> Result<(), Fault> {
    if !offers(extensions, "XOAUTH2") {
        return Err(Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{member}: `[mail.{}]` signs in with `AUTH XOAUTH2`, and {} does not offer it. \
                 Nothing was sent.",
                endpoint.name, endpoint.host
            ),
        ));
    }
    let line = format!("AUTH XOAUTH2 {}", xoauth2(user, token));
    session
        .command(&line, 235, "the token", member)
        .map(drop)
        .inspect_err(|_| super::oauth::forget(&endpoint.name))
}

/// The `SIZE` limit in `extensions`, when the server advertises one that is not zero.
///
/// RFC 1870 makes `SIZE 0` and a bare `SIZE` mean "no fixed limit".
fn size_limit(extensions: &[String]) -> Option<u64> {
    advertised(extensions, "SIZE")
        .and_then(|limit| limit.trim().parse::<u64>().ok())
        .filter(|limit| *limit > 0)
}

/// The `MAIL FROM` command for `from` and `recipients`, given the `extensions` the session's
/// last `EHLO` advertised.
///
/// An address whose local part is not ASCII needs RFC 6531's `SMTPUTF8`, which the command then
/// asks for. Every address reaching here came through [`super::compose::address_of`], so its
/// domain is already ASCII.
///
/// # Errors
///
/// A catchable `IOError` naming the first such address when `host` does not advertise
/// `SMTPUTF8`. Nothing has been sent but the greeting.
fn envelope_sender(
    from: &str,
    recipients: &[String],
    extensions: &[String],
    host: &str,
    member: &str,
) -> Result<String, Fault> {
    let Some(international) = std::iter::once(from)
        .chain(recipients.iter().map(String::as_str))
        .find(|address| !address.is_ascii())
    else {
        return Ok(format!("MAIL FROM:<{from}>"));
    };
    if advertised(extensions, "SMTPUTF8").is_none() {
        return Err(Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{member}: the address {international} is not ASCII, and {host} does not offer \
                 `SMTPUTF8`, which such an address needs. Nothing was sent."
            ),
        ));
    }
    Ok(format!("MAIL FROM:<{from}> SMTPUTF8"))
}

/// The whole exchange over a new connection that is closed after it, once every argument has been
/// checked and every deployment question answered. Every wait in it ends at `until`.
///
/// # Errors
///
/// [`open`]'s and [`transact`]'s. In each case no `DATA` is sent.
pub(super) fn deliver(
    address: SocketAddr,
    endpoint: &Endpoint,
    until: Instant,
    envelope: &Envelope<'_>,
    message: &str,
    member: &str,
) -> Result<(), Fault> {
    let mut session = open(address, endpoint, until, member)?;
    let sent = transact(&mut session, endpoint, envelope, message, member);
    session.quit();
    sent
}

/// A session ready for its first message: connected, greeted, secured where the block asks for
/// TLS, and signed in where it has a credential. Every wait in it ends at `until`.
///
/// # Errors
///
/// A catchable `IOError` for anything the endpoint said or did — see [`Session`] — and for an
/// endpoint that cannot carry the TLS the block asks for.
pub(super) fn open(
    address: SocketAddr,
    endpoint: &Endpoint,
    until: Instant,
    member: &str,
) -> Result<Session, Fault> {
    // The token is fetched before the relay is reached, so a refused token leaves the relay
    // untouched.
    let token = match &endpoint.oauth {
        Some(grant) => Some(super::oauth::token(grant, &endpoint.name, until, member)?),
        None => None,
    };
    let mut session = Session::open(address, endpoint, until, member)?;
    // The domain the operator's own `from` names, so nothing about the host
    // serving the request is announced to the relay.
    let domain = endpoint
        .from
        .split_once('@')
        .map_or("localhost", |(_, d)| d);
    let greeting = format!("EHLO {domain}");
    let mut extensions = session.command(&greeting, 250, "the greeting", member)?;

    if endpoint.tls == Tls::StartTls {
        if advertised(&extensions, "STARTTLS").is_none() {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member}: `[mail]` needs TLS for {} and that endpoint offers no \
                     `STARTTLS`. Nothing was sent.",
                    endpoint.host
                ),
            ));
        }
        session = session.secure(endpoint, member)?;
        // A second `EHLO`, because the extension list belongs to the session and
        // this is a new one — an endpoint that offers `AUTH` only once the
        // stream is secured is the ordinary case, not an unusual one.
        extensions = session.command(&greeting, 250, "the greeting", member)?;
    }

    if endpoint.credential.is_some() || token.is_some() {
        // `endpoint_of` refuses a credential with `tls = "none"`. This is the
        // second lock on the same door, at the place the password is written.
        if !session.secured() {
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!(
                    "{member}: the session with {} is not secured, and a password is never sent \
                     without TLS. Nothing was sent.",
                    endpoint.host
                ),
            ));
        }
    }
    if let Some((user, password)) = &endpoint.credential {
        authenticate(
            &mut session,
            endpoint,
            &extensions,
            (user, password),
            member,
        )?;
    }
    if let (Some(grant), Some(token)) = (&endpoint.oauth, &token) {
        bearer(
            &mut session,
            endpoint,
            &extensions,
            (&grant.user, token),
            member,
        )?;
    }
    session.extensions = extensions;
    Ok(session)
}

/// Ends the transaction a kept session may still have open with `RSET`, so the next message
/// starts clean. Every wait ends at `until`.
///
/// # Errors
///
/// A catchable `IOError` when the server closed the session or did not answer `250`. The caller
/// then opens a new one.
pub(super) fn reset(session: &mut Session, until: Instant, member: &str) -> Result<(), Fault> {
    session.set_deadline(until);
    session.command("RSET", 250, "the reset", member).map(drop)
}

/// One message over an open session: the `SIZE` check, the envelope and `DATA`.
///
/// # Errors
///
/// A catchable `IOError` for anything the endpoint said or did, for a message over the advertised
/// `SIZE`, and for a rejected recipient, naming every one. In each of these cases no `DATA` is
/// sent.
pub(super) fn transact(
    session: &mut Session,
    endpoint: &Endpoint,
    envelope: &Envelope<'_>,
    message: &str,
    member: &str,
) -> Result<(), Fault> {
    let extensions = std::mem::take(&mut session.extensions);
    let sent = transaction(session, endpoint, &extensions, envelope, message, member);
    session.extensions = extensions;
    sent
}

/// [`transact`], with the extensions taken out of the session so the session can be borrowed
/// mutably beside them.
fn transaction(
    session: &mut Session,
    endpoint: &Endpoint,
    extensions: &[String],
    envelope: &Envelope<'_>,
    message: &str,
    member: &str,
) -> Result<(), Fault> {
    // `DATA` sends the message and the five bytes that end it.
    let size = u64::try_from(message.len() + 5).unwrap_or(u64::MAX);
    if let Some(limit) = size_limit(extensions).filter(|limit| size > *limit) {
        return Err(Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{member}: the message is {size} bytes, and {} accepts at most {limit}. Nothing \
                 was sent.",
                endpoint.host
            ),
        ));
    }

    let mail_from = envelope_sender(
        envelope.sender,
        envelope.recipients,
        extensions,
        &endpoint.host,
        member,
    )?;
    session.command(&mail_from, 250, "the envelope sender", member)?;
    let mut rejected = Vec::new();
    for recipient in envelope.recipients {
        let (code, reply) =
            session.ask(&format!("RCPT TO:<{recipient}>"), "a recipient", member)?;
        // 251 is "not local, will forward", which is an acceptance.
        if code != 250 && code != 251 {
            let said = reply.last().map_or("", String::as_str);
            rejected.push(format!("{recipient} ({said})"));
        }
    }
    if !rejected.is_empty() {
        // The transaction ends here, so the server forgets the recipients it
        // did accept and nobody gets part of a message.
        let _ = session.command("RSET", 250, "the reset", member);
        return Err(Fault::thrown_as(
            ThrownClass::Io,
            format!(
                "{member}: {} rejected {} recipient(s): {}. Nothing was sent.",
                endpoint.host,
                rejected.len(),
                rejected.join(", ")
            ),
        ));
    }
    session.command("DATA", 354, "the body", member)?;
    session
        .stream
        .write_all(message.as_bytes())
        .and_then(|()| session.stream.write_all(b"\r\n.\r\n"))
        .and_then(|()| session.stream.flush())
        .map_err(|why| {
            Fault::thrown_as(
                ThrownClass::Io,
                format!("{member} could not send the message: {why}"),
            )
        })?;
    session.expect(250, "the message", member)?;
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use super::*;

    /// The extension reader, over the three things an `EHLO` reply can do to
    /// it: name the endpoint on its first line, spell a keyword in whatever
    /// case it likes, and carry parameters after it.
    ///
    /// The greeting line matters because it is the one line that is *not* a
    /// keyword: a reader that did not skip it would read an endpoint calling
    /// itself `starttls.example.com` as an offer of `STARTTLS`, and offer to
    /// upgrade a connection nothing would answer.
    #[test]
    fn the_ehlo_reply_is_read_as_keywords_after_its_first_line() {
        let reply: Vec<String> = [
            "250-starttls.example.com at your service",
            "250-SIZE 35882577",
            "250-StartTls",
            "250 AUTH LOGIN PLAIN XOAUTH2",
        ]
        .iter()
        .map(|line| (*line).to_owned())
        .collect();

        // Case-insensitive, per RFC 5321 § 2.4, and found on a continuation
        // line as readily as on the last one.
        assert_eq!(advertised(&reply, "STARTTLS"), Some(""));
        assert_eq!(advertised(&reply, "size"), Some("35882577"));
        assert_eq!(advertised(&reply, "AUTH"), Some("LOGIN PLAIN XOAUTH2"));

        // And the greeting is not a keyword, however much it looks like one.
        let greeting_only = vec![reply[0].clone()];
        assert_eq!(advertised(&greeting_only, "STARTTLS"), None);

        // A keyword nobody offered is absent rather than empty.
        assert_eq!(advertised(&reply, "DSN"), None);
    }

    /// A reply line that never ends and a reply that never stops continuing
    /// are both refused at their cap, so an endpoint cannot choose how much a
    /// session holds. The endpoint here reads until the session closes, so the
    /// refusal is never a reset.
    #[test]
    fn a_reply_is_capped_per_line_and_per_reply() {
        let endless_line = "a".repeat(REPLY_LINE_CEILING + 1024);
        let endless_reply = "220-mail.example.com\r\n".repeat(REPLY_LINES + 1);
        for (greeting, refused) in [
            (endless_line, "an SMTP reply line may be"),
            (endless_reply, "lines an SMTP reply may have"),
        ] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
            let address = listener.local_addr().expect("its address");
            let endpoint = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().expect("the session");
                stream.write_all(greeting.as_bytes()).expect("the greeting");
                let mut rest = Vec::new();
                drop(stream.read_to_end(&mut rest));
            });
            let until = Instant::now() + Duration::from_secs(10);
            let Err(fault) = Session::open(address, &block(Tls::None), until, "Core\\Mail::send")
            else {
                panic!("a greeting past its cap was read");
            };
            assert!(format!("{fault:?}").contains(refused), "{fault:?}");
            endpoint.join().expect("the endpoint thread");
        }
    }

    /// RFC 4616's framing, which is three fields and not two: the empty
    /// authorization identity in front is what makes the endpoint authorize as
    /// whoever authenticated, and a message missing it is refused by every
    /// server rather than read as a shorter one.
    #[test]
    fn auth_plain_sends_an_empty_authorization_identity_first() {
        let message = plain("postmaster@example.com", "hunter2");
        let decoded = STANDARD.decode(&message).expect("`plain` emits base64");

        assert_eq!(
            decoded, b"\0postmaster@example.com\0hunter2",
            "the PLAIN message is NUL-separated with an empty first field"
        );
        // The credential is never on the wire in any other spelling, so the
        // encoded form is the only thing a reply can quote back.
        assert!(!message.contains("hunter2"));
    }

    /// An address whose local part is not ASCII is kept as written, and the send asks for
    /// `SMTPUTF8` when the server offers it. A server that does not offer it gets no `MAIL FROM`,
    /// and the `IOError` names the address. An ASCII envelope never asks.
    // covers: Core\Mail::send
    #[test]
    fn a_non_ascii_local_part_needs_smtputf8() {
        let member = "Core\\Mail::send";
        let jose = super::super::compose::address_of("josé@bücher.example", "to", member)
            .expect("a non-ASCII local part is an address");
        assert_eq!(jose, "josé@xn--bcher-kva.example");

        let with = ["250-mail.example.com".to_owned(), "250 SMTPUTF8".to_owned()];
        let without = ["250-mail.example.com".to_owned(), "250 8BITMIME".to_owned()];
        let ascii = ["ada@example.com".to_owned()];
        let international = ["ada@example.com".to_owned(), jose.clone()];

        let line = |from: &str, to: &[String], offered: &[String]| {
            envelope_sender(from, to, offered, "mail.example.com", member)
        };
        assert_eq!(
            line("noreply@example.com", &ascii, &without)
                .ok()
                .as_deref(),
            Some("MAIL FROM:<noreply@example.com>")
        );
        assert_eq!(
            line("noreply@example.com", &ascii, &with).ok().as_deref(),
            Some("MAIL FROM:<noreply@example.com>")
        );
        assert_eq!(
            line("noreply@example.com", &international, &with)
                .ok()
                .as_deref(),
            Some("MAIL FROM:<noreply@example.com> SMTPUTF8")
        );
        assert_eq!(
            line("josé@xn--bcher-kva.example", &ascii, &with)
                .ok()
                .as_deref(),
            Some("MAIL FROM:<josé@xn--bcher-kva.example> SMTPUTF8")
        );
        let Err(fault) = line("noreply@example.com", &international, &without) else {
            panic!("a non-ASCII address was sent to a server without SMTPUTF8");
        };
        let said = format!("{fault:?}");
        assert!(
            said.contains("josé@xn--bcher-kva.example") && said.contains("SMTPUTF8"),
            "{said}"
        );
    }

    /// A `[mail.relay]` block that reaches `localhost` with `tls`, and nothing else set.
    pub(in crate::mail) fn block(tls: Tls) -> Endpoint {
        Endpoint {
            name: "relay".to_owned(),
            host: "localhost".to_owned(),
            port: 0,
            from: "noreply@example.com".to_owned(),
            from_name: None,
            timeout: Duration::from_secs(10),
            credential: None,
            oauth: None,
            tls,
            ca_file: None,
            auth: None,
            return_path: "noreply@example.com".to_owned(),
        }
    }

    /// A stream the fake server reads and writes, plain or TLS.
    trait Duplex: Read + Write {}
    impl<T: Read + Write> Duplex for T {}

    /// What the fake server does: TLS from the first byte or after `STARTTLS`, the extensions its
    /// `EHLO` reply lists, the recipients it rejects, and whether it closes the connection after
    /// the first message it accepts.
    #[derive(Default)]
    pub(in crate::mail) struct Fake {
        pub(in crate::mail) implicit: bool,
        pub(in crate::mail) starttls: bool,
        pub(in crate::mail) extensions: &'static [&'static str],
        pub(in crate::mail) reject: &'static [&'static str],
        pub(in crate::mail) certificate: Option<Arc<rustls::ServerConfig>>,
        pub(in crate::mail) close_after_message: bool,
    }

    /// A self-signed certificate for `localhost`, the server's configuration that presents it, and
    /// a scratch PEM file holding it for a `tls_ca_file`.
    pub(in crate::mail) fn localhost() -> (Arc<rustls::ServerConfig>, nvs_repo::Scratch, PathBuf) {
        let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .expect("a loopback certificate");
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .expect("the shipped versions")
        .with_no_client_auth()
        .with_single_cert(
            vec![issued.cert.der().clone()],
            rustls::pki_types::PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(
                issued.signing_key.serialize_der(),
            )),
        )
        .expect("the certificate and its key");
        let dir = nvs_repo::scratch("mail-smtp");
        let path = dir.join("ca.pem");
        std::fs::write(&path, issued.cert.pem()).expect("the bundle");
        (Arc::new(config), dir, path)
    }

    fn say(wire: &mut dyn Duplex, line: &str) -> std::io::Result<()> {
        wire.write_all(format!("{line}\r\n").as_bytes())?;
        wire.flush()
    }

    fn hear(wire: &mut dyn Duplex) -> Option<String> {
        let mut line = Vec::new();
        let mut byte = [0_u8; 1];
        loop {
            match wire.read(&mut byte) {
                Ok(1) if byte[0] == b'\n' => break,
                Ok(1) => line.push(byte[0]),
                _ => return None,
            }
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        Some(String::from_utf8_lossy(&line).into_owned())
    }

    /// One SMTP session on a loopback port, served as `fake` says. The thread returns every line
    /// the client sent, with a message body written as `<body>`.
    pub(in crate::mail) fn serve(fake: Fake) -> (SocketAddr, std::thread::JoinHandle<Vec<String>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        let thread = std::thread::spawn(move || match listener.accept() {
            Ok((socket, _)) => converse(&fake, &socket),
            Err(_) => Vec::new(),
        });
        (address, thread)
    }

    /// A fake server on a loopback port that serves every connection as `fake` says, each on a
    /// thread of its own, until [`Many::stop`].
    pub(in crate::mail) struct Many {
        pub(in crate::mail) address: SocketAddr,
        stop: Arc<AtomicBool>,
        acceptor: std::thread::JoinHandle<Vec<Vec<String>>>,
    }

    impl Many {
        /// Stops accepting and returns the lines of every connection the client opened, in the
        /// order they were accepted. A session the client still holds open must be closed
        /// first, or its thread waits for it.
        pub(in crate::mail) fn stop(self) -> Vec<Vec<String>> {
            self.stop.store(true, Ordering::SeqCst);
            // The acceptor is blocked in `accept`, and this connection wakes it.
            drop(std::net::TcpStream::connect(self.address));
            self.acceptor.join().expect("the acceptor")
        }
    }

    pub(in crate::mail) fn serve_many(fake: Fake) -> Many {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let fake = Arc::new(fake);
        let acceptor = std::thread::spawn(move || {
            let mut sessions = Vec::new();
            while let Ok((socket, _)) = listener.accept() {
                if stopped.load(Ordering::SeqCst) {
                    break;
                }
                let fake = Arc::clone(&fake);
                sessions.push(std::thread::spawn(move || converse(&fake, &socket)));
            }
            sessions
                .into_iter()
                .map(|session| session.join().expect("a session"))
                .collect()
        });
        Many {
            address,
            stop,
            acceptor,
        }
    }

    /// One SMTP conversation over `socket`, served as `fake` says, and every line the client sent.
    fn converse(fake: &Fake, socket: &std::net::TcpStream) -> Vec<String> {
        {
            let mut seen = Vec::new();
            drop(socket.set_nonblocking(false));
            drop(socket.set_read_timeout(Some(Duration::from_secs(10))));
            let secure = |socket: &std::net::TcpStream| -> Option<Box<dyn Duplex>> {
                let config = Arc::clone(fake.certificate.as_ref()?);
                let session = rustls::ServerConnection::new(config).ok()?;
                Some(Box::new(rustls::StreamOwned::new(
                    session,
                    socket.try_clone().ok()?,
                )))
            };
            let mut wire: Box<dyn Duplex> = if fake.implicit {
                match secure(socket) {
                    Some(wire) => wire,
                    None => return seen,
                }
            } else {
                Box::new(socket.try_clone().expect("a second handle"))
            };
            let mut secured = fake.implicit;
            if say(&mut *wire, "220 fake.example ready").is_err() {
                return seen;
            }
            while let Some(line) = hear(&mut *wire) {
                seen.push(line.clone());
                let upper = line.to_ascii_uppercase();
                let reply = if upper.starts_with("EHLO") {
                    let mut words = vec!["fake.example"];
                    if fake.starttls && !secured {
                        words.push("STARTTLS");
                    }
                    words.extend(fake.extensions);
                    let last = words.len() - 1;
                    words
                        .iter()
                        .enumerate()
                        .map(|(at, word)| {
                            format!("250{}{word}", if at == last { ' ' } else { '-' })
                        })
                        .collect::<Vec<_>>()
                        .join("\r\n")
                } else if upper == "STARTTLS" {
                    if say(&mut *wire, "220 go ahead").is_err() {
                        break;
                    }
                    match secure(socket) {
                        Some(upgraded) => {
                            wire = upgraded;
                            secured = true;
                            continue;
                        }
                        None => break,
                    }
                } else if upper == "AUTH LOGIN" {
                    let mut answers = Vec::new();
                    for prompt in ["334 VXNlcm5hbWU6", "334 UGFzc3dvcmQ6"] {
                        if say(&mut *wire, prompt).is_err() {
                            break;
                        }
                        let Some(answer) = hear(&mut *wire) else {
                            break;
                        };
                        answers.push(answer);
                    }
                    seen.extend(answers);
                    "235 accepted".to_owned()
                } else if upper.starts_with("AUTH ") {
                    "235 accepted".to_owned()
                } else if upper.starts_with("RCPT TO:") {
                    let address = line[8..].trim_matches(|c| c == '<' || c == '>');
                    if fake.reject.contains(&address) {
                        "550 no such user".to_owned()
                    } else {
                        "250 ok".to_owned()
                    }
                } else if upper == "DATA" {
                    if say(&mut *wire, "354 go ahead").is_err() {
                        break;
                    }
                    while hear(&mut *wire).is_some_and(|body| body != ".") {}
                    seen.push("<body>".to_owned());
                    if fake.close_after_message {
                        drop(say(&mut *wire, "250 queued"));
                        break;
                    }
                    "250 queued".to_owned()
                } else if upper == "QUIT" {
                    drop(say(&mut *wire, "221 bye"));
                    break;
                } else {
                    "250 ok".to_owned()
                };
                if say(&mut *wire, &reply).is_err() {
                    break;
                }
            }
            seen
        }
    }

    /// [`deliver`] of a short message to `ada@example.com` from `sender`.
    pub(in crate::mail) fn send_to(
        address: SocketAddr,
        endpoint: &Endpoint,
        sender: &str,
        recipients: &[String],
        message: &str,
    ) -> Result<(), Fault> {
        deliver(
            address,
            endpoint,
            Instant::now() + Duration::from_secs(10),
            &Envelope { sender, recipients },
            message,
            "Core\\Mail::send",
        )
    }

    pub(in crate::mail) fn ada() -> Vec<String> {
        vec!["ada@example.com".to_owned()]
    }

    pub(in crate::mail) const MESSAGE: &str = "Subject: hi\r\n\r\nhello";

    /// A certificate from an issuer the block does not trust gives an `IOError` that names
    /// `[mail.<name>] tls_ca_file` as the fix, and no `MAIL FROM` is sent.
    // covers: Core\Mail::send
    #[test]
    fn an_unknown_issuer_names_the_tls_ca_file() {
        let (certificate, _dir, _ca) = localhost();
        let (address, server) = serve(Fake {
            starttls: true,
            certificate: Some(certificate),
            ..Fake::default()
        });
        let endpoint = block(Tls::StartTls);
        let Err(fault) = send_to(address, &endpoint, "noreply@example.com", &ada(), MESSAGE) else {
            panic!("a certificate from an unknown issuer was accepted");
        };
        let said = format!("{fault:?}");
        assert!(
            said.contains("`[mail.relay] tls_ca_file`") && said.contains("does not trust"),
            "{said}"
        );
        let seen = server.join().expect("the server");
        assert!(
            !seen.iter().any(|line| line.starts_with("MAIL")),
            "{seen:?}"
        );
    }

    /// `tls = "implicit"`, the default on port 465, starts the handshake before the server's
    /// greeting. A server that answers in plain text gets no `EHLO`.
    // covers: Core\Mail::send
    #[test]
    fn port_465_speaks_tls_from_the_first_byte() {
        let (certificate, _dir, ca) = localhost();
        let mut endpoint = block(Tls::Implicit);
        endpoint.ca_file = Some(ca);
        let (address, server) = serve(Fake {
            implicit: true,
            certificate: Some(certificate),
            ..Fake::default()
        });
        send_to(address, &endpoint, "noreply@example.com", &ada(), MESSAGE)
            .expect("the message is sent over TLS");
        let seen = server.join().expect("the server");
        assert_eq!(seen.first().map(String::as_str), Some("EHLO example.com"));
        assert!(seen.contains(&"<body>".to_owned()), "{seen:?}");

        assert_eq!(
            nvs_config::mail::tls_from(None, 465, false),
            Some(Tls::Implicit)
        );
        let (address, server) = serve(Fake::default());
        let sent = send_to(address, &endpoint, "noreply@example.com", &ada(), MESSAGE);
        assert!(sent.is_err(), "a plain server was read as TLS");
        let seen = server.join().expect("the server");
        assert!(
            !seen.iter().any(|line| line.starts_with("EHLO")),
            "{seen:?}"
        );
    }

    /// `tls = "starttls"` with no credential still needs TLS: a server without `STARTTLS` gets no
    /// `MAIL FROM`, and a certificate the built-in anchors do not sign stops the send.
    // covers: Core\Mail::send
    #[test]
    fn starttls_without_a_credential_is_required_and_verified() {
        let endpoint = block(Tls::StartTls);
        let (address, server) = serve(Fake::default());
        let Err(fault) = send_to(address, &endpoint, "noreply@example.com", &ada(), MESSAGE) else {
            panic!("a server with no STARTTLS was sent the message");
        };
        assert!(format!("{fault:?}").contains("STARTTLS"), "{fault:?}");
        let seen = server.join().expect("the server");
        assert!(
            !seen.iter().any(|line| line.starts_with("MAIL")),
            "{seen:?}"
        );

        let (certificate, _dir, _ca) = localhost();
        let (address, server) = serve(Fake {
            starttls: true,
            certificate: Some(certificate),
            ..Fake::default()
        });
        let Err(fault) = send_to(address, &endpoint, "noreply@example.com", &ada(), MESSAGE) else {
            panic!("a certificate nobody signed was accepted");
        };
        assert!(format!("{fault:?}").contains("TLS handshake"), "{fault:?}");
        let seen = server.join().expect("the server");
        assert!(
            !seen.iter().any(|line| line.starts_with("MAIL")),
            "{seen:?}"
        );
    }

    /// A `tls_ca_file` makes the block trust the certificate it names. It replaces the built-in
    /// anchors, so a file that names another certificate fails.
    // covers: Core\Mail::send
    #[test]
    fn a_tls_ca_file_replaces_the_anchors_for_one_block() {
        let (certificate, _dir, ca) = localhost();
        let (_other, _other_dir, other_ca) = localhost();
        let mut endpoint = block(Tls::StartTls);
        endpoint.ca_file = Some(ca);
        let (address, server) = serve(Fake {
            starttls: true,
            certificate: Some(Arc::clone(&certificate)),
            ..Fake::default()
        });
        send_to(address, &endpoint, "noreply@example.com", &ada(), MESSAGE)
            .expect("the private CA signs the certificate");
        let seen = server.join().expect("the server");
        assert!(seen.contains(&"<body>".to_owned()), "{seen:?}");

        endpoint.ca_file = Some(other_ca);
        let (address, server) = serve(Fake {
            starttls: true,
            certificate: Some(certificate),
            ..Fake::default()
        });
        assert!(
            send_to(address, &endpoint, "noreply@example.com", &ada(), MESSAGE).is_err(),
            "a CA file of another certificate was accepted"
        );
        drop(server.join());
    }

    /// A server that offers only `LOGIN` gets the user and the password, each base64 on its own
    /// line.
    // covers: Core\Mail::send
    #[test]
    fn auth_login_is_used_when_plain_is_not_offered() {
        let (certificate, _dir, ca) = localhost();
        let mut endpoint = block(Tls::Implicit);
        endpoint.ca_file = Some(ca);
        endpoint.credential = Some(("app".to_owned(), "hunter2".to_owned()));
        let (address, server) = serve(Fake {
            implicit: true,
            extensions: &["AUTH LOGIN"],
            certificate: Some(certificate),
            ..Fake::default()
        });
        send_to(address, &endpoint, "noreply@example.com", &ada(), MESSAGE)
            .expect("the credential is sent with LOGIN");
        let seen = server.join().expect("the server");
        let login = seen
            .iter()
            .position(|line| line == "AUTH LOGIN")
            .expect("AUTH LOGIN was sent");
        assert_eq!(seen[login + 1], STANDARD.encode("app"));
        assert_eq!(seen[login + 2], STANDARD.encode("hunter2"));
        assert!(!seen.iter().any(|line| line.starts_with("AUTH PLAIN")));
    }

    /// A message larger than the advertised `SIZE` throws an `IOError` with both sizes, and the
    /// server gets no `MAIL FROM` and no `DATA`.
    // covers: Core\Mail::send
    #[test]
    fn a_message_over_the_advertised_size_sends_no_data() {
        let (address, server) = serve(Fake {
            extensions: &["SIZE 100"],
            ..Fake::default()
        });
        let message = "x".repeat(495);
        let Err(fault) = send_to(
            address,
            &block(Tls::None),
            "noreply@example.com",
            &ada(),
            &message,
        ) else {
            panic!("a message over SIZE was sent");
        };
        let said = format!("{fault:?}");
        assert!(
            said.contains("500 bytes") && said.contains("at most 100"),
            "{said}"
        );
        let seen = server.join().expect("the server");
        assert!(
            !seen
                .iter()
                .any(|line| line.starts_with("MAIL") || line == "DATA"),
            "{seen:?}"
        );
    }

    /// Two rejected recipients out of three: the error names both, the server gets `RSET`, and no
    /// `DATA` is sent, so the accepted one gets nothing either.
    // covers: Core\Mail::send
    #[test]
    fn a_rejected_recipient_is_named_and_nothing_is_sent() {
        let (address, server) = serve(Fake {
            reject: &["bob@example.com", "eve@example.com"],
            ..Fake::default()
        });
        let recipients = [
            "bob@example.com".to_owned(),
            "ada@example.com".to_owned(),
            "eve@example.com".to_owned(),
        ];
        let Err(fault) = send_to(
            address,
            &block(Tls::None),
            "noreply@example.com",
            &recipients,
            MESSAGE,
        ) else {
            panic!("a message with rejected recipients was sent");
        };
        let said = format!("{fault:?}");
        assert!(
            said.contains("bob@example.com") && said.contains("eve@example.com"),
            "{said}"
        );
        assert!(!said.contains("ada@example.com"), "{said}");
        let seen = server.join().expect("the server");
        assert!(seen.contains(&"RSET".to_owned()), "{seen:?}");
        assert!(!seen.contains(&"DATA".to_owned()), "{seen:?}");
    }

    /// `MAIL FROM` is the block's `return_path`, and the message's `From:` header is not touched.
    // covers: Core\Mail::send
    #[test]
    fn the_return_path_is_the_envelope_sender() {
        let mut endpoint = block(Tls::None);
        endpoint.return_path = "bounces@example.com".to_owned();
        let sender = bounce_sender(&endpoint.return_path, None, "Core\\Mail::send")
            .expect("no tag is the return path");
        assert_eq!(sender, "bounces@example.com");
        let (address, server) = serve(Fake::default());
        send_to(address, &endpoint, &sender, &ada(), MESSAGE).expect("the message is sent");
        let seen = server.join().expect("the server");
        assert!(
            seen.contains(&"MAIL FROM:<bounces@example.com>".to_owned()),
            "{seen:?}"
        );
    }

    /// A bounce tag goes after a `+` in the local part of `return_path`.
    // covers: Core\Mail::send
    #[test]
    fn a_bounce_tag_goes_after_a_plus_in_the_return_path() {
        let member = "Core\\Mail::send";
        assert_eq!(
            bounce_sender("bounces@example.com", Some("order-42"), member).ok(),
            Some("bounces+order-42@example.com".to_owned())
        );
        assert_eq!(
            bounce_sender("b@mail.example.com", Some("A.b_C-9"), member).ok(),
            Some("b+A.b_C-9@mail.example.com".to_owned())
        );
    }

    /// A tag that is empty, too long, or has a character outside `A-Z a-z 0-9 . _ -` throws, so a
    /// tag can never change the domain or add a second address.
    // covers: Core\Mail::send
    #[test]
    fn a_bounce_tag_with_another_character_throws() {
        let long = "a".repeat(BOUNCE_TAG_CEILING + 1);
        for tag in [
            "",
            "a b",
            "x@evil.example",
            "a+b",
            "é",
            "a\r\nRCPT",
            "a>b",
            long.as_str(),
        ] {
            let Err(fault) = bounce_sender("bounces@example.com", Some(tag), "Core\\Mail::send")
            else {
                panic!("the bounce tag {tag:?} was accepted");
            };
            assert!(format!("{fault:?}").contains("bounceTag"), "{fault:?}");
        }
    }
}
