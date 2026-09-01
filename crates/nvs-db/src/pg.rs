//! PostgreSQL: opening a socket, upgrading it in band, and authenticating.
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

use crate::conn::{PgConn, State};

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
        })
    }

    /// What a second connection needs to cancel this one's running statement.
    #[must_use]
    pub fn cancel_key(&self) -> CancelKey {
        self.cancel
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

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    use std::io::{self, Read, Write};

    use super::{CancelKey, PgTarget, Wire, authenticate, request_tls};

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
}
