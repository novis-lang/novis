//! TLS over the parking stream: a `rustls` client session on top of
//! [`NvsTcp`], and the trust anchor set it verifies against.
//!
//! [`crate::net`] promises that an unmodified protocol implementation runs over
//! its stream, and its `a_rustls_session_streams_over_it_unmodified` has proven
//! that for TLS since ADR 0115 § 3 was written. What was missing to make that
//! usable was never the transport — it was a **client**, and a client is a
//! session plus an answer to "whose certificates do you believe". This module
//! is those two things and nothing else: [`NvsTls::over`] takes a connected
//! stream, completes a handshake on it, and hands back a plaintext
//! `Read`/`Write` that parks exactly like the stream underneath it.
//!
//! Nothing here knows about HTTP or SMTP. `Core\Http\Client` reaches it for an
//! `https` URL and `Core\Mail` reaches it for `STARTTLS`, and both of those are
//! the same three lines, because a protocol that was written against a socket
//! is written against this too.
//!
//! # The transport is generic, and `NvsTcp` is its default
//!
//! [`NvsTls`] is `NvsTls<T: Read + Write>` and `NvsTls` on its own still means
//! `NvsTls<NvsTcp>`, which is what every caller in the tree writes. The
//! parameter exists for one shape the socket cannot express: a handshake
//! **tunnelled inside another protocol's framing**, where the bytes `rustls`
//! produces are not the bytes that go on the wire. SQL Server is that case — it
//! wraps handshake records in TDS `PRELOGIN` packets — and the adapter that
//! reconciles the two is an ordinary `Read`/`Write` in `nvs-db` rather than a
//! second TLS client
//! ([ADR 0132 § 3](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)).
//! That is the whole reason to generalise rather than to let a driver build its
//! own session: the anchors below, the protocol versions and the verifier are
//! decided once, here, and a driver cannot widen any of them by construction.
//! What stays on `NvsTls<NvsTcp>` alone is what belongs to the socket rather
//! than to the session — the deadline and the peer address.
//!
//! # The core is still handed back, and this module does nothing to keep it
//!
//! `rustls` drives the socket through `std::io::Read` and `std::io::Write` and
//! has no other route to it, so every wait inside a handshake or a record read
//! is [`crate::net`]'s park: issue the syscall, and on `WouldBlock` register,
//! suspend and loop. That is the whole reason this type is a wrapper rather
//! than a transport — there is no second I/O path here to get wrong, and
//! `a_tls_session_over_the_parking_stream_survives_a_park` asserts a session
//! completing across at least one suspension.
//!
//! The deadline works the same way and for the same reason: it lives on the
//! [`NvsTcp`] ([`NvsTcp::set_deadline`]), so a caller bounds a handshake by
//! setting it *before* [`NvsTls::over`] and bounds the exchange after it by
//! [`NvsTls::set_deadline`]. One clock, on the thing that waits, rather than a
//! second budget this layer would have to keep agreeing with.
//!
//! # The trust anchors are compiled in, and an operator adds to them
//!
//! Novis trusts **Mozilla's CA set, carried in the binary** (`webpki-roots`),
//! and not the host's own certificate store. Three reasons, in this project's
//! priority order:
//!
//! 1. **The same binary trusts the same certificates everywhere.** A platform
//!    store makes the outcome of an `https` call a property of the *host image*
//!    rather than of the deployment, so a call that verified in CI can fail —
//!    or, far worse, succeed against a CA an operator never chose — on a
//!    machine nobody reread. Priority 1 does not survive a trust set that is
//!    invisible from the artifact.
//! 2. **A minimal container has no store at all.** Scratch and distroless
//!    images ship no `ca-certificates`, and a client that read the platform
//!    store would fail every outbound `https` call in exactly the deployment
//!    shape Novis is built for, with a diagnostic about a missing file.
//! 3. **Reading the platform store is a filesystem and registry reach** on a
//!    path with no request behind it, which
//!    [ADR 0118](../../../docs/adr/0118-capabilities-are-checked-at-one-door-per-resource.md)
//!    § 1 would then owe a door and an answer for. A constant in the binary
//!    owes neither.
//!
//! What this gives up is the private CA — an internal PKI, or a corporate
//! inspection proxy — and that is deliberately left to the **operator**, whose
//! decision it is, in the file that already holds every other one
//! ([ADR 0103](../../../docs/adr/0103-configuration-is-a-tree-of-files.md)). A
//! configured anchor bundle is an `nvs.toml` slice that has not landed;
//! [`upgraded`] is already the seam it plugs into. What is closed permanently
//! is a *program* choosing anchors, or turning verification off: neither has a
//! spelling here, and the whole point of ADR 0058's pinned outbound door is
//! that a script does not get to widen a decision the deployment made.
//!
//! What that spends, per [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md):
//! one parsed root store and one `ClientConfig` for the whole **process**, built
//! once on first use and shared by every session after it — roughly 150 trust
//! anchors, a few hundred kilobytes, O(1) in requests served. Per session it is
//! `rustls`'s own connection state, which is O(in-flight) and released with the
//! stream.
//!
//! # `ring` is the provider, and it is spent under ADR 0051 § 4
//!
//! `rustls`'s default provider is `aws-lc-rs`, which is a C library built by
//! `cmake`; it is off. `ring` is the provider instead, and it is not pure Rust
//! either — its primitives are BoringSSL's pregenerated assembly behind a Rust
//! API. So ADR 0051 § 4 applies with its first question answered **yes**: a TLS
//! record layer is where attacker-controlled bytes land, by construction.
//!
//! Question 2 is what accepts it, on the same footing as that section's SQLite:
//! the primitives are BoringSSL's, which is continuously fuzzed under OSS-Fuzz,
//! carries formally verified field arithmetic (fiat-crypto) for the curves it
//! uses, and is the code path a browser ships to a billion machines. That is
//! the "demonstrable, exceptional verification record" the test asks for, and
//! it is a stronger record than any pure-Rust provider can offer today. The
//! section's standing fallback — confine it to wasm — is not available, because
//! a TLS client owns the socket it reads.
//!
//! Everything **above** the primitives, which is where TLS's historical
//! failures actually live — state machine, record framing, certificate path
//! building, name verification — is `rustls` and `rustls-webpki`, pure Rust.

use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use crate::net::NvsTcp;

/// A TLS client session over the parking stream: plaintext in, plaintext out,
/// and every wait underneath it hands the core back.
///
/// Built by [`NvsTls::over`] from a stream that is already connected, because
/// where to connect is [ADR 0058](../../../docs/adr/0058-outbound-request-policy.md)'s
/// pinned address and not a name this layer would re-resolve. The name passed
/// in is what the certificate is checked against, and it is the name the caller
/// was granted — never the address it was pinned to.
pub struct NvsTls<T: Read + Write = NvsTcp> {
    inner: StreamOwned<ClientConnection, T>,
}

impl<T: Read + Write + std::fmt::Debug> std::fmt::Debug for NvsTls<T> {
    /// The socket and the deadline. The session state is `rustls`'s and holds
    /// key material, so it is not printed even in a debug rendering.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NvsTls")
            .field("sock", &self.inner.sock)
            .finish_non_exhaustive()
    }
}

impl<T: Read + Write> NvsTls<T> {
    /// Completes a TLS handshake over `stream`, verifying the peer against the
    /// compiled-in anchors, and hands back the plaintext stream.
    ///
    /// `name` is the DNS name or IP literal the certificate must be valid for.
    /// The handshake runs under whatever deadline `stream` already carries, so
    /// a caller bounding it sets [`NvsTcp::set_deadline`] first — this module's
    /// docs say why the clock lives there and not here.
    ///
    /// Returns once the handshake is complete, so a certificate that does not
    /// verify is an error *here* rather than half a request later.
    ///
    /// `stream` is usually an [`NvsTcp`], and that is the case every caller in
    /// the tree has: a few plaintext bytes of the protocol's own upgrade, then
    /// every subsequent byte is a TLS record on the same socket. It is generic
    /// for the one shape that is not — SQL Server wraps the handshake records
    /// in TDS `PRELOGIN` packets, so during the handshake the bytes `rustls`
    /// produces are not the bytes that go on the socket, and the framer that
    /// reconciles them is an ordinary `Read`/`Write` adapter in `nvs-db`. That
    /// keeps one TLS client and one answer to "whose certificates do you
    /// believe"; a second `rustls` session built inside a driver would be a
    /// second answer to a question this module has already decided at length
    /// ([ADR 0132 § 3](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)).
    ///
    /// # Errors
    ///
    /// `InvalidInput` when `name` is neither a DNS name nor an IP literal,
    /// `InvalidData` when the peer's certificate does not verify against the
    /// anchors or the handshake is otherwise refused, `TimedOut` when the
    /// stream's deadline passed mid-handshake, and whatever the socket itself
    /// reported. `UnexpectedEof` when the peer went away mid-handshake.
    pub fn over(stream: T, name: &str) -> io::Result<Self> {
        upgraded(stream, name, anchors())
    }
}

/// The methods that are the socket's rather than the session's.
///
/// They stay on the `NvsTcp` transport because that is what they are about: a
/// deadline is kept by the thing that waits, and a peer address is a property
/// of a socket. A session over some other transport reaches both through
/// whatever owns the stream underneath its framer, which is where they remain
/// one clock and one address rather than two.
impl NvsTls<NvsTcp> {
    /// Bounds every wait on the session — handshake renegotiation, reads,
    /// writes — by `at`, or lifts the bound.
    ///
    /// Forwarded to the socket, which is the only thing that waits.
    pub fn set_deadline(&mut self, at: Option<Instant>) {
        self.inner.sock.set_deadline(at);
    }

    /// The instant every wait on this session is bounded by, if any.
    #[must_use]
    pub fn deadline(&self) -> Option<Instant> {
        self.inner.sock.deadline()
    }

    /// The address at the other end.
    ///
    /// # Errors
    ///
    /// The platform's answer for a socket that is no longer connected.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.inner.sock.peer_addr()
    }
}

impl<T: Read + Write> Read for NvsTls<T> {
    /// Plaintext out of the session, decrypting whole records and parking for
    /// the rest of a record that has not arrived.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<T: Read + Write> Write for NvsTls<T> {
    /// Plaintext into the session, framed into records and written through.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    /// Pushes whatever `rustls` is still holding, then the socket.
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// The one seam a configured anchor bundle plugs into: a handshake against a
/// caller-supplied configuration rather than the compiled-in one.
///
/// Private, and it stays private until an operator can name a bundle in
/// `nvs.toml` — this module's docs § *The trust anchors are compiled in* is why
/// a program may never reach it.
fn upgraded<T: Read + Write>(
    stream: T,
    name: &str,
    config: Arc<ClientConfig>,
) -> io::Result<NvsTls<T>> {
    let name = ServerName::try_from(name)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?
        .to_owned();
    let mut conn = ClientConnection::new(config, name).map_err(io::Error::other)?;
    let mut stream = stream;
    while conn.is_handshaking() {
        // `complete_io` drives both directions until neither wants anything;
        // still handshaking with nothing moved means the peer closed, which it
        // reports as `Ok((0, 0))` rather than as an error.
        let (read, written) = conn.complete_io(&mut stream)?;
        if read == 0 && written == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the peer closed the connection during the TLS handshake",
            ));
        }
    }
    Ok(NvsTls {
        inner: StreamOwned::new(conn, stream),
    })
}

/// The process-wide client configuration, built on first use.
///
/// One root store for the process and not one per session: parsing ~150 anchors
/// per outbound call would be priority 3 spent on a constant. `ClientConfig` is
/// `Send + Sync` and is only ever read after this, so sharing it across cores
/// costs an `Arc` clone and no lock.
fn anchors() -> Arc<ClientConfig> {
    static DEFAULT: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    Arc::clone(DEFAULT.get_or_init(|| {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        Arc::new(config_over(roots))
    }))
}

/// A client configuration over `roots`, on the one provider this build has.
///
/// Split out so a test can hand its own root store to [`upgraded`] and reach
/// the same versions, the same cipher suites and the same verifier a shipped
/// session gets — a test on a configuration of its own would be asserting
/// against something Novis does not run.
fn config_over(roots: RootCertStore) -> ClientConfig {
    ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("the ring provider refused the default protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::NvsTcp;
    use crate::reactor::{Reactor, install, run_until_idle};
    use crate::scheduler::Scheduler;
    use nvs_runtime::{Ctx, OutputSink, TaskRoot};
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
    }

    /// A self-signed certificate for `localhost`, and the client configuration
    /// that trusts exactly it.
    ///
    /// The whole test-only part of a TLS test: everything else below runs the
    /// shipped path, because [`config_over`] is the same builder [`anchors`]
    /// calls.
    fn issued() -> (
        CertificateDer<'static>,
        PrivateKeyDer<'static>,
        Arc<ClientConfig>,
    ) {
        let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .expect("the certificate could not be generated");
        let cert = issued.cert.der().clone();
        let key =
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(issued.signing_key.serialize_der()));
        let mut roots = RootCertStore::empty();
        roots
            .add(cert.clone())
            .expect("the root store refused the certificate");
        (cert, key, Arc::new(config_over(roots)))
    }

    /// A TLS server on loopback that answers `pong\n` to whatever it is told,
    /// with a pause before it does so the client is already parked.
    fn peer(
        cert: CertificateDer<'static>,
        key: PrivateKeyDer<'static>,
    ) -> (SocketAddr, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let handle = std::thread::spawn(move || {
            let config = rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .expect("the provider refused the default versions")
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .expect("the certificate and the key did not pair");
            let (mut sock, _) = listener.accept().expect("the accept failed");
            let mut conn = rustls::ServerConnection::new(Arc::new(config))
                .expect("the server config was rejected");
            // Late on purpose: the client is parked on this flight.
            std::thread::sleep(Duration::from_millis(20));
            let mut tls = rustls::Stream::new(&mut conn, &mut sock);
            let mut heard = [0_u8; 5];
            tls.read_exact(&mut heard).expect("the peer's read failed");
            assert_eq!(&heard, b"ping\n");
            tls.write_all(b"pong\n").expect("the peer's write failed");
            tls.flush().expect("the peer's flush failed");
        });
        (addr, handle)
    }

    /// The claim this module exists for: a verified session runs to completion
    /// on a core, and the core served something else while it waited.
    ///
    /// `resumes > 1` is what says the second half — a session that never parked
    /// would carry the same plaintext and prove nothing about the transport.
    #[test]
    fn a_tls_session_over_the_parking_stream_survives_a_park() {
        let (cert, key, config) = issued();
        let (addr, joined) = peer(cert, key);

        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        let session = Rc::new(RefCell::new(None));
        let recorded = Rc::clone(&session);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let sock = NvsTcp::connect(addr).expect("the connect failed");
            let mut tls = upgraded(sock, "localhost", config).expect("the handshake failed");
            tls.write_all(b"ping\n").expect("the write failed");
            tls.flush().expect("the flush failed");
            let mut heard = [0_u8; 5];
            tls.read_exact(&mut heard).expect("the read failed");
            *recorded.borrow_mut() =
                Some(String::from_utf8(heard.to_vec()).expect("the plaintext was not text"));
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        joined.join().expect("the peer thread panicked");
        assert_eq!(
            session.borrow().as_deref(),
            Some("pong\n"),
            "the session lost its plaintext"
        );
        assert!(
            report.resumes > 1,
            "the session never parked, so nothing was driven across a park"
        );
    }

    /// The anchors are load-bearing, not decoration: the same certificate the
    /// test above trusts explicitly is refused by the compiled-in set.
    ///
    /// Off a core deliberately — this asserts what [`NvsTls::over`] does, and
    /// [`NvsTcp`]'s blocking path is the same `Read` and `Write` underneath.
    #[test]
    fn a_self_signed_certificate_is_refused_by_the_compiled_in_anchors() {
        let (cert, key, _trusting) = issued();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let joined = std::thread::spawn(move || {
            let config = rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .expect("the provider refused the default versions")
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .expect("the certificate and the key did not pair");
            let (mut sock, _) = listener.accept().expect("the accept failed");
            let mut conn = rustls::ServerConnection::new(Arc::new(config))
                .expect("the server config was rejected");
            // The client is expected to hang up on the certificate, so the
            // server's own view of that is an error and not a failure.
            let _ = conn.complete_io(&mut sock);
        });

        let sock = NvsTcp::connect(addr).expect("the connect failed");
        let refused = NvsTls::over(sock, "localhost").expect_err("an untrusted peer was accepted");
        joined.join().expect("the peer thread panicked");
        assert_eq!(
            refused.kind(),
            io::ErrorKind::InvalidData,
            "an unverifiable certificate came back as {refused}"
        );
        assert!(
            refused.to_string().contains("certificate"),
            "the refusal did not say what was wrong: {refused}"
        );
    }

    /// The stream's deadline bounds the handshake, which is the whole reason
    /// the clock is on the socket rather than on this type.
    ///
    /// The peer accepts and then says nothing at all, so the client is waiting
    /// on the server's first flight when its budget runs out.
    #[test]
    fn a_handshake_past_its_deadline_reports_a_timeout() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let joined = std::thread::spawn(move || {
            let (sock, _) = listener.accept().expect("the accept failed");
            // Held open and mute: a close would end the wait with an EOF
            // instead of with the clock, which is the other test.
            std::thread::sleep(Duration::from_millis(600));
            drop(sock);
        });

        let mut sock = NvsTcp::connect(addr).expect("the connect failed");
        sock.set_deadline(Some(Instant::now() + Duration::from_millis(50)));
        let expired =
            NvsTls::over(sock, "localhost").expect_err("a mute peer completed a handshake");
        joined.join().expect("the peer thread panicked");
        assert_eq!(
            expired.kind(),
            io::ErrorKind::TimedOut,
            "the handshake ended with {expired} rather than with its clock"
        );
    }
}
