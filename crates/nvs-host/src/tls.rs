//! TLS over the parking stream: a `rustls` client session on top of
//! [`NvsTcp`], and the trust anchor set it verifies against.
//!
//! [`crate::net`] promises that an unmodified protocol implementation runs over
//! its stream (`rule:concurrency/try-the-syscall-then-park`), and its
//! `a_rustls_session_streams_over_it_unmodified` proves that for TLS. What
//! makes the promise usable is a **client**, and a client is a
//! session plus an answer to "whose certificates do you believe". This module
//! is those two things and nothing else: [`NvsTls::over`] takes a connected
//! stream, completes a handshake on it, and hands back a plaintext
//! `Read`/`Write` that parks exactly like the stream underneath it.
//!
//! Nothing here knows about HTTP or SMTP. `Core\Http\Client` reaches it for an
//! `https` URL and `Core\Mail` reaches it for `STARTTLS`, and both of those
//! reach it the same way, because a protocol that was written against a socket
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
//! ([ADR 0132 § 3](/docs/decisions/0132.md)).
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
//! # The trust anchors are compiled in, and an operator may name their own
//!
//! Novis trusts **Mozilla's CA set, carried in the binary** (`webpki-roots`),
//! and not the host's own certificate store. The reasons, in this project's
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
//!    `rule:security/capability-question-is-grant-and-scope`
//!    would then owe a door and an answer for. A constant in the binary
//!    owes neither.
//!
//! What this gives up is the private CA — an internal PKI, or a corporate
//! inspection proxy — and that is deliberately left to the **operator**, whose
//! decision it is, in the file that already holds every other one
//! (`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`),
//! who names a PEM bundle in `nvs.toml` and gets [`NvsTls::over_bundle`]
//! against exactly it. The bundle **replaces** the compiled-in set for the
//! endpoint that names it rather than adding to it — [`anchors_from`] argues
//! that, and it is the reading every other client an operator has configured
//! already has. What is closed permanently is a *program* choosing anchors, or
//! turning verification off: neither has a spelling here, and the whole point
//! of `rule:http-server/allow-url-pins-the-address`'s pinned outbound door is that a script does not get to widen a
//! decision the deployment made.
//!
//! What that spends, per `rule:programs/memory-priority`:
//! one parsed root store and one `ClientConfig` for the whole **process**, built
//! once on first use and shared by every session after it — the whole Mozilla
//! anchor set, a few hundred kilobytes, O(1) in requests served. Per session
//! it is `rustls`'s own connection state, which is O(in-flight) and released
//! with the stream.
//!
//! # `ring` is the provider, and it is spent under `rule:packaging/a-c-dependency-answers-two-questions`
//!
//! `rustls`'s default provider is `aws-lc-rs`, which is a C library built by
//! `cmake`; it is off. `ring` is the provider instead, and it is not pure Rust
//! either — its primitives are BoringSSL's pregenerated assembly behind a Rust
//! API. So `rule:packaging/a-c-dependency-answers-two-questions` applies with its first question answered **yes**: a TLS
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

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use rustls::client::WantsClientCert;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::{ClientConfig, ClientConnection, ConfigBuilder, RootCertStore, StreamOwned};

use crate::net::NvsTcp;

/// A TLS client session over the parking stream: plaintext in, plaintext out,
/// and every wait underneath it hands the core back.
///
/// Built by [`NvsTls::over`] from a stream that is already connected, because
/// where to connect is `rule:http-server/allow-url-pins-the-address`'s
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
    /// ([ADR 0132 § 3](/docs/decisions/0132.md)).
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

    /// [`over`](Self::over), against the anchors an operator named instead of
    /// the compiled-in set.
    ///
    /// `bundle` is a PEM file of certificates, already resolved and
    /// trust-checked by whoever read the configuration — this function opens
    /// the path it is given and asks no questions about where it came from,
    /// which is what keeps `rule:security/capability-question-is-grant-and-scope`'s door on the config reader rather than
    /// here. `nvs_config::db` is that reader today.
    ///
    /// # Errors
    ///
    /// [`over`](Self::over)'s, plus `NotFound`/`InvalidData` for a bundle that
    /// cannot be opened or holds no certificate.
    pub fn over_bundle(stream: T, name: &str, bundle: &Path) -> io::Result<Self> {
        upgraded(stream, name, anchors_from(bundle)?)
    }

    /// [`over`](Self::over), presenting `identity` when the server asks the
    /// client for a certificate.
    ///
    /// Presenting is the server's decision: a handshake that carries no
    /// `CertificateRequest` sends nothing, so the same call is a plain session
    /// against a server that does not want one. The anchors are still the
    /// compiled-in set — an identity says who *this* end is and changes nothing
    /// about whom this end believes.
    ///
    /// # Errors
    ///
    /// [`over`](Self::over)'s. The chain and the key were parsed and matched
    /// when the identity was read, so nothing about them can fail here.
    pub fn over_identity(stream: T, name: &str, identity: &NvsIdentity) -> io::Result<Self> {
        upgraded(stream, name, Arc::clone(&identity.config))
    }
}

/// A client identity: the certificate chain a handshake presents when a server
/// asks for one, and the private key that proves the leaf is this client's.
///
/// **Built once and shared by every session that presents it.** What it holds
/// is a whole [`ClientConfig`] rather than the chain and the key, because that
/// is the value `rustls` takes and building one costs a key parse and a chain
/// parse. The anchor set inside it is [`root_store`]'s `Arc`, so an identity
/// re-parses no certificate of the compiled-in set.
///
/// **The leaf is public and the key is not.** [`NvsIdentity::leaf`] answers the
/// end-entity certificate's DER, which is what a caller names an identity by —
/// a connection pool keyed on it never hands one identity's connection to
/// another's call, and the value that decides it is one the server was going to
/// be sent anyway. Nothing here answers the key.
///
/// What it spends, per `rule:programs/memory-priority`: one parsed chain, one
/// parsed private key and one `ClientConfig` per identity, plus an `Arc` onto
/// the process's anchor set. Held as long as the identity is and released with
/// it, which is O(identities in flight) and never O(requests served).
pub struct NvsIdentity {
    /// The configuration every session under this identity handshakes with.
    config: Arc<ClientConfig>,
    /// The end-entity certificate, kept as the identity's public name.
    leaf: CertificateDer<'static>,
}

impl std::fmt::Debug for NvsIdentity {
    /// The leaf's length and nothing else: the configuration holds the private
    /// key, so no rendering of this type reaches it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NvsIdentity")
            .field("leaf", &self.leaf.as_ref().len())
            .finish_non_exhaustive()
    }
}

impl NvsIdentity {
    /// Reads a PEM certificate chain over the PKCS#8 private key that goes with
    /// it, refusing a chain whose leaf belongs to a different key.
    ///
    /// `chain_pem` is the end-entity certificate first and then whatever
    /// intermediates a server needs to build a path, which is the order every
    /// other client takes a chain in. The match is a comparison of
    /// `SubjectPublicKeyInfo` between that leaf and the key, and it is the only
    /// check that can be made here at all: whether the chain is *trusted* is the
    /// far end's decision, and it arrives as a handshake that fails.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a `chain_pem` that is not PEM or holds no certificate,
    /// a key this provider cannot sign with — an X25519 key agrees rather than
    /// signs, so it is one of these — and a leaf whose public key is not this
    /// key's. Each is a mistake in what the program was deployed with rather
    /// than a verdict on anybody, which is the reading
    /// `Core\Crypto\KeyPair::read` already takes of the same material.
    pub fn read(chain_pem: &[u8], pkcs8: &[u8]) -> io::Result<Self> {
        let refused = |why: String| io::Error::new(io::ErrorKind::InvalidData, why);
        let chain = CertificateDer::pem_slice_iter(chain_pem)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| refused(format!("the chain is not a PEM certificate chain: {err}")))?;
        let Some(leaf) = chain.first().cloned() else {
            return Err(refused("the chain holds no certificate".to_owned()));
        };

        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8.to_vec()));
        let config = verifying(root_store())
            .with_client_auth_cert(chain, key)
            .map_err(|err| {
                refused(match err {
                    rustls::Error::InconsistentKeys(_) => "the chain's leaf certificate carries a \
                                                           different public key than this key pair"
                        .to_owned(),
                    other => format!("this key cannot sign a TLS handshake: {other}"),
                })
            })?;
        Ok(Self {
            config: Arc::new(config),
            leaf,
        })
    }

    /// The end-entity certificate's DER, which is this identity's public name.
    #[must_use]
    pub fn leaf(&self) -> &[u8] {
        self.leaf.as_ref()
    }

    /// Whether a handshake under this identity answers a server's certificate
    /// request with a certificate.
    ///
    /// The *when asked* half of a client identity, as something a caller can
    /// assert rather than infer: a session presents nothing until the server
    /// sends a `CertificateRequest`, and what it presents then is whatever this
    /// answers. A configuration built by [`config_over`] answers `false` here,
    /// which is what "and never otherwise" is.
    #[must_use]
    pub fn presents(&self) -> bool {
        self.config.client_auth_cert_resolver.has_certs()
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

/// The socket's clock, reached through however many wrappers are between this
/// session and it.
///
/// The inherent pair above is `NvsTls<NvsTcp>`'s and stays that way — it is what
/// every caller in the tree writes, and it needs no bound. This impl is for the
/// tunnelled case: SQL Server's session sits on `nvs-db`'s framing adapter,
/// which sits on the socket, and neither this crate nor that one can see the
/// whole stack from where it stands.
impl<T: Read + Write + crate::net::Deadline> crate::net::Deadline for NvsTls<T> {
    fn set_deadline(&mut self, at: Option<Instant>) {
        self.inner.sock.set_deadline(at);
    }

    fn deadline(&self) -> Option<Instant> {
        self.inner.sock.deadline()
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
/// Private, even though an operator *can* name a bundle:
/// [`NvsTls::over_bundle`] is that door and it takes a path, so the only
/// configurations this module will build are the compiled-in set and a file
/// `nvs_config` resolved. A caller handing in its own `ClientConfig` is a
/// program choosing anchors, which this module's docs § *The trust anchors are
/// compiled in* closes permanently.
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
/// One root store for the process and not one per session: parsing the whole
/// anchor set per outbound call would be priority 3 spent on a constant.
/// `ClientConfig` is `Send + Sync` and is only ever read after this, so sharing
/// it across cores costs an `Arc` clone and no lock.
fn anchors() -> Arc<ClientConfig> {
    static DEFAULT: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    Arc::clone(DEFAULT.get_or_init(|| Arc::new(config_over(root_store()))))
}

/// The compiled-in anchor set, parsed once for the process.
///
/// Separate from [`anchors`] because the builder takes the store by `Arc`: that
/// configuration and every [`NvsIdentity`]'s are built over this one parse, so
/// an identity costs a chain and a key rather than the whole Mozilla set a
/// second time.
fn root_store() -> Arc<RootCertStore> {
    static ROOTS: OnceLock<Arc<RootCertStore>> = OnceLock::new();
    Arc::clone(ROOTS.get_or_init(|| {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        Arc::new(roots)
    }))
}

/// The client configuration for one operator-named PEM bundle, built on first
/// use of that path.
///
/// **The bundle replaces the compiled-in set for the connection that names it
/// and does not add to it.** That is `sslrootcert`'s meaning on libpq and
/// `ssl-ca`'s on MySQL, so it is what an operator writing the key already
/// expects; and it is the stricter of the two readings, which decides it under
/// priority 1. A server behind a private CA is precisely the deployment where
/// any public CA still being able to vouch for its name is the attack
/// the bundle was written to prevent. An operator who wants both writes both
/// into the file.
///
/// Cached per path for [`anchors`]'s reason and with its bound: a connection
/// pool re-opens against the same `[db.<name>]` block for the life of the
/// process, and re-parsing a bundle per handshake would be priority 3 spent on
/// a constant. The map is O(distinct bundles the configuration names), which is
/// O(1) in requests served — a path is only ever inserted from a value
/// `nvs_config` resolved at boot.
fn anchors_from(path: &Path) -> io::Result<Arc<ClientConfig>> {
    static BUNDLES: OnceLock<Mutex<HashMap<PathBuf, Arc<ClientConfig>>>> = OnceLock::new();
    let cache = BUNDLES.get_or_init(|| Mutex::new(HashMap::new()));

    if let Some(hit) = cache
        .lock()
        .expect("the anchor bundle cache was poisoned")
        .get(path)
    {
        return Ok(Arc::clone(hit));
    }

    // `pem_file_iter` folds "the file is not there" and "the file is not a
    // bundle" into one error type, and those are the two an operator acts on
    // differently. Opening it here keeps them apart: past this line every
    // refusal is `InvalidData` and is about the content.
    drop(std::fs::File::open(path)?);

    // A file that holds no certificate — empty, or a PEM of something else —
    // would otherwise verify nothing at all, and the handshake would fail with
    // an unknown issuer: a message that sends an operator looking at the server
    // rather than at the bundle they wrote. So a parse error and an empty
    // result are the same refusal, and both name the file.
    let refused = |why: &dyn std::fmt::Display| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("`{}` names no trust anchor: {why}", path.display()),
        )
    };
    let mut roots = RootCertStore::empty();
    for cert in CertificateDer::pem_file_iter(path).map_err(|err| refused(&err))? {
        roots
            .add(cert.map_err(|err| refused(&err))?)
            .map_err(|err| refused(&err))?;
    }
    if roots.is_empty() {
        return Err(refused(&"it holds no certificate"));
    }

    let config = Arc::new(config_over(roots));
    cache
        .lock()
        .expect("the anchor bundle cache was poisoned")
        .insert(path.to_path_buf(), Arc::clone(&config));
    Ok(config)
}

/// A client configuration over `roots`, on the one provider this build has.
///
/// Split out so a test can hand its own root store to [`upgraded`] and reach
/// the same versions, the same cipher suites and the same verifier a shipped
/// session gets — a test on a configuration of its own would be asserting
/// against something Novis does not run.
fn config_over(roots: impl Into<Arc<RootCertStore>>) -> ClientConfig {
    verifying(roots).with_no_client_auth()
}

/// The shipped builder, stopped at the point where a client certificate is or
/// is not named.
///
/// The seam [`NvsIdentity`] plugs into, and the reason it is a seam rather than
/// a second builder: an identity changes what this end presents and nothing
/// about the versions, the cipher suites or the verifier, so a session that
/// presents one and a session that does not are the same session either way.
fn verifying(roots: impl Into<Arc<RootCertStore>>) -> ConfigBuilder<ClientConfig, WantsClientCert> {
    ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("the ring provider refused the default protocol versions")
        .with_root_certificates(roots)
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

    /// A path under the system temporary directory, named for its case.
    ///
    /// No `tempfile` dependency for this: what these cases need is a name
    /// nothing else writes, and the case's own is that.
    fn scratch(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nvs-anchors-{name}.pem"))
    }

    /// One bundle is parsed once however many sessions name it.
    ///
    /// The memoization [`anchors_from`] promises, asserted by identity rather
    /// than by equality: `ClientConfig` has no `PartialEq`, and a second parse
    /// producing an equal configuration would be exactly the cost the cache
    /// exists to avoid.
    #[test]
    fn an_anchor_bundle_is_parsed_once_per_path() {
        let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .expect("the certificate could not be generated");
        let path = scratch("parsed-once");
        std::fs::write(&path, issued.cert.pem()).expect("the bundle could not be written");

        let first = anchors_from(&path).expect("the bundle was refused");
        let second = anchors_from(&path).expect("the bundle was refused on the second call");
        assert!(
            Arc::ptr_eq(&first, &second),
            "the bundle was parsed a second time for the same path"
        );
        std::fs::remove_file(&path).ok();
    }

    /// An identity is the chain **and** the key, and the one check that can be
    /// made without asking a server anything is that they are each other's.
    ///
    /// The mismatch is the case worth having: a deployment that renews a
    /// certificate and leaves the old key beside it otherwise finds out during
    /// a handshake against a live server, as an alert about the far end.
    #[test]
    fn an_identity_reads_a_chain_over_its_own_key_and_refuses_another_key() {
        let mine = rcgen::generate_simple_self_signed(vec!["client.example".to_owned()])
            .expect("the certificate could not be generated");
        let theirs = rcgen::generate_simple_self_signed(vec!["client.example".to_owned()])
            .expect("the second certificate could not be generated");
        let chain = mine.cert.pem();

        let identity = NvsIdentity::read(chain.as_bytes(), &mine.signing_key.serialize_der())
            .expect("the chain and its own key were refused");
        assert_eq!(
            identity.leaf(),
            mine.cert.der().as_ref(),
            "the leaf is the first certificate of the chain"
        );
        assert!(
            identity.presents(),
            "an identity answers a server that asks for a certificate"
        );
        assert!(
            !config_over(root_store())
                .client_auth_cert_resolver
                .has_certs(),
            "a session with no identity presents nothing to a server that asks"
        );

        let mismatched = NvsIdentity::read(chain.as_bytes(), &theirs.signing_key.serialize_der())
            .expect_err("a leaf carrying a different public key than the key was accepted");
        assert!(
            mismatched.to_string().contains("different public key"),
            "the refusal says which half does not match: {mismatched}"
        );

        let empty = NvsIdentity::read(b"", &mine.signing_key.serialize_der())
            .expect_err("a chain with no certificate in it was accepted");
        assert!(empty.to_string().contains("no certificate"), "{empty}");
    }

    /// A bundle holding no certificate is refused where it is read.
    ///
    /// The alternative is an empty root store and a handshake that fails with
    /// an unknown issuer, which sends an operator to look at the server rather
    /// than at the file they wrote.
    #[test]
    fn an_anchor_bundle_with_no_certificate_is_refused_rather_than_trusted_empty() {
        let path = scratch("no-certificate");
        std::fs::write(&path, "# not a certificate\n").expect("the bundle could not be written");

        let refused = anchors_from(&path).expect_err("an empty bundle built a configuration");
        assert_eq!(
            refused.kind(),
            io::ErrorKind::InvalidData,
            "the refusal was {refused} rather than one about the file's content"
        );
        std::fs::remove_file(&path).ok();
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
