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

use std::cell::Cell;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::time::Instant;

use bytes::BytesMut;
use fallible_iterator::FallibleIterator;
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;
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
}

impl std::fmt::Debug for PgTarget<'_> {
    /// Everything but the password, which is a `secret` at the language level
    /// (ADR 0067 § 3) and is not printed in any rendering.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgTarget")
            .field("host", &self.host)
            .field("user", &self.user)
            .field("database", &self.database)
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
            // ADR 0067 § 1 sizes this by `statement_cache` in the connection's
            // config block; nothing opens a connection *from* config yet, so
            // the driver's own default stands until the connect path reads one.
            cache: StatementCache::new(StatementCache::DEFAULT_CAPACITY),
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
    frontend::startup_message(
        [
            ("user", target.user),
            ("database", target.database),
            // ADR 0067 § 3's third default: text columns arrive as valid UTF-8
            // by construction rather than by inspection, which is what ADR
            // 0009's guarantee needs. The zone-less-`DATETIME` half of that
            // section (`TimeZone`) is § 9's and arrives with the type map.
            ("client_encoding", "UTF8"),
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
            // The server's own settings, echoed. Nothing is read off them yet;
            // the type map (ADR 0067 § 9) is what will want `TimeZone`.
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
/// [ADR 0067 § 9](../../../docs/adr/0067-core-db.md)'s type map reads
/// [`PgColumn::type_oid`] to decide which Novis type a column's bytes become,
/// so this is deliberately the server's own OID and not a Novis type yet: the
/// mapping is one table in one place, and it is not this module's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgColumn {
    /// The column's label, as the server wrote it in the row description.
    pub name: String,
    /// The PostgreSQL type OID of the column's values.
    pub type_oid: Oid,
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
    /// This is where the affected-row count ADR 0067 § 4's `execute` answers
    /// with comes from, and parsing it is that slice's job: the tag's shape is
    /// per command and the driver hands back what the server said.
    #[must_use]
    pub fn command_tag(&self) -> Option<&str> {
        self.tag.as_deref()
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

    use super::{CancelKey, PgTarget, State, Wire, authenticate, request_tls, start_statement};
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
        }
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
        // wire before anything can arrive in another encoding.
        let startup = &wire.peer().sent[0];
        assert!(
            startup.windows(20).any(|w| w == b"client_encoding\0UTF8"),
            "the startup message did not force UTF-8"
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
}
