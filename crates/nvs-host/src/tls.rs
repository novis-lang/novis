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
//!    machine nobody reread. Security does not survive a trust set that is
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
//! who writes it in `nvs.toml`. There are two spellings, and they mean opposite
//! things on purpose:
//!
//! - **One endpoint's bundle**, `[db.<name>] tls_ca_file`, reaches
//!   [`NvsTls::over_bundle`] and **replaces** the compiled-in set for that
//!   endpoint rather than adding to it — [`anchors_from`] argues that, and it is
//!   the reading every other client an operator has configured already has.
//! - **The whole process's list**, `[http.client.tls] roots`, reaches
//!   [`configure`] and says what it holds: `["bundled", "corp.pem"]` is both
//!   sets and a list without [`BUNDLED`] is only its files. Nothing is inferred
//!   there because the operator wrote the list out ([`store_for`]).
//!
//! The same block carries the version floor and the key log, and all three are
//! the **operator's** alone.
//!
//! # A call relaxes verification only where a grant already named its host
//!
//! Strict is the default and the only thing a program reaches on its own. The
//! four relaxations a call may ask for — [`CallPolicy`]'s fields, which are
//! `curl`'s `CAINFO_BLOB`, `--pinnedpubkey`, `SSL_VERIFYHOST=0` and `-k` — are
//! answered only where a `[capabilities.tls]` grant lists the host being
//! called, which `nvs-stdlib` asks of the request's own configuration snapshot
//! before a socket is opened
//! (`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`). Two halves,
//! because each answers a different party: the deployment says *where* this may
//! happen, the code says *here*, and neither alone relaxes anything.
//!
//! What arrives here is therefore a **policy value and never a session**
//! ([`NvsTls::over_policy`]): the verifier, the protocol versions and the
//! anchor set are still decided in this module, so a caller cannot widen any of
//! them by construction and `rule:security/one-tls-client` still holds. Every
//! relaxed session still checks the handshake signature against the key the
//! peer presented, so the peer does hold that key; what is skipped is the
//! question of whose key it is.
//!
//! What that spends, per `rule:programs/memory-priority`:
//! one parsed root store and one `ClientConfig` for the whole **process**, built
//! at boot by [`configure`] or on first use from the compiled-in set, and shared
//! by every session after it — the whole Mozilla anchor set, a few hundred
//! kilobytes, O(1) in requests served. A reload that changes `[http.client.tls]`
//! builds a second one and [`install`]s it, and the two live together only until
//! the last session on the old one ends. A key log adds one open file descriptor
//! and one lock acquisition per secret written, on a host that has already said
//! it is being debugged. A call that names one of [`CallPolicy`]'s keys adds one
//! more `ClientConfig` — its own parsed anchors, or a verifier over the set the
//! process already parsed — built for that call and released with it, so it is
//! O(in-flight) too. Per session it is `rustls`'s own connection state,
//! which is O(in-flight) and released with the stream.
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
use std::sync::{Arc, Mutex, OnceLock, PoisonError, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::{WantsClientCert, WebPkiServerVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{
    CertificateError, CipherSuite, ClientConfig, ClientConnection, ConfigBuilder,
    DigitallySignedStruct, ProtocolVersion, RootCertStore, SignatureScheme, StreamOwned,
    WantsVerifier,
};
use sha2::{Digest, Sha256};
use x509_parser::prelude::{FromDer, X509Certificate};

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
    /// `name` is the DNS name or IP address the certificate must be valid for.
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
    /// `InvalidInput` when `name` is neither a DNS name nor an IP address,
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
        Self::over_policy(stream, name, &CallPolicy::default(), Some(identity))
    }

    /// [`over`](Self::over), under the relaxations one call asked for and a
    /// `[capabilities.tls]` grant already allows for the host it is calling.
    ///
    /// A [`CallPolicy`] and never a session: what a caller may say is *which*
    /// of ADR 0180 § 11's four relaxations it wants, and every verifier that
    /// answers one is built here (`rule:security/one-tls-client`). A default
    /// policy asks for nothing, so this is also the plain door for a caller
    /// holding a policy it has not looked at — that call gets the process's own
    /// configuration, the same `Arc` [`over`](Self::over) uses.
    ///
    /// `identity` is presented under the relaxed session just as it is under a
    /// strict one, because the two answer different questions: trusting a
    /// private CA and proving who is calling it are the two halves of mutual
    /// TLS, and a call may well write both options.
    ///
    /// # Errors
    ///
    /// [`over`](Self::over)'s, plus `InvalidData` for `anchors` text that holds
    /// no certificate and `InvalidInput` for a pin that is not `sha256//` and
    /// the base64 of a 32-byte hash.
    pub fn over_policy(
        stream: T,
        name: &str,
        policy: &CallPolicy,
        identity: Option<&NvsIdentity>,
    ) -> io::Result<Self> {
        upgraded(stream, name, config_for(policy, identity)?)
    }

    /// What this session negotiated: the protocol version, the cipher suite and
    /// the peer's certificate chain.
    ///
    /// `rule:http-server/a-reply-reports-its-tls-session`'s three facts, read
    /// off the completed handshake rather than off the configuration the
    /// handshake ran under. A caller asserting what one call got has to be
    /// answered by the session, because a policy is what was *asked* for and
    /// the peer is the other half of what was agreed. Whether the chain and the
    /// name were checked is not a fact about the session at all — that is
    /// [`CallPolicy::verifies`], which belongs to the call.
    ///
    /// A version or a cipher that reads empty would be a session that
    /// negotiated neither, which none of the doors above can hand back: each
    /// returns only once the handshake is complete.
    #[must_use]
    pub fn session(&self) -> Session {
        let state = &self.inner.conn;
        Session {
            version: state
                .protocol_version()
                .map(named_version)
                .unwrap_or_default(),
            cipher: state
                .negotiated_cipher_suite()
                .map(|suite| named_suite(suite.suite()))
                .unwrap_or_default(),
            chain: state
                .peer_certificates()
                .unwrap_or_default()
                .iter()
                .map(|cert| cert.as_ref().to_vec())
                .collect(),
        }
    }
}

/// What one TLS session negotiated, as `Core\Http\Response::tls` reports it.
///
/// A **snapshot and not a handle**, which is the whole shape: the connection a
/// reply arrived over goes back into the per-core store the moment the reply is
/// framed
/// (`rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`),
/// while the response object a program holds outlives it and may be read at any
/// point afterwards. Three owned values taken at the framing cannot disagree
/// with a session that has since served somebody else's request.
///
/// The chain is **DER**, leaf first, because that is what `rustls` hands over.
/// Which encoding a program reads it in — PEM, per ADR 0180 § 13 — is
/// `Core\Http\TlsInfo`'s question, and what the leaf's fields say is [`leaf`]'s,
/// beside this type rather than on it: what the handshake settled is a snapshot
/// taken at the framing, and what one certificate inside it says is a reading
/// nothing has to hold a session to ask for.
///
/// What it spends: one copy of the peer's chain per reply that arrived over
/// TLS, released with the response, which is O(in-flight) and never O(replies
/// served).
#[derive(Clone, Debug)]
pub struct Session {
    /// The protocol version, spelled as [`named_version`] writes it.
    version: String,
    /// The cipher suite, spelled as [`named_suite`] writes it.
    cipher: String,
    /// The peer's certificate chain as DER, leaf first.
    chain: Vec<Vec<u8>>,
}

impl Session {
    /// A session no handshake produced: the one [`described`] issues, and the
    /// one a test's answer table hands back a copy of for every reply it
    /// answers. The three values are taken as they are, so the caller is the
    /// one that already checked them.
    #[must_use]
    pub fn recorded(version: String, cipher: String, chain: Vec<Vec<u8>>) -> Self {
        Self {
            version,
            cipher,
            chain,
        }
    }

    /// The protocol version — `TLSv1.3` or `TLSv1.2`.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The negotiated cipher suite, under its IANA registry name.
    #[must_use]
    pub fn cipher(&self) -> &str {
        &self.cipher
    }

    /// The peer's certificate chain as DER, leaf first, or empty where the peer
    /// presented none.
    #[must_use]
    pub fn chain(&self) -> &[Vec<u8>] {
        &self.chain
    }
}

/// A protocol version as a program reads it: `TLSv1.3`, not `rustls`'s Rust
/// identifier `TLSv1_3` for the same constant, and the registry's own code for
/// one this build negotiated but cannot name.
fn named_version(version: ProtocolVersion) -> String {
    version.as_str().map_or_else(
        || format!("0x{:04x}", u16::from(version)),
        |spelled| spelled.replace('_', "."),
    )
}

/// A cipher suite under its IANA name.
///
/// `rustls` spells the TLS 1.3 suites with the version inside the prefix —
/// `TLS13_AES_128_GCM_SHA256` — where the registry and every other tool write
/// `TLS_AES_128_GCM_SHA256`; its TLS 1.2 names already match. So the prefix is
/// the one thing rewritten, and a suite with no name reports its code.
fn named_suite(suite: CipherSuite) -> String {
    suite.as_str().map_or_else(
        || format!("0x{:04x}", u16::from(suite)),
        |spelled| spelled.replacen("TLS13_", "TLS_", 1),
    )
}

/// What one certificate says: the three facts `Core\Http\TlsInfo` reports about
/// the leaf a peer presented (`rule:http-server/a-reply-reports-its-tls-session`).
///
/// Read in this crate rather than in the standard library because the parser is
/// already here — [`spki_sha256`] walks the same DER for `tlsPin` — and a second
/// crate linking an ASN.1 reader to answer three members is the copy that comes
/// to disagree about what a certificate says. ADR 0180 § 13 is the record, and
/// the alternative it rejects is handing a program PEM alone and expecting it to
/// write an ASN.1 parser in Novis to learn when a certificate expires.
///
/// The two names are written as [`written`] writes one: `KEY=value` pairs in
/// the certificate's order with each value escaped per RFC 4514 § 2.4, which is
/// close to what `openssl x509 -subject` prints and therefore the spelling an
/// operator comparing the two already holds.
#[derive(Clone, Debug)]
pub struct Leaf {
    /// The subject distinguished name.
    subject: String,
    /// The issuer distinguished name.
    issuer: String,
    /// `notAfter` — when the certificate stops being valid.
    expiry: SystemTime,
}

impl Leaf {
    /// The subject distinguished name, RFC 4514.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The issuer distinguished name, RFC 4514.
    #[must_use]
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// When the certificate stops being valid — its `notAfter` field.
    #[must_use]
    pub fn expiry(&self) -> SystemTime {
        self.expiry
    }
}

/// What `der` says, or `None` where it is not a certificate this can read.
///
/// An `Option` and not an error: the one caller is a member reporting on a peer
/// that has already been accepted, so what it needs is the distinction between
/// "here is what the certificate says" and "there is nothing to say", and every
/// reason a DER blob will not parse collapses into the second for a program that
/// cannot fix the other end anyway.
#[must_use]
pub fn leaf(der: &[u8]) -> Option<Leaf> {
    let (_, parsed) = X509Certificate::from_der(der).ok()?;
    let seconds = parsed.validity().not_after.timestamp();
    let since = Duration::from_secs(seconds.unsigned_abs());
    let expiry = if seconds < 0 {
        UNIX_EPOCH.checked_sub(since)?
    } else {
        UNIX_EPOCH.checked_add(since)?
    };
    Some(Leaf {
        subject: written(parsed.subject()),
        issuer: written(parsed.issuer()),
        expiry,
    })
}

/// `name` as [`Leaf::subject`] writes it: `KEY=value` per attribute in the
/// certificate's own order, the attributes of one RDN joined with ` + ` and the
/// RDNs with `, `, and every value escaped as RFC 4514 § 2.4 escapes one.
///
/// The escaping is what keeps two names apart: without it a value `x, O=y` or
/// `x + O=y` reads back as the same text as a name with two attributes. A key
/// the registry has no short name for is its dotted OID, and a value that is
/// not a string is `#` and its content octets in hex.
fn written(name: &x509_parser::x509::X509Name<'_>) -> String {
    let registry = x509_parser::objects::oid_registry();
    let mut out = String::new();
    for (index, rdn) in name.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        for (position, attribute) in rdn.iter().enumerate() {
            if position > 0 {
                out.push_str(" + ");
            }
            match x509_parser::objects::oid2abbrev(attribute.attr_type(), registry) {
                Ok(short) => out.push_str(short),
                Err(_) => out.push_str(&attribute.attr_type().to_id_string()),
            }
            out.push('=');
            match attribute.as_str() {
                Ok(value) => escape_value(value, &mut out),
                Err(_) => {
                    out.push('#');
                    for byte in attribute.as_slice() {
                        out.push_str(&format!("{byte:02X}"));
                    }
                }
            }
        }
    }
    out
}

/// Appends `value` escaped per RFC 4514 § 2.4: a backslash before each of
/// `"+,;<>\`, before a leading space or `#` and before a trailing space, and
/// `\00` for a zero byte.
fn escape_value(value: &str, out: &mut String) {
    let last = value.chars().count().saturating_sub(1);
    for (index, c) in value.chars().enumerate() {
        match c {
            '"' | '+' | ',' | ';' | '<' | '>' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            ' ' | '#' if index == 0 => {
                out.push('\\');
                out.push(c);
            }
            ' ' if index == last => out.push_str("\\ "),
            '\0' => out.push_str("\\00"),
            _ => out.push(c),
        }
    }
}

/// A TLS session a test describes rather than negotiates —
/// `Core\Test::tlsSession`'s options, with the two defaults that need a clock
/// already resolved by the caller.
#[derive(Clone, Copy, Debug)]
pub struct Description<'a> {
    /// `TLSv1.3` or `TLSv1.2`; `None` is `TLSv1.3`.
    pub version: Option<&'a str>,
    /// A suite this build negotiates over `version`, under its IANA name;
    /// `None` is [`DEFAULT_SUITES`]'s entry for the version.
    pub cipher: Option<&'a str>,
    /// The leaf's subject, written as [`Leaf::subject`] reads it back.
    pub subject: &'a str,
    /// The issuing CA's subject, which is the leaf's issuer.
    pub issuer: &'a str,
    /// The leaf's `notAfter`.
    pub expiry: SystemTime,
    /// The caller's wall clock. The leaf is valid from one day before this or
    /// before `expiry`, whichever is earlier.
    pub now: SystemTime,
}

/// The suite a described session reports when the test names none, per
/// version: the AES-128-GCM suite each version negotiates with an ECDSA
/// certificate, which is what [`described`] issues.
pub const DEFAULT_SUITES: [(&str, &str); 2] = [
    ("TLSv1.3", "TLS_AES_128_GCM_SHA256"),
    ("TLSv1.2", "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256"),
];

/// The earliest `notAfter` [`described`] issues: 1950-01-01, the first instant
/// X.509's `UTCTime` can write.
const EARLIEST_CERTIFICATE_SECONDS: i64 = -631_152_000;
/// The latest: 9999-12-31T23:59:59, the last instant `GeneralizedTime` can
/// write.
const LATEST_CERTIFICATE_SECONDS: i64 = 253_402_300_799;

/// The session `description` describes: its version and suite checked against
/// what this build negotiates, and a real two-certificate chain — a leaf for
/// `subject`, signed by a CA certificate for `issuer` — so every
/// `Core\Http\TlsInfo` member reads it exactly as it reads a peer's.
///
/// **Test-only in purpose and harmless outside a test.** Nothing trusts the CA
/// and nothing verifies the chain: the session goes into a `Core\Http\TlsInfo`
/// the program built itself, or into a test's answer table, where no reply
/// crosses a network. What it spends is two fresh P-256 keys and two
/// signatures per call, on the caller's own thread.
///
/// # Errors
///
/// A sentence naming the field, for a version that is not `TLSv1.3` or
/// `TLSv1.2`, a suite this build does not negotiate over that version, a name
/// that is not a list of `KEY=value` pairs with keys from `CN`, `O`, `OU`, `C`,
/// `ST` and `L`, or an expiry outside what a certificate can hold.
pub fn described(description: &Description<'_>) -> Result<Session, String> {
    let version = description.version.unwrap_or("TLSv1.3");
    let Some((_, default_suite)) = DEFAULT_SUITES.iter().find(|(named, _)| *named == version)
    else {
        return Err(format!(
            "`version` is `TLSv1.3` or `TLSv1.2`, and this one is `{version}`"
        ));
    };
    let suites: Vec<String> = provider()
        .cipher_suites
        .iter()
        .filter(|suite| named_version(suite.version().version) == version)
        .map(|suite| named_suite(suite.suite()))
        .collect();
    let cipher = description.cipher.unwrap_or(default_suite);
    if !suites.iter().any(|suite| suite == cipher) {
        return Err(format!(
            "`cipher` is a suite this build negotiates over {version} — {} — and `{cipher}` is \
             not one of them",
            suites.join(", ")
        ));
    }

    let expiry = seconds_of(description.expiry);
    if !(EARLIEST_CERTIFICATE_SECONDS..=LATEST_CERTIFICATE_SECONDS).contains(&expiry) {
        return Err(
            "`expiry` is between 1950 and the end of 9999, which is what a certificate can hold"
                .to_owned(),
        );
    }
    let valid_from =
        (seconds_of(description.now).min(expiry) - 86_400).max(EARLIEST_CERTIFICATE_SECONDS);
    // `rcgen`'s own epoch plus a duration, so this crate names no second date
    // library for the one type `rcgen` writes a validity in.
    let epoch = rcgen::date_time_ymd(1970, 1, 1);
    let certificate_time = |seconds: i64| {
        let span = Duration::from_secs(seconds.unsigned_abs());
        if seconds < 0 {
            epoch - span
        } else {
            epoch + span
        }
    };

    let failed = |error: rcgen::Error| format!("the certificate could not be issued: {error}");
    let mut authority = rcgen::CertificateParams::new(Vec::<String>::new()).map_err(failed)?;
    authority.distinguished_name = distinguished(description.issuer, "issuer")?;
    authority.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    authority.key_usages = vec![
        rcgen::KeyUsagePurpose::KeyCertSign,
        rcgen::KeyUsagePurpose::CrlSign,
    ];
    authority.not_before = certificate_time(valid_from);
    authority.not_after = certificate_time(expiry);
    let authority_key = rcgen::KeyPair::generate().map_err(failed)?;
    let authority_certificate = authority.self_signed(&authority_key).map_err(failed)?;

    let mut leaf = rcgen::CertificateParams::new(Vec::<String>::new()).map_err(failed)?;
    leaf.distinguished_name = distinguished(description.subject, "subject")?;
    leaf.not_before = certificate_time(valid_from);
    leaf.not_after = certificate_time(expiry);
    let leaf_key = rcgen::KeyPair::generate().map_err(failed)?;
    let signer = rcgen::Issuer::new(authority, authority_key);
    let leaf_certificate = leaf.signed_by(&leaf_key, &signer).map_err(failed)?;

    Ok(Session::recorded(
        version.to_owned(),
        cipher.to_owned(),
        vec![
            leaf_certificate.der().to_vec(),
            authority_certificate.der().to_vec(),
        ],
    ))
}

/// `at` as whole seconds from the Unix epoch, negative before it.
fn seconds_of(at: SystemTime) -> i64 {
    match at.duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_secs()).unwrap_or(i64::MAX),
        Err(before) => i64::try_from(before.duration().as_secs()).map_or(i64::MIN, |secs| -secs),
    }
}

/// A distinguished name written the way [`Leaf::subject`] reads one back —
/// `CN=api.example.com, O=Shop` — as the name `rcgen` writes, in the order it
/// was written.
///
/// # Errors
///
/// A sentence naming `field`, for a part with no `=`, an empty value, a key
/// outside the six below, or a key written twice.
fn distinguished(text: &str, field: &str) -> Result<rcgen::DistinguishedName, String> {
    let mut name = rcgen::DistinguishedName::new();
    let mut seen: Vec<&str> = Vec::new();
    for part in text.split(',') {
        let part = part.trim();
        let Some((key, value)) = part.split_once('=') else {
            return Err(format!(
                "`{field}` is written as `KEY=value` pairs such as `CN=api.example.com, O=Shop`, \
                 and `{part}` has no `=`"
            ));
        };
        let (key, value) = (key.trim(), value.trim());
        let kind = match key {
            "CN" => rcgen::DnType::CommonName,
            "O" => rcgen::DnType::OrganizationName,
            "OU" => rcgen::DnType::OrganizationalUnitName,
            "C" => rcgen::DnType::CountryName,
            "ST" => rcgen::DnType::StateOrProvinceName,
            "L" => rcgen::DnType::LocalityName,
            other => {
                return Err(format!(
                    "`{field}` uses the keys `CN`, `O`, `OU`, `C`, `ST` and `L`, and `{other}` is \
                     not one of them"
                ));
            }
        };
        if value.is_empty() {
            return Err(format!("`{field}` gives `{key}` an empty value"));
        }
        if seen.contains(&key) {
            return Err(format!("`{field}` writes `{key}` twice"));
        }
        seen.push(key);
        name.push(kind, value);
    }
    Ok(name)
}

/// A client identity: the certificate chain a handshake presents when a server
/// asks for one, and the private key that proves the leaf is this client's.
///
/// **Built once and shared by every session that presents it.** It holds a
/// whole [`ClientConfig`], because that is the value `rustls` takes and
/// building one costs a key parse and a chain parse, *and* the chain and key it
/// was built from, because a call that also relaxes trust needs the same
/// certificates under a verifier this configuration does not have and a built
/// one hands neither back ([`config_for`]). The anchor set inside it is
/// [`root_store`]'s `Arc`, so an identity re-parses no certificate of the
/// compiled-in set.
///
/// **The leaf is public and the key is not.** [`NvsIdentity::leaf`] answers the
/// end-entity certificate's DER, which is what a caller names an identity by —
/// a connection pool keyed on it never hands one identity's connection to
/// another's call, and the value that decides it is one the server was going to
/// be sent anyway. Nothing here answers the key.
///
/// What it spends, per `rule:programs/memory-priority`: one parsed chain, one
/// parsed private key and one `ClientConfig` per identity, plus the DER the two
/// were parsed from and an `Arc` onto the process's anchor set. Held as long as
/// the identity is and released with it, which is O(identities in flight) and
/// never O(requests served). The key bytes are a second copy of material the
/// configuration already holds, in a process that has been handed the key on
/// purpose; nothing here writes either copy anywhere.
pub struct NvsIdentity {
    /// The configuration every ordinary session under this identity handshakes
    /// with.
    config: Arc<ClientConfig>,
    /// The end-entity certificate, kept as the identity's public name.
    leaf: CertificateDer<'static>,
    /// The chain [`NvsIdentity::config`] was built from, for the session a
    /// relaxing call builds instead.
    chain: Vec<CertificateDer<'static>>,
    /// The PKCS#8 key, for [`NvsIdentity::chain`]'s reason: `rustls` takes the
    /// two together.
    pkcs8: Vec<u8>,
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

        let pkcs8 = pkcs8.to_vec();
        let config = verifying(root_store())
            .with_client_auth_cert(
                chain.clone(),
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8.clone())),
            )
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
            chain,
            pkcs8,
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

    /// The key, in the form `rustls` takes when a second session is built over
    /// this identity.
    ///
    /// A parse per relaxed call rather than a shared parsed key, because
    /// `rustls` moves the key into the builder and the one it already moved is
    /// inside a configuration with the wrong verifier.
    fn key(&self) -> PrivateKeyDer<'static> {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.pkcs8.clone()))
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

/// The process's one outbound client, and `None` until [`configure`] or
/// [`install`] put one in place.
///
/// A lock rather than a `OnceLock`, because a reload replaces the client
/// (`rule:config/reloadability-is-its-own-field`). A handshake holds the read
/// lock for one `Arc` clone and nothing after it, so a session that started
/// before a reload finishes on the client it started with, and the old client
/// is freed when the last such session ends.
static INSTALLED: RwLock<Option<Installed>> = RwLock::new(None);

/// The configuration built from the compiled-in anchors on first use, in a
/// process nothing configured — which is what makes a program with no
/// `nvs.toml`, and every test in the tree, still get a verifying client.
static FALLBACK: OnceLock<Arc<ClientConfig>> = OnceLock::new();

/// [`INSTALLED`]'s value: the client, and which one it is.
struct Installed {
    client: Client,
    /// `1` for the client a boot installed, one more for each replacement
    /// that changed something, and `0` is [`FALLBACK`]. [`generation`] is
    /// the reader.
    generation: u64,
}

/// [`INSTALLED`], read whether or not a writer panicked: every write is one
/// assignment, so no holder can leave it half written.
fn installed() -> std::sync::RwLockReadGuard<'static, Option<Installed>> {
    INSTALLED.read().unwrap_or_else(PoisonError::into_inner)
}

/// The process-wide client configuration.
///
/// One root store for the process and not one per session: parsing the whole
/// anchor set per outbound call would spend latency on a constant.
/// `ClientConfig` is `Send + Sync` and is only ever read once built, so sharing
/// it across cores costs an `Arc` clone and one uncontended read lock.
fn anchors() -> Arc<ClientConfig> {
    let held = installed();
    match held.as_ref() {
        Some(installed) => Arc::clone(&installed.client.config),
        None => Arc::clone(FALLBACK.get_or_init(|| Arc::new(config_over(root_store())))),
    }
}

/// The anchor set the installed client was built over, or the compiled-in set
/// where nothing is installed.
///
/// A `ClientConfig` does not answer what it verifies against, and a call that
/// raises its own version floor, or skips only the name, still builds its chain
/// against **the operator's** anchors rather than the compiled-in ones. This is
/// where that set is read back from. The fallback is [`root_store`]'s, the
/// same set [`anchors`] falls back to, so the two cannot disagree about what
/// "the configured anchors" are.
fn configured_store() -> Arc<RootCertStore> {
    installed()
        .as_ref()
        .map_or_else(root_store, |installed| Arc::clone(&installed.client.roots))
}

/// Which client a handshake starting now is built from: `0` before anything
/// was installed, and a larger number after each [`install`] that changed the
/// anchors, the version floor or the key log.
///
/// A connection pool files a connection under this, so a socket opened under
/// the old anchors never serves a call made after a reload replaced them.
#[must_use]
pub fn generation() -> u64 {
    installed()
        .as_ref()
        .map_or(0, |installed| installed.generation)
}

/// The `roots` entry naming the compiled-in Mozilla set rather than a file.
///
/// `nvs_config::http` spells the same word while resolving the list's paths, and
/// `the_bundled_spelling_is_the_one_the_configuration_resolves` holds the two
/// equal — this crate is the one that reads the value, and that one is the one
/// that decides which entries are files, so neither can hold it alone.
pub const BUNDLED: &str = "bundled";

/// `[http.client.tls]` in this crate's own terms: the outbound TLS policy an
/// operator wrote, resolved.
///
/// Plain strings and a path rather than `nvs_config`'s block, because nothing
/// in `src/` reads a configuration file — this crate carries `nvs-config` as a
/// dev dependency alone, and this struct is the seam that keeps it that way.
/// Whoever boots or reloads the process reads the block and fills this in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClientPolicy {
    /// The trust anchors, in the order they were written. [`BUNDLED`] is the
    /// compiled-in Mozilla set and every other entry is a PEM file that
    /// `nvs_config` has already resolved and proved is inside the trust
    /// boundary. Empty is the compiled-in set, which is what an unwritten key
    /// means.
    pub roots: Vec<String>,
    /// The version floor — `"1.2"`, `"1.3"`, or `None` for the shipped one,
    /// which is `"1.2"`.
    pub min_version: Option<String>,
    /// Where each session appends its secrets in the `SSLKEYLOGFILE` format,
    /// and `None` for nowhere. `nvs_config` has already refused this on a
    /// `production` host (`E0640`).
    pub keylog: Option<PathBuf>,
}

/// What one call asked to relax about verifying its peer, and the version floor
/// it asked to raise.
///
/// [`ClientPolicy`] is the operator's answer to "whose certificates do you
/// believe"; this is one call's request to be judged differently, and it is
/// owed a `[capabilities.tls]` grant naming the host it is calling before it
/// reaches this crate at all
/// (`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`). One field
/// per option of the request bag, because the options are independent and a
/// call may write more than one.
///
/// **The default relaxes nothing**, and it is the value every call that names
/// none of the five keys carries: [`config_for`] answers it with the process's
/// own configuration rather than building a second one that would say the same.
///
/// `Eq` and `Hash` because a connection is pooled under it, so a socket opened
/// for a call that verifies nothing never serves one that verifies
/// (ADR 0180 § 6).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CallPolicy {
    /// `tlsCa` — the PEM certificates to trust *in place of* the configured
    /// anchors for this call, as `curl`'s `CAINFO_BLOB` does. Text rather than
    /// a path, because the program hands over what it trusts and no filesystem
    /// is reached to find it.
    pub anchors: Option<String>,
    /// `tlsPin` — `sha256//<base64>` SubjectPublicKeyInfo hashes, as `curl`'s
    /// `--pinnedpubkey`. A peer whose key hashes to one of them is accepted
    /// with no chain built at all, and an empty list pins nothing.
    pub pins: Vec<String>,
    /// `tlsVerifyHost: false` — build and check the chain, and skip only the
    /// name, as `SSL_VERIFYHOST=0` does.
    pub any_name: bool,
    /// `tlsVerify: false` — check neither the chain nor the name, as `-k` does.
    /// The widest of the four, so it decides the session when a call asks for
    /// it beside another: what it asked for is a superset of the rest.
    pub insecure: bool,
    /// `tlsMinVersion` — `"1.3"` to drop 1.2 for this call. It needs no grant
    /// because it can only tighten, and a value below `[http.client.tls]
    /// min_version` is refused before it reaches here.
    pub min_version: Option<String>,
}

impl CallPolicy {
    /// Whether a session under this policy checked **both** the chain and the
    /// name, which is what `Core\Http\TlsInfo::verified` reports
    /// (`rule:http-server/a-reply-reports-its-tls-session`).
    ///
    /// `anchors` is not a relaxation and answers `true`: `tlsCa` replaces the
    /// trust set and then builds and checks the chain against it exactly as the
    /// configured roots are, which is ADR 0180 § 13's *against `roots` or
    /// `tlsCa`*. `min_version` only tightens. Each of the other three drops one
    /// of the two checks — a pin accepts a key with no chain built at all — so
    /// any of them answers `false`.
    ///
    /// It lives on the policy rather than at the call site that reports it
    /// because what each field means is this type's, and a second reading of
    /// these five fields is the copy that comes to disagree.
    #[must_use]
    pub fn verifies(&self) -> bool {
        self.pins.is_empty() && !self.any_name && !self.insecure
    }
}

/// A [`ClientPolicy`] built into a client and not installed yet.
///
/// A reload builds one before it publishes, so a bundle that does not parse is
/// known while the running client can still be kept, and [`install`]s it after.
#[derive(Clone)]
pub struct Client {
    config: Arc<ClientConfig>,
    roots: Arc<RootCertStore>,
    min_version: Option<String>,
    keylog: Option<PathBuf>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("anchors", &self.roots.roots.len())
            .field("min_version", &self.min_version)
            .field("keylog", &self.keylog)
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Parses the anchors `policy` names and builds the client over them.
    ///
    /// The anchors are parsed here rather than on the first outbound call for
    /// the reason [`anchors`] gives — the whole set is a constant, and a
    /// deployment learns that its bundle does not parse while an operator is
    /// reading boot or reload output rather than inside somebody's request.
    ///
    /// # Errors
    ///
    /// `NotFound`/`InvalidData` for a `roots` entry that cannot be opened or
    /// holds no certificate, `InvalidInput` for a `min_version` this build does
    /// not speak, and whatever opening the key log reported — each of which
    /// names the file it is about, because the operator's next move is to open
    /// that file.
    pub fn build(policy: &ClientPolicy) -> io::Result<Self> {
        let roots = Arc::new(store_for(&policy.roots)?);
        let config = Arc::new(built_over(policy, Arc::clone(&roots))?);
        Ok(Self {
            config,
            roots,
            min_version: policy.min_version.clone(),
            keylog: policy.keylog.clone(),
        })
    }

    /// Whether a handshake under `self` would be judged as one under `other`
    /// is: the same anchors, the same version floor and the same key log.
    ///
    /// The anchors are compared, not the paths that named them, so a bundle
    /// replaced in place is a change and an unrelated reload is not.
    fn same_as(&self, other: &Self) -> bool {
        self.roots.roots == other.roots.roots
            && self.min_version == other.min_version
            && self.keylog == other.keylog
    }
}

/// Builds the process's one outbound client from `policy` and installs it, so
/// every [`NvsTls::over`] after this verifies against it.
///
/// The boot's call, made before any request runs. A reload builds a
/// [`Client`] and hands it to [`install`] instead.
///
/// # Errors
///
/// [`Client::build`]'s, and `AlreadyExists` when a client is already in place:
/// a session that has run against the compiled-in default is a boot that
/// reached the network before it read its own configuration, and a second boot
/// in one process is a caller bug.
pub fn configure(policy: &ClientPolicy) -> io::Result<()> {
    let client = Client::build(policy)?;
    let mut held = INSTALLED.write().unwrap_or_else(PoisonError::into_inner);
    // Under the write lock, so no handshake can build the fallback between
    // this check and the assignment: [`anchors`] builds it under the read lock.
    if held.is_some() || FALLBACK.get().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "the outbound TLS client was already built, so a session has run against anchors this \
             configuration did not choose",
        ));
    }
    *held = Some(Installed {
        client,
        generation: 1,
    });
    Ok(())
}

/// Puts `client` in place of the running one, so every handshake that starts
/// after this verifies against it, and answers whether anything changed.
///
/// A client judged the same as the running one ([`Client::same_as`]) is
/// dropped and the running one stays, with its generation, so a reload that
/// did not touch `[http.client.tls]` leaves every pooled connection usable.
pub fn install(client: Client) -> bool {
    let mut held = INSTALLED.write().unwrap_or_else(PoisonError::into_inner);
    let generation = match held.as_ref() {
        Some(running) if running.client.same_as(&client) => return false,
        Some(running) => running.generation + 1,
        None => 1,
    };
    *held = Some(Installed { client, generation });
    true
}

/// [`Client::build`]'s configuration over an anchor set already resolved.
///
/// Split out so that every case below builds its own client here, without
/// installing it, and reaches the same [`upgraded`] a shipped session does. It
/// takes the store rather than reading `roots` itself so that a build parses
/// the anchor set once and [`Client`] keeps that same `Arc`.
fn built_over(policy: &ClientPolicy, roots: Arc<RootCertStore>) -> io::Result<ClientConfig> {
    let mut config = floored(policy.min_version.as_deref(), roots).with_no_client_auth();
    if let Some(path) = policy.keylog.as_deref() {
        config.key_log = Arc::new(KeyLogTo::at(path).map_err(|err| at_path(path, &err))?);
    }
    Ok(config)
}

/// `err` with `path` in front of it, keeping its kind.
///
/// `std::io::Error` carries no path of its own, so a boot refusing a `roots`
/// entry or a key log would otherwise report a reason with nothing to act on —
/// and the operator's next move is always to look at a file. This is the last
/// frame that still knows which one.
fn at_path(path: &Path, err: &io::Error) -> io::Error {
    io::Error::new(err.kind(), format!("`{}`: {err}", path.display()))
}

/// The anchor set a `roots` list names: the compiled-in one wherever
/// [`BUNDLED`] appears, and each other entry's PEM file added to it.
///
/// **Additive, which is the opposite of [`anchors_from`] and deliberately.** A
/// `[db]` block's `tls_ca_file` is one endpoint's bundle and replaces the set
/// for that endpoint alone; `roots` is the whole process's list, and the
/// operator writing it says what it holds — `["bundled", "corp.pem"]` adds a
/// company CA and a list without [`BUNDLED`] trusts only its files. There is
/// nothing to infer, so nothing here infers it.
///
/// An empty list is the compiled-in set. `nvs_config` refuses `roots = []`
/// (`E0638`), so the only caller that reaches this arm is one with no block
/// written at all.
fn store_for(roots: &[String]) -> io::Result<RootCertStore> {
    if roots.is_empty() {
        return Ok(root_store().as_ref().clone());
    }
    let mut store = RootCertStore::empty();
    for entry in roots {
        if entry == BUNDLED {
            store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            continue;
        }
        read_anchors(Path::new(entry), &mut store)?;
    }
    Ok(store)
}

/// Every certificate in the PEM file at `path`, added to `store`.
///
/// A file that holds no certificate — empty, or a PEM of something else —
/// would otherwise verify nothing at all, and the handshake would fail with
/// an unknown issuer: a message that sends an operator looking at the server
/// rather than at the bundle they wrote. So a parse error and an empty
/// result are the same refusal, and both name the file.
fn read_anchors(path: &Path, store: &mut RootCertStore) -> io::Result<()> {
    // `pem_file_iter` folds "the file is not there" and "the file is not a
    // bundle" into one error type, and those are the two an operator acts on
    // differently. Opening it here keeps them apart: past this line every
    // refusal is `InvalidData` and is about the content.
    drop(std::fs::File::open(path).map_err(|err| at_path(path, &err))?);

    let refused = |why: &dyn std::fmt::Display| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("`{}` names no trust anchor: {why}", path.display()),
        )
    };
    let before = store.len();
    for cert in CertificateDer::pem_file_iter(path).map_err(|err| refused(&err))? {
        store
            .add(cert.map_err(|err| refused(&err))?)
            .map_err(|err| refused(&err))?;
    }
    if store.len() == before {
        return Err(refused(&"it holds no certificate"));
    }
    Ok(())
}

/// `[http.client.tls] keylog`'s destination: each session's secrets appended in
/// the `SSLKEYLOGFILE` format, which is the one format Wireshark reads.
///
/// One handle for the process behind a mutex rather than an open per secret,
/// because a handshake emits several and every core shares this. What it spends
/// is one file descriptor and one lock acquisition per secret logged, on a host
/// that has already said it is being debugged.
#[derive(Debug)]
struct KeyLogTo {
    file: Mutex<std::fs::File>,
}

impl KeyLogTo {
    /// The file opened for appending, created when it is not there.
    ///
    /// Appending and never truncating: the capture an operator wants is usually
    /// several runs of a program, and a restart that emptied the file would
    /// take the session they were reading with it.
    fn at(path: &Path) -> io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }
}

impl rustls::KeyLog for KeyLogTo {
    /// One `SSLKEYLOGFILE` line: the label, the client random and the secret,
    /// each hex, space separated.
    ///
    /// A failed write is dropped rather than reported. This is a debugging
    /// instrument bolted onto a request that was going to succeed, and a full
    /// disk turning outbound calls into failures would be the instrument
    /// deciding the deployment's availability.
    fn log(&self, label: &str, client_random: &[u8], secret: &[u8]) {
        let mut line =
            String::with_capacity(label.len() + (client_random.len() + secret.len()) * 2);
        line.push_str(label);
        line.push(' ');
        hex_into(&mut line, client_random);
        line.push(' ');
        hex_into(&mut line, secret);
        line.push('\n');
        let Ok(mut file) = self.file.lock() else {
            return;
        };
        drop(file.write_all(line.as_bytes()));
    }
}

/// `bytes` appended to `out` as lower-case hex.
fn hex_into(out: &mut String, bytes: &[u8]) {
    for byte in bytes {
        out.push(char::from_digit((u32::from(*byte)) >> 4, 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(*byte) & 0xf, 16).unwrap_or('0'));
    }
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
/// expects; and it is the stricter of the two readings, which decides it on
/// security. A server behind a private CA is precisely the deployment where
/// any public CA still being able to vouch for its name is the attack
/// the bundle was written to prevent. An operator who wants both writes both
/// into the file.
///
/// Cached per path for [`anchors`]'s reason and with its bound: a connection
/// pool re-opens against the same `[db.<name>]` block for the life of the
/// process, and re-parsing a bundle per handshake would spend latency on a
/// constant. The map is O(distinct bundles the configuration names), which is
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

    let mut roots = RootCertStore::empty();
    read_anchors(path, &mut roots)?;

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

/// The configuration one call runs under: the process's own where the call
/// named none of [`CallPolicy`]'s keys, and a session built for this call where
/// it named any.
///
/// **Built per call and released with it, never cached per policy.** A pin or a
/// PEM blob comes from the program rather than from the configuration, so a
/// cache keyed on one would be O(distinct values a program composes), which is
/// O(requests served) — the growth `rule:programs/memory-priority` calls a leak
/// rather than a footprint. What that costs is one `ClientConfig` on a path
/// that has already decided to do its own handshake, and [`anchors_from`]'s
/// cache stays where the key is a configured constant.
///
/// An `identity` is presented under either one, and under a default policy it
/// is the identity's own configuration that comes back — the relaxing branch is
/// the only one that pays for a session at all.
///
/// # Errors
///
/// `InvalidData` when `anchors` holds no certificate, and `InvalidInput` for a
/// pin that is not `sha256//` and the base64 of a 32-byte hash.
fn config_for(
    policy: &CallPolicy,
    identity: Option<&NvsIdentity>,
) -> io::Result<Arc<ClientConfig>> {
    if policy == &CallPolicy::default() {
        return Ok(identity.map_or_else(anchors, |held| Arc::clone(&held.config)));
    }
    let min_version = policy.min_version.as_deref();
    let session = match relaxing(policy)? {
        Some(verifier) => versioned(min_version)
            .dangerous()
            .with_custom_certificate_verifier(verifier),
        None => floored(min_version, asked_anchors(policy)?),
    };
    Ok(Arc::new(match identity {
        // The chain and the key were matched when the identity was read, so
        // what this can still refuse is a provider that cannot sign with the
        // key — and that one was refused there too.
        Some(held) => session
            .with_client_auth_cert(held.chain.clone(), held.key())
            .map_err(io::Error::other)?,
        None => session.with_no_client_auth(),
    }))
}

/// The verifier `policy` asks for, and `None` where it asks for the shipped one
/// over an anchor set [`asked_anchors`] answers.
///
/// The order is the widest first, which is what a call asking for two
/// relaxations gets: `tlsVerify: false` is a superset of the other three, and a
/// pin is an answer about the peer's key that a chain would not change.
fn relaxing(policy: &CallPolicy) -> io::Result<Option<Arc<dyn ServerCertVerifier>>> {
    if policy.insecure {
        return Ok(Some(Arc::new(PresentedKey {
            pinned: None,
            provider: provider(),
        })));
    }
    if !policy.pins.is_empty() {
        let pinned = policy
            .pins
            .iter()
            .map(String::as_str)
            .map(pinned_key)
            .collect::<io::Result<Vec<_>>>()?;
        return Ok(Some(Arc::new(PresentedKey {
            pinned: Some(pinned),
            provider: provider(),
        })));
    }
    if policy.any_name {
        let chain = WebPkiServerVerifier::builder_with_provider(asked_anchors(policy)?, provider())
            .build()
            .map_err(io::Error::other)?;
        return Ok(Some(Arc::new(AnyName { chain })));
    }
    Ok(None)
}

/// The anchor set this call builds a chain against: the certificates it handed
/// in, and the process's own where it handed none.
fn asked_anchors(policy: &CallPolicy) -> io::Result<Arc<RootCertStore>> {
    match policy.anchors.as_deref() {
        Some(pem) => Ok(Arc::new(anchors_of(pem)?)),
        None => Ok(configured_store()),
    }
}

/// Every certificate in the PEM text a call handed in, as a store.
///
/// [`read_anchors`]'s refusal, from text rather than from a file: a blob that
/// holds no certificate would verify nothing at all, and the handshake would
/// fail with an unknown issuer — a message that sends a programmer looking at
/// the server rather than at the string they passed.
fn anchors_of(pem: &str) -> io::Result<RootCertStore> {
    let refused = |why: &dyn std::fmt::Display| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("the certificates the call handed in name no trust anchor: {why}"),
        )
    };
    let mut store = RootCertStore::empty();
    for cert in CertificateDer::pem_slice_iter(pem.as_bytes()) {
        store
            .add(cert.map_err(|err| refused(&err))?)
            .map_err(|err| refused(&err))?;
    }
    if store.is_empty() {
        return Err(refused(&"it holds no certificate"));
    }
    Ok(store)
}

/// The `ring` provider, one per session that relaxes anything.
///
/// The same provider [`floored`] builds the shipped sessions over, named here
/// as well because a custom verifier is handed the signature algorithms
/// directly: a relaxed session verifies the handshake signature with exactly
/// the implementation a strict one does, and there is no second answer to
/// which algorithms are acceptable.
fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// `sha256//<base64>` — `curl --pinnedpubkey`'s spelling — as the 32 bytes it
/// names.
///
/// The spelling lives with the verifier that implements it rather than with the
/// caller that accepts it, so there is one reading of a pin in the tree. Every
/// refusal quotes the value, because the reader is the programmer who wrote it
/// into the call.
fn pinned_key(spelled: &str) -> io::Result<[u8; 32]> {
    let refused = |why: &dyn std::fmt::Display| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("`{spelled}` is not a public key pin: {why}"),
        )
    };
    let encoded = spelled.strip_prefix("sha256//").ok_or_else(|| {
        refused(&"a pin is `sha256//` and the base64 of a SHA-256 hash, as `curl --pinnedpubkey` spells it")
    })?;
    let raw = STANDARD
        .decode(encoded)
        .map_err(|err| refused(&format!("its base64 {err}")))?;
    let len = raw.len();
    raw.try_into()
        .map_err(|_| refused(&format!("a SHA-256 hash is 32 bytes and this names {len}")))
}

/// The SHA-256 of `cert`'s SubjectPublicKeyInfo, which is what a pin names.
///
/// The whole DER of that field, tag and length included, because that is what
/// `openssl pkey -pubin -outform der | sha256sum` produces and therefore what
/// every pin an operator already holds was computed over. The certificate is
/// parsed here rather than taken from `rustls`, which hands a custom verifier
/// unparsed DER by contract — a certificate this cannot read is `BadEncoding`
/// and not a pin failure, so the two are distinguishable in a trace.
fn spki_sha256(cert: &CertificateDer<'_>) -> Result<[u8; 32], rustls::Error> {
    let (_, parsed) = X509Certificate::from_der(cert.as_ref())
        .map_err(|_| rustls::Error::InvalidCertificate(CertificateError::BadEncoding))?;
    Ok(Sha256::digest(parsed.public_key().raw).into())
}

/// The verifier behind the two options that build no chain at all: `tlsPin`,
/// which accepts the peer whose key hashes to one of the values a call named,
/// and `tlsVerify: false`, which accepts any peer.
///
/// One type for both because they differ in one question — is this key one of
/// the keys named — and agree on everything else: no chain is built, no name is
/// checked, and the handshake signature is still verified against the key the
/// peer presented, so a peer that does not hold the key it sent is refused
/// either way.
#[derive(Debug)]
struct PresentedKey {
    /// The hashes the peer's key must be one of, and `None` where any key is
    /// accepted.
    pinned: Option<Vec<[u8; 32]>>,
    /// The provider whose signature algorithms the handshake signature is
    /// checked against.
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PresentedKey {
    /// The pinned hashes, against the key this peer presented — and, where
    /// nothing is pinned, nothing at all.
    ///
    /// The chain, the name, the validity dates and the OCSP response are all
    /// unread on purpose: a pin is a statement about one key, and a call that
    /// made it has said the chain is not the question.
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let Some(pinned) = self.pinned.as_deref() else {
            return Ok(ServerCertVerified::assertion());
        };
        if pinned.contains(&spki_sha256(end_entity)?) {
            return Ok(ServerCertVerified::assertion());
        }
        Err(rustls::Error::InvalidCertificate(
            CertificateError::ApplicationVerificationFailure,
        ))
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// `tlsVerifyHost: false`: the shipped verifier, with a name that does not
/// match accepted.
///
/// **The chain is built and checked first, and only its name question is
/// answered differently.** That rests on `rustls` verifying the path before it
/// verifies the name, which is the order `WebPkiServerVerifier` has and the
/// order this asks about: a name error is reachable only once the chain is
/// good, so accepting one accepts nothing about the path. If that ever
/// inverted, `any_name_policy_still_refuses_an_untrusted_chain` is the case
/// that goes red.
#[derive(Debug)]
struct AnyName {
    /// The verifier every question but the name is delegated to.
    chain: Arc<WebPkiServerVerifier>,
}

impl ServerCertVerifier for AnyName {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        match self.chain.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        ) {
            Err(rustls::Error::InvalidCertificate(
                CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. },
            )) => Ok(ServerCertVerified::assertion()),
            other => other,
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.chain.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.chain.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.chain.supported_verify_schemes()
    }
}

/// The shipped builder, stopped at the point where a client certificate is or
/// is not named.
///
/// The seam [`NvsIdentity`] plugs into, and the reason it is a seam rather than
/// a second builder: an identity changes what this end presents and nothing
/// about the versions, the cipher suites or the verifier, so a session that
/// presents one and a session that does not are the same session either way.
fn verifying(roots: impl Into<Arc<RootCertStore>>) -> ConfigBuilder<ClientConfig, WantsClientCert> {
    floored(None, roots)
}

/// [`verifying`] with `[http.client.tls] min_version` applied.
///
/// `None` and `"1.2"` are the same builder, because 1.2 is already the lowest
/// version this build speaks — `rustls` implements neither TLS 1.0 nor 1.1, so
/// there is no floor to lower and the shipped default *is* the floor. `"1.3"`
/// is the one value that changes anything, and it drops 1.2 rather than
/// preferring 1.3, which is the whole point of a floor.
///
/// A spelling that is neither cannot arrive: `nvs_config::http` refuses one at
/// boot with the line named (`E0639`), which is a better refusal than anything
/// reachable from here, so this treats an unknown value as the shipped floor
/// rather than growing a second, worse diagnostic for a case that is closed
/// upstream.
fn floored(
    min_version: Option<&str>,
    roots: impl Into<Arc<RootCertStore>>,
) -> ConfigBuilder<ClientConfig, WantsClientCert> {
    versioned(min_version).with_root_certificates(roots)
}

/// [`floored`], stopped one step earlier — before the anchors, which is where
/// `rustls` takes a verifier of its own instead.
///
/// The seam a relaxing [`CallPolicy`] plugs into, and the reason the floor is
/// applied on this side of it: the version a session speaks and whom it
/// believes are separate questions, so a call may raise one and relax the
/// other without either branch knowing about the other.
fn versioned(min_version: Option<&str>) -> ConfigBuilder<ClientConfig, WantsVerifier> {
    let provider = provider();
    let versions = match min_version {
        Some("1.3") => ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13]),
        _ => ClientConfig::builder_with_provider(provider).with_safe_default_protocol_versions(),
    };
    versions.expect("the ring provider refused a protocol version it implements")
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

    /// A path for a bundle in a scratch directory of the case's own, and the
    /// guard that deletes the directory when the case ends.
    fn scratch(name: &str) -> (nvs_repo::Scratch, std::path::PathBuf) {
        let dir = nvs_repo::scratch("anchors");
        let path = dir.join(format!("{name}.pem"));
        (dir, path)
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
        let (_dir, path) = scratch("parsed-once");
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
        let (_dir, path) = scratch("no-certificate");
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
        peer_speaking(cert, key, rustls::ALL_VERSIONS)
    }

    /// [`peer`], restricted to the protocol versions it is given.
    ///
    /// The one thing a version-floor case needs that the default peer cannot
    /// give it: an origin that speaks 1.2 and nothing else, which is what a
    /// floor is written to refuse.
    fn peer_speaking(
        cert: CertificateDer<'static>,
        key: PrivateKeyDer<'static>,
        versions: &'static [&'static rustls::SupportedProtocolVersion],
    ) -> (SocketAddr, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let handle = std::thread::spawn(move || {
            let config = rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_protocol_versions(versions)
            .expect("the provider refused the versions")
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .expect("the certificate and the key did not pair");
            let (mut sock, _) = listener.accept().expect("the accept failed");
            let mut conn = rustls::ServerConnection::new(Arc::new(config))
                .expect("the server config was rejected");
            // Late on purpose: the client is parked on this flight.
            std::thread::sleep(Duration::from_millis(20));
            // A client that refuses the certificate, or the version floor, hangs
            // up mid-handshake — which is the answer two of the cases below are
            // asserting, so it ends this thread quietly instead of panicking it.
            // Every claim is made on the client side.
            let mut tls = rustls::Stream::new(&mut conn, &mut sock);
            let mut heard = [0_u8; 5];
            if tls.read_exact(&mut heard).is_err() {
                return;
            }
            assert_eq!(&heard, b"ping\n");
            drop(tls.write_all(b"pong\n"));
            drop(tls.flush());
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

    /// A self-signed certificate for `localhost` and its key, with the PEM a
    /// `roots` entry would name written to `path`.
    ///
    /// The whole test-only part of the cases below: what they build from that
    /// path is [`built_from`], which is the function `configure` installs.
    fn issued_at(path: &std::path::Path) -> (CertificateDer<'static>, PrivateKeyDer<'static>) {
        let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .expect("the certificate could not be generated");
        std::fs::write(path, issued.cert.pem()).expect("the bundle could not be written");
        (
            issued.cert.der().clone(),
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(issued.signing_key.serialize_der())),
        )
    }

    /// [`built_over`] over the anchors `policy` names, which is the pair of
    /// steps a boot takes in one call.
    fn built_from(policy: &ClientPolicy) -> io::Result<ClientConfig> {
        built_over(policy, Arc::new(store_for(&policy.roots)?))
    }

    /// An installed client replaces the running one and moves the generation a
    /// pool files connections under, and the same anchors installed again
    /// change nothing.
    ///
    /// The only case in this binary that installs, so no other case's outcome
    /// depends on it: the client it leaves behind trusts the compiled-in set
    /// and one self-signed certificate nothing else here presents.
    #[test]
    fn an_installed_client_replaces_the_running_one_and_moves_the_generation() {
        let (_dir, path) = scratch("installed");
        drop(issued_at(&path));
        let wider = trusting(&[BUNDLED, path.to_str().expect("the scratch path is UTF-8")]);
        let before = generation();
        assert!(super::install(
            Client::build(&wider).expect("the bundle builds")
        ));
        let after = generation();
        assert!(after > before, "{after} is not past {before}");
        assert_eq!(
            configured_store().roots.len(),
            webpki_roots::TLS_SERVER_ROOTS.len() + 1,
            "the installed anchors are the ones a relaxed call builds its chain against"
        );
        assert!(
            !super::install(Client::build(&wider).expect("the bundle builds")),
            "the same anchors are not a change"
        );
        assert_eq!(generation(), after);
    }

    /// A policy naming `roots` and nothing else.
    fn trusting(roots: &[&str]) -> ClientPolicy {
        ClientPolicy {
            roots: roots.iter().map(|entry| (*entry).to_string()).collect(),
            ..ClientPolicy::default()
        }
    }

    /// One `ping`/`pong` exchange against `addr` under `config`, and whatever it
    /// came back with.
    ///
    /// Off a core deliberately, as `a_self_signed_certificate_is_refused_…` is:
    /// what these cases assert is which certificates a configuration accepts,
    /// and [`NvsTcp`]'s blocking path is the same `Read` and `Write` underneath.
    fn exchange(addr: SocketAddr, config: Arc<ClientConfig>) -> io::Result<String> {
        let sock = NvsTcp::connect(addr).expect("the connect failed");
        let mut tls = upgraded(sock, "localhost", config)?;
        tls.write_all(b"ping\n")?;
        tls.flush()?;
        let mut heard = [0_u8; 5];
        tls.read_exact(&mut heard)?;
        Ok(String::from_utf8_lossy(&heard).into_owned())
    }

    /// `["bundled", <file>]` is both sets and not one: the file's CA verifies
    /// its own origin, and the compiled-in anchors are still in the store.
    ///
    /// The count is the half worth asserting separately — a handshake against
    /// the file's CA would pass just as well if `bundled` had been dropped on
    /// the floor, and the list an operator wrote says `["bundled", …]` adds.
    #[test]
    fn roots_of_bundled_and_a_file_trust_the_files_ca() {
        let (_dir, path) = scratch("bundled-and-a-file");
        let (cert, key) = issued_at(&path);
        let named = path.to_string_lossy().into_owned();

        let both = store_for(&[BUNDLED.to_string(), named.clone()])
            .expect("the two-entry list was refused");
        let alone =
            store_for(std::slice::from_ref(&named)).expect("the one-entry list was refused");
        assert_eq!(
            both.len(),
            alone.len() + root_store().len(),
            "`bundled` beside a file did not add the compiled-in anchors to the file's",
        );

        let config =
            Arc::new(built_from(&trusting(&[BUNDLED, &named])).expect("the roots were refused"));
        let (addr, joined) = peer(cert, key);
        let heard = exchange(addr, config).expect("the handshake failed");
        joined.join().expect("the peer thread panicked");

        assert_eq!(heard, "pong\n");
        std::fs::remove_file(&path).ok();
    }

    /// A list of one file trusts that file and nothing else, which is the
    /// reading that makes a private-CA deployment worth configuring at all: a
    /// chain from any other CA is refused.
    #[test]
    fn roots_of_one_file_refuse_a_chain_from_another_ca() {
        let (_ours, ours) = scratch("one-file-ours");
        drop(issued_at(&ours));
        let (_theirs, theirs) = scratch("one-file-theirs");
        let (cert, key) = issued_at(&theirs);

        let config = Arc::new(
            built_from(&trusting(&[&ours.to_string_lossy()])).expect("the roots were refused"),
        );
        let (addr, joined) = peer(cert, key);
        let refused = exchange(addr, config).expect_err("a chain from another CA was accepted");
        drop(joined.join());

        assert_eq!(
            refused.kind(),
            io::ErrorKind::InvalidData,
            "a certificate no root vouches for came back as {refused}"
        );
        std::fs::remove_file(&ours).ok();
        std::fs::remove_file(&theirs).ok();
    }

    /// `min_version = "1.3"` drops 1.2 rather than preferring 1.3: an origin
    /// that speaks only 1.2 is refused, and the same origin is answered under
    /// the shipped floor.
    ///
    /// Both halves, because the refusal alone would pass on a client that could
    /// not talk to the peer for any reason at all — the second exchange is what
    /// says the floor is the only thing that changed.
    #[test]
    fn min_version_1_3_refuses_an_origin_that_speaks_only_1_2() {
        const ONLY_1_2: &[&rustls::SupportedProtocolVersion] = &[&rustls::version::TLS12];
        let (_dir, path) = scratch("min-version");
        let (cert, key) = issued_at(&path);
        let named = path.to_string_lossy().into_owned();

        let (addr, joined) = peer_speaking(cert.clone(), key.clone_key(), ONLY_1_2);
        let under_the_floor = ClientPolicy {
            min_version: Some("1.3".to_string()),
            ..trusting(&[&named])
        };
        let config = Arc::new(built_from(&under_the_floor).expect("the floor was refused"));
        let refused =
            exchange(addr, config).expect_err("a 1.2-only origin was answered under a 1.3 floor");
        drop(joined.join());
        assert_eq!(
            refused.kind(),
            io::ErrorKind::InvalidData,
            "the floor reported {refused} rather than refusing the version"
        );

        let (addr, joined) = peer_speaking(cert, key, ONLY_1_2);
        let shipped = Arc::new(built_from(&trusting(&[&named])).expect("the roots were refused"));
        let heard = exchange(addr, shipped).expect("the shipped floor refused a 1.2 origin");
        joined.join().expect("the peer thread panicked");
        assert_eq!(heard, "pong\n");

        std::fs::remove_file(&path).ok();
    }

    /// The key log is per session and appended to, not per process and
    /// truncated: two handshakes leave two sessions' secrets in the file, each
    /// an `SSLKEYLOGFILE` line.
    ///
    /// The distinct client randoms are what say it is two sessions rather than
    /// one written twice, which is the failure a truncating or a caching
    /// implementation would produce.
    #[test]
    fn keylog_appends_each_sessions_secrets() {
        let (dir, bundle) = scratch("keylog-bundle");
        let (cert, key) = issued_at(&bundle);
        let log = dir.join("keylog.log");

        let policy = ClientPolicy {
            keylog: Some(log.clone()),
            ..trusting(&[&bundle.to_string_lossy()])
        };
        let config = Arc::new(built_from(&policy).expect("the key log was refused"));
        for _ in 0..2 {
            let (addr, joined) = peer(cert.clone(), key.clone_key());
            let heard = exchange(addr, Arc::clone(&config)).expect("the handshake failed");
            joined.join().expect("the peer thread panicked");
            assert_eq!(heard, "pong\n");
        }

        let written = std::fs::read_to_string(&log).expect("nothing was written to the key log");
        let randoms: std::collections::BTreeSet<&str> = written
            .lines()
            .filter(|line| line.starts_with("CLIENT_HANDSHAKE_TRAFFIC_SECRET "))
            .filter_map(|line| line.split(' ').nth(1))
            .collect();
        assert_eq!(
            randoms.len(),
            2,
            "two sessions left {} client random(s) in the log:\n{written}",
            randoms.len()
        );
        for line in written.lines() {
            let fields: Vec<&str> = line.split(' ').collect();
            assert_eq!(fields.len(), 3, "`{line}` is not an `SSLKEYLOGFILE` line");
            assert!(
                fields[1..]
                    .iter()
                    .all(|field| field.chars().all(|c| c.is_ascii_hexdigit())),
                "`{line}` carries something that is not hex"
            );
        }

        std::fs::remove_file(&bundle).ok();
        std::fs::remove_file(&log).ok();
    }

    /// The two crates that each hold half of `roots` agree on the one word that
    /// is not a path.
    ///
    /// `nvs_config::http` decides which entries are files and this crate decides
    /// what the other one means, so a literal in each would be one typo away
    /// from a deployment silently trusting a file named `bundled` — or, worse,
    /// from a boot that trust-checked a path nothing ever opened.
    #[test]
    fn the_bundled_spelling_is_the_one_the_configuration_resolves() {
        assert_eq!(BUNDLED, nvs_config::http::BUNDLED);
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

    /// A self-signed `localhost` certificate, its key, and the two ways a call
    /// names it: the PEM text `tlsCa` takes and the `sha256//` pin `tlsPin`
    /// takes.
    ///
    /// The pin is derived from `rcgen`'s own SubjectPublicKeyInfo rather than
    /// from the certificate [`spki_sha256`] parses, so the cases below agree
    /// only if this module found the same field the key was written from — a
    /// pin computed by the code under test would pass against any field at all.
    fn named() -> (
        CertificateDer<'static>,
        PrivateKeyDer<'static>,
        String,
        String,
    ) {
        use base64::Engine as _;
        use rcgen::PublicKeyData as _;

        let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .expect("the certificate could not be generated");
        let pin = format!(
            "sha256//{}",
            STANDARD.encode(Sha256::digest(issued.signing_key.subject_public_key_info()))
        );
        (
            issued.cert.der().clone(),
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(issued.signing_key.serialize_der())),
            issued.cert.pem(),
            pin,
        )
    }

    /// One `ping`/`pong` exchange against `addr` under `policy`, through the
    /// door a call reaches.
    ///
    /// [`exchange`]'s body against [`NvsTls::over_policy`] rather than against
    /// a configuration, because what these cases are about is the value a
    /// caller is allowed to hand in — a case that built the `ClientConfig`
    /// itself would assert nothing about the door that refuses to take one.
    fn relaxed(addr: SocketAddr, name: &str, policy: &CallPolicy) -> io::Result<String> {
        let sock = NvsTcp::connect(addr).expect("the connect failed");
        let mut tls = NvsTls::over_policy(sock, name, policy, None)?;
        tls.write_all(b"ping\n")?;
        tls.flush()?;
        let mut heard = [0_u8; 5];
        tls.read_exact(&mut heard)?;
        Ok(String::from_utf8_lossy(&heard).into_owned())
    }

    /// `tlsCa` is `CAINFO_BLOB`: the certificates the call handed in are the
    /// whole trust set for it, so its own origin verifies and one from any
    /// other CA does not.
    ///
    /// Both halves, because the acceptance alone would pass just as well on a
    /// client that had stopped checking anything.
    #[test]
    fn anchors_policy_trusts_only_the_given_certificates() {
        let (cert, key, pem, _) = named();
        let (other, other_key, _, _) = named();
        let policy = CallPolicy {
            anchors: Some(pem),
            ..CallPolicy::default()
        };

        let (addr, joined) = peer(cert, key);
        let heard = relaxed(addr, "localhost", &policy).expect("the handshake failed");
        joined.join().expect("the peer thread panicked");
        assert_eq!(heard, "pong\n");

        let (elsewhere, hung_up) = peer(other, other_key);
        let refused = relaxed(elsewhere, "localhost", &policy)
            .expect_err("a CA the call did not name was trusted");
        drop(hung_up.join());
        assert_eq!(
            refused.kind(),
            io::ErrorKind::InvalidData,
            "a certificate from another CA came back as {refused}"
        );
    }

    /// `tlsPin` is `--pinnedpubkey`: a peer no chain vouches for is accepted on
    /// the strength of its key alone.
    ///
    /// The same self-signed certificate
    /// `a_self_signed_certificate_is_refused_by_the_compiled_in_anchors`
    /// refuses, so the pin is the only thing that changed.
    #[test]
    fn pin_policy_accepts_a_self_signed_peer_whose_key_matches() {
        let (cert, key, _, pin) = named();
        let policy = CallPolicy {
            pins: vec![pin],
            ..CallPolicy::default()
        };

        let (addr, joined) = peer(cert, key);
        let heard = relaxed(addr, "localhost", &policy).expect("the pinned peer was refused");
        joined.join().expect("the peer thread panicked");
        assert_eq!(heard, "pong\n");
    }

    /// A pin is a statement about one key: another key's hash refuses the
    /// handshake, and a value that is not a pin at all is refused before a
    /// socket is opened.
    #[test]
    fn pin_policy_refuses_a_peer_whose_key_does_not_match() {
        let (cert, key, _, _) = named();
        let (_, _, _, elsewhere) = named();
        let policy = CallPolicy {
            pins: vec![elsewhere],
            ..CallPolicy::default()
        };

        let (addr, hung_up) = peer(cert, key);
        let refused =
            relaxed(addr, "localhost", &policy).expect_err("a peer whose key was not pinned");
        drop(hung_up.join());
        assert_eq!(
            refused.kind(),
            io::ErrorKind::InvalidData,
            "an unpinned key came back as {refused}"
        );

        let misspelled = config_for(
            &CallPolicy {
                pins: vec!["deadbeef".to_owned()],
                ..CallPolicy::default()
            },
            None,
        )
        .expect_err("a value that is not a pin was taken as one");
        assert_eq!(
            misspelled.kind(),
            io::ErrorKind::InvalidInput,
            "a misspelled pin came back as {misspelled}"
        );
    }

    /// `tlsVerifyHost: false` is `SSL_VERIFYHOST=0`: the name is skipped and
    /// the chain is not.
    ///
    /// The first half is the option working — a certificate for `localhost`
    /// answers a call to another name — and the second is the half that makes
    /// it worth having separately from `tlsVerify: false`: the same call
    /// against a chain no anchor vouches for is still refused.
    #[test]
    fn any_name_policy_still_refuses_an_untrusted_chain() {
        let (cert, key, pem, _) = named();
        let skipping = CallPolicy {
            anchors: Some(pem),
            any_name: true,
            ..CallPolicy::default()
        };

        let (addr, joined) = peer(cert, key);
        let heard = relaxed(addr, "elsewhere.invalid", &skipping).expect("the name was checked");
        joined.join().expect("the peer thread panicked");
        assert_eq!(heard, "pong\n");

        let (elsewhere, its_key, _, _) = named();
        let (untrusted, hung_up) = peer(elsewhere, its_key);
        let refused = relaxed(
            untrusted,
            "localhost",
            &CallPolicy {
                any_name: true,
                ..CallPolicy::default()
            },
        )
        .expect_err("a chain no anchor vouches for was accepted");
        drop(hung_up.join());
        assert_eq!(
            refused.kind(),
            io::ErrorKind::InvalidData,
            "an untrusted chain came back as {refused}"
        );
    }

    /// `tlsVerify: false` is `-k`: neither the chain nor the name is asked
    /// about, and the exchange completes against a peer nothing vouches for.
    #[test]
    fn insecure_policy_completes_against_an_untrusted_peer() {
        let (cert, key, _, _) = named();
        let policy = CallPolicy {
            insecure: true,
            ..CallPolicy::default()
        };

        let (addr, joined) = peer(cert, key);
        let heard = relaxed(addr, "elsewhere.invalid", &policy).expect("the peer was refused");
        joined.join().expect("the peer thread panicked");
        assert_eq!(heard, "pong\n");
    }

    /// An identity and a relaxing option are one session rather than a choice
    /// between them, which is what mutual TLS against a private CA is: the call
    /// that decides whom to believe is the same call that proves who it is.
    ///
    /// The second half is the fast path the first must not have cost: a call
    /// that relaxes nothing still gets the identity's own configuration back,
    /// built once when the identity was read.
    #[test]
    fn an_identity_is_presented_under_a_relaxed_policy_as_well() {
        let issued = rcgen::generate_simple_self_signed(vec!["client.example".to_owned()])
            .expect("the certificate could not be generated");
        let identity = NvsIdentity::read(
            issued.cert.pem().as_bytes(),
            &issued.signing_key.serialize_der(),
        )
        .expect("the chain and its own key were refused");

        let relaxing = config_for(
            &CallPolicy {
                insecure: true,
                ..CallPolicy::default()
            },
            Some(&identity),
        )
        .expect("the relaxed session was refused");
        assert!(
            relaxing.client_auth_cert_resolver.has_certs(),
            "a relaxed session dropped the certificate the call presents"
        );

        let plain = config_for(&CallPolicy::default(), Some(&identity))
            .expect("the identity's own session was refused");
        assert!(
            Arc::ptr_eq(&plain, &identity.config),
            "a call that relaxes nothing built a second session instead of reusing the identity's"
        );
    }

    /// Which of the five option fields is a relaxation, as one table: `tlsCa`
    /// and `tlsMinVersion` still check the chain and the name, and each of the
    /// other three drops one of them
    /// (`rule:http-server/a-reply-reports-its-tls-session`, ADR 0180 § 13).
    ///
    /// Every field, in one case, because the answer is a statement about the
    /// *set*: a reading that missed one would report a call as verified that
    /// checked nothing, and the two fields that are not relaxations are the pair
    /// a plausible reading gets wrong. No handshake, because what each option
    /// does on the wire is what the cases above already prove.
    #[test]
    fn a_policy_verifies_unless_it_dropped_the_chain_or_the_name() {
        for (policy, verifies, why) in [
            (CallPolicy::default(), true, "the default relaxes nothing"),
            (
                CallPolicy {
                    anchors: Some("a bundle the call handed over".to_owned()),
                    ..CallPolicy::default()
                },
                true,
                "`tlsCa` replaces the trust set and then checks against it",
            ),
            (
                CallPolicy {
                    min_version: Some("1.3".to_owned()),
                    ..CallPolicy::default()
                },
                true,
                "`tlsMinVersion` only tightens",
            ),
            (
                CallPolicy {
                    pins: vec!["sha256//a key this case never hashes".to_owned()],
                    ..CallPolicy::default()
                },
                false,
                "a pin accepts a key with no chain built at all",
            ),
            (
                CallPolicy {
                    any_name: true,
                    ..CallPolicy::default()
                },
                false,
                "`tlsVerifyHost: false` drops the name",
            ),
            (
                CallPolicy {
                    insecure: true,
                    ..CallPolicy::default()
                },
                false,
                "`tlsVerify: false` drops both",
            ),
        ] {
            assert_eq!(policy.verifies(), verifies, "{why}");
        }
    }

    /// A description with every field a test may leave out left out, at a
    /// fixed clock and expiry.
    fn describing(
        version: Option<&'static str>,
        cipher: Option<&'static str>,
    ) -> Description<'static> {
        Description {
            version,
            cipher,
            subject: "CN=api.example.com, O=Shop",
            issuer: "CN=Example Test CA",
            expiry: UNIX_EPOCH + Duration::from_secs(1_938_000_000),
            now: UNIX_EPOCH + Duration::from_secs(1_900_000_000),
        }
    }

    /// A described session carries the version and suite it was given or the
    /// defaults, and a two-certificate chain whose leaf reads back, through
    /// the same [`leaf`] a peer's does, the subject, issuer and expiry written.
    // covers: Core\Test::tlsSession
    #[test]
    fn a_described_session_reads_back_what_was_written() {
        let session = described(&describing(None, None)).expect("the defaults describe a session");
        assert_eq!(session.version(), "TLSv1.3");
        assert_eq!(session.cipher(), "TLS_AES_128_GCM_SHA256");
        assert_eq!(
            session.chain().len(),
            2,
            "the leaf, then the CA that signed it"
        );
        let read = leaf(&session.chain()[0]).expect("the leaf parses");
        assert_eq!(read.subject(), "CN=api.example.com, O=Shop");
        assert_eq!(read.issuer(), "CN=Example Test CA");
        assert_eq!(
            read.expiry(),
            UNIX_EPOCH + Duration::from_secs(1_938_000_000)
        );
        let authority = leaf(&session.chain()[1]).expect("the CA certificate parses");
        assert_eq!(authority.subject(), "CN=Example Test CA");

        let older = described(&describing(Some("TLSv1.2"), None)).expect("TLS 1.2 is described");
        assert_eq!(older.version(), "TLSv1.2");
        assert_eq!(older.cipher(), "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256");
    }

    /// A value holding one of RFC 4514's special characters reads back escaped,
    /// so a one-attribute name never reads as the same text as a name with two:
    /// `CN=x+O=y` written as one common name is `CN=x\+O=y`, and a comma inside
    /// an organization is `\,`.
    #[test]
    fn a_name_value_reads_back_escaped_as_rfc_4514_writes_it() {
        let session = described(&Description {
            subject: "CN=x+O=y",
            issuer: "CN=#1 \"Test\"; <CA> a\\b",
            ..describing(None, None)
        })
        .expect("special characters are allowed in a value");
        let read = leaf(&session.chain()[0]).expect("the leaf parses");
        assert_eq!(read.subject(), "CN=x\\+O=y");
        assert_eq!(read.issuer(), "CN=\\#1 \\\"Test\\\"\\; \\<CA\\> a\\\\b");

        let mut name = rcgen::DistinguishedName::new();
        name.push(rcgen::DnType::OrganizationName, "Shop, Inc.");
        let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).expect("no names");
        params.distinguished_name = name;
        let key = rcgen::KeyPair::generate().expect("a key");
        let issued = params.self_signed(&key).expect("a certificate");
        let read = leaf(issued.der()).expect("the certificate parses");
        assert_eq!(read.subject(), "O=Shop\\, Inc.");

        let mut out = String::new();
        escape_value(" a\0b ", &mut out);
        assert_eq!(out, "\\ a\\00b\\ ");
    }

    /// Every default suite is one this build negotiates over its version, so a
    /// session with no `cipher` never fails its own check.
    #[test]
    fn every_default_suite_is_one_this_build_negotiates() {
        for (version, suite) in DEFAULT_SUITES {
            let session = described(&describing(Some(version), Some(suite)))
                .unwrap_or_else(|why| panic!("{version} {suite}: {why}"));
            assert_eq!(session.cipher(), suite);
        }
    }

    /// A version, a suite, a name or an expiry the session cannot hold is
    /// refused with a sentence naming the field.
    // covers: Core\Test::tlsSession
    #[test]
    fn a_description_the_session_cannot_hold_is_refused() {
        let refused = |description: Description<'static>, field: &str| {
            let why = described(&description).expect_err(field);
            assert!(why.contains(&format!("`{field}`")), "{field}: {why}");
        };
        refused(describing(Some("TLSv1.1"), None), "version");
        refused(
            describing(
                Some("TLSv1.3"),
                Some("TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256"),
            ),
            "cipher",
        );
        for subject in ["api.example.com", "CN=", "XX=api", "CN=a, CN=b"] {
            refused(
                Description {
                    subject,
                    ..describing(None, None)
                },
                "subject",
            );
        }
        refused(
            Description {
                issuer: "",
                ..describing(None, None)
            },
            "issuer",
        );
        refused(
            Description {
                expiry: UNIX_EPOCH - Duration::from_secs(700_000_000),
                ..describing(None, None)
            },
            "expiry",
        );
    }
}
