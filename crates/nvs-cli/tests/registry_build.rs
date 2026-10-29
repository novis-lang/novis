//! `nvs registry build` against a git repository served over HTTPS on loopback, and
//! `nvs pkg publish --dry-run` against the same commit
//! (`rule:packaging/the-registry-builds-the-archive-from-git`).
//!
//! Each test owns a private folder with a repository the `git` command line builds, a data
//! folder, and a working folder whose `nvs.toml` writes `internal = ["127.0.0.1"]`, the one key
//! that lets a clone reach loopback. The host is `git http-backend`, the CGI program every git
//! install ships, behind a `rustls` listener with a certificate `rcgen` issues per test. The
//! build trusts it through `--ca-file`. The host counts the bytes it sends, so a test can show
//! the clone fetched one commit and none of its history.
//!
//! Every git command runs with no system or global configuration, and the repository writes
//! `core.autocrlf = false`, so a file has the same bytes on every runner.
//!
//! The last test serves a copy of `apps/registry` with `nvs serve` and runs `nvs pkg login`,
//! `whoami`, `publish`, `retract` and `unlist` against it
//! (`rule:packaging/a-publish-token-is-scoped-and-expires`). The test signs up over HTTP with an
//! Ed25519 passkey it answers the WebAuthn options with, and the service builds the release from
//! the same git host.

use std::fs;
use std::io::{self, BufRead, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::Value;

/// The longest the host waits for a request it has accepted. A bound on a stuck client, never a
/// wait the tests rely on.
const BOUND: Duration = Duration::from_secs(30);

/// The size limit of a clone, `git::LIMITS.bytes`.
const CLONE_LIMIT: u64 = 512 * 1024 * 1024;

/// The most bytes a package's files may have together, `pack::MAX_BYTES`.
const PACK_LIMIT: u64 = 16 * 1024 * 1024;

/// The package's manifest at `version`.
fn manifest(version: &str) -> String {
    format!(
        "[package]\nname = \"Acme\\\\Http\"\nversion = \"{version}\"\n\n[capabilities]\nrequired = [\"net.connect\"]\n"
    )
}

/// The package's one class, with a second public method when `label` is set.
fn client(label: bool) -> String {
    let extra = if label {
        "\n    public static function label(): string\n    {\n        return 'label';\n    }\n"
    } else {
        ""
    };
    format!(
        "<?nvs\nnamespace Acme\\Http;\n\nfinal class Client\n{{\n    public static function \
         name(): string\n    {{\n        return 'client';\n    }}\n{extra}}}\n"
    )
}

/// One test's private folder: a repository at `repo/`, a data folder, and a working folder with
/// the `nvs.toml` that allows loopback.
struct Case {
    root: nvs_repo::Scratch,
}

impl Case {
    fn new(name: &str) -> Self {
        let case = Self {
            root: nvs_repo::scratch_private(name),
        };
        for dir in [case.repo(), case.data(), case.work()] {
            fs::create_dir_all(dir).expect("the test makes its folders");
        }
        fs::write(
            case.work().join("nvs.toml"),
            "[capabilities.net]\nconnect = [\"127.0.0.1\"]\ninternal = [\"127.0.0.1\"]\n",
        )
        .expect("the test writes the configuration");
        case.git(&["init", "-q"]);
        case.git(&["config", "core.autocrlf", "false"]);
        case.git(&["config", "uploadpack.allowAnySHA1InWant", "true"]);
        case.git(&["config", "core.bigFileThreshold", "1m"]);
        case
    }

    fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    fn data(&self) -> PathBuf {
        self.root.join("data")
    }

    fn work(&self) -> PathBuf {
        self.root.join("work")
    }

    /// The certificate of the case's host, which `--ca-file` names.
    fn ca(&self) -> PathBuf {
        self.root.join("ca.pem")
    }

    /// A global git configuration file that does not exist, so git reads none.
    fn no_config(&self) -> PathBuf {
        self.root.join("no-global-gitconfig")
    }

    /// `git <args>` in the repository. Its standard output, trimmed.
    fn git(&self, args: &[&str]) -> String {
        let mut command = nvs_repo::spawn("git", &[]);
        isolated(&mut command, &self.no_config());
        let out = command
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .current_dir(self.repo())
            .output()
            .expect("`git` runs");
        assert!(out.status.success(), "git {args:?}: {}", shown(&out));
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    /// `contents` written at `path` in the repository's working copy.
    fn write(&self, path: &str, contents: &[u8]) {
        let path = self.repo().join(path);
        fs::create_dir_all(path.parent().expect("a file has a folder"))
            .expect("the test makes the folder");
        fs::write(path, contents).expect("the test writes the file");
    }

    /// The package `Acme\Http` at `version` in the folder `dir` of the repository, with the
    /// `label` method when `label` is set.
    fn package(&self, dir: &str, version: &str, label: bool) {
        let at = |path: &str| {
            if dir.is_empty() {
                path.to_owned()
            } else {
                format!("{dir}/{path}")
            }
        };
        self.write(&at("package.toml"), manifest(version).as_bytes());
        self.write(&at("LICENSE"), b"MIT\n");
        self.write(&at("src/Client.nvs"), client(label).as_bytes());
    }

    /// Every change in the working copy committed. The commit's hash.
    fn commit(&self, message: &str) -> String {
        self.git(&["add", "-A"]);
        self.commit_index(message)
    }

    /// The index committed as it is. The commit's hash.
    fn commit_index(&self, message: &str) -> String {
        self.git(&["commit", "-q", "-m", message]);
        self.git(&["rev-parse", "HEAD"])
    }

    /// `nvs --data <data> --no-init <args>` in `dir`.
    fn nvs(&self, dir: &Path, args: &[&str]) -> Output {
        nvs_repo::spawn(env!("CARGO_BIN_EXE_nvs"), &[])
            .arg("--data")
            .arg(self.data())
            .arg("--no-init")
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::null())
            .output()
            .expect("the `nvs` binary this test was built beside runs")
    }

    /// `nvs registry build <url> <commit> --ca-file <ca> <extra>` in the working folder, for the
    /// case's repository on `host`, and the JSON document it printed. The host's faults are added
    /// to the build's standard error, so a failure message shows both.
    fn build(&self, host: &Host, commit: &str, extra: &[&str]) -> (Output, Value) {
        let ca = self.ca();
        let ca = ca.to_str().expect("the scratch path is UTF-8");
        let url = host.url();
        let mut args = vec!["registry", "build", &url, commit, "--ca-file", ca];
        args.extend_from_slice(extra);
        let mut out = self.nvs(&self.work(), &args);
        for fault in host.faults() {
            out.stderr
                .extend_from_slice(format!("\nthe test's git host: {fault}").as_bytes());
        }
        let json = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|err| panic!("the build printed no JSON ({err}): {}", shown(&out)));
        (out, json)
    }
}

/// `command` with no system or global git configuration and no repository from the environment.
fn isolated(command: &mut Command, no_config: &Path) {
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", no_config)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES");
}

/// A git host on loopback: `git http-backend` behind TLS, with a certificate for `127.0.0.1`,
/// serving each repository under the case's folder. One connection after another on a thread of
/// its own. Dropping it stops the thread.
struct Host {
    port: u16,
    served: Arc<AtomicU64>,
    /// Each request that failed, and what `git http-backend` wrote to its standard error.
    faults: Arc<Mutex<Vec<String>>>,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Host {
    /// Issues the certificate, writes it to the case's `ca.pem`, and starts serving.
    fn start(case: &Case) -> Self {
        let issued = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()])
            .expect("the certificate is issued");
        fs::write(case.ca(), issued.cert.pem()).expect("the test writes the certificate");
        let key = rustls::pki_types::PrivateKeyDer::Pkcs8(
            rustls::pki_types::PrivatePkcs8KeyDer::from(issued.signing_key.serialize_der()),
        );
        let config = Arc::new(
            rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .expect("the provider speaks the default versions")
            .with_no_client_auth()
            .with_single_cert(vec![issued.cert.der().clone()], key)
            .expect("the certificate and its key match"),
        );
        let listener = TcpListener::bind("127.0.0.1:0").expect("the test listens");
        let port = listener.local_addr().expect("it has an address").port();
        let served = Arc::new(AtomicU64::new(0));
        let stopped = Arc::new(AtomicBool::new(false));
        let faults = Arc::new(Mutex::new(Vec::new()));
        let (count, stop, noted) = (
            Arc::clone(&served),
            Arc::clone(&stopped),
            Arc::clone(&faults),
        );
        let projects = case.root.to_path_buf();
        let no_config = case.no_config();
        let thread = thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                // A request that fails here fails the clone, which the case asserts.
                if let Err(err) = serve(stream, &config, &projects, &no_config, &count, &noted) {
                    note(&noted, err.to_string());
                }
            }
        });
        Self {
            port,
            served,
            faults,
            stopped,
            thread: Some(thread),
        }
    }

    /// The URL of the case's repository on this host.
    fn url(&self) -> String {
        format!("https://127.0.0.1:{}/repo", self.port)
    }

    /// The bytes the host has sent, headers and bodies.
    fn served(&self) -> u64 {
        self.served.load(Ordering::SeqCst)
    }

    /// What the host noted so far: failed requests and the backend's standard error.
    fn faults(&self) -> Vec<String> {
        self.faults
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

/// `fault` added to `faults`.
fn note(faults: &Mutex<Vec<String>>, fault: String) {
    faults
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(fault);
}

impl Drop for Host {
    /// Ends the thread: the flag is set, and one connection wakes its `accept`.
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// One request over TLS on `stream`, run through `git http-backend` with `projects` as its
/// project root, and the CGI program's output sent back as an HTTP/1.1 reply.
fn serve(
    stream: TcpStream,
    config: &Arc<rustls::ServerConfig>,
    projects: &Path,
    no_config: &Path,
    served: &AtomicU64,
    faults: &Mutex<Vec<String>>,
) -> io::Result<()> {
    stream.set_read_timeout(Some(BOUND))?;
    let conn = rustls::ServerConnection::new(Arc::clone(config)).map_err(io::Error::other)?;
    let mut tls = BufReader::new(rustls::StreamOwned::new(conn, stream));
    let mut request = String::new();
    tls.read_line(&mut request)?;
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        if tls.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
        }
    }
    let header = |name: &str| {
        headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    let body = if header("transfer-encoding").is_some_and(|v| v.eq_ignore_ascii_case("chunked")) {
        chunked(&mut tls)?
    } else {
        let length: usize = header("content-length")
            .unwrap_or("0")
            .parse()
            .map_err(io::Error::other)?;
        let mut body = vec![0; length];
        tls.read_exact(&mut body)?;
        body
    };
    let mut parts = request.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let target = parts.next().unwrap_or_default();
    let (path, query) = target.split_once('?').unwrap_or((target, ""));

    let mut command = nvs_repo::spawn("git", &[]);
    isolated(&mut command, no_config);
    command
        .arg("http-backend")
        .env("GIT_PROJECT_ROOT", projects)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("PATH_INFO", path)
        .env("REQUEST_METHOD", &method)
        .env("QUERY_STRING", query)
        .env("CONTENT_TYPE", header("content-type").unwrap_or_default())
        .env("CONTENT_LENGTH", body.len().to_string())
        .env("REMOTE_ADDR", "127.0.0.1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(protocol) = header("git-protocol") {
        command.env("GIT_PROTOCOL", protocol);
    }
    // `git http-backend` inflates a gzip body itself when the encoding is named.
    if let Some(encoding) = header("content-encoding") {
        command.env("HTTP_CONTENT_ENCODING", encoding);
    }
    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take().expect("standard input is piped");
    // The body is written beside the read of the output, so neither pipe can fill and stall.
    let output = thread::scope(|scope| {
        scope.spawn(move || {
            // A backend that stops before it reads closes the pipe; its output says why.
            let _ = stdin.write_all(&body);
        });
        child.wait_with_output()
    })?;
    if !output.stderr.is_empty() {
        note(
            faults,
            format!(
                "git http-backend: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        );
    }
    let reply = reply(&output.stdout);
    let mut tls = tls.into_inner();
    tls.write_all(&reply)?;
    served.fetch_add(
        u64::try_from(reply.len()).unwrap_or(u64::MAX),
        Ordering::SeqCst,
    );
    tls.conn.send_close_notify();
    tls.flush()
}

/// A chunked request body, read to its last chunk and its trailer.
fn chunked(reader: &mut impl BufRead) -> io::Result<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let size = line.trim().split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size, 16).map_err(io::Error::other)?;
        if size == 0 {
            loop {
                let mut trailer = String::new();
                if reader.read_line(&mut trailer)? == 0 || trailer.trim().is_empty() {
                    return Ok(body);
                }
            }
        }
        let start = body.len();
        body.resize(start + size, 0);
        reader.read_exact(&mut body[start..])?;
        let mut end = String::new();
        reader.read_line(&mut end)?;
    }
}

/// The HTTP/1.1 reply for a CGI program's output: its `Status:` line, its other headers, and its
/// body with a `Content-Length`.
fn reply(cgi: &[u8]) -> Vec<u8> {
    let crlf = cgi
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|at| (at, 4));
    let lf = cgi.windows(2).position(|w| w == b"\n\n").map(|at| (at, 2));
    let Some((end, gap)) = [crlf, lf].into_iter().flatten().min_by_key(|(at, _)| *at) else {
        let body = b"the CGI program wrote no headers";
        let mut out = format!(
            "HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        out.extend_from_slice(body);
        return out;
    };
    let (head, body) = (&cgi[..end], &cgi[end + gap..]);
    let mut status = "200 OK".to_owned();
    let mut lines = String::new();
    for line in String::from_utf8_lossy(head).lines() {
        let Some((name, value)) = line.trim_end_matches('\r').split_once(':') else {
            continue;
        };
        let (name, value) = (name.trim(), value.trim());
        if name.eq_ignore_ascii_case("status") {
            value.clone_into(&mut status);
        } else if !name.eq_ignore_ascii_case("content-length") {
            lines.push_str(&format!("{name}: {value}\r\n"));
        }
    }
    let mut out = format!(
        "HTTP/1.1 {status}\r\n{lines}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    out
}

/// Both streams, for a message on failure.
fn shown(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Asserts that the build failed with a document whose refusal contains `said`.
fn refused(out: &Output, json: &Value, said: &str) {
    assert!(!out.status.success(), "it succeeded: {}", shown(out));
    assert_eq!(json["schema"], 3, "{json}");
    assert_eq!(json["ok"], false, "{json}");
    let refusal = json["refusal"].as_str().unwrap_or_default();
    assert!(refusal.contains(said), "`{said}` is not in: {}", shown(out));
}

/// `bytes` pseudo-random bytes from `seed`, which compress to about their own size.
fn noise(bytes: usize, seed: u64) -> Vec<u8> {
    let mut state = seed | 1;
    let mut out = Vec::with_capacity(bytes + 8);
    while out.len() < bytes {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        out.extend_from_slice(&state.to_le_bytes());
    }
    out.truncate(bytes);
    out
}

/// A build of a good package prints its name, version, folder, digest and files, and no API
/// report. The file `--archive` writes has that digest and that size.
#[test]
fn a_build_prints_the_package_and_the_archive_has_its_digest() {
    let case = Case::new("registry-build-good");
    case.package("", "1.0.0", false);
    case.write("README.md", b"Acme HTTP kit.\n");
    case.write("docs/notes.txt", b"not in the archive\n");
    let commit = case.commit("one");
    let host = Host::start(&case);
    let url = host.url();
    let archive = case.root.join("archive.tar");
    let archive_arg = archive.to_str().expect("the scratch path is UTF-8");

    let (out, json) = case.build(&host, &commit, &["--archive", archive_arg]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(json["schema"], 3, "{json}");
    assert_eq!(json["ok"], true, "{json}");
    assert_eq!(json["name"], "Acme\\Http", "{json}");
    assert_eq!(json["version"], "1.0.0", "{json}");
    assert_eq!(json["repository"], url.as_str(), "{json}");
    assert_eq!(json["commit"], commit.as_str(), "{json}");
    assert_eq!(json["directory"], "", "{json}");
    assert_eq!(json["api"], Value::Null, "{json}");
    assert_eq!(
        json["doc_coverage"],
        serde_json::json!({"documented": 0, "public": 2}),
        "{json}"
    );
    assert_eq!(json["docs"], "", "{json}");
    assert_eq!(json["summary"], "Acme HTTP kit.", "{json}");
    assert_eq!(json["readme"], "Acme HTTP kit.\n", "{json}");
    assert_eq!(json["downloads"], serde_json::json!([]), "{json}");
    assert_eq!(json["supersedes"], Value::Null, "{json}");
    assert_eq!(json["dependencies"], serde_json::json!({}), "{json}");
    assert_eq!(
        json["capabilities"],
        serde_json::json!({"required": ["net.connect"], "optional": []}),
        "{json}"
    );
    let files: Vec<(String, u64)> = json["files"]
        .as_array()
        .expect("`files` is a list")
        .iter()
        .map(|file| {
            (
                file["path"].as_str().unwrap_or_default().to_owned(),
                file["size"].as_u64().unwrap_or_default(),
            )
        })
        .collect();
    let class = client(false);
    assert_eq!(
        files,
        [
            ("LICENSE".to_owned(), 4),
            ("README.md".to_owned(), 15),
            ("package.toml".to_owned(), manifest("1.0.0").len() as u64),
            ("src/Client.nvs".to_owned(), class.len() as u64),
        ]
    );

    let bytes = fs::read(&archive).expect("the archive was written");
    let digest = format!("blake3:{}", blake3::hash(&bytes).to_hex());
    assert_eq!(json["digest"], digest.as_str(), "{json}");
    assert_eq!(json["size"], bytes.len(), "{json}");
}

/// `nvs pkg publish --dry-run` in a package's folder inside a working copy prints the digest
/// the build gives for the same commit and the same folder.
// covers: tools:packages/nvs-pkg-publish
#[test]
fn a_dry_run_publish_prints_the_digest_the_build_gives() {
    let case = Case::new("registry-build-dry-run");
    case.package("packages/http", "1.0.0", false);
    case.write("README.md", b"the repository's own file\n");
    let commit = case.commit("one");
    let host = Host::start(&case);
    let url = host.url();

    let (out, json) = case.build(&host, &commit, &["--dir", "packages/http"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(json["directory"], "packages/http", "{json}");
    let digest = json["digest"].as_str().expect("the build has a digest");
    assert!(digest.starts_with("blake3:"), "{json}");

    let folder = case.repo().join("packages/http");
    let out = case.nvs(
        &folder,
        &["pkg", "publish", "--dry-run", "--repository", &url],
    );
    assert!(out.status.success(), "{}", shown(&out));
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(said.contains(&format!("Commit: {commit}")), "{said}");
    assert!(said.contains("Folder: packages/http"), "{said}");
    assert!(said.contains(&format!("Digest: {digest}")), "{said}");
}

/// The publish token [`Service`] accepts.
const TOKEN: &str = "nvsreg_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// A plain-HTTP stand-in for the registry service on loopback: `/api/whoami` answers an account
/// for every token, and `/api/publish` answers `publish`. One connection after another on a
/// thread of its own. Dropping it stops the thread.
struct Service {
    port: u16,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Service {
    fn start(publish: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the test listens");
        let port = listener.local_addr().expect("it has an address").port();
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&stopped);
        let thread = thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                // A request that fails here fails the command, which the case asserts.
                let _ = answer(stream, &publish);
            }
        });
        Self {
            port,
            stopped,
            thread: Some(thread),
        }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for Service {
    /// Ends the thread: the flag is set, and one connection wakes its `accept`.
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// One request on `stream`, its body read to the end: `publish` for `/api/publish`, and an
/// account that may publish `Acme` for anything else.
fn answer(mut stream: TcpStream, publish: &str) -> io::Result<()> {
    stream.set_read_timeout(Some(BOUND))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    let path = request.split_whitespace().nth(1).unwrap_or_default();
    let reply = if path == "/api/publish" {
        publish.to_owned()
    } else {
        r#"{"account":"00112233445566778899aabbccddeeff","scope":"Acme","expiresAt":1830297600}"#
            .to_owned()
    };
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{reply}",
        reply.len()
    )?;
    stream.flush()
}

/// `nvs pkg publish` of a committed package to a [`Service`] that answers `publish`, after
/// `nvs pkg login` stored [`TOKEN`]. The publish's output.
fn publish_to(name: &str, publish: &str) -> Output {
    let case = Case::new(name);
    case.package("", "1.0.0", false);
    case.commit("one");
    let service = Service::start(publish.to_owned());
    let url = service.url();
    let mut child = nvs_repo::spawn(env!("CARGO_BIN_EXE_nvs"), &[])
        .arg("--data")
        .arg(case.data())
        .args(["--no-init", "pkg", "login", "--registry", &url])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the `nvs` binary this test was built beside runs");
    let mut stdin = child.stdin.take().expect("standard input is piped");
    stdin
        .write_all(format!("{TOKEN}\n").as_bytes())
        .expect("`nvs pkg login` reads its token");
    drop(stdin);
    let out = child
        .wait_with_output()
        .expect("the `nvs` binary this test was built beside runs");
    assert!(out.status.success(), "{}", shown(&out));
    case.nvs(
        &case.repo(),
        &[
            "pkg",
            "publish",
            "--registry",
            &url,
            "--repository",
            "https://git.example.com/acme/http",
        ],
    )
}

/// `nvs pkg publish` prints the account's pending releases after the registry's answer, and the
/// registry's warning on standard error with a successful exit
/// (`rule:packaging/an-owner-is-notified-without-email`).
// covers: tools:packages/nvs-pkg-publish
#[test]
fn a_publish_prints_the_pending_releases_and_the_warning() {
    let out = publish_to(
        "registry-publish-pending",
        r#"{"digest":"blake3:00","message":"Acme\\Http 1.0.0 is pending.","pending":[
            {"package":"Acme\\Http","version":"1.0.0","until":"2026-11-04T12:00:00Z"},
            {"package":"Acme\\Http","version":"0.9.0","until":null}],
            "warning":"No owner of `Acme` has a working device or webhook."}"#,
    );
    assert!(out.status.success(), "{}", shown(&out));
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(
        said.contains(
            "Acme\\Http 1.0.0 is pending.\nReleases that are not available yet:\n  Acme\\Http \
             1.0.0 will be available at 2026-11-04T12:00:00Z.\n  Acme\\Http 0.9.0 waits until \
             an owner confirms it with a passkey on the website.\n"
        ),
        "{said}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Warning: No owner of `Acme` has a working device or webhook."),
        "{}",
        shown(&out)
    );
}

/// A publish reply without `pending` and `warning`, as an older registry sends, prints the
/// answer alone.
#[test]
fn a_publish_reply_without_pending_or_warning_prints_the_answer_alone() {
    let out = publish_to(
        "registry-publish-older",
        r#"{"digest":"blake3:00","message":"Acme\\Http 1.0.0 is released."}"#,
    );
    assert!(out.status.success(), "{}", shown(&out));
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(said.contains("Acme\\Http 1.0.0 is released."), "{said}");
    assert!(!said.contains("not available yet"), "{said}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("Warning"), "{}", shown(&out));
}

/// A repository URL that starts with `http://` is refused, and the build connects nowhere.
#[test]
fn an_http_url_is_refused() {
    let case = Case::new("registry-build-http");
    let out = case.nvs(
        &case.work(),
        &[
            "registry",
            "build",
            "http://127.0.0.1:9/repo",
            &"1".repeat(40),
        ],
    );
    let json: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("the build printed no JSON ({err}): {}", shown(&out)));
    refused(&out, &json, "is not an `https://` URL");
}

/// A commit with a file over 512 MiB is refused when the clone reaches that file. The file is
/// zeros, so the host sends well under one mebibyte.
#[test]
fn a_clone_over_512_mib_is_refused() {
    let case = Case::new("registry-build-huge");
    case.package("", "1.0.0", false);
    case.git(&["add", "-A"]);
    let zeros = case.root.join("zeros.bin");
    let file = fs::File::create(&zeros).expect("the test makes the file");
    // A file extended this way reads as zeros and is written by the file system, not the test.
    file.set_len(CLONE_LIMIT + 1024 * 1024)
        .expect("the test sizes the file");
    drop(file);
    let zeros_arg = zeros.to_str().expect("the scratch path is UTF-8");
    let blob = case.git(&["hash-object", "-w", zeros_arg]);
    fs::remove_file(&zeros).expect("the test removes the file");
    case.git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("100644,{blob},src/Zeros.bin"),
    ]);
    let commit = case.commit_index("huge");
    let host = Host::start(&case);

    let (out, json) = case.build(&host, &commit, &[]);
    refused(&out, &json, &format!("larger than {CLONE_LIMIT} bytes"));
    assert!(host.served() < 4 * 1024 * 1024, "{} bytes", host.served());
}

/// A package whose files are over 16 MiB together is refused by the pack.
#[test]
fn an_archive_over_16_mib_is_refused() {
    let case = Case::new("registry-build-large");
    case.package("", "1.0.0", false);
    let toml = manifest("1.0.0").replace(
        "\n\n[capabilities]",
        "\nfiles = [\"assets/data.bin\"]\n\n[capabilities]",
    );
    case.write("package.toml", toml.as_bytes());
    fs::create_dir_all(case.repo().join("assets")).expect("the test makes the folder");
    let file = fs::File::create(case.repo().join("assets/data.bin")).expect("the test makes it");
    file.set_len(PACK_LIMIT + 1)
        .expect("the test sizes the file");
    drop(file);
    let commit = case.commit("large");
    let host = Host::start(&case);

    let (out, json) = case.build(&host, &commit, &[]);
    refused(&out, &json, &format!("The limit is {PACK_LIMIT} bytes"));
}

/// A `[[download]]` entry with an `http://` URL is refused.
#[test]
fn a_download_with_an_http_url_is_refused() {
    let case = Case::new("registry-build-download");
    case.package("", "1.0.0", false);
    let entry = format!(
        "\n[[download]]\npath = \"fonts/Big.otf\"\nurls = [\"http://example.com/Big.otf\"]\n\
         sha256 = \"{}\"\nsize = 10\n",
        "a".repeat(64)
    );
    let toml = format!("{}{entry}", manifest("1.0.0"));
    case.write("package.toml", toml.as_bytes());
    let commit = case.commit("download");
    let host = Host::start(&case);

    let (out, json) = case.build(&host, &commit, &[]);
    refused(&out, &json, "Every URL must start with `https://`");
}

/// The clone fetches the one commit and none of its history. An earlier commit has a large file
/// that the built commit deletes, and the host sends far fewer bytes than that file has.
#[test]
fn the_clone_fetches_the_commit_without_its_history() {
    let case = Case::new("registry-build-history");
    case.package("", "1.0.0", false);
    let big = noise(4 * 1024 * 1024, 0x5eed);
    case.write("src/Big.bin", &big);
    case.commit("with the large file");
    fs::remove_file(case.repo().join("src/Big.bin")).expect("the test deletes the file");
    let commit = case.commit("without it");
    let host = Host::start(&case);

    let (out, json) = case.build(&host, &commit, &[]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(json["ok"], true, "{json}");
    let served = host.served();
    assert!(
        served < big.len() as u64 / 8,
        "the host sent {served} bytes for a history with a file of {} bytes",
        big.len()
    );
}

/// `doc_coverage` counts the public declarations with a `///` doc comment, of all public
/// declarations: the enum, its cases, the class and its public constant and method. A private
/// method is not counted.
#[test]
fn a_build_counts_the_public_declarations_with_a_doc_comment() {
    let case = Case::new("registry-build-coverage");
    case.package("", "1.0.0", false);
    case.write(
        "src/Client.nvs",
        b"<?nvs\nnamespace Acme\\Http;\n\n/// What a request does.\nenum Verb\n{\n    \
          /// Reads a resource.\n    Get = 1,\n    Post = 2,\n}\n\nfinal class Client\n{\n    \
          /// The name of the client.\n    public const string NAME = 'client';\n\n    \
          public static function name(): string\n    {\n        return Client::inner();\n    \
          }\n\n    /// Not counted, because it is private.\n    \
          private static function inner(): string\n    {\n        return Client::NAME;\n    \
          }\n}\n",
    );
    let commit = case.commit("documented");
    let host = Host::start(&case);

    let (out, json) = case.build(&host, &commit, &[]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(
        json["doc_coverage"],
        serde_json::json!({"documented": 3, "public": 6}),
        "{json}"
    );
    assert_eq!(
        json["docs"], "The name of the client.\nWhat a request does.\nReads a resource.\n",
        "{json}"
    );
}

/// A class in a namespace outside the package's name is refused.
#[test]
fn a_class_outside_the_package_namespace_is_refused() {
    let case = Case::new("registry-build-namespace");
    case.package("", "1.0.0", false);
    case.write(
        "src/Other.nvs",
        b"<?nvs\nnamespace Other;\n\nfinal class Thing\n{\n}\n",
    );
    let commit = case.commit("outside");
    let host = Host::start(&case);

    let (out, json) = case.build(&host, &commit, &[]);
    refused(
        &out,
        &json,
        "is outside the package's namespace `Acme\\Http`",
    );
}

/// A minor version that removes a public method breaks the API the previous build wrote, so it
/// is refused, and `api.changes` lists the change.
#[test]
fn a_minor_version_that_removes_a_method_is_refused() {
    let case = Case::new("registry-build-api");
    case.package("", "1.0.0", true);
    let first = case.commit("1.0.0");
    case.package("", "1.1.0", false);
    let second = case.commit("1.1.0");
    let host = Host::start(&case);
    let api = case.root.join("api-1.0.0.json");
    let api_arg = api.to_str().expect("the scratch path is UTF-8");

    let (out, json) = case.build(&host, &first, &["--api", api_arg]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(json["ok"], true, "{json}");
    assert!(api.is_file(), "`--api` wrote no file");

    let (out, json) = case.build(
        &host,
        &second,
        &[
            "--version",
            "1.1.0",
            "--previous",
            api_arg,
            "--previous-version",
            "1.0.0",
        ],
    );
    refused(&out, &json, "breaks the API of 1.0.0");
    let changes = json["api"]["changes"]
        .as_array()
        .expect("the report lists the changes");
    assert!(!changes.is_empty(), "{json}");
}

/// The longest `nvs serve` takes to print its `listening on` line. A bound on a stuck start,
/// never a wait the test relies on.
const BOOT: Duration = Duration::from_secs(120);

/// The address the copy's `hosting.json` names. Its host is the passkey's domain.
const ORIGIN: &str = "http://localhost:8000";

/// What the copy runs to give an account a verified webhook, as
/// `Registry\BrowserTest::verifiedChannel` does in the application's own tests. A webhook under
/// `.invalid` never has an address, so each message to it fails at once with no network.
const CHANNEL_SCRIPT: &str = "<?nvs\nautoload 'Registry' from '../src';\n\n\
array<tainted string> $words = Core\\Cli::arguments();\n\
string $account = Registry\\Accounts::idOf($words[0]) ?? '';\n\
var $channels = new Registry\\Channels(Registry\\Schema::open());\n\
array<string> $added = $channels->add($account, Registry\\Channels::WEBHOOK, 'https://hooks.invalid/' . $account);\n\
if (!$channels->verify($account, $added['id'], $added['code'])) {\n\
    throw new RuntimeError('The webhook is not verified.');\n\
}\n";

/// A copy of `apps/registry` in the case's folder, served by `nvs serve` on loopback. Its
/// `hosting.json` names the `nvs` this test was built beside, a log key `nvs registry key` made,
/// and the certificate of the case's git host, and its `nvs.toml` grants that program and lets
/// the build reach loopback. The copy has its own database, so the vendor `Acme` is free.
/// Dropping it stops the server.
struct Registry {
    child: std::process::Child,
    addr: String,
    dir: PathBuf,
    /// Both streams of the server, for a failure message.
    said: Arc<Mutex<String>>,
}

impl Registry {
    /// Copies the application, writes its settings, and starts serving it.
    fn start(case: &Case) -> Self {
        let app = nvs_repo::path("apps/registry");
        let dir = case.root.join("registry");
        for folder in ["src", "public", "jobs", "fixtures"] {
            copy_tree(&app.join(folder), &dir.join(folder));
        }
        let keys = dir.join("keys");
        fs::create_dir_all(&keys).expect("the test makes the key folder");
        fs::copy(case.ca(), keys.join("ca.pem")).expect("the test copies the certificate");
        let key = keys.join("log.key");
        let key = key.to_str().expect("the scratch path is UTF-8");
        let out = case.nvs(&case.root, &["registry", "key", key, "--name", "localhost"]);
        assert!(out.status.success(), "{}", shown(&out));
        fs::write(dir.join("jobs/channel.nvs"), CHANNEL_SCRIPT)
            .expect("the test writes the script");

        let nvs = env!("CARGO_BIN_EXE_nvs");
        let hosting = serde_json::json!({
            "origin": ORIGIN,
            "signUpWork": 8,
            "nvs": nvs,
            "gitCaFile": "keys/ca.pem",
            "projectAccount": "",
            "work": "work",
            "stores": [{ "kind": "dir", "path": "files" }],
            "hosts": [],
            "backup": { "kind": "dir", "path": "backup" },
            "backupKey": "",
            "backupMinutes": 5,
            "downloadLog": null,
            "logKey": "keys/log.key",
            "logName": "localhost",
            "pushKey": "",
            "pushSubject": "mailto:registry@example.com",
        });
        fs::write(dir.join("hosting.json"), hosting.to_string())
            .expect("the test writes the settings");
        let config = fs::read_to_string(app.join("nvs.toml"))
            .expect("the application has its configuration")
            .replace("\r\n", "\n");
        let config = config
            .replacen(
                "read = [\"hosting.json\",",
                "read = [\"keys\", \"hosting.json\",",
                1,
            )
            .replacen(
                "[capabilities.net]\ninternal = []\n",
                "[capabilities.net]\ninternal = [\"127.0.0.1\"]\n",
                1,
            );
        assert!(
            config.contains("\"keys\"") && config.contains("internal = [\"127.0.0.1\"]"),
            "the application's `nvs.toml` changed its `read` or `internal` line: {config}"
        );
        fs::write(
            dir.join("nvs.toml"),
            format!("{config}\n[capabilities.process]\nexec = ['{nvs}']\n"),
        )
        .expect("the test writes the configuration");

        let serve_data = case.root.join("serve-data");
        fs::create_dir_all(&serve_data).expect("the test makes the server's data folder");
        let mut child = nvs_repo::spawn(nvs, &[])
            .arg("--data")
            .arg(&serve_data)
            .arg("--no-init")
            .args(["serve", "public/index.nvs", "--config", "nvs.toml"])
            .args(["--listen", "127.0.0.1:0"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the `nvs` binary this test was built beside starts");
        // Both pipes are read to their end on a thread of their own, so a server that writes a
        // lot never blocks on a full pipe.
        let said = Arc::new(Mutex::new(String::new()));
        let err_pipe = child.stderr.take().expect("standard error is piped");
        let collected = Arc::clone(&said);
        thread::spawn(move || {
            for line in BufReader::new(err_pipe).lines().map_while(Result::ok) {
                note_line(&collected, &line);
            }
        });
        let out_pipe = child.stdout.take().expect("standard output is piped");
        let (sent, bound) = std::sync::mpsc::channel();
        let collected = Arc::clone(&said);
        thread::spawn(move || {
            for line in BufReader::new(out_pipe).lines().map_while(Result::ok) {
                note_line(&collected, &line);
                if let Some(rest) = line.strip_prefix("listening on http://") {
                    let _ = sent.send(
                        rest.split_whitespace()
                            .next()
                            .unwrap_or_default()
                            .to_owned(),
                    );
                }
            }
        });
        let Ok(addr) = bound.recv_timeout(BOOT) else {
            let _ = child.kill();
            let _ = child.wait();
            let said = said
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            panic!("`nvs serve` did not start listening within {BOOT:?}: {said}");
        };
        Self {
            child,
            addr,
            dir,
            said,
        }
    }

    /// The registry URL `nvs pkg` is given.
    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// What the server wrote so far.
    fn said(&self) -> String {
        self.said
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Gives `account` a verified webhook with the operator's `nvs run`, since the code of a
    /// webhook reaches only the webhook.
    fn verify_channel(&self, case: &Case, account: &str) {
        let out = case.nvs(
            &self.dir,
            &["run", "jobs/channel.nvs", "--config", "nvs.toml", account],
        );
        assert!(out.status.success(), "{}", shown(&out));
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `line` added to `said`.
fn note_line(said: &Mutex<String>, line: &str) {
    let mut all = said
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    all.push_str(line);
    all.push('\n');
}

/// Every file under `from` copied to the same path under `to`.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("the test makes the folder");
    for entry in fs::read_dir(from).expect("the folder lists") {
        let entry = entry.expect("the entry reads");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("the entry has a type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("the test copies the file");
        }
    }
}

/// A browser with one passkey device, as `Registry\BrowserTest` plays one in the application's
/// own tests: it keeps the session cookies and answers the WebAuthn options with an Ed25519 key.
struct Browser {
    addr: String,
    cookies: Vec<(String, String)>,
    key: ring::signature::Ed25519KeyPair,
    passkey: [u8; 16],
}

impl Browser {
    fn new(registry: &Registry) -> Self {
        let pkcs8 =
            ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new())
                .expect("the key is made");
        Self {
            addr: registry.addr.clone(),
            cookies: Vec::new(),
            key: ring::signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
                .expect("the key reads back"),
            passkey: *b"passkey-of-test!",
        }
    }

    /// `method path` with `body` as JSON and the session cookies. The status and the JSON reply.
    fn send(&mut self, method: &str, path: &str, body: &str) -> (u16, Value) {
        let (status, body) = self.exchange(method, path, body);
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }

    /// `GET path` with the session cookies. The status and the page as text.
    fn page(&mut self, path: &str) -> (u16, String) {
        let (status, body) = self.exchange("GET", path, "");
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    /// `method path` with `body` as JSON and the session cookies. The status and the reply's bytes.
    fn exchange(&mut self, method: &str, path: &str, body: &str) -> (u16, Vec<u8>) {
        let mut stream = TcpStream::connect(&self.addr).expect("the registry accepts");
        stream
            .set_read_timeout(Some(BOUND))
            .expect("the socket takes a timeout");
        let cookie = if self.cookies.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> = self
                .cookies
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            format!("Cookie: {}\r\n", pairs.join("; "))
        };
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\n{cookie}Connection: close\r\n\r\n{body}",
            self.addr,
            body.len()
        )
        .expect("the request is sent");
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).expect("the reply has a status");
        let status = line
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or_else(|| panic!("`{line}` is not a status line"));
        let (mut length, mut chunks) = (None, false);
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).expect("the header reads") == 0
                || header.trim().is_empty()
            {
                break;
            }
            let Some((name, value)) = header.split_once(':') else {
                continue;
            };
            let (name, value) = (name.trim().to_ascii_lowercase(), value.trim());
            if name == "content-length" {
                length = value.parse().ok();
            } else if name == "transfer-encoding" && value.eq_ignore_ascii_case("chunked") {
                chunks = true;
            } else if name == "set-cookie"
                && let Some((key, rest)) = value.split_once('=')
            {
                let kept = rest.split(';').next().unwrap_or_default().to_owned();
                self.cookies.retain(|(had, _)| had != key);
                self.cookies.push((key.to_owned(), kept));
            }
        }
        let body = if chunks {
            chunked(&mut reader).expect("the chunked body reads")
        } else if let Some(length) = length {
            let mut body = vec![0; length];
            reader.read_exact(&mut body).expect("the body reads");
            body
        } else {
            let mut body = Vec::new();
            reader.read_to_end(&mut body).expect("the body reads");
            body
        };
        (status, body)
    }

    /// A sign-up: the proof-of-work solved, a new passkey for the options, and the account the
    /// registry made, which is signed in with that passkey.
    fn sign_up(&mut self) -> String {
        let (status, work) = self.send("POST", "/api/signup/work", "");
        assert_eq!(status, 200, "{work}");
        let challenge = work["challenge"].as_str().expect("a challenge").to_owned();
        let bits =
            u32::try_from(work["bits"].as_u64().expect("a number of bits")).expect("the bits fit");
        let nonce = (0u64..)
            .find(|nonce| {
                zero_bits(&<sha2::Sha256 as sha2::Digest>::digest(
                    format!("{challenge}:{nonce}").as_bytes(),
                )) >= bits
            })
            .expect("a nonce solves it");
        let answer = serde_json::json!({ "challenge": challenge, "nonce": nonce.to_string() });
        let (status, options) = self.send("POST", "/api/signup/options", &answer.to_string());
        assert_eq!(status, 200, "{options}");
        let registration = self.register(&options);
        let (status, account) = self.send("POST", "/api/signup", &registration);
        assert_eq!(status, 201, "{account}");
        account["account"].as_str().expect("an account").to_owned()
    }

    /// The answer of the device to the registration options: client data with their challenge,
    /// and an attestation object of format `none` with the device's Ed25519 key as a COSE key.
    fn register(&self, options: &Value) -> String {
        use base64::Engine as _;
        use ring::signature::KeyPair as _;
        let challenge = options["challenge"].as_str().expect("a challenge");
        let client = serde_json::json!({ "type": "webauthn.create", "challenge": challenge, "origin": ORIGIN })
            .to_string();
        let mut data = <sha2::Sha256 as sha2::Digest>::digest(b"localhost").to_vec();
        // The flags (user present, user verified, credential data), the counter and the AAGUID.
        data.push(0x45);
        data.extend_from_slice(&[0; 20]);
        data.extend_from_slice(&16u16.to_be_bytes());
        data.extend_from_slice(&self.passkey);
        // A COSE key: OKP, EdDSA, Ed25519, and the 32 bytes of the public key.
        data.extend_from_slice(&[0xa4, 0x01, 0x01, 0x03, 0x27, 0x20, 0x06, 0x21, 0x58, 0x20]);
        data.extend_from_slice(self.key.public_key().as_ref());
        let mut object = b"\xa3\x63fmt\x64none\x67attStmt\xa0\x68authData\x58".to_vec();
        object.push(u8::try_from(data.len()).expect("the data fits one byte of length"));
        object.extend_from_slice(&data);
        let encode = |bytes: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        let id = encode(&self.passkey);
        serde_json::json!({
            "type": "public-key",
            "id": id,
            "rawId": id,
            "response": {
                "clientDataJSON": encode(client.as_bytes()),
                "attestationObject": encode(&object),
            },
        })
        .to_string()
    }
}

/// The zero bits at the start of `hash`, as `Registry\Work::zeroBits` counts them.
fn zero_bits(hash: &[u8]) -> u32 {
    let mut bits = 0;
    for byte in hash {
        if *byte != 0 {
            return bits + byte.leading_zeros();
        }
        bits += 8;
    }
    bits
}

/// `nvs pkg login` stores `token` for `url`. Its output.
fn login(case: &Case, url: &str, token: &str) -> Output {
    let mut child = nvs_repo::spawn(env!("CARGO_BIN_EXE_nvs"), &[])
        .arg("--data")
        .arg(case.data())
        .args(["--no-init", "pkg", "login", "--registry", url])
        .current_dir(case.repo())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the `nvs` binary this test was built beside runs");
    let mut stdin = child.stdin.take().expect("standard input is piped");
    stdin
        .write_all(format!("{token}\n").as_bytes())
        .expect("`nvs pkg login` reads its token");
    drop(stdin);
    child
        .wait_with_output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// The publishing commands against a served copy of `apps/registry`: an account signs up with a
/// passkey and makes a token for `Acme`, then `nvs pkg login`, `whoami`, `publish`, `retract` and
/// `unlist` each get the service's real answer. The service builds the release from the case's
/// git host, and the client's own digest of the commit agrees with it.
/// An account signed up with a passkey in `browser`, with a verified channel, and `nvs pkg login`
/// done in the case with a token for the vendor `Acme`. The browser and the account.
fn publisher(case: &Case, registry: &Registry) -> (Browser, String) {
    let mut browser = Browser::new(registry);
    let account = browser.sign_up();
    let (status, made) = browser.send(
        "POST",
        "/api/tokens",
        &serde_json::json!({ "scope": "Acme", "days": 30 }).to_string(),
    );
    assert_eq!(status, 201, "{made}: {}", registry.said());
    let token = made["token"].as_str().expect("a token").to_owned();
    registry.verify_channel(case, &account);
    let out = login(case, &registry.url(), &token);
    assert!(
        out.status.success(),
        "{}\nthe server: {}",
        shown(&out),
        registry.said()
    );
    (browser, account)
}

// covers: tools:packages/nvs-pkg-login-and-nvs-pkg-whoami, tools:packages/nvs-pkg-publish, tools:packages/nvs-pkg-retract-and-nvs-pkg-unlist
#[test]
fn the_publishing_commands_work_against_the_registry_service() {
    let case = Case::new("registry-service-client");
    case.package("", "1.0.0", false);
    case.commit("1.0.0");
    let host = Host::start(&case);
    let registry = Registry::start(&case);
    let url = registry.url();
    let (_browser, account) = publisher(&case, &registry);
    let shown_all = |out: &Output| format!("{}\nthe server: {}", shown(out), registry.said());

    let out = case.nvs(&case.repo(), &["pkg", "whoami", "--registry", &url]);
    assert!(out.status.success(), "{}", shown_all(&out));
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(said.contains(&format!("Account: {account}")), "{said}");
    assert!(said.contains("Scope: Acme"), "{said}");

    let repository = host.url();
    let out = case.nvs(
        &case.repo(),
        &[
            "pkg",
            "publish",
            "--registry",
            &url,
            "--repository",
            &repository,
        ],
    );
    assert!(out.status.success(), "{}", shown_all(&out));
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(said.contains("`Acme\\Http` 1.0.0 is published."), "{said}");
    assert!(
        !String::from_utf8_lossy(&out.stderr).contains("this computer packs"),
        "the digests differ: {}",
        shown_all(&out)
    );
    assert!(
        registry.dir.join("files/registry.json").is_file(),
        "the store has no `registry.json`: {}",
        registry.said()
    );

    let out = case.nvs(&case.repo(), &["pkg", "whoami", "--registry", &url]);
    assert!(out.status.success(), "{}", shown_all(&out));
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(
        said.contains("Acme\\Http 1.0.0 waits until an owner confirms it with a passkey"),
        "{said}"
    );

    let out = case.nvs(
        &case.repo(),
        &[
            "pkg",
            "retract",
            "1.0.0",
            "--reason",
            "It deletes the cache.",
            "--registry",
            &url,
        ],
    );
    assert!(out.status.success(), "{}", shown_all(&out));

    let out = case.nvs(&case.repo(), &["pkg", "unlist", "--registry", &url]);
    assert!(out.status.success(), "{}", shown_all(&out));
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(
        said.contains(&format!(
            "sign in with a passkey and confirm on {ORIGIN}/packages/"
        )),
        "{said}"
    );
}

// covers: tools:packages/the-registry-website
#[test]
fn the_website_shows_a_published_package_and_search_finds_it_once_released() {
    let case = Case::new("registry-website");
    case.package("", "1.0.0", false);
    let commit = case.commit("1.0.0");
    let host = Host::start(&case);
    let registry = Registry::start(&case);
    let url = registry.url();
    let (mut browser, _account) = publisher(&case, &registry);
    let repository = host.url();
    let out = case.nvs(
        &case.repo(),
        &[
            "pkg",
            "publish",
            "--registry",
            &url,
            "--repository",
            &repository,
        ],
    );
    assert!(
        out.status.success(),
        "{}\nthe server: {}",
        shown(&out),
        registry.said()
    );

    // The first version at 1.0.0 waits for an owner, so the page shows it and search does not.
    let (status, page) = browser.page("/packages/acme/http");
    assert_eq!(status, 200, "{page}\nthe server: {}", registry.said());
    for fact in [
        "Acme\\Http",
        "waits for an owner",
        &commit,
        "Order in a search",
        "Owners",
    ] {
        assert!(
            page.contains(fact),
            "the package page lacks `{fact}`: {page}"
        );
    }
    let (status, found) = browser.send("GET", "/api/search?q=http", "");
    assert_eq!(status, 200, "{found}");
    assert_eq!(found["packages"], serde_json::json!([]), "{found}");

    // The version page shows what the registry's own build of the commit found.
    let (status, page) = browser.page("/packages/acme/http/1.0.0");
    assert_eq!(status, 200, "{page}");
    for fact in ["src/Client.nvs", "package.toml", "LICENSE", "net.connect"] {
        assert!(
            page.contains(fact),
            "the version page lacks `{fact}`: {page}"
        );
    }

    // An owner who signed in with a passkey confirms the API, and search finds the package.
    let (status, confirmed) = browser.send("POST", "/api/packages/acme/http/1.0.0/stable", "");
    assert_eq!(status, 200, "{confirmed}\nthe server: {}", registry.said());
    let (status, found) = browser.send("GET", "/api/search?q=http&sort=downloads", "");
    assert_eq!(status, 200, "{found}");
    assert_eq!(found["sort"], "downloads", "{found}");
    assert_eq!(found["packages"][0]["name"], "Acme\\Http", "{found}");
    assert_eq!(found["packages"][0]["declarations"], 2, "{found}");
    let (status, page) = browser.page("/packages/acme/http");
    assert_eq!(status, 200, "{page}");
    assert!(page.contains("released"), "{page}");
    let (status, page) = browser.page("/packages/acme/nothing");
    assert_eq!(status, 404, "{page}");
}
