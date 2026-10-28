//! `nvs pkg login` and `nvs pkg whoami` against a loopback stand-in for the registry service's
//! `GET /api/whoami` (`rule:packaging/a-publish-token-is-scoped-and-expires`,
//! `rule:config/ownership-is-the-trust-boundary`).
//!
//! Each test owns a private data folder and a plain-HTTP host on `127.0.0.1`, which the client
//! allows for a registry on this computer. The host accepts one token, answers `401` for every
//! other one, and counts the requests it served, so a test can show a token was never sent.

use std::fs;
use std::io::{BufRead as _, BufReader, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// The longest the host waits for a request it has accepted. A bound on a stuck client, never a
/// wait the tests rely on.
const BOUND: Duration = Duration::from_secs(30);

/// The token the host accepts.
const GOOD: &str = "nvsreg_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// A well-formed token the host answers `401` for.
const REVOKED: &str = "nvsreg_fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

/// The account the host names for [`GOOD`].
const ACCOUNT: &str = "00112233445566778899aabbccddeeff";

/// A plain-HTTP host on loopback that answers `/api/whoami`, one connection after another on a
/// thread of its own. Dropping it stops the thread.
struct Host {
    port: u16,
    served: Arc<AtomicUsize>,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Host {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the test listens");
        let port = listener.local_addr().expect("it has an address").port();
        let served = Arc::new(AtomicUsize::new(0));
        let stopped = Arc::new(AtomicBool::new(false));
        let (count, stop) = (Arc::clone(&served), Arc::clone(&stopped));
        let thread = thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                // A request that fails here fails the command, which the case asserts.
                let _ = serve(stream, &count);
            }
        });
        Self {
            port,
            served,
            stopped,
            thread: Some(thread),
        }
    }

    /// The registry URL of this host.
    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// The number of requests the host has answered.
    fn served(&self) -> usize {
        self.served.load(Ordering::SeqCst)
    }
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

/// One request on `stream`: `200` with the account and scope for `GET /api/whoami` with
/// [`GOOD`] as its bearer token, and `401` for anything else.
fn serve(mut stream: TcpStream, served: &AtomicUsize) -> std::io::Result<()> {
    stream.set_read_timeout(Some(BOUND))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    let mut bearer = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case("authorization")
        {
            bearer = value.trim().strip_prefix("Bearer ").map(str::to_owned);
        }
    }
    let path = request.split_whitespace().nth(1).unwrap_or_default();
    let (status, body) = if path == "/api/whoami" && bearer.as_deref() == Some(GOOD) {
        (
            "200 OK",
            format!(r#"{{"account":"{ACCOUNT}","scope":"Acme","expiresAt":1830297600}}"#),
        )
    } else {
        (
            "401 Unauthorized",
            r#"{"error":"The token is not valid."}"#.to_owned(),
        )
    };
    served.fetch_add(1, Ordering::SeqCst);
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()
}

/// `nvs --data <data> pkg <args>`, with `input` on standard input.
fn nvs(data: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(data)
        .arg("--no-init")
        .arg("pkg")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the `nvs` binary this test was built beside runs");
    let mut stdin = child.stdin.take().expect("standard input is piped");
    // A command that stops before it reads closes the pipe, and that is what the case asserts.
    let _ = stdin.write_all(input.as_bytes());
    drop(stdin);
    child
        .wait_with_output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// Both streams, for a message on failure.
fn shown(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Asserts that `out` failed and that its standard error contains `said`.
fn refused(out: &Output, said: &str) {
    assert!(!out.status.success(), "it succeeded: {}", shown(out));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(said), "`{said}` is not in: {}", shown(out));
}

/// Every file under the data folder's `tokens/`.
fn token_files(data: &Path) -> Vec<PathBuf> {
    fs::read_dir(data.join("tokens")).map_or_else(
        |_| Vec::new(),
        |entries| entries.map_while(Result::ok).map(|e| e.path()).collect(),
    )
}

#[test]
fn login_stores_a_token_and_whoami_prints_its_account_and_scope() {
    let data = nvs_repo::scratch_private("pkg-login-stores");
    let host = Host::start();
    let url = host.url();

    let out = nvs(&data, &["login", "--registry", &url], &format!("{GOOD}\n"));
    assert!(out.status.success(), "{}", shown(&out));
    let said = shown(&out);
    assert!(said.contains(ACCOUNT) && said.contains("Scope: Acme"), "{said}");
    assert!(!said.contains(GOOD), "the token was printed: {said}");
    let files = token_files(&data);
    assert_eq!(files.len(), 1, "one file per registry: {files:?}");
    let kept = fs::read_to_string(&files[0]).expect("the token file reads");
    assert_eq!(kept.trim(), GOOD);

    let out = nvs(&data, &["whoami", "--registry", &url], "");
    assert!(out.status.success(), "{}", shown(&out));
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(said.contains(&format!("Account: {ACCOUNT}")), "{said}");
    assert!(said.contains("Scope: Acme"), "{said}");
    assert!(said.contains("Expires: 2028-01-01"), "{said}");
    assert!(!said.contains(GOOD), "the token was printed: {said}");
    assert_eq!(host.served(), 2);
}

#[test]
fn login_refuses_a_malformed_token_without_sending_it() {
    let data = nvs_repo::scratch_private("pkg-login-malformed");
    let host = Host::start();
    let short = "nvsreg_0123456789abcdef";
    let out = nvs(&data, &["login", "--registry", &host.url()], short);
    refused(&out, "not a publish token");
    let upper = GOOD.to_uppercase().replacen("NVSREG_", "nvsreg_", 1);
    let out = nvs(&data, &["login", "--registry", &host.url()], &upper);
    refused(&out, "not a publish token");
    assert_eq!(host.served(), 0, "a malformed token is never sent");
    assert!(token_files(&data).is_empty());
}

#[test]
fn login_refuses_a_token_the_registry_answers_401_for() {
    let data = nvs_repo::scratch_private("pkg-login-revoked");
    let host = Host::start();
    let out = nvs(&data, &["login", "--registry", &host.url()], REVOKED);
    refused(&out, "does not accept this token");
    assert_eq!(host.served(), 1);
    assert!(token_files(&data).is_empty(), "a refused token is not stored");
}

#[test]
fn login_never_takes_the_token_as_an_argument() {
    let data = nvs_repo::scratch_private("pkg-login-argument");
    let host = Host::start();
    let out = nvs(&data, &["login", "--registry", &host.url(), GOOD], "");
    assert!(!out.status.success(), "it succeeded: {}", shown(&out));
    assert_eq!(host.served(), 0, "a token on the command line is never sent");
    assert!(token_files(&data).is_empty());
}

#[test]
fn whoami_without_a_stored_token_says_how_to_log_in() {
    let data = nvs_repo::scratch_private("pkg-whoami-none");
    let host = Host::start();
    let out = nvs(&data, &["whoami", "--registry", &host.url()], "");
    refused(&out, "Run `nvs pkg login`");
    assert_eq!(host.served(), 0);
}

#[test]
fn whoami_refuses_a_token_file_another_account_can_write() {
    let data = nvs_repo::scratch_private("pkg-whoami-foreign");
    let host = Host::start();
    let url = host.url();
    let out = nvs(&data, &["login", "--registry", &url], GOOD);
    assert!(out.status.success(), "{}", shown(&out));
    let files = token_files(&data);
    assert_eq!(files.len(), 1, "{files:?}");
    let file = &files[0];

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(file, fs::Permissions::from_mode(0o622))
            .expect("the test makes the file writable by others");
    }
    // `BUILTIN\Users` by SID, because an account name is localized.
    #[cfg(windows)]
    {
        let done = nvs_repo::spawn("icacls", &[])
            .arg(file)
            .args(["/grant", "*S-1-5-32-545:(W)"])
            .output()
            .expect("`icacls` ships with Windows");
        assert!(
            done.status.success(),
            "icacls: {}",
            String::from_utf8_lossy(&done.stderr)
        );
    }

    let out = nvs(&data, &["whoami", "--registry", &url], "");
    refused(&out, "cannot be used for a token");
    assert_eq!(host.served(), 1, "the token in a foreign-writable file is never sent");
}
