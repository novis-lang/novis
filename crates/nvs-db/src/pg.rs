//! PostgreSQL: opening a socket, upgrading it in band, authenticating, and
//! running one statement at a time over the extended-query protocol.
//!
//! [ADR 0132 § 2](/docs/decisions/0132.md)'s
//! rule decides what is here and what is not. `postgres-protocol` frames every
//! message and owns the SCRAM mechanism itself — that is the borrowed codec, it
//! is fuzzed, and writing a fifth SASL implementation by hand is exactly the
//! trade this project already refused. **The sequencing is ours**: which
//! message follows which, what a park in the middle of one means, and which of
//! the answers a server may legally give this connection will accept.
//!
//! # Opening is three exchanges, and the first two are refusals waiting to
//! happen
//!
//! 1. **The upgrade.** [`request_tls`] writes § 3's `SSLRequest` — eight bytes,
//!    and the only plaintext this connection will ever carry — and reads the
//!    one-byte answer. `S` continues; **`N` is refused**, because [ADR 0067
//!    § 3](/docs/decisions/0067.md) makes `VerifyFull` the default
//!    with no spelling for turning it off, and PHP's `sslmode=prefer`
//!    silently connecting in the clear when the server says so is the exact
//!    behaviour that rule exists to remove.
//! 2. **The handshake**, `NvsTls::over` on the same `NvsTcp`. One TLS client in
//!    this tree, one answer to whose certificates it believes (`rule:security/one-tls-client`).
//! 3. **SASL**, and only SASL. A server that asks for a cleartext or an MD5
//!    password is refused rather than answered: those are the two exchanges
//!    that put the password itself, or a hash trivially replayable against the
//!    same salt, on the wire. `SCRAM-SHA-256` never sends the password in any
//!    form, and it authenticates the *server* to us as well.
//!
//! `AuthenticationOk` with no exchange at all — a `trust` line in the server's
//! `pg_hba.conf` — is accepted. Nothing of ours leaks in that case, and the
//! peer is already authenticated by step 2's certificate; refusing it would
//! only mean Novis cannot reach a database an operator deliberately configured.
//!
//! # Channel binding is `unsupported`, deliberately and not by omission
//!
//! `SCRAM-SHA-256-PLUS` binds the exchange to the TLS session by hashing the
//! server's certificate, which needs that certificate out of the `rustls`
//! session — something `NvsTls` does not hand back today. The spelling here is
//! [`ChannelBinding::unsupported`] (`n,,`) and **not** `unrequested` (`y,,`):
//! the second one asserts "the server does not offer binding", and a server
//! that does offer it reads that as a stripped-`PLUS` downgrade and fails the
//! authentication outright. Against a TLS-enabled PostgreSQL — which is every
//! connection this driver makes — `unrequested` cannot succeed.
//!
//! What that gives up is one layer of defence against a peer holding a
//! certificate that verifies for the name we asked for, which is a narrower
//! gap than it sounds: step 2 has already refused everything else.
//!
//! # Reading is one buffer per connection
//!
//! [`Wire`] is the stream and that buffer together, and it is generic in the
//! stream for one reason: the exchanges above are then assertable against a
//! server that computes its answers, with no socket and no certificate. A
//! connection's own wire is always the concrete `NvsTls<NvsTcp>`, which is what
//! the default type parameter says.
//!
//! [`Wire::read_message`] frames out of that buffer, reading more only when
//! what is there is not yet a whole message. A message can span reads, so the
//! tail of a short one has to survive to the next call, and the buffer is the
//! connection's rather than the call's — O(in-flight) per
//! `rule:programs/memory-priority`. It is filled
//! through a `resize` and a `truncate` rather than into uninitialised spare
//! capacity, because this crate forbids `unsafe`: that costs one `memset` of
//! the read window per syscall and buys the whole file having no unsafe block
//! to review.
//!
//! # A statement is one round trip, and the wire's state is the whole of it
//!
//! [`PgConn::query`] writes `Parse`, `Bind`, `Describe`, `Execute` and `Sync`
//! into one buffer and flushes them once. That is [ADR 0067
//! § 1](/docs/decisions/0067.md)'s "PostgreSQL's extended protocol
//! pays nothing extra" made literal: there is no prepare round trip to save,
//! because the prepare travels with the execution.
//!
//! The `Sync` at the end is what makes `rule:core-classes/db-connection-busy-state`'s per-driver call come out
//! in PostgreSQL's favour. It is already on the wire before any answer is read,
//! so the server *will* end this statement with a `ReadyForQuery` whatever
//! happens in between — a syntax error at `Parse`, a constraint violation at
//! `Execute`, or a caller that stops reading rows half way. Every one of those
//! is drained to that message and the connection goes back to [`State::Idle`],
//! poolable. **Poison is only ever a wire that failed**: a read that errored,
//! a frame that did not decode, a peer that closed. Those leave no boundary to
//! find, and § 4 closes such a connection rather than resetting it.
//!
//! [`PgRows`] is the statement's borrow of the connection, and dropping it
//! early is the ordinary abandonment case rather than an error: its `Drop`
//! drains to the same `ReadyForQuery`.
//!
//! A statement whose rows a *held* cursor walks cannot be a borrow — the
//! program advances it from a later call, with nothing borrowed in between — so
//! its read state is parked on the connection instead ([`PgConn::stream`]) and
//! [`State::Streaming`] is what refuses the second statement. [`PgCursor`] is
//! that state, both paths carry one, and [`next_row_of`] is the single place a
//! `DataRow` is read.
//!
//! # Parameters and results are in text format
//!
//! Both format lists in `Bind` are empty, which is the protocol's spelling for
//! "everything in text". [ADR 0067 § 9](/docs/decisions/0067.md)'s
//! type map is what forces it: `interval`, `hstore`, ranges, `inet`/`cidr` and
//! geometry all map to `tainted string` **as the server renders them**, and
//! there is no way to produce that rendering from a binary body short of
//! reimplementing the server's output functions. The types where exactness is
//! the point lose nothing — `numeric` and the integers are exact in text, and
//! `extra_float_digits` has defaulted to 3 since PostgreSQL 12, so a `float`
//! round-trips bit for bit. What it costs is parsing an integer out of ASCII
//! per column, which is nanoseconds against the syscall that carried it.
//!
//! **A text body is checked to be UTF-8 before it becomes a `string`**, and the
//! `client_encoding` in the startup message is not what makes that safe.
//! `rule:types/bytes`'s promise is read
//! *unchecked* downstream — `nvs_runtime::NvsStr::text_of` is that crate's one
//! unchecked read — and a database server is a network peer rather than a part
//! of this process: an ill-formed body from a compromised or simply
//! misconfigured server would be undefined behaviour several calls later, in a
//! crate that never saw a wire. The check is one pass over bytes already in
//! cache, and it is the only thing this driver spends on a value it is not
//! otherwise parsing.

use std::borrow::Cow;
use std::cell::Cell;
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::Instant;

use bytes::BytesMut;
use fallible_iterator::FallibleIterator;
use nvs_config::tree::Database;
use nvs_host::net::NvsTcp;
#[cfg(unix)]
use nvs_host::net::NvsUnix;
use nvs_host::tls::NvsTls;
use nvs_runtime::{Decimal, NotDecimal, NvsArray, NvsStr, Tag, Value};
use postgres_protocol::authentication::sasl::{ChannelBinding, ScramSha256};
use postgres_protocol::message::{backend, frontend};
use postgres_protocol::{IsNull, Oid};

use crate::conn::{
    BlockError, ColumnType, DbErrorKind, Driver, Endpoint, Isolation, PgConn, ServerError, State,
    written_value,
};
use crate::span::QuerySpan;
use crate::sql::{Prepared, StatementCache, statement_cache_for, time_zone_for};

/// The one mechanism this driver authenticates with.
const SCRAM_SHA_256: &str = "SCRAM-SHA-256";

/// How much room a read is given when the inbox holds no whole message.
///
/// A TLS record caps at 16 KiB, so a larger chunk buys no fewer syscalls; a
/// much smaller one costs a syscall per row on a wide result set.
const READ_CHUNK: usize = 16 * 1024;

/// Where one PostgreSQL server is, and who to be there.
///
/// The *resolved* form of a `[db.<name>]` block and not the block itself:
/// `nvs_config::tree::Database` is every field as an `Option<String>` as it was
/// written, and deciding which of them are present is the resolver's job rather
/// than the driver's.
///
/// The address and the name are two fields on purpose, and it is the same
/// split `nvs_host::tls` makes: `addr` is [ADR
/// 0058](/docs/decisions/0058.md)'s pinned address,
/// already resolved by whoever checked the capability, and `host` is the name
/// the certificate must be valid for. A driver that re-resolved the name would
/// be connecting somewhere nobody approved.
pub struct PgTarget<'a> {
    /// The name the server's certificate is checked against.
    pub host: &'a str,
    /// The role to log in as. `rule:core-classes/db-capabilities` accepts `tainted` here freely: it is
    /// a length-prefixed protocol field, never parsed text.
    pub user: &'a str,
    /// The role's password, used only to derive a SCRAM proof.
    pub password: &'a str,
    /// The database to attach to.
    pub database: &'a str,
    /// The PEM bundle whose anchors this server's certificate is verified
    /// against, or the compiled-in Mozilla set where the block names none.
    ///
    /// [ADR 0067 § 3](/docs/decisions/0067.md) has no spelling for
    /// turning verification off and this is not one: what it changes is *whose*
    /// certificates are believed, never whether they are checked. The path
    /// arrives absolute and already inside `rule:config/ownership-is-the-trust-boundary`'s trust boundary —
    /// `nvs_config::db` resolves it at boot — so the connect path opens it and
    /// asks nothing further about it.
    pub tls_ca_file: Option<&'a Path>,
    /// The zone a zone-less `TIMESTAMP` column is read in, as a whole number
    /// of seconds east of UTC.
    ///
    /// [ADR 0067 § 9](/docs/decisions/0067.md)'s declared zone:
    /// `time_zone` in the connection's config block, `timeZone` in `Settings`,
    /// and `0` — UTC — with neither set. It is sent to the server as well as
    /// read here, so `CURRENT_TIMESTAMP` and a decoded column agree about
    /// which zone they are in.
    ///
    /// [`crate::sql::time_zone_for`] is what turns the written field into this
    /// number, and it is the same reader for every driver: what arrives here is
    /// already seconds, so this path never sees a zone name.
    pub time_zone: i32,
    /// How many prepared statements this connection may keep alive on the
    /// server, [ADR 0067 § 1](/docs/decisions/0067.md)'s
    /// `statement_cache`.
    ///
    /// Resolved the same way the zone is, and by the same rule about who
    /// decides: [`statement_cache_for`] reads the `[db.<name>]` block and
    /// answers a number, so the connect path below never sees an absent
    /// field. `0` is the cache off, which is a supported size and not a
    /// caller that forgot to fill this in.
    pub statement_cache: usize,
}

/// The port an absent `port` in a `[db.<name>]` block means — PostgreSQL's
/// own, and a fact about this protocol rather than about whoever opens the
/// socket.
///
/// Not a field of [`PgTarget`] for the reason the address is not one: a port is
/// half of an address, and the address is resolved by the caller that checked
/// the `db.connect` capability. What this crate owes that caller is the number
/// to fall back to.
pub const DEFAULT_PORT: u16 = 5432;

impl std::fmt::Debug for PgTarget<'_> {
    /// Everything but the password, which is a `secret` at the language level
    /// (`rule:core-classes/db-capabilities`) and is not printed in any rendering.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgTarget")
            .field("host", &self.host)
            .field("user", &self.user)
            .field("database", &self.database)
            .field("time_zone", &self.time_zone)
            .field("statement_cache", &self.statement_cache)
            .finish_non_exhaustive()
    }
}

impl<'a> PgTarget<'a> {
    /// One `[db.<name>]` block as this driver's target, or why it is not one.
    ///
    /// **The target borrows the block and copies nothing**, which is what the
    /// `'a` is for: a resolved target has to outlive the [`PgConn::connect`]
    /// call that opens with it, and the thing that already does is the
    /// configuration snapshot the block lives in. Owning the strings instead
    /// would mean a second copy of the password — a `secret` at the language
    /// level ([ADR 0067 § 3](/docs/decisions/0067.md)) — in a struct
    /// nothing zeroes, for no gain: `nvs_config`'s snapshot is held for the
    /// whole of a boot generation and a connection is opened inside one.
    ///
    /// **Every field is decided here and none of it in the connect path.**
    /// [`statement_cache_for`] and [`crate::sql::time_zone_for`] are
    /// the two readers that own what an absent field means, and this is where
    /// their answers become a number; § 9's `None` — a zone that is not an
    /// offset — becomes [`BlockError::TimeZone`] rather than UTC, which is the
    /// refusal that reader's doc comment promises somebody makes.
    ///
    /// The address is not here. § 3 pre-approves an operator-written endpoint
    /// and `rule:http-server/allow-url-pins-the-address` pins
    /// it, so `host`'s resolution to a [`std::net::SocketAddr`] belongs to
    /// whoever checked the `db.connect` capability, and [`PgConn::connect`]
    /// takes the [`Endpoint`] that came out of it beside this target. What is
    /// here is the name the certificate is checked against, which is the
    /// written `host` and never a reverse lookup — and a `host` that is a
    /// socket directory names no certificate at all, which is `PgStream`'s
    /// local arm.
    ///
    /// # Errors
    ///
    /// [`BlockError`], in the order the checks run: the `driver` first, since a
    /// MySQL block resolved as PostgreSQL would send this handshake to a server
    /// that cannot answer it; then a field belonging to another driver; then
    /// every field the startup exchange sends, each by its own key; then
    /// § 9's zone.
    pub fn resolve(block: &'a Database) -> Result<PgTarget<'a>, BlockError<'a>> {
        let written = block.driver.as_deref().ok_or(BlockError::NoDriver)?;
        match Driver::from_config_name(written) {
            Some(Driver::Postgres) => {}
            Some(driver) => {
                return Err(BlockError::OtherDriver {
                    written,
                    driver,
                    expected: Driver::Postgres,
                });
            }
            None => return Err(BlockError::UnknownDriver { written }),
        }

        if block.path.is_some() {
            return Err(BlockError::Unusable {
                field: "path",
                expected: Driver::Postgres,
            });
        }

        let password = match (block.password.as_deref(), block.password_file.is_some()) {
            // `nvs_config::secret` materializes the file's content into
            // `password` and leaves `password_file` set, so a block with the
            // file and no value is one that never went through that pass.
            (None, true) => return Err(BlockError::SecretUnread),
            // A password may legitimately be spaces — `nvs_config::secret`
            // trims nothing off a secret file for exactly that reason — so this
            // one field is empty only when it is *empty*.
            (Some(""), _) => return Err(BlockError::Blank { field: "password" }),
            (Some(password), _) => password,
            (None, false) => {
                return Err(BlockError::Missing {
                    field: "password",
                    expected: Driver::Postgres,
                });
            }
        };

        let host = written_value(block.host.as_deref(), "host", Driver::Postgres)?;
        let user = written_value(block.user.as_deref(), "user", Driver::Postgres)?;
        let database = written_value(block.database.as_deref(), "database", Driver::Postgres)?;

        let Some(time_zone) = time_zone_for(block) else {
            return Err(BlockError::TimeZone {
                // `time_zone_for` answers `Some(0)` for an absent field, so
                // reaching here means the block wrote one.
                written: block.time_zone.as_deref().unwrap_or_default(),
            });
        };

        Ok(PgTarget {
            host,
            user,
            password,
            database,
            // Absolute and trust-checked by `nvs_config::db` before the block
            // reached here, so there is nothing for this resolver to decide:
            // written or not written is the whole of it.
            tls_ca_file: block.tls_ca_file.as_deref().map(Path::new),
            time_zone,
            statement_cache: statement_cache_for(block),
        })
    }
}

/// What a *second* connection has to send to cancel this one's query.
///
/// PostgreSQL's only cancellation path, and the key is unrepeatable: it arrives
/// once, in the `BackendKeyData` of the startup exchange, over the connection
/// it cancels. A driver that dropped it could not ask again, so this is read
/// during the handshake and kept whether or not anything has asked to cancel
/// yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancelKey {
    /// The backend process ID.
    pub process_id: i32,
    /// The secret that makes the cancel request this connection's own.
    pub secret_key: i32,
}

/// The socket underneath a PostgreSQL connection: TLS over TCP, or a
/// Unix-domain socket carrying the protocol in the clear.
///
/// [`crate::conn::Endpoint`]'s two transports, arriving here as the two things a
/// [`Wire`] can be framed over. The message framing has no opinion about which
/// it is, which is why this is a stream and not a second codec, and
/// [`crate::mysql::MyStream`] is the same type under the other driver.
///
/// **There is no TLS over the local arm, and that is not a downgrade a server
/// can ask for.** A path reaches this driver only where an operator wrote one
/// into root-owned configuration
/// (`rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it`), it
/// names no host for a certificate to be valid for, and the bytes never leave
/// the machine — the socket's own file permissions are the boundary TLS would
/// otherwise be standing in for. § 3's in-band upgrade is still mandatory on the
/// TCP arm, which is the one with a network in it, and the transport is decided
/// before the first message goes out rather than negotiated.
///
/// The local arm is `#[cfg(unix)]`, as [`crate::conn::Endpoint`]'s socket arm
/// is, so this match is exhaustive on both platforms with no arm that exists
/// only to refuse.
///
/// `clippy::large_enum_variant` is allowed, for the reason and at the cost
/// [`crate::mysql::MyStream`] gives.
#[allow(clippy::large_enum_variant)]
pub enum PgStream {
    /// A server that named a host, reached over TCP and upgraded in band.
    Tls(NvsTls<NvsTcp>),
    /// A server that named a directory, reached over the socket file
    /// [`socket_endpoint`] derived from it.
    #[cfg(unix)]
    Local(NvsUnix),
}

impl Read for PgStream {
    /// Whatever has arrived, from whichever socket this is.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Tls(stream) => stream.read(buf),
            #[cfg(unix)]
            Self::Local(stream) => stream.read(buf),
        }
    }
}

impl Write for PgStream {
    /// As much of `buf` as the socket took.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Tls(stream) => stream.write(buf),
            #[cfg(unix)]
            Self::Local(stream) => stream.write(buf),
        }
    }

    /// Pushes what is held, which on the TLS arm is a record not yet sealed.
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Tls(stream) => stream.flush(),
            #[cfg(unix)]
            Self::Local(stream) => stream.flush(),
        }
    }
}

impl std::fmt::Debug for PgStream {
    /// Which transport this is, and nothing about the socket: a `rustls` session
    /// holds key material and a buffered message is one request's data, which is
    /// [`Wire`]'s own `Debug` and its reason.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tls(_) => f.write_str("PgStream::Tls"),
            #[cfg(unix)]
            Self::Local(_) => f.write_str("PgStream::Local"),
        }
    }
}

impl nvs_host::net::Deadline for PgStream {
    /// Bounds every wait on this connection, whichever socket carries it.
    fn set_deadline(&mut self, at: Option<Instant>) {
        match self {
            Self::Tls(stream) => stream.set_deadline(at),
            #[cfg(unix)]
            Self::Local(stream) => stream.set_deadline(at),
        }
    }

    fn deadline(&self) -> Option<Instant> {
        match self {
            Self::Tls(stream) => stream.deadline(),
            #[cfg(unix)]
            Self::Local(stream) => stream.deadline(),
        }
    }
}

/// The path a `[db.<name>] host` naming a **directory** means, which is the
/// socket PostgreSQL's own engine binds inside it.
///
/// `rule:core-classes/db-unix-socket-path`: `/var/run/postgresql` with
/// `port = 5432` is `/var/run/postgresql/.s.PGSQL.5432`, because that is what
/// libpq, `psql` and PDO take, and the string an operator's deployment already
/// holds is the directory. The port is part of the *name* here rather than of
/// an address — one server per port, and a machine running two engines has two
/// files in one directory — which is why this takes it where
/// [`crate::conn::socket_endpoint`], the as-written spelling
/// [`crate::mysql`] uses, does not.
///
/// # Errors
///
/// Nothing on this platform; the signature is the one
/// [`crate::conn::socket_endpoint`]'s refusal needs on a build with no
/// `AF_UNIX` transport, which this delegates to rather than wording a second
/// time.
#[cfg(unix)]
pub fn socket_endpoint(directory: &str, port: u16) -> io::Result<crate::conn::Endpoint> {
    let mut path = std::path::PathBuf::from(directory);
    path.push(format!(".s.PGSQL.{port}"));
    Ok(crate::conn::Endpoint::Socket(path))
}

/// The refusal a build with no `AF_UNIX` transport answers a socket directory
/// with — [`crate::conn::socket_endpoint`]'s, wording it once for every driver.
///
/// # Errors
///
/// `Unsupported`, always.
#[cfg(not(unix))]
pub fn socket_endpoint(directory: &str, _port: u16) -> io::Result<crate::conn::Endpoint> {
    crate::conn::socket_endpoint(directory)
}

/// A PostgreSQL connection's stream, and what has been read off it.
///
/// Generic in the stream so the exchanges above can be asserted against a
/// server that answers rather than a socket; a real connection's wire is the
/// [`PgStream`] the default names, and nothing constructs any other outside
/// this module's tests.
pub(crate) struct Wire<S: Read + Write = PgStream> {
    stream: S,
    inbox: BytesMut,
}

impl<S: Read + Write> std::fmt::Debug for Wire<S> {
    /// How much is buffered, and nothing about the stream: a `rustls` session
    /// holds key material, and rows in flight are one request's data.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wire")
            .field("buffered", &self.inbox.len())
            .finish_non_exhaustive()
    }
}

impl<S: Read + Write> Wire<S> {
    /// A wire over a stream that is already connected and already encrypted.
    fn new(stream: S) -> Wire<S> {
        Wire {
            stream,
            inbox: BytesMut::new(),
        }
    }

    /// The stream underneath, for the tests that assert on what was written to
    /// it. There is no non-test reader: a driver talks through the two methods
    /// below, and reaching past them is how a half-message gets written.
    #[cfg(test)]
    fn peer(&self) -> &S {
        &self.stream
    }

    /// Bounds every wait on this wire by `at`, or lifts the bound.
    ///
    /// `rule:core-classes/db-statement-members`'s statement deadline, filed exactly where
    /// [`PgConn::connect`]'s handshake deadline already is — one clock, on the
    /// thing that waits. It bounds the *conversation* and not a call: a
    /// statement is a prepare, an execute and every row of the answer over one
    /// socket, so a caller sets the deadline before that exchange and lifts it
    /// after.
    ///
    /// The bound is on the method rather than on [`Wire`] itself because the
    /// exchanges here are asserted against a scripted stream that never waits,
    /// and a stream that never waits has no clock to keep.
    pub(crate) fn set_deadline(&mut self, at: Option<std::time::Instant>)
    where
        S: nvs_host::net::Deadline,
    {
        self.stream.set_deadline(at);
    }

    /// Writes everything buffered in `out`, empties it, and flushes.
    ///
    /// One function because a half-written message is the shape `rule:core-classes/db-connection-busy-state`
    /// calls poison, and `write_all` is the only spelling that cannot leave
    /// one. `out` is emptied whether or not the write succeeded: what it holds
    /// after a failure is a fragment nothing may send later.
    ///
    /// # Errors
    ///
    /// Whatever the stream reported.
    pub(crate) fn send(&mut self, out: &mut BytesMut) -> io::Result<()> {
        let result = self
            .stream
            .write_all(out)
            .and_then(|()| self.stream.flush());
        out.clear();
        result
    }

    /// Reads until the inbox holds a whole message, and takes it.
    ///
    /// Bytes that arrived after this message — or the head of one that has not
    /// finished arriving — are still buffered for the next call.
    ///
    /// # Errors
    ///
    /// `UnexpectedEof` when the peer closed mid-message, `InvalidData` for a
    /// frame that does not decode, and whatever the stream reported.
    pub(crate) fn read_message(&mut self) -> io::Result<backend::Message> {
        loop {
            if let Some(message) = backend::Message::parse(&mut self.inbox)? {
                return Ok(message);
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
                    "the server closed the connection part-way through a message",
                ));
            }
        }
    }
}

impl PgConn {
    /// Opens a connection: the transport [`Endpoint`] names, § 3's in-band
    /// upgrade where there is a network to upgrade, then SASL.
    ///
    /// A socket endpoint is **dialled as it stands**: the directory an operator
    /// wrote became `<directory>/.s.PGSQL.<port>` in [`socket_endpoint`], which
    /// is where `rule:core-classes/db-unix-socket-path`'s derivation lives, so
    /// nothing here rewrites a path either.
    ///
    /// `deadline` bounds the whole of that and not one leg of it — the connect,
    /// the handshake and every authentication round trip share one clock,
    /// filed on the socket where `nvs-host` keeps it. `None` is unbounded and
    /// is for a caller that has its own bound. **The clock is lifted before the
    /// connection is handed back**, so a statement run later takes the deadline
    /// its own caller files and never the leftover of getting here.
    ///
    /// The connection comes back [`State::Idle`]: at a message boundary, with
    /// a statement allowed.
    ///
    /// # Errors
    ///
    /// `ConnectionRefused` when the server will not upgrade to TLS or offers no
    /// SASL mechanism this driver accepts, `InvalidData` for a message that is
    /// not what the protocol allows at that point — or for a `tls_ca_file` that
    /// holds no certificate — `TimedOut` when the deadline
    /// passes, and whatever the socket or the TLS handshake itself reported. A
    /// refusal the *server* worded — a wrong password, a database that does not
    /// exist — carries its own `SQLSTATE` and message.
    pub fn connect(
        endpoint: impl Into<Endpoint>,
        target: &PgTarget<'_>,
        deadline: Option<Instant>,
    ) -> io::Result<PgConn> {
        let stream = match endpoint.into() {
            Endpoint::Tcp(addr) => {
                let mut tcp = match deadline {
                    Some(at) => {
                        NvsTcp::connect_timeout(addr, at.saturating_duration_since(Instant::now()))?
                    }
                    None => NvsTcp::connect(addr)?,
                };
                tcp.set_deadline(deadline);

                request_tls(&mut tcp)?;
                PgStream::Tls(match target.tls_ca_file {
                    Some(bundle) => NvsTls::over_bundle(tcp, target.host, bundle)?,
                    None => NvsTls::over(tcp, target.host)?,
                })
            }
            // Nothing is upgraded on this arm, so no `SSLRequest` goes out:
            // asking for TLS over a socket that never leaves the machine would
            // refuse every engine built without it, for a threat this transport
            // does not have — `PgStream` owns that argument in full.
            #[cfg(unix)]
            Endpoint::Socket(path) => {
                let mut local = match deadline {
                    Some(at) => NvsUnix::connect_timeout(
                        path,
                        at.saturating_duration_since(Instant::now()),
                    )?,
                    None => NvsUnix::connect(path)?,
                };
                local.set_deadline(deadline);
                PgStream::Local(local)
            }
        };
        let mut wire = Wire::new(stream);
        let startup = authenticate(&mut wire, target)?;
        // The handshake's budget ends with the handshake. `NvsTcp::connect_timeout`
        // lifts its own bound before it hands a stream back, and the one re-filed
        // above — which is what carries the budget across the upgrade and the
        // authentication round trips — is lifted here for the same reason: a
        // statement a caller runs later files its own through [`Wire::set_deadline`].
        // Left filed, this is a clock the caller never set, and it expires the
        // connection a fixed time after opening it. It bites only on a read that has
        // to wait, so a caller that files no deadline of its own — `nvs-cli`'s queue
        // worker, and the `schema::opened` behind `nvs queue migrate` and `nvs schema
        // apply` — keeps working on an idle machine and loses its connection under load.
        wire.set_deadline(None);

        Ok(PgConn {
            wire,
            state: Cell::new(State::Idle),
            cancel: startup.cancel,
            // What the startup exchange already said — see the field.
            server_version: startup.server_version,
            // `rule:core-classes/db-one-api`'s size, already read off the `[db.<name>]` block by
            // `statement_cache_for` and carried here on the target —
            // this path takes a number and has no opinion about where an
            // unwritten field's default comes from.
            cache: StatementCache::new(target.statement_cache),
            // § 9's zone-less row is decoded a layer up, and the target is
            // gone by then — see the field.
            time_zone: target.time_zone,
            depth: Cell::new(0),
            reading: None,
        })
    }

    /// Bounds every wait on this connection by `at`, or lifts the bound.
    ///
    /// `rule:core-classes/db-statement-members`'s statement deadline for a
    /// caller holding a `PgConn` itself. [`crate::Connection::set_deadline`] is
    /// the same clock reached through the enum and owns what a deadline bounds,
    /// why it outlives the call that filed it, and why every statement path
    /// files its own — `None` included.
    pub fn set_deadline(&mut self, at: Option<Instant>) {
        self.wire.set_deadline(at);
    }

    /// The zone a zone-less `TIMESTAMP` off this connection is read in, as
    /// seconds east of UTC — [`PgTarget::time_zone`], and the same number the
    /// handshake sent the server.
    ///
    /// `nvs-stdlib` asks: [`PgScalar::Timestamp`] carries the civil fields the
    /// server rendered and no offset at all, because § 9 reads that row in the
    /// zone the *connection* declared rather than one the column carries.
    #[must_use]
    pub fn time_zone(&self) -> i32 {
        self.time_zone
    }

    /// What a second connection needs to cancel this one's running statement.
    #[must_use]
    pub fn cancel_key(&self) -> CancelKey {
        self.cancel
    }

    /// Runs one statement, borrowing the connection until its rows are drained.
    ///
    /// `params` are `rule:core-classes/db-parameters`'s bound values, each already rendered in the
    /// text format the module doc chose, and `None` is SQL `NULL`. Nothing is
    /// interpolated into `sql` — this signature is the shape of the goal's
    /// standing decision that emulated prepares do not exist in any form, and
    /// there is no second entry point that takes a formatted string.
    ///
    /// The connection is [`State::Executing`] while the portal is in flight and
    /// [`State::Streaming`] until the returned [`PgRows`] is drained or
    /// dropped; both of those return it to [`State::Idle`].
    ///
    /// # Errors
    ///
    /// `InvalidInput` when the connection is not [`State::Idle`] — `rule:core-classes/db-statement-members`'s second concurrent statement, which `nvs-stdlib` words as a
    /// `LogicError`. Otherwise the server's own error, which leaves the
    /// connection idle and poolable, or a wire failure, which poisons it.
    pub fn query(&mut self, sql: &str, params: &[Option<&[u8]>]) -> io::Result<PgRows<'_>> {
        start_statement(&mut self.wire, &self.state, &mut self.cache, sql, params)
    }

    /// Runs one statement and leaves its portal open, **borrowing nothing**:
    /// the read state is parked on this connection and the rows come off it one
    /// [`Self::stream_next_row`] at a time.
    ///
    /// The wire half of `rule:core-classes/db-statement-members`'s `stream`. It is the same batch
    /// [`Self::query`] sends and reaches [`State::Streaming`] the same way; the
    /// only difference is where the state a row is read against lives, and
    /// [`PgCursor`] owns why that has to be here rather than in a borrow. The
    /// answer is what the portal described, empty for a statement returning no
    /// rows, and [`Self::stream_columns`] hands the same slice back to the later
    /// calls that decode against it.
    ///
    /// A second statement is refused while this one is open — that is § 4's
    /// `LogicError`, and it is [`State::Streaming`] that says so rather than a
    /// lifetime. [`Self::end_stream`] is the abandonment [`PgRows`] gets from
    /// `Drop`; a stream read to its end needs no call at all.
    ///
    /// # Errors
    ///
    /// As [`Self::query`].
    pub fn stream(&mut self, sql: &str, params: &[Option<&[u8]>]) -> io::Result<&[PgColumn]> {
        let reading = open_portal(&mut self.wire, &self.state, &mut self.cache, sql, params)?;
        Ok(&self.reading.insert(reading).columns)
    }

    /// What the parked stream's portal described, or `None` for a connection
    /// that has not streamed since its last reset.
    ///
    /// It outlives the rows on purpose: the decode of the last row happens after
    /// the walk that produced it, and § 9's type map is read off these.
    #[must_use]
    pub fn stream_columns(&self) -> Option<&[PgColumn]> {
        Some(&self.reading.as_ref()?.columns)
    }

    /// The next row of the parked stream, or `None` once it has ended — and
    /// `None` too for a connection with no stream parked on it at all.
    ///
    /// Ending it returns the connection to [`State::Idle`], exactly as
    /// [`PgRows::next_row`] does. The state itself stays parked, holding the tag
    /// and the span the statement finished with, until the next
    /// [`Self::stream`] replaces it or [`Self::end_stream`] drops it.
    ///
    /// # Errors
    ///
    /// As [`PgRows::next_row`].
    pub fn stream_next_row(&mut self) -> io::Result<Option<PgRow>> {
        let Some(reading) = self.reading.as_mut() else {
            return Ok(None);
        };
        next_row_of(&mut self.wire, &self.state, reading)
    }

    /// `rule:observability/a-query-is-a-trace-event`'s trace event for the parked stream, or `None` where there
    /// is none — [`PgRows::span`] for what a caller reading one mid-stream gets.
    #[must_use]
    pub fn stream_span(&self) -> Option<&QuerySpan> {
        Some(&self.reading.as_ref()?.span)
    }

    /// Names the `[db.<name>]` block the parked stream is running on, and does
    /// nothing where there is no stream — [`PgRows::name_connection`] owns why
    /// the driver cannot work the name out for itself.
    pub fn name_stream_connection(&mut self, connection: &str) {
        if let Some(reading) = self.reading.as_mut() {
            reading.span.name(connection);
        }
    }

    /// Abandons the parked stream: drains to the `ReadyForQuery` its `Sync`
    /// guaranteed and forgets what it read.
    ///
    /// This is [`PgRows`]' `Drop` written as a call, and for the same reason —
    /// a cursor a program stopped walking is ordinary, and draining is what
    /// keeps the connection poolable instead of poisoned. Best effort by the
    /// same argument `Drop` makes: a read that fails on the way poisons the
    /// connection through `drain_to_ready`, and the pool refuses it there.
    pub fn end_stream(&mut self) {
        if self.state.get() == State::Streaming {
            drop(drain_to_ready(&mut self.wire, &self.state));
        }
        self.reading = None;
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s `executeMany`: one
    /// prepare, one execution per member of `sets`, and their affected counts
    /// summed.
    ///
    /// Each member of `sets` is one execution's parameters in [`Self::query`]'s
    /// shape, and they must all be the same length — the prepare is one
    /// prepare. Nothing streams: the connection is [`State::Idle`] again by the
    /// time this answers, so there is no handle to drain and no second
    /// statement to refuse.
    ///
    /// [`execute_many`] owns the two rules a caller can be surprised by: an
    /// execution is its own transaction, so a failure part way through does not
    /// undo the writes before it, and an empty `sets` is § 4's no-op answering
    /// `0`.
    ///
    /// # Errors
    ///
    /// As [`execute_many`].
    pub fn execute_many(&mut self, sql: &str, sets: &[&[Option<&[u8]>]]) -> io::Result<u64> {
        execute_many(&mut self.wire, &self.state, &mut self.cache, sql, sets)
    }

    /// [ADR 0067 § 7](/docs/decisions/0067.md)'s `BEGIN`, or the
    /// `SAVEPOINT` a nested `transaction()` is.
    ///
    /// This is the driver half of § 7 and nothing more: the callable, the
    /// rollback-only flag and the retry rule are `nvs-stdlib`'s, and what a
    /// driver owes them is the commands that leave the connection at a message
    /// boundary. [`begin`] owns which command a given nesting depth gets and
    /// [`begin_command`] the rendering of the two options, including which of
    /// § 7's five isolation levels PostgreSQL spells with another name.
    ///
    /// **It answers the span of the command it sent**, which is `rule:observability/a-query-is-a-trace-event`'s
    /// event for a statement that lends no [`PgRows`] out — the shape
    /// `PgConn::execute_many`'s caller builds for itself. The caller cannot
    /// build this one: which command a level gets is the depth's answer and the
    /// depth is this connection's, so the text only exists down here.
    ///
    /// # Errors
    ///
    /// As [`begin`].
    pub fn begin(
        &mut self,
        isolation: Option<Isolation>,
        read_only: bool,
    ) -> io::Result<QuerySpan> {
        begin(
            &mut self.wire,
            &self.state,
            &self.depth,
            isolation,
            read_only,
        )
    }

    /// How many transaction levels are open on this connection — 0 outside one,
    /// 1 inside an outermost `transaction()`, deeper inside a nested one.
    ///
    /// `nvs-stdlib` asks, and § 7's retry rule is the only reason this is
    /// public: a serialization failure is retried **only for an outermost
    /// transaction**, because re-running the callable of a nested one would
    /// re-run it inside an outer transaction the conflict already aborted. The
    /// caller cannot tell the two apart on its own — [`begin`] owns the nesting
    /// and deliberately gives the two cases the same signature — so it reads the
    /// depth *before* the level is opened and retries only what it saw at zero.
    ///
    /// Not a guess about the *server's* state: the count moves only after a
    /// command the server accepted (see [`begin`]), and [`reset`] zeroes it.
    ///
    /// [`reset`]: PgConn::reset
    #[must_use]
    pub fn depth(&self) -> u32 {
        self.depth.get()
    }

    /// § 7's `COMMIT`, or the `RELEASE SAVEPOINT` closing a nested one — a
    /// normal return out of the callable either way.
    ///
    /// # Errors
    ///
    /// As [`commit`]. A refused outermost commit is § 7's failed commit, which
    /// `nvs-stdlib` throws as `DbError`. The span it answers with on success is
    /// [`PgConn::begin`]'s, for the same reason.
    pub fn commit(&mut self) -> io::Result<QuerySpan> {
        commit(&mut self.wire, &self.state, &self.depth)
    }

    /// § 7's `ROLLBACK`, or the `ROLLBACK TO SAVEPOINT` undoing a nested one —
    /// a throw out of the callable, or `rollBack`'s own signal.
    ///
    /// # Errors
    ///
    /// As [`roll_back`]. The span it answers with on success is
    /// [`PgConn::begin`]'s, for the same reason.
    pub fn roll_back(&mut self) -> io::Result<QuerySpan> {
        roll_back(&mut self.wire, &self.state, &self.depth)
    }

    /// `rule:security/db-pool-reset-is-a-boundary`'s reset, and the connection back only if it worked.
    ///
    /// **`self` by value is the enforcement**, not a convenience. § 13 makes the
    /// reset a security boundary: a connection that cannot be proven clean is
    /// closed, never returned to the pool, because one request's session state
    /// read by the next is a cross-tenant leak. A `&mut self` signature would
    /// leave the caller holding a connection it must remember not to reuse, and
    /// this one hands it back only on the path where it is provably clean —
    /// on the other, `self` is dropped here and [`Drop`] says `Terminate`.
    ///
    /// Deliberately not expressed as [`State::Poisoned`], which answers a
    /// different question: a refused `RESET ALL` still leaves the wire at a
    /// known message boundary, so poisoning it would weaken the one thing that
    /// state means.
    ///
    /// # Errors
    ///
    /// The first command the server refused, or the wire failure that stopped
    /// the batch. Either way the connection is gone by the time the caller sees
    /// it.
    pub fn reset(mut self) -> io::Result<PgConn> {
        reset_session(&mut self.wire, &self.state)?;
        // § 13's first command is `ROLLBACK`, so every level § 7 opened is
        // closed by the time this returns — including the savepoints inside
        // one, which do not outlive the transaction that held them.
        self.depth.set(0);
        // "No open cursor" is one of the properties § 13 states the reset has to
        // establish, and a stream parked here is one request's statement shape
        // and command tag: the next request must not be able to read either.
        self.reading = None;
        Ok(self)
    }
}

impl Drop for PgConn {
    /// Says goodbye rather than vanishing — [`say_goodbye`] has the reasoning.
    fn drop(&mut self) {
        say_goodbye(&mut self.wire);
    }
}

/// The last thing a destroyed connection writes: `Terminate`, best effort.
///
/// It lets the backend exit on its own instead of discovering a reset socket,
/// which is one fewer error line in the server's log per connection and one
/// backend freed a round trip earlier. Best effort by definition: the
/// connection is going away whatever the write reports, so there is nobody
/// left to tell.
///
/// Free and generic in the stream rather than written inside [`Drop`] for the
/// reason every other sequence in this module is: `PgConn`'s wire is a
/// `Wire<PgStream>`, which no unit test can build, so what a *destroyed*
/// connection puts on the wire would otherwise be unassertable without a socket
/// and a certificate. `a_failed_reset_destroys_the_connection_rather_than_returning_it`
/// is what asserts it.
fn say_goodbye<S: Read + Write>(wire: &mut Wire<S>) {
    let mut out = BytesMut::new();
    frontend::terminate(&mut out);
    drop(wire.send(&mut out));
}

/// Asks for § 3's in-band upgrade and reads the one-byte answer.
///
/// Writes `SSLRequest` and returns once the server has agreed. The caller
/// wraps the same stream in TLS immediately: every byte after this one is a
/// record, which is why nothing else may be written in between.
///
/// # Errors
///
/// `ConnectionRefused` when the server answers `N` (no TLS configured) or `E`
/// (it is old enough not to understand the request), `InvalidData` for any
/// other byte, and `UnexpectedEof` when it answers nothing at all.
fn request_tls<S: Read + Write>(stream: &mut S) -> io::Result<()> {
    let mut out = BytesMut::new();
    frontend::ssl_request(&mut out);
    // Written straight to the socket rather than through a `Wire`: this runs
    // before the upgrade, and a `Wire` is by definition the encrypted side.
    stream.write_all(&out)?;
    stream.flush()?;

    let mut answer = [0u8; 1];
    stream.read_exact(&mut answer)?;
    match answer[0] {
        b'S' => Ok(()),
        b'N' => Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "the server offers no TLS, and `rule:core-classes/db-capabilities`'s VerifyFull has no spelling for connecting \
             without it: configure `ssl = on` on the server",
        )),
        b'E' => Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "the server refused an SSLRequest outright, which is what a pre-8.0 PostgreSQL does",
        )),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("the answer to an SSLRequest was {other:#04x}, not S or N"),
        )),
    }
}

/// A fixed UTC offset, spelled as PostgreSQL's own numeric `TimeZone` value.
///
/// [ADR 0067 § 9](/docs/decisions/0067.md) requires a numeric offset
/// and never a zone name, and PostgreSQL's numeric spelling is a POSIX one:
/// `<+02>-02` is two hours *east* of UTC, because a POSIX `TZ` string counts
/// its offset westwards while the abbreviation inside the brackets is free
/// text and carries the sign a reader expects. This is byte for byte what the
/// server builds for `SET TIME ZONE INTERVAL`, which is the reason for the
/// shape: a bare `+02:00` is not a POSIX zone at all, and a startup parameter
/// the server cannot parse fails the connection rather than an hour of it.
///
/// The minute and second fields are written only when the offset has them,
/// which is again the server's own rule for the same string.
fn posix_time_zone(offset: i32) -> String {
    let magnitude = offset.unsigned_abs();
    let (hours, minutes, seconds) = (magnitude / 3600, magnitude % 3600 / 60, magnitude % 60);

    let mut spelled = format!("{hours:02}");
    if minutes != 0 || seconds != 0 {
        spelled.push_str(&format!(":{minutes:02}"));
    }
    if seconds != 0 {
        spelled.push_str(&format!(":{seconds:02}"));
    }

    if offset < 0 {
        format!("<-{spelled}>+{spelled}")
    } else {
        format!("<+{spelled}>-{spelled}")
    }
}

/// Runs the startup exchange over an already-encrypted stream.
///
/// Returns once the server has reported `ReadyForQuery`, which is the point the
/// wire is at a message boundary and [`State::Idle`] is true.
///
/// # Errors
///
/// As [`PgConn::connect`], minus the socket and TLS legs.
fn authenticate<S: Read + Write>(wire: &mut Wire<S>, target: &PgTarget<'_>) -> io::Result<Startup> {
    let mut out = BytesMut::new();
    let time_zone = posix_time_zone(target.time_zone);
    frontend::startup_message(
        [
            ("user", target.user),
            ("database", target.database),
            // `rule:core-classes/db-capabilities`'s third default: text columns arrive as valid UTF-8
            // by construction rather than by inspection, which is what ADR
            // 0009's guarantee needs.
            ("client_encoding", "UTF8"),
            // § 9's structured rows are read positionally out of the text
            // format, so the two parameters that decide what that text looks
            // like are pinned here rather than discovered from whatever the
            // server was configured with: `DateStyle` fixes the rendering as
            // ISO 8601, and `TimeZone` fixes the offset a `TIMESTAMPTZ` is
            // rendered at — an offset that is then always present, always
            // numeric and never an abbreviation this driver would have to
            // know the rules of.
            ("DateStyle", "ISO"),
            // § 9's zone-less half. It is sent as well as decoded so that
            // `CURRENT_TIMESTAMP` and a `TIMESTAMP` column agree.
            ("TimeZone", time_zone.as_str()),
            // Not decoration: this is what an operator reads in
            // `pg_stat_activity` when a query is holding a lock.
            ("application_name", "novis"),
        ],
        &mut out,
    )?;
    wire.send(&mut out)?;

    let mut scram: Option<ScramSha256> = None;
    let mut cancel: Option<CancelKey> = None;
    let mut version: Option<String> = None;

    loop {
        match wire.read_message()? {
            backend::Message::AuthenticationSasl(body) => {
                let mut offered = Vec::new();
                let mut mechanisms = body.mechanisms();
                while let Some(name) = mechanisms.next()? {
                    offered.push(name.to_owned());
                }
                if !offered.iter().any(|name| name == SCRAM_SHA_256) {
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionRefused,
                        format!(
                            "the server offers {}, and this driver authenticates with \
                             {SCRAM_SHA_256} only",
                            offered.join(", ")
                        ),
                    ));
                }

                let client = ScramSha256::new(
                    target.password.as_bytes(),
                    // The module doc owns why this is `unsupported` and not
                    // `unrequested`: the second is read as a downgrade by a
                    // server that does offer binding, which over TLS is all of
                    // them.
                    ChannelBinding::unsupported(),
                );
                frontend::sasl_initial_response(SCRAM_SHA_256, client.message(), &mut out)?;
                wire.send(&mut out)?;
                scram = Some(client);
            }
            backend::Message::AuthenticationSaslContinue(body) => {
                let client = scram.as_mut().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "the server continued a SASL exchange it never started",
                    )
                })?;
                client.update(body.data())?;
                frontend::sasl_response(client.message(), &mut out)?;
                wire.send(&mut out)?;
            }
            backend::Message::AuthenticationSaslFinal(body) => {
                let client = scram.as_mut().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "the server finished a SASL exchange it never started",
                    )
                })?;
                // This is the half that authenticates the *server*: it proves
                // it holds the stored key, and a failure here is an error
                // rather than a connection.
                client.finish(body.data())?;
            }
            // A `trust` line in `pg_hba.conf`. The module doc owns why this is
            // accepted: nothing of ours has left, and the peer is already
            // authenticated by its certificate.
            backend::Message::AuthenticationOk => {}
            backend::Message::AuthenticationCleartextPassword
            | backend::Message::AuthenticationMd5Password(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    "the server asked for a cleartext or MD5 password, and this driver sends \
                     neither: set `password_encryption = scram-sha-256` and reset the role's \
                     password",
                ));
            }
            backend::Message::BackendKeyData(body) => {
                cancel = Some(CancelKey {
                    process_id: body.process_id(),
                    secret_key: body.secret_key(),
                });
            }
            // The server's own settings, echoed, and one of them is kept:
            // `server_version` is what `Core\Db\Connection::serverVersion`
            // answers, and startup is the only place this server says it, so a
            // member that reads like a field read costs no round trip
            // ([ADR 0187 § 2](/docs/decisions/0187.md)). The two the type map
            // depends on, `DateStyle` and `TimeZone`, are startup parameters
            // this driver set, so reading those back would only be asking
            // whether the server agreed with itself.
            backend::Message::ParameterStatus(body) => {
                if body.name()? == "server_version" {
                    version = Some(body.value()?.to_owned());
                }
            }
            backend::Message::NoticeResponse(_) => {}
            backend::Message::ErrorResponse(body) => return Err(server_error(&body)),
            backend::Message::ReadyForQuery(_) => {
                let cancel = cancel.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "the server completed startup without a BackendKeyData, so this connection \
                         has no cancellation key and its statements could not be bounded",
                    )
                })?;
                // Every PostgreSQL reports `server_version` at startup, and so
                // does every pooler that forwards the startup parameters. A
                // startup without one is not the server this driver takes it
                // for, and the alternative — answering the empty string — is a
                // silent wrong answer to `rule:core-classes/schema-plan`'s
                // grader as much as to the program that asked.
                let server_version = version.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "the server completed startup without reporting `server_version`, which \
                         every PostgreSQL sends and `Core\\Db\\Connection::serverVersion` answers \
                         out of",
                    )
                })?;
                return Ok(Startup {
                    cancel,
                    server_version,
                });
            }
            // `backend::Message` is `#[non_exhaustive]` and carries no
            // rendering of itself, so this says what happened rather than what
            // arrived: everything a startup exchange may legally carry is
            // named above, and a row description here is a server that is not
            // speaking the protocol we are.
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "the server sent a message a startup exchange may not carry",
                ));
            }
        }
    }
}

/// What a startup exchange leaves on the connection, both halves of which
/// arrive exactly once.
///
/// The [`CancelKey`] is unrepeatable because the protocol offers no way to ask
/// again, and the version is unrepeatable by choice: asking would be a round
/// trip under a member that reads like a field read
/// ([ADR 0187 § 2](/docs/decisions/0187.md)).
#[derive(Debug)]
struct Startup {
    /// What a second connection cancels this one's statement with.
    cancel: CancelKey,
    /// The `server_version` parameter, in the server's own words.
    server_version: String,
}

/// An `ErrorResponse` in the words the server used, carrying [ADR 0067
/// § 8](/docs/decisions/0067.md)'s normalised kind.
///
/// Severity, `SQLSTATE`, message and the constraint the condition names, which
/// are the fields an operator or a `catch` acts on; the rest (position, hint,
/// the source line in the server's own C) are dropped rather than rendered into
/// a sentence nobody reads. A field that does not decode ends the walk, so a
/// malformed error is still an error — and the only one of these that carries
/// no [`ServerError`], because there is no code to classify.
///
/// The sentence is [`ServerError`]'s `Display` and is unchanged by the kind
/// riding beside it: a caller that prints this reads the sentence alone, and
/// one that branches — § 7's retry rule is the first — asks
/// [`ServerError::of`].
fn server_error(body: &backend::ErrorResponseBody) -> io::Error {
    let mut severity = String::from("ERROR");
    let mut code = String::from("XX000");
    let mut message = String::new();
    let mut constraint = None;

    let mut fields = body.fields();
    loop {
        match fields.next() {
            Ok(Some(field)) => {
                let value = String::from_utf8_lossy(field.value_bytes()).into_owned();
                match field.type_() {
                    // 'V' is the non-localized severity and wins over 'S',
                    // which the server translates.
                    b'V' => severity = value,
                    b'S' if severity == "ERROR" => severity = value,
                    b'C' => code = value,
                    b'M' => message = value,
                    b'n' => constraint = Some(value),
                    _ => {}
                }
            }
            Ok(None) => break,
            Err(e) => return e,
        }
    }

    io::Error::other(ServerError {
        kind: kind_of(&code),
        sql_state: code,
        severity,
        message,
        constraint,
        driver_code: None,
        backend: "postgres",
    })
}

/// The § 8 kind a PostgreSQL `SQLSTATE` means.
///
/// The five characters are the whole input: PostgreSQL has no vendor integer
/// beside them, and its *classes* — the first two characters — are specified
/// rather than incidental, so a class that means one kind is matched as a class
/// and only the codes that disagree with their own class are named one by one.
/// Everything unnamed is [`DbErrorKind::Other`] rather than a guess: § 8
/// normalises the conditions applications branch on, and a code outside that
/// set is one they read the `SQLSTATE` for.
pub(crate) fn kind_of(code: &str) -> DbErrorKind {
    match code {
        "23505" => DbErrorKind::UniqueViolation,
        "23503" => DbErrorKind::ForeignKeyViolation,
        "23502" => DbErrorKind::NotNullViolation,
        "23514" => DbErrorKind::CheckViolation,
        "40P01" => DbErrorKind::Deadlock,
        "40001" => DbErrorKind::SerializationFailure,
        // `query_canceled` is what `statement_timeout` raises, and `25P03` is
        // `idle_in_transaction_session_timeout`. A cancellation asked for by
        // the other end of § 3's cancellation key arrives as `57014` too, and
        // is the same thing from the statement's point of view.
        "57014" | "25P03" => DbErrorKind::Timeout,
        // The rest of class 57 is the server going away or not yet accepting
        // work: `admin_shutdown`, `crash_shutdown`, `cannot_connect_now`.
        "57P01" | "57P02" | "57P03" => DbErrorKind::ConnectionLost,
        // The one access rule inside the syntax class, which is where
        // PostgreSQL puts `insufficient_privilege`.
        "42501" => DbErrorKind::Permission,
        _ => match code.get(..2) {
            // Class 08 — connection exception, every member of it.
            Some("08") => DbErrorKind::ConnectionLost,
            // Class 28 — invalid authorization specification, which is a bad
            // password as much as a refused role.
            Some("28") => DbErrorKind::Permission,
            // Class 42 — syntax error *or* access rule violation, and § 8's
            // `Syntax` is the half an application branches on: an undefined
            // table and a malformed statement are the same bug to a caller.
            Some("42") => DbErrorKind::Syntax,
            _ => DbErrorKind::Other,
        },
    }
}

/// The unnamed statement and the unnamed portal, which is what an uncached
/// execution uses.
///
/// PostgreSQL destroys the unnamed portal at the next `Sync`, so nothing
/// accumulates on the server between calls and there is nothing to deallocate.
/// The *statement* is named by [`StatementCache`] whenever `rule:core-classes/db-one-api`'s cache
/// is on, and this remains the spelling of the unnamed one — which is what a
/// capacity of zero, and only that, still sends.
const UNNAMED: &str = "";

/// One column of a portal's row description.
///
/// The fields are the server's own — an OID and a type modifier, never a Novis
/// type — because a row description is what the server said rather than what
/// this driver made of it. [`PgColumn::decode`] is where [ADR 0067
/// § 9](/docs/decisions/0067.md)'s type map turns the pair into a
/// value, and it is the only place in this driver that knows a `pg_type` OID
/// means anything at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgColumn {
    /// The column's label, as the server wrote it in the row description.
    pub name: String,
    /// The PostgreSQL type OID of the column's values.
    pub type_oid: Oid,
    /// The type's modifier, or `-1` where the type takes none.
    ///
    /// Only § 9's `BIT(1)` row reads it: `bit` and `bit varying` share one OID
    /// and carry their width here, so this is the only signal separating the
    /// row that is a `bool` from the row that is a `tainted string`.
    pub type_modifier: i32,
}

/// One row, still in the bytes the wire framed it out of.
///
/// No column has been parsed and none is copied: [`PgRow::column`] hands back a
/// slice of the message body, which the row owns. `rule:core-classes/db-column-types`'s decoders are
/// what read them, one column at a time, in whatever order the caller's target
/// type asks for.
pub struct PgRow {
    body: backend::DataRowBody,
}

impl std::fmt::Debug for PgRow {
    /// How many columns, and none of their values: a row in flight is one
    /// request's data — the same rule [`Wire`]'s own rendering follows.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgRow")
            .field("bytes", &self.body.buffer().len())
            .finish_non_exhaustive()
    }
}

impl PgRow {
    /// The bytes of column `index`, or `None` where the column is SQL `NULL`.
    ///
    /// # Errors
    ///
    /// `InvalidInput` when the row has no such column, and `InvalidData` when
    /// its length prefixes do not add up.
    pub fn column(&self, index: usize) -> io::Result<Option<&[u8]>> {
        let mut ranges = self.body.ranges();
        let mut at = 0;
        while let Some(range) = ranges.next()? {
            if at == index {
                return Ok(range.map(|range| &self.body.buffer()[range]));
            }
            at += 1;
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("column {index} was asked for in a row that has {at}"),
        ))
    }
}

/// The `pg_type` OIDs [ADR 0067 § 9](/docs/decisions/0067.md)'s table
/// and [`ColumnType`](crate::ColumnType) name, spelled as PostgreSQL numbers
/// them.
///
/// A built-in type's OID is bootstrap data — fixed in `pg_type.dat` and the
/// same on every server of every version — which is what makes a literal table
/// legitimate here rather than a `SELECT` against the catalog at connect time.
/// `postgres-protocol` is framing and vends none of them.
///
/// **Two questions are asked of this table, and the second is why the text
/// types are in it.** [`super::PgColumn::scalar`] asks what a body decodes to,
/// and for that a type is either named here or reaches § 9's last row as text.
/// [`super::PgColumn::column_type`] asks what the column *is*, and there
/// `varchar` is `ColumnType::Text` while `inet` is `ColumnType::Other` — so the
/// text family and the two JSON types are named as well, though no decoder
/// branches on them. What stays absent is what both questions answer the same
/// way: `xml`, `inet`, ranges, `hstore`, geometry and `interval` are `tainted
/// string` and `Other` by simply not being in this list.
///
/// [`element`] names an array type per element type for a third reason, which
/// its own doc gives.
mod oid {
    use postgres_protocol::Oid;

    /// `BOOLEAN`.
    pub(super) const BOOL: Oid = 16;
    /// `BYTEA`.
    pub(super) const BYTEA: Oid = 17;
    /// `"char"`, the one-byte internal text type the catalog uses.
    pub(super) const CHAR: Oid = 18;
    /// `name`, the internal text type an identifier column has.
    pub(super) const NAME: Oid = 19;
    /// `BIGINT`.
    pub(super) const INT8: Oid = 20;
    /// `SMALLINT`.
    pub(super) const INT2: Oid = 21;
    /// `INTEGER`.
    pub(super) const INT4: Oid = 23;
    /// `TEXT`.
    pub(super) const TEXT: Oid = 25;
    /// `oid`, the only unsigned integer a query is likely to select.
    pub(super) const OID: Oid = 26;
    /// `json`, which decodes as text and describes as `ColumnType::Json` — the
    /// module doc's second question, and the whole reason it is named.
    pub(super) const JSON: Oid = 114;
    /// `REAL`.
    pub(super) const FLOAT4: Oid = 700;
    /// `DOUBLE PRECISION`.
    pub(super) const FLOAT8: Oid = 701;
    /// `money`.
    pub(super) const MONEY: Oid = 790;
    /// `CHAR(n)`, which PostgreSQL types as `bpchar` — blank-padded.
    pub(super) const BPCHAR: Oid = 1042;
    /// `VARCHAR(n)`.
    pub(super) const VARCHAR: Oid = 1043;
    /// `DATE`.
    pub(super) const DATE: Oid = 1082;
    /// `TIME`, which is `time without time zone`. `timetz` is a different OID
    /// with no row of its own in § 9, so it stays text.
    pub(super) const TIME: Oid = 1083;
    /// `TIMESTAMP`, § 9's zone-less row.
    pub(super) const TIMESTAMP: Oid = 1114;
    /// `TIMESTAMPTZ`, the row that carries its own offset.
    pub(super) const TIMESTAMPTZ: Oid = 1184;
    /// `BIT(n)`, whose `n` is the type modifier.
    pub(super) const BIT: Oid = 1560;
    /// `BIT VARYING(n)`, sharing `BIT`'s rendering and its modifier.
    pub(super) const VARBIT: Oid = 1562;
    /// `NUMERIC`/`DECIMAL`.
    pub(super) const NUMERIC: Oid = 1700;
    /// `UUID`.
    pub(super) const UUID: Oid = 2950;
    /// `jsonb`, named for the reason [`JSON`] is.
    pub(super) const JSONB: Oid = 3802;

    /// The element type of the array type `oid` names, or `None` for an OID
    /// that is not an array this driver knows.
    ///
    /// An array's element OID is `pg_type.typelem` — catalog data, and a row
    /// description carries only the array's own OID — so the choice is this
    /// table or a `SELECT` against the catalog at connect time, and the
    /// bootstrap argument the module doc makes for the list above decides it
    /// the same way.
    ///
    /// Every type § 9 has a row for is here as its array, and so are the
    /// text-ish ones whose elements reach the table's last row: `text[]` is
    /// `array<string>`, which is the array a query is most likely to select.
    /// An array this does not name — `point[]`, and every array of a type an
    /// extension added — stays whole at that last row as the server rendered
    /// it, because a body split into elements this driver has no type for
    /// would be a worse answer than the rendering itself.
    pub(super) fn element(oid: Oid) -> Option<Oid> {
        Some(match oid {
            // `xml[]`. An element OID written as a literal is one this module
            // has no constant for: nothing asks about `xml` on its own, which
            // is text as a value and `Other` as a description alike.
            143 => 142,
            // `json[]`.
            199 => JSON,
            // `cidr[]`.
            651 => 650,
            791 => MONEY,
            1000 => BOOL,
            1001 => BYTEA,
            // `"char"[]` and `name[]`, the two internal text types a query
            // against the catalog selects without meaning to.
            1002 => CHAR,
            1003 => NAME,
            1005 => INT2,
            1007 => INT4,
            // `text[]`.
            1009 => TEXT,
            // `bpchar[]` and `varchar[]`.
            1014 => BPCHAR,
            1015 => VARCHAR,
            1016 => INT8,
            1021 => FLOAT4,
            1022 => FLOAT8,
            1028 => OID,
            // `inet[]`.
            1041 => 869,
            1115 => TIMESTAMP,
            1182 => DATE,
            1183 => TIME,
            1185 => TIMESTAMPTZ,
            // `interval[]`, which § 9 keeps at text deliberately: an interval
            // carries months and is not a `Duration`.
            1187 => 1186,
            1231 => NUMERIC,
            // `timetz[]`, text for the reason `timetz` itself is.
            1270 => 1266,
            1561 => BIT,
            1563 => VARBIT,
            2951 => UUID,
            // `jsonb[]`.
            3807 => JSONB,
            _ => return None,
        })
    }
}

/// A calendar date, in the fields the server rendered and no further.
///
/// This driver hands back components rather than a `Core\Time\Date` because it
/// cannot build one: an instance needs `nvs-stdlib`'s class descriptors, and
/// `Cargo.toml` states on both manifests why an edge to that crate does not
/// exist. [`PgScalar`] is this crate's public answer beside a [`Value`], and
/// § 9's structured rows are finished one layer up.
///
/// The fields are checked for *shape* — a month is 1 to 12, a day 1 to 31 —
/// and never against the calendar: whether the 31st exists in this month is
/// settled by the type that has a calendar in it, at the point the instance is
/// built.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PgDate {
    /// The astronomical year, so `1 BC` is `0` and `44 BC` is `-43`. The
    /// server writes an era suffix instead; this is the same number, counted
    /// the way every `Core\Time` type counts it.
    pub year: i32,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day of the month, 1 to 31.
    pub day: u8,
}

impl std::fmt::Debug for PgDate {
    /// The type and none of the fields, for the reason [`PgRow`]'s own
    /// rendering gives: a decoded column is one request's data.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgDate").finish_non_exhaustive()
    }
}

/// A time of day, to the nanosecond, in the fields the server rendered.
///
/// The shape rule is [`PgDate`]'s, and so is the reason this is components.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PgTime {
    /// The hour, 0 to 24: PostgreSQL's `TIME` includes `24:00:00`, which is
    /// the one value of the column type `Core\Time\TimeOfDay` has no
    /// representation for and refuses one layer up.
    pub hour: u8,
    /// The minute, 0 to 59.
    pub minute: u8,
    /// The second, 0 to 59.
    pub second: u8,
    /// The nanosecond within the second. PostgreSQL stores microseconds, so
    /// the last three digits are always zero; the field counts nanoseconds
    /// because that is what every `Core\Time` type holds.
    pub nanosecond: u32,
}

impl std::fmt::Debug for PgTime {
    /// As [`PgDate`]'s: the type, and none of the data.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgTime").finish_non_exhaustive()
    }
}

/// One column's value, decoded but not yet allocated as a [`Value`].
///
/// The decode and the allocation are two steps for one reason: everything that
/// can go wrong with a column happens in the first, and none of it needs an
/// allocator, so § 9's whole table is assertable in a `-p nvs-db` test that
/// leaks nothing. This crate forbids `unsafe` and [`Value::release`] is unsafe,
/// so a `Value` a test built here could never be freed by one — a reference is
/// minted in [`PgScalar::into_value`] and nowhere else, and that function is
/// one arm per variant with nothing left to get wrong.
///
/// It is also this driver's public answer beside a [`Value`], because some of
/// § 9's rows are not values at all: a `DATE` is a `Core\Time\Date`, an
/// *instance* of an `nvs-stdlib` class, and that crate is the only one that
/// can allocate one. Those rows arrive here as parsed components — see
/// [`PgDate`] — and `into_value` answers `None` for them, as it does for an
/// array holding one of them at any depth.
pub enum PgScalar<'a> {
    /// SQL `NULL`: the row of § 9's table that makes every column `?T`.
    Null,
    /// `BOOLEAN`, and `BIT(1)`.
    Bool(bool),
    /// `SMALLINT`/`INTEGER`/`BIGINT`.
    Int(i64),
    /// `oid`.
    UInt(u64),
    /// `REAL`/`DOUBLE PRECISION`.
    Float(f64),
    /// `NUMERIC` and `money`.
    Decimal(Decimal),
    /// A `tainted string`'s text, already proven well-formed UTF-8.
    ///
    /// Borrowed out of the row body, except for an element of an array whose
    /// quoting carried a backslash escape: unescaping cannot happen in place,
    /// so that one element owns its text instead — [`Self::into_owned`] is
    /// where the second case is made, and it is the only place it is made.
    Text(Cow<'a, str>),
    /// A `tainted bytes`'s octets, which had to be decoded out of a text
    /// rendering and so are the driver's own rather than a borrow of the row.
    Bytes(NvsStr),
    /// `DATE`, which is a `Core\Time\Date`.
    Date(PgDate),
    /// `TIME`, which is a `Core\Time\TimeOfDay`.
    Time(PgTime),
    /// `TIMESTAMP`: § 9's zone-less row, a `Core\Time\DateTime` in the zone
    /// [`PgTarget::time_zone`] declared and the server was told.
    Timestamp {
        /// The civil date, as rendered.
        date: PgDate,
        /// The civil time, as rendered.
        time: PgTime,
    },
    /// `TIMESTAMPTZ`, which is a `Core\Time\Instant`: the civil fields the
    /// server rendered, plus what turns them into a point in time.
    Instant {
        /// The civil date, at `offset`.
        date: PgDate,
        /// The civil time, at `offset`.
        time: PgTime,
        /// Seconds east of UTC — the sign every `Core\Time` type uses, and
        /// the session's own `TimeZone` because this driver set it.
        offset: i32,
    },
    /// `UUID`, as its sixteen octets in the order the text spells them.
    Uuid([u8; 16]),
    /// A PostgreSQL array, as its elements — each one a row of § 9's table in
    /// its own right, so a multi-dimensional array is elements that are
    /// themselves arrays and needs nothing else.
    ///
    /// The elements are decoded and the array is *not* built: a [`Value`] is
    /// minted in [`Self::into_value`] and nowhere else, which is also what
    /// leaves this variant usable by the caller `into_value` cannot serve —
    /// an `array<Core\Time\Date>` arrives here as components, exactly as the
    /// bare `DATE` beside it does.
    Array(Vec<PgScalar<'a>>),
}

impl std::fmt::Debug for PgScalar<'_> {
    /// Which row of § 9's table this landed on, and never the value: a column
    /// in flight is one request's data, which is the rule [`PgRow`]'s own
    /// rendering holds.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PgScalar::Null => "null",
            PgScalar::Bool(_) => "bool",
            PgScalar::Int(_) => "int",
            PgScalar::UInt(_) => "uint",
            PgScalar::Float(_) => "float",
            PgScalar::Decimal(_) => "decimal",
            PgScalar::Text(_) => "string",
            PgScalar::Bytes(_) => "bytes",
            PgScalar::Date(_) => "date",
            PgScalar::Time(_) => "time",
            PgScalar::Timestamp { .. } => "timestamp",
            PgScalar::Instant { .. } => "instant",
            PgScalar::Uuid(_) => "uuid",
            PgScalar::Array(_) => "array",
        })
    }
}

impl PgScalar<'_> {
    /// The Novis value, taking on the one reference a `string` or a `bytes`
    /// costs and nothing at all for the rest.
    ///
    /// `None` for § 9's structured rows, whose Novis type is a class instance
    /// this crate cannot allocate at all — [`PgDate`] owns why — and for an
    /// array holding one of them at any depth, which is as much
    /// `nvs-stdlib`'s to finish as a bare one is. A caller that wants the
    /// whole table matches those variants first, and an [`Self::Array`]
    /// whose elements it has finished itself, and reaches this for everything
    /// left.
    pub fn into_value(self) -> Option<Value> {
        Some(match self {
            PgScalar::Null => Value::null(),
            PgScalar::Bool(value) => Value::bool(value),
            PgScalar::Int(value) => Value::int(value),
            PgScalar::UInt(value) => Value::uint(value),
            PgScalar::Float(value) => Value::float(value),
            PgScalar::Decimal(value) => Value::decimal(value),
            PgScalar::Text(text) => Value::str(NvsStr::new(text.as_bytes())),
            PgScalar::Bytes(bytes) => Value::bytes(bytes),
            PgScalar::Date(_)
            | PgScalar::Time(_)
            | PgScalar::Timestamp { .. }
            | PgScalar::Instant { .. }
            | PgScalar::Uuid(_) => return None,
            PgScalar::Array(items) => {
                // Asked before anything is allocated, and that is the whole
                // reason it is a pass of its own: a `?` on the fourth element
                // would abandon three built values and the array holding
                // them, and this crate cannot free one — `Value::release` is
                // `unsafe` and forbidden here.
                if !items.iter().all(PgScalar::is_value) {
                    return None;
                }

                let mut array = NvsArray::new();
                for item in items {
                    array.append(item.into_value()?);
                }
                Value::array(array)
            }
        })
    }

    /// Whether [`Self::into_value`] has a value for this row, asked without
    /// allocating anything.
    ///
    /// The recursion is an array's, and it is the point: an
    /// `array<array<Core\Uuid>>` has no `Value` either, and finding that out
    /// after two levels of it were built is what this exists to prevent.
    fn is_value(&self) -> bool {
        match self {
            PgScalar::Date(_)
            | PgScalar::Time(_)
            | PgScalar::Timestamp { .. }
            | PgScalar::Instant { .. }
            | PgScalar::Uuid(_) => false,
            PgScalar::Array(items) => items.iter().all(PgScalar::is_value),
            _ => true,
        }
    }

    /// The same row with nothing borrowed from the body it was decoded out
    /// of, so that it can outlive one.
    ///
    /// The single caller is an array element whose quoting had to be
    /// unescaped into a buffer of the decoder's own — see [`PgColumn::array`]
    /// — and [`Self::Text`] is the only variant that borrows at all. The match
    /// is exhaustive rather than a wildcard so that a variant added later
    /// cannot quietly keep a borrow this promises it does not have.
    fn into_owned(self) -> PgScalar<'static> {
        match self {
            PgScalar::Null => PgScalar::Null,
            PgScalar::Bool(value) => PgScalar::Bool(value),
            PgScalar::Int(value) => PgScalar::Int(value),
            PgScalar::UInt(value) => PgScalar::UInt(value),
            PgScalar::Float(value) => PgScalar::Float(value),
            PgScalar::Decimal(value) => PgScalar::Decimal(value),
            PgScalar::Text(text) => PgScalar::Text(Cow::Owned(text.into_owned())),
            PgScalar::Bytes(bytes) => PgScalar::Bytes(bytes),
            PgScalar::Date(date) => PgScalar::Date(date),
            PgScalar::Time(time) => PgScalar::Time(time),
            PgScalar::Timestamp { date, time } => PgScalar::Timestamp { date, time },
            PgScalar::Instant { date, time, offset } => PgScalar::Instant { date, time, offset },
            PgScalar::Uuid(octets) => PgScalar::Uuid(octets),
            PgScalar::Array(items) => {
                PgScalar::Array(items.into_iter().map(PgScalar::into_owned).collect())
            }
        }
    }
}

/// One bound value in the text format [`PgConn::query`] sends, with `None` for
/// SQL `NULL` — [`PgColumn::decode`]'s direction, reversed.
///
/// **The rendering is here and the refusal is not.** Which Novis values may be
/// bound at all is `rule:core-classes/db-parameters`'s
/// question and `nvs-stdlib`'s to word, because that is where the call's own
/// spelling is known; what a driver owns is the octets each accepted one
/// becomes, and those differ per backend even where the Novis type does not.
/// So an unbindable value is an `InvalidInput` naming its tag, which the
/// `Core\Db` boundary turns into a `LogicError` naming the parameter — the
/// same route [`crate::sql::rewrite`]'s refusals take.
///
/// A `Core\Db\InList` never reaches here: § 5's marker has already expanded
/// into one bound value per element by the time a statement has its bind list.
///
/// # What the server makes of it
///
/// Nothing is sent with a parameter type OID, so the server infers each
/// parameter's type from where it appears in the statement and reads these
/// octets as that type's own text input. That is why a `bytes` renders as
/// `bytea`'s hex form and not as raw octets: a `Bind` carrying a length is
/// still text on this path, and the only reading of it the server has is the
/// column's.
///
/// # Errors
///
/// `InvalidInput` for a value with no text form to send — an array, an object,
/// a callable — where the whole answer is the tag, and never the value, for the
/// reason [`PgColumn::decode`]'s own refusals give: a bound parameter is the
/// one thing in a statement most likely to be a credential.
pub fn encode(value: Value) -> io::Result<Option<Vec<u8>>> {
    let rendered = match value.tag() {
        Some(Tag::Null) => return Ok(None),
        // PostgreSQL's `boolean` input accepts a dozen spellings and outputs
        // exactly these two, so this is also what a round trip through a
        // column answers with.
        Some(Tag::Bool) => String::from(if value.as_bool() == Some(true) {
            "t"
        } else {
            "f"
        }),
        Some(Tag::Int) => value.as_int().unwrap_or_default().to_string(),
        Some(Tag::Uint) => value.as_uint().unwrap_or_default().to_string(),
        Some(Tag::Float) => {
            let float = value.as_float().unwrap_or_default();
            // Rust spells the three non-finite values `inf`, `-inf` and `NaN`,
            // and PostgreSQL's `float8` input reads none of them. Every finite
            // one goes out in Rust's shortest round-tripping form, which
            // `float8` reads back to the same bits.
            if float.is_nan() {
                String::from("NaN")
            } else if float.is_infinite() {
                String::from(if float > 0.0 { "Infinity" } else { "-Infinity" })
            } else {
                float.to_string()
            }
        }
        // Exact on both sides: `rule:types/decimal`'s `decimal` renders as digits and a
        // point, which is `numeric`'s own input form, so nothing rounds here
        // the way binding it as a `float8` would.
        Some(Tag::Decimal) => value
            .as_decimal()
            .map(|exact| exact.to_string())
            .unwrap_or_default(),
        // A `string` is UTF-8 by `rule:types/bytes` and the session is UTF-8 by the
        // connect path, so the octets go out as they are.
        Some(Tag::Str) => {
            return Ok(Some(value.as_str_bytes().unwrap_or_default().to_vec()));
        }
        Some(Tag::Bytes) => {
            let octets = value.as_bytes().unwrap_or_default();
            let mut hex = String::with_capacity(octets.len() * 2 + 2);
            hex.push_str("\\x");
            for byte in octets {
                use std::fmt::Write as _;
                let _ = write!(hex, "{byte:02x}");
            }
            hex
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "a value of tag {} has no form this driver can bind",
                    value.tag_byte()
                ),
            ));
        }
    };
    Ok(Some(rendered.into_bytes()))
}

impl PgColumn {
    /// The column's declared type, as spec § 18's `ColumnType` names it — what
    /// `Core\Db\Rows::columns` answers for this column.
    ///
    /// **This describes the column rather than summarising [`Self::scalar`]**,
    /// and [`ColumnType`]'s own doc owns why the two differ. What follows here
    /// is that it reads no body at all: a column whose every row is NULL still
    /// has a type, which is most of why `columns()` exists. The one thing it
    /// reads besides the OID is [ADR 0067
    /// § 9](/docs/decisions/0067.md)'s `BIT(1)` row, for the reason
    /// [`Self::scalar`] reads the modifier there.
    ///
    /// A PostgreSQL `ENUM` is the one member of § 9's text family this answers
    /// [`ColumnType::Other`] for rather than [`ColumnType::Text`]: an enum
    /// type's OID is created with the type and is not bootstrap data, so the
    /// only way to recognise one is a catalog lookup per connection — which is
    /// exactly what the [`oid`] table exists to avoid, and `Other` is a true
    /// answer rather than a wrong one.
    #[must_use]
    pub fn column_type(&self) -> ColumnType {
        match self.type_oid {
            oid::INT2 | oid::INT4 | oid::INT8 => ColumnType::Int,
            oid::OID => ColumnType::Uint,
            oid::FLOAT4 | oid::FLOAT8 => ColumnType::Float,
            oid::NUMERIC | oid::MONEY => ColumnType::Decimal,
            oid::CHAR | oid::NAME | oid::TEXT | oid::BPCHAR | oid::VARCHAR => ColumnType::Text,
            oid::BYTEA => ColumnType::Bytes,
            oid::BOOL => ColumnType::Bool,
            // § 9's `BIT(1)` row, and the width is the only thing that says so.
            oid::BIT | oid::VARBIT if self.type_modifier == 1 => ColumnType::Bool,
            oid::DATE => ColumnType::Date,
            oid::TIME => ColumnType::Time,
            oid::TIMESTAMP => ColumnType::DateTime,
            oid::TIMESTAMPTZ => ColumnType::Instant,
            oid::UUID => ColumnType::Uuid,
            oid::JSON | oid::JSONB => ColumnType::Json,
            // Every array, every `BIT(n>1)`, and every type with no Novis type
            // of its own: § 9's last row, which `scalar` reads as text.
            _ => ColumnType::Other,
        }
    }

    /// This column's `body` as the Novis value [ADR 0067
    /// § 9](/docs/decisions/0067.md)'s table names, with `None` — SQL
    /// `NULL` — as `null`, which is why every column reads back as `?T`.
    ///
    /// `body` is what [`PgRow::column`] handed back, still in the text format
    /// the module doc chose.
    ///
    /// `Ok(None)` is the *other* absence, and the two never collide: it says
    /// the column is one of § 9's structured rows — `DATE`, `TIME`,
    /// `TIMESTAMP`, `TIMESTAMPTZ`, `UUID` — whose Novis type is a class
    /// instance no driver can allocate. [`Self::scalar`] is what reads those,
    /// and this is the scalar half in full — plus § 9's array row, which is a
    /// `Value` like any other once its elements are. An array whose elements
    /// are one of those rows is the same `Ok(None)` as a single one, whole and
    /// at any nesting.
    ///
    /// The `tainted` half of `tainted string` is nowhere in this signature and
    /// is not missing.
    /// `rule:security/tainted-qualifier`'s
    /// qualifier is a property of the *type* a row is read at, declared in
    /// `nvs-stdlib`'s registry rows; there is no runtime bit for it, and a
    /// driver could not set one if there were.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a body the column's own type cannot be read out of: a
    /// `NUMERIC` whose value is past what
    /// `rule:types/decimal`'s `decimal`
    /// holds, which [`Self::decimal`] refuses as the schema question it is
    /// rather than narrowing, or a `NaN` in one, which that type has no
    /// representation for; a text body that is not UTF-8; a malformed `bytea`.
    /// Every such message names the column and its OID and **never the body**,
    /// for the reason [`Self::malformed`] gives.
    pub fn decode(&self, body: Option<&[u8]>) -> io::Result<Option<Value>> {
        Ok(self.scalar(body)?.into_value())
    }

    /// [`PgColumn::decode`]'s whole decision, before anything is allocated,
    /// and every row of § 9's table rather than the scalar half.
    ///
    /// `nvs-stdlib` calls this one: it is the only crate that can turn a
    /// [`PgScalar::Date`] and its siblings into the `Core\Time` and
    /// `Core\Uuid` instances the table names.
    ///
    /// # Errors
    ///
    /// As [`Self::decode`], plus a structured row in a shape this driver does
    /// not read: a rendering no `DateStyle = ISO` session produces, or an
    /// `infinity`, which no `Core\Time` type has a value for.
    pub fn scalar<'a>(&self, body: Option<&'a [u8]>) -> io::Result<PgScalar<'a>> {
        let Some(body) = body else {
            return Ok(PgScalar::Null);
        };

        Ok(match self.type_oid {
            oid::BOOL => PgScalar::Bool(match body {
                b"t" => true,
                b"f" => false,
                _ => return Err(self.malformed("a bool")),
            }),
            // § 9's `BIT(1)` row, and the width is the only thing that says so.
            // A one-bit string renders as `0`/`1` rather than `f`/`t`: on the
            // wire a `bit` is not a `boolean`, only in the table.
            oid::BIT | oid::VARBIT if self.type_modifier == 1 => PgScalar::Bool(match body {
                b"1" => true,
                b"0" => false,
                _ => return Err(self.malformed("a one-bit string")),
            }),
            oid::INT2 | oid::INT4 | oid::INT8 => PgScalar::Int(
                self.text(body)?
                    .parse()
                    .map_err(|_| self.malformed("an int"))?,
            ),
            oid::OID => PgScalar::UInt(
                self.text(body)?
                    .parse()
                    .map_err(|_| self.malformed("a uint"))?,
            ),
            // `NaN` and `±Infinity` are what the server writes and what Rust's
            // parser reads, so the three values a `float` has beyond the finite
            // ones need no arm of their own.
            oid::FLOAT4 | oid::FLOAT8 => PgScalar::Float(
                self.text(body)?
                    .parse()
                    .map_err(|_| self.malformed("a float"))?,
            ),
            oid::NUMERIC => PgScalar::Decimal(self.decimal(self.text(body)?)?),
            oid::MONEY => PgScalar::Decimal(self.money(self.text(body)?)?),
            oid::BYTEA => PgScalar::Bytes(NvsStr::new(&self.bytea(body)?)),
            oid::DATE => PgScalar::Date(self.date(self.text(body)?)?),
            oid::TIME => PgScalar::Time(self.time_of_day(self.text(body)?)?),
            oid::TIMESTAMP => {
                let (date, time, _) = self.timestamp(self.text(body)?, false)?;
                PgScalar::Timestamp { date, time }
            }
            oid::TIMESTAMPTZ => {
                let (date, time, offset) = self.timestamp(self.text(body)?, true)?;
                PgScalar::Instant { date, time, offset }
            }
            oid::UUID => PgScalar::Uuid(self.uuid(self.text(body)?)?),
            _ => match oid::element(self.type_oid) {
                Some(element) => PgScalar::Array(self.array(element, self.text(body)?)?),
                // § 9's last row: every type this driver has no arm for, and
                // every array whose element type it cannot name.
                None => PgScalar::Text(Cow::Borrowed(self.text(body)?)),
            },
        })
    }

    /// The body as text, **checked**.
    ///
    /// The module doc's § *Parameters and results are in text format* owns why
    /// the check is here rather than trusted from `client_encoding`.
    fn text<'a>(&self, body: &'a [u8]) -> io::Result<&'a str> {
        std::str::from_utf8(body).map_err(|_| self.malformed("well-formed UTF-8"))
    }

    /// § 9's array row: the elements `array_out` wrote between braces, each
    /// decoded through this same table.
    ///
    /// `{1,2,3}`, `{}` for the empty one, and `{{1,2},{3,4}}` for a
    /// multi-dimensional one — which is nested [`PgScalar::Array`]s here,
    /// because Novis has no rectangular array type and § 9's `array<T>` with
    /// an array for `T` is what that shape means. A lower bound other than 1
    /// is written as a `[0:1]=` prefix and is dropped: a Novis array is a
    /// list, and a base has nowhere to go in one.
    ///
    /// An unquoted `NULL` is the SQL null and a quoted `"NULL"` is the four
    /// characters — that distinction is what the quoting exists for, and
    /// `array_out` quotes every element that would otherwise read back as
    /// something else: an empty one, or one holding a brace, a comma, a quote
    /// or whitespace. Inside the quotes only `"` and `\` are escaped.
    ///
    /// `element` is the OID [`oid::element`] answered, and the element column
    /// carries **this** column's modifier: PostgreSQL stores an array's typmod
    /// as its element's, so a `BIT(1)[]` reaches § 9's `bool` row the same way
    /// a `BIT(1)` does. It costs one clone of the column's name per array
    /// cell, so that a refusal still names the column the operator sees.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a body that is not that rendering — an unclosed brace
    /// or quote, an empty unquoted element, a stray brace after the outer one
    /// closed, or nesting past the six dimensions PostgreSQL itself allows —
    /// and for any element its own type refuses, which is the refusal that
    /// element would have been given as a column of its own.
    fn array<'a>(&self, element: Oid, text: &'a str) -> io::Result<Vec<PgScalar<'a>>> {
        let body = match text.split_once('=') {
            Some((bounds, rest)) if bounds.starts_with('[') => rest,
            _ => text,
        };

        let column = PgColumn {
            name: self.name.clone(),
            type_oid: element,
            type_modifier: self.type_modifier,
        };
        let mut at = 0;
        let items = column.elements(body, &mut at, 1)?;
        if at != body.len() {
            return Err(self.malformed("an array"));
        }

        Ok(items)
    }

    /// One brace-delimited run of [`Self::array`]'s elements, starting at
    /// `at`, which is left just past the closing brace. `self` is the element
    /// column, so every refusal here names the element's own type.
    ///
    /// `depth` is the nesting this call sits at and the bound is PostgreSQL's
    /// own `MAXDIM`. The body comes from a server this connection has already
    /// authenticated, but "authenticated" is not "may drive this process's
    /// stack": the recursion stops where the sender's own limit is rather than
    /// wherever the stack happens to end.
    fn elements<'a>(
        &self,
        text: &'a str,
        at: &mut usize,
        depth: usize,
    ) -> io::Result<Vec<PgScalar<'a>>> {
        /// PostgreSQL's `MAXDIM`, which is the widest array it will build.
        const MAX_DIMENSIONS: usize = 6;

        /// Whitespace between elements, which `array_out` never writes and
        /// `array_in` accepts: a reader strictly narrower than the writer it
        /// pairs with refuses a body somebody typed into a `SELECT` by hand.
        fn skip_space(bytes: &[u8], at: &mut usize) {
            while matches!(bytes.get(*at), Some(byte) if byte.is_ascii_whitespace()) {
                *at += 1;
            }
        }

        if depth > MAX_DIMENSIONS {
            return Err(self.malformed("an array"));
        }

        let bytes = text.as_bytes();
        if bytes.get(*at) != Some(&b'{') {
            return Err(self.malformed("an array"));
        }
        *at += 1;

        let mut items = Vec::new();
        if bytes.get(*at) == Some(&b'}') {
            *at += 1;
            return Ok(items);
        }

        loop {
            skip_space(bytes, at);
            let item = match bytes.get(*at) {
                Some(b'{') => PgScalar::Array(self.elements(text, at, depth + 1)?),
                Some(b'"') => match self.quoted(text, at)? {
                    Cow::Borrowed(body) => self.scalar(Some(body.as_bytes()))?,
                    // The unescaped body is this call's own buffer, so the one
                    // row that would borrow it takes it over instead.
                    Cow::Owned(body) => self.scalar(Some(body.as_bytes()))?.into_owned(),
                },
                _ => self.unquoted(text, at)?,
            };
            items.push(item);
            skip_space(bytes, at);

            match bytes.get(*at) {
                Some(b',') => *at += 1,
                Some(b'}') => {
                    *at += 1;
                    return Ok(items);
                }
                _ => return Err(self.malformed("an array")),
            }
        }
    }

    /// A quoted array element, from the opening quote at `at`, which is left
    /// just past the closing one.
    ///
    /// Borrowed wherever the quotes hold no escape at all, which is every
    /// element but those carrying a `"` or a `\` — and a `bytea`, whose whole
    /// rendering starts `\x`, is the common one of those rather than an exotic
    /// one.
    fn quoted<'a>(&self, text: &'a str, at: &mut usize) -> io::Result<Cow<'a, str>> {
        let bytes = text.as_bytes();
        let open = *at + 1;
        let mut start = open;
        let mut cursor = open;
        let mut owned: Option<String> = None;

        loop {
            match bytes.get(cursor) {
                None => return Err(self.malformed("an array")),
                Some(b'"') => {
                    *at = cursor + 1;
                    return Ok(match owned {
                        Some(mut buffer) => {
                            buffer.push_str(&text[start..cursor]);
                            Cow::Owned(buffer)
                        }
                        None => Cow::Borrowed(&text[open..cursor]),
                    });
                }
                Some(b'\\') => {
                    // Whatever follows the backslash is that character
                    // literally, taken as a `char` rather than a byte so that
                    // an escaped multi-byte one cannot be cut in half.
                    let Some(escaped) = text[cursor + 1..].chars().next() else {
                        return Err(self.malformed("an array"));
                    };
                    let buffer = owned.get_or_insert_with(String::new);
                    buffer.push_str(&text[start..cursor]);
                    buffer.push(escaped);
                    cursor += 1 + escaped.len_utf8();
                    start = cursor;
                }
                Some(_) => cursor += 1,
            }
        }
    }

    /// An unquoted array element, from `at`, which is left at the delimiter
    /// that ended it.
    ///
    /// `NULL` here is the SQL null, compared without case because `array_in`
    /// reads it that way — which is also why `array_out` quotes a text element
    /// that spells it in any case, and why a quoted one is four characters.
    fn unquoted<'a>(&self, text: &'a str, at: &mut usize) -> io::Result<PgScalar<'a>> {
        let bytes = text.as_bytes();
        let start = *at;

        while let Some(&byte) = bytes.get(*at) {
            if matches!(byte, b',' | b'}') {
                break;
            }
            // A brace, a quote or a backslash outside quoting is a body no
            // `array_out` wrote, and guessing at what it meant is how a
            // decoder starts accepting two spellings of one array.
            if matches!(byte, b'{' | b'"' | b'\\') {
                return Err(self.malformed("an array"));
            }
            *at += 1;
        }

        let body = text[start..*at].trim_end();
        if body.is_empty() {
            return Err(self.malformed("an array"));
        }
        if body.eq_ignore_ascii_case("null") {
            return Ok(PgScalar::Null);
        }

        self.scalar(Some(body.as_bytes()))
    }

    /// One numeric field of a date, a time or a zone offset: ASCII digits
    /// only, within the bounds that field has in the rendering.
    ///
    /// The bounds are the rendering's and not the calendar's — `1..=31` for a
    /// day, whatever the month is — because the calendar belongs to the type
    /// being built, one layer up. What they buy here is that every field is a
    /// plausible number before it is narrowed, so no caller casts anything.
    fn number<T: TryFrom<u32>>(
        &self,
        text: &str,
        low: u32,
        high: u32,
        wanted: &str,
    ) -> io::Result<T> {
        // `str::parse` accepts a leading `+` and Unicode digits; a field the
        // server wrote is ASCII and has neither.
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(self.malformed(wanted));
        }
        let value: u32 = text.parse().map_err(|_| self.malformed(wanted))?;
        if !(low..=high).contains(&value) {
            return Err(self.malformed(wanted));
        }

        T::try_from(value).map_err(|_| self.malformed(wanted))
    }

    /// A `DATE`, in the ISO rendering `DateStyle` is pinned to at startup.
    ///
    /// `2024-01-02`, and `0044-03-15 BC` for the half of the calendar the year
    /// field does not have: an era suffix becomes an astronomical year here,
    /// since that is the only counting `Core\Time` knows. The year is a digit
    /// run rather than four digits, because PostgreSQL's range reaches
    /// 294276 AD.
    ///
    /// # Errors
    ///
    /// `InvalidData` for anything else, `infinity` included: a date with no
    /// end has no `Core\Time\Date` to become, and a column holding one is cast
    /// or filtered rather than read.
    fn date(&self, text: &str) -> io::Result<PgDate> {
        let (text, bc) = era(text);
        let mut fields = text.split('-');
        let (Some(year), Some(month), Some(day), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(self.malformed("a date"));
        };

        let year: i32 = self.number(year, 0, 294_276, "a date")?;
        Ok(PgDate {
            // 1 BC is astronomical year 0, so an era year counts down from one.
            year: if bc { 1 - year } else { year },
            month: self.number(month, 1, 12, "a date")?,
            day: self.number(day, 1, 31, "a date")?,
        })
    }

    /// A `TIME`, or the time half of a timestamp: `HH:MM:SS` and a fraction of
    /// up to nine digits.
    ///
    /// PostgreSQL stores microseconds, writes at most six of them and writes
    /// none at all on a whole second. Nine are read because the field they
    /// land in counts nanoseconds, so a server that grew precision is a value
    /// this driver still reads rather than a truncation nothing reported.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a body with the wrong field count, a field out of the
    /// range the rendering has for it, or more than nine fractional digits.
    fn time_of_day(&self, text: &str) -> io::Result<PgTime> {
        let (clock, fraction) = match text.split_once('.') {
            Some((clock, fraction)) => (clock, fraction),
            None => (text, ""),
        };
        let mut fields = clock.split(':');
        let (Some(hour), Some(minute), Some(second), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(self.malformed("a time"));
        };

        let mut nanosecond = 0;
        if !fraction.is_empty() {
            let width = u32::try_from(fraction.len()).unwrap_or(u32::MAX);
            if width > 9 {
                return Err(self.malformed("a time"));
            }
            let digits: u32 = self.number(fraction, 0, 999_999_999, "a time")?;
            nanosecond = digits * 10u32.pow(9 - width);
        }

        Ok(PgTime {
            hour: self.number(hour, 0, 24, "a time")?,
            minute: self.number(minute, 0, 59, "a time")?,
            second: self.number(second, 0, 59, "a time")?,
            nanosecond,
        })
    }

    /// A `TIMESTAMP` or a `TIMESTAMPTZ`: a date, a space, a time, and for the
    /// zoned one the offset the session is rendering at.
    ///
    /// That offset is the whole difference between the two rows, and it is why
    /// `TimeZone` is a startup parameter rather than whatever the server was
    /// configured with: a numeric zone makes the suffix `+02`, `-05:30` or
    /// `+00`, always present and never a name. A zone-less column has no
    /// suffix and is read in [`PgTarget::time_zone`] instead, which is § 9's
    /// rule and the reason that field is sent as well as held.
    ///
    /// # Errors
    ///
    /// `InvalidData` as [`Self::date`] and [`Self::time_of_day`], and for a
    /// zoned body carrying no offset at all.
    fn timestamp(&self, text: &str, zoned: bool) -> io::Result<(PgDate, PgTime, i32)> {
        // The era suffix trails the offset, so it comes off the whole body
        // before anything is split; the date half is then an AD one, and the
        // year is turned around here rather than in two places.
        let (body, bc) = era(text);
        let Some((day, clock)) = body.split_once(' ') else {
            return Err(self.malformed("a timestamp"));
        };

        let mut date = self.date(day)?;
        if bc {
            date.year = 1 - date.year;
        }

        let (clock, offset) = if zoned {
            // A time has no sign in it, so the last one is the offset's.
            let Some(at) = clock.rfind(['+', '-']) else {
                return Err(self.malformed("a timestamp with a zone"));
            };
            (&clock[..at], self.offset(&clock[at..])?)
        } else {
            (clock, 0)
        };

        Ok((date, self.time_of_day(clock)?, offset))
    }

    /// A `TIMESTAMPTZ`'s trailing offset — `±HH`, `±HH:MM` or `±HH:MM:SS` — as
    /// seconds east of UTC, which is the sign every `Core\Time` type uses.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a missing sign, a field that is not digits, or an
    /// hour past 18, which is the widest offset a zone has ever had.
    fn offset(&self, text: &str) -> io::Result<i32> {
        let sign = match text.as_bytes().first() {
            Some(b'+') => 1,
            Some(b'-') => -1,
            _ => return Err(self.malformed("a zone offset")),
        };

        let mut fields = text[1..].split(':');
        let hours: i32 = self.number(fields.next().unwrap_or_default(), 0, 18, "a zone offset")?;
        let minutes: i32 = match fields.next() {
            Some(field) => self.number(field, 0, 59, "a zone offset")?,
            None => 0,
        };
        let seconds: i32 = match fields.next() {
            Some(field) => self.number(field, 0, 59, "a zone offset")?,
            None => 0,
        };
        if fields.next().is_some() {
            return Err(self.malformed("a zone offset"));
        }

        Ok(sign * (hours * 3600 + minutes * 60 + seconds))
    }

    /// A `UUID`'s sixteen octets, out of the only rendering PostgreSQL writes:
    /// lower-case hex in the 8-4-4-4-12 grouping.
    ///
    /// The braced, bare and `{...}` forms the server *accepts* on input are
    /// never what it hands back, so they are not read here — a body in one of
    /// them did not come from a `uuid` column.
    ///
    /// # Errors
    ///
    /// `InvalidData` for any other length, a hyphen out of place, or a
    /// non-hexadecimal digit.
    fn uuid(&self, text: &str) -> io::Result<[u8; 16]> {
        let bytes = text.as_bytes();
        if bytes.len() != 36 || [8, 13, 18, 23].iter().any(|&at| bytes[at] != b'-') {
            return Err(self.malformed("a uuid"));
        }

        let mut octets = [0u8; 16];
        let mut digits = bytes.iter().filter(|&&byte| byte != b'-');
        for octet in &mut octets {
            let (Some(&high), Some(&low)) = (digits.next(), digits.next()) else {
                return Err(self.malformed("a uuid"));
            };
            let (Some(high), Some(low)) = (hex_digit(high), hex_digit(low)) else {
                return Err(self.malformed("a uuid"));
            };
            *octet = high * 16 + low;
        }

        Ok(octets)
    }

    /// PostgreSQL's `money`, out of whatever rendering `lc_monetary` chose.
    ///
    /// `cash_out` writes the server's locale: a currency symbol, a group
    /// separator and a decimal separator that are all the locale's, and a
    /// negative that is either a sign or the accounting parentheses. Two of
    /// those read unambiguously — the symbol is whatever is neither a digit nor
    /// a separator, and both negative forms are visible — so the rule is
    /// positional and about the separators alone: **the last separator is the
    /// decimal point when one or two digits follow it, and every other
    /// separator is grouping.**
    ///
    /// That is exact for every locale whose currency has two fractional digits
    /// and for every locale whose currency has none, which between them is
    /// every locale a server is realistically running: `$1,234.56`,
    /// `1.234,56 €` and `¥1,234` all decode to what they mean. It is wrong for
    /// the three-digit currencies (`KWD`, `BHD`, `OMR`), where a grouped
    /// `1,234` and a fractional `1.234` are the same shape and the text carries
    /// no second signal to break the tie. A deployment on one of those casts
    /// the column — `amount::numeric` is § 9's `NUMERIC` row and has no locale
    /// in it at all — and that is cheaper than the alternative, which is a
    /// round trip per connection to read `lc_monetary` out of the server.
    ///
    /// # Errors
    ///
    /// `InvalidData` when what is left after the separators is not a decimal
    /// this type can hold.
    fn money(&self, text: &str) -> io::Result<Decimal> {
        let negative = text.starts_with('(') || text.contains('-');
        let point = text.rfind(['.', ',']).filter(|&at| {
            let fraction = text[at + 1..]
                .bytes()
                .take_while(u8::is_ascii_digit)
                .count();
            (1..=2).contains(&fraction)
        });

        let mut digits = String::with_capacity(text.len() + 2);
        if negative {
            digits.push('-');
        }
        for (at, ch) in text.char_indices() {
            if ch.is_ascii_digit() {
                digits.push(ch);
            } else if Some(at) == point {
                if !digits.contains(|ch: char| ch.is_ascii_digit()) {
                    digits.push('0');
                }
                digits.push('.');
            }
        }

        self.decimal(&digits)
    }

    /// A `bytea`'s octets, out of either of the two text renderings.
    ///
    /// `bytea_output` decides which one arrives and both are legal, so both are
    /// read here rather than one being pinned by a startup parameter: `hex` is
    /// the default, `escape` is what an older application's server is often
    /// still set to, and a `SET` this driver did not write is not something it
    /// can rule out of a session.
    ///
    /// The octets land in a `Vec` and are copied once more into the string
    /// allocation. The hex form's length is exactly known and could fill one
    /// directly; the escape form's is not, and one path for two renderings is
    /// worth a `memcpy` on the one column type where nothing else in the row
    /// is bigger.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a hex body of odd length or with a non-hex digit in
    /// it, and for an escape body whose backslash is not followed by another
    /// backslash or by three octal digits.
    fn bytea(&self, body: &[u8]) -> io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(body.len());

        if let Some(hex) = body.strip_prefix(br"\x") {
            if hex.len() % 2 != 0 {
                return Err(self.malformed("a hex-format bytea"));
            }
            for pair in hex.chunks_exact(2) {
                let (Some(high), Some(low)) = (hex_digit(pair[0]), hex_digit(pair[1])) else {
                    return Err(self.malformed("a hex-format bytea"));
                };
                out.push(high * 16 + low);
            }
            return Ok(out);
        }

        let mut at = 0;
        while at < body.len() {
            if body[at] != b'\\' {
                out.push(body[at]);
                at += 1;
                continue;
            }
            match body.get(at + 1) {
                Some(b'\\') => {
                    out.push(b'\\');
                    at += 2;
                }
                Some(&first @ b'0'..=b'3') => {
                    let (Some(&second), Some(&third)) = (body.get(at + 2), body.get(at + 3)) else {
                        return Err(self.malformed("an escape-format bytea"));
                    };
                    if !matches!(second, b'0'..=b'7') || !matches!(third, b'0'..=b'7') {
                        return Err(self.malformed("an escape-format bytea"));
                    }
                    out.push((first - b'0') * 64 + (second - b'0') * 8 + (third - b'0'));
                    at += 4;
                }
                _ => return Err(self.malformed("an escape-format bytea")),
            }
        }

        Ok(out)
    }

    /// § 9's `NUMERIC` and `money` rows: the server's own rendering as a
    /// [`Decimal`], and the two reasons it may not be one.
    ///
    /// **The two are separate refusals because they are separate jobs.** A
    /// column holding a number past `rule:types/decimal`'s 96 mantissa bits or
    /// scale of 28 — a `NUMERIC(30,10)` is one — is a schema question, and the
    /// answer is the column's own type or a cast in the statement; it is
    /// refused rather than narrowed, because a truncated amount that looks
    /// right is the failure this type exists to make impossible. A rendering
    /// that is not a decimal literal at all is the other refusal, and it is
    /// [`Self::malformed`]'s: a `NUMERIC`'s `NaN`, which this type has no
    /// value for, and a body this driver was not sent, which is a bug over
    /// here rather than a number over there. [`Decimal::read`] is what tells
    /// the two apart.
    ///
    /// # Errors
    ///
    /// `InvalidData` either way, and neither message carries the value.
    fn decimal(&self, text: &str) -> io::Result<Decimal> {
        Decimal::read(text).map_err(|why| match why {
            NotDecimal::PastRange => io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "column {:?} of type OID {} holds more precision than a `decimal` does — 96 \
                     mantissa bits and a scale of 28 — so the column's own type is what narrows",
                    self.name, self.type_oid
                ),
            ),
            NotDecimal::Unreadable => self.malformed("a decimal"),
        })
    }

    /// The error a body that will not decode carries.
    ///
    /// **The body is not in it.** A decode failure is exactly the case where
    /// the value is most likely to be the thing that must not be written down,
    /// and this module's `Debug` implementations hold the same line for the
    /// same reason: an operator reads the offending value out of the database,
    /// where it is already access-controlled, rather than out of a log where it
    /// is not.
    fn malformed(&self, wanted: &str) -> io::Error {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "column {:?} of type OID {} did not decode as {wanted}",
                self.name, self.type_oid
            ),
        )
    }
}

/// An ISO-rendered date's era suffix, split off: `true` is the `BC` half of the
/// calendar, which PostgreSQL writes as a suffix and `Core\Time` counts as a
/// year at or below zero.
fn era(text: &str) -> (&str, bool) {
    match text.strip_suffix(" BC") {
        Some(head) => (head, true),
        None => (text, false),
    }
}

/// One hexadecimal digit's value, in either case, or `None` for anything else.
fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// A statement's read state: what the portal described, what ended it, and the
/// event it is being timed by — everything a row needs that is not the wire.
///
/// It is a type of its own because `rule:core-classes/db-statement-members`'s rows are reached two ways and
/// only one of them can hold a borrow:
///
/// - The **buffered** members drain their rows inside the call that started the
///   statement, so [`PgRows`] keeps this beside a borrow of the connection and
///   the borrow checker is what refuses a second statement.
/// - **`Core\Db\Connection::stream`** hands a cursor back to the program and is
///   advanced by a *later* call, with nothing of the connection borrowed in
///   between. A borrow cannot span that, so its copy of this state is parked on
///   the connection ([`PgConn::stream`]) and [`State::Streaming`] is what
///   refuses the second statement instead — the same refusal `rule:core-classes/db-connection-busy-state` gives
///   every driver, read off the state rather than off a lifetime.
///
/// Both drive [`next_row_of`], which is the one place in this driver a
/// `DataRow` is read, so the two paths cannot disagree about what ends a stream
/// or about what `lastId` saw on the way past.
#[derive(Debug)]
pub(crate) struct PgCursor {
    columns: Vec<PgColumn>,
    tag: Option<String>,
    last_id: Option<u64>,
    /// `rule:observability/a-query-is-a-trace-event`'s trace event for this statement, opened when it went out
    /// and ended by whatever ends the stream — [`crate::span`] owns why it is
    /// built from the SQL and never from the parameters.
    span: QuerySpan,
}

/// A statement's result stream, and the connection it is borrowed from.
///
/// Alive, this is [`State::Streaming`]: the wire holds messages belonging to
/// this statement, and `rule:core-classes/db-statement-members` refuses a second one. Drained by
/// [`PgRows::next_row`] or dropped, it is [`State::Idle`] again — the module doc
/// owns why abandonment is ordinary here rather than poison.
///
/// Generic in the stream for the same reason [`Wire`] is: the sequencing below
/// is then assertable against a scripted server, with no socket and no
/// certificate. A connection's own rows are always over the default.
pub struct PgRows<'a, S: Read + Write = PgStream> {
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    /// Everything about this stream that is not the borrow — [`PgCursor`] owns
    /// why that is a split rather than its fields inlined here.
    reading: PgCursor,
}

impl<S: Read + Write> std::fmt::Debug for PgRows<'_, S> {
    /// The shape of the result and where the wire is, and nothing that arrived.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgRows")
            .field("columns", &self.reading.columns.len())
            .field("state", &self.state.get())
            .finish_non_exhaustive()
    }
}

/// The affected-row count inside a `CommandComplete` tag, or `None` where the
/// command has none.
///
/// PostgreSQL writes the count as the tag's last field, and which commands
/// carry one at all is fixed by the protocol rather than derivable from the
/// shape: `UPDATE 3`, `SELECT 2`, `MERGE 7`, and `INSERT 0 3`, whose *first*
/// number is an OID and neither a row count nor a last insert id — a driver
/// reading that one would answer `0` for every insert on every server since
/// PostgreSQL 12, and `lastId` comes from `RETURNING` here for the same
/// reason.
///
/// `BEGIN`, `SET` and every DDL tag carry no count, which is answered as
/// `None` rather than `0`: those are two different facts, and the `Core\Db`
/// layer renders the second as zero rows knowing which one it has.
///
/// The command word is matched rather than the last field alone, because a
/// tag that happens to end in a number is not the same as a tag reporting a
/// count, and a command word this driver has never heard of is better read as
/// "no count" than as whatever its last field parses to.
fn affected_rows(tag: &str) -> Option<u64> {
    let (command, rest) = tag.split_once(' ')?;
    if !matches!(
        command,
        "INSERT" | "UPDATE" | "DELETE" | "MERGE" | "SELECT" | "MOVE" | "FETCH" | "COPY"
    ) {
        return None;
    }

    rest.rsplit(' ').next()?.parse().ok()
}

/// The identifier the row just read carries, for [`PgRows::last_id`]: its first
/// column, where the statement declared that column as an integer.
///
/// The type check is what keeps this off the ordinary query path. A statement
/// whose first column is text — every `SELECT` of a name, and every statement
/// with no `RETURNING` clause at all — pays one comparison per row and never
/// looks at the body; one that really does hand back a key pays a parse of the
/// twenty bytes a `BIGINT` can spell.
fn returned_id(columns: &[PgColumn], row: &PgRow) -> Option<u64> {
    let first = columns.first()?;
    if !matches!(first.type_oid, oid::INT2 | oid::INT4 | oid::INT8 | oid::OID) {
        return None;
    }

    let body = row.column(0).ok()??;
    std::str::from_utf8(body).ok()?.parse().ok()
}

impl<S: Read + Write> PgRows<'_, S> {
    /// What the portal said its rows look like, empty for a statement that
    /// returns none.
    #[must_use]
    pub fn columns(&self) -> &[PgColumn] {
        &self.reading.columns
    }

    /// The server's `CommandComplete` tag — `INSERT 0 3`, `SELECT 2` — once the
    /// stream has ended, and `None` while rows may still arrive.
    ///
    /// The raw tag, because it says more than a count does: [`Self::affected`]
    /// is the number [ADR 0067 § 4](/docs/decisions/0067.md)'s
    /// `execute` answers with, and this is what an error message quotes when a
    /// statement did something other than what its caller expected.
    #[must_use]
    pub fn command_tag(&self) -> Option<&str> {
        self.reading.tag.as_deref()
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s affected-row count,
    /// once the stream has ended.
    ///
    /// `None` twice over, and the caller can tell which from
    /// [`Self::command_tag`]: while rows may still arrive, and for a command
    /// whose tag carries no count at all. [`affected_rows`] owns which
    /// commands those are.
    ///
    /// § 4's `changed` is this same number on PostgreSQL. The distinction
    /// MySQL draws between rows matched and rows actually altered has nothing
    /// in this protocol to read it out of, and inventing a second count that
    /// always equalled the first would be a difference callers wrote code
    /// against.
    #[must_use]
    pub fn affected(&self) -> Option<u64> {
        affected_rows(self.reading.tag.as_deref()?)
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s `lastId`: the first
    /// column of the **last** row this statement returned, where the statement
    /// declared that column as an integer.
    ///
    /// PostgreSQL has no last-insert-id in its protocol at all. An `INSERT`'s
    /// tag is `INSERT 0 3`, whose first number is the inserted row's OID and is
    /// `0` on every server since PostgreSQL 12 — never a key, and never the one
    /// of a table declared the ordinary way. The id is therefore whatever a
    /// `RETURNING` clause handed back, and reading it here is also what makes it
    /// belong to the write that produced it rather than to the connection:
    /// there is no connection-level state for `mysqli_insert_id`'s
    /// stale-after-an-unrelated-statement hazard to live in, which is the
    /// refusal `docs/spec/02-php-migration.md` records against that function.
    ///
    /// `None`, then, for a statement with no `RETURNING` clause — it returned no
    /// rows — for one whose first returned column is not an integer, and while
    /// rows may still arrive. The spec's `lastId` is a `?uint`, so a negative
    /// value is not an id either and is `None` with them.
    #[must_use]
    pub fn last_id(&self) -> Option<u64> {
        self.reading.last_id
    }

    /// [ADR 0067 § 11](/docs/decisions/0067.md)'s trace event for
    /// this statement.
    ///
    /// Borrowed rather than taken, because a caller reading it mid-stream is
    /// asking a running statement how far it has got — [`QuerySpan::duration`]
    /// answers from the clock until the stream ends and freezes it. Nothing
    /// consumes one yet; [`crate::span`]'s module doc owns what it is waiting
    /// for.
    #[must_use]
    pub fn span(&self) -> &QuerySpan {
        &self.reading.span
    }

    /// Names the `[db.<name>]` block this statement ran on, for the layer that
    /// resolved it — [`QuerySpan::name`] owns why the driver cannot.
    pub fn name_connection(&mut self, connection: &str) {
        self.reading.span.name(connection);
    }

    /// The next row, or `None` once the stream has ended.
    ///
    /// Ending it is what returns the connection to [`State::Idle`]: the
    /// `CommandComplete` is followed by the `ReadyForQuery` the `Sync` in the
    /// same flush guaranteed, and this reads through to it before answering.
    /// Deliberately not `Iterator::next` — every call can fail, and a stream
    /// that hides that behind `Option` would have to swallow a wire error.
    ///
    /// # Errors
    ///
    /// The server's own error, which still ends the stream cleanly and leaves
    /// the connection idle, or a wire failure, which poisons it.
    pub fn next_row(&mut self) -> io::Result<Option<PgRow>> {
        next_row_of(self.wire, self.state, &mut self.reading)
    }
}

/// One row off a live stream, or `None` once it has ended — the whole of
/// [`PgRows::next_row`], and of [`PgConn::stream_next_row`] with it.
///
/// Free, and generic in the stream, for two reasons that point the same way.
/// The streamed path has no `PgRows` to call a method on: its state is parked
/// on the connection and its wire is reached through a fresh borrow per row.
/// And a `PgConn`'s own `wire` is at the default type parameter, so anything
/// reachable only through an inherent method on it needs a socket and a
/// certificate to reach at all — written here, the sequencing is assertable
/// against a scripted server instead.
///
/// # Errors
///
/// As [`PgRows::next_row`].
fn next_row_of<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    reading: &mut PgCursor,
) -> io::Result<Option<PgRow>> {
    // The state is the only bookkeeping: anything that ended this stream —
    // a completion, a server error, a poisoning — has already left it.
    if state.get() != State::Streaming {
        return Ok(None);
    }

    loop {
        match read_or_poison(wire, state)? {
            backend::Message::DataRow(body) => {
                let row = PgRow { body };
                // § 4's `lastId`, taken as the row goes past: the last row
                // is the answer, and a row borrows the wire's buffer, so
                // once the next one has arrived there is nothing left to
                // read it out of.
                let id = returned_id(&reading.columns, &row);
                reading.last_id = id;
                reading.span.row();
                return Ok(Some(row));
            }
            backend::Message::CommandComplete(body) => {
                let tag = body
                    .tag()
                    .inspect_err(|_| state.set(State::Poisoned))?
                    .to_owned();
                reading.span.finished(affected_rows(&tag));
                reading.tag = Some(tag);
                drain_to_ready(wire, state)?;
                return Ok(None);
            }
            // `Bind` on an empty query string. Not an error: it is what a
            // caller that built its SQL from an empty template sent.
            backend::Message::EmptyQueryResponse => {
                reading.span.finished(None);
                drain_to_ready(wire, state)?;
                return Ok(None);
            }
            backend::Message::ErrorResponse(body) => {
                let error = server_error(&body);
                // A refused statement is still a statement that took time,
                // and § 11 gives a span no success field to lose: the rows
                // it reports are the ones that did arrive, and the error is
                // the caller's own return value.
                reading.span.finished(None);
                drain_to_ready(wire, state)?;
                return Err(error);
            }
            backend::Message::NoticeResponse(_)
            | backend::Message::ParameterStatus(_)
            | backend::Message::NotificationResponse(_) => {}
            _ => return Err(out_of_sequence(state)),
        }
    }
}

impl<S: Read + Write> Drop for PgRows<'_, S> {
    /// Abandonment, and it is the ordinary case rather than an error.
    ///
    /// The `Sync` went out with the statement, so the `ReadyForQuery` that ends
    /// it is coming whether or not anybody read the rows in between: draining
    /// to it is deterministic, and `rule:core-classes/db-connection-busy-state` is explicit that a driver which
    /// can do that returns the connection to the pool instead of closing it.
    /// A read that fails on the way poisons it, through the same helper every
    /// other read here uses.
    fn drop(&mut self) {
        if self.state.get() == State::Streaming {
            drop(drain_to_ready(self.wire, self.state));
        }
    }
}

/// Writes `rule:core-classes/db-one-api`'s one round trip and reads up to the first row.
///
/// `Parse`, `Bind`, `Describe`, `Execute` and `Sync` go into one buffer and out
/// in one flush, and this returns once the portal has described itself — which
/// is the point rows may start arriving and [`State::Streaming`] is true.
///
/// `cache` is what decides whether the `Parse` is in that buffer at all. On a
/// hit it is not — the round trip `rule:core-classes/db-one-api` says PostgreSQL already does not
/// pay — and the batch opens at `Bind` against a name the server is already
/// holding. On a miss that had to evict, the victim's `Close` rides in the
/// *same* buffer: the batch's own `Sync` is what bounds it, so deallocating a
/// statement never costs a round trip of its own.
///
/// The cache is written only once the parse has landed. `ParseComplete` arrives
/// before the row description this returns at, so every path that reaches the
/// commit has proof the server holds the statement; every path that does not
/// leaves the cache exactly as it found it, and the statement is simply parsed
/// again next time.
///
/// # Errors
///
/// As [`PgConn::query`].
fn open_portal<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    cache: &mut StatementCache,
    sql: &str,
    params: &[Option<&[u8]>],
) -> io::Result<PgCursor> {
    if !state.get().may_start_statement() {
        return Err(second_statement(state));
    }

    // `rule:observability/a-query-is-a-trace-event`'s span, opened before the batch is built so its duration is
    // what the caller waited rather than what the server spent. It is handed
    // `sql` and not `params`, which is the whole of § 11's "never parameters" —
    // `crate::span`'s module doc owns why that is a signature and not a rule.
    let span = QuerySpan::opened(Driver::Postgres, sql);

    // The arity is the parameter count, which `rule:core-classes/db-parameters`'s `inList` expansion
    // has already moved by the time the SQL reaches here.
    let arity = params.len();
    let prepared = cache.prepare(sql, arity);

    let mut out = BytesMut::new();
    match &prepared {
        // The server has it: no `Parse`, and the batch is four messages.
        Prepared::Hit(_) => {}
        Prepared::Miss { name, evicted } => {
            if let Some(closing) = evicted {
                // `b'S'` is the statement, not the portal. It goes first so the
                // server is never holding both at once.
                frontend::close(b'S', closing, &mut out)?;
            }
            frontend::parse(name, sql, [], &mut out)?;
        }
        Prepared::Unnamed => frontend::parse(UNNAMED, sql, [], &mut out)?,
    }
    frontend::bind(
        // The **portal** is `bind`'s first name and the statement its second,
        // which is the order the `Bind` message itself carries them in. Passing
        // the statement first binds against the unnamed statement instead, and
        // that one was only ever parsed when the cache is disabled — with a
        // capacity of any size it is a `26000` on the very first statement.
        UNNAMED,
        prepared.name(),
        // Both format lists empty, which is the protocol's spelling for "all
        // text". The module doc owns why text and not binary.
        [],
        params.iter().copied(),
        |param, buf| match param {
            Some(bytes) => {
                buf.extend_from_slice(bytes);
                Ok(IsNull::No)
            }
            None => Ok(IsNull::Yes),
        },
        [],
        &mut out,
    )
    .map_err(|e| match e {
        frontend::BindError::Conversion(e) => io::Error::new(io::ErrorKind::InvalidInput, e),
        frontend::BindError::Serialization(e) => e,
    })?;
    // `b'P'` is the *portal*, not the statement: describing the portal answers
    // with the row description alone, where describing the statement would also
    // send a `ParameterDescription` nothing here reads.
    frontend::describe(b'P', UNNAMED, &mut out)?;
    // `0` is every row. This driver never suspends a portal, so
    // `PortalSuspended` is a message it does not have to have an answer for.
    frontend::execute(UNNAMED, 0, &mut out)?;
    frontend::sync(&mut out);

    state.set(State::Executing);
    if let Err(e) = wire.send(&mut out) {
        // A failed `write_all` may have left part of a message on the wire, and
        // there is no boundary to find after one.
        state.set(State::Poisoned);
        return Err(e);
    }

    let columns = loop {
        match read_or_poison(wire, state)? {
            // `CloseComplete` is the evicted statement's, and it arrives ahead
            // of this batch's own answers.
            backend::Message::CloseComplete
            | backend::Message::ParseComplete
            | backend::Message::BindComplete => {}
            backend::Message::RowDescription(body) => {
                break columns_of(&body).inspect_err(|_| state.set(State::Poisoned))?;
            }
            // A statement that returns no rows at all: `Streaming` is still the
            // right state, because the `CommandComplete` and `ReadyForQuery`
            // that end it are messages a second statement must not be written
            // over.
            backend::Message::NoData => break Vec::new(),
            backend::Message::ErrorResponse(body) => {
                let error = server_error(&body);
                drain_to_ready(wire, state)?;
                return Err(error);
            }
            backend::Message::NoticeResponse(_)
            | backend::Message::ParameterStatus(_)
            | backend::Message::NotificationResponse(_) => {}
            _ => return Err(out_of_sequence(state)),
        }
    };

    // The parse landed, so the name is one the server will answer to until we
    // close it. A `Hit` was already recorded and an `Unnamed` never is.
    if let Prepared::Miss { name, .. } = prepared {
        cache.commit(sql, arity, name);
    }

    state.set(State::Streaming);
    Ok(PgCursor {
        columns,
        tag: None,
        last_id: None,
        span,
    })
}

/// [`open_portal`] with the read state lent out beside a borrow of the
/// connection: the shape every buffered member of `rule:core-classes/db-statement-members` wants, and the
/// one `Core\Db\Connection::stream` is the single caller that cannot use.
///
/// # Errors
///
/// As [`PgConn::query`].
fn start_statement<'a, S: Read + Write>(
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    cache: &mut StatementCache,
    sql: &str,
    params: &[Option<&[u8]>],
) -> io::Result<PgRows<'a, S>> {
    let reading = open_portal(wire, state, cache, sql, params)?;
    Ok(PgRows {
        wire,
        state,
        reading,
    })
}

/// `rule:core-classes/db-statement-members`'s `executeMany`: one `Parse`, N `Bind`/`Execute` pairs, and the
/// affected counts summed.
///
/// All of it goes out in **one flush**, which is the whole reason § 4 has a
/// member for this rather than leaving it to a loop over [`start_statement`]: a
/// thousand writes cost the round trip one statement costs, and the `Parse` is
/// in the buffer at most once — [`StatementCache`] answers whether it is there
/// at all, exactly as it does for a single statement.
///
/// **Each execution carries its own `Sync`, and that is a semantic choice
/// rather than a spelling.** One `Sync` for the whole batch would cost nothing
/// less — the messages are in the same buffer either way — but it would make
/// the batch a single implicit transaction, so a failure at set 900 would roll
/// back the 899 writes before it and would have held their locks from the
/// first. That is the hidden `BEGIN` § 4 refuses in as many words: a caller who
/// wants all-or-nothing writes `transaction(fn ($tx) => $tx->executeMany(…))`,
/// and one who did not ask for it gets what an autocommitting MySQL would give
/// — each execution independent, and the ones after a failure still attempted,
/// because the server resumes at the next `Sync`.
///
/// The answer is the sum of what each `CommandComplete` reports, and a tag
/// carrying no count contributes nothing rather than failing the batch;
/// [`affected_rows`] owns which commands those are. A `DataRow` a `RETURNING`
/// clause produced is discarded: § 4 gives this member a `uint` return and no
/// second one to hand rows back through.
///
/// An empty `sets` is § 4's no-op answering `0` — nothing is written, and
/// nothing is prepared for executions that will not happen. The state check
/// still runs ahead of it, because § 4's refusal is a property of the
/// connection rather than of the payload: a statement written to a streaming
/// connection is the same bug whether or not its parameter list was empty, and
/// a rule that fired only for some inputs would surface in production on the
/// batch that happened not to be empty.
///
/// # Errors
///
/// `InvalidInput` when the connection is not [`State::Idle`], as
/// [`PgConn::query`], and for a `sets` whose members do not all bind the same
/// number of parameters — one prepare has one parameter count, and it is what
/// § 1's cache is keyed on beside the SQL. Otherwise the **first** error any
/// execution drew, reported only once every `Sync` has been accounted for so
/// that the connection is left idle and poolable, or a wire failure, which
/// poisons it.
fn execute_many<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    cache: &mut StatementCache,
    sql: &str,
    sets: &[&[Option<&[u8]>]],
) -> io::Result<u64> {
    if !state.get().may_start_statement() {
        return Err(second_statement(state));
    }

    let Some(first) = sets.first() else {
        return Ok(0);
    };
    let arity = first.len();
    if let Some(odd) = sets.iter().find(|set| set.len() != arity) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "one executeMany bound {arity} parameters in its first set and {} in another, and \
                 `rule:core-classes/db-statement-members`'s one prepare has one parameter count",
                odd.len()
            ),
        ));
    }

    let prepared = cache.prepare(sql, arity);

    let mut out = BytesMut::new();
    match &prepared {
        Prepared::Hit(_) => {}
        Prepared::Miss { name, evicted } => {
            if let Some(closing) = evicted {
                frontend::close(b'S', closing, &mut out)?;
            }
            frontend::parse(name, sql, [], &mut out)?;
        }
        Prepared::Unnamed => frontend::parse(UNNAMED, sql, [], &mut out)?,
    }
    for set in sets {
        frontend::bind(
            // The same unnamed portal every time, which the protocol destroys
            // at the next `Bind` — and by then this one's `Execute` has run.
            // A name per set would be N `Close` messages for no gain. It is the
            // *first* name because that is the order `Bind` carries them in;
            // [`start_statement`] owns what naming them the other way round
            // costs.
            UNNAMED,
            prepared.name(),
            [],
            set.iter().copied(),
            |param, buf| match param {
                Some(bytes) => {
                    buf.extend_from_slice(bytes);
                    Ok(IsNull::No)
                }
                None => Ok(IsNull::Yes),
            },
            [],
            &mut out,
        )
        .map_err(|e| match e {
            frontend::BindError::Conversion(e) => io::Error::new(io::ErrorKind::InvalidInput, e),
            frontend::BindError::Serialization(e) => e,
        })?;
        // No `Describe`: nothing here reads a row description, and the count
        // comes from the completion tag.
        frontend::execute(UNNAMED, 0, &mut out)?;
        frontend::sync(&mut out);
    }

    state.set(State::Executing);
    if let Err(e) = wire.send(&mut out) {
        state.set(State::Poisoned);
        return Err(e);
    }

    let mut affected = 0u64;
    let mut parsed = false;
    let mut refused: Option<io::Error> = None;
    // One `ReadyForQuery` per `Sync`, which is one per execution: reading them
    // by count is what keeps the answers from skewing against the requests
    // after the server has skipped an execution it refused.
    for _ in sets {
        loop {
            match read_or_poison(wire, state)? {
                backend::Message::ReadyForQuery(_) => break,
                backend::Message::ParseComplete => parsed = true,
                // `CloseComplete` is the evicted statement's, `BindComplete`
                // this execution's, and a `DataRow` is a `RETURNING` clause's.
                backend::Message::CloseComplete
                | backend::Message::BindComplete
                | backend::Message::DataRow(_)
                | backend::Message::EmptyQueryResponse => {}
                backend::Message::CommandComplete(body) => {
                    let tag = body.tag().inspect_err(|_| state.set(State::Poisoned))?;
                    affected = affected.saturating_add(affected_rows(tag).unwrap_or(0));
                }
                backend::Message::ErrorResponse(body) => {
                    let error = server_error(&body);
                    // The first one is the one with a cause, as § 13's reset
                    // reports it: the executions after it may only be that
                    // execution's consequences.
                    if refused.is_none() {
                        refused = Some(error);
                    }
                }
                backend::Message::NoticeResponse(_)
                | backend::Message::ParameterStatus(_)
                | backend::Message::NotificationResponse(_) => {}
                _ => return Err(out_of_sequence(state)),
            }
        }
    }

    state.set(State::Idle);
    if let Some(error) = refused {
        return Err(error);
    }

    // As [`start_statement`], and with one more thing to be sure of: a `Parse`
    // is undone with the implicit transaction it ran in, so the proof a name is
    // cacheable is a `ParseComplete` *and* a batch that drew no error at all.
    // Caching one the server rolled back is a `26000` on the next hit, on a
    // connection that is otherwise fine.
    if let Prepared::Miss { name, .. } = prepared
        && parsed
    {
        cache.commit(sql, arity, name);
    }

    Ok(affected)
}

/// `rule:security/db-pool-reset-is-a-boundary`'s PostgreSQL reset, in the order that section lists it.
///
/// `DISCARD TEMP` is the server's own spelling of § 13's "drop the session's
/// temporary schema", and the reason it is that rather than `DISCARD ALL` is the
/// same reason § 13 gives: `DISCARD ALL` also runs `DEALLOCATE ALL`, which would
/// throw away the statement cache pooling exists to preserve.
///
/// `RESET ALL` covers `SET ROLE` and `SET SESSION AUTHORIZATION` without a
/// command of their own — `role` and `session_authorization` are settable
/// run-time parameters, so they are two of the things "all" means.
const RESET_COMMANDS: [&str; 6] = [
    "ROLLBACK",
    "RESET ALL",
    "CLOSE ALL",
    "UNLISTEN *",
    "SELECT pg_advisory_unlock_all()",
    "DISCARD TEMP",
];

/// Runs [`RESET_COMMANDS`] as one pipelined batch and reports the first refusal.
///
/// **One round trip for six commands**, which is what makes § 13's reset cheap
/// enough to be unconditional. These go out as *simple* `Query` messages rather
/// than through the extended path, and that is the whole reason the batch is
/// safe to pipeline: a simple `Query` carries its own implicit `Sync`, so each
/// of the six ends in a `ReadyForQuery` whether it succeeded or not and the
/// answers cannot skew against the requests. `RESET ALL` is not a statement
/// worth preparing anyway, and none of the six takes a parameter.
///
/// Every command runs even after one has failed — they are independent, and
/// stopping early would leave the connection *less* clean than continuing.
/// The **first** refusal is the one reported, because that is the one with a
/// cause; the ones after it may only be consequences.
///
/// A message that is not an error or a boundary is discarded, and unlike the
/// extended path that is not poison: this batch is a fixed list carrying no
/// caller's SQL, so there is nothing a peer can answer with that changes what
/// happens next, and the boundary after each command is the protocol's promise
/// rather than an inference from the sequence.
///
/// # Errors
///
/// `InvalidInput` for a connection that is not [`State::Idle`], the first
/// command's own error, or the wire failure that stopped the batch — which
/// poisons the connection, as any failed read does.
fn reset_session<S: Read + Write>(wire: &mut Wire<S>, state: &Cell<State>) -> io::Result<()> {
    if !state.get().may_start_statement() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "a connection that is {:?} was asked to reset, and § 13's reset is only meaningful \
                 at a message boundary",
                state.get()
            ),
        ));
    }

    let mut out = BytesMut::new();
    for command in RESET_COMMANDS {
        frontend::query(command, &mut out)?;
    }

    state.set(State::Executing);
    if let Err(e) = wire.send(&mut out) {
        state.set(State::Poisoned);
        return Err(e);
    }

    let mut refused: Option<io::Error> = None;
    for _ in RESET_COMMANDS {
        loop {
            match read_or_poison(wire, state)? {
                backend::Message::ReadyForQuery(_) => break,
                backend::Message::ErrorResponse(body) => {
                    let error = server_error(&body);
                    if refused.is_none() {
                        refused = Some(error);
                    }
                }
                _ => {}
            }
        }
    }

    state.set(State::Idle);
    match refused {
        None => Ok(()),
        Some(error) => Err(error),
    }
}

/// [ADR 0067 § 7](/docs/decisions/0067.md)'s `BEGIN`, or the
/// `SAVEPOINT` a nested `transaction()` is.
///
/// **The depth decides which**, and the depth is the connection's rather than
/// the caller's: § 7 gives no explicit savepoint API and no `inTransaction()`,
/// so a library that wraps its own writes stays callable from inside a caller's
/// transaction precisely because it cannot tell and does not have to. `depth`
/// counts the levels open — 0 means this is the outermost `BEGIN`.
///
/// A **nested** call may not carry `{isolation, readOnly}`, and asking is
/// refused rather than ignored. PostgreSQL settles both for the whole
/// transaction, at its first statement, so there is nothing a savepoint could
/// do with them; running the callable at the *outer* transaction's level while
/// its author wrote `Isolation::Serializable` would give it weaker semantics
/// than the program asked for, and nothing would say so. § 7's composition is
/// not what this costs: a nested `transaction()` with no options is the case
/// § 7 argues for and it still composes.
///
/// # Errors
///
/// `InvalidInput` for a nested call carrying either option, otherwise as
/// [`simple_command`]. The depth moves only after the command was accepted, so
/// a refused `BEGIN` leaves a connection that is still in no transaction.
fn begin<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    depth: &Cell<u32>,
    isolation: Option<Isolation>,
    read_only: bool,
) -> io::Result<QuerySpan> {
    let open = depth.get();
    let command = if open == 0 {
        begin_command(isolation, read_only)
    } else if isolation.is_some() || read_only {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "a transaction nested {open} deep asked for its own isolation level or read-only \
                 mode, and PostgreSQL settles both for the whole transaction: ask for them on the \
                 outermost `transaction()`, or give this one a `{{shared: false}}` connection of \
                 its own"
            ),
        ));
    } else {
        format!("SAVEPOINT {}", savepoint_name(open))
    };

    let span = simple_command(wire, state, &command)?;
    depth.set(open + 1);
    Ok(span)
}

/// § 7's `COMMIT`, or the `RELEASE SAVEPOINT` that closes a nested level.
///
/// # Errors
///
/// `InvalidInput` for a connection in no transaction, otherwise as
/// [`simple_command`]. PostgreSQL has already rolled the transaction back by
/// the time it refuses an outermost `COMMIT`, so there is nothing left for the
/// caller to undo — the connection is idle and poolable, and only the callable's
/// own side effects outlive it. **The count follows the connection there**: an
/// outermost commit the *server* refused leaves the depth at 0, unlike every
/// other refusal in this family, because the level really is gone. A caller that
/// opens the next transaction on that connection — § 7's `{retries: n}` is the
/// one that does — must get a `BEGIN` and not a `SAVEPOINT` against nothing. A
/// refused `RELEASE SAVEPOINT` says nothing of the kind and moves nothing, as in
/// [`roll_back`], and neither does a refusal this crate made itself — § 4's busy
/// connection, or a wire that failed before the command was processed.
fn commit<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    depth: &Cell<u32>,
) -> io::Result<QuerySpan> {
    let open = open_transaction(depth, "commit")?;
    let command: Cow<'_, str> = if open == 1 {
        Cow::Borrowed("COMMIT")
    } else {
        Cow::Owned(format!("RELEASE SAVEPOINT {}", savepoint_name(open - 1)))
    };

    let span = match simple_command(wire, state, &command) {
        Ok(span) => span,
        Err(refused) => {
            // Only a refusal the *server* worded says the transaction is over: a
            // busy connection and a wire failure are refusals this crate made
            // without the `COMMIT` ever being processed, and moving the count on
            // one of those would tell the caller a level closed that is still open.
            if open == 1 && ServerError::of(&refused).is_some() {
                depth.set(0);
            }
            return Err(refused);
        }
    };
    depth.set(open - 1);
    Ok(span)
}

/// § 7's `ROLLBACK`, or the `ROLLBACK TO SAVEPOINT` that undoes a nested level.
///
/// **A nested rollback releases the savepoint it returned to, in the same
/// command.** `ROLLBACK TO SAVEPOINT` leaves the savepoint established — it can
/// be returned to again — but § 7's nested transaction is over by then, and a
/// loop that opens and abandons one per iteration would otherwise leave the
/// server holding a savepoint per iteration for as long as the outer
/// transaction runs. Two statements in one simple `Query` is one round trip and
/// carries no caller's SQL.
///
/// # Errors
///
/// `InvalidInput` for a connection in no transaction, otherwise as
/// [`simple_command`]. A refused rollback leaves the depth where it was: the
/// level is still open as far as the server is concerned, and the level above
/// it will roll back over this one anyway.
fn roll_back<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    depth: &Cell<u32>,
) -> io::Result<QuerySpan> {
    let open = open_transaction(depth, "roll back")?;
    let command: Cow<'_, str> = if open == 1 {
        Cow::Borrowed("ROLLBACK")
    } else {
        let name = savepoint_name(open - 1);
        Cow::Owned(format!(
            "ROLLBACK TO SAVEPOINT {name}; RELEASE SAVEPOINT {name}"
        ))
    };

    let span = simple_command(wire, state, &command)?;
    depth.set(open - 1);
    Ok(span)
}

/// The number of levels open, or the refusal for a connection in none.
///
/// § 7 has no `commit()` and no `rollBack()` on the connection, so only the
/// callable's own two exits reach these: a call with nothing open is this
/// driver's bug rather than a program's, and saying so is worth more than
/// sending a bare `ROLLBACK` that PostgreSQL answers with a warning nobody
/// reads.
///
/// Shared with [`crate::mysql`] rather than written twice: the wording is about
/// § 7's surface — which has no `commit()` on any backend — and not about
/// anything one protocol does.
///
/// # Errors
///
/// `InvalidInput`, naming the depth it was asked to close.
pub(crate) fn open_transaction(depth: &Cell<u32>, verb: &str) -> io::Result<u32> {
    match depth.get() {
        0 => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("a connection with no open transaction was asked to {verb} one"),
        )),
        open => Ok(open),
    }
}

/// The savepoint name for the level opened at `depth`.
///
/// Named by the depth rather than by a counter that only climbs: a name is in
/// use only while that one level is open, two open levels never share one, and
/// a transaction that opens and closes a nested one a thousand times reuses
/// `nvs_1` rather than leaving the server a thousand names. § 7 has no explicit
/// savepoint API, so no program can name one of these and nothing outside this
/// crate may depend on the spelling — [`crate::mysql`] takes the same names for
/// the same reason, so one backend's savepoint and another's read alike in a
/// server log.
pub(crate) fn savepoint_name(depth: u32) -> String {
    format!("nvs_{depth}")
}

/// The `BEGIN` a given `{isolation, readOnly}` renders to.
///
/// **§ 7's five levels become PostgreSQL's four, and only [`Isolation::Snapshot`]
/// collapses**: `REPEATABLE READ` *is* PostgreSQL's snapshot isolation, so
/// asking for either gets the same guarantee under the name this server uses
/// and neither is the missing level § 7 says to throw over.
/// [`Isolation::ReadUncommitted`] keeps its own spelling, which PostgreSQL
/// accepts and then runs as `READ COMMITTED`: the level asked for is the
/// weakest one there is and the level delivered is stronger, so no promise a
/// program was given is broken. § 7's refusal is for the other direction, and
/// PostgreSQL never has to make it.
///
/// **`read_only` renders `READ ONLY` or nothing, never `READ WRITE`.** The
/// option's absence means the connection's default, and an operator may have
/// set `default_transaction_read_only` on a standby; spelling `READ WRITE`
/// would widen from inside a program what the operator narrowed outside it,
/// which is the wrong direction for a security rule. A caller that wanted a
/// write transaction on such a server gets the server's refusal, which names
/// the real problem.
///
/// A `String` rather than a table of the ten spellings: this runs once per
/// transaction, next to a round trip, and the two options are independent.
fn begin_command(isolation: Option<Isolation>, read_only: bool) -> String {
    let mut command = String::from("BEGIN");

    if let Some(level) = isolation {
        command.push_str(" ISOLATION LEVEL ");
        command.push_str(match level {
            Isolation::ReadUncommitted => "READ UNCOMMITTED",
            Isolation::ReadCommitted => "READ COMMITTED",
            Isolation::RepeatableRead | Isolation::Snapshot => "REPEATABLE READ",
            Isolation::Serializable => "SERIALIZABLE",
        });
    }

    if read_only {
        command.push_str(" READ ONLY");
    }

    command
}

/// Writes one simple `Query` and reads to the `ReadyForQuery` it carries.
///
/// § 7's three commands take no parameters and are worth no cache entry, so
/// they go out on the *simple* path exactly as § 13's reset does: a simple
/// `Query` carries its own implicit `Sync`, so the boundary after it is the
/// protocol's promise rather than an inference from the sequence, and a message
/// that is neither an error nor that boundary is discarded rather than treated
/// as poison.
///
/// A busy connection is refused with [`second_statement`]'s wording rather than
/// one of its own, because § 4 is the rule being broken and both of its fixes
/// are what this caller needs to hear. § 13's reset is the one caller that says
/// something else, since "read the first statement's rows" is not advice a pool
/// return can act on.
///
/// # Errors
///
/// `InvalidInput` for a connection that is not [`State::Idle`], the server's
/// own error — which leaves the connection idle and poolable, the transaction
/// being over either way — or the wire failure that poisons it.
fn simple_command<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    sql: &str,
) -> io::Result<QuerySpan> {
    if !state.get().may_start_statement() {
        return Err(second_statement(state));
    }

    // `rule:observability/a-query-is-a-trace-event`'s span, opened where the extended-query path opens its own:
    // once the connection is this command's, so the duration is the round trip
    // and not the wait for a busy one. The text is this module's — § 7's
    // commands carry no caller's SQL at all — and [`QuerySpan`] is what carries
    // it to a trace, so a `COMMIT` reads there beside the statements it closed.
    let span = QuerySpan::opened(Driver::Postgres, sql);
    let mut out = BytesMut::new();
    frontend::query(sql, &mut out)?;

    state.set(State::Executing);
    if let Err(e) = wire.send(&mut out) {
        state.set(State::Poisoned);
        return Err(e);
    }

    let mut refused: Option<io::Error> = None;
    loop {
        match read_or_poison(wire, state)? {
            backend::Message::ReadyForQuery(_) => break,
            // The first refusal is the one reported; a second error before the
            // boundary can only be a consequence of it.
            backend::Message::ErrorResponse(body) if refused.is_none() => {
                refused = Some(server_error(&body));
            }
            _ => {}
        }
    }

    state.set(State::Idle);
    match refused {
        None => {
            let mut span = span;
            // No count and no rows: § 7's commands change nothing themselves,
            // and `affected` says "this statement reported a count" rather than
            // "it reported zero".
            span.finished(None);
            Ok(span)
        }
        Some(error) => Err(error),
    }
}

/// Reads one message, and poisons the connection if the wire itself failed.
///
/// Every read in the extended-query path goes through this, because the rule is
/// one rule: a message that did not arrive leaves no boundary to resume from,
/// and `rule:core-classes/db-connection-busy-state` closes such a connection rather than resetting it.
///
/// # Errors
///
/// As [`Wire::read_message`].
fn read_or_poison<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
) -> io::Result<backend::Message> {
    let message = wire.read_message();
    if message.is_err() {
        state.set(State::Poisoned);
    }
    message
}

/// Reads to the `ReadyForQuery` the statement's own `Sync` guarantees, and
/// returns the connection to [`State::Idle`].
///
/// Everything on the way is discarded: rows nobody asked for, a second error
/// after the first, the notices that come with either. That is what the server
/// itself does after an error — skip to the next `Sync` — and it is only
/// deterministic because the `Sync` was already on the wire before any of this
/// was read.
///
/// # Errors
///
/// As [`read_or_poison`], which is also what leaves the connection poisoned.
fn drain_to_ready<S: Read + Write>(wire: &mut Wire<S>, state: &Cell<State>) -> io::Result<()> {
    loop {
        if let backend::Message::ReadyForQuery(_) = read_or_poison(wire, state)? {
            state.set(State::Idle);
            return Ok(());
        }
    }
}

/// The columns a row description names.
///
/// # Errors
///
/// `InvalidData` for a description whose field count and body disagree.
fn columns_of(body: &backend::RowDescriptionBody) -> io::Result<Vec<PgColumn>> {
    let mut columns = Vec::new();
    let mut fields = body.fields();
    while let Some(field) = fields.next()? {
        columns.push(PgColumn {
            name: field.name().to_owned(),
            type_oid: field.type_oid(),
            type_modifier: field.type_modifier(),
        });
    }
    Ok(columns)
}

/// `rule:core-classes/db-statement-members`'s refusal of a second statement, and the one place its wording
/// lives.
///
/// **It names both fixes, always**, because § 4 requires the refusal to: the
/// buffered read that frees the connection, and the `{shared: false}`
/// connection that gives this statement one of its own. Those are the only two
/// answers, they fix different programs — one loop wants its rows in memory,
/// one genuinely wants two connections — and a message saying only that the
/// connection is busy leaves the caller to guess which of them their code
/// needs.
///
/// `nvs-stdlib` re-words this as § 4's `LogicError`: the fault class is its own
/// and so is the call site's spelling, per [`State::may_start_statement`].
/// What it re-words is this sentence, which is also the one a log carries
/// wherever nothing has — so it is the whole answer rather than a hint.
///
/// The state is quoted because it is the difference between the two bugs this
/// catches: `Streaming` is § 4's unread stream, and `Poisoned` is a connection
/// that is not going to work again whatever the caller does next.
pub(crate) fn second_statement(state: &Cell<State>) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "a statement was written to a connection that is {:?}, and `rule:core-classes/db-statement-members` allows one at \
             a time: read the first statement's rows into memory (`->all()`), or open a \
             `{{shared: false}}` connection so this statement has one of its own",
            state.get()
        ),
    )
}

/// Poisons the connection over a message the extended-query sequence does not
/// put where it arrived.
///
/// The frame decoded, so the wire is arguably at a boundary — but a peer that
/// sent this is not tracking the same sequence the driver is, and draining would
/// mean trusting exactly the sequencing that has just proven wrong. § 4's answer
/// to a boundary that cannot be proven is poison, and a poisoned connection is
/// closed rather than pooled.
fn out_of_sequence(state: &Cell<State>) -> io::Error {
    state.set(State::Poisoned);
    io::Error::new(
        io::ErrorKind::InvalidData,
        "the server sent a message the extended-query protocol does not put in a result stream",
    )
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    use std::io::{self, Read, Write};

    use std::cell::Cell;

    use postgres_protocol::Oid;

    use nvs_config::tree::Database;

    use super::{
        BlockError, CancelKey, PgColumn, PgConn, PgDate, PgScalar, PgTarget, PgTime, State, Wire,
        affected_rows, authenticate, execute_many, next_row_of, oid, open_portal, posix_time_zone,
        request_tls, start_statement,
    };
    use crate::conn::{ColumnType, DbErrorKind, Driver, Isolation, ServerError};
    use crate::sql::{DEFAULT_STATEMENT_CACHE, StatementCache};

    /// A cache that never caches, so a test about the wire asserts the unnamed
    /// statement rather than a cached name. The cached path has its own case.
    fn no_cache() -> StatementCache {
        StatementCache::new(0)
    }

    /// A server that answers the client rather than a script: the SASL
    /// exchange's every message depends on the one before it, so a canned
    /// transcript could only ever assert that we send *something*.
    ///
    /// A flush is the message boundary — every write in this module is
    /// `write_all` then `flush` — so the Rust closure is handed exactly what the
    /// driver considers one send.
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

    impl<F: FnMut(&[u8]) -> Vec<u8>> Write for Peer<F> {
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

    impl<F: FnMut(&[u8]) -> Vec<u8>> Read for Peer<F> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let left = &self.inbound[self.read..];
            let take = left.len().min(buf.len());
            buf[..take].copy_from_slice(&left[..take]);
            self.read += take;
            Ok(take)
        }
    }

    /// One backend message: tag, length including itself, body.
    fn message(tag: u8, body: &[u8]) -> Vec<u8> {
        let len = i32::try_from(body.len() + 4).expect("a test message fits in an i32");
        let mut out = vec![tag];
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(body);
        out
    }

    /// An `Authentication*` message, which is one tag with a code inside it.
    fn auth(code: i32, rest: &[u8]) -> Vec<u8> {
        let mut body = code.to_be_bytes().to_vec();
        body.extend_from_slice(rest);
        message(b'R', &body)
    }

    /// `BackendKeyData`, `ParameterStatus` and `ReadyForQuery` — what a real
    /// server sends between `AuthenticationOk` and the first statement.
    fn startup_tail() -> Vec<u8> {
        let mut key = 4242i32.to_be_bytes().to_vec();
        key.extend_from_slice(&99i32.to_be_bytes());

        let mut out = message(b'K', &key);
        out.extend_from_slice(&message(b'S', b"client_encoding\0UTF8\0"));
        // The one parameter this driver keeps, carrying the suffix a
        // distribution puts in it: a driver that kept the number alone would
        // pass a case whose fixture answered `16.4`.
        out.extend_from_slice(&message(
            b'S',
            format!("server_version\0{SERVER_VERSION}\0").as_bytes(),
        ));
        out.extend_from_slice(&message(b'Z', b"I"));
        out
    }

    /// What the scripted server reports itself as.
    const SERVER_VERSION: &str = "16.4 (Debian 16.4-1.pgdg120+1)";

    fn hmac(key: &[u8], data: &[u8]) -> [u8; 32] {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC takes any key length");
        mac.update(data);
        mac.finalize().into_bytes().into()
    }

    /// RFC 5802's `Hi`, at the one iteration this fake uses: a real server's
    /// 4096 would only make the test slower, since the client applies whatever
    /// count it is told.
    fn salted(password: &[u8], salt: &[u8]) -> [u8; 32] {
        let mut seed = salt.to_vec();
        seed.extend_from_slice(&1u32.to_be_bytes());
        hmac(password, &seed)
    }

    const SALT: &[u8] = b"novis-scram-salt";
    const SERVER_NONCE: &str = "serverhalfofthenonce";

    /// The server half of SCRAM-SHA-256, computed rather than canned.
    ///
    /// It verifies the client's proof — so a driver that got the auth message
    /// or the key derivation wrong is refused here rather than waved through —
    /// and signs its own final message, which is the half the driver verifies.
    struct Scram {
        password: &'static str,
        client_first_bare: String,
        server_first: String,
    }

    impl Scram {
        fn new(password: &'static str) -> Scram {
            Scram {
                password,
                client_first_bare: String::new(),
                server_first: String::new(),
            }
        }

        /// `SASLInitialResponse` in, `AuthenticationSASLContinue` out.
        fn first(&mut self, sent: &[u8]) -> Vec<u8> {
            // 'p', length, mechanism cstring, i32 length, the SCRAM message.
            let body = &sent[5..];
            let nul = body.iter().position(|b| *b == 0).expect("a mechanism name");
            assert_eq!(
                &body[..nul],
                super::SCRAM_SHA_256.as_bytes(),
                "the driver named a mechanism the server never offered"
            );
            let client_first = std::str::from_utf8(&body[nul + 5..]).expect("UTF-8");
            assert!(
                client_first.starts_with("n,,"),
                "channel binding must be `unsupported`, or a TLS server reads `y,,` as a downgrade"
            );

            self.client_first_bare = client_first["n,,".len()..].to_owned();
            // `n=<user>,r=<nonce>`, split on the attribute's own delimiter and
            // not on a bare `r=`. RFC 5802's nonce is printable ASCII with the
            // comma excluded — `postgres_protocol` draws 24 of those — so it
            // may contain `r=` and cannot contain `,r=`. Splitting on the
            // shorter needle takes a *suffix* of the nonce whenever one holds
            // `r=`, so the server echoes a prefix the client never sent and the
            // driver refuses its own exchange.
            let client_nonce = self
                .client_first_bare
                .split_once(",r=")
                .expect("a client nonce")
                .1
                .to_owned();
            self.server_first = format!(
                "r={client_nonce}{SERVER_NONCE},s={},i=1",
                STANDARD.encode(SALT)
            );
            auth(11, self.server_first.as_bytes())
        }

        /// `SASLResponse` in; the proof is checked and the exchange finished.
        fn last(&mut self, sent: &[u8]) -> Vec<u8> {
            let client_final = std::str::from_utf8(&sent[5..]).expect("UTF-8");
            let (without_proof, proof) = client_final.rsplit_once(",p=").expect("a client proof");
            let proof = STANDARD.decode(proof).expect("base64");

            let auth_message = format!(
                "{},{},{}",
                self.client_first_bare, self.server_first, without_proof
            );
            let salted = salted(self.password.as_bytes(), SALT);
            let client_key = hmac(&salted, b"Client Key");
            let stored_key: [u8; 32] = Sha256::digest(client_key).into();
            let client_signature = hmac(&stored_key, auth_message.as_bytes());

            let mut recovered = [0u8; 32];
            for (slot, (p, s)) in recovered
                .iter_mut()
                .zip(proof.iter().zip(client_signature.iter()))
            {
                *slot = p ^ s;
            }
            if <[u8; 32]>::from(Sha256::digest(recovered)) != stored_key {
                return message(b'E', b"SFATAL\0C28P01\0Mthe proof did not verify\0\0");
            }

            let server_key = hmac(&salted, b"Server Key");
            let signature = hmac(&server_key, auth_message.as_bytes());

            let mut out = auth(12, format!("v={}", STANDARD.encode(signature)).as_bytes());
            out.extend_from_slice(&auth(0, b""));
            out.extend_from_slice(&startup_tail());
            out
        }
    }

    /// The fake server echoes the client's **whole** nonce, including one that
    /// contains `r=`.
    ///
    /// This pins the fixture rather than the driver, and it is here because a
    /// fixture bug here is indistinguishable from a driver bug at the point it
    /// shows: `authenticate` refuses a `server-first` whose nonce does not begin
    /// with the one it sent, so a fake that echoes a suffix fails inside
    /// whichever SCRAM case the run happens to reach. `postgres_protocol` draws
    /// 24 characters from RFC 5802's comma-free printable set, so `r=` lands in
    /// a nonce about one run in 368 — often enough to fail an acceptance check,
    /// rarely enough to pass every re-run of it.
    #[test]
    fn the_fake_server_echoes_a_client_nonce_that_contains_the_attribute_marker() {
        const NONCE: &str = "abcr=defr=ghi";
        let mut sent = vec![b'p', 0, 0, 0, 0];
        sent.extend_from_slice(super::SCRAM_SHA_256.as_bytes());
        sent.push(0);
        sent.extend_from_slice(&0i32.to_be_bytes());
        sent.extend_from_slice(format!("n,,n=,r={NONCE}").as_bytes());

        let mut scram = Scram::new("Novis-Test-Pw1");
        scram.first(&sent);
        assert!(
            scram
                .server_first
                .starts_with(&format!("r={NONCE}{SERVER_NONCE},")),
            "the server echoed something other than the whole client nonce: {}",
            scram.server_first
        );
    }

    fn target<'a>(password: &'a str) -> PgTarget<'a> {
        PgTarget {
            host: "postgres.test",
            user: "novis",
            password,
            database: "novis_test",
            // These cases drive the exchange over a recorded peer rather than a
            // socket, so no handshake runs and the anchors are never consulted.
            tls_ca_file: None,
            // Deliberately not UTC: a zone this driver sends is visible in the
            // startup message only when it is not the default of every field
            // around it.
            time_zone: 2 * 3600,
            statement_cache: DEFAULT_STATEMENT_CACHE,
        }
    }

    /// A `[db.<name>]` block with every field a PostgreSQL connection reads,
    /// as an operator writes it and as `nvs_config` hands it over.
    fn block() -> Database {
        Database {
            driver: Some("postgres".to_owned()),
            host: Some("postgres.test".to_owned()),
            user: Some("novis".to_owned()),
            password: Some("hunter2".to_owned()),
            database: Some("novis_test".to_owned()),
            time_zone: Some("+02:00".to_owned()),
            ..Database::default()
        }
    }

    /// `rule:core-classes/db-unix-socket-path`: a `host` naming a **directory**
    /// becomes the socket PostgreSQL's own engine binds inside it,
    /// `<directory>/.s.PGSQL.<port>`, which is what libpq, `psql` and PDO take.
    ///
    /// Asserted against **where the dial landed** rather than against a
    /// completed handshake: a unit test has no PostgreSQL server, and the
    /// derivation is what is under test, so a listener bound at the derived
    /// name that accepts once and hangs up answers that question and nothing
    /// else. A driver that dialled the directory, or that hardcoded 5432, would
    /// find nothing bound and the accept would never happen — which is why the
    /// port here is not the default one.
    #[test]
    fn a_postgres_socket_directory_becomes_the_engines_own_name() {
        #[cfg(unix)]
        {
            let (directory, _) = nvs_repo::socket(".s.PGSQL.5433");
            let written = directory.to_str().expect("a scratch path is UTF-8");

            let super::Endpoint::Socket(derived) = super::socket_endpoint(written, 5433)
                .expect("a build with `AF_UNIX` derives rather than refuses")
            else {
                panic!("a directory is a socket endpoint and never a TCP one");
            };
            assert_eq!(
                derived,
                directory.join(".s.PGSQL.5433"),
                "the engine's own name is the directory plus the port, and the \
                 port is part of the name rather than of an address"
            );

            let listener =
                std::os::unix::net::UnixListener::bind(&derived).expect("the OS refused the path");
            let server = std::thread::spawn(move || listener.accept().is_ok());

            let opened = PgConn::connect(
                super::Endpoint::Socket(derived.clone()),
                &target("hunter2"),
                None,
            );

            assert!(
                server.join().expect("the fake server runs to completion"),
                "the driver did not dial the name derived from the directory"
            );
            assert!(
                opened.is_err(),
                "a server that hung up before the startup answer is not a connection"
            );
        }

        #[cfg(not(unix))]
        {
            let refused = super::socket_endpoint("/var/run/postgresql", 5433)
                .expect_err("a build with no `AF_UNIX` transport has no socket to derive");
            assert_eq!(refused.kind(), io::ErrorKind::Unsupported);
        }
    }

    /// A complete block resolves to exactly what the handshake sends, and the
    /// two fields with readers of their own are asserted through them: an
    /// unwritten `statement_cache` is § 1's default and a written `0` is the
    /// cache off, which is the answer a `unwrap_or_default` reader loses.
    #[test]
    fn a_complete_block_resolves_to_the_target_the_handshake_sends() {
        let mut block = block();
        let target = PgTarget::resolve(&block).expect("a complete block resolves");
        assert_eq!(target.host, "postgres.test");
        assert_eq!(target.user, "novis");
        assert_eq!(target.password, "hunter2");
        assert_eq!(target.database, "novis_test");
        assert_eq!(target.time_zone, 2 * 3600);
        assert_eq!(target.statement_cache, DEFAULT_STATEMENT_CACHE);

        block.statement_cache = Some(0);
        let sized = PgTarget::resolve(&block).expect("a block that turns the cache off resolves");
        assert_eq!(sized.statement_cache, 0);
    }

    /// One row of the sweep below: a block's key, and how a test writes that
    /// key's value.
    type Field = (&'static str, fn(&mut Database, Option<String>));

    /// Every field the startup exchange sends, refused by its own key when it
    /// is absent and again when it is written empty — asserted by sweeping the
    /// roster and counting, because a resolver that named one field in every
    /// message still answers plausibly on any single line.
    #[test]
    fn every_field_the_handshake_needs_is_refused_by_its_own_name() {
        let fields: [Field; 4] = [
            ("host", |block, value| block.host = value),
            ("user", |block, value| block.user = value),
            ("password", |block, value| block.password = value),
            ("database", |block, value| block.database = value),
        ];

        let mut swept = 0;
        for (field, write) in fields {
            let mut absent = block();
            write(&mut absent, None);
            assert_eq!(
                PgTarget::resolve(&absent).unwrap_err(),
                BlockError::Missing {
                    field,
                    expected: Driver::Postgres
                }
            );

            let mut empty = block();
            write(&mut empty, Some(String::new()));
            assert_eq!(
                PgTarget::resolve(&empty).unwrap_err(),
                BlockError::Blank { field }
            );
            swept += 1;
        }
        assert_eq!(swept, fields.len());
    }

    /// § 3's `password_file`, on both sides. `nvs_config::secret` materializes
    /// the file's content into `password` and *leaves `password_file` set*, so
    /// a resolver refusing on the field's presence would refuse every
    /// deployment that keeps its credential out of the config file — and one
    /// reading `password` alone would report a missing password to an operator
    /// who wrote one.
    #[test]
    fn a_password_file_is_a_password_once_it_has_been_read_and_a_refusal_before() {
        let mut block = block();
        block.password_file = Some("/run/secrets/db-password".to_owned());
        assert_eq!(
            PgTarget::resolve(&block)
                .expect("a materialized secret resolves")
                .password,
            "hunter2"
        );

        block.password = None;
        assert_eq!(
            PgTarget::resolve(&block).unwrap_err(),
            BlockError::SecretUnread
        );
    }

    /// § 9's zone, on both sides of the reader's own answer: an unwritten field
    /// is UTC and a value that is not an offset refuses the block instead of
    /// being folded into UTC, which would run a deployment two hours out on a
    /// typo. The message names the block and quotes what was written, since
    /// the operator's next act is to find that line.
    #[test]
    fn a_zone_that_is_not_an_offset_refuses_the_block_rather_than_resolving_to_utc() {
        let mut block = block();
        block.time_zone = None;
        assert_eq!(
            PgTarget::resolve(&block)
                .expect("an unwritten zone is UTC")
                .time_zone,
            0
        );

        block.time_zone = Some("Europe/Vienna".to_owned());
        let refused = PgTarget::resolve(&block).unwrap_err();
        assert_eq!(
            refused,
            BlockError::TimeZone {
                written: "Europe/Vienna"
            }
        );
        let message = refused.refusal("main");
        assert!(message.starts_with("[db.main]: "), "{message}");
        assert!(message.contains("Europe/Vienna"), "{message}");
    }

    /// The discriminant, and what a block belonging to another driver does
    /// here: it is refused rather than opened as PostgreSQL, whether the name
    /// is another driver's or none at all. The case a silent resolver
    /// loses is `path` — a valid field on the struct, and one this handshake
    /// has nothing to do with, so ignoring it would open a *server*
    /// connection for a block that named a file.
    #[test]
    fn a_block_that_is_not_postgresqls_is_refused_rather_than_opened() {
        let mut block = block();
        block.driver = None;
        assert_eq!(PgTarget::resolve(&block).unwrap_err(), BlockError::NoDriver);

        block.driver = Some("mysql".to_owned());
        assert_eq!(
            PgTarget::resolve(&block).unwrap_err(),
            BlockError::OtherDriver {
                written: "mysql",
                driver: Driver::MySql,
                expected: Driver::Postgres
            }
        );

        block.driver = Some("pgsql".to_owned());
        assert_eq!(
            PgTarget::resolve(&block).unwrap_err(),
            BlockError::UnknownDriver { written: "pgsql" }
        );

        // A file is written by a human, so the capital is a spelling and not
        // another backend.
        block.driver = Some("Postgres".to_owned());
        assert!(PgTarget::resolve(&block).is_ok());

        block.path = Some("app.db".to_owned());
        assert_eq!(
            PgTarget::resolve(&block).unwrap_err(),
            BlockError::Unusable {
                field: "path",
                expected: Driver::Postgres
            }
        );
    }

    /// § 9's declared zone reaches the server as PostgreSQL's own numeric
    /// spelling, and the sign is the whole trap.
    ///
    /// A POSIX zone counts westwards, so two hours *east* of UTC is `-02`
    /// after the abbreviation — while the abbreviation itself carries the sign
    /// a reader expects, which is why the two halves of every one of these
    /// disagree. A driver that sent `+02:00` instead would not be sending a
    /// zone the server can parse at all, and the connection would fail at
    /// startup rather than read an hour wrong.
    #[test]
    fn the_declared_zone_is_sent_as_postgresqls_own_numeric_spelling() {
        assert_eq!(posix_time_zone(0), "<+00>-00");
        assert_eq!(posix_time_zone(2 * 3600), "<+02>-02");
        assert_eq!(posix_time_zone(-5 * 3600), "<-05>+05");
        assert_eq!(posix_time_zone(5 * 3600 + 45 * 60), "<+05:45>-05:45");
        assert_eq!(
            posix_time_zone(-(3 * 3600 + 30 * 60 + 15)),
            "<-03:30:15>+03:30:15"
        );
    }

    /// The upgrade request is exactly `rule:security/one-tls-client`'s eight bytes, and `S` is
    /// the answer that continues. Asserted on the bytes rather than on the
    /// outcome, because those eight are the only plaintext a connection ever
    /// carries and a wrong request code reads as a plain startup message.
    #[test]
    fn an_ssl_request_is_the_first_thing_on_the_wire_and_s_is_the_yes() {
        let mut peer = Peer::new(|_: &[u8]| b"S".to_vec());
        request_tls(&mut peer).expect("S is the server agreeing");
        assert_eq!(peer.sent, vec![vec![0, 0, 0, 8, 0x04, 0xd2, 0x16, 0x2f]]);
    }

    /// The refusal `rule:core-classes/db-capabilities` is written for: a server with no TLS is not
    /// silently used in the clear, which is what `sslmode=prefer` does.
    #[test]
    fn a_server_that_offers_no_tls_is_refused_rather_than_used_in_the_clear() {
        for answer in [&b"N"[..], &b"E"[..]] {
            let mut peer = Peer::new(move |_: &[u8]| answer.to_vec());
            let refused = request_tls(&mut peer).expect_err("a plaintext connection was accepted");
            assert_eq!(refused.kind(), io::ErrorKind::ConnectionRefused);
        }

        // And a byte that is no answer at all is a protocol error, not a
        // silent fallthrough to either branch.
        let mut peer = Peer::new(|_: &[u8]| b"?".to_vec());
        assert_eq!(
            request_tls(&mut peer)
                .expect_err("garbage was accepted")
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    /// The version a connection keeps is the `server_version` its startup
    /// reported, verbatim and bought with no statement —
    /// [ADR 0187 § 2](/docs/decisions/0187.md).
    ///
    /// The fixture's value carries a distribution's suffix, so a driver that
    /// parsed the number out and rebuilt the string fails here; the peer
    /// panics on a fourth request, so a driver that asked the server instead
    /// fails too. The second half is the startup that reported no version at
    /// all, which is refused rather than answered with an empty string.
    #[test]
    fn postgres_keeps_server_version_from_its_parameter_status() {
        let mut scram = Scram::new("Novis-Test-Pw1");
        let mut step = 0;
        let mut wire = Wire::new(Peer::new(move |sent: &[u8]| {
            step += 1;
            match step {
                1 => auth(10, b"SCRAM-SHA-256\0SCRAM-SHA-256-PLUS\0\0"),
                2 => scram.first(sent),
                3 => scram.last(sent),
                _ => panic!("the driver asked the server for something startup had said"),
            }
        }));

        let startup =
            authenticate(&mut wire, &target("Novis-Test-Pw1")).expect("the exchange completed");
        assert_eq!(startup.server_version, SERVER_VERSION);

        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut key = 4242i32.to_be_bytes().to_vec();
            key.extend_from_slice(&99i32.to_be_bytes());
            let mut out = auth(0, b"");
            out.extend_from_slice(&message(b'K', &key));
            out.extend_from_slice(&message(b'Z', b"I"));
            out
        }));
        let refused = authenticate(&mut wire, &target("Novis-Test-Pw1"))
            .expect_err("a startup that reported no version opened a connection");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert!(refused.to_string().contains("server_version"), "{refused}");
    }

    /// The whole exchange, against a server that checks what it is sent: the
    /// driver's proof verifies, the driver verifies the server's signature,
    /// and the cancellation key survives into the connection.
    #[test]
    fn scram_sha_256_authenticates_and_the_cancellation_key_survives_startup() {
        let mut scram = Scram::new("Novis-Test-Pw1");
        let mut step = 0;
        let mut wire = Wire::new(Peer::new(move |sent: &[u8]| {
            step += 1;
            match step {
                1 => auth(10, b"SCRAM-SHA-256\0SCRAM-SHA-256-PLUS\0\0"),
                2 => scram.first(sent),
                _ => scram.last(sent),
            }
        }));

        let startup =
            authenticate(&mut wire, &target("Novis-Test-Pw1")).expect("the exchange completed");

        assert_eq!(
            startup.cancel,
            CancelKey {
                process_id: 4242,
                secret_key: 99
            }
        );
        // § 9's rendering parameters are pinned in the startup message: a text
        // rendering this driver parses positionally has to be the rendering it
        // asked for, not the one the server was configured with. The charset is
        // [`the_connection_charset_is_forced_to_utf8`]'s.
        let startup = &wire.peer().sent[0];
        assert!(
            startup.windows(14).any(|w| w == b"DateStyle\0ISO\0"),
            "the startup message did not pin the date rendering"
        );
        assert!(
            startup.windows(18).any(|w| w == b"TimeZone\0<+02>-02\0"),
            "the startup message did not declare the connection's zone"
        );
    }

    /// The startup message's parameter list, as the pairs it carries.
    ///
    /// Four bytes of length, four of protocol version, then `key\0value\0`
    /// until an empty key ends the list. Parsed rather than searched for,
    /// because a `windows` match cannot tell `UTF8` from a value that merely
    /// starts with it, and cannot see a second setting of the same key later
    /// in the list — which is exactly how a forced parameter stops being
    /// forced.
    fn startup_parameters(startup: &[u8]) -> Vec<(String, String)> {
        let mut fields = startup[8..]
            .split(|byte| *byte == 0)
            .map(|field| String::from_utf8(field.to_vec()).expect("a startup parameter is text"));
        let mut pairs = Vec::new();
        while let Some(key) = fields.next().filter(|key| !key.is_empty()) {
            pairs.push((key, fields.next().expect("a key carries a value")));
        }
        pairs
    }

    /// `rule:core-classes/db-column-types`'s "connection charset forces UTF-8", which is two claims
    /// and needs both: the connection asks for UTF-8 before it can be sent
    /// anything, and a text body is checked anyway.
    ///
    /// The parameter is set once, in the startup message, so there is no
    /// window in which a row could arrive in the server's own encoding — and
    /// nothing an operator writes in a `[db.<name>]` block reaches it, which is
    /// why the value is asserted rather than merely its presence.
    ///
    /// The check on the way back is not belt and braces. The module doc's
    /// § *Parameters and results are in text format* owns why: a server is a
    /// network peer, `rule:types/bytes`'s guarantee is read unchecked downstream, and
    /// `client_encoding` is a request rather than a proof. It is asserted over
    /// **every** OID that reads back as text — § 9's text rows, its JSON ones
    /// and the "no Novis type" row every unnamed OID falls to — by counting, so
    /// an OID added later that decodes text without the check fails here rather
    /// than passing on the rows it shares with these.
    #[test]
    fn the_connection_charset_is_forced_to_utf8() {
        let mut scram = Scram::new("Novis-Test-Pw1");
        let mut step = 0;
        let mut wire = Wire::new(Peer::new(move |sent: &[u8]| {
            step += 1;
            match step {
                1 => auth(10, b"SCRAM-SHA-256\0SCRAM-SHA-256-PLUS\0\0"),
                2 => scram.first(sent),
                _ => scram.last(sent),
            }
        }));
        authenticate(&mut wire, &target("Novis-Test-Pw1")).expect("the exchange completed");

        let parameters = startup_parameters(&wire.peer().sent[0]);
        let charsets: Vec<&str> = parameters
            .iter()
            .filter(|(key, _)| key == "client_encoding")
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(
            charsets,
            vec!["UTF8"],
            "the connection did not ask for UTF-8 exactly once, in {parameters:?}"
        );

        // Latin-1 `été`, which is what a server configured with another
        // encoding sends for a column of every type below.
        const LATIN1: &[u8] = &[0xE9, 0x74, 0xE9];
        let text_oids = [
            oid::CHAR,
            oid::NAME,
            oid::TEXT,
            oid::BPCHAR,
            oid::VARCHAR,
            oid::JSON,
            oid::JSONB,
            // § 9's last row: `inet`, and with it every OID this driver has no
            // arm for.
            869,
        ];
        let mut refused = 0usize;
        for type_oid in text_oids {
            let column = column(type_oid, -1);
            assert_eq!(
                rendered(
                    &column
                        .scalar(Some("été".as_bytes()))
                        .expect("UTF-8 decoded")
                ),
                "text été",
                "OID {type_oid} did not read a well-formed body back as it was sent"
            );
            let refusal = column
                .scalar(Some(LATIN1))
                .expect_err("a body in the server's own encoding became a string");
            assert!(
                refusal.to_string().contains("well-formed UTF-8"),
                "OID {type_oid} refused a Latin-1 body for another reason: {refusal}"
            );
            refused += 1;
        }
        assert_eq!(refused, text_oids.len());

        // The other side of the bound: § 9's binary rows have no text form at
        // all, so the same bytes are a value there rather than an error.
        assert_eq!(
            rendered(
                &column(oid::BYTEA, -1)
                    .scalar(Some(b"\\x0102"))
                    .expect("bytes decoded")
            ),
            "bytes [1, 2]"
        );
    }

    /// A wrong password fails at the server's check rather than anywhere
    /// softer, and the server's own wording reaches the caller.
    #[test]
    fn a_password_the_server_cannot_verify_is_the_servers_own_error() {
        let mut scram = Scram::new("Novis-Test-Pw1");
        let mut step = 0;
        let mut wire = Wire::new(Peer::new(move |sent: &[u8]| {
            step += 1;
            match step {
                1 => auth(10, b"SCRAM-SHA-256\0\0"),
                2 => scram.first(sent),
                _ => scram.last(sent),
            }
        }));

        let refused = authenticate(&mut wire, &target("not-the-password"))
            .expect_err("a wrong password authenticated");
        let said = refused.to_string();
        assert!(said.contains("28P01"), "{said}");
        assert!(said.contains("FATAL"), "{said}");
    }

    /// The two exchanges that put a password, or a replayable hash of one, on
    /// the wire are refused rather than answered — and the refusal names the
    /// server setting that fixes it.
    #[test]
    fn a_cleartext_or_md5_password_request_is_refused_rather_than_answered() {
        for request in [auth(3, b""), auth(5, b"salt")] {
            let mut wire = Wire::new(Peer::new(move |_: &[u8]| request.clone()));
            let refused = authenticate(&mut wire, &target("Novis-Test-Pw1"))
                .expect_err("the driver answered a weak authentication request");

            assert_eq!(refused.kind(), io::ErrorKind::ConnectionRefused);
            assert!(refused.to_string().contains("scram-sha-256"), "{refused}");
            // Nothing after the startup message left: the password was never
            // encoded at all, in any form.
            assert_eq!(wire.peer().sent.len(), 1);
        }
    }

    /// A server offering only mechanisms this driver does not implement is
    /// refused naming what it offered, which is the difference between a
    /// five-minute fix and an afternoon.
    #[test]
    fn a_server_with_no_scram_mechanism_is_refused_naming_what_it_offered() {
        let mut wire = Wire::new(Peer::new(|_: &[u8]| auth(10, b"GSSAPI\0SCRAM-SHA-1\0\0")));
        let refused = authenticate(&mut wire, &target("Novis-Test-Pw1"))
            .expect_err("an unknown mechanism was accepted");

        let said = refused.to_string();
        assert!(said.contains("GSSAPI"), "{said}");
        assert!(said.contains("SCRAM-SHA-1"), "{said}");
    }

    /// `ReadyForQuery` without a `BackendKeyData` is refused: the key arrives
    /// once and cannot be asked for again, so a connection that reached idle
    /// without one could never have a statement cancelled.
    #[test]
    fn a_startup_that_carried_no_cancellation_key_is_refused() {
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut out = auth(0, b"");
            out.extend_from_slice(&message(b'Z', b"I"));
            out
        }));
        let refused = authenticate(&mut wire, &target("Novis-Test-Pw1"))
            .expect_err("a connection with no cancellation key was opened");
        assert!(refused.to_string().contains("BackendKeyData"), "{refused}");
    }

    /// A one-column `RowDescription`, in text format because that is what the
    /// driver's `Bind` asked for.
    fn row_description(name: &[u8], type_oid: u32) -> Vec<u8> {
        let mut body = 1i16.to_be_bytes().to_vec();
        body.extend_from_slice(name);
        body.push(0);
        body.extend_from_slice(&0u32.to_be_bytes()); // table OID
        body.extend_from_slice(&0i16.to_be_bytes()); // column ID
        body.extend_from_slice(&type_oid.to_be_bytes());
        body.extend_from_slice(&(-1i16).to_be_bytes()); // type size
        body.extend_from_slice(&(-1i32).to_be_bytes()); // type modifier
        body.extend_from_slice(&0i16.to_be_bytes()); // text format
        message(b'T', &body)
    }

    /// A `DataRow`, `None` being a column the server sent as SQL `NULL`.
    fn data_row(columns: &[Option<&[u8]>]) -> Vec<u8> {
        let count = i16::try_from(columns.len()).expect("a test row has few columns");
        let mut body = count.to_be_bytes().to_vec();
        for column in columns {
            match column {
                Some(bytes) => {
                    let len = i32::try_from(bytes.len()).expect("a test value is short");
                    body.extend_from_slice(&len.to_be_bytes());
                    body.extend_from_slice(bytes);
                }
                None => body.extend_from_slice(&(-1i32).to_be_bytes()),
            }
        }
        message(b'D', &body)
    }

    /// An `ErrorResponse` that also names the constraint the condition broke,
    /// which is field `n` and the one § 8 field a `SQLSTATE` cannot imply.
    fn constrained_error_response(code: &str, said: &str, constraint: &str) -> Vec<u8> {
        let mut body = vec![b'S'];
        body.extend_from_slice(b"ERROR\0");
        body.push(b'C');
        body.extend_from_slice(code.as_bytes());
        body.push(0);
        body.push(b'M');
        body.extend_from_slice(said.as_bytes());
        body.push(0);
        body.push(b'n');
        body.extend_from_slice(constraint.as_bytes());
        body.push(0);
        body.push(0);
        message(b'E', &body)
    }

    /// An `ErrorResponse` carrying the fields [`super::server_error`] reads,
    /// and the trailing zero that ends the field list.
    fn error_response(code: &str, said: &str) -> Vec<u8> {
        let mut body = vec![b'S'];
        body.extend_from_slice(b"ERROR\0");
        body.push(b'C');
        body.extend_from_slice(code.as_bytes());
        body.push(0);
        body.push(b'M');
        body.extend_from_slice(said.as_bytes());
        body.push(0);
        body.push(0);
        message(b'E', &body)
    }

    /// An `ErrorResponse` carrying whatever further fields the case names, for
    /// the § 8 rule that turns on the fields [`super::server_error`] *drops*
    /// rather than on the ones it keeps.
    fn error_response_with(code: &str, said: &str, extra: &[(u8, &str)]) -> Vec<u8> {
        let mut body = vec![b'S'];
        body.extend_from_slice(b"ERROR\0");
        body.push(b'C');
        body.extend_from_slice(code.as_bytes());
        body.push(0);
        body.push(b'M');
        body.extend_from_slice(said.as_bytes());
        body.push(0);
        for (field, value) in extra {
            body.push(*field);
            body.extend_from_slice(value.as_bytes());
            body.push(0);
        }
        body.push(0);
        message(b'E', &body)
    }

    /// The tag of every message in one flushed group, in order.
    ///
    /// Walks the length prefixes rather than searching for bytes: a tag letter
    /// also occurs inside the SQL and inside a parameter, so a `contains` here
    /// would pass on a message that was never sent.
    fn tags(flushed: &[u8]) -> Vec<u8> {
        let mut tags = Vec::new();
        let mut at = 0;
        while at + 5 <= flushed.len() {
            tags.push(flushed[at]);
            let len = u32::from_be_bytes(
                flushed[at + 1..at + 5]
                    .try_into()
                    .expect("four bytes are four bytes"),
            );
            at += usize::try_from(len).expect("a test message fits in a usize") + 1;
        }
        tags
    }

    /// A server that answers one statement with one `text` column, `rows`, a
    /// completion and a `ReadyForQuery` — the whole of the ordinary path.
    fn one_statement(rows: Vec<Vec<u8>>) -> Vec<u8> {
        let mut out = message(b'1', b""); // ParseComplete
        out.extend_from_slice(&message(b'2', b"")); // BindComplete
        out.extend_from_slice(&row_description(b"greeting", 25));
        for row in rows {
            out.extend_from_slice(&row);
        }
        out.extend_from_slice(&message(b'C', b"SELECT 2\0"));
        out.extend_from_slice(&message(b'Z', b"I"));
        out
    }

    /// The same shape, answering only the messages the batch actually carried:
    /// a `CloseComplete` for an eviction and a `ParseComplete` for a parse.
    ///
    /// A server does not answer a `Parse` that was never sent, so a cache hit
    /// the driver got wrong would stall here rather than pass quietly.
    fn statement_answer(closed: bool, parsed: bool) -> Vec<u8> {
        let mut out = Vec::new();
        if closed {
            out.extend_from_slice(&message(b'3', b"")); // CloseComplete
        }
        if parsed {
            out.extend_from_slice(&message(b'1', b"")); // ParseComplete
        }
        out.extend_from_slice(&message(b'2', b"")); // BindComplete
        out.extend_from_slice(&row_description(b"greeting", 25));
        out.extend_from_slice(&message(b'C', b"SELECT 0\0"));
        out.extend_from_slice(&message(b'Z', b"I"));
        out
    }

    /// `rule:core-classes/db-one-api`'s cache on the wire: the round trip PostgreSQL stops paying
    /// on the second execution is the `Parse` that is no longer in the batch.
    #[test]
    fn a_cached_statement_is_parsed_once_and_bound_by_name_after() {
        let state = Cell::new(State::Idle);
        let mut cache = StatementCache::new(2);
        let mut answered = 0usize;
        let mut wire = Wire::new(Peer::new(move |_: &[u8]| {
            answered += 1;
            statement_answer(false, answered == 1)
        }));

        for _ in 0..2 {
            let mut rows = start_statement(&mut wire, &state, &mut cache, "select greeting", &[])
                .expect("the portal described itself");
            while rows.next_row().expect("the stream drained").is_some() {}
        }

        assert_eq!(tags(&wire.peer().sent[0]), b"PBDES".to_vec());
        assert_eq!(
            tags(&wire.peer().sent[1]),
            b"BDES".to_vec(),
            "the second execution parsed a statement the server was already holding"
        );
        assert_eq!(cache.len(), 1);
    }

    /// The parked form of a stream, which is the whole point of splitting the
    /// read state off the borrow: the cursor is a local of its own, and every
    /// row is read through a **fresh** borrow of the wire and the state.
    ///
    /// The compiler is half the assertion. A `PgRows` cannot express this shape
    /// at all — its borrow would have to span the calls between the rows — and
    /// that is exactly what a `Core\Db\Connection` holding a cursor across
    /// `advance()` calls needs, because the connection goes back to the request
    /// in between. The other half is the run: the same two rows arrive, in
    /// order, and the stream still drains to `ReadyForQuery`, so the connection
    /// this was read off is poolable at the end of it.
    #[test]
    fn a_parked_cursor_reads_every_row_through_a_fresh_borrow() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            one_statement(vec![
                data_row(&[Some(b"first")]),
                data_row(&[Some(b"second")]),
            ])
        }));

        let mut reading = open_portal(&mut wire, &state, &mut no_cache(), "select greeting", &[])
            .expect("the portal described itself");
        assert_eq!(state.get(), State::Streaming);
        assert_eq!(reading.columns.len(), 1);

        let mut seen = Vec::new();
        while let Some(row) =
            next_row_of(&mut wire, &state, &mut reading).expect("the stream advanced")
        {
            seen.push(
                row.column(0)
                    .expect("a column this row has")
                    .expect("a value that is not null")
                    .to_vec(),
            );
        }

        assert_eq!(seen, vec![b"first".to_vec(), b"second".to_vec()]);
        assert_eq!(
            state.get(),
            State::Idle,
            "a drained stream left the connection unpoolable"
        );
        assert_eq!(reading.tag.as_deref(), Some("SELECT 2"));
    }

    /// Every `Bind` in one flush, as the pair of names it carries: the portal
    /// first, then the prepared statement.
    ///
    /// The tag-only assertions above cannot see this, and neither can any test
    /// running on [`no_cache`]: a disabled cache makes both names the same
    /// empty string, so a `Bind` naming them the other way round is a `PBDES`
    /// that looks exactly right.
    fn binds(flushed: &[u8]) -> Vec<(String, String)> {
        let mut found = Vec::new();
        let mut at = 0;
        while at + 5 <= flushed.len() {
            let len = usize::try_from(u32::from_be_bytes(
                flushed[at + 1..at + 5]
                    .try_into()
                    .expect("four bytes are four bytes"),
            ))
            .expect("a test message fits in a usize");
            if flushed[at] == b'B' {
                let mut names = flushed[at + 5..at + 1 + len].splitn(3, |byte| *byte == 0);
                let portal = names.next().expect("a Bind carries a portal name");
                let statement = names.next().expect("a Bind carries a statement name");
                found.push((
                    String::from_utf8(portal.to_vec()).expect("a name is text"),
                    String::from_utf8(statement.to_vec()).expect("a name is text"),
                ));
            }
            at += len + 1;
        }
        found
    }

    /// The `Bind` names the **statement** the `Parse` created and leaves the
    /// portal unnamed, which is the order the message carries the two in.
    ///
    /// Naming them the other way round binds the unnamed statement, which is
    /// parsed only when the cache is disabled — so with `rule:core-classes/db-one-api`'s default
    /// capacity every connection's first statement draws SQLSTATE 26000, while
    /// every test on [`no_cache`] passes.
    #[test]
    fn a_statement_is_bound_by_name_in_the_unnamed_portal() {
        let state = Cell::new(State::Idle);
        let mut cache = StatementCache::new(2);
        let mut answered = 0usize;
        let mut wire = Wire::new(Peer::new(move |_: &[u8]| {
            answered += 1;
            statement_answer(false, answered == 1)
        }));

        for _ in 0..2 {
            let mut rows = start_statement(&mut wire, &state, &mut cache, "select greeting", &[])
                .expect("the portal described itself");
            while rows.next_row().expect("the stream drained").is_some() {}
        }

        for flush in &wire.peer().sent {
            assert_eq!(
                binds(flush),
                vec![(String::new(), "s0".to_string())],
                "a bind did not name the statement the parse created"
            );
        }
    }

    /// § 4's batch binds the same way, and asserted over every execution: one
    /// `Parse` shared by N `Bind`s is a saving only if all N name it.
    #[test]
    fn an_execute_many_binds_the_named_statement_in_the_unnamed_portal() {
        let state = Cell::new(State::Idle);
        let mut cache = StatementCache::new(2);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            many_answer(&[Some("INSERT 0 1"), Some("INSERT 0 1"), Some("INSERT 0 1")])
        }));

        execute_many(
            &mut wire,
            &state,
            &mut cache,
            "insert into t values ($1)",
            &[&[Some(b"a")], &[Some(b"b")], &[Some(b"c")]],
        )
        .expect("the batch ran");

        assert_eq!(
            binds(&wire.peer().sent[0]),
            vec![(String::new(), "s0".to_string()); 3],
            "the batch's binds did not all name the one parsed statement"
        );
    }

    /// The tag of every message in one flushed group whose body carries
    /// `needle`, in order — "where did the value end up", read off the wire.
    ///
    /// Walks the length prefixes for the same reason [`tags`] does. A `B` in
    /// the answer is the value arriving as a `Bind` parameter, which is the
    /// only place it belongs; a `P` is a `Parse` whose SQL text carries it and
    /// a `Q` is the simple-query protocol carrying it as a whole statement,
    /// which are the two shapes an emulated prepare takes.
    fn carriers(flushed: &[u8], needle: &[u8]) -> Vec<u8> {
        let mut found = Vec::new();
        let mut at = 0;
        while at + 5 <= flushed.len() {
            let len = usize::try_from(u32::from_be_bytes(
                flushed[at + 1..at + 5]
                    .try_into()
                    .expect("four bytes are four bytes"),
            ))
            .expect("a test message fits in a usize");
            if flushed[at + 5..at + 1 + len]
                .windows(needle.len())
                .any(|window| window == needle)
            {
                found.push(flushed[at]);
            }
            at += len + 1;
        }
        found
    }

    /// `rule:core-classes/db-one-api`'s "emulated prepares do not exist in any form", asserted
    /// over **every** way a value reaches the wire rather than one path at a
    /// time: a path added later fails here without anyone remembering to come
    /// back, which is the whole point of sweeping.
    ///
    /// The value is catastrophic if it is ever interpolated — it closes a
    /// quoted literal, ends the statement and comments out what followed — so
    /// this does not have to model what an escaper would have done with it.
    /// Either it reaches the server inside a `Bind` or the driver has
    /// reintroduced the thing § 1 removes.
    ///
    /// Counted per path rather than read off one flush: a path that sent the
    /// value nowhere at all would satisfy "never in statement text" and fail
    /// its own count, and both cache states are swept because the parse and
    /// the bind are separate flushes only on the miss.
    #[test]
    fn no_driver_path_interpolates_a_value_into_sql() {
        const HOSTILE: &[u8] = b"'); drop table users; --";

        let mut paths: Vec<(&str, usize, Vec<Vec<u8>>)> = Vec::new();

        // `query`, over a live cache: the first execution parses and the
        // second binds a statement the server is already holding.
        {
            let state = Cell::new(State::Idle);
            let mut cache = StatementCache::new(2);
            let mut answered = 0usize;
            let mut wire = Wire::new(Peer::new(move |_: &[u8]| {
                answered += 1;
                statement_answer(false, answered == 1)
            }));
            for _ in 0..2 {
                let mut rows =
                    start_statement(&mut wire, &state, &mut cache, "select $1", &[Some(HOSTILE)])
                        .expect("the portal described itself");
                while rows.next_row().expect("the stream drained").is_some() {}
            }
            paths.push(("query, cached", 2, wire.peer().sent.clone()));
        }

        // The same statement with the cache disabled, which is the unnamed
        // prepare — a different branch of `start_statement` entirely.
        {
            let state = Cell::new(State::Idle);
            let mut wire = Wire::new(Peer::new(|_: &[u8]| one_statement(Vec::new())));
            {
                let mut rows = start_statement(
                    &mut wire,
                    &state,
                    &mut no_cache(),
                    "select $1",
                    &[Some(HOSTILE)],
                )
                .expect("the portal described itself");
                while rows.next_row().expect("the stream drained").is_some() {}
            }
            paths.push(("query, uncached", 1, wire.peer().sent.clone()));
        }

        // § 4's batch, on both cache states for the same reason.
        for (path, mut cache) in [
            ("executeMany, cached", StatementCache::new(2)),
            ("executeMany, uncached", no_cache()),
        ] {
            let state = Cell::new(State::Idle);
            let mut wire = Wire::new(Peer::new(|_: &[u8]| {
                many_answer(&[Some("INSERT 0 1"), Some("INSERT 0 1"), Some("INSERT 0 1")])
            }));
            execute_many(
                &mut wire,
                &state,
                &mut cache,
                "insert into t values ($1)",
                &[&[Some(HOSTILE)], &[Some(HOSTILE)], &[Some(HOSTILE)]],
            )
            .expect("the batch ran");
            paths.push((path, 3, wire.peer().sent.clone()));
        }

        for (path, executions, flushes) in &paths {
            let mut bound = 0usize;
            for flush in flushes {
                for tag in carriers(flush, HOSTILE) {
                    assert_eq!(
                        char::from(tag),
                        'B',
                        "{path} put the value in a {} message, so it reached the server as \
                         statement text",
                        char::from(tag)
                    );
                    bound += 1;
                }
            }
            assert_eq!(
                bound, *executions,
                "{path} bound the value {bound} times for {executions} execution(s)"
            );
        }
    }

    /// The eviction's `Close` rides in the batch that replaced it, so making
    /// room costs no round trip of its own — the batch's `Sync` bounds both.
    #[test]
    fn an_evicted_statement_is_closed_in_the_batch_that_replaced_it() {
        let state = Cell::new(State::Idle);
        let mut cache = StatementCache::new(1);
        let mut answered = 0usize;
        let mut wire = Wire::new(Peer::new(move |_: &[u8]| {
            answered += 1;
            statement_answer(answered > 1, true)
        }));

        for sql in ["select 'a'", "select 'b'"] {
            let mut rows = start_statement(&mut wire, &state, &mut cache, sql, &[])
                .expect("the portal described itself");
            while rows.next_row().expect("the stream drained").is_some() {}
        }

        assert_eq!(wire.peer().sent.len(), 2, "the close was its own flush");
        assert_eq!(tags(&wire.peer().sent[0]), b"PBDES".to_vec());
        assert_eq!(tags(&wire.peer().sent[1]), b"CPBDES".to_vec());
        assert_eq!(cache.len(), 1, "the cache outgrew its capacity");
    }

    /// A statement the server refused leaves the cache exactly as it was: the
    /// name was minted but never parsed, and binding it later would draw a
    /// `26000` on a connection that was otherwise fine.
    #[test]
    fn a_statement_the_server_would_not_parse_is_not_left_in_the_cache() {
        let state = Cell::new(State::Idle);
        let mut cache = StatementCache::new(2);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut out = error_response("42601", "syntax error at or near \"slect\"");
            out.extend_from_slice(&message(b'Z', b"I"));
            out
        }));

        start_statement(&mut wire, &state, &mut cache, "slect 1", &[])
            .expect_err("a statement that never parsed was accepted");

        assert!(
            cache.is_empty(),
            "the server is not holding what was cached"
        );
        assert_eq!(state.get(), State::Idle);
    }

    /// `rule:core-classes/db-one-api`'s "PostgreSQL's extended protocol pays nothing extra",
    /// asserted as bytes: all five messages go out in **one** flush, so there is
    /// no prepare round trip to save. Asserted by walking the length prefixes
    /// because every tag letter also occurs inside the SQL.
    #[test]
    fn a_statement_is_parse_bind_describe_execute_and_sync_in_one_flush() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| one_statement(Vec::new())));

        {
            let mut rows = start_statement(
                &mut wire,
                &state,
                &mut no_cache(),
                "select $1",
                &[Some(b"7")],
            )
            .expect("the portal described itself");
            while rows.next_row().expect("the stream drained").is_some() {}
        }

        assert_eq!(wire.peer().sent.len(), 1, "the statement was not one flush");
        assert_eq!(tags(&wire.peer().sent[0]), b"PBDES".to_vec());
    }

    /// A server answering one `executeMany`: a `ParseComplete` for the single
    /// parse, then a `BindComplete`, a completion and a `ReadyForQuery` for
    /// every execution — one boundary per `Sync`, which is what the driver
    /// counts. `None` is an execution the server refused instead.
    fn many_answer(completions: &[Option<&str>]) -> Vec<u8> {
        let mut out = message(b'1', b""); // ParseComplete
        for completion in completions {
            match completion {
                Some(tag) => {
                    out.extend_from_slice(&message(b'2', b"")); // BindComplete
                    let mut body = tag.as_bytes().to_vec();
                    body.push(0);
                    out.extend_from_slice(&message(b'C', &body));
                }
                None => out.extend_from_slice(&error_response(
                    "23505",
                    "duplicate key value violates unique constraint",
                )),
            }
            out.extend_from_slice(&message(b'Z', b"I"));
        }
        out
    }

    /// `rule:core-classes/db-statement-members`'s `executeMany` on the wire: **one** `Parse` for N executions,
    /// all of it in one flush, and the counts summed. The `Sync` per execution
    /// is the section's "no transaction of its own" — one for the whole batch
    /// would make a failure at the last set roll back every set before it.
    #[test]
    fn an_execute_many_is_one_parse_and_a_bind_execute_sync_per_set() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            many_answer(&[Some("INSERT 0 1"), Some("INSERT 0 2"), Some("INSERT 0 3")])
        }));

        let affected = execute_many(
            &mut wire,
            &state,
            &mut no_cache(),
            "insert into t values ($1)",
            &[&[Some(b"a")], &[Some(b"b")], &[Some(b"c")]],
        )
        .expect("the batch ran");

        assert_eq!(affected, 6, "the affected counts were not summed");
        assert_eq!(wire.peer().sent.len(), 1, "the batch was not one flush");
        assert_eq!(tags(&wire.peer().sent[0]), b"PBESBESBES".to_vec());
        assert_eq!(state.get(), State::Idle);
        assert!(state.get().is_poolable());
    }

    /// § 4's "an empty set list is a no-op returning `0`", and the half worth
    /// pinning is that nothing reaches the wire: there is no statement to
    /// prepare for executions that are not going to happen.
    #[test]
    fn an_empty_execute_many_writes_nothing_and_answers_zero() {
        let state = Cell::new(State::Idle);
        let mut cache = StatementCache::new(2);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            panic!("an empty batch reached the server")
        }));

        let affected = execute_many(
            &mut wire,
            &state,
            &mut cache,
            "insert into t values ($1)",
            &[],
        )
        .expect("the no-op ran");

        assert_eq!(affected, 0);
        assert!(wire.peer().sent.is_empty(), "the no-op was not a no-op");
        assert!(cache.is_empty(), "an unexecuted statement was prepared");
        assert_eq!(state.get(), State::Idle);
    }

    /// One prepare has one parameter count, which is also what § 1's cache is
    /// keyed on: a set list that disagrees with itself is refused before
    /// anything is written, rather than drawing the server's bind error on
    /// whichever set was odd.
    #[test]
    fn an_execute_many_whose_sets_bind_different_arities_is_refused_unsent() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| panic!("a refused batch was sent")));

        let refused = execute_many(
            &mut wire,
            &state,
            &mut no_cache(),
            "insert into t values ($1)",
            &[&[Some(b"a")], &[Some(b"b"), Some(b"c")]],
        )
        .expect_err("a ragged set list was accepted");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert!(wire.peer().sent.is_empty());
        assert_eq!(state.get(), State::Idle);
    }

    /// An execution the server refused ends its own `Sync` and no more: the
    /// driver still reads the boundary of every execution after it, so the
    /// connection comes back idle rather than holding answers the next
    /// statement would read as its own. The first error is the reported one,
    /// and the statement is not cached — a `Parse` is undone with the implicit
    /// transaction it ran in.
    #[test]
    fn an_execute_many_that_one_set_failed_reads_every_sync_and_reports_the_first() {
        let state = Cell::new(State::Idle);
        let mut cache = StatementCache::new(2);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            many_answer(&[Some("INSERT 0 1"), None, None])
        }));

        let refused = execute_many(
            &mut wire,
            &state,
            &mut cache,
            "insert into t values ($1)",
            &[&[Some(b"a")], &[Some(b"b")], &[Some(b"c")]],
        )
        .expect_err("a batch with a refused execution was reported as success");

        assert!(
            refused.to_string().contains("23505"),
            "the reported error was not the first one: {refused}"
        );
        assert!(cache.is_empty(), "a rolled-back parse was cached");
        assert_eq!(state.get(), State::Idle);
        assert!(state.get().is_poolable());
    }

    /// The values of `rule:core-classes/db-connection-busy-state`, walked by one statement: `Idle` before,
    /// `Streaming` while rows are unread, and `Idle` again once the
    /// `ReadyForQuery` the `Sync` guaranteed has been read. A `NULL` column is
    /// `None` and not an empty slice — the distinction `rule:core-classes/db-column-types` maps to
    /// `?T`.
    #[test]
    fn rows_stream_until_command_complete_and_the_connection_returns_to_idle() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            one_statement(vec![
                data_row(&[Some(b"hello")]),
                data_row(&[None]),
                data_row(&[Some(b"")]),
            ])
        }));

        let mut rows = start_statement(&mut wire, &state, &mut no_cache(), "select greeting", &[])
            .expect("the portal opened");
        assert_eq!(state.get(), State::Streaming);
        assert_eq!(rows.columns().len(), 1);
        assert_eq!(rows.columns()[0].name, "greeting");
        assert_eq!(rows.columns()[0].type_oid, 25);
        assert_eq!(rows.command_tag(), None);

        let mut seen = Vec::new();
        while let Some(row) = rows.next_row().expect("the stream drained") {
            seen.push(row.column(0).expect("column 0 exists").map(<[u8]>::to_vec));
            // Still streaming until the completion is read, whatever has
            // arrived so far.
            assert_eq!(state.get(), State::Streaming);
        }

        assert_eq!(
            seen,
            vec![Some(b"hello".to_vec()), None, Some(Vec::new())],
            "a NULL column and an empty one are not the same answer"
        );
        assert_eq!(rows.command_tag(), Some("SELECT 2"));
        assert_eq!(state.get(), State::Idle);
        assert!(state.get().is_poolable());
    }

    /// One row's body, wide enough that a result of them is megabytes.
    ///
    /// The width is the point rather than the content: the bound below only
    /// means something against a volume that dwarfs it, and it is the *server*
    /// that decides that volume.
    const WIDE_ROW: &[u8] = b"a row body wide enough that the volume, and not the row count, is what this case is about";

    /// A server that answers a result of any size **without holding it**.
    ///
    /// This is the half [`Peer`] cannot do: `Peer` keeps every byte it ever
    /// answered in `inbound` and every byte it was ever sent in `sent`, so a
    /// large result read over it would measure the test rather than the driver.
    /// A bound on a stream has to hold on both sides of it — the server that
    /// generates the rows and the driver that reads them are on one thread and
    /// on one heap, so a harness that buffers fails the measurement exactly as
    /// a driver that buffers would.
    ///
    /// One message is generated at a time, into the buffer the last one used.
    /// What the driver writes is dropped: this case asserts nothing about what
    /// was sent, and the cases above own that question.
    struct Firehose {
        /// How many `DataRow`s this result has.
        rows: usize,
        /// How many messages have been generated, the prologue included.
        emitted: usize,
        /// The message being read, and never more than one.
        out: Vec<u8>,
        /// How much of that message has been handed over.
        read: usize,
        /// Every byte ever generated — the volume the bound is stated against.
        served: usize,
    }

    impl Firehose {
        fn new(rows: usize) -> Firehose {
            Firehose {
                rows,
                emitted: 0,
                out: Vec::new(),
                read: 0,
                served: 0,
            }
        }

        /// Generates the next message over the last one, answering `false` once
        /// the result has ended — which is the peer having nothing left to
        /// send, not a closed connection.
        fn refill(&mut self) -> bool {
            self.out.clear();
            self.read = 0;
            let at = self.emitted;
            self.emitted += 1;
            if at == 0 {
                self.out.extend_from_slice(&message(b'1', b"")); // ParseComplete
                self.out.extend_from_slice(&message(b'2', b"")); // BindComplete
                self.out
                    .extend_from_slice(&row_description(b"greeting", 25));
            } else if at <= self.rows {
                self.out.extend_from_slice(&data_row(&[Some(WIDE_ROW)]));
            } else if at == self.rows + 1 {
                let tag = format!("SELECT {}\0", self.rows);
                self.out.extend_from_slice(&message(b'C', tag.as_bytes()));
                self.out.extend_from_slice(&message(b'Z', b"I"));
            } else {
                return false;
            }
            self.served += self.out.len();
            true
        }
    }

    impl Write for Firehose {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Read for Firehose {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.read == self.out.len() && !self.refill() {
                return Ok(0);
            }
            let left = &self.out[self.read..];
            let take = left.len().min(buf.len());
            buf[..take].copy_from_slice(&left[..take]);
            self.read += take;
            Ok(take)
        }
    }

    /// What one drained result cost, as [`drained`] read it.
    struct Drain {
        /// How many rows arrived.
        rows: usize,
        /// How many bytes the server generated for them.
        served: usize,
        /// The most the thread's live heap ever stood above where the stream
        /// opened — the number the bound is on.
        peak: isize,
        /// How many bytes the drain asked the allocator for, in total. A stream
        /// that is working spends this and gives it back, so it is the reading
        /// that separates a flat `peak` from a measurement that never ran —
        /// `nvs_runtime::budget`'s own module doc makes that distinction.
        churn: usize,
    }

    /// Drains a result of `rows` rows off a [`Firehose`].
    ///
    /// The counters are `nvs_runtime::budget`'s, which every profile maintains
    /// because the memory limit is read off them; this binary installs no
    /// allocator of its own and could not, since `nvs-runtime` registers one in
    /// every `not(test)` build and it is linked here as a dependency.
    fn drained(rows: usize) -> Drain {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Firehose::new(rows));
        let mut result =
            start_statement(&mut wire, &state, &mut no_cache(), "select greeting", &[])
                .expect("the portal opened");

        // Read with the portal already open and the description already
        // decoded: the fixed cost of having *a* stream is not what is being
        // bounded, the cost of having a long one is.
        let floor = nvs_runtime::budget::live_bytes();
        let spent = nvs_runtime::budget::allocated_bytes();
        let mut peak = 0;
        let mut seen = 0;
        while let Some(row) = result.next_row().expect("the stream drained") {
            // A row that is read is a row that is really there — a driver
            // answering `None` early would otherwise pass this on a flat heap.
            assert_eq!(row.column(0).expect("column 0 exists"), Some(WIDE_ROW));
            seen += 1;
            peak = peak.max(nvs_runtime::budget::live_bytes() - floor);
        }
        let churn = nvs_runtime::budget::allocated_bytes() - spent;

        let tag = format!("SELECT {rows}");
        assert_eq!(result.command_tag(), Some(tag.as_str()));
        assert_eq!(state.get(), State::Idle);
        drop(result);
        Drain {
            rows: seen,
            served: wire.peer().served,
            peak,
            churn,
        }
    }

    /// `rule:core-classes/db-statement-members`'s one member that does not buffer, asserted as a **memory
    /// bound** rather than as a row count.
    ///
    /// A row count is what a buffering driver passes: it hands back every row,
    /// in order, having read them all first. What separates streaming from that
    /// is the heap while the rows are going past, so that is what is read here
    /// — [`PgRows::next_row`] reads one message and answers with a row that
    /// borrows the inbox, and the inbox is bounded by `READ_CHUNK` and the
    /// widest message in it.
    ///
    /// Measured at two sizes an order of magnitude apart so that the bound is
    /// visibly not a function of the row count. Both readings come out far
    /// under it, because the inbox is already allocated by the time the stream
    /// opens and streaming reuses it; the bound is left at several
    /// `READ_CHUNK`s anyway, since what it is written to catch is a driver
    /// holding megabytes and not one holding a buffer more.
    #[test]
    fn a_large_result_streams_at_constant_memory() {
        /// Ten times `SMALL`, so a driver that keeps a row it has already
        /// handed back fails this by an order of magnitude and not by a margin.
        const LARGE: usize = 100_000;
        const SMALL: usize = 10_000;
        /// Several times the 16 KiB `READ_CHUNK` the inbox grows by, and a tiny
        /// fraction of the volume the assertion below requires to have gone
        /// past. Between those two it does not need to be tight.
        const BOUND: isize = 128 * 1024;

        let small = drained(SMALL);
        let large = drained(LARGE);

        assert_eq!(
            (small.rows, large.rows),
            (SMALL, LARGE),
            "every row of both results arrived"
        );
        assert!(
            isize::try_from(large.served).expect("a test volume fits in an isize") > 40 * BOUND,
            "the server generated only {} bytes, which is too close to the {BOUND}-byte bound for \
             holding all of it to fail this test",
            large.served
        );

        // The two readings the bound is made of, and they have to disagree:
        // what the drain *spends* grows with the result, and what it *holds*
        // does not. A stream that never ran would have both flat and would pass
        // the second assertion alone.
        assert!(
            large.churn >= 5 * small.churn,
            "{} rows asked the allocator for {} bytes against {} for {} rows, which is not the \
             per-row cost a real drain has: the counters are not seeing this run",
            LARGE,
            large.churn,
            small.churn,
            SMALL
        );
        assert!(
            small.peak <= BOUND && large.peak <= BOUND,
            "a stream of {SMALL} rows peaked at {} bytes and one of {LARGE} rows at {}, against a \
             bound of {BOUND}: the rows are being buffered rather than streamed",
            small.peak,
            large.peak
        );
        assert!(
            (large.peak - small.peak).abs() <= BOUND / 2,
            "ten times the rows moved the peak from {} to {} bytes, so what the driver holds is a \
             function of the result's size",
            small.peak,
            large.peak
        );
    }

    /// `rule:core-classes/db-statement-members`'s refusal, and the half of it that matters is that
    /// **nothing reaches the wire**: writing a second statement over an
    /// unfinished one is the shape § 4 exists to prevent, not merely one it
    /// reports.
    #[test]
    fn a_second_statement_on_a_busy_connection_writes_nothing_and_is_refused() {
        for busy in [State::Executing, State::Streaming, State::Poisoned] {
            let state = Cell::new(busy);
            let mut wire = Wire::new(Peer::new(|_: &[u8]| Vec::new()));

            let refused = start_statement(&mut wire, &state, &mut no_cache(), "select 1", &[])
                .expect_err("a second statement was accepted");

            assert_eq!(refused.kind(), io::ErrorKind::InvalidInput, "{busy:?}");
            assert!(wire.peer().sent.is_empty(), "{busy:?} reached the wire");
            assert_eq!(state.get(), busy, "{busy:?} was changed by a refusal");
        }
    }

    /// `rule:core-classes/db-statement-members`'s refusal is a **`LogicError`** — a mistake in the program
    /// rather than a failure of the connection — and this is the half of that
    /// the driver decides.
    ///
    /// `nvs-stdlib` words the throw, because the fault class and the call
    /// site's spelling are its to know ([`State::may_start_statement`]'s own
    /// doc says so). What has to be true *here* is what makes that class the
    /// right one: the refusal is decided by one predicate rather than by a busy
    /// test each driver grew, and it is recoverable by fixing the program —
    /// read the rows and the very same statement runs on the very same wire,
    /// with nothing reconnected. The case above pins that nothing was *sent*;
    /// this one pins that nothing was *spent*.
    ///
    /// [`State::Poisoned`] is the other side of that: it refuses the same
    /// statement and is deliberately not the same answer, since a wire that is
    /// not at a message boundary is closed rather than reused (§ 13).
    #[test]
    fn a_second_statement_on_a_busy_connection_is_a_logic_error() {
        // One predicate and one permitted state. A variant added later has to
        // be added here too, which is the point: a driver that answered this
        // question for itself could disagree with the class the caller catches.
        let states = [
            State::Idle,
            State::Executing,
            State::Streaming,
            State::Poisoned,
        ];
        let permitted: Vec<State> = states
            .into_iter()
            .filter(|state| state.may_start_statement())
            .collect();
        assert_eq!(
            permitted,
            vec![State::Idle],
            "§ 4 permits a statement on an idle connection and on no other"
        );

        // The busy states that are *healthy*. `Poisoned` refuses for its
        // own reason and is below.
        for busy in [State::Executing, State::Streaming] {
            let state = Cell::new(busy);
            let mut wire = Wire::new(Peer::new(|_: &[u8]| {
                one_statement(vec![data_row(&[Some(b"hello")])])
            }));

            let refused =
                start_statement(&mut wire, &state, &mut no_cache(), "select greeting", &[])
                    .expect_err("a second statement was accepted");
            assert_eq!(refused.kind(), io::ErrorKind::InvalidInput, "{busy:?}");
            assert_eq!(
                state.get(),
                busy,
                "{busy:?}: the refusal moved the connection it refused on"
            );

            // The caller reads its rows — or abandons them, which drains the
            // same way — and then re-runs what was refused. That it needs no
            // reconnect is the whole difference between § 4's refusal and a
            // wire failure, and it is why the class is `LogicError`.
            state.set(State::Idle);
            let mut rows =
                start_statement(&mut wire, &state, &mut no_cache(), "select greeting", &[])
                    .expect("the connection was still usable after the refusal");
            assert!(
                rows.next_row().expect("the stream drained").is_some(),
                "{busy:?}: the re-run statement returned no rows"
            );
            drop(rows);

            assert_eq!(state.get(), State::Idle, "{busy:?}");
            assert_eq!(
                wire.peer().sent.len(),
                1,
                "{busy:?}: the refusal spent a round trip of its own"
            );
        }

        assert!(
            !State::Poisoned.may_start_statement() && !State::Poisoned.is_poolable(),
            "a poisoned connection refuses a statement and is closed rather than reused"
        );
        assert!(
            State::Idle.is_poolable(),
            "the state a refused caller recovers to is the one the pool takes back"
        );
    }

    /// § 4 does not only require the refusal — it requires it to name **both**
    /// fixes, `->all()` *or* a `{shared: false}` connection, because they fix
    /// different programs. Asserted for every entry point that can refuse, so a
    /// member added later with a message of its own fails here rather than
    /// shipping half the answer.
    #[test]
    fn a_refused_statement_names_both_of_the_fixes_section_4_gives() {
        let state = Cell::new(State::Streaming);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| Vec::new()));

        let refusals = [
            start_statement(&mut wire, &state, &mut no_cache(), "select 1", &[])
                .expect_err("a second statement was accepted")
                .to_string(),
            execute_many(
                &mut wire,
                &state,
                &mut no_cache(),
                "insert into t values ($1)",
                &[&[Some(b"a")]],
            )
            .expect_err("a second batch was accepted")
            .to_string(),
        ];

        for said in refusals {
            assert!(
                said.contains("->all()"),
                "the buffered read is unnamed: {said}"
            );
            assert!(
                said.contains("{shared: false}"),
                "the dedicated connection is unnamed: {said}"
            );
        }
    }

    /// `rule:core-classes/db-connection-busy-state`'s per-driver call, in PostgreSQL's favour: a caller that
    /// stops reading leaves the connection `Idle` and poolable, because the
    /// `Sync` that ends the statement was already on the wire before the first
    /// row arrived.
    #[test]
    fn an_abandoned_result_set_is_drained_back_to_idle_rather_than_poisoned() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            one_statement(vec![data_row(&[Some(b"one")]), data_row(&[Some(b"two")])])
        }));

        {
            let mut rows =
                start_statement(&mut wire, &state, &mut no_cache(), "select greeting", &[])
                    .expect("the portal opened");
            assert!(rows.next_row().expect("the first row arrived").is_some());
            assert_eq!(state.get(), State::Streaming);
        }

        assert_eq!(state.get(), State::Idle);
        assert!(state.get().is_poolable());
    }

    /// A statement the server refuses at `Parse` is the server's own error, and
    /// the connection is still poolable afterwards: the `Sync` means a
    /// `ReadyForQuery` follows the refusal, so there is a boundary to resume
    /// from and § 4's poison would be wrong here.
    #[test]
    fn a_statement_the_server_refuses_leaves_the_connection_idle_and_poolable() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut out = error_response("42601", "syntax error at or near \"slect\"");
            out.extend_from_slice(&message(b'Z', b"I"));
            out
        }));

        let refused = start_statement(&mut wire, &state, &mut no_cache(), "slect 1", &[])
            .expect_err("a syntax error was accepted");

        let said = refused.to_string();
        assert!(said.contains("42601"), "{said}");
        assert!(said.contains("syntax error"), "{said}");
        assert_eq!(state.get(), State::Idle);
        assert!(state.get().is_poolable());
    }

    /// The same rule after rows have already arrived — a constraint violation
    /// part way through a stream. The error reaches the caller and the
    /// connection is still clean.
    #[test]
    fn an_error_part_way_through_a_stream_ends_it_without_poisoning() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut out = message(b'1', b"");
            out.extend_from_slice(&message(b'2', b""));
            out.extend_from_slice(&row_description(b"greeting", 25));
            out.extend_from_slice(&data_row(&[Some(b"one")]));
            out.extend_from_slice(&error_response("22012", "division by zero"));
            out.extend_from_slice(&message(b'Z', b"I"));
            out
        }));

        {
            let mut rows = start_statement(&mut wire, &state, &mut no_cache(), "select 1/x", &[])
                .expect("the portal opened");
            assert!(rows.next_row().expect("the first row arrived").is_some());
            let failed = rows.next_row().expect_err("the error was swallowed");
            assert!(failed.to_string().contains("22012"), "{failed}");
            // And the stream is over: a second call answers `None` rather than
            // reading into the next statement's messages.
            assert!(rows.next_row().expect("the stream is over").is_none());
        }

        assert_eq!(state.get(), State::Idle);
    }

    /// The one case that *is* poison: a peer that stops mid-message leaves no
    /// boundary to resume from, so § 4 closes the connection rather than
    /// resetting it. Asserted on `is_poolable` and not only on the state,
    /// because that is the security-relevant half.
    #[test]
    fn a_peer_that_closes_mid_statement_poisons_the_connection() {
        let state = Cell::new(State::Idle);
        // ParseComplete, then the head of a message that never finishes.
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut out = message(b'1', b"");
            out.extend_from_slice(&[b'T', 0, 0, 0, 40]);
            out
        }));

        let failed = start_statement(&mut wire, &state, &mut no_cache(), "select 1", &[])
            .expect_err("a truncated message was accepted");

        assert_eq!(failed.kind(), io::ErrorKind::UnexpectedEof);
        assert_eq!(state.get(), State::Poisoned);
        assert!(!state.get().is_poolable());
    }

    /// A well-framed message the extended-query sequence does not put in a
    /// result stream is poison too, and for the reason [`super::out_of_sequence`]
    /// gives: the frame decoded, but the peer is not tracking the sequence this
    /// driver is, so draining would trust what has just proven wrong.
    #[test]
    fn a_message_out_of_sequence_poisons_rather_than_draining() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut out = message(b'1', b"");
            // `CopyInResponse` — legal PostgreSQL, and not something a `Bind`
            // and `Execute` of a prepared statement can produce.
            out.extend_from_slice(&message(b'G', &[0, 0, 0]));
            out.extend_from_slice(&message(b'Z', b"I"));
            out
        }));

        let failed = start_statement(&mut wire, &state, &mut no_cache(), "copy t from stdin", &[])
            .expect_err("an out-of-sequence message was accepted");

        assert_eq!(failed.kind(), io::ErrorKind::InvalidData);
        assert_eq!(state.get(), State::Poisoned);
        assert!(!state.get().is_poolable());
    }

    /// The SQL of every simple `Query` in one flushed group, in order.
    fn queries(flushed: &[u8]) -> Vec<String> {
        let mut out = Vec::new();
        let mut at = 0;
        while at + 5 <= flushed.len() {
            let len = usize::try_from(u32::from_be_bytes(
                flushed[at + 1..at + 5]
                    .try_into()
                    .expect("four bytes are four bytes"),
            ))
            .expect("a test message fits in a usize");
            if flushed[at] == b'Q' {
                let body = &flushed[at + 5..at + 1 + len];
                let sql = body.strip_suffix(b"\0").unwrap_or(body);
                out.push(String::from_utf8_lossy(sql).into_owned());
            }
            at += len + 1;
        }
        out
    }

    /// `ReadyForQuery`, which every simple `Query` ends with whatever happened.
    fn ready() -> Vec<u8> {
        message(b'Z', b"I")
    }

    /// `rule:security/db-pool-reset-is-a-boundary`'s list, in its order, in **one** flush — six commands for
    /// one round trip, which is what makes an unconditional reset affordable.
    /// The two exclusions are asserted by name because they are the section's
    /// own: `DISCARD ALL` would run `DEALLOCATE ALL` and throw away the
    /// statement cache the pool exists to preserve.
    #[test]
    fn a_reset_is_section_13s_six_commands_in_one_flush_and_not_discard_all() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| ready().repeat(6)));

        super::reset_session(&mut wire, &state).expect("the reset ran");

        assert_eq!(
            wire.peer().sent.len(),
            1,
            "the reset was not one round trip"
        );
        let sent = queries(&wire.peer().sent[0]);
        assert_eq!(
            sent,
            vec![
                "ROLLBACK",
                "RESET ALL",
                "CLOSE ALL",
                "UNLISTEN *",
                "SELECT pg_advisory_unlock_all()",
                "DISCARD TEMP",
            ]
        );
        for command in &sent {
            assert!(!command.contains("DISCARD ALL"), "{command}");
            assert!(!command.contains("DEALLOCATE"), "{command}");
        }
        assert_eq!(state.get(), State::Idle);
        assert!(state.get().is_poolable());
    }

    /// § 13's exclusion read from the other side: not what the reset's flush
    /// says, but what the flush *after* it costs.
    ///
    /// The case above asserts the command list, which only implies the
    /// property; this one asserts the property itself — a statement parsed
    /// before the reset is still the server's after it, so the batch after the
    /// reset carries no `Parse`. `reset_session` takes no cache and touches
    /// none, so a driver that started deallocating would fail here first. The
    /// scripted server answers only what each batch actually asked for, so a
    /// re-parse would stall on a `ParseComplete` that never comes rather than
    /// pass quietly.
    #[test]
    fn postgres_resets_without_losing_its_statement_cache() {
        let state = Cell::new(State::Idle);
        let mut cache = StatementCache::new(2);
        let mut flushed = 0usize;
        let mut wire = Wire::new(Peer::new(move |_: &[u8]| {
            flushed += 1;
            match flushed {
                1 => statement_answer(false, true),
                2 => ready().repeat(super::RESET_COMMANDS.len()),
                _ => statement_answer(false, false),
            }
        }));

        {
            let mut rows = start_statement(&mut wire, &state, &mut cache, "select greeting", &[])
                .expect("the portal described itself");
            while rows.next_row().expect("the stream drained").is_some() {}
        }

        super::reset_session(&mut wire, &state).expect("the reset ran");

        {
            let mut rows = start_statement(&mut wire, &state, &mut cache, "select greeting", &[])
                .expect("the portal described itself");
            while rows.next_row().expect("the stream drained").is_some() {}
        }

        assert_eq!(tags(&wire.peer().sent[0]), b"PBDES".to_vec());
        for command in queries(&wire.peer().sent[1]) {
            assert!(!command.contains("DEALLOCATE"), "{command}");
            assert!(!command.contains("DISCARD ALL"), "{command}");
        }
        assert_eq!(
            tags(&wire.peer().sent[2]),
            b"BDES".to_vec(),
            "the execution after the reset re-parsed a statement the server still holds"
        );
        assert_eq!(cache.len(), 1, "the reset emptied the cache");
    }

    /// § 13 states what a reset must remove as a **property list**, not as a
    /// command list, because "a backend added later satisfies that property or
    /// is not pooled". This sweeps the list.
    ///
    /// The case above compares the flush against six literals, which fails
    /// loudly but says only that two lists differ. Here a property whose
    /// command went missing fails **by name**, and the second loop closes the
    /// other direction: a command in the reset that no § 13 property asks for
    /// is a round trip nothing justifies.
    #[test]
    fn no_session_state_survives_a_return_to_the_pool() {
        /// Each of § 13's properties against the command that removes it. Two
        /// share `RESET ALL`, which is why this is a sweep and not a zip.
        const REMOVED: [(&str, &str); 7] = [
            ("an open transaction", "ROLLBACK"),
            ("a session variable", "RESET ALL"),
            ("a `SET ROLE`", "RESET ALL"),
            ("an open cursor", "CLOSE ALL"),
            ("a listener", "UNLISTEN *"),
            ("an advisory lock", "SELECT pg_advisory_unlock_all()"),
            ("a temporary table", "DISCARD TEMP"),
        ];

        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            ready().repeat(super::RESET_COMMANDS.len())
        }));

        super::reset_session(&mut wire, &state).expect("the reset ran");

        let sent = queries(&wire.peer().sent[0]);
        for (property, command) in REMOVED {
            assert!(
                sent.iter().any(|c| c.as_str() == command),
                "{property} survives the reset: no command in it removes one"
            );
        }
        for command in &sent {
            assert!(
                REMOVED.iter().any(|(_, c)| *c == command.as_str()),
                "`{command}` is in the reset and no § 13 property asks for it"
            );
        }
        // The one property the list *keeps* — "no prepared statement the cache
        // does not still account for" — is asserted by
        // `postgres_resets_without_losing_its_statement_cache` above.
        assert_eq!(sent.len(), super::RESET_COMMANDS.len());
    }

    /// A refused command fails the whole reset, the **first** refusal is the one
    /// reported, and every later command is still read to its boundary — which
    /// the second error's code proves, since reaching it means the batch was not
    /// abandoned at the first.
    #[test]
    fn a_refused_reset_reports_the_first_error_and_still_reads_every_boundary() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut out = ready(); // ROLLBACK
            out.extend_from_slice(&error_response("42501", "permission denied"));
            out.extend_from_slice(&ready()); // RESET ALL, refused
            out.extend_from_slice(&ready().repeat(3));
            out.extend_from_slice(&error_response("55000", "object not in prerequisite state"));
            out.extend_from_slice(&ready()); // DISCARD TEMP, refused
            out
        }));

        let failed = super::reset_session(&mut wire, &state).expect_err("a refused reset passed");

        let said = failed.to_string();
        assert!(said.contains("42501"), "the later error won: {said}");
        assert!(said.contains("permission denied"), "{said}");
        // The wire is still at a boundary — that is what six `ReadyForQuery`
        // means — so § 13 destroys this connection through `PgConn::reset`
        // taking `self`, not by claiming the stream is unreadable.
        assert_eq!(state.get(), State::Idle);
    }

    /// `rule:security/db-pool-reset-is-a-boundary`'s "a connection that cannot be proven clean is closed": a
    /// refused reset hands the caller an error and **no connection**, so there
    /// is nothing left that could rejoin the pool carrying one request's
    /// session state into the next one's.
    ///
    /// Two halves, because the enforcement is a type and the destruction is a
    /// write.
    ///
    /// The type: `PgConn::reset` takes `self` **by value** and names a `PgConn`
    /// only inside its `Ok`, so the coercion below is the whole assertion — a
    /// `&mut self` reset, or one handing the connection back beside the error,
    /// does not compile against this signature. The caller of a failed reset
    /// therefore has nothing to return, rather than a connection it must
    /// remember not to reuse.
    ///
    /// The write: what dropping it does is [`super::say_goodbye`], driven here
    /// over the same refused batch, and the assertion is that the connection's
    /// last flush is one `Terminate` and that no seventh command went out after
    /// the six. A driver that answered a failed reset by carrying on would
    /// write one here.
    ///
    /// The two are asserted apart because `PgConn`'s `wire` is a
    /// `Wire<PgStream>` — no unit test can build one, so no unit test can
    /// call `reset` and watch the real drop. `nvs_stdlib`'s `warm_connection`
    /// is where they meet in a running program: `postgres.reset().ok()`, whose
    /// `None` is both an empty pool and a connection that has just been closed.
    #[test]
    fn a_failed_reset_destroys_the_connection_rather_than_returning_it() {
        let _: fn(PgConn) -> io::Result<PgConn> = PgConn::reset;

        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut out = ready(); // ROLLBACK
            out.extend_from_slice(&error_response("42501", "permission denied"));
            out.extend_from_slice(&ready()); // RESET ALL, refused
            out.extend_from_slice(&ready().repeat(4));
            out
        }));

        super::reset_session(&mut wire, &state).expect_err("a refused reset passed");
        // The `?` in `reset` is reached, which drops `self` — and this is the
        // whole of what dropping it does.
        super::say_goodbye(&mut wire);

        let sent = &wire.peer().sent;
        assert_eq!(sent.len(), 2, "a destroyed connection wrote a third time");
        assert_eq!(queries(&sent[0]).len(), super::RESET_COMMANDS.len());
        // `X` and a length of four: the message has no body at all.
        assert_eq!(sent[1].as_slice(), b"X\x00\x00\x00\x04");
    }

    /// A wire that fails part way through the batch poisons the connection: six
    /// boundaries were promised and fewer arrived, so where the next message
    /// starts is not known.
    #[test]
    fn a_wire_that_fails_during_a_reset_poisons_the_connection() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| ready().repeat(2)));

        let failed = super::reset_session(&mut wire, &state).expect_err("a short reset passed");

        assert_eq!(failed.kind(), io::ErrorKind::UnexpectedEof);
        assert_eq!(state.get(), State::Poisoned);
        assert!(!state.get().is_poolable());
    }

    /// A reset is a statement like any other as far as § 4 goes: a connection
    /// with rows still in flight is not at a boundary, and nothing is written.
    #[test]
    fn a_reset_of_a_busy_connection_writes_nothing_and_is_refused() {
        for busy in [State::Executing, State::Streaming, State::Poisoned] {
            let state = Cell::new(busy);
            let mut wire = Wire::new(Peer::new(|_: &[u8]| Vec::new()));

            let refused = super::reset_session(&mut wire, &state)
                .expect_err("a busy connection was reset in place");

            assert_eq!(refused.kind(), io::ErrorKind::InvalidInput, "{busy:?}");
            assert!(wire.peer().sent.is_empty(), "{busy:?} reached the wire");
            assert_eq!(state.get(), busy, "{busy:?} was changed by a refusal");
        }
    }

    /// § 7's two options as PostgreSQL spells them, and the one place its five
    /// isolation levels become four: `Snapshot` renders exactly as
    /// `RepeatableRead` does, asserted against it rather than against a literal
    /// so that a level that grows its own spelling fails here.
    #[test]
    fn a_begin_renders_section_7s_options_and_snapshot_is_repeatable_read() {
        assert_eq!(super::begin_command(None, false), "BEGIN");
        assert_eq!(super::begin_command(None, true), "BEGIN READ ONLY");
        assert_eq!(
            super::begin_command(Some(Isolation::ReadUncommitted), false),
            "BEGIN ISOLATION LEVEL READ UNCOMMITTED"
        );
        assert_eq!(
            super::begin_command(Some(Isolation::ReadCommitted), false),
            "BEGIN ISOLATION LEVEL READ COMMITTED"
        );
        assert_eq!(
            super::begin_command(Some(Isolation::Serializable), true),
            "BEGIN ISOLATION LEVEL SERIALIZABLE READ ONLY"
        );
        assert_eq!(
            super::begin_command(Some(Isolation::Snapshot), false),
            super::begin_command(Some(Isolation::RepeatableRead), false),
        );
        assert_eq!(
            super::begin_command(Some(Isolation::Snapshot), false),
            "BEGIN ISOLATION LEVEL REPEATABLE READ"
        );

        // A caller that did not ask for a read-only transaction gets the
        // server's default rather than an override of it, whatever else it
        // asked for.
        for level in [
            None,
            Some(Isolation::ReadUncommitted),
            Some(Isolation::ReadCommitted),
            Some(Isolation::RepeatableRead),
            Some(Isolation::Snapshot),
            Some(Isolation::Serializable),
        ] {
            for read_only in [false, true] {
                let rendered = super::begin_command(level, read_only);
                assert!(!rendered.contains("READ WRITE"), "{rendered}");
                assert_eq!(
                    rendered.contains("READ ONLY"),
                    read_only,
                    "{rendered} against {read_only}"
                );
            }
        }
    }

    /// The SQL of every simple `Query` a peer was sent, flush by flush.
    fn every_query<F: FnMut(&[u8]) -> Vec<u8>>(wire: &Wire<Peer<F>>) -> Vec<String> {
        wire.peer()
            .sent
            .iter()
            .flat_map(|group| queries(group))
            .collect()
    }

    /// Each of § 7's commands is one simple `Query` in its own flush, and the
    /// connection is back at a boundary after each — which is what makes the
    /// callable's own statements ordinary ones rather than a mode.
    #[test]
    fn a_transaction_is_simple_queries_each_leaving_the_connection_idle() {
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| ready()));

        for close in [true, false] {
            super::begin(
                &mut wire,
                &state,
                &depth,
                Some(Isolation::Serializable),
                close,
            )
            .expect("the transaction opened");
            assert_eq!(state.get(), State::Idle);
            assert_eq!(depth.get(), 1);

            if close {
                super::commit(&mut wire, &state, &depth).expect("the commit ran");
            } else {
                super::roll_back(&mut wire, &state, &depth).expect("the rollback ran");
            }
            assert_eq!(state.get(), State::Idle);
            assert_eq!(depth.get(), 0);
        }

        assert_eq!(wire.peer().sent.len(), 4, "a command shared a flush");
        assert_eq!(
            every_query(&wire),
            vec![
                "BEGIN ISOLATION LEVEL SERIALIZABLE READ ONLY",
                "COMMIT",
                "BEGIN ISOLATION LEVEL SERIALIZABLE",
                "ROLLBACK",
            ]
        );
    }

    /// § 7's nesting: the second `transaction()` on one connection is a
    /// `SAVEPOINT` named by the depth it opened at, and the levels close
    /// inwards-out under the same names.
    #[test]
    fn a_nested_transaction_is_a_savepoint_named_by_its_depth() {
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| ready()));

        for open in 0..3 {
            super::begin(&mut wire, &state, &depth, None, false).expect("a level opened");
            assert_eq!(depth.get(), open + 1);
        }
        for open in (0..3).rev() {
            super::commit(&mut wire, &state, &depth).expect("a level closed");
            assert_eq!(depth.get(), open);
        }

        assert_eq!(
            every_query(&wire),
            vec![
                "BEGIN",
                "SAVEPOINT nvs_1",
                "SAVEPOINT nvs_2",
                "RELEASE SAVEPOINT nvs_2",
                "RELEASE SAVEPOINT nvs_1",
                "COMMIT",
            ]
        );
    }

    /// A nested rollback returns to its own savepoint and releases it in the
    /// same round trip, so a loop that abandons one nested transaction per
    /// iteration leaves the server holding nothing per iteration.
    #[test]
    fn a_nested_rollback_returns_to_its_savepoint_and_releases_it() {
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| ready()));

        super::begin(&mut wire, &state, &depth, None, false).expect("the transaction opened");
        for _ in 0..2 {
            super::begin(&mut wire, &state, &depth, None, false).expect("a level opened");
            super::roll_back(&mut wire, &state, &depth).expect("a level rolled back");
            assert_eq!(depth.get(), 1, "the outer transaction did not survive");
        }
        super::roll_back(&mut wire, &state, &depth).expect("the transaction rolled back");
        assert_eq!(depth.get(), 0);

        assert_eq!(
            every_query(&wire),
            vec![
                "BEGIN",
                "SAVEPOINT nvs_1",
                "ROLLBACK TO SAVEPOINT nvs_1; RELEASE SAVEPOINT nvs_1",
                "SAVEPOINT nvs_1",
                "ROLLBACK TO SAVEPOINT nvs_1; RELEASE SAVEPOINT nvs_1",
                "ROLLBACK",
            ]
        );
    }

    /// § 7's nesting past the depth the two cases above stop at, and the three
    /// things that only appear there: a level's savepoint is named by the
    /// depth it opened at however deep that gets, a name is taken again as
    /// soon as its level has closed, and exactly one level of a whole run —
    /// the outermost — is a real transaction, whatever mixture of commits and
    /// rollbacks closes the rest. A nested rollback with other levels still
    /// open beneath it is the case the pair above cannot reach at depth two.
    #[test]
    fn savepoints_nest() {
        const DEEP: u32 = 5;

        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| ready()));

        for open in 0..DEEP {
            super::begin(&mut wire, &state, &depth, None, false).expect("a level opened");
            assert_eq!(depth.get(), open + 1);
        }

        // The innermost level is abandoned and its name immediately taken by
        // the level that replaces it, which is what makes the name the depth's
        // and not a counter's.
        super::roll_back(&mut wire, &state, &depth).expect("the innermost rolled back");
        assert_eq!(depth.get(), DEEP - 1);
        super::begin(&mut wire, &state, &depth, None, false).expect("the level reopened");
        assert_eq!(depth.get(), DEEP);

        // Closed inwards-out in a mixture, so a rolled-back level sits beneath
        // levels that then commit and above levels that then roll back.
        for open in (0..DEEP).rev() {
            if open % 2 == 0 {
                super::roll_back(&mut wire, &state, &depth).expect("a level rolled back");
            } else {
                super::commit(&mut wire, &state, &depth).expect("a level closed");
            }
            assert_eq!(depth.get(), open, "a level closed more than itself");
        }

        let sent = every_query(&wire);
        assert_eq!(
            sent,
            vec![
                "BEGIN",
                "SAVEPOINT nvs_1",
                "SAVEPOINT nvs_2",
                "SAVEPOINT nvs_3",
                "SAVEPOINT nvs_4",
                "ROLLBACK TO SAVEPOINT nvs_4; RELEASE SAVEPOINT nvs_4",
                "SAVEPOINT nvs_4",
                "ROLLBACK TO SAVEPOINT nvs_4; RELEASE SAVEPOINT nvs_4",
                "RELEASE SAVEPOINT nvs_3",
                "ROLLBACK TO SAVEPOINT nvs_2; RELEASE SAVEPOINT nvs_2",
                "RELEASE SAVEPOINT nvs_1",
                "ROLLBACK",
            ]
        );

        // The invariant that list is one instance of, counted over the whole
        // run rather than read off it: two commands — the `BEGIN` and the
        // `ROLLBACK` that answers it — are the transaction, and every other
        // one names a savepoint strictly inside it. `nvs_0` is the outermost
        // level, which is never a savepoint, and `nvs_5` would be a level that
        // was never opened.
        assert_eq!(
            sent.iter().filter(|sql| !sql.contains("SAVEPOINT")).count(),
            2,
            "{sent:?}"
        );
        assert!(!sent.iter().any(|sql| sql.contains("nvs_0")), "{sent:?}");
        assert!(
            !sent
                .iter()
                .any(|sql| sql.contains(format!("nvs_{DEEP}").as_str())),
            "{sent:?}"
        );
        assert!(state.get().is_poolable());
    }

    /// A nested transaction cannot ask for its own isolation level or read-only
    /// mode, because PostgreSQL settles both for the whole transaction — and
    /// the refusal is unsent, rather than the option being dropped and the
    /// callable running at a level its author did not write.
    #[test]
    fn a_nested_transaction_asking_for_its_own_isolation_is_refused_unsent() {
        for asked in [(Some(Isolation::Serializable), false), (None, true)] {
            let state = Cell::new(State::Idle);
            let depth = Cell::new(0);
            let mut wire = Wire::new(Peer::new(|_: &[u8]| ready()));

            super::begin(&mut wire, &state, &depth, asked.0, asked.1)
                .expect("the outermost transaction may ask for either");
            let refused = super::begin(&mut wire, &state, &depth, asked.0, asked.1)
                .expect_err("a nested transaction set its own mode");

            assert_eq!(refused.kind(), io::ErrorKind::InvalidInput, "{asked:?}");
            assert_eq!(wire.peer().sent.len(), 1, "{asked:?} reached the wire");
            assert_eq!(depth.get(), 1, "{asked:?} opened a level anyway");
        }
    }

    /// Closing a transaction that was never opened is this driver's bug rather
    /// than a program's — § 7 has no `commit()` on the connection — so it is
    /// refused by name instead of sent as a `ROLLBACK` PostgreSQL answers with
    /// a warning nobody reads.
    #[test]
    fn closing_a_transaction_that_was_never_opened_is_refused_unsent() {
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| ready()));

        for refused in [
            super::commit(&mut wire, &state, &depth).expect_err("a commit with nothing open ran"),
            super::roll_back(&mut wire, &state, &depth)
                .expect_err("a rollback with nothing open ran"),
        ] {
            assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
            assert!(refused.to_string().contains("no open transaction"));
        }
        assert!(wire.peer().sent.is_empty());
        assert_eq!(state.get(), State::Idle);
    }

    /// A refused `COMMIT` is § 7's failed commit and carries the server's own
    /// error, and the connection is idle and poolable after it: the transaction
    /// is over, and nothing about where the next message starts is unknown.
    #[test]
    fn a_refused_commit_is_the_servers_own_error_and_the_connection_still_pools() {
        let state = Cell::new(State::Idle);
        let depth = Cell::new(1);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut out = error_response("40001", "could not serialize access");
            out.extend_from_slice(&ready());
            out
        }));

        let failed = super::commit(&mut wire, &state, &depth).expect_err("a refused commit passed");

        let said = failed.to_string();
        assert!(said.contains("40001"), "{said}");
        assert!(said.contains("could not serialize access"), "{said}");
        assert_eq!(state.get(), State::Idle);
        assert!(state.get().is_poolable());

        // § 7's retry rule reads this off the error rather than off the
        // sentence, and a serialization failure is one of its two codes.
        let server = ServerError::of(&failed).expect("a refusal carried no kind");
        assert_eq!(server.kind, DbErrorKind::SerializationFailure);
        assert!(server.kind.is_retryable());

        // And the transaction went with it, so the count did too: the retry
        // that rule licenses must open its next attempt with a `BEGIN`.
        assert_eq!(depth.get(), 0);
    }

    /// The other side of that bound: a refused `RELEASE SAVEPOINT` moves
    /// nothing. The nested level failed, but the transaction holding it is still
    /// open and only its own `ROLLBACK` closes it — so a count that fell here
    /// would send the outer commit as a savepoint command.
    #[test]
    fn a_refused_release_leaves_the_transaction_holding_it_open() {
        let state = Cell::new(State::Idle);
        let depth = Cell::new(2);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut out = error_response("40001", "could not serialize access");
            out.extend_from_slice(&ready());
            out
        }));

        super::commit(&mut wire, &state, &depth).expect_err("a refused release passed");

        assert_eq!(depth.get(), 2);
    }

    /// § 8's kind, `SQLSTATE` and constraint ride inside the `io::Error` that
    /// carries the sentence, and the errors this driver words itself carry
    /// none — which is what makes asking cheaper than matching on text.
    #[test]
    fn a_refused_statement_carries_section_8s_kind_beside_the_sentence() {
        let state = Cell::new(State::Idle);
        let depth = Cell::new(1);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut out =
                constrained_error_response("23505", "duplicate key value", "users_email_key");
            out.extend_from_slice(&ready());
            out
        }));

        let failed = super::commit(&mut wire, &state, &depth).expect_err("a refused commit passed");

        let said = failed.to_string();
        assert!(
            said.contains("postgres ERROR: duplicate key value"),
            "{said}"
        );
        assert!(said.contains("SQLSTATE 23505"), "{said}");

        let server = ServerError::of(&failed).expect("a refusal carried no kind");
        assert_eq!(server.kind, DbErrorKind::UniqueViolation);
        assert_eq!(server.sql_state, "23505");
        assert_eq!(server.constraint.as_deref(), Some("users_email_key"));
        assert!(!server.kind.is_retryable());

        let ours = super::second_statement(&Cell::new(State::Streaming));
        assert!(ServerError::of(&ours).is_none(), "{ours}");
    }

    /// § 8's rule that a bound parameter reaches the wire and never the error,
    /// so a refusal can be logged whole: the value is in the `Bind` we sent and
    /// in none of the sentence, the `ServerError` or the `Debug` a trace
    /// prints. PostgreSQL is what makes this a rule rather than a tautology —
    /// it returns the offending value in `D` (`Key (email)=(…) already
    /// exists.`) and never in `M`, so the fields [`super::server_error`] drops
    /// are the whole of the answer. Asserted over every field a value can
    /// arrive in rather than over `D` alone, because a walk that grew a second
    /// kept field would still pass on the first row.
    #[test]
    fn a_db_error_message_contains_no_bound_value() {
        // Deliberately not a substring of the SQL, which § 8 does allow on the
        // error, being developer-authored.
        const BOUND: &str = "correct-horse-battery-staple";

        // The detail, the hint, the internal query and the context of the
        // function that raised it: the four fields PostgreSQL words itself and
        // can quote a value into.
        for field in *b"DHqW" {
            let state = Cell::new(State::Idle);
            let echoed = format!("Key (email)=({BOUND}) already exists.");
            let mut wire = Wire::new(Peer::new(|_: &[u8]| {
                let mut out = error_response_with(
                    "23505",
                    "duplicate key value violates unique constraint \"users_email_key\"",
                    &[(b'n', "users_email_key"), (field, echoed.as_str())],
                );
                out.extend_from_slice(&ready());
                out
            }));

            let failed = start_statement(
                &mut wire,
                &state,
                &mut no_cache(),
                "insert into users (email) values ($1)",
                &[Some(BOUND.as_bytes())],
            )
            .expect_err("a unique violation was accepted");

            // The value did reach the server, so what follows is about where it
            // stopped rather than about a parameter nobody ever sent.
            assert!(
                wire.peer().sent.iter().any(|flushed| flushed
                    .windows(BOUND.len())
                    .any(|window| window == BOUND.as_bytes())),
                "the bound value never reached the wire",
            );

            let said = failed.to_string();
            assert!(said.contains("duplicate key value"), "{said}");
            assert!(!said.contains(BOUND), "{said}");

            // A trace prints the `Debug`, and § 8 covers that spelling too.
            let traced = format!("{failed:?}");
            assert!(!traced.contains(BOUND), "{traced}");

            let server = ServerError::of(&failed).expect("a refusal carried no kind");
            assert_eq!(server.kind, DbErrorKind::UniqueViolation);
            assert_eq!(server.constraint.as_deref(), Some("users_email_key"));
            assert!(!server.message.contains(BOUND), "{}", server.message);
        }
    }

    /// § 11 over § 7's own commands: each of the three answers the span of the
    /// command it *sent*, which is the only reason the driver hands one back at
    /// all — a nested level is a `SAVEPOINT` and its caller, which asked for a
    /// transaction, has no way to know that or to spell the name.
    #[test]
    fn a_transaction_command_answers_the_span_of_the_command_it_sent() {
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| ready()));

        let opened = super::begin(
            &mut wire,
            &state,
            &depth,
            Some(Isolation::Serializable),
            false,
        )
        .expect("the transaction opened");
        assert_eq!(opened.sql(), "BEGIN ISOLATION LEVEL SERIALIZABLE");
        assert_eq!(opened.driver(), Driver::Postgres);
        // Never `Some(0)`: § 7's commands report no count of their own, and
        // `affected` is where a reader tells "no count" from "zero rows".
        assert_eq!(opened.affected(), None);
        assert_eq!(opened.rows(), 0);

        let nested = super::begin(&mut wire, &state, &depth, None, false).expect("a level opened");
        assert_eq!(nested.sql(), "SAVEPOINT nvs_1");
        assert_eq!(
            super::roll_back(&mut wire, &state, &depth)
                .expect("the level rolled back")
                .sql(),
            "ROLLBACK TO SAVEPOINT nvs_1; RELEASE SAVEPOINT nvs_1",
        );
        assert_eq!(
            super::commit(&mut wire, &state, &depth)
                .expect("the transaction closed")
                .sql(),
            "COMMIT",
        );
    }

    /// § 11's span, and the property that makes it exportable at all: the bound
    /// value goes out on the wire and appears in no field of the span, in
    /// neither of its renderings, and — the part that is about the future — in
    /// nothing a field added later could carry, since the `Debug` it is
    /// asserted over is derived.
    ///
    /// The positive half is asserted first and is not decoration: "contains no
    /// parameter value" is trivially true of a span that carries nothing, so
    /// the driver, the row count, the affected count and the SQL are named
    /// before the absence is. What survives in the text is the **placeholder**
    /// — `$1` is still there where the value never was, which is the same fact
    /// § 1's refusal of emulated prepares states from the other side.
    #[test]
    fn a_query_span_contains_no_parameter_value_anywhere() {
        const BOUND: &str = "correct-horse-battery-staple";
        const SQL: &str = "select greeting from greetings where token = $1";

        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            one_statement(vec![
                data_row(&[Some(b"hello")]),
                data_row(&[Some(b"world")]),
            ])
        }));

        let mut rows = start_statement(
            &mut wire,
            &state,
            &mut no_cache(),
            SQL,
            &[Some(BOUND.as_bytes())],
        )
        .expect("the portal described itself");
        while rows.next_row().expect("the stream drained").is_some() {}
        // Cloned and the stream dropped, so the wire is readable below: the
        // value having reached the server is what makes this about where it
        // stopped rather than about a parameter nobody sent.
        let span = rows.span().clone();
        drop(rows);

        assert!(
            wire.peer().sent.iter().any(|flushed| flushed
                .windows(BOUND.len())
                .any(|window| window == BOUND.as_bytes())),
            "the bound value never reached the wire",
        );

        assert_eq!(span.driver(), Driver::Postgres);
        assert_eq!(span.rows(), 2);
        assert_eq!(span.affected(), Some(2));
        assert_eq!(span.sql(), SQL);
        assert!(!span.is_truncated());
        assert!(span.connection().is_none(), "the driver names no block");
        assert!(span.sql().contains("$1"), "{}", span.sql());

        let shown = span.to_string();
        assert!(!shown.contains(BOUND), "{shown}");
        assert!(shown.contains("driver=postgres"), "{shown}");
        assert!(shown.contains("rows=2"), "{shown}");

        let traced = format!("{span:?}");
        assert!(!traced.contains(BOUND), "{traced}");

        // The one field the driver does not fill, filled: a named connection
        // adds the block's name and still no value.
        let mut named = span.clone();
        named.name("main");
        let shown = named.to_string();
        assert!(shown.contains("connection=main"), "{shown}");
        assert!(!shown.contains(BOUND), "{shown}");
        assert!(!format!("{named:?}").contains(BOUND));
    }

    /// Every `SQLSTATE` this driver classifies, and the agreement that matters:
    /// exactly the two codes § 7 names re-run the callable, asserted over the
    /// whole table rather than on the two rows that answer `true`.
    #[test]
    fn every_sqlstate_classifies_and_only_the_two_the_retry_rule_names_retry() {
        let table = [
            ("23505", DbErrorKind::UniqueViolation),
            ("23503", DbErrorKind::ForeignKeyViolation),
            ("23502", DbErrorKind::NotNullViolation),
            ("23514", DbErrorKind::CheckViolation),
            ("40P01", DbErrorKind::Deadlock),
            ("40001", DbErrorKind::SerializationFailure),
            ("08006", DbErrorKind::ConnectionLost),
            ("08P01", DbErrorKind::ConnectionLost),
            ("57P01", DbErrorKind::ConnectionLost),
            ("57014", DbErrorKind::Timeout),
            ("25P03", DbErrorKind::Timeout),
            ("42601", DbErrorKind::Syntax),
            ("42P01", DbErrorKind::Syntax),
            ("42501", DbErrorKind::Permission),
            ("28P01", DbErrorKind::Permission),
            // Class 23 is not mapped wholesale: § 8 names four integrity
            // conditions and `restrict_violation` is not one of them.
            ("23001", DbErrorKind::Other),
            ("XX000", DbErrorKind::Other),
            ("", DbErrorKind::Other),
        ];

        for (code, kind) in table {
            assert_eq!(super::kind_of(code), kind, "{code}");
            assert_eq!(
                kind.is_retryable(),
                matches!(code, "40P01" | "40001"),
                "{code}"
            );
        }
    }

    /// § 4 governs these three like any other statement, and all three answer
    /// the refusal that names both of its fixes rather than a third wording.
    #[test]
    fn a_transaction_command_on_a_busy_connection_writes_nothing_and_is_refused() {
        for busy in [State::Executing, State::Streaming, State::Poisoned] {
            let state = Cell::new(busy);
            // One level open, so `commit` and `roll_back` reach § 4's refusal
            // rather than the one for a connection in no transaction at all.
            let depth = Cell::new(1);
            let mut wire = Wire::new(Peer::new(|_: &[u8]| Vec::new()));

            let checked = |refused: &io::Error| {
                assert_eq!(refused.kind(), io::ErrorKind::InvalidInput, "{busy:?}");
                let said = refused.to_string();
                assert!(said.contains("->all()"), "{said}");
                assert!(said.contains("shared: false"), "{said}");
            };
            checked(
                &super::begin(&mut wire, &state, &depth, None, false)
                    .expect_err("a busy connection opened a transaction"),
            );
            checked(
                &super::commit(&mut wire, &state, &depth).expect_err("a busy connection committed"),
            );
            checked(
                &super::roll_back(&mut wire, &state, &depth)
                    .expect_err("a busy connection rolled back"),
            );

            assert!(wire.peer().sent.is_empty(), "{busy:?} reached the wire");
            assert_eq!(state.get(), busy, "{busy:?} was changed by a refusal");
        }
    }

    /// A column of one type and width, which is all a decode reads.
    fn column(type_oid: Oid, type_modifier: i32) -> PgColumn {
        PgColumn {
            name: "c".to_owned(),
            type_oid,
            type_modifier,
        }
    }

    /// Every OID this driver names, described as the `ColumnType` spec § 18
    /// gives it, plus the two directions in which a description is not what a
    /// decode of the same column says.
    #[test]
    fn a_column_type_describes_the_column_where_a_decode_answers_for_the_value() {
        for (type_oid, expected) in [
            (oid::INT2, ColumnType::Int),
            (oid::INT4, ColumnType::Int),
            (oid::INT8, ColumnType::Int),
            (oid::OID, ColumnType::Uint),
            (oid::FLOAT4, ColumnType::Float),
            (oid::FLOAT8, ColumnType::Float),
            (oid::NUMERIC, ColumnType::Decimal),
            (oid::MONEY, ColumnType::Decimal),
            (oid::CHAR, ColumnType::Text),
            (oid::NAME, ColumnType::Text),
            (oid::TEXT, ColumnType::Text),
            (oid::BPCHAR, ColumnType::Text),
            (oid::VARCHAR, ColumnType::Text),
            (oid::BYTEA, ColumnType::Bytes),
            (oid::BOOL, ColumnType::Bool),
            (oid::DATE, ColumnType::Date),
            (oid::TIME, ColumnType::Time),
            (oid::TIMESTAMP, ColumnType::DateTime),
            (oid::TIMESTAMPTZ, ColumnType::Instant),
            (oid::UUID, ColumnType::Uuid),
            (oid::JSON, ColumnType::Json),
            (oid::JSONB, ColumnType::Json),
        ] {
            assert_eq!(
                column(type_oid, -1).column_type(),
                expected,
                "OID {type_oid} described"
            );
        }

        // § 9's `BIT(1)` row is a `bool` and every wider bit string is text,
        // and the description reads the modifier for exactly that reason.
        for type_oid in [oid::BIT, oid::VARBIT] {
            assert_eq!(column(type_oid, 1).column_type(), ColumnType::Bool);
            assert_eq!(column(type_oid, 8).column_type(), ColumnType::Other);
        }

        // One direction: a description no decode could have produced, because
        // both of these decode as the text § 9 leaves them at.
        for type_oid in [oid::JSON, oid::JSONB] {
            let json = column(type_oid, -1);
            assert_eq!(json.column_type(), ColumnType::Json);
            assert_eq!(
                rendered(&json.scalar(Some(&b"{}"[..])).expect("json decoded")),
                "text {}"
            );
        }

        // The other: `text[]` decodes to an array and `inet` to text, and both
        // describe as `Other` — the enum has no array case, by § 18's design.
        assert_eq!(column(1009, -1).column_type(), ColumnType::Other);
        assert_eq!(column(869, -1).column_type(), ColumnType::Other);
    }

    /// What a decoded column is, rendered so a whole table of § 9's rows fits
    /// in a line each.
    ///
    /// `PgScalar`'s own `Debug` names the row and never the value — a row's
    /// data is not something this module renders, which is the rule `PgRow`'s
    /// `Debug` holds — so a test that wants to read one says so here, where it
    /// is a test's own decision over data it wrote itself.
    fn rendered(scalar: &PgScalar<'_>) -> String {
        match scalar {
            PgScalar::Null => "null".to_owned(),
            PgScalar::Bool(value) => format!("bool {value}"),
            PgScalar::Int(value) => format!("int {value}"),
            PgScalar::UInt(value) => format!("uint {value}"),
            PgScalar::Float(value) => format!("float {value}"),
            PgScalar::Decimal(value) => format!("decimal {value}"),
            PgScalar::Text(value) => format!("text {value}"),
            PgScalar::Bytes(value) => format!("bytes {:?}", value.as_bytes()),
            PgScalar::Date(date) => format!("date {}", civil(date)),
            PgScalar::Time(time) => format!("time {}", clock(time)),
            PgScalar::Timestamp { date, time } => {
                format!("timestamp {} {}", civil(date), clock(time))
            }
            PgScalar::Instant { date, time, offset } => {
                format!("instant {} {} {offset:+}", civil(date), clock(time))
            }
            PgScalar::Uuid(octets) => format!("uuid {octets:02x?}"),
            PgScalar::Array(items) => format!(
                "array [{}]",
                items.iter().map(rendered).collect::<Vec<_>>().join(", ")
            ),
        }
    }

    /// A decoded date, with the year unpadded and signed so that the BC half
    /// of the calendar is visible rather than plausible.
    fn civil(date: &PgDate) -> String {
        format!("{}-{:02}-{:02}", date.year, date.month, date.day)
    }

    /// A decoded time, always to the nanosecond, so a row whose fraction was
    /// scaled by the wrong power of ten cannot print like one that was not.
    fn clock(time: &PgTime) -> String {
        format!(
            "{:02}:{:02}:{:02}.{:09}",
            time.hour, time.minute, time.second, time.nanosecond
        )
    }

    /// `rule:core-classes/db-column-types`'s scalar rows, PostgreSQL's half of them: the whole table
    /// asserted a row at a time, including the rows that reach it by *not*
    /// being in the OID list — 25 is `text` and 114 is `json`, and both arrive
    /// at the last row's `tainted string` along with every type this driver
    /// has no arm for.
    #[test]
    fn every_scalar_row_of_the_type_map_decodes_to_its_novis_type() {
        let cases: &[(Oid, i32, &[u8], &str)] = &[
            (oid::BOOL, -1, b"t", "bool true"),
            (oid::BOOL, -1, b"f", "bool false"),
            (oid::BIT, 1, b"1", "bool true"),
            (oid::VARBIT, 1, b"0", "bool false"),
            (oid::BIT, 8, b"10110000", "text 10110000"),
            (oid::INT2, -1, b"-32768", "int -32768"),
            (oid::INT4, -1, b"2147483647", "int 2147483647"),
            (
                oid::INT8,
                -1,
                b"-9223372036854775808",
                "int -9223372036854775808",
            ),
            (oid::OID, -1, b"4294967295", "uint 4294967295"),
            (oid::FLOAT4, -1, b"1.5", "float 1.5"),
            (oid::FLOAT8, -1, b"-Infinity", "float -inf"),
            (oid::FLOAT8, -1, b"NaN", "float NaN"),
            (oid::NUMERIC, -1, b"0.000001", "decimal 0.000001"),
            (
                oid::NUMERIC,
                -1,
                b"-12345678901234567890.12",
                "decimal -12345678901234567890.12",
            ),
            (oid::MONEY, -1, b"$1,234.56", "decimal 1234.56"),
            (oid::MONEY, -1, b"-$1,234.56", "decimal -1234.56"),
            (oid::MONEY, -1, b"($1,234.56)", "decimal -1234.56"),
            (
                oid::MONEY,
                -1,
                "1.234,56 \u{20ac}".as_bytes(),
                "decimal 1234.56",
            ),
            (oid::MONEY, -1, "\u{a5}1,234".as_bytes(), "decimal 1234"),
            (oid::BYTEA, -1, br"\x00ff", "bytes [0, 255]"),
            (oid::BYTEA, -1, br"a\\b\001", "bytes [97, 92, 98, 1]"),
            (25, -1, b"hi", "text hi"),
            (114, -1, b"{\"a\":1}", "text {\"a\":1}"),
        ];

        for &(type_oid, type_modifier, body, expected) in cases {
            let subject = column(type_oid, type_modifier);
            let decoded = subject
                .scalar(Some(body))
                .unwrap_or_else(|error| panic!("OID {type_oid} did not decode: {error}"));

            assert_eq!(rendered(&decoded), expected, "OID {type_oid}");
        }
    }

    /// § 4's affected-row count, over every tag shape PostgreSQL writes.
    ///
    /// The `INSERT` row is the one that matters: its tag carries two numbers
    /// and the first is an OID, so a driver reading the tag's *first* field
    /// would report zero rows inserted on every modern server — plausibly, and
    /// on the one statement whose count is checked most often.
    #[test]
    fn the_affected_row_count_is_the_tags_last_field_and_a_ddl_tag_has_none() {
        assert_eq!(affected_rows("INSERT 0 3"), Some(3));
        assert_eq!(affected_rows("INSERT 0 1"), Some(1));
        assert_eq!(affected_rows("UPDATE 7"), Some(7));
        assert_eq!(affected_rows("DELETE 0"), Some(0));
        assert_eq!(affected_rows("SELECT 2"), Some(2));
        assert_eq!(affected_rows("MERGE 5"), Some(5));
        assert_eq!(affected_rows("MOVE 4"), Some(4));
        assert_eq!(affected_rows("FETCH 1"), Some(1));
        assert_eq!(affected_rows("COPY 9"), Some(9));

        // A tag with no count is not a tag reporting zero, and neither is a
        // command word this driver has never seen.
        assert_eq!(affected_rows("CREATE TABLE"), None);
        assert_eq!(affected_rows("BEGIN"), None);
        assert_eq!(affected_rows("SET"), None);
        assert_eq!(affected_rows("VACUUM"), None);
        assert_eq!(affected_rows("REINDEX 3"), None);
    }

    /// § 4's count on this protocol is the rows **matched**, and `changed` is
    /// that same number.
    ///
    /// The statement is the one where the two answers diverge everywhere they
    /// can: `UPDATE t SET a = a` matches every row its `WHERE` selects and
    /// alters none of them, which is where MySQL answers zero affected while
    /// reporting three matched out of the same execution. PostgreSQL's tag is
    /// `UPDATE 3` either way — the protocol carries the matched count and
    /// carries nothing else — so § 4's `changed` reads this same number, and
    /// the doc on `PgRows::affected` owns why inventing a second one would be
    /// worse than having none.
    ///
    /// Tag parsing is not asked again here: the case above owns every tag
    /// shape, and this one asks only which of the two numbers the one field
    /// holds. The zero row is the half that matters — a statement that matched
    /// nothing and a statement that altered nothing carry the same tag, so a
    /// driver reporting rows *changed* would be right on it by accident.
    #[test]
    fn affected_is_the_matched_count_and_changed_is_mysql_only() {
        for matched in [0_u64, 1, 3] {
            let state = Cell::new(State::Idle);
            let completion = format!("UPDATE {matched}\0");
            let mut wire = Wire::new(Peer::new(move |_: &[u8]| {
                let mut out = message(b'1', b""); // ParseComplete
                out.extend_from_slice(&message(b'2', b"")); // BindComplete
                // An `UPDATE` with no `RETURNING` clause describes as no data
                // at all, so the completion is the only thing that carries a
                // number in the whole answer.
                out.extend_from_slice(&message(b'n', b"")); // NoData
                out.extend_from_slice(&message(b'C', completion.as_bytes()));
                out.extend_from_slice(&message(b'Z', b"I"));
                out
            }));

            let mut rows = start_statement(
                &mut wire,
                &state,
                &mut no_cache(),
                "update t set a = a where id > 0",
                &[],
            )
            .expect("the portal opened");
            assert!(rows.columns().is_empty(), "an UPDATE described rows");
            assert_eq!(
                rows.affected(),
                None,
                "a count was answered before the completion arrived"
            );

            assert!(rows.next_row().expect("the stream drained").is_none());

            let tag = format!("UPDATE {matched}");
            assert_eq!(rows.command_tag(), Some(tag.as_str()));
            assert_eq!(rows.affected(), Some(matched));
        }

        // The other half of the claim, and the only place in this crate that
        // can state it: there is no second count on the PostgreSQL half for
        // § 4's `changed` to read, so it reads `affected` or it reads nothing.
        // The needle is assembled rather than written out, because a literal
        // would itself be a line of this file's source and the scan would then
        // find nothing but itself.
        assert!(
            !include_str!("pg.rs").contains(concat!("fn ", "changed")),
            "a second row count appeared on this driver; § 4 has one number"
        );
    }

    /// § 9's structured rows: those whose Novis type is a class instance, and
    /// so are components here rather than a value.
    ///
    /// Asserted through `scalar`, which is the seam itself — `decode` answers
    /// `None` for every one of these, and the test below pins that. The last
    /// case is `timetz`, which has an OID of its own and no row in § 9: it
    /// stays at the table's `tainted string`, and a driver that read it as a
    /// `TIME` would be dropping an offset silently.
    #[test]
    fn every_structured_row_of_the_type_map_decodes_to_its_components() {
        let cases: &[(Oid, &[u8], &str)] = &[
            (oid::DATE, b"2024-01-02", "date 2024-01-02"),
            (oid::DATE, b"0044-03-15 BC", "date -43-03-15"),
            (oid::TIME, b"03:04:05", "time 03:04:05.000000000"),
            (oid::TIME, b"03:04:05.123456", "time 03:04:05.123456000"),
            (oid::TIME, b"24:00:00", "time 24:00:00.000000000"),
            (
                oid::TIMESTAMP,
                b"2024-01-02 03:04:05.5",
                "timestamp 2024-01-02 03:04:05.500000000",
            ),
            (
                oid::TIMESTAMPTZ,
                b"2024-01-02 03:04:05+02",
                "instant 2024-01-02 03:04:05.000000000 +7200",
            ),
            (
                oid::TIMESTAMPTZ,
                b"2024-01-02 03:04:05.123456-05:30",
                "instant 2024-01-02 03:04:05.123456000 -19800",
            ),
            (
                oid::TIMESTAMPTZ,
                b"0001-01-01 00:00:00+00 BC",
                "instant 0-01-01 00:00:00.000000000 +0",
            ),
            (
                oid::UUID,
                b"a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11",
                "uuid [a0, ee, bc, 99, 9c, 0b, 4e, f8, bb, 6d, 6b, b9, bd, 38, 0a, 11]",
            ),
            (1266, b"03:04:05+02", "text 03:04:05+02"),
        ];

        for &(type_oid, body, expected) in cases {
            let subject = column(type_oid, -1);
            let decoded = subject
                .scalar(Some(body))
                .unwrap_or_else(|error| panic!("OID {type_oid} did not decode: {error}"));

            assert_eq!(rendered(&decoded), expected, "OID {type_oid}");
        }
    }

    /// § 9's two timestamp rows, and the zone each of them is read in.
    ///
    /// A zone-less `TIMESTAMP` reads in the zone the *connection* declared —
    /// `time_zone` in the block, `PgConn::time_zone` to the caller — while a
    /// `TIMESTAMPTZ` carries its own offset and ignores that declaration
    /// entirely. Both halves are asserted over a sweep of declared zones,
    /// because the failure worth pinning is a decode that **moves** with one:
    /// the zone-less row hands back civil fields and no offset at all, leaving
    /// the caller nothing to read the zone off but `time_zone()`, and the
    /// zoned row hands back the offset its own body carried.
    ///
    /// The `+00` body against a `+02:00` connection is the bound. A driver
    /// that applied the declared zone to a `TIMESTAMPTZ` answers `+7200`
    /// there — the same wall clock two hours out, and right on every row whose
    /// connection happens to be UTC, which is most of them in a test suite.
    #[test]
    fn a_zoneless_column_reads_in_the_declared_zone_and_a_timestamptz_ignores_it() {
        for (written, declared) in [
            (None, 0),
            (Some("+02:00"), 2 * 3600),
            (Some("-05:00"), -5 * 3600),
            (Some("+05:45"), 5 * 3600 + 45 * 60),
        ] {
            let block = Database {
                time_zone: written.map(str::to_owned),
                ..block()
            };
            let target = PgTarget::resolve(&block).expect("a zone that is an offset resolves");
            assert_eq!(target.time_zone, declared, "{written:?}");

            for (type_oid, body, expected) in [
                (
                    oid::TIMESTAMP,
                    b"2024-01-02 03:04:05".as_slice(),
                    "timestamp 2024-01-02 03:04:05.000000000",
                ),
                (
                    oid::TIMESTAMPTZ,
                    b"2024-01-02 03:04:05+00".as_slice(),
                    "instant 2024-01-02 03:04:05.000000000 +0",
                ),
                (
                    oid::TIMESTAMPTZ,
                    b"2024-01-02 03:04:05-05:30".as_slice(),
                    "instant 2024-01-02 03:04:05.000000000 -19800",
                ),
            ] {
                let decoded = column(type_oid, -1)
                    .scalar(Some(body))
                    .unwrap_or_else(|error| panic!("OID {type_oid} did not decode: {error}"));

                assert_eq!(
                    rendered(&decoded),
                    expected,
                    "OID {type_oid} moved with a declared zone of {declared}"
                );
            }
        }

        // The other direction, and what stops the rule above from being
        // satisfied by a truncation: a zone-less column whose body carries an
        // offset is refused rather than read with the suffix dropped.
        assert!(
            column(oid::TIMESTAMP, -1)
                .scalar(Some(b"2024-01-02 03:04:05+02"))
                .is_err(),
            "an offset was dropped off a TIMESTAMP rather than refused"
        );
    }

    /// § 9's table, one row per constant the [`oid`] module names: the OID,
    /// its modifier, a body the server would really write, spec § 18's
    /// description of the column, and § 9's decoded value as [`rendered`]
    /// spells it.
    ///
    /// Shared rather than local because two tests below sweep it, and sharing
    /// it is what makes the second one's coverage follow the first's: a type
    /// this driver gains is asked both questions from one row, and
    /// [`every_row_of_the_type_map_round_trips`]'s count against
    /// [`oid_constants`] is what says the table is whole.
    const TYPE_MAP: &[(Oid, i32, &[u8], ColumnType, &str)] = &[
        (oid::BOOL, -1, b"t", ColumnType::Bool, "bool true"),
        (
            oid::BYTEA,
            -1,
            br"\x00ff",
            ColumnType::Bytes,
            "bytes [0, 255]",
        ),
        (oid::CHAR, -1, b"c", ColumnType::Text, "text c"),
        (
            oid::NAME,
            -1,
            b"pg_class",
            ColumnType::Text,
            "text pg_class",
        ),
        (
            oid::INT8,
            -1,
            b"-9223372036854775808",
            ColumnType::Int,
            "int -9223372036854775808",
        ),
        (oid::INT2, -1, b"-32768", ColumnType::Int, "int -32768"),
        (
            oid::INT4,
            -1,
            b"2147483647",
            ColumnType::Int,
            "int 2147483647",
        ),
        (oid::TEXT, -1, b"hi", ColumnType::Text, "text hi"),
        (
            oid::OID,
            -1,
            b"4294967295",
            ColumnType::Uint,
            "uint 4294967295",
        ),
        (oid::JSON, -1, b"{}", ColumnType::Json, "text {}"),
        (oid::FLOAT4, -1, b"1.5", ColumnType::Float, "float 1.5"),
        (
            oid::FLOAT8,
            -1,
            b"-Infinity",
            ColumnType::Float,
            "float -inf",
        ),
        (
            oid::MONEY,
            -1,
            b"$1,234.56",
            ColumnType::Decimal,
            "decimal 1234.56",
        ),
        (oid::BPCHAR, -1, b"ab", ColumnType::Text, "text ab"),
        (oid::VARCHAR, -1, b"ab", ColumnType::Text, "text ab"),
        (
            oid::DATE,
            -1,
            b"2024-01-02",
            ColumnType::Date,
            "date 2024-01-02",
        ),
        (
            oid::TIME,
            -1,
            b"03:04:05",
            ColumnType::Time,
            "time 03:04:05.000000000",
        ),
        (
            oid::TIMESTAMP,
            -1,
            b"2024-01-02 03:04:05.5",
            ColumnType::DateTime,
            "timestamp 2024-01-02 03:04:05.500000000",
        ),
        (
            oid::TIMESTAMPTZ,
            -1,
            b"2024-01-02 03:04:05+02",
            ColumnType::Instant,
            "instant 2024-01-02 03:04:05.000000000 +7200",
        ),
        // § 9's `BIT(1)` row, the one place a modifier rather than an OID
        // decides which row of the table a column is on. The wider bit
        // string is the test above's, whose subject is that boundary.
        (oid::BIT, 1, b"1", ColumnType::Bool, "bool true"),
        (oid::VARBIT, 1, b"0", ColumnType::Bool, "bool false"),
        (
            oid::NUMERIC,
            -1,
            b"0.000001",
            ColumnType::Decimal,
            "decimal 0.000001",
        ),
        (
            oid::UUID,
            -1,
            b"a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11",
            ColumnType::Uuid,
            "uuid [a0, ee, bc, 99, 9c, 0b, 4e, f8, bb, 6d, 6b, b9, bd, 38, 0a, 11]",
        ),
        (oid::JSONB, -1, b"{}", ColumnType::Json, "text {}"),
    ];

    /// § 9's whole table in one sweep, bounded by the OID table's own length
    /// rather than by what an author remembered to list.
    ///
    /// The two tests above split the table in half and read it a line at a
    /// time, so an OID this driver names and neither of them covers is
    /// invisible to both — the one failure a per-row table cannot see. Here
    /// the rows are counted against the number of constants the [`oid`] module
    /// declares, read out of this file's own source by [`oid_constants`]: a
    /// type added to the driver without a row here fails, and so does one
    /// given two rows.
    ///
    /// Each row asserts the round trip § 9 specifies end to end — the
    /// description `columns()` gives the column, the scalar a decode of a body
    /// the server would really write produces, and, through
    /// [`row_of_the_type_map`], that those two name the **same** row of the
    /// table. That last assertion is the one the halves cannot make: a driver
    /// that described a column as one type and decoded it as another passes
    /// both of them, a line each, looking right.
    #[test]
    fn every_row_of_the_type_map_round_trips() {
        for &(type_oid, type_modifier, body, described, decoded) in TYPE_MAP {
            let subject = column(type_oid, type_modifier);
            assert_eq!(subject.column_type(), described, "OID {type_oid} described");

            let scalar = subject
                .scalar(Some(body))
                .unwrap_or_else(|error| panic!("OID {type_oid} did not decode: {error}"));
            assert_eq!(rendered(&scalar), decoded, "OID {type_oid} decoded");

            assert_eq!(
                decoded.split(' ').next(),
                Some(row_of_the_type_map(described)),
                "OID {type_oid} describes as one row of § 9 and decodes as another"
            );
        }

        let mut covered: Vec<Oid> = TYPE_MAP.iter().map(|row| row.0).collect();
        covered.sort_unstable();
        covered.dedup();
        assert_eq!(
            covered.len(),
            TYPE_MAP.len(),
            "one OID has two rows in the sweep"
        );
        assert_eq!(
            covered.len(),
            oid_constants(),
            "the sweep and the `oid` module disagree about how many types this driver names"
        );
    }

    /// `rule:core-classes/db-one-api`'s § *Context* defect,
    /// pinned: **no column § 9 gives a number decodes to a string**, whatever
    /// the value.
    ///
    /// This is a thing `Core\Db` exists to remove rather than a row of the
    /// map. PDO hands `DECIMAL` back as a string on every driver it has, and
    /// `pgsql` hands back *every* column as one, so PHP code compares a price
    /// with `==` and gets away with it until a value stops surviving the float
    /// it is silently coerced through — § 9's `decimal` row says exactly that,
    /// in the column beside it.
    ///
    /// Three parts, and the first is why this is not a second copy of the
    /// sweep above: it takes its OIDs from [`TYPE_MAP`], so a numeric type
    /// this driver gains is asked this question too without being listed here
    /// again. The values then are the ones at which *a number as a string* and
    /// *a number as a `float`* stop being the same answer as a number, so a
    /// driver that went through either loses digits rather than an assertion.
    /// The last part is the other side of the bound: the rule is the column's
    /// declared type and never the body, so a `text` column holding digits
    /// stays a string — which is what a driver that fixed the defect by
    /// sniffing bodies would fail.
    #[test]
    fn no_driver_returns_a_number_as_a_string() {
        for &(type_oid, type_modifier, body, described, _) in TYPE_MAP {
            if !matches!(
                described,
                ColumnType::Int | ColumnType::Uint | ColumnType::Float | ColumnType::Decimal
            ) {
                continue;
            }

            let scalar = column(type_oid, type_modifier)
                .scalar(Some(body))
                .unwrap_or_else(|error| panic!("OID {type_oid} did not decode: {error}"));

            assert!(
                matches!(
                    scalar,
                    PgScalar::Int(_)
                        | PgScalar::UInt(_)
                        | PgScalar::Float(_)
                        | PgScalar::Decimal(_)
                ),
                "OID {type_oid} describes as a number and decoded as {}",
                rendered(&scalar)
            );
        }

        // 2^53 + 1 is the first integer a `float` cannot hold, `i64::MAX` is
        // where PHP's own `int` stops, and the `NUMERIC` here carries twenty
        // significant digits no `float` has room for. Each is a value whose
        // string and whose number are still the same characters, and whose
        // float is not.
        for (type_oid, body, expected) in [
            (oid::INT8, &b"9007199254740993"[..], "int 9007199254740993"),
            (oid::INT8, b"9223372036854775807", "int 9223372036854775807"),
            (oid::OID, b"4294967295", "uint 4294967295"),
            (
                oid::NUMERIC,
                b"-12345678901234567890.12",
                "decimal -12345678901234567890.12",
            ),
            (oid::MONEY, b"($1,234.56)", "decimal -1234.56"),
            (oid::FLOAT8, b"0.1", "float 0.1"),
        ] {
            let scalar = column(type_oid, -1)
                .scalar(Some(body))
                .unwrap_or_else(|error| panic!("OID {type_oid} did not decode: {error}"));

            assert_eq!(rendered(&scalar), expected, "OID {type_oid}");
        }

        // The other side: what makes a value a number is the column's declared
        // type and never the body. A driver reading the body instead would
        // answer an int for `select code from …` and be right often enough to
        // ship — the same defect, fixed in the direction that loses a string.
        for type_oid in [oid::TEXT, oid::VARCHAR, oid::BPCHAR, oid::CHAR, oid::NAME] {
            let scalar = column(type_oid, -1)
                .scalar(Some(&b"12"[..]))
                .unwrap_or_else(|error| panic!("OID {type_oid} did not decode: {error}"));

            assert_eq!(rendered(&scalar), "text 12", "OID {type_oid}");
        }
    }

    /// The [`PgScalar`] § 9 pairs with a description, spelled as [`rendered`]
    /// spells it.
    ///
    /// Exhaustive on purpose: a [`ColumnType`] added to spec § 18 has to be
    /// given its row of § 9 here, and that is the one thing a table of OIDs
    /// cannot be made to notice on its own.
    fn row_of_the_type_map(column_type: ColumnType) -> &'static str {
        match column_type {
            ColumnType::Int => "int",
            ColumnType::Uint => "uint",
            ColumnType::Float => "float",
            ColumnType::Decimal => "decimal",
            ColumnType::Text => "text",
            ColumnType::Bytes => "bytes",
            ColumnType::Bool => "bool",
            ColumnType::Date => "date",
            ColumnType::Time => "time",
            ColumnType::DateTime => "timestamp",
            ColumnType::Instant => "instant",
            ColumnType::Uuid => "uuid",
            // § 9's `JSON`/`JSONB` row is `tainted string` — JSON is never
            // auto-decoded — so this pairing is what agreement *is* for a JSON
            // column, not an exception to it.
            ColumnType::Json => "text",
            // § 9's last row. An array describes as `Other` too and decodes to
            // an `Array`, so this pairing is the one the sweep does not reach:
            // every OID the driver names that lands on `Other` is text, and
            // the test above owns the array direction.
            ColumnType::Other => "text",
        }
    }

    /// How many types the [`oid`] module names, read out of this file rather
    /// than written down beside it.
    ///
    /// A number kept by hand agrees with the table until the session that adds
    /// a type, which is precisely the session it has to disagree in. The slice
    /// runs from the module's own line to [`oid::element`], the first thing in
    /// it that is not a constant.
    fn oid_constants() -> usize {
        let source = include_str!("pg.rs");
        let start = source
            .find("\nmod oid {")
            .expect("the OID table is in this file");
        let end = start
            + source[start..]
                .find("pub(super) fn element")
                .expect("the OID table ends where `element` begins");

        source[start..end].matches("pub(super) const ").count()
    }

    /// § 4's `lastId` on PostgreSQL is the id a `RETURNING` clause handed back:
    /// the last returned row's first column, and never the tag.
    ///
    /// The `INSERT 0 2` here is the trap in full — a driver reading the tag's
    /// first number would answer `0`, plausibly, on every server since
    /// PostgreSQL 12, and on the statement whose id is asked for most often.
    /// The two rows are the other half: `RETURNING` on a multi-row insert
    /// hands back one row each, and the *last* is the id.
    #[test]
    fn the_last_id_is_the_returning_rows_and_never_the_insert_tag() {
        let state = Cell::new(State::Idle);
        let mut wire = Wire::new(Peer::new(|_: &[u8]| {
            let mut out = message(b'1', b""); // ParseComplete
            out.extend_from_slice(&message(b'2', b"")); // BindComplete
            out.extend_from_slice(&row_description(b"id", 20));
            out.extend_from_slice(&data_row(&[Some(b"7")]));
            out.extend_from_slice(&data_row(&[Some(b"8")]));
            out.extend_from_slice(&message(b'C', b"INSERT 0 2\0"));
            out.extend_from_slice(&message(b'Z', b"I"));
            out
        }));

        let mut rows = start_statement(
            &mut wire,
            &state,
            &mut no_cache(),
            "insert into t default values returning id",
            &[],
        )
        .expect("the portal opened");
        assert_eq!(rows.last_id(), None, "an id was reported before a row was");

        while rows.next_row().expect("the stream drained").is_some() {}

        assert_eq!(rows.affected(), Some(2));
        assert_eq!(rows.last_id(), Some(8));
    }

    /// A statement that returned no key has no `lastId`, and the two ways of
    /// having none are both here: no rows at all, which is every write without
    /// a `RETURNING` clause, and rows whose first column is not an integer.
    ///
    /// The second is the one worth a case. A `text` column holding `12` reads
    /// as a number by every test a body can be given, so a driver that looked
    /// at the body rather than at the column's declared type would answer an id
    /// for `select name from …` and be right often enough to ship.
    #[test]
    fn a_statement_that_returned_no_key_has_no_last_id() {
        for sent in [Vec::new(), vec![data_row(&[Some(b"12")])]] {
            let state = Cell::new(State::Idle);
            let mut wire = Wire::new(Peer::new(move |_: &[u8]| one_statement(sent.clone())));

            let mut rows =
                start_statement(&mut wire, &state, &mut no_cache(), "select greeting", &[])
                    .expect("the portal opened");
            while rows.next_row().expect("the stream drained").is_some() {}

            assert_eq!(rows.last_id(), None, "a text column was read as an id");
        }
    }

    /// § 9's array row is every other row again, one element at a time: what a
    /// `text[]` holds is what a `text` column holds, and a driver that grew a
    /// second decoder for elements would be the thing this asserts against.
    ///
    /// The cases are the shapes `array_out` writes: the empty array, a quoted
    /// element, an escaped one, the unquoted `NULL` that is SQL null beside
    /// the quoted `"NULL"` that is four characters, a nested array, and the
    /// `[0:1]=` prefix a lower bound other than one produces. The last three
    /// are the seams rather than the syntax — a `bytea` element is escaped
    /// because its own rendering starts with a backslash, a `BIT(1)[]` reads
    /// its width off the array column because PostgreSQL stores an array's
    /// modifier as its element's, and `point[]` is an array this driver cannot
    /// name the element type of, so it stays whole at the table's last row.
    #[test]
    fn an_array_decodes_element_by_element_through_the_same_table() {
        let cases: &[(Oid, i32, &[u8], &str)] = &[
            (1007, -1, b"{1,2,3}", "array [int 1, int 2, int 3]"),
            (1007, -1, b"{}", "array []"),
            (1007, -1, b"{NULL,1}", "array [null, int 1]"),
            (1009, -1, br#"{a,"b,c"}"#, "array [text a, text b,c]"),
            (1009, -1, br#"{"","NULL"}"#, "array [text , text NULL]"),
            (1009, -1, br#"{"a\"b\\c"}"#, r#"array [text a"b\c]"#),
            (
                1007,
                -1,
                b"{{1,2},{3,4}}",
                "array [array [int 1, int 2], array [int 3, int 4]]",
            ),
            (1007, -1, b"[0:1]={1,2}", "array [int 1, int 2]"),
            (1001, -1, br#"{"\\x00ff"}"#, "array [bytes [0, 255]]"),
            (1561, 1, b"{1,0}", "array [bool true, bool false]"),
            (1182, -1, b"{2024-01-02}", "array [date 2024-01-02]"),
            (1017, -1, br#"{"(1,2)"}"#, r#"text {"(1,2)"}"#),
        ];

        for &(type_oid, type_modifier, body, expected) in cases {
            let subject = column(type_oid, type_modifier);
            let decoded = subject
                .scalar(Some(body))
                .unwrap_or_else(|error| panic!("OID {type_oid} did not decode: {error}"));

            assert_eq!(rendered(&decoded), expected, "OID {type_oid}");
        }
    }

    /// An array of § 9's structured rows is `nvs-stdlib`'s to finish whole:
    /// `decode` answers `None` for it exactly as it does for a single one.
    ///
    /// The nested case is the one that matters. The answer is decided before
    /// anything is allocated, so a two-dimensional `date[]` cannot leave a
    /// half-built inner array behind — and this crate could not free one, for
    /// the reason `PgScalar`'s own doc gives.
    #[test]
    fn an_array_of_structured_rows_has_no_value_either() {
        for body in [b"{2024-01-02}".as_slice(), b"{{2024-01-02}}".as_slice()] {
            assert!(
                column(1182, -1)
                    .decode(Some(body))
                    .expect("a date array the server could render")
                    .is_none(),
                "an array of a structured row was allocated by the driver"
            );
        }
    }

    /// A structured row has no `Value` a driver can build, and `decode` says
    /// so rather than approximating one.
    ///
    /// The two absences are the point: `Ok(None)` is "this row is `scalar`'s
    /// to answer and `nvs-stdlib`'s to finish", and SQL `NULL` on the same
    /// column is still `Some(null)`. A `decode` that collapsed them would make
    /// every timestamp column read as empty.
    #[test]
    fn a_structured_row_has_no_value_and_a_null_one_still_does() {
        let subject = column(oid::TIMESTAMPTZ, -1);

        assert!(
            subject
                .decode(Some(b"2024-01-02 03:04:05+00"))
                .expect("a timestamptz the server could render")
                .is_none(),
            "a structured row was allocated by the driver"
        );
        assert!(
            subject
                .decode(None)
                .expect("a null body was refused")
                .expect("SQL NULL is a value every column has")
                .as_int()
                .is_none()
        );
    }

    /// § 9's `NULL` row does not depend on the column, which is what makes
    /// every column `?T`. A `0` for an integer column is the one wrong answer
    /// that would still look right on the line that printed it.
    #[test]
    fn an_absent_body_is_null_whatever_the_column_holds() {
        for (type_oid, type_modifier) in [
            (oid::BOOL, -1),
            (oid::INT8, -1),
            (oid::NUMERIC, -1),
            (oid::BYTEA, -1),
            (oid::BIT, 1),
            (oid::TIMESTAMPTZ, -1),
            (oid::UUID, -1),
            (25, -1),
        ] {
            let subject = column(type_oid, type_modifier);

            let scalar = subject.scalar(None).expect("a null body was refused");
            assert_eq!(rendered(&scalar), "null", "OID {type_oid}");
            assert_eq!(
                subject
                    .decode(None)
                    .expect("a null body was refused")
                    .expect("SQL NULL is a value every column has")
                    .as_int(),
                None,
                "OID {type_oid}"
            );
        }
    }

    /// A body its column cannot hold is refused rather than approximated, and
    /// the refusal does not quote it.
    ///
    /// Both halves matter. The first is § 9 read strictly: a `NUMERIC` past
    /// what `rule:types/decimal`'s `decimal` holds, or a `NaN` in one, has no value to
    /// answer with and inventing the nearest one would be a wrong number that
    /// nothing downstream could detect. The second is `PgColumn::malformed`'s
    /// rule — the value stays out of the message — and a test is the only
    /// thing that can hold it.
    #[test]
    fn a_body_its_column_cannot_hold_is_refused_without_quoting_it() {
        let cases: &[(Oid, i32, &[u8])] = &[
            (oid::BOOL, -1, b"true"),
            (oid::BIT, 1, b"9"),
            (oid::INT4, -1, b"1.5"),
            (oid::INT8, -1, b"99999999999999999999"),
            (oid::OID, -1, b"-1"),
            (oid::FLOAT8, -1, b"one"),
            (
                oid::NUMERIC,
                -1,
                b"123456789012345678901234567890123456789.9",
            ),
            (oid::NUMERIC, -1, b"NaN"),
            (oid::MONEY, -1, b"$"),
            (oid::BYTEA, -1, br"\xzz"),
            (oid::BYTEA, -1, br"\x0"),
            (oid::BYTEA, -1, br"\9"),
            (oid::DATE, -1, b"infinity"),
            (oid::DATE, -1, b"2024-13-02"),
            (oid::DATE, -1, b"01/02/2024"),
            (oid::TIME, -1, b"03:04"),
            (oid::TIME, -1, b"25:00:00"),
            (oid::TIME, -1, b"03:04:05.1234567890"),
            (oid::TIMESTAMP, -1, b"2024-01-02T03:04:05"),
            (oid::TIMESTAMPTZ, -1, b"2024-01-02 03:04:05"),
            (oid::UUID, -1, b"a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a1"),
            (oid::UUID, -1, b"a0eebc999c0b4ef8bb6d6bb9bd380a11xxxx"),
            (25, -1, b"\xff\xfe"),
            // § 9's array row, whose refusals are the rendering's own: an
            // unclosed brace or quote, an empty element, anything after the
            // outer brace closed, an element its element type refuses, and a
            // nesting past PostgreSQL's own six dimensions.
            (1007, -1, b"{1,2"),
            (1007, -1, b"{1,2}}"),
            (1007, -1, b"{,}"),
            (1007, -1, b"{x}"),
            (1009, -1, b"{\"a}"),
            (1007, -1, b"{{{{{{{{1}}}}}}}}"),
        ];

        for &(type_oid, type_modifier, body) in cases {
            let subject = column(type_oid, type_modifier);

            let refused = subject
                .scalar(Some(body))
                .err()
                .unwrap_or_else(|| panic!("OID {type_oid} decoded {body:?}"));

            assert_eq!(refused.kind(), io::ErrorKind::InvalidData, "OID {type_oid}");
            let said = refused.to_string();
            let quoted = String::from_utf8_lossy(body).into_owned();
            assert!(
                !said.contains(&quoted),
                "OID {type_oid} quoted the body: {said}"
            );
        }
    }

    /// The map through the constructor its callers use, for every row that
    /// allocates nothing.
    ///
    /// The `string` and `bytes` rows are asserted above through `scalar`
    /// instead: `Value::release` is `unsafe` and this crate forbids it, so a
    /// test that built one of those here could not free it, and all it would
    /// be asserting past what `scalar` already says is `NvsStr::new`.
    #[test]
    fn a_decoded_value_carries_the_scalar_the_table_names() {
        // Every column here is a scalar row, so the second `expect` is the
        // seam rather than the value: a `None` would mean § 9 had put this
        // type in the structured half.
        let value = |type_oid: Oid, body: Option<&[u8]>| {
            column(type_oid, -1)
                .decode(body)
                .expect("a body the column could hold")
                .expect("a scalar row is a value this crate builds")
        };

        let int = value(oid::INT4, Some(b"7"));
        let uint = value(oid::OID, Some(b"7"));
        let boolean = value(oid::BOOL, Some(b"t"));
        let float = value(oid::FLOAT8, Some(b"0.5"));
        let decimal = value(oid::NUMERIC, Some(b"1.25"));
        let absent = value(oid::INT4, None);

        assert_eq!(int.as_int(), Some(7));
        assert_eq!(uint.as_uint(), Some(7));
        assert_eq!(boolean.as_bool(), Some(true));
        assert_eq!(float.as_float(), Some(0.5));
        assert_eq!(
            decimal.as_decimal().map(|value| value.to_string()),
            Some("1.25".to_owned())
        );
        assert_eq!(absent.as_int(), None);
    }
}
