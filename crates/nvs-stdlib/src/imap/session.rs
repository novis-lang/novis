//! One IMAP session: the connection, implicit TLS or `STARTTLS`, the sign-in, and the commands
//! `Core\Imap` reads a mailbox with, over [`nvs_host`]'s parking stream.
//!
//! **Every command is written by `imap-codec`.** A folder name, a user, a password and every
//! search string reach the wire as an atom, a quoted string or a literal, chosen by the codec from
//! the bytes, so no value can end a command or start another one. A literal the codec asks to be
//! synchronized waits for the server's `+` before its bytes are sent. Folder names are modified
//! UTF-7 on the wire (RFC 3501 § 5.1.3), and [`utf7`] converts both ways.
//!
//! **Every response is read by `imap-codec` too, within bounds.** A response's lines may take at
//! most [`LINE_CEILING`] bytes beside its literals, and a literal is checked against the ceiling
//! the caller passes before its bytes are read. `fetch` passes the request's remaining memory, so
//! a message that does not fit is an `IOError` before its body arrives. The decoder runs again
//! only once a new line has arrived, so a long response is parsed a bounded number of times.
//!
//! This module knows nothing about configuration or the program. [`super`] reads the block and
//! fetches an `xoauth2` token, and passes the result as a [`Server`] and a [`SignIn`]. Memory: one
//! session per `open`, with its read buffer, freed when the request or run ends.

use std::io::{Read, Write};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::time::Instant;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use imap_codec::decode::{Decoder, ResponseDecodeError};
use imap_codec::encode::{Encoder, Fragment};
use imap_codec::imap_types::auth::AuthMechanism;
use imap_codec::imap_types::command::{Command, CommandBody};
use imap_codec::imap_types::core::{AString, Charset, LiteralMode, NonEmptyVec};
use imap_codec::imap_types::datetime::NaiveDate;
use imap_codec::imap_types::fetch::{MessageDataItem, MessageDataItemName};
use imap_codec::imap_types::flag::FlagNameAttribute;
use imap_codec::imap_types::mailbox::Mailbox;
use imap_codec::imap_types::response::{Capability, Code, Data, GreetingKind, Response, Status};
use imap_codec::imap_types::search::SearchKey;
use imap_codec::{CommandCodec, GreetingCodec, ResponseCodec};
use nvs_config::mail::{Auth, Tls};
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;
use nvs_runtime::{Fault, ThrownClass};

/// The most bytes a response's lines may take, beside its literals: 8 MiB.
///
/// A `UID SEARCH` of a large folder is the longest line a server writes, at about eight bytes
/// per message, so the ceiling reads a folder of a million messages. A server that never ends a
/// line is stopped here.
pub(super) const LINE_CEILING: usize = 8 << 20;

/// The longest literal a response other than a fetched message may carry, such as a folder name
/// sent as a literal, and the longest greeting: 64 KiB.
const SMALL_LITERAL: u64 = 64 << 10;

/// [`SMALL_LITERAL`] as a length.
const GREETING_CEILING: usize = 64 << 10;

/// The most folders [`Session::folders`] keeps.
pub(super) const MAX_FOLDERS: usize = 10_000;

/// What the block said about the server, once [`super`] has checked it.
pub(super) struct Server {
    /// The block's name, `<name>` in `[imap.<name>]`, which a message names.
    pub(super) name: String,
    /// `host`, as written. The certificate is checked against it.
    pub(super) host: String,
    /// `tls`: `implicit` or `starttls`. `none` never reaches here.
    pub(super) tls: Tls,
    /// `tls_ca_file`, which replaces the compiled-in anchors for this block.
    pub(super) ca_file: Option<PathBuf>,
    /// `auth`, or `None` for `AUTHENTICATE PLAIN` where offered, else `LOGIN`.
    pub(super) auth: Option<Auth>,
}

/// How the session signs in.
pub(super) enum SignIn {
    /// `user` and `password`, with `PLAIN` or `LOGIN`.
    Password { user: String, password: String },
    /// `user` and an `xoauth2` token [`crate::mail`]'s oauth module fetched.
    Token { user: String, token: String },
}

/// The filters of `Folder::search`. Every one that is set must match.
#[derive(Default)]
pub(super) struct Filter {
    /// Only messages without `\Seen`.
    pub(super) unseen: bool,
    /// Only messages received on or after this day: year, month, day.
    pub(super) since: Option<(i32, u32, u32)>,
    /// Only messages received before this day.
    pub(super) before: Option<(i32, u32, u32)>,
    /// Only messages whose `From` contains this text.
    pub(super) from: Option<String>,
    /// Only messages whose `Subject` contains this text.
    pub(super) subject: Option<String>,
    /// At most this many UIDs, the oldest first.
    pub(super) limit: Option<usize>,
}

/// What a session talks over: the socket before `STARTTLS`, and TLS after it or from the first
/// byte. The upgrade consumes the socket, so no plaintext stream is left to a secured server.
/// `Secured` is about a kilobyte wider, once per open mailbox, for `smtp`'s reason.
#[allow(clippy::large_enum_variant)]
enum Wire {
    Plain(NvsTcp),
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

/// An open, signed-in IMAP session.
pub(super) struct Session {
    stream: Wire,
    /// Bytes read past the last response.
    held: Vec<u8>,
    /// The number of the next command's tag, `N1`, `N2` and so on.
    next: u32,
    /// The last `CAPABILITY` list, in upper case.
    capabilities: Vec<String>,
    /// The block's name and host, for messages.
    name: String,
    host: String,
}

/// An `IOError` with `message`.
fn io(message: String) -> Fault {
    Fault::thrown_as(ThrownClass::Io, message)
}

/// A TLS session over `stream`, verified against `host` and the block's `tls_ca_file`.
fn handshake(stream: NvsTcp, server: &Server, member: &str) -> Result<NvsTls, Fault> {
    let name = server.host.as_str();
    match &server.ca_file {
        Some(bundle) => NvsTls::over_bundle(stream, name, bundle),
        None => NvsTls::over(stream, name),
    }
    .map_err(|why| {
        let block = format!("imap.{}", server.name);
        let fix = crate::tls_fix::Settings::Block {
            block: &block,
            address: "host",
        };
        io(crate::tls_fix::handshake_failed(member, name, &why, fix))
    })
}

/// What one response was, as far as the command loop needs to know.
enum Seen {
    /// The tagged status that ends the command: `Ok(())` for `OK`, `Err(text)` for `NO` or `BAD`.
    Done(Result<(), String>),
    /// A `+` continuation request.
    More,
    /// Any untagged response, already given to the handler.
    Untagged,
    /// `* BYE`.
    Bye(String),
}

impl Session {
    /// The connected, secured and signed-in session. Every wait ends at `until`.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` for a socket that does not open, a handshake that does not complete,
    /// a greeting that is not `OK`, a server without `STARTTLS` where the block needs it, and a
    /// sign-in the server refuses.
    pub(super) fn open(
        address: SocketAddr,
        server: &Server,
        sign_in: &SignIn,
        until: Instant,
        member: &str,
    ) -> Result<Self, Fault> {
        let budget = until.saturating_duration_since(Instant::now());
        let mut stream = NvsTcp::connect_timeout(address, budget)
            .map_err(|why| io(format!("{member} could not reach {address}: {why}")))?;
        stream.set_deadline(Some(until));
        let stream = if server.tls == Tls::Implicit {
            Wire::Secured(handshake(stream, server, member)?)
        } else {
            Wire::Plain(stream)
        };
        let mut session = Self {
            stream,
            held: Vec::new(),
            next: 1,
            capabilities: Vec::new(),
            name: server.name.clone(),
            host: server.host.clone(),
        };
        session.greeting(member)?;
        session.capability(member)?;
        if server.tls == Tls::StartTls {
            if !session.offers("STARTTLS") {
                return Err(io(format!(
                    "{member}: `[imap.{}]` needs TLS for {} and that server offers no \
                     `STARTTLS`. Nothing was sent.",
                    server.name, server.host
                )));
            }
            session.command(CommandBody::StartTLS, "the upgrade", member, &mut |_| {})?;
            // Bytes sent after the server agreed and before the handshake would be read as if
            // the secured server had sent them, so any is an error.
            if !session.held.is_empty() {
                return Err(io(format!(
                    "{member}: the server sent {} more byte(s) after agreeing to `STARTTLS`",
                    session.held.len()
                )));
            }
            let Wire::Plain(plain) = session.stream else {
                return Err(io(format!("{member}: the session is already secured")));
            };
            session.stream = Wire::Secured(handshake(plain, server, member)?);
            session.capability(member)?;
        }
        if !matches!(session.stream, Wire::Secured(_)) {
            return Err(io(format!(
                "{member}: the session with {} is not secured, and a password is never sent \
                 without TLS. Nothing was sent.",
                server.host
            )));
        }
        session.sign_in(server, sign_in, member)?;
        Ok(session)
    }

    /// Bounds every later wait of the session by `until`.
    pub(super) fn set_deadline(&mut self, until: Instant) {
        match &mut self.stream {
            Wire::Plain(stream) => stream.set_deadline(Some(until)),
            Wire::Secured(stream) => stream.set_deadline(Some(until)),
        }
    }

    /// Ends the session with `LOGOUT`. Its reply is not read.
    pub(super) fn logout(&mut self) {
        let tag = self.tag();
        let _ = write!(self.stream, "{tag} LOGOUT\r\n");
    }

    /// Every folder the server lists, decoded from modified UTF-7, in the server's order.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the server refuses `LIST` or the connection fails.
    pub(super) fn folders(&mut self, member: &str) -> Result<Vec<String>, Fault> {
        let body = CommandBody::list("", "*").map_err(|_| io(format!("{member}: `LIST`")))?;
        let mut names = Vec::new();
        self.command(body, "the folder list", member, &mut |response| {
            if let Response::Data(Data::List { items, mailbox, .. }) = response
                && names.len() < MAX_FOLDERS
                && !items.contains(&FlagNameAttribute::Noselect)
            {
                names.push(name_of(mailbox));
            }
        })?;
        Ok(names)
    }

    /// Selects `folder` for the commands after it, and returns its `UIDVALIDITY`.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the folder does not exist or the server sends no `UIDVALIDITY`.
    pub(super) fn select(&mut self, folder: &str, member: &str) -> Result<u32, Fault> {
        let body = CommandBody::select(mailbox_of(folder, member)?)
            .map_err(|_| io(format!("{member}: the folder name `{folder}`")))?;
        let mut validity = None;
        self.command(body, "the folder", member, &mut |response| {
            if let Response::Status(Status::Ok {
                tag: None,
                code: Some(Code::UidValidity(value)),
                ..
            }) = response
            {
                validity = Some(value.get());
            }
        })
        .map_err(|fault| {
            io(format!(
                "{member}: {} could not open the folder `{folder}`: {}",
                self.host,
                fault_text(&fault)
            ))
        })?;
        validity.ok_or_else(|| {
            io(format!(
                "{member}: {} sent no `UIDVALIDITY` for the folder `{folder}`",
                self.host
            ))
        })
    }

    /// The UIDs in the selected folder that match `filter`, the oldest first.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when the server refuses the search.
    pub(super) fn search(&mut self, filter: &Filter, member: &str) -> Result<Vec<u32>, Fault> {
        let mut keys = Vec::new();
        if filter.unseen {
            keys.push(SearchKey::Unseen);
        }
        for (day, before) in [(filter.since, false), (filter.before, true)] {
            if let Some((year, month, date)) = day {
                let day = chrono::NaiveDate::from_ymd_opt(year, month, date)
                    .and_then(|day| NaiveDate::try_from(day).ok())
                    .ok_or_else(|| io(format!("{member}: the date {year}-{month}-{date}")))?;
                keys.push(if before {
                    SearchKey::Before(day)
                } else {
                    SearchKey::Since(day)
                });
            }
        }
        let mut wide = false;
        for (text, subject) in [(&filter.from, false), (&filter.subject, true)] {
            if let Some(text) = text {
                wide |= !text.is_ascii();
                let text = AString::try_from(text.as_str())
                    .map_err(|_| io(format!("{member}: a search text has a NUL byte")))?;
                keys.push(if subject {
                    SearchKey::Subject(text)
                } else {
                    SearchKey::From(text)
                });
            }
        }
        let criteria = match keys.len() {
            0 => SearchKey::All,
            1 => keys.remove(0),
            _ => SearchKey::And(NonEmptyVec::try_from(keys).map_err(|_| io(member.to_owned()))?),
        };
        let charset = if wide {
            Some(Charset::try_from("UTF-8").map_err(|_| io(member.to_owned()))?)
        } else {
            None
        };
        let mut uids = Vec::new();
        self.command(
            CommandBody::search(charset, criteria, true),
            "the search",
            member,
            &mut |response| {
                if let Response::Data(Data::Search(found)) = response {
                    uids.extend(found.iter().map(|uid| uid.get()));
                }
            },
        )?;
        uids.sort_unstable();
        uids.dedup();
        if let Some(limit) = filter.limit {
            uids.truncate(limit);
        }
        Ok(uids)
    }

    /// The raw bytes of message `uid` in the selected folder, read without setting `\Seen`.
    /// `ceiling` is how many bytes the caller can still hold.
    ///
    /// # Errors
    ///
    /// A catchable `IOError` when there is no message with that UID, and when its `RFC822.SIZE`
    /// is over `ceiling`, before its body is read.
    pub(super) fn fetch(&mut self, uid: u32, ceiling: u64, member: &str) -> Result<Vec<u8>, Fault> {
        let Some(id) = NonZeroU32::new(uid) else {
            return Err(io(format!("{member}: a UID is 1 or more, and this one is 0")));
        };
        let body = CommandBody::fetch(
            id,
            vec![MessageDataItemName::Uid, MessageDataItemName::Rfc822Size],
            true,
        )
        .map_err(|_| io(member.to_owned()))?;
        let mut size = None;
        self.command(body, "the message size", member, &mut |response| {
            if let Response::Data(Data::Fetch { items, .. }) = response
                && items.as_ref().contains(&MessageDataItem::Uid(id))
            {
                for item in items.as_ref() {
                    if let MessageDataItem::Rfc822Size(found) = item {
                        size = Some(u64::from(*found));
                    }
                }
            }
        })?;
        let Some(size) = size else {
            return Err(io(format!(
                "{member}: the folder has no message with the UID {uid}"
            )));
        };
        if size > ceiling {
            return Err(io(format!(
                "{member}: the message with the UID {uid} is {size} bytes, and the request can \
                 hold {ceiling} more. Its body was not read."
            )));
        }
        let peek = MessageDataItemName::BodyExt {
            section: None,
            partial: None,
            peek: true,
        };
        let body = CommandBody::fetch(id, vec![MessageDataItemName::Uid, peek], true)
            .map_err(|_| io(member.to_owned()))?;
        let mut raw = None;
        self.run(body, "the message", ceiling, member, &mut |response| {
            if let Response::Data(Data::Fetch { items, .. }) = response
                && items.as_ref().contains(&MessageDataItem::Uid(id))
            {
                for item in items.as_ref() {
                    if let MessageDataItem::BodyExt { data, .. } = item {
                        raw = data.0.as_ref().map(|data| data.as_ref().to_vec());
                    }
                }
            }
        })?;
        raw.ok_or_else(|| {
            io(format!(
                "{member}: the folder has no message with the UID {uid}"
            ))
        })
    }

    /// The server's greeting, which must be `OK`. `PREAUTH` is refused, because the session
    /// would be signed in as somebody the block did not name.
    fn greeting(&mut self, member: &str) -> Result<(), Fault> {
        loop {
            if let Some(end) = self.held.iter().position(|byte| *byte == b'\n') {
                let kind = GreetingCodec::default()
                    .decode(&self.held[..=end])
                    .map(|(_, greeting)| greeting.kind);
                self.held.drain(..=end);
                return match kind {
                    Ok(GreetingKind::Ok) => Ok(()),
                    Ok(GreetingKind::PreAuth) => Err(io(format!(
                        "{member}: {} signed the session in before the block's user did",
                        self.host
                    ))),
                    _ => Err(io(format!(
                        "{member}: {} did not greet with `* OK`",
                        self.host
                    ))),
                };
            }
            if self.held.len() > GREETING_CEILING {
                return Err(io(format!("{member}: the greeting has no end")));
            }
            self.fill("the greeting", member)?;
        }
    }

    /// `CAPABILITY`, kept in upper case for [`Self::offers`].
    fn capability(&mut self, member: &str) -> Result<(), Fault> {
        let mut found = Vec::new();
        self.command(
            CommandBody::Capability,
            "the capabilities",
            member,
            &mut |response| {
                if let Response::Data(Data::Capability(list)) = response {
                    found.extend(list.as_ref().iter().map(capability_word));
                }
            },
        )?;
        self.capabilities = found;
        Ok(())
    }

    /// Whether the last `CAPABILITY` list has `word`.
    fn offers(&self, word: &str) -> bool {
        self.capabilities.iter().any(|each| each == word)
    }

    /// Signs in as the block says. The credential never reaches a message.
    fn sign_in(&mut self, server: &Server, sign_in: &SignIn, member: &str) -> Result<(), Fault> {
        let refused = |fault: Fault| {
            io(format!(
                "{member}: {} refused the sign-in of `[imap.{}]`: {}",
                server.host,
                server.name,
                fault_text(&fault)
            ))
        };
        match sign_in {
            SignIn::Token { user, token } => {
                if !self.offers("AUTH=XOAUTH2") {
                    return Err(io(format!(
                        "{member}: `[imap.{}]` signs in with `XOAUTH2`, and {} does not offer \
                         it. Nothing was sent.",
                        server.name, server.host
                    )));
                }
                let answer = format!("user={user}\x01auth=Bearer {token}\x01\x01");
                self.authenticate(AuthMechanism::XOAuth2, answer.as_bytes(), member)
                    .map_err(refused)
            }
            SignIn::Password { user, password } => {
                let plain = match server.auth {
                    Some(Auth::Plain) => true,
                    Some(Auth::Login) => false,
                    _ => self.offers("AUTH=PLAIN"),
                };
                if plain {
                    if !self.offers("AUTH=PLAIN") {
                        return Err(io(format!(
                            "{member}: `[imap.{}]` signs in with `PLAIN`, and {} does not \
                             offer it. Nothing was sent.",
                            server.name, server.host
                        )));
                    }
                    let answer = format!("\0{user}\0{password}");
                    return self
                        .authenticate(AuthMechanism::Plain, answer.as_bytes(), member)
                        .map_err(refused);
                }
                if self.offers("LOGINDISABLED") {
                    return Err(io(format!(
                        "{member}: {} does not allow `LOGIN`. Nothing was sent.",
                        server.host
                    )));
                }
                let body = CommandBody::login(user.as_str(), password.as_str())
                    .map_err(|_| io(format!("{member}: the user or password has a NUL byte")))?;
                self.command(body, "the sign-in", member, &mut |_| {})
                    .map_err(refused)
            }
        }
    }

    /// `AUTHENTICATE` with `answer`, sent at once with `SASL-IR` and after the server's `+`
    /// otherwise. A `+` after the answer is an error report, which gets an empty line.
    fn authenticate(
        &mut self,
        mechanism: AuthMechanism<'static>,
        answer: &[u8],
        member: &str,
    ) -> Result<(), Fault> {
        let encoded = STANDARD.encode(answer);
        let initial = self.offers("SASL-IR");
        let body = if initial {
            CommandBody::authenticate_with_ir(mechanism, answer.to_vec())
        } else {
            CommandBody::authenticate(mechanism)
        };
        let tag = self.send(body, "the sign-in", member)?;
        let mut answered = initial;
        loop {
            match self.next_response(&tag, SMALL_LITERAL, "the sign-in", member, &mut |_| {})? {
                Seen::Done(Ok(())) => return Ok(()),
                Seen::Done(Err(text)) => return Err(io(text)),
                Seen::Bye(text) => return Err(io(text)),
                Seen::More if answered => self.write(b"\r\n", "the sign-in", member)?,
                Seen::More => {
                    answered = true;
                    self.write(format!("{encoded}\r\n").as_bytes(), "the sign-in", member)?;
                }
                Seen::Untagged => {}
            }
        }
    }

    /// The next tag, `N<n>`.
    fn tag(&mut self) -> String {
        let tag = format!("N{}", self.next);
        self.next = self.next.wrapping_add(1);
        tag
    }

    /// `body`, run to its tagged status, with each untagged response given to `take`.
    fn command(
        &mut self,
        body: CommandBody<'_>,
        what: &str,
        member: &str,
        take: &mut dyn FnMut(&Response<'_>),
    ) -> Result<(), Fault> {
        self.run(body, what, SMALL_LITERAL, member, take)
    }

    /// [`Self::command`], with literals in the response up to `ceiling` bytes.
    fn run(
        &mut self,
        body: CommandBody<'_>,
        what: &str,
        ceiling: u64,
        member: &str,
        take: &mut dyn FnMut(&Response<'_>),
    ) -> Result<(), Fault> {
        let tag = self.send(body, what, member)?;
        loop {
            match self.next_response(&tag, ceiling, what, member, take)? {
                Seen::Done(Ok(())) => return Ok(()),
                Seen::Done(Err(text)) => {
                    return Err(io(format!("{member}: {} refused {what}: {text}", self.host)));
                }
                Seen::Bye(text) => {
                    return Err(io(format!(
                        "{member}: {} closed the session during {what}: {text}",
                        self.host
                    )));
                }
                Seen::More => {
                    return Err(io(format!(
                        "{member}: {} asked for more during {what}",
                        self.host
                    )));
                }
                Seen::Untagged => {}
            }
        }
    }

    /// Writes `body` with a new tag, and returns the tag. A synchronized literal waits for the
    /// server's `+` before its bytes are written.
    fn send(&mut self, body: CommandBody<'_>, what: &str, member: &str) -> Result<String, Fault> {
        let tag = self.tag();
        let command = Command::new(tag.as_str(), body).map_err(|_| io(member.to_owned()))?;
        for fragment in CommandCodec::default().encode(&command) {
            match fragment {
                Fragment::Line { data } => self.write(&data, what, member)?,
                Fragment::Literal { data, mode } => {
                    if mode == LiteralMode::Sync {
                        loop {
                            match self.next_response(&tag, SMALL_LITERAL, what, member, &mut |_| {})?
                            {
                                Seen::More => break,
                                Seen::Untagged => {}
                                Seen::Done(_) | Seen::Bye(_) => {
                                    return Err(io(format!(
                                        "{member}: {} refused {what}",
                                        self.host
                                    )));
                                }
                            }
                        }
                    }
                    self.write(&data, what, member)?;
                }
            }
        }
        Ok(tag)
    }

    fn write(&mut self, data: &[u8], what: &str, member: &str) -> Result<(), Fault> {
        self.stream
            .write_all(data)
            .and_then(|()| self.stream.flush())
            .map_err(|why| io(format!("{member} could not send {what}: {why}")))
    }

    /// Reads more bytes into `held`.
    fn fill(&mut self, what: &str, member: &str) -> Result<usize, Fault> {
        let mut chunk = [0_u8; 16 << 10];
        let read = self
            .stream
            .read(&mut chunk)
            .map_err(|why| io(format!("{member} could not read {what}: {why}")))?;
        if read == 0 {
            return Err(io(format!(
                "{member}: {} closed the connection during {what}",
                self.host
            )));
        }
        self.held.extend_from_slice(&chunk[..read]);
        Ok(read)
    }

    /// The next response, decoded once enough bytes arrived. An untagged one is given to `take`.
    fn next_response(
        &mut self,
        tag: &str,
        ceiling: u64,
        what: &str,
        member: &str,
        take: &mut dyn FnMut(&Response<'_>),
    ) -> Result<Seen, Fault> {
        // The bytes the decoder needs before it runs again, and the most the response may take.
        let mut want = 0;
        let mut allowed = LINE_CEILING;
        loop {
            if self.held.len() >= want {
                let decoded = ResponseCodec::default().decode(&self.held);
                let (seen, used) = match decoded {
                    Ok((rest, response)) => {
                        let used = self.held.len() - rest.len();
                        (Some(seen_of(&response, tag, take)), used)
                    }
                    Err(ResponseDecodeError::Incomplete) => (None, 0),
                    Err(ResponseDecodeError::LiteralFound { length }) => {
                        if u64::from(length) > ceiling {
                            return Err(io(format!(
                                "{member}: {} sent {length} bytes during {what}, and the request \
                                 can hold {ceiling} more",
                                self.host
                            )));
                        }
                        let length = length as usize;
                        allowed = allowed.saturating_add(length);
                        want = self.held.len().saturating_add(length);
                        (None, 0)
                    }
                    Err(ResponseDecodeError::Failed) => {
                        return Err(io(format!(
                            "{member}: {} sent a response during {what} that is not IMAP",
                            self.host
                        )));
                    }
                };
                if let Some(seen) = seen {
                    self.held.drain(..used);
                    return Ok(seen);
                }
            }
            if self.held.len() > allowed {
                return Err(io(format!(
                    "{member}: a response from {} during {what} is longer than {LINE_CEILING} \
                     bytes",
                    self.host
                )));
            }
            // The decoder runs again only once a new line or the whole literal has arrived.
            let before = self.held.len();
            self.fill(what, member)?;
            if want <= before && !self.held[before..].contains(&b'\n') {
                want = self.held.len() + 1;
            } else if want <= before {
                want = 0;
            }
        }
    }
}

/// What `response` is, for a command tagged `tag`.
fn seen_of(response: &Response<'_>, tag: &str, take: &mut dyn FnMut(&Response<'_>)) -> Seen {
    let ours = |found: &Option<imap_codec::imap_types::core::Tag<'_>>| {
        found.as_ref().is_some_and(|found| found.as_ref() == tag)
    };
    match response {
        Response::Status(Status::Ok { tag: found, .. }) if ours(found) => Seen::Done(Ok(())),
        Response::Status(
            Status::No {
                tag: found, text, ..
            }
            | Status::Bad {
                tag: found, text, ..
            },
        ) if ours(found) => Seen::Done(Err(text.as_ref().to_owned())),
        Response::Status(Status::Bye { text, .. }) => Seen::Bye(text.as_ref().to_owned()),
        Response::CommandContinuationRequest(_) => Seen::More,
        untagged => {
            take(untagged);
            Seen::Untagged
        }
    }
}

/// A capability as one upper-case word: `AUTH=PLAIN`, `STARTTLS`, `SASL-IR`.
fn capability_word(capability: &Capability<'_>) -> String {
    capability.to_string().to_ascii_uppercase()
}

/// A folder name the program wrote, in modified UTF-7 for the wire.
fn mailbox_of(folder: &str, member: &str) -> Result<Mailbox<'static>, Fault> {
    Mailbox::try_from(utf7::encode(folder))
        .map_err(|_| io(format!("{member}: the folder name `{folder}` is not allowed")))
}

/// A listed folder's name, decoded from modified UTF-7.
fn name_of(mailbox: &Mailbox<'_>) -> String {
    match mailbox {
        Mailbox::Inbox => "INBOX".to_owned(),
        Mailbox::Other(other) => utf7::decode(&String::from_utf8_lossy(other.as_ref())),
    }
}

/// The message of a fault, for a longer message that wraps it.
fn fault_text(fault: &Fault) -> String {
    match fault {
        Fault::Thrown(_, message) | Fault::Fatal(message) => message.as_str().to_owned(),
        _ => String::new(),
    }
}

/// Modified UTF-7, the encoding of an IMAP folder name (RFC 3501 § 5.1.3).
///
/// Printable ASCII is itself, `&` is `&-`, and every other run of characters is its UTF-16 code
/// units in base64 with `,` in place of `/`, between `&` and `-`.
pub(super) mod utf7 {
    use base64::Engine as _;
    use base64::alphabet::Alphabet;
    use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};

    /// The base64 of modified UTF-7: `,` for `/` and no padding.
    fn engine() -> GeneralPurpose {
        let alphabet =
            Alphabet::new("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+,")
                .expect("64 distinct characters");
        GeneralPurpose::new(
            &alphabet,
            GeneralPurposeConfig::new()
                .with_encode_padding(false)
                .with_decode_padding_mode(DecodePaddingMode::RequireNone),
        )
    }

    /// `name` in modified UTF-7.
    pub(in crate::imap) fn encode(name: &str) -> String {
        let mut out = String::with_capacity(name.len());
        let mut run: Vec<u16> = Vec::new();
        let flush = |run: &mut Vec<u16>, out: &mut String| {
            if !run.is_empty() {
                let bytes: Vec<u8> = run.iter().flat_map(|unit| unit.to_be_bytes()).collect();
                out.push('&');
                out.push_str(&engine().encode(bytes));
                out.push('-');
                run.clear();
            }
        };
        for character in name.chars() {
            if (' '..='~').contains(&character) {
                flush(&mut run, &mut out);
                out.push_str(if character == '&' { "&-" } else { "" });
                if character != '&' {
                    out.push(character);
                }
            } else {
                let mut units = [0_u16; 2];
                run.extend_from_slice(character.encode_utf16(&mut units));
            }
        }
        flush(&mut run, &mut out);
        out
    }

    /// `name` decoded from modified UTF-7. A part that is not valid stays as it was written.
    pub(in crate::imap) fn decode(name: &str) -> String {
        let mut out = String::with_capacity(name.len());
        let mut rest = name;
        while let Some(start) = rest.find('&') {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            let Some(end) = after.find('-') else {
                out.push_str(&rest[start..]);
                return out;
            };
            let encoded = &after[..end];
            if encoded.is_empty() {
                out.push('&');
            } else {
                let units = engine().decode(encoded).ok().and_then(|bytes| {
                    (bytes.len() % 2 == 0).then(|| {
                        bytes
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|pair| u16::from_be_bytes(*pair))
                            .collect::<Vec<_>>()
                    })
                });
                match units.and_then(|units| String::from_utf16(&units).ok()) {
                    Some(text) => out.push_str(&text),
                    None => out.push_str(&rest[start..=start + 1 + end]),
                }
            }
            rest = &after[end + 1..];
        }
        out.push_str(rest);
        out
    }
}
