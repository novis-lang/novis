//! MySQL: reading the server's greeting, upgrading the socket in band,
//! authenticating with a proof rather than a password, and forcing the
//! connection's charset to `utf8mb4`.
//!
//! [ADR 0132 § 2](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)
//! decides what is here and what is not. `mysql_common` frames every packet,
//! keeps the sequence-id discipline, and owns each authentication plugin's
//! challenge/response arithmetic — that is the borrowed codec. **The sequencing
//! is ours**: which packet may follow which, where the TLS upgrade sits inside
//! the packet stream, and — the part no codec can decide — *which* of the
//! plugins a server may name this driver will answer at all.
//!
//! # Opening is four exchanges, and MySQL's order is the awkward one
//!
//! PostgreSQL asks to upgrade before it says anything; MySQL speaks first. The
//! server's greeting — its version, its capability bits, the nonce every plugin
//! challenges against — arrives in the clear, before there is any way to
//! upgrade. That is the protocol's shape and not a choice, and it is why this
//! module's plaintext leg reads a packet where [`crate::pg`]'s only writes
//! eight bytes.
//!
//! 1. **The greeting**, in the clear. [`read_greeting`] takes it and nothing
//!    acts on it yet. A greeting is not authenticated: every field in it is a
//!    claim by a peer whose certificate has not been checked, so the only two
//!    things read off it are the ones the next step *must* have — the
//!    capability bits, and the version that decides how a response is framed.
//! 2. **The upgrade.** [`request_tls`] writes an `SSLRequest` — a bare
//!    capability word, the last plaintext this connection carries — and a
//!    server whose greeting does not offer `CLIENT_SSL` is **refused** rather
//!    than answered. [ADR 0067 § 3](../../../docs/adr/0067-core-db.md) makes
//!    `VerifyFull` the default with no spelling for turning it off, and a
//!    handshake that continued in the clear here is exactly PHP's
//!    `sslmode=prefer` under another name.
//! 3. **The handshake response and its auth loop**, over TLS.
//!    [`authenticate`] sends the user, the database, the collation and the
//!    first challenge response together, then answers whatever the server asks
//!    next until it says `OK` or `ERR`.
//! 4. **The session's declared zone**, [`set_session_time_zone`] — one
//!    `COM_QUERY`, because MySQL has no startup-parameter list to carry it the
//!    way PostgreSQL's does. It costs a round trip per *connection*, not per
//!    request; § 13's pool is what makes that the right place to spend it, and
//!    the reset that pool performs is `COM_RESET_CONNECTION`, which clears
//!    session variables — so whatever lands that reset owes this call again
//!    after it.
//!
//! # Two plugins, and the rest are refused
//!
//! This driver authenticates with [`CACHING_SHA2_PASSWORD`] and
//! [`MYSQL_NATIVE_PASSWORD`], and refuses every other plugin by name — the same
//! shape [`crate::pg`] takes with `SCRAM-SHA-256`, and for the same reason.
//! Both of those answer a server's nonce with a *proof*: what leaves this
//! process is a hash the server can check and cannot replay elsewhere. The ones
//! refused are the ones that move the secret itself or a broken digest of it —
//! `mysql_old_password`'s pre-4.1 hash, and `mysql_clear_password`, which
//! exists to hand the password verbatim to a server-side PAM module. MariaDB's
//! `ed25519` and `parsec` belong to MariaDB's own driver, which is a separate
//! roster and not a flag on this one.
//!
//! The refusal matters because the *server* chooses. `AuthSwitchRequest` lets a
//! server name a different plugin after the response has gone out, so a peer
//! that talked this driver into `mysql_clear_password` would be handed the
//! password in exchange for nothing. The gate is therefore applied in both
//! places a plugin can be named, and [`plugin_or_refuse`] is the one function
//! that decides it.
//!
//! `caching_sha2_password`'s cache-miss path does send the password, over TLS,
//! to a server whose certificate has already been verified to
//! [ADR 0067 § 3](../../../docs/adr/0067-core-db.md)'s standard. That is the
//! plugin's design and MySQL 8's default; it is not a downgrade a server can
//! ask for, because the fast path and the full path are the same plugin and the
//! same verified peer.
//!
//! # `LOCAL INFILE` is not a capability this client has
//!
//! [`CLIENT_CAPABILITIES`] does not contain `CLIENT_LOCAL_FILES`, so a
//! conforming server may not ask for a file at all — that is
//! [ADR 0067 § 3](../../../docs/adr/0067-core-db.md)'s first closed hole, and
//! it is closed by an absent bit rather than by a check. The check exists
//! anyway, in [`read_ok`], because the bit is a request and a malicious server
//! is under no obligation to honour it: a `0xFB` response is answered with the
//! empty packet that terminates a transfer having sent no bytes, and then the
//! connection is refused. Nothing on this path ever opens a path, so there is
//! no file for a bug to leak; what the refusal buys is that the *wire* says so
//! too.
//!
//! # UTF-8 is forced by the collation byte, not by a statement
//!
//! `HandshakeResponse` carries the connection's collation, and this driver
//! sends `utf8mb4_general_ci`. That is
//! [ADR 0067 § 3](../../../docs/adr/0067-core-db.md)'s third default — text
//! columns arrive as valid UTF-8 by construction, which is what
//! [ADR 0009](../../../docs/adr/0009-string-and-bytes.md)'s guarantee needs —
//! and it costs nothing, where a `SET NAMES` after the fact would be a round
//! trip and a window in which one was not set.

use std::cell::Cell;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::time::Instant;

use bytes::BytesMut;
use mysql_common::auth::plugins::{
    AuthProc, ChallengeResponsePlugin, Context as AuthContextTrait, Response,
};
use mysql_common::collations::CollationId;
use mysql_common::constants::CapabilityFlags;
use mysql_common::io::ParseBuf;
use mysql_common::packets::{
    AuthMoreData, AuthPlugin, AuthSwitchRequest, ErrPacket, HandshakePacket, HandshakeResponse,
    LocalInfilePacket, SslRequest,
};
use mysql_common::proto::codec::PacketCodec;
use mysql_common::proto::codec::error::PacketCodecError;
use mysql_common::proto::{MyDeserialize, MySerialize};
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;

use crate::conn::{MySqlConn, State};

/// MySQL's own port, which an absent `port` in a `[db.<name>]` block means.
///
/// Not a field of [`MySqlTarget`], for the reason it is not one on
/// [`crate::pg::PgTarget`]: a port is half of an address, and the address is
/// resolved by whoever checked the `db.connect` capability. What this crate
/// owes that caller is the number to fall back to.
pub const DEFAULT_PORT: u16 = 3306;

/// The plugin MySQL 8 defaults to, and the first this driver accepts.
pub const CACHING_SHA2_PASSWORD: &str = "caching_sha2_password";

/// The plugin MySQL 5.7 defaults to, and the second this driver accepts.
pub const MYSQL_NATIVE_PASSWORD: &str = "mysql_native_password";

/// `COM_QUERY` — the only text command this driver composes, and it composes
/// exactly one of them ([`set_session_time_zone`]).
const COM_QUERY: u8 = 0x03;

/// `COM_QUIT`, the goodbye a destroyed connection writes.
const COM_QUIT: u8 = 0x01;

/// `COM_RESET_CONNECTION`, ADR 0067 § 13's reset for this backend.
const COM_RESET_CONNECTION: u8 = 0x1F;

/// How much room a read is given when the inbox holds no whole packet.
///
/// [`crate::pg`]'s number and for its reason: a TLS record caps at 16 KiB, so a
/// larger chunk buys no fewer syscalls and a much smaller one costs a syscall
/// per row.
const READ_CHUNK: usize = 16 * 1024;

/// The largest packet payload this client will send or accept, in bytes.
///
/// It is a *ceiling* rather than a size: the codec refuses anything larger
/// instead of allocating it, which is the whole point of naming a number a
/// server does not get to choose. 16 MiB is MySQL's own `max_allowed_packet`
/// default, so a value this client can send is one a stock server accepts.
const MAX_PACKET: u32 = 16 * 1024 * 1024;

/// The collation every connection this driver opens is in —
/// [ADR 0067 § 3](../../../docs/adr/0067-core-db.md)'s forced `utf8mb4`.
const COLLATION: u8 = CollationId::UTF8MB4_GENERAL_CI as u8;

/// What this client claims it can do, and the absences are the interesting
/// half.
///
/// **`CLIENT_LOCAL_FILES` is not here**, which is ADR 0067 § 3's `LOCAL INFILE`
/// closed at the only place it can be closed *before* a server asks — the
/// module doc owns the second half of that rule. **`CLIENT_COMPRESS` and
/// `CLIENT_ZSTD_COMPRESSION_ALGORITHM` are not here** either: the workspace
/// takes `mysql_common` with its compression feature off, so there is no
/// implementation behind the bit, and a compressed stream would in any case be
/// a second framing layer under a TLS session that already has one.
/// **`CLIENT_MULTI_STATEMENTS` is not here**, and that is a security bit: it is
/// what turns one injected `;` into two statements, and ADR 0067 § 1's
/// every-statement-is-prepared has no use for it.
const CLIENT_CAPABILITIES: CapabilityFlags = CapabilityFlags::CLIENT_PROTOCOL_41
    .union(CapabilityFlags::CLIENT_SECURE_CONNECTION)
    .union(CapabilityFlags::CLIENT_PLUGIN_AUTH)
    .union(CapabilityFlags::CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA)
    .union(CapabilityFlags::CLIENT_SSL)
    .union(CapabilityFlags::CLIENT_LONG_PASSWORD)
    .union(CapabilityFlags::CLIENT_LONG_FLAG)
    .union(CapabilityFlags::CLIENT_TRANSACTIONS)
    .union(CapabilityFlags::CLIENT_MULTI_RESULTS)
    .union(CapabilityFlags::CLIENT_PS_MULTI_RESULTS)
    .union(CapabilityFlags::CLIENT_DEPRECATE_EOF)
    .union(CapabilityFlags::CLIENT_CONNECT_WITH_DB);

/// The bits a server must offer before any credential is composed.
///
/// `CLIENT_SSL` is in here rather than checked separately because it is the
/// same kind of fact as the other three: without it there is no connection this
/// driver is willing to have, and the refusal should read as one sentence.
const REQUIRED_CAPABILITIES: CapabilityFlags = CapabilityFlags::CLIENT_PROTOCOL_41
    .union(CapabilityFlags::CLIENT_SECURE_CONNECTION)
    .union(CapabilityFlags::CLIENT_PLUGIN_AUTH)
    .union(CapabilityFlags::CLIENT_SSL);

/// Where one MySQL server is, and who to be there.
///
/// [`crate::pg::PgTarget`]'s shape and its split: `host` is the name the
/// certificate must be valid for, and the *address* is not here at all because
/// it was resolved by whoever checked the `db.connect` capability. A driver
/// that re-resolved the name would be connecting somewhere nobody approved.
pub struct MySqlTarget<'a> {
    /// The name the server's certificate is checked against.
    pub host: &'a str,
    /// The user to log in as. ADR 0067 § 3 accepts `tainted` here freely: it is
    /// a length-prefixed protocol field, never parsed text.
    pub user: &'a str,
    /// The user's password, used only to derive a challenge response.
    pub password: &'a str,
    /// The schema to select on connect.
    pub database: &'a str,
    /// The PEM bundle whose anchors this server's certificate is verified
    /// against, or the compiled-in Mozilla set where the block names none.
    ///
    /// Not a way to turn verification off — ADR 0067 § 3 has no spelling for
    /// that. What it changes is *whose* certificates are believed.
    pub tls_ca_file: Option<&'a Path>,
    /// The zone a `DATETIME` or `TIMESTAMP` off this connection is read in, as
    /// a whole number of seconds east of UTC.
    ///
    /// [ADR 0067 § 9](../../../docs/adr/0067-core-db.md)'s declared zone. It is
    /// sent to the server as well as held here, so `CURRENT_TIMESTAMP` and a
    /// decoded column agree about which zone they are in —
    /// [`set_session_time_zone`] is where it goes out, and as a numeric offset
    /// rather than a name because named zones need `mysql.time_zone` populated
    /// and it usually is not.
    pub time_zone: i32,
}

impl std::fmt::Debug for MySqlTarget<'_> {
    /// Everything but the password, which is a `secret` at the language level
    /// (ADR 0067 § 3) and is not printed in any rendering.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MySqlTarget")
            .field("host", &self.host)
            .field("user", &self.user)
            .field("database", &self.database)
            .field("time_zone", &self.time_zone)
            .finish_non_exhaustive()
    }
}

/// A MySQL connection's stream, the bytes read off it that are not yet a whole
/// packet, and the sequence-id state that says which packet is next.
///
/// Generic in the stream so the exchanges below can be asserted against a
/// server that answers rather than against a socket — the playbook's rule for
/// this crate, and the reason every routine here takes a `&mut Wire<S>` instead
/// of being an inherent method on [`MySqlConn`]. A real connection's wire is
/// the `NvsTls<NvsTcp>` the default names.
pub(crate) struct Wire<S: Read + Write = NvsTls<NvsTcp>> {
    stream: S,
    inbox: BytesMut,
    /// `mysql_common`'s framing, and the sequence counter with it. It survives
    /// [`Wire::upgrade`] because MySQL's TLS upgrade happens *inside* the
    /// packet stream: the `SSLRequest` is packet 1 and the handshake response
    /// that follows it, on the encrypted socket, is packet 2. A codec reset at
    /// the upgrade would put the connection out of sync on its first
    /// authenticated byte.
    codec: PacketCodec,
}

impl<S: Read + Write> std::fmt::Debug for Wire<S> {
    /// How much is buffered, and nothing about the stream: a `rustls` session
    /// holds key material, and a buffered packet is one request's data.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wire")
            .field("buffered", &self.inbox.len())
            .finish_non_exhaustive()
    }
}

impl<S: Read + Write> Wire<S> {
    /// A wire over a stream that is already connected.
    ///
    /// Unlike [`crate::pg`]'s, this one is built *before* the upgrade: MySQL
    /// speaks first, so the greeting is read through the codec on a plaintext
    /// socket and the same codec continues once the socket is encrypted.
    fn new(stream: S) -> Wire<S> {
        let mut codec = PacketCodec::default();
        codec.max_allowed_packet = MAX_PACKET as usize;
        Wire {
            stream,
            inbox: BytesMut::new(),
            codec,
        }
    }

    /// The same conversation over an encrypted stream.
    ///
    /// Takes the codec and the inbox across with it — see the field. `wrap`
    /// receives the plaintext stream and answers the encrypted one, which is
    /// the only shape that works: the TLS session is built *from* this wire's
    /// socket, so a signature taking the new stream would need the old one
    /// moved out first and leave a half-consumed `Wire` behind.
    ///
    /// The caller must have flushed the `SSLRequest` first, and must not have
    /// read anything since: whatever is buffered here arrived in the clear.
    ///
    /// # Errors
    ///
    /// Whatever `wrap` reported.
    fn upgrade<T: Read + Write>(
        self,
        wrap: impl FnOnce(S) -> io::Result<T>,
    ) -> io::Result<Wire<T>> {
        Ok(Wire {
            stream: wrap(self.stream)?,
            inbox: self.inbox,
            codec: self.codec,
        })
    }

    /// The stream underneath, for the tests that assert on what was written to
    /// it. There is no non-test reader: a driver talks through the two methods
    /// below, and reaching past them is how a half-packet gets written.
    #[cfg(test)]
    fn peer(&self) -> &S {
        &self.stream
    }

    /// Frames `payload` as the next packet in sequence, writes all of it, and
    /// flushes.
    ///
    /// One function because a half-written packet is the shape ADR 0132 § 4
    /// calls poison, and `write_all` is the only spelling that cannot leave
    /// one.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a payload past [`MAX_PACKET`], and whatever the
    /// stream reported.
    pub(crate) fn send(&mut self, payload: &[u8]) -> io::Result<()> {
        let mut out = BytesMut::new();
        self.codec
            .encode(&mut &payload[..], &mut out)
            .map_err(codec_failed)?;
        self.stream.write_all(&out)?;
        self.stream.flush()
    }

    /// Reads until the inbox holds a whole packet, and takes its payload.
    ///
    /// Bytes that arrived after this packet — or the head of one that has not
    /// finished arriving — stay buffered for the next call.
    ///
    /// # Errors
    ///
    /// `UnexpectedEof` when the peer closed mid-packet, `InvalidData` for a
    /// sequence id that is not the one owed or a payload past [`MAX_PACKET`],
    /// and whatever the stream reported.
    pub(crate) fn read_packet(&mut self) -> io::Result<Vec<u8>> {
        let mut packet = Vec::new();
        loop {
            if self
                .codec
                .decode(&mut self.inbox, &mut packet)
                .map_err(codec_failed)?
            {
                return Ok(packet);
            }

            let filled = self.inbox.len();
            self.inbox.resize(filled + READ_CHUNK, 0);
            let read = self.stream.read(&mut self.inbox[filled..]);
            // Restore the length before the `?`: a failed read must not leave
            // the inbox claiming the zeroes it was given room in.
            let read = match read {
                Ok(read) => read,
                Err(e) => {
                    self.inbox.truncate(filled);
                    return Err(e);
                }
            };
            self.inbox.truncate(filled + read);

            if read == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "the server closed the connection part-way through a packet",
                ));
            }
        }
    }
}

/// A framing failure, as the `io::Error` every routine here answers with.
///
/// `PacketCodecError` is `mysql_common`'s and does not implement
/// `std::error::Error`, so it is rendered rather than boxed. `PacketsOutOfSync`
/// is the one worth reading twice: it means the server sent a sequence id other
/// than the one owed, which is a wire this connection can no longer locate a
/// boundary in.
fn codec_failed(error: PacketCodecError) -> io::Error {
    match error {
        PacketCodecError::Io(inner) => inner,
        other => io::Error::new(io::ErrorKind::InvalidData, other.to_string()),
    }
}

/// What the server said before anything of ours was sent, kept only as long as
/// it is needed.
///
/// Owned rather than borrowed from the greeting packet because the packet's
/// buffer is gone by the time the response is composed, and because *nothing
/// here is trusted yet*: it arrived in the clear from a peer whose certificate
/// has not been checked. Only the fields the next two steps cannot proceed
/// without are kept.
#[derive(Debug)]
struct Greeting {
    /// The capability bits the server claims, already checked against
    /// [`REQUIRED_CAPABILITIES`].
    capabilities: CapabilityFlags,
    /// The challenge every plugin's response is derived from.
    nonce: Vec<u8>,
    /// The plugin the server wants, as it spelled it.
    plugin: Vec<u8>,
    /// The version, which decides `HandshakeResponse`'s collation width. Absent
    /// from a server whose banner does not parse, in which case the modern
    /// framing is assumed — every server this driver will speak to is past
    /// 5.5.3 by decades.
    server_version: (u16, u16, u16),
}

/// Reads the server's greeting off a plaintext socket.
///
/// # Errors
///
/// `ConnectionRefused` for a server that offers none of
/// [`REQUIRED_CAPABILITIES`] — including TLS — `InvalidData` for a packet that
/// is not a greeting, and the server's own refusal where it declined the
/// connection before greeting at all.
fn read_greeting<S: Read + Write>(wire: &mut Wire<S>) -> io::Result<Greeting> {
    let packet = wire.read_packet()?;
    if packet.first() == Some(&0xFF) {
        return Err(server_refusal(&packet, CapabilityFlags::empty()));
    }

    let handshake = HandshakePacket::deserialize((), &mut ParseBuf(&packet))?;
    let capabilities = handshake.capabilities();
    let missing = REQUIRED_CAPABILITIES.difference(capabilities);
    if !missing.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!(
                "the server offers no {missing:?}, and this driver needs \
                 {REQUIRED_CAPABILITIES:?}: ADR 0067 § 3 has no spelling for a \
                 connection without TLS or without plugin authentication"
            ),
        ));
    }

    Ok(Greeting {
        capabilities,
        nonce: handshake.nonce(),
        plugin: handshake
            .auth_plugin_name_ref()
            .unwrap_or_default()
            .to_vec(),
        server_version: handshake.server_version_parsed().unwrap_or((5, 5, 3)),
    })
}

/// Writes the `SSLRequest` that turns the rest of this socket into TLS records.
///
/// It is a capability word and nothing else — no user, no database, no
/// credential — because everything identifying is owed to the encrypted half.
///
/// # Errors
///
/// Whatever the stream reported.
fn request_tls<S: Read + Write>(wire: &mut Wire<S>, greeting: &Greeting) -> io::Result<()> {
    let mut payload = Vec::new();
    SslRequest::new(
        CLIENT_CAPABILITIES.intersection(greeting.capabilities),
        MAX_PACKET,
        COLLATION,
    )
    .serialize(&mut payload);
    wire.send(&payload)
}

/// The plugin a name selects, or the refusal that name earns.
///
/// One function because a plugin is named in two places — the greeting, and an
/// `AuthSwitchRequest` that arrives after the response has gone out — and a
/// gate applied at only the first of them is not a gate: the module doc owns
/// why a server choosing the second is the case that matters.
///
/// # Errors
///
/// `ConnectionRefused`, naming what was asked for and what this driver answers.
fn plugin_or_refuse(name: &[u8]) -> io::Result<AuthPlugin<'static>> {
    match AuthPlugin::from_bytes(name) {
        AuthPlugin::CachingSha2Password => Ok(AuthPlugin::CachingSha2Password),
        AuthPlugin::MysqlNativePassword => Ok(AuthPlugin::MysqlNativePassword),
        _ => Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!(
                "the server asked to authenticate with `{}`, and this driver answers \
                 `{CACHING_SHA2_PASSWORD}` and `{MYSQL_NATIVE_PASSWORD}` only: those are \
                 the two that send a proof rather than the password",
                String::from_utf8_lossy(name)
            ),
        )),
    }
}

/// What a plugin is allowed to know while it composes a response.
///
/// `is_tls_transport` is unconditionally `true` and that is a fact rather than
/// an assertion: [`request_tls`] refused every path to this point that was not
/// encrypted. `server_key_pem` is `None` because the RSA key exchange it feeds
/// is the *unencrypted* transport's answer to the same problem, and there is no
/// unencrypted transport here.
#[derive(Clone, Copy)]
struct AuthContext<'a> {
    password: &'a [u8],
    nonce: &'a [u8],
}

impl AuthContextTrait for AuthContext<'_> {
    fn pass(&self) -> &[u8] {
        self.password
    }

    fn is_ipc_transport(&self) -> bool {
        false
    }

    fn is_tls_transport(&self) -> bool {
        true
    }

    fn scramble(&self) -> &[u8] {
        self.nonce
    }

    fn server_key_pem(&self) -> Option<&[u8]> {
        None
    }
}

/// Sends the handshake response and answers the server until it says `OK`.
///
/// Returns the capability set the two ends agreed on, which every later packet
/// on this connection is decoded against.
///
/// # Errors
///
/// `ConnectionRefused` for a plugin this driver does not answer,
/// `PermissionDenied` for the server's own refusal — a wrong password, a schema
/// that does not exist — carrying its `SQLSTATE` and message, `InvalidData` for
/// a packet the protocol does not allow at that point, and whatever the stream
/// reported.
fn authenticate<S: Read + Write>(
    wire: &mut Wire<S>,
    target: &MySqlTarget<'_>,
    greeting: &Greeting,
) -> io::Result<CapabilityFlags> {
    let capabilities = CLIENT_CAPABILITIES.intersection(greeting.capabilities);
    let context = AuthContext {
        password: target.password.as_bytes(),
        nonce: &greeting.nonce,
    };

    let plugin = plugin_or_refuse(&greeting.plugin)?;
    let mut exchange = AuthProc::init(&plugin)
        .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?;
    let first = exchange
        .run(context, &greeting.nonce)
        .map_err(auth_failed)?;

    let mut payload = Vec::new();
    HandshakeResponse::new(
        first.data().map(<[u8]>::to_vec),
        greeting.server_version,
        Some(target.user.as_bytes()),
        Some(target.database.as_bytes()),
        Some(plugin),
        capabilities,
        None,
        MAX_PACKET,
    )
    .serialize(&mut payload);
    wire.send(&payload)?;

    loop {
        let packet = wire.read_packet()?;
        match packet.first() {
            // `OK`, and the only way out of this loop that is a connection.
            Some(0x00) => return Ok(capabilities),
            Some(0xFF) => return Err(server_refusal(&packet, capabilities)),
            // `AuthSwitchRequest`: the server names a different plugin, and the
            // gate applies again — this is the packet the module doc is about.
            Some(0xFE) => {
                let switch = AuthSwitchRequest::deserialize((), &mut ParseBuf(&packet))?;
                let named = plugin_or_refuse(switch.auth_plugin().as_bytes())?;
                let challenge = switch.plugin_data().to_vec();
                exchange = AuthProc::init(&named)
                    .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?;
                let step = exchange.run(context, &challenge).map_err(auth_failed)?;
                send_step(wire, &step)?;
            }
            // `AuthMoreData`: the plugin's own next challenge, with the leading
            // `0x01` the server escapes binary data with already stripped by
            // the deserializer — `ChallengeResponsePlugin::run` requires that.
            Some(0x01) => {
                let more = AuthMoreData::deserialize((), &mut ParseBuf(&packet))?;
                let challenge = more.data().to_vec();
                let step = exchange.run(context, &challenge).map_err(auth_failed)?;
                send_step(wire, &step)?;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "the server sent something that is not an authentication packet",
                ));
            }
        }
    }
}

/// Writes a plugin step's packet, where it has one.
///
/// [`Response::Last`] with no packet is a plugin that has said everything it
/// will say and is waiting for the server's verdict; writing an empty packet
/// there would be a message the protocol does not expect.
fn send_step<S: Read + Write>(wire: &mut Wire<S>, step: &Response) -> io::Result<()> {
    match step.data() {
        Some(data) => wire.send(data),
        None => Ok(()),
    }
}

/// A plugin's own refusal, as an `io::Error`.
///
/// These are all client-side arithmetic failures — a challenge of the wrong
/// length, a password the plugin cannot represent — so they are `InvalidData`
/// rather than a permission verdict the server has not yet given.
fn auth_failed(error: mysql_common::auth::plugins::Error) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("the authentication exchange failed: {error}"),
    )
}

/// The server's `ERR` packet, worded as this crate's callers read it.
///
/// `PermissionDenied` rather than `Other` because every refusal that reaches
/// this path during a handshake is one: the credentials, the schema, or the
/// host's right to connect at all. The code and `SQLSTATE` are carried because
/// they are what an operator greps for; ADR 0067 § 8's mapping to a
/// `DbErrorKind` is a table this driver does not have yet.
fn server_refusal(packet: &[u8], capabilities: CapabilityFlags) -> io::Error {
    let Ok(err) = ErrPacket::deserialize(capabilities, &mut ParseBuf(packet)) else {
        return io::Error::new(
            io::ErrorKind::InvalidData,
            "the server sent an error packet that does not decode",
        );
    };
    if !err.is_error() {
        return io::Error::new(
            io::ErrorKind::InvalidData,
            "the server sent a progress report where an error or a result was owed",
        );
    }
    let error = err.server_error();
    let state = error
        .sql_state_ref()
        .map(|s| String::from_utf8_lossy(&s.as_bytes()).into_owned())
        .unwrap_or_else(|| "HY000".to_owned());
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!(
            "MySQL {} (SQLSTATE {state}): {}",
            error.error_code(),
            error.message_str()
        ),
    )
}

/// Reads the answer to a command that returns no rows.
///
/// Three of the four packets it can be are ordinary; the fourth is
/// [ADR 0067 § 3](../../../docs/adr/0067-core-db.md)'s `LOCAL INFILE`, and the
/// module doc owns why the check exists when the capability bit was never sent.
/// The refusal writes the empty packet that terminates a transfer — so the
/// server is told the file is zero bytes long rather than being left waiting —
/// and then refuses the connection. **Nothing on this path can open a file**:
/// there is no filesystem call in this module, and the packet's name is read
/// only to say it in the error.
///
/// # Errors
///
/// `PermissionDenied` for the server's own error and for a `LOCAL INFILE`
/// request, `InvalidData` for anything else.
pub(crate) fn read_ok<S: Read + Write>(
    wire: &mut Wire<S>,
    capabilities: CapabilityFlags,
) -> io::Result<()> {
    let packet = wire.read_packet()?;
    match packet.first() {
        Some(0x00) => Ok(()),
        Some(0xFF) => Err(server_refusal(&packet, capabilities)),
        Some(0xFB) => {
            let named = LocalInfilePacket::deserialize((), &mut ParseBuf(&packet))
                .map(|request| request.file_name_str().into_owned())
                .unwrap_or_else(|_| "an unreadable path".to_owned());
            // The empty packet is the protocol's end-of-transfer, and it is the
            // whole of what this client sends: no path was opened and no byte
            // of one is on the wire.
            wire.send(&[])?;
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "the server asked this client to send the local file `{named}`, and \
                     ADR 0067 § 3 turns `LOCAL INFILE` off with no option to enable it: \
                     `CLIENT_LOCAL_FILES` was never offered, so a server that asks is \
                     one this connection stops talking to"
                ),
            ))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the server answered a command with something that is neither a result nor \
             a status",
        )),
    }
}

/// Sends ADR 0067 § 9's declared zone as a session variable.
///
/// A numeric offset and never a zone name, because a name needs the
/// `mysql.time_zone` tables populated and they usually are not — § 9 says so,
/// and this is the one place that rule is spent.
///
/// # Errors
///
/// As [`read_ok`]. A server that refuses this refuses the connection: a
/// connection whose zone is not the declared one would decode every zone-less
/// column wrong, silently.
fn set_session_time_zone<S: Read + Write>(
    wire: &mut Wire<S>,
    capabilities: CapabilityFlags,
    seconds_east: i32,
) -> io::Result<()> {
    // `COM_QUERY`, and the only text statement this driver ever composes: its
    // one interpolated value is a number this process computed, so ADR 0067
    // § 1's no-emulated-prepares rule has nothing to bite on.
    let mut payload = vec![COM_QUERY];
    payload.extend_from_slice(
        format!("SET time_zone = '{}'", offset_literal(seconds_east)).as_bytes(),
    );
    wire.codec.reset_seq_id();
    wire.send(&payload)?;
    read_ok(wire, capabilities)
}

/// A whole number of seconds east of UTC as MySQL's `±HH:MM`.
///
/// Seconds that are not a whole minute are truncated toward zero: MySQL's
/// `time_zone` has no finer resolution, and every zone in use has been on a
/// whole minute since 1972.
fn offset_literal(seconds_east: i32) -> String {
    let sign = if seconds_east < 0 { '-' } else { '+' };
    let minutes = seconds_east.unsigned_abs() / 60;
    format!("{sign}{:02}:{:02}", minutes / 60, minutes % 60)
}

impl MySqlConn {
    /// Opens a connection: TCP, the greeting, § 3's in-band upgrade, TLS, the
    /// authentication exchange, and the declared zone.
    ///
    /// `deadline` bounds the whole of that and not one leg of it — the connect,
    /// the handshake and every authentication round trip share one clock, filed
    /// on the socket where `nvs-host` keeps it. `None` is unbounded and is for
    /// a caller that has its own bound.
    ///
    /// The connection comes back [`State::Idle`]: at a packet boundary, with a
    /// statement allowed.
    ///
    /// # Errors
    ///
    /// `ConnectionRefused` when the server will not upgrade to TLS, offers none
    /// of [`REQUIRED_CAPABILITIES`], or names an authentication plugin this
    /// driver does not answer; `PermissionDenied` for the server's own refusal;
    /// `InvalidData` for a packet the protocol does not allow at that point —
    /// or for a `tls_ca_file` that holds no certificate — `TimedOut` when the
    /// deadline passes, and whatever the socket or the TLS handshake itself
    /// reported.
    pub fn connect(
        addr: SocketAddr,
        target: &MySqlTarget<'_>,
        deadline: Option<Instant>,
    ) -> io::Result<MySqlConn> {
        let mut tcp = match deadline {
            Some(at) => {
                NvsTcp::connect_timeout(addr, at.saturating_duration_since(Instant::now()))?
            }
            None => NvsTcp::connect(addr)?,
        };
        tcp.set_deadline(deadline);

        let mut plain = Wire::new(tcp);
        let greeting = read_greeting(&mut plain)?;
        request_tls(&mut plain, &greeting)?;

        // Whatever is still buffered arrived in the clear and is carried across
        // with the codec: the greeting is the only thing a server may say
        // before the upgrade, and `read_greeting` took it.
        let mut wire = plain.upgrade(|tcp| match target.tls_ca_file {
            Some(bundle) => NvsTls::over_bundle(tcp, target.host, bundle),
            None => NvsTls::over(tcp, target.host),
        })?;

        let capabilities = authenticate(&mut wire, target, &greeting)?;
        set_session_time_zone(&mut wire, capabilities, target.time_zone)?;

        Ok(MySqlConn {
            wire,
            state: Cell::new(State::Idle),
            capabilities,
            // § 9's zone-less row is decoded a layer up, where the target is
            // gone — see the field.
            time_zone: target.time_zone,
        })
    }

    /// The zone a zone-less `DATETIME` or `TIMESTAMP` off this connection is
    /// read in, as seconds east of UTC — the same number the handshake sent the
    /// server.
    #[must_use]
    pub fn time_zone(&self) -> i32 {
        self.time_zone
    }

    /// [ADR 0067 § 13](../../../docs/adr/0067-core-db.md)'s reset, before this
    /// connection may be handed to another request.
    ///
    /// Takes `self` by value for [`crate::conn::PgConn::reset`]'s reason: a reset
    /// that failed must not be able to hand a connection back, and a signature
    /// that borrowed would let a caller ignore the `Err` and pool it anyway.
    ///
    /// # Errors
    ///
    /// As [`read_ok`]. The connection is consumed either way.
    pub fn reset(mut self) -> io::Result<MySqlConn> {
        reset_session(&mut self.wire, self.capabilities, self.time_zone)?;
        Ok(self)
    }
}

impl Drop for MySqlConn {
    /// Says goodbye rather than vanishing — [`say_goodbye`] has the reasoning.
    fn drop(&mut self) {
        say_goodbye(&mut self.wire);
    }
}

/// `COM_RESET_CONNECTION`, and the one session variable it clears that this
/// driver put there.
///
/// § 13 calls this reset "atomic and complete", which is what makes it the
/// right primitive and also what obliges the second half: it drops session
/// variables, so ADR 0067 § 9's declared zone has to be sent again or the next
/// request decodes every zone-less column in UTC while believing otherwise.
/// The prepared-statement cache goes with it too — § 13 names that as the real
/// asymmetry with PostgreSQL, and it is the protocol's rather than a choice.
///
/// Free and generic in the stream for this crate's usual reason: a
/// `Wire<NvsTls<NvsTcp>>` needs a socket and a certificate that no unit test
/// has.
///
/// # Errors
///
/// As [`read_ok`], for either of the two commands.
fn reset_session<S: Read + Write>(
    wire: &mut Wire<S>,
    capabilities: CapabilityFlags,
    seconds_east: i32,
) -> io::Result<()> {
    wire.codec.reset_seq_id();
    wire.send(&[COM_RESET_CONNECTION])?;
    read_ok(wire, capabilities)?;
    set_session_time_zone(wire, capabilities, seconds_east)
}

/// The last thing a destroyed connection writes: `COM_QUIT`, best effort.
///
/// [`crate::pg`]'s `say_goodbye` and for its reasons: the server closes its own
/// side instead of discovering a reset socket, which is one fewer error line in
/// its log per connection. Best effort by definition — the connection is going
/// away whatever the write reports, so there is nobody left to tell.
fn say_goodbye<S: Read + Write>(wire: &mut Wire<S>) {
    wire.codec.reset_seq_id();
    let _ = wire.send(&[COM_QUIT]);
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::{
        AuthContext, CLIENT_CAPABILITIES, COLLATION, Greeting, MySqlTarget, Wire, authenticate,
        offset_literal, read_greeting, read_ok, request_tls,
    };
    use mysql_common::constants::{CapabilityFlags, StatusFlags};
    use mysql_common::packets::HandshakePacket;
    use mysql_common::proto::MySerialize;
    use std::io;

    /// The nonce every case challenges with, and the password every case
    /// refuses to find on the wire.
    const NONCE: &[u8; 20] = b"NR3HP:qIYa_9=db?Sd{`";
    const PASSWORD: &str = "correct-horse-battery";

    /// A server that answers the client rather than a script.
    ///
    /// [`crate::pg`]'s `Peer` and for its reason: an authentication exchange's
    /// every packet depends on the one before it, so a canned transcript could
    /// only assert that we send *something*. A flush is the message boundary —
    /// every write in this module is `write_all` then `flush` — so the closure
    /// is handed exactly what the driver considers one send.
    struct Peer<F: FnMut(&[u8]) -> Vec<u8>> {
        server: F,
        pending: Vec<u8>,
        inbound: Vec<u8>,
        read: usize,
        /// Every flushed group, for the assertions that are about what we sent.
        sent: Vec<Vec<u8>>,
    }

    impl<F: FnMut(&[u8]) -> Vec<u8>> Peer<F> {
        fn new(server: F) -> Peer<F> {
            Peer {
                server,
                pending: Vec::new(),
                inbound: Vec::new(),
                read: 0,
                sent: Vec::new(),
            }
        }
    }

    impl<F: FnMut(&[u8]) -> Vec<u8>> io::Write for Peer<F> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.pending.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            let message = std::mem::take(&mut self.pending);
            self.inbound.extend_from_slice(&(self.server)(&message));
            self.sent.push(message);
            Ok(())
        }
    }

    impl<F: FnMut(&[u8]) -> Vec<u8>> io::Read for Peer<F> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let left = &self.inbound[self.read..];
            let take = left.len().min(buf.len());
            buf[..take].copy_from_slice(&left[..take]);
            self.read += take;
            Ok(take)
        }
    }

    /// One packet: a three-byte little-endian length, the sequence id, the
    /// payload.
    fn packet(seq: u8, payload: &[u8]) -> Vec<u8> {
        let len = u32::try_from(payload.len()).expect("a test packet fits in a u32");
        let mut out = len.to_le_bytes()[..3].to_vec();
        out.push(seq);
        out.extend_from_slice(payload);
        out
    }

    /// A server greeting naming `plugin`, with `capabilities` on top of the set
    /// every modern server offers.
    fn greeting_bytes(plugin: &str, capabilities: CapabilityFlags) -> Vec<u8> {
        // A real server's second scramble field carries a trailing NUL, and
        // the length byte in front of it counts that byte — a 12-byte tail
        // here would make the *plugin name* start one byte late, which is
        // exactly the shape a hand-rolled greeting gets wrong.
        let mut tail = NONCE[8..].to_vec();
        tail.push(0);

        let mut payload = Vec::new();
        HandshakePacket::new(
            10,
            &b"8.0.36"[..],
            42,
            NONCE[..8].try_into().expect("eight bytes of nonce"),
            Some(tail),
            capabilities,
            COLLATION,
            StatusFlags::empty(),
            Some(plugin.as_bytes()),
        )
        .serialize(&mut payload);
        packet(0, &payload)
    }

    /// Everything a stock MySQL 8 offers that this driver cares about.
    fn server_capabilities() -> CapabilityFlags {
        CLIENT_CAPABILITIES.union(CapabilityFlags::CLIENT_LOCAL_FILES)
    }

    /// The target every case authenticates as.
    fn target() -> MySqlTarget<'static> {
        MySqlTarget {
            host: "db.example.internal",
            user: "novis",
            password: PASSWORD,
            database: "shop",
            tls_ca_file: None,
            time_zone: 0,
        }
    }

    /// `caching_sha2_password`'s fast-path proof, computed here rather than
    /// borrowed from the codec: a test that called the same function could pass
    /// on a bug the two shared.
    ///
    /// `XOR(SHA256(password), SHA256(SHA256(SHA256(password)), nonce))`.
    fn expected_proof(password: &[u8], nonce: &[u8]) -> Vec<u8> {
        let once = Sha256::digest(password);
        let twice = Sha256::digest(once);
        let mut hasher = Sha256::new();
        hasher.update(twice);
        hasher.update(nonce);
        let salted = hasher.finalize();
        once.iter().zip(salted).map(|(a, b)| a ^ b).collect()
    }

    /// Every byte this client wrote, flattened — what the security assertions
    /// search.
    fn everything_sent<F: FnMut(&[u8]) -> Vec<u8>>(peer: &Peer<F>) -> Vec<u8> {
        peer.sent.iter().flatten().copied().collect()
    }

    /// ADR 0067 § 3's third default and § 1's proof-not-password rule, in the
    /// one exchange that decides both.
    ///
    /// Four properties of the same handshake response, because they are four
    /// readings of one packet and splitting them would script the same server
    /// four times: the collation is `utf8mb4`, the scramble is the plugin's
    /// proof rather than anything derivable from the password by looking at it,
    /// the password's own bytes are nowhere in what was sent, and
    /// `CLIENT_LOCAL_FILES` is not among the capabilities claimed.
    #[test]
    fn the_mysql_handshake_forces_utf8mb4_and_sends_a_proof_rather_than_the_password() {
        let mut step = 0;
        let mut wire = Wire::new(Peer::new(move |_sent: &[u8]| {
            step += 1;
            match step {
                // The `SSLRequest`, which is answered with nothing: a real
                // server's next words arrive over TLS.
                1 => Vec::new(),
                // The handshake response earns an `OK`, and then the
                // `SET time_zone` does too.
                _ => packet(u8::try_from(step).expect("a small step count") + 1, &[0x00]),
            }
        }));
        wire.inbox.extend_from_slice(&greeting_bytes(
            super::CACHING_SHA2_PASSWORD,
            server_capabilities(),
        ));

        let greeting = read_greeting(&mut wire).expect("the greeting decodes");
        request_tls(&mut wire, &greeting).expect("the upgrade request goes out");
        let agreed = authenticate(&mut wire, &target(), &greeting).expect("the server says OK");

        let response = wire.peer().sent[1].clone();
        // Past the four-byte header, `HandshakeResponse` is a capability word,
        // a max-packet word, then the collation byte.
        let claimed = CapabilityFlags::from_bits_retain(u32::from_le_bytes(
            response[4..8].try_into().expect("a capability word"),
        ));
        assert!(
            !claimed.contains(CapabilityFlags::CLIENT_LOCAL_FILES),
            "the client claimed `CLIENT_LOCAL_FILES` at a server that offered it"
        );
        assert!(
            !agreed.contains(CapabilityFlags::CLIENT_LOCAL_FILES),
            "the agreed capability set carries `CLIENT_LOCAL_FILES`"
        );
        assert_eq!(
            response[12], COLLATION,
            "the connection was opened in a collation that is not utf8mb4"
        );

        let sent = everything_sent(wire.peer());
        assert!(
            !sent
                .windows(PASSWORD.len())
                .any(|w| w == PASSWORD.as_bytes()),
            "the password itself appears in what this client sent"
        );
        let proof = expected_proof(PASSWORD.as_bytes(), NONCE);
        assert!(
            sent.windows(proof.len()).any(|w| w == proof),
            "the handshake response does not carry `caching_sha2_password`'s own proof"
        );
    }

    /// ADR 0067 § 3's TLS default, asserted where it has to hold: **before**
    /// anything identifying is composed.
    ///
    /// The bound is named on both sides — a server offering `CLIENT_SSL` is the
    /// case above, and this is the first one refused — and the refusal is
    /// checked to have cost the client nothing: not a user name, not a schema,
    /// not a proof.
    #[test]
    fn a_server_that_offers_no_tls_is_refused_before_anything_is_sent() {
        let mut wire = Wire::new(Peer::new(|_sent: &[u8]| {
            panic!("the client answered a server that offered no TLS")
        }));
        wire.inbox.extend_from_slice(&greeting_bytes(
            super::CACHING_SHA2_PASSWORD,
            server_capabilities().difference(CapabilityFlags::CLIENT_SSL),
        ));

        let refused = read_greeting(&mut wire).expect_err("a plaintext server is not a connection");
        assert_eq!(refused.kind(), io::ErrorKind::ConnectionRefused);
        assert!(
            wire.peer().sent.is_empty(),
            "the client wrote something to a server it had already refused"
        );
    }

    /// The `AuthSwitchRequest` gate: a server that asks for a
    /// password-sending plugin *after* the response has gone out is refused,
    /// and the password does not follow.
    ///
    /// This is the case the plugin roster exists for. A gate applied only to
    /// the greeting would pass this exchange, because the greeting names a
    /// plugin this driver accepts and the switch names the one it does not.
    #[test]
    fn an_auth_switch_to_a_password_sending_plugin_is_refused() {
        let mut step = 0;
        let mut wire = Wire::new(Peer::new(move |_sent: &[u8]| {
            step += 1;
            match step {
                1 => Vec::new(),
                // `AuthSwitchRequest`: 0xFE, the plugin name, the nonce.
                _ => {
                    let mut body = vec![0xFE];
                    body.extend_from_slice(b"mysql_clear_password\0");
                    body.extend_from_slice(NONCE);
                    body.push(0);
                    packet(3, &body)
                }
            }
        }));
        wire.inbox.extend_from_slice(&greeting_bytes(
            super::MYSQL_NATIVE_PASSWORD,
            server_capabilities(),
        ));

        let greeting = read_greeting(&mut wire).expect("the greeting decodes");
        request_tls(&mut wire, &greeting).expect("the upgrade request goes out");
        let refused = authenticate(&mut wire, &target(), &greeting)
            .expect_err("`mysql_clear_password` is not a plugin this driver answers");

        assert_eq!(refused.kind(), io::ErrorKind::ConnectionRefused);
        assert!(
            refused.to_string().contains("mysql_clear_password"),
            "the refusal does not name the plugin that was asked for: {refused}"
        );
        let sent = everything_sent(wire.peer());
        assert!(
            !sent
                .windows(PASSWORD.len())
                .any(|w| w == PASSWORD.as_bytes()),
            "the password was handed to a server that asked for it in the clear"
        );
    }

    /// ADR 0067 § 3's first closed hole, asserted as a property of the
    /// **client**: a malicious server can send a `LOCAL INFILE` request at any
    /// time, whatever capability the client claimed, so the refusal cannot be
    /// the absent bit alone.
    ///
    /// Two halves, named together because either alone reads green on a broken
    /// client: the call fails, *and* the only thing that went out is the empty
    /// packet the protocol terminates a transfer with — four bytes of header
    /// and no payload. A client that streamed the file would satisfy the first
    /// half by erroring afterwards.
    #[test]
    fn local_infile_is_refused_and_no_file_is_sent() {
        let mut wire = Wire::new(Peer::new(|_sent: &[u8]| Vec::new()));
        // 0xFB, then the path the server would like, with no length prefix.
        let mut request = vec![0xFB];
        request.extend_from_slice(b"/etc/passwd");
        wire.inbox.extend_from_slice(&packet(0, &request));

        let refused = read_ok(&mut wire, CLIENT_CAPABILITIES)
            .expect_err("a `LOCAL INFILE` request is not an answer this client accepts");

        assert_eq!(refused.kind(), io::ErrorKind::PermissionDenied);
        assert!(
            refused.to_string().contains("/etc/passwd"),
            "the refusal does not name the path that was asked for: {refused}"
        );
        let sent = everything_sent(wire.peer());
        assert_eq!(
            sent.len(),
            4,
            "the client wrote {} bytes where only an empty packet was owed",
            sent.len()
        );
        assert_eq!(
            &sent[..3],
            &[0, 0, 0],
            "the packet the client terminated the transfer with is not empty"
        );
    }

    /// ADR 0067 § 9's zone, as the numeric offset the section requires and
    /// never a name.
    ///
    /// A sweep rather than a line: every offset in use is a whole number of
    /// minutes, and the rendering has to hold at the sign boundary and either
    /// side of an hour.
    #[test]
    fn a_declared_zone_renders_as_a_numeric_offset_on_both_sides_of_utc() {
        assert_eq!(offset_literal(0), "+00:00");
        assert_eq!(offset_literal(3600), "+01:00");
        assert_eq!(offset_literal(-3600), "-01:00");
        assert_eq!(offset_literal(19800), "+05:30");
        assert_eq!(offset_literal(-12600), "-03:30");
        assert_eq!(offset_literal(50400), "+14:00");
        // A zone is not a clock: seconds under a minute have no rendering, so
        // they truncate toward zero rather than rounding a zone into another.
        assert_eq!(offset_literal(59), "+00:00");
        assert_eq!(offset_literal(-59), "-00:00");
    }

    /// The plugin gate is a roster and not a denylist: a plugin nobody has
    /// heard of is refused for the same reason a known-bad one is.
    #[test]
    fn an_unknown_authentication_plugin_is_refused_by_name() {
        let refused = super::plugin_or_refuse(b"auth_gssapi_client")
            .expect_err("a plugin off the roster is not answered");
        assert_eq!(refused.kind(), io::ErrorKind::ConnectionRefused);
        assert!(
            refused.to_string().contains("auth_gssapi_client"),
            "the refusal does not name the plugin: {refused}"
        );
    }

    /// The context every plugin runs against says the transport is encrypted,
    /// and that is a fact rather than an assertion: [`request_tls`] refuses
    /// every path to that point that is not.
    #[test]
    fn a_plugin_is_told_the_transport_is_encrypted_and_is_offered_no_server_key() {
        use mysql_common::auth::plugins::Context as _;

        let context = AuthContext {
            password: PASSWORD.as_bytes(),
            nonce: NONCE,
        };
        assert!(context.is_tls_transport());
        assert!(!context.is_ipc_transport());
        assert!(
            context.server_key_pem().is_none(),
            "an RSA key exchange was offered on a transport that is already encrypted"
        );
    }

    /// A `Greeting` is a value and its `Debug` is not a credential — the
    /// nonce is a server's challenge, not a secret, but the type is one a
    /// future field could be added to carelessly.
    #[test]
    fn a_greeting_renders_without_a_password_because_it_never_held_one() {
        let rendered = format!(
            "{:?}",
            Greeting {
                capabilities: CLIENT_CAPABILITIES,
                nonce: NONCE.to_vec(),
                plugin: super::CACHING_SHA2_PASSWORD.as_bytes().to_vec(),
                server_version: (8, 0, 36),
            }
        );
        assert!(!rendered.contains(PASSWORD));
        let target = format!("{:?}", target());
        assert!(
            !target.contains(PASSWORD),
            "a target rendered its password: {target}"
        );
    }
}
