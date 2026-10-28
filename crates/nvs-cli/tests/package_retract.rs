//! `nvs pkg retract` and `nvs pkg unlist` against a loopback stand-in for the registry service's
//! `retract`, `unlist` and `relist` routes
//! (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`,
//! `rule:packaging/an-owner-may-unlist`).
//!
//! Each test owns a private data folder, a package folder with a `package.toml` named
//! `Acme\Http`, and a plain-HTTP host on `127.0.0.1`. The host answers `/api/whoami` so
//! `nvs pkg login` stores the token, then records every other request's line and body and answers
//! it as the service answers a token: `200` with a message for a retraction, `202` with the page
//! to confirm on for an unlist, and a refusal for a relist.

use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::Value;

/// The longest the host waits for a request it has accepted. A bound on a stuck client, never a
/// wait the tests rely on.
const BOUND: Duration = Duration::from_secs(30);

/// The token the host accepts.
const GOOD: &str = "nvsreg_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// What the host answers a retraction sent with a token.
const RETRACTED: &str = "Version 1.4.0 is retracted from 2026-11-04T00:00:00Z. An owner can \
                         retract it now with a passkey on the website.";

/// What the host answers an unlist sent with a token.
const CONFIRM: &str =
    "To unlist it, sign in with a passkey and confirm on https://r.example.com/packages/acme/http.";

/// What the host answers a relist sent with a token.
const PASSKEY: &str = "Sign in with a passkey as an owner of the package first.";

/// One request the host served other than `/api/whoami`: its path and its JSON body.
type Seen = (String, Value);

/// A plain-HTTP host on loopback, one connection after another on a thread of its own. Dropping
/// it stops the thread.
struct Host {
    port: u16,
    seen: Arc<Mutex<Vec<Seen>>>,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Host {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the test listens");
        let port = listener.local_addr().expect("it has an address").port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let stopped = Arc::new(AtomicBool::new(false));
        let (kept, stop) = (Arc::clone(&seen), Arc::clone(&stopped));
        let thread = thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                // A request that fails here fails the command, which the case asserts.
                let _ = serve(stream, &kept);
            }
        });
        Self {
            port,
            seen,
            stopped,
            thread: Some(thread),
        }
    }

    /// The registry URL of this host.
    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Every request but `/api/whoami` the host has answered, in order.
    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().expect("no test thread panicked").clone()
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

/// One request on `stream`, answered as the module doc says. Any request without [`GOOD`] as its
/// bearer token gets `401`.
fn serve(mut stream: TcpStream, seen: &Mutex<Vec<Seen>>) -> std::io::Result<()> {
    stream.set_read_timeout(Some(BOUND))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    let mut bearer = None;
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim();
            if name.eq_ignore_ascii_case("authorization") {
                bearer = value.trim().strip_prefix("Bearer ").map(str::to_owned);
            } else if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    let path = request
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_owned();
    let (status, reply) = if bearer.as_deref() != Some(GOOD) {
        (
            "401 Unauthorized",
            r#"{"error":"The token is not valid."}"#.to_owned(),
        )
    } else if path == "/api/whoami" {
        (
            "200 OK",
            r#"{"account":"00112233445566778899aabbccddeeff","scope":"Acme","expiresAt":1830297600}"#
                .to_owned(),
        )
    } else {
        let json = serde_json::from_slice(&body).unwrap_or(Value::Null);
        seen.lock()
            .expect("no test thread panicked")
            .push((path.clone(), json));
        match path.as_str() {
            "/api/packages/acme/http/retract" => (
                "200 OK",
                serde_json::json!({ "message": RETRACTED, "sent": true }).to_string(),
            ),
            "/api/packages/acme/http/unlist" => (
                "202 Accepted",
                serde_json::json!({ "message": CONFIRM }).to_string(),
            ),
            "/api/packages/acme/http/relist" => (
                "403 Forbidden",
                serde_json::json!({ "error": PASSKEY }).to_string(),
            ),
            _ => (
                "404 Not Found",
                r#"{"error":"There is no such package."}"#.to_owned(),
            ),
        }
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{reply}",
        reply.len()
    )?;
    stream.flush()
}

/// `nvs --data <data> pkg <args>` in the folder `cwd`, with `input` on standard input.
fn nvs(data: &Path, cwd: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .current_dir(cwd)
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

/// A package folder named `Acme\Http`, with [`GOOD`] stored for `host` in `data`.
fn logged_in(data: &Path, host: &Host, name: &str) -> nvs_repo::Scratch {
    let package = nvs_repo::scratch(name);
    fs::write(
        package.join("package.toml"),
        "[package]\nname = 'Acme\\Http'\nversion = \"1.4.0\"\n",
    )
    .expect("the test writes the package's manifest");
    let out = nvs(
        data,
        &package,
        &["login", "--registry", &host.url()],
        &format!("{GOOD}\n"),
    );
    assert!(out.status.success(), "{}", shown(&out));
    package
}

// covers: tools:packages/nvs-pkg-retract-and-nvs-pkg-unlist
#[test]
fn retract_sends_the_version_reason_and_fix_and_prints_the_registry_message() {
    let data = nvs_repo::scratch_private("pkg-retract-sends");
    let host = Host::start();
    let package = logged_in(&data, &host, "pkg-retract-sends-package");
    let out = nvs(
        &data,
        &package,
        &[
            "retract",
            "1.4.0",
            "--reason",
            "  It deletes the cache.  ",
            "--fixed-in",
            "1.4.1",
            "--registry",
            &host.url(),
        ],
        "",
    );
    assert!(out.status.success(), "{}", shown(&out));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains(RETRACTED),
        "{}",
        shown(&out)
    );
    let seen = host.seen();
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert_eq!(seen[0].0, "/api/packages/acme/http/retract");
    assert_eq!(
        seen[0].1,
        serde_json::json!({ "version": "1.4.0", "reason": "It deletes the cache.", "fixedIn": "1.4.1" })
    );
}

#[test]
fn retract_refuses_a_malformed_version_or_an_empty_reason_without_sending() {
    let data = nvs_repo::scratch_private("pkg-retract-malformed");
    let host = Host::start();
    let package = logged_in(&data, &host, "pkg-retract-malformed-package");
    let url = host.url();
    for args in [
        &["retract", "1.4", "--reason", "broken", "--registry", &url][..],
        &[
            "retract",
            "1.4.0",
            "--reason",
            "x",
            "--fixed-in",
            "v1",
            "--registry",
            &url,
        ][..],
        &["retract", "1.4.0", "--reason", "   ", "--registry", &url][..],
    ] {
        let out = nvs(&data, &package, args, "");
        assert!(!out.status.success(), "{args:?} succeeded: {}", shown(&out));
    }
    assert!(host.seen().is_empty(), "nothing is sent: {:?}", host.seen());
}

// covers: tools:packages/nvs-pkg-retract-and-nvs-pkg-unlist
#[test]
fn unlist_with_a_token_prints_where_to_confirm_and_a_refused_undo_fails() {
    let data = nvs_repo::scratch_private("pkg-unlist-token");
    let host = Host::start();
    let package = logged_in(&data, &host, "pkg-unlist-token-package");
    let url = host.url();

    let out = nvs(&data, &package, &["unlist", "--registry", &url], "");
    assert!(out.status.success(), "{}", shown(&out));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains(CONFIRM),
        "{}",
        shown(&out)
    );

    let out = nvs(
        &data,
        &package,
        &["unlist", "1.4.0", "--undo", "--registry", &url],
        "",
    );
    assert!(!out.status.success(), "it succeeded: {}", shown(&out));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(PASSKEY),
        "{}",
        shown(&out)
    );

    let seen = host.seen();
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert_eq!(seen[0].0, "/api/packages/acme/http/unlist");
    assert_eq!(seen[0].1, serde_json::json!({ "version": "" }));
    assert_eq!(seen[1].0, "/api/packages/acme/http/relist");
    assert_eq!(seen[1].1, serde_json::json!({ "version": "1.4.0" }));
}
