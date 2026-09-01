//! PostgreSQL: opening a socket, upgrading it in band, authenticating, and
//! running one statement at a time over the extended-query protocol.
//!
//! [ADR 0132 § 2](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)'s
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
//!    § 3](../../../docs/adr/0067-core-db.md) makes `VerifyFull` the default
//!    with no spelling for turning it off, and PHP's `sslmode=prefer`
//!    silently connecting in the clear when the server says so is the exact
//!    behaviour that rule exists to remove.
//! 2. **The handshake**, `NvsTls::over` on the same `NvsTcp`. One TLS client in
//!    this tree, one answer to whose certificates it believes (ADR 0132 § 3).
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
//! [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md). It is filled
//! through a `resize` and a `truncate` rather than into uninitialised spare
//! capacity, because this crate forbids `unsafe`: that costs one `memset` of
//! the read window per syscall and buys the whole file having no unsafe block
//! to review.
//!
//! # A statement is one round trip, and the wire's state is the whole of it
//!
//! [`PgConn::query`] writes `Parse`, `Bind`, `Describe`, `Execute` and `Sync`
//! into one buffer and flushes them once. That is [ADR 0067
//! § 1](../../../docs/adr/0067-core-db.md)'s "PostgreSQL's extended protocol
//! pays nothing extra" made literal: there is no prepare round trip to save,
//! because the prepare travels with the execution.
//!
//! The `Sync` at the end is what makes ADR 0132 § 4's per-driver call come out
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
//! # Parameters and results are in text format
//!
//! Both format lists in `Bind` are empty, which is the protocol's spelling for
//! "everything in text". [ADR 0067 § 9](../../../docs/adr/0067-core-db.md)'s
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
//! [ADR 0009](../../../docs/adr/0009-string-and-bytes.md)'s promise is read
//! *unchecked* downstream — `nvs_runtime::NvsStr::text_of` is that crate's one
//! unchecked read — and a database server is a network peer rather than a part
//! of this process: an ill-formed body from a compromised or simply
//! misconfigured server would be undefined behaviour several calls later, in a
//! crate that never saw a wire. The check is one pass over bytes already in
//! cache, and it is the only thing this driver spends on a value it is not
//! otherwise parsing.

use std::cell::Cell;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::time::Instant;

use bytes::BytesMut;
use fallible_iterator::FallibleIterator;
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;
use nvs_runtime::{Decimal, NvsStr, Value};
use postgres_protocol::authentication::sasl::{ChannelBinding, ScramSha256};
use postgres_protocol::message::{backend, frontend};
use postgres_protocol::{IsNull, Oid};

use crate::conn::{PgConn, State};
use crate::sql::{Prepared, StatementCache};

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
/// 0058](../../../docs/adr/0058-outbound-request-policy.md)'s pinned address,
/// already resolved by whoever checked the capability, and `host` is the name
/// the certificate must be valid for. A driver that re-resolved the name would
/// be connecting somewhere nobody approved.
pub struct PgTarget<'a> {
    /// The name the server's certificate is checked against.
    pub host: &'a str,
    /// The role to log in as. ADR 0067 § 3 accepts `tainted` here freely: it is
    /// a length-prefixed protocol field, never parsed text.
    pub user: &'a str,
    /// The role's password, used only to derive a SCRAM proof.
    pub password: &'a str,
    /// The database to attach to.
    pub database: &'a str,
    /// The zone a zone-less `TIMESTAMP` column is read in, as a whole number
    /// of seconds east of UTC.
    ///
    /// [ADR 0067 § 9](../../../docs/adr/0067-core-db.md)'s declared zone:
    /// `time_zone` in the connection's config block, `timeZone` in `Settings`,
    /// and `0` — UTC — with neither set. It is sent to the server as well as
    /// read here, so `CURRENT_TIMESTAMP` and a decoded column agree about
    /// which zone they are in.
    ///
    /// [`crate::sql::time_zone_for`] is what turns the written field into this
    /// number, and it is the same reader for all five drivers: what arrives
    /// here is already seconds, so this path never sees a zone name.
    pub time_zone: i32,
    /// How many prepared statements this connection may keep alive on the
    /// server, [ADR 0067 § 1](../../../docs/adr/0067-core-db.md)'s
    /// `statement_cache`.
    ///
    /// Resolved the same way the zone is, and by the same rule about who
    /// decides: [`StatementCache::capacity_for`] reads the `[db.<name>]` block
    /// and answers a number, so the connect path below never sees an absent
    /// field. `0` is the cache off, which is a supported size and not a
    /// caller that forgot to fill this in.
    pub statement_cache: usize,
}

impl std::fmt::Debug for PgTarget<'_> {
    /// Everything but the password, which is a `secret` at the language level
    /// (ADR 0067 § 3) and is not printed in any rendering.
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

/// A PostgreSQL connection's stream, and what has been read off it.
///
/// Generic in the stream so the exchanges above can be asserted against a
/// server that answers rather than a socket; a real connection's wire is the
/// `NvsTls<NvsTcp>` the default names, and nothing constructs any other outside
/// this module's tests.
pub(crate) struct Wire<S: Read + Write = NvsTls<NvsTcp>> {
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

    /// Writes everything buffered in `out`, empties it, and flushes.
    ///
    /// One function because a half-written message is the shape ADR 0132 § 4
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
    /// Opens a connection: TCP, § 3's in-band upgrade, TLS, then SASL.
    ///
    /// `deadline` bounds the whole of that and not one leg of it — the connect,
    /// the handshake and every authentication round trip share one clock,
    /// filed on the socket where `nvs-host` keeps it. `None` is unbounded and
    /// is for a caller that has its own bound.
    ///
    /// The connection comes back [`State::Idle`]: at a message boundary, with
    /// a statement allowed.
    ///
    /// # Errors
    ///
    /// `ConnectionRefused` when the server will not upgrade to TLS or offers no
    /// SASL mechanism this driver accepts, `InvalidData` for a message that is
    /// not what the protocol allows at that point, `TimedOut` when the deadline
    /// passes, and whatever the socket or the TLS handshake itself reported. A
    /// refusal the *server* worded — a wrong password, a database that does not
    /// exist — carries its own `SQLSTATE` and message.
    pub fn connect(
        addr: SocketAddr,
        target: &PgTarget<'_>,
        deadline: Option<Instant>,
    ) -> io::Result<PgConn> {
        let mut tcp = match deadline {
            Some(at) => {
                NvsTcp::connect_timeout(addr, at.saturating_duration_since(Instant::now()))?
            }
            None => NvsTcp::connect(addr)?,
        };
        tcp.set_deadline(deadline);

        request_tls(&mut tcp)?;
        let mut wire = Wire::new(NvsTls::over(tcp, target.host)?);
        let cancel = authenticate(&mut wire, target)?;

        Ok(PgConn {
            wire,
            state: Cell::new(State::Idle),
            cancel,
            // ADR 0067 § 1's size, already read off the `[db.<name>]` block by
            // `StatementCache::capacity_for` and carried here on the target —
            // this path takes a number and has no opinion about where an
            // unwritten field's default comes from.
            cache: StatementCache::new(target.statement_cache),
        })
    }

    /// What a second connection needs to cancel this one's running statement.
    #[must_use]
    pub fn cancel_key(&self) -> CancelKey {
        self.cancel
    }

    /// Runs one statement, borrowing the connection until its rows are drained.
    ///
    /// `params` are ADR 0067 § 5's bound values, each already rendered in the
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
    /// `InvalidInput` when the connection is not [`State::Idle`] — ADR 0067
    /// § 4's second concurrent statement, which `nvs-stdlib` words as a
    /// `LogicError`. Otherwise the server's own error, which leaves the
    /// connection idle and poolable, or a wire failure, which poisons it.
    pub fn query(&mut self, sql: &str, params: &[Option<&[u8]>]) -> io::Result<PgRows<'_>> {
        start_statement(&mut self.wire, &self.state, &mut self.cache, sql, params)
    }

    /// ADR 0067 § 13's reset, and the connection back only if it worked.
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
        Ok(self)
    }
}

impl Drop for PgConn {
    /// Says goodbye rather than vanishing.
    ///
    /// `Terminate` lets the backend exit on its own instead of discovering a
    /// reset socket, which is one fewer error line in the server's log per
    /// connection and one backend freed a round trip earlier. Best effort by
    /// definition: the connection is going away whatever the write reports, so
    /// there is nobody left to tell.
    fn drop(&mut self) {
        let mut out = BytesMut::new();
        frontend::terminate(&mut out);
        drop(self.wire.send(&mut out));
    }
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
            "the server offers no TLS, and ADR 0067 § 3's VerifyFull has no spelling for connecting \
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
/// [ADR 0067 § 9](../../../docs/adr/0067-core-db.md) requires a numeric offset
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
fn authenticate<S: Read + Write>(
    wire: &mut Wire<S>,
    target: &PgTarget<'_>,
) -> io::Result<CancelKey> {
    let mut out = BytesMut::new();
    let time_zone = posix_time_zone(target.time_zone);
    frontend::startup_message(
        [
            ("user", target.user),
            ("database", target.database),
            // ADR 0067 § 3's third default: text columns arrive as valid UTF-8
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
            // The server's own settings, echoed, and nothing is read off them:
            // the two the type map depends on, `DateStyle` and `TimeZone`, are
            // startup parameters this driver set, so reading them back would
            // only be asking whether the server agreed with itself.
            backend::Message::ParameterStatus(_) | backend::Message::NoticeResponse(_) => {}
            backend::Message::ErrorResponse(body) => return Err(server_error(&body)),
            backend::Message::ReadyForQuery(_) => {
                return cancel.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "the server completed startup without a BackendKeyData, so this connection \
                         has no cancellation key and its statements could not be bounded",
                    )
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

/// An `ErrorResponse` in the words the server used.
///
/// Severity, `SQLSTATE` and message, which are the three fields an operator
/// acts on; the rest (position, hint, the source line in the server's own C)
/// are dropped rather than rendered into a sentence nobody reads. A field that
/// does not decode ends the walk, so a malformed error is still an error.
fn server_error(body: &backend::ErrorResponseBody) -> io::Error {
    let mut severity = String::from("ERROR");
    let mut code = String::from("XX000");
    let mut message = String::new();

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
                    _ => {}
                }
            }
            Ok(None) => break,
            Err(e) => return e,
        }
    }

    io::Error::other(format!("postgres {severity}: {message} (SQLSTATE {code})"))
}

/// The unnamed statement and the unnamed portal, which is what an uncached
/// execution uses.
///
/// PostgreSQL destroys the unnamed portal at the next `Sync`, so nothing
/// accumulates on the server between calls and there is nothing to deallocate.
/// The *statement* is named by [`StatementCache`] whenever ADR 0067 § 1's cache
/// is on, and this remains the spelling of the unnamed one — which is what a
/// capacity of zero, and only that, still sends.
const UNNAMED: &str = "";

/// One column of a portal's row description.
///
/// The fields are the server's own — an OID and a type modifier, never a Novis
/// type — because a row description is what the server said rather than what
/// this driver made of it. [`PgColumn::decode`] is where [ADR 0067
/// § 9](../../../docs/adr/0067-core-db.md)'s type map turns the pair into a
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
/// slice of the message body, which the row owns. ADR 0067 § 9's decoders are
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

/// The `pg_type` OIDs [ADR 0067 § 9](../../../docs/adr/0067-core-db.md)'s table
/// names, spelled as PostgreSQL numbers them.
///
/// A built-in type's OID is bootstrap data — fixed in `pg_type.dat` and the
/// same on every server of every version — which is what makes a literal table
/// legitimate here rather than a `SELECT` against the catalog at connect time.
/// `postgres-protocol` is framing and vends none of them.
///
/// Only the types that are **not** text are named. § 9's last row sends
/// everything without a Novis type to `tainted string` as the server rendered
/// it, and `text`, `varchar`, `json`, `inet`, `interval` and the rest arrive
/// there by simply not being in this list.
mod oid {
    use postgres_protocol::Oid;

    /// `BOOLEAN`.
    pub(super) const BOOL: Oid = 16;
    /// `BYTEA`.
    pub(super) const BYTEA: Oid = 17;
    /// `BIGINT`.
    pub(super) const INT8: Oid = 20;
    /// `SMALLINT`.
    pub(super) const INT2: Oid = 21;
    /// `INTEGER`.
    pub(super) const INT4: Oid = 23;
    /// `oid`, the only unsigned integer a query is likely to select.
    pub(super) const OID: Oid = 26;
    /// `REAL`.
    pub(super) const FLOAT4: Oid = 700;
    /// `DOUBLE PRECISION`.
    pub(super) const FLOAT8: Oid = 701;
    /// `money`.
    pub(super) const MONEY: Oid = 790;
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
/// It is also this driver's public answer beside a [`Value`], because five of
/// § 9's rows are not values at all: a `DATE` is a `Core\Time\Date`, an
/// *instance* of an `nvs-stdlib` class, and that crate is the only one that
/// can allocate one. Those five arrive here as parsed components — see
/// [`PgDate`] — and `into_value` answers `None` for them.
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
    Text(&'a str),
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
        })
    }
}

impl PgScalar<'_> {
    /// The Novis value, taking on the one reference a `string` or a `bytes`
    /// costs and nothing at all for the rest.
    ///
    /// `None` for § 9's five structured rows, whose Novis type is a class
    /// instance this crate cannot allocate at all — [`PgDate`] owns why. A
    /// caller that wants the whole table matches those five variants first and
    /// reaches this for everything left.
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
        })
    }
}

impl PgColumn {
    /// This column's `body` as the Novis value [ADR 0067
    /// § 9](../../../docs/adr/0067-core-db.md)'s table names, with `None` — SQL
    /// `NULL` — as `null`, which is why every column reads back as `?T`.
    ///
    /// `body` is what [`PgRow::column`] handed back, still in the text format
    /// the module doc chose.
    ///
    /// `Ok(None)` is the *other* absence, and the two never collide: it says
    /// the column is one of § 9's structured rows — `DATE`, `TIME`,
    /// `TIMESTAMP`, `TIMESTAMPTZ`, `UUID` — whose Novis type is a class
    /// instance no driver can allocate. [`Self::scalar`] is what reads those,
    /// and this is the scalar half in full. PostgreSQL's array types are not
    /// decoded at all yet and still fall to the table's last row, arriving as
    /// the server's own rendering.
    ///
    /// The `tainted` half of `tainted string` is nowhere in this signature and
    /// is not missing.
    /// [ADR 0024](../../../docs/adr/0024-taint-tracking-for-injection-sinks.md)'s
    /// qualifier is a property of the *type* a row is read at, declared in
    /// `nvs-stdlib`'s registry rows; there is no runtime bit for it, and a
    /// driver could not set one if there were.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a body the column's own type cannot be read out of: a
    /// `NUMERIC` past what
    /// [ADR 0054](../../../docs/adr/0054-decimal-scalar-type.md)'s `decimal`
    /// holds or a `NaN` in one, which that type has no representation for; a
    /// text body that is not UTF-8; a malformed `bytea`. Every such message
    /// names the column and its OID and **never the body**, for the reason
    /// [`Self::malformed`] gives.
    pub fn decode(&self, body: Option<&[u8]>) -> io::Result<Option<Value>> {
        Ok(self.scalar(body)?.into_value())
    }

    /// [`PgColumn::decode`]'s whole decision, before anything is allocated,
    /// and every row of § 9's table rather than the scalar half.
    ///
    /// `nvs-stdlib` calls this one: it is the only crate that can turn a
    /// [`PgScalar::Date`] and its four siblings into the `Core\Time` and
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
            oid::NUMERIC => PgScalar::Decimal(
                Decimal::parse(self.text(body)?).ok_or_else(|| self.malformed("a decimal"))?,
            ),
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
            _ => PgScalar::Text(self.text(body)?),
        })
    }

    /// The body as text, **checked**.
    ///
    /// The module doc's § *Parameters and results are in text format* owns why
    /// the check is here rather than trusted from `client_encoding`.
    fn text<'a>(&self, body: &'a [u8]) -> io::Result<&'a str> {
        std::str::from_utf8(body).map_err(|_| self.malformed("well-formed UTF-8"))
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

        Decimal::parse(&digits).ok_or_else(|| self.malformed("a decimal"))
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

/// A statement's result stream, and the connection it is borrowed from.
///
/// Alive, this is [`State::Streaming`]: the wire holds messages belonging to
/// this statement, and ADR 0067 § 4 refuses a second one. Drained by
/// [`PgRows::next_row`] or dropped, it is [`State::Idle`] again — the module doc
/// owns why abandonment is ordinary here rather than poison.
///
/// Generic in the stream for the same reason [`Wire`] is: the sequencing below
/// is then assertable against a scripted server, with no socket and no
/// certificate. A connection's own rows are always over the default.
pub struct PgRows<'a, S: Read + Write = NvsTls<NvsTcp>> {
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    columns: Vec<PgColumn>,
    tag: Option<String>,
}

impl<S: Read + Write> std::fmt::Debug for PgRows<'_, S> {
    /// The shape of the result and where the wire is, and nothing that arrived.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgRows")
            .field("columns", &self.columns.len())
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

impl<S: Read + Write> PgRows<'_, S> {
    /// What the portal said its rows look like, empty for a statement that
    /// returns none.
    #[must_use]
    pub fn columns(&self) -> &[PgColumn] {
        &self.columns
    }

    /// The server's `CommandComplete` tag — `INSERT 0 3`, `SELECT 2` — once the
    /// stream has ended, and `None` while rows may still arrive.
    ///
    /// The raw tag, because it says more than a count does: [`Self::affected`]
    /// is the number [ADR 0067 § 4](../../../docs/adr/0067-core-db.md)'s
    /// `execute` answers with, and this is what an error message quotes when a
    /// statement did something other than what its caller expected.
    #[must_use]
    pub fn command_tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    /// [ADR 0067 § 4](../../../docs/adr/0067-core-db.md)'s affected-row count,
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
        affected_rows(self.tag.as_deref()?)
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
        // The state is the only bookkeeping: anything that ended this stream —
        // a completion, a server error, a poisoning — has already left it.
        if self.state.get() != State::Streaming {
            return Ok(None);
        }

        loop {
            match read_or_poison(self.wire, self.state)? {
                backend::Message::DataRow(body) => return Ok(Some(PgRow { body })),
                backend::Message::CommandComplete(body) => {
                    let tag = body
                        .tag()
                        .inspect_err(|_| self.state.set(State::Poisoned))?
                        .to_owned();
                    self.tag = Some(tag);
                    drain_to_ready(self.wire, self.state)?;
                    return Ok(None);
                }
                // `Bind` on an empty query string. Not an error: it is what a
                // caller that built its SQL from an empty template sent.
                backend::Message::EmptyQueryResponse => {
                    drain_to_ready(self.wire, self.state)?;
                    return Ok(None);
                }
                backend::Message::ErrorResponse(body) => {
                    let error = server_error(&body);
                    drain_to_ready(self.wire, self.state)?;
                    return Err(error);
                }
                backend::Message::NoticeResponse(_)
                | backend::Message::ParameterStatus(_)
                | backend::Message::NotificationResponse(_) => {}
                _ => return Err(out_of_sequence(self.state)),
            }
        }
    }
}

impl<S: Read + Write> Drop for PgRows<'_, S> {
    /// Abandonment, and it is the ordinary case rather than an error.
    ///
    /// The `Sync` went out with the statement, so the `ReadyForQuery` that ends
    /// it is coming whether or not anybody read the rows in between: draining
    /// to it is deterministic, and ADR 0132 § 4 is explicit that a driver which
    /// can do that returns the connection to the pool instead of closing it.
    /// A read that fails on the way poisons it, through the same helper every
    /// other read here uses.
    fn drop(&mut self) {
        if self.state.get() == State::Streaming {
            drop(drain_to_ready(self.wire, self.state));
        }
    }
}

/// Writes ADR 0067 § 1's one round trip and reads up to the first row.
///
/// `Parse`, `Bind`, `Describe`, `Execute` and `Sync` go into one buffer and out
/// in one flush, and this returns once the portal has described itself — which
/// is the point rows may start arriving and [`State::Streaming`] is true.
///
/// `cache` is what decides whether the `Parse` is in that buffer at all. On a
/// hit it is not — the round trip ADR 0067 § 1 says PostgreSQL already does not
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
fn start_statement<'a, S: Read + Write>(
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    cache: &mut StatementCache,
    sql: &str,
    params: &[Option<&[u8]>],
) -> io::Result<PgRows<'a, S>> {
    if !state.get().may_start_statement() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "a statement was written to a connection that is {:?}, and ADR 0067 § 4 allows one \
                 at a time: read the previous statement's rows or drop its handle first",
                state.get()
            ),
        ));
    }

    // The arity is the parameter count, which ADR 0067 § 5's `inList` expansion
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
        prepared.name(),
        UNNAMED,
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
    Ok(PgRows {
        wire,
        state,
        columns,
        tag: None,
    })
}

/// ADR 0067 § 13's PostgreSQL reset, in the order that section lists it.
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

/// Reads one message, and poisons the connection if the wire itself failed.
///
/// Every read in the extended-query path goes through this, because the rule is
/// one rule: a message that did not arrive leaves no boundary to resume from,
/// and ADR 0132 § 4 closes such a connection rather than resetting it.
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

    use super::{
        CancelKey, PgColumn, PgDate, PgScalar, PgTarget, PgTime, State, Wire, affected_rows,
        authenticate, oid, posix_time_zone, request_tls, start_statement,
    };
    use crate::sql::StatementCache;

    /// A cache that never caches, so a test about the wire asserts the unnamed
    /// statement it has always asserted. The cached path has its own case.
    fn no_cache() -> StatementCache {
        StatementCache::new(0)
    }

    /// A server that answers the client rather than a script: the SASL
    /// exchange's every message depends on the one before it, so a canned
    /// transcript could only ever assert that we send *something*.
    ///
    /// A flush is the message boundary — every write in this module is
    /// `write_all` then `flush` — so the closure is handed exactly what the
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
        out.extend_from_slice(&message(b'Z', b"I"));
        out
    }

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
            let client_nonce = self
                .client_first_bare
                .rsplit_once("r=")
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

    fn target<'a>(password: &'a str) -> PgTarget<'a> {
        PgTarget {
            host: "postgres.test",
            user: "novis",
            password,
            database: "novis_test",
            // Deliberately not UTC: a zone this driver sends is visible in the
            // startup message only when it is not the default of every field
            // around it.
            time_zone: 2 * 3600,
            statement_cache: StatementCache::DEFAULT_CAPACITY,
        }
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

    /// The upgrade request is exactly ADR 0132 § 3's eight bytes, and `S` is
    /// the answer that continues. Asserted on the bytes rather than on the
    /// outcome, because those eight are the only plaintext a connection ever
    /// carries and a wrong request code reads as a plain startup message.
    #[test]
    fn an_ssl_request_is_the_first_thing_on_the_wire_and_s_is_the_yes() {
        let mut peer = Peer::new(|_: &[u8]| b"S".to_vec());
        request_tls(&mut peer).expect("S is the server agreeing");
        assert_eq!(peer.sent, vec![vec![0, 0, 0, 8, 0x04, 0xd2, 0x16, 0x2f]]);
    }

    /// The refusal ADR 0067 § 3 is written for: a server with no TLS is not
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

        let key =
            authenticate(&mut wire, &target("Novis-Test-Pw1")).expect("the exchange completed");

        assert_eq!(
            key,
            CancelKey {
                process_id: 4242,
                secret_key: 99
            }
        );
        // ADR 0067 § 3's forced charset is a startup parameter, so it is on the
        // wire before anything can arrive in another encoding. § 9's two are
        // there for the same reason: a text rendering this driver parses
        // positionally has to be the rendering it asked for, not the one the
        // server was configured with.
        let startup = &wire.peer().sent[0];
        assert!(
            startup.windows(20).any(|w| w == b"client_encoding\0UTF8"),
            "the startup message did not force UTF-8"
        );
        assert!(
            startup.windows(14).any(|w| w == b"DateStyle\0ISO\0"),
            "the startup message did not pin the date rendering"
        );
        assert!(
            startup.windows(18).any(|w| w == b"TimeZone\0<+02>-02\0"),
            "the startup message did not declare the connection's zone"
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

    /// An `ErrorResponse` carrying the three fields [`super::server_error`]
    /// reads, and the trailing zero that ends the field list.
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

    /// ADR 0067 § 1's cache on the wire: the round trip PostgreSQL stops paying
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

    /// ADR 0067 § 1's "PostgreSQL's extended protocol pays nothing extra",
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

    /// The four values of ADR 0132 § 4, walked by one statement: `Idle` before,
    /// `Streaming` while rows are unread, and `Idle` again once the
    /// `ReadyForQuery` the `Sync` guaranteed has been read. A `NULL` column is
    /// `None` and not an empty slice — the distinction ADR 0067 § 9 maps to
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

    /// ADR 0067 § 4's refusal, and the half of it that matters is that
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

    /// ADR 0132 § 4's per-driver call, in PostgreSQL's favour: a caller that
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

    /// ADR 0067 § 13's list, in its order, in **one** flush — six commands for
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

    /// A column of one type and width, which is all a decode reads.
    fn column(type_oid: Oid, type_modifier: i32) -> PgColumn {
        PgColumn {
            name: "c".to_owned(),
            type_oid,
            type_modifier,
        }
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

    /// ADR 0067 § 9's scalar rows, PostgreSQL's half of them: the whole table
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

    /// § 9's structured rows: the five whose Novis type is a class instance,
    /// and so are components here rather than a value.
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
    /// what ADR 0054's `decimal` holds, or a `NaN` in one, has no value to
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
