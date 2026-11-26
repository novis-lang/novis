//! `[tls.client] roots` from outside the process: every outbound client that names no
//! `tls_ca_file` trusts the CAs that list names, a block's `tls_ca_file` replaces the list for that
//! block, and a failed handshake names the setting that fixes it.
//!
//! The client configuration is process-wide, so each case runs the built `nvs run` with a settings
//! file of its own, never a configuration installed inside this test binary. Its peer is a
//! loopback [`Peer`] whose certificate a CA made here signed. The peer reports whether each
//! handshake completed, and a completed handshake is the pass: the protocol after it fails,
//! because the peer closes the connection at once. A peer speaks TLS from its first byte, as
//! `ldaps://`, mail's `tls = "implicit"` and `rediss://` do, or answers PostgreSQL's `SSLRequest`
//! with `S` first. MySQL and SQL Server reach the same TLS client through the same call in
//! `nvs-db` as PostgreSQL, so they get no case of their own.

use std::fs;
use std::io::Read;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

/// How long a peer waits on one read, and a case waits on the peer's report.
const BOUND: Duration = Duration::from_secs(30);

/// A certificate authority made for one case, and its certificate as PEM.
struct Ca {
    params: CertificateParams,
    key: KeyPair,
    pem: String,
}

impl Ca {
    /// A CA whose common name is `name`. Two CAs of one case have different names, so a
    /// verifier tells them apart by name and not only by key.
    fn new(name: &str) -> Self {
        let key = KeyPair::generate().expect("a key pair is generated");
        let mut params = CertificateParams::new(Vec::<String>::new()).expect("no names are valid");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.distinguished_name.push(DnType::CommonName, name);
        let pem = params
            .self_signed(&key)
            .expect("the CA certificate is issued")
            .pem();
        Self { params, key, pem }
    }

    /// A leaf certificate for `names`, signed by this CA, and its private key.
    fn leaf(&self, names: &[&str]) -> (CertificateDer<'static>, PrivateKeyDer<'static>) {
        let key = KeyPair::generate().expect("a key pair is generated");
        let names: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
        let issuer = Issuer::from_params(&self.params, &self.key);
        let cert = CertificateParams::new(names)
            .expect("the names are valid")
            .signed_by(&key, &issuer)
            .expect("the leaf certificate is issued");
        let der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der()));
        (cert.der().clone(), der)
    }
}

/// What a client sends before its TLS handshake.
#[derive(Clone, Copy)]
enum Opening {
    /// Nothing: the handshake is the first thing on the connection.
    Tls,
    /// PostgreSQL's eight-byte `SSLRequest`, which the peer answers with `S`.
    Postgres,
}

/// A loopback TLS server. Each connection's outcome arrives on [`Peer::outcome`]: `Ok` for a
/// handshake that completed, and the server's error for one that did not.
struct Peer {
    addr: SocketAddr,
    outcome: mpsc::Receiver<Result<(), String>>,
}

impl Peer {
    /// Binds a free loopback port and serves every connection with `leaf` on a thread of its own.
    fn start(leaf: (CertificateDer<'static>, PrivateKeyDer<'static>), opening: Opening) -> Self {
        let config = Arc::new(
            rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .expect("the provider speaks the default versions")
            .with_no_client_auth()
            .with_single_cert(vec![leaf.0], leaf.1)
            .expect("the certificate and its key match"),
        );
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port is free");
        let addr = listener
            .local_addr()
            .expect("a bound socket has an address");
        let (sender, outcome) = mpsc::channel();
        thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                let config = Arc::clone(&config);
                let sender = sender.clone();
                thread::spawn(move || drop(sender.send(handshake(stream, config, opening))));
            }
        });
        Self { addr, outcome }
    }

    /// The outcome of the first connection, waited on for up to [`BOUND`]. A client that never
    /// connected is an `Err` too.
    fn first(&self) -> Result<(), String> {
        self.outcome
            .recv_timeout(BOUND)
            .unwrap_or_else(|_| Err("no connection arrived".to_owned()))
    }
}

/// Reads `opening`'s bytes from `stream`, completes a TLS handshake on it, and closes it.
fn handshake(
    mut stream: TcpStream,
    config: Arc<rustls::ServerConfig>,
    opening: Opening,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(BOUND))
        .map_err(|error| error.to_string())?;
    if let Opening::Postgres = opening {
        let mut request = [0_u8; 8];
        stream
            .read_exact(&mut request)
            .map_err(|error| format!("no SSLRequest: {error}"))?;
        std::io::Write::write_all(&mut stream, b"S").map_err(|error| error.to_string())?;
    }
    let mut conn = rustls::ServerConnection::new(config).map_err(|error| error.to_string())?;
    while conn.is_handshaking() {
        conn.complete_io(&mut stream)
            .map_err(|error| format!("the handshake failed: {error}"))?;
    }
    Ok(())
}

/// What one `nvs run` printed on standard output and standard error.
struct Ran {
    out: String,
}

/// Writes `settings` as `nvs.toml` and `program` as `main.nvs` into `dir`, beside each `(name,
/// text)` of `files`, and runs `nvs run main.nvs` there.
fn run(dir: &Path, settings: &str, program: &str, files: &[(&str, &str)]) -> Ran {
    fs::write(dir.join("nvs.toml"), settings).expect("the settings file is written");
    fs::write(dir.join("main.nvs"), program).expect("the program is written");
    for (name, text) in files {
        fs::write(dir.join(name), text).expect("the file is written");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(dir.join(".nvsdata"))
        .arg("run")
        .arg("main.nvs")
        .current_dir(dir)
        .env_remove("NOVIS_NO_INIT")
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    Ran {
        out: format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    }
}

/// A directory of the case's own, locked to this account so its settings file and PEM files pass
/// the ownership check.
fn scratch(case: &str) -> nvs_repo::Scratch {
    nvs_repo::scratch_private(&format!("client-roots-{case}"))
}

/// `[tls.client]` trusting `ca.pem` alone, or the compiled-in set when `roots` is `false`, beside
/// the grants every case needs to reach loopback.
fn settings(dir: &Path, roots: bool, rest: &str) -> String {
    let block = if roots {
        format!("[tls.client]\nroots = ['{}']\n\n", dir.join("ca.pem").display())
    } else {
        String::new()
    };
    format!(
        "{block}[capabilities.net]\nconnect = [\"127.0.0.1\"]\ninternal = [\"127.0.0.1\"]\n\n{rest}"
    )
}

/// An `[ldap.corp]` block at `peer`, with `tls_ca_file` when `ca_file` is given, and its grant.
fn ldap_block(peer: &Peer, ca_file: Option<&Path>) -> String {
    let ca = ca_file.map_or_else(String::new, |path| {
        format!("tls_ca_file = '{}'\n", path.display())
    });
    format!(
        "[ldap.corp]\nurl = \"ldaps://127.0.0.1:{}\"\nbase = \"DC=example,DC=test\"\n\
         user = \"CN=Reader,DC=example,DC=test\"\npassword = \"Example-Pw1\"\ntimeout = \"5s\"\n\
         {ca}\n[capabilities.ldap]\nconnect = [\"corp\"]\n",
        peer.addr.port()
    )
}

/// Runs `call` and prints the message of what it throws, after `failed: `.
fn program(call: &str) -> String {
    format!(
        "<?nvs\ntry {{\n    {call}\n    echo \"done\\n\";\n}} catch (Throwable $error) {{\n    \
         echo 'failed: ', $error->message, \"\\n\";\n}}\n"
    )
}

/// Opens `[ldap.corp]` and asks who it is bound as.
const LDAP_CONNECT: &str = "echo Core\\Ldap::connect('corp')->whoami();";

/// `Core\Ldap::open` at `peer`, which the program names itself.
fn ldap_open(peer: &Peer) -> String {
    format!(
        "echo Core\\Ldap::open({{ url: 'ldaps://127.0.0.1:{}', user: 'CN=Reader,DC=example,DC=test', \
         password: 'Example-Pw1', timeout: Core\\Time\\Duration::seconds(5) }})->whoami();",
        peer.addr.port()
    )
}

/// The `[capabilities.ldap]` grant `Core\Ldap::open` needs for loopback.
const LDAP_OPEN_GRANT: &str = "[capabilities.ldap]\nopen = [\"127.0.0.1\"]\n";

/// Asserts that the peer completed a handshake, and shows what the run printed if not.
fn trusted(peer: &Peer, ran: &Ran) {
    if let Err(error) = peer.first() {
        panic!(
            "the client did not trust the CA in `[tls.client] roots`: {error}\nthe run printed: {}",
            ran.out
        );
    }
}

/// `Core\Ldap::open` names no CA file, so it trusts `[tls.client] roots`.
#[test]
fn ldap_open_trusts_a_ca_in_tls_client_roots() {
    let dir = scratch("ldap-open");
    let ca = Ca::new("Example Roots CA");
    let peer = Peer::start(ca.leaf(&["127.0.0.1"]), Opening::Tls);
    let ran = run(
        &dir,
        &settings(&dir, true, LDAP_OPEN_GRANT),
        &program(&ldap_open(&peer)),
        &[("ca.pem", &ca.pem)],
    );
    trusted(&peer, &ran);
}

/// An `[ldap.<name>]` block with no `tls_ca_file` trusts `[tls.client] roots`.
#[test]
fn an_ldap_block_without_a_ca_file_trusts_tls_client_roots() {
    let dir = scratch("ldap-block");
    let ca = Ca::new("Example Roots CA");
    let peer = Peer::start(ca.leaf(&["127.0.0.1"]), Opening::Tls);
    let ran = run(
        &dir,
        &settings(&dir, true, &ldap_block(&peer, None)),
        &program(LDAP_CONNECT),
        &[("ca.pem", &ca.pem)],
    );
    trusted(&peer, &ran);
}

/// A `[mail.<name>]` block with `tls = "implicit"` and no `tls_ca_file` trusts `[tls.client]
/// roots`.
#[test]
fn a_mail_block_without_a_ca_file_trusts_tls_client_roots() {
    let dir = scratch("mail-block");
    let ca = Ca::new("Example Roots CA");
    let peer = Peer::start(ca.leaf(&["127.0.0.1"]), Opening::Tls);
    let block = format!(
        "[mail.shop]\nhost = \"127.0.0.1\"\nport = {}\ntls = \"implicit\"\n\
         from = \"shop@example.com\"\ntimeout = \"5s\"\n\n[capabilities.mail]\nsend = [\"shop\"]\n",
        peer.addr.port()
    );
    let ran = run(
        &dir,
        &settings(&dir, true, &block),
        &program("Core\\Mail::send('shop', ['anna@example.com'], 'Hello', 'Hello');"),
        &[("ca.pem", &ca.pem)],
    );
    trusted(&peer, &ran);
}

/// A PostgreSQL `[db.<name>]` block with no `tls_ca_file` trusts `[tls.client] roots` after its
/// `SSLRequest`.
#[test]
fn a_postgres_block_without_a_ca_file_trusts_tls_client_roots() {
    let dir = scratch("postgres-block");
    let ca = Ca::new("Example Roots CA");
    let peer = Peer::start(ca.leaf(&["127.0.0.1"]), Opening::Postgres);
    let block = format!(
        "[db.main]\ndriver = \"postgres\"\nhost = \"127.0.0.1\"\nport = {}\nuser = \"app\"\n\
         password = \"Example-Pw1\"\ndatabase = \"app\"\n\n[capabilities.db]\nconnect = [\"main\"]\n",
        peer.addr.port()
    );
    let ran = run(
        &dir,
        &settings(&dir, true, &block),
        &program("Core\\Db::connect('main')->query('select 1', []);"),
        &[("ca.pem", &ca.pem)],
    );
    trusted(&peer, &ran);
}

/// A shared cache at a `rediss://` URL trusts `[tls.client] roots`.
#[test]
fn a_shared_cache_over_tls_trusts_tls_client_roots() {
    let dir = scratch("shared-cache");
    let ca = Ca::new("Example Roots CA");
    let peer = Peer::start(ca.leaf(&["127.0.0.1"]), Opening::Tls);
    let block = format!(
        "[cache.shared]\nurl = \"rediss://127.0.0.1:{}\"\ntimeout = \"5s\"\n\n\
         [capabilities.cache]\nshared = true\n",
        peer.addr.port()
    );
    let ran = run(
        &dir,
        &settings(&dir, true, &block),
        &program("Core\\Cache::shared()->get('key');"),
        &[("ca.pem", &ca.pem)],
    );
    trusted(&peer, &ran);
}

/// A block's `tls_ca_file` replaces `[tls.client] roots` for that block: the roots trust the
/// peer's CA, the block names another CA, and the handshake fails.
#[test]
fn a_block_ca_file_replaces_tls_client_roots() {
    let dir = scratch("ca-file-replaces");
    let ca = Ca::new("Example Roots CA");
    let other = Ca::new("Example Other CA");
    let peer = Peer::start(ca.leaf(&["127.0.0.1"]), Opening::Tls);
    let ran = run(
        &dir,
        &settings(&dir, true, &ldap_block(&peer, Some(&dir.join("other.pem")))),
        &program(LDAP_CONNECT),
        &[("ca.pem", &ca.pem), ("other.pem", &other.pem)],
    );
    assert!(
        peer.first().is_err(),
        "the block trusted a CA its `tls_ca_file` does not name: {}",
        ran.out
    );
    assert!(ran.out.contains("failed: "), "the call did not throw: {}", ran.out);
}

/// `Core\Ldap::open` against a CA nobody trusts names `[tls.client] roots`, and no block setting,
/// because the program wrote the URL.
#[test]
fn an_unknown_issuer_from_ldap_open_names_tls_client_roots() {
    let dir = scratch("ldap-open-unknown");
    let ca = Ca::new("Example Unknown CA");
    let peer = Peer::start(ca.leaf(&["127.0.0.1"]), Opening::Tls);
    let ran = run(
        &dir,
        &settings(&dir, false, LDAP_OPEN_GRANT),
        &program(&ldap_open(&peer)),
        &[],
    );
    assert!(peer.first().is_err(), "the handshake completed: {}", ran.out);
    assert!(
        ran.out.contains("`[tls.client] roots`"),
        "the error does not name `[tls.client] roots`: {}",
        ran.out
    );
    assert!(
        !ran.out.contains("tls_ca_file"),
        "the error names a block setting the program cannot use: {}",
        ran.out
    );
}

/// An `[ldap.<name>]` block against a CA nobody trusts names its own `tls_ca_file` and
/// `[tls.client] roots`.
#[test]
fn an_unknown_issuer_from_a_block_names_both_settings() {
    let dir = scratch("ldap-block-unknown");
    let ca = Ca::new("Example Unknown CA");
    let peer = Peer::start(ca.leaf(&["127.0.0.1"]), Opening::Tls);
    let ran = run(
        &dir,
        &settings(&dir, false, &ldap_block(&peer, None)),
        &program(LDAP_CONNECT),
        &[],
    );
    assert!(peer.first().is_err(), "the handshake completed: {}", ran.out);
    for setting in ["`[ldap.corp] tls_ca_file`", "`[tls.client] roots`"] {
        assert!(
            ran.out.contains(setting),
            "the error does not name {setting}: {}",
            ran.out
        );
    }
}

/// A certificate a trusted CA issued for another name lists the names it is for and the block's
/// `url` that fixes it. It names no CA setting, because no CA setting fixes a name.
#[test]
fn a_certificate_for_another_name_lists_its_names() {
    let dir = scratch("another-name");
    let ca = Ca::new("Example Roots CA");
    let peer = Peer::start(
        ca.leaf(&["ldap.example.com", "directory.example.com"]),
        Opening::Tls,
    );
    let ran = run(
        &dir,
        &settings(&dir, true, &ldap_block(&peer, None)),
        &program(LDAP_CONNECT),
        &[("ca.pem", &ca.pem)],
    );
    assert!(peer.first().is_err(), "the handshake completed: {}", ran.out);
    for name in [
        "`ldap.example.com`",
        "`directory.example.com`",
        "`[ldap.corp] url`",
    ] {
        assert!(
            ran.out.contains(name),
            "the error does not name {name}: {}",
            ran.out
        );
    }
    assert!(
        !ran.out.contains("tls_ca_file") && !ran.out.contains("roots"),
        "the error names a CA setting, which cannot fix a name: {}",
        ran.out
    );
}
