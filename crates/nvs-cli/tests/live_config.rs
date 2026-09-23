//! `rule:config/reloadability-is-its-own-field`, from outside the process: the
//! built `nvs serve` over a program in a directory of its own, a changed
//! `nvs.toml` and `nvs ctl reload`, and the answer a request gets afterwards.
//!
//! Every case goes through [`Server`], which is `live_edit.rs`'s harness with
//! a control endpoint added. Its `nvs.toml` always names `[control] socket`, so
//! [`Server::reload`] can rewrite the file and ask the running process to read
//! it again. A reload is observed by polling: [`Server::awaits`] asks until the
//! answer is the one wanted or [`BOUND`] runs out, and the failure message says
//! what the last answer was.
//!
//! The program lives under `CARGO_TARGET_TMPDIR`, inside the build directory,
//! and is removed when its [`Server`] is dropped.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How long a reload has to reach a request before the case fails.
const BOUND: Duration = Duration::from_secs(30);

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// The time between two requests of one poll.
const POLL: Duration = Duration::from_millis(50);

/// `[mode] default` written out, and the background check run every 100ms, so
/// a case does not depend on what a configuration with nothing in it starts
/// with.
const PRODUCTION: &str =
    "[mode]\ndefault = \"production\"\n\n[opcache]\nrevalidate_freq = \"100ms\"\n";

/// One answer: the status code and the body, as text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Answer {
    status: u16,
    body: String,
}

/// A running `nvs serve app.nvs` over a program in its own directory.
///
/// Dropping it stops the process and removes the directory.
struct Server {
    child: Child,
    addr: SocketAddr,
    dir: PathBuf,
    /// The control endpoint `nvs.toml` names.
    socket: PathBuf,
    stderr: Arc<Mutex<String>>,
}

impl Server {
    /// Writes `files` — each a path relative to the program's directory and its
    /// text — into a fresh directory named for `case`, writes `nvs.toml` as
    /// [`Server::configure`] does, and starts `nvs serve app.nvs` there, on a
    /// free loopback port.
    ///
    /// # Panics
    ///
    /// When the process cannot start, or does not print its `listening on`
    /// line within [`BOOT`]; the message carries what it wrote to standard
    /// error.
    fn start(case: &str, config: &str, files: &[(&str, &str)]) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("live-config-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        for (path, text) in files {
            write_file(&dir.join(path), text);
        }
        let socket = endpoint(&dir, case);
        write_file(&dir.join("nvs.toml"), &controlled(&socket, config));

        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the `nvs` binary this test was built beside starts");

        // Both pipes are read to their end on a thread of their own, so a
        // server that writes a lot never blocks on a full pipe.
        let stderr = Arc::new(Mutex::new(String::new()));
        let err_pipe = child.stderr.take().expect("standard error is piped");
        let collected = Arc::clone(&stderr);
        thread::spawn(move || {
            for line in BufReader::new(err_pipe).lines().map_while(Result::ok) {
                let mut all = collected.lock().expect("no reader panicked");
                all.push_str(&line);
                all.push('\n');
            }
        });
        let out_pipe = child.stdout.take().expect("standard output is piped");
        let (sent, bound) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(out_pipe).lines().map_while(Result::ok) {
                if let Some(rest) = line.strip_prefix("listening on http://") {
                    let addr = rest
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_owned();
                    let _ = sent.send(addr);
                }
            }
        });

        let Ok(addr) = bound.recv_timeout(BOOT) else {
            let _ = child.kill();
            let _ = child.wait();
            let said = stderr.lock().expect("no reader panicked").clone();
            panic!("`nvs serve` did not start listening within {BOOT:?}: {said}");
        };
        let addr = addr
            .parse()
            .unwrap_or_else(|error| panic!("`{addr}` is not an address: {error}"));
        Self {
            child,
            addr,
            dir,
            socket,
            stderr,
        }
    }

    /// One `GET` of `path`, on a connection of its own.
    ///
    /// # Panics
    ///
    /// When the server does not answer, or answers something that is not HTTP.
    fn get(&self, path: &str) -> Answer {
        let mut stream = TcpStream::connect(self.addr)
            .unwrap_or_else(|error| panic!("{} does not answer: {error}", self.addr));
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost\r\n\r\n")
            .expect("the request is sent");
        let mut raw = Vec::new();
        stream
            .read_to_end(&mut raw)
            .unwrap_or_else(|error| panic!("the answer to `{path}` was cut off: {error}"));
        let text = String::from_utf8_lossy(&raw);
        let (head, body) = text
            .split_once("\r\n\r\n")
            .unwrap_or_else(|| panic!("the answer to `{path}` has no header block: {text}"));
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or_else(|| panic!("the answer to `{path}` has no status: {head}"));
        Answer {
            status,
            body: body.to_owned(),
        }
    }

    /// Polls `path` until `wanted` holds of its answer, and returns that answer.
    ///
    /// # Panics
    ///
    /// When [`BOUND`] runs out first. The message names `what` was awaited, the
    /// last answer and what the server wrote to standard error.
    fn awaits(&self, path: &str, what: &str, wanted: impl Fn(&Answer) -> bool) -> Answer {
        let started = Instant::now();
        loop {
            let answer = self.get(path);
            if wanted(&answer) {
                return answer;
            }
            if started.elapsed() > BOUND {
                let said = self.stderr.lock().expect("no reader panicked").clone();
                panic!(
                    "{what} did not happen within {BOUND:?}; the last answer was {answer:?}, \
                     and the server wrote: {said}"
                );
            }
            thread::sleep(POLL);
        }
    }

    /// Rewrites `nvs.toml` as [`Server::start`] wrote it, with `config` in
    /// place of what it held, and runs `nvs ctl reload` against this server.
    /// Returns what the reload printed.
    ///
    /// # Panics
    ///
    /// When the reload fails; the message carries what both processes wrote to
    /// standard error.
    fn reload(&self, config: &str) -> String {
        write_file(
            &self.dir.join("nvs.toml"),
            &controlled(&self.socket, config),
        );
        let ran = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("ctl")
            .arg("--socket")
            .arg(&self.socket)
            .arg("reload")
            .current_dir(&self.dir)
            .stdin(Stdio::null())
            .output()
            .expect("the `nvs` binary this test was built beside starts");
        assert!(
            ran.status.success(),
            "`nvs ctl reload` failed: {}\nand the server wrote: {}",
            String::from_utf8_lossy(&ran.stderr),
            self.stderr.lock().expect("no reader panicked")
        );
        String::from_utf8_lossy(&ran.stdout).into_owned()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The control endpoint one case's server creates: a socket in its directory
/// on Unix, and a pipe name of its own on Windows.
fn endpoint(dir: &Path, case: &str) -> PathBuf {
    #[cfg(unix)]
    {
        let _ = case;
        dir.join("control.sock")
    }
    #[cfg(windows)]
    {
        let _ = dir;
        PathBuf::from(format!(
            r"\\.\pipe\nvs-live-config-{}-{case}",
            std::process::id()
        ))
    }
}

/// `nvs.toml`: [`PRODUCTION`], `[control] socket` naming `socket`, and then
/// `config`.
fn controlled(socket: &Path, config: &str) -> String {
    format!(
        "{PRODUCTION}\n[control]\nsocket = '{}'\n\n{config}",
        socket.display()
    )
}

/// Writes `text` to `path`, creating any directory it needs.
fn write_file(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the file's directory is created");
    }
    std::fs::write(path, text).unwrap_or_else(|error| {
        panic!("`{}` could not be written: {error}", path.display());
    });
}

/// An `[[app]]` block for `app.nvs` whose origin is `origin`.
fn app_at(origin: &str) -> String {
    format!("[[app]]\nentry = \"app.nvs\"\norigin = \"{origin}\"\n")
}

/// A program that declares one route and prints the absolute link to it.
const LINKING: &str = r#"<?nvs
class Docs {
    #[Core\Route(path: "/here", method: Core\Http\Method::Get, name: "Docs::here")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function here(): string { return "here"; }
}

echo Core\Router::urlAbsolute("Docs::here", []);
"#;

/// `[[app]] origin` is folded into the row a request selects, so a reload that
/// moves it has to fold it again: a request after the reload links from the
/// new origin, and none links from the old one any more.
#[test]
fn a_changed_app_origin_reaches_the_mount_rows() {
    let server = Server::start(
        "origin",
        &app_at("https://one.example.test"),
        &[("app.nvs", LINKING)],
    );
    server.awaits("/here", "a link from the boot's origin", |answer| {
        answer.status == 200 && answer.body.contains("https://one.example.test/here")
    });

    server.reload(&app_at("https://two.example.test"));
    server.awaits("/here", "a link from the reloaded origin", |answer| {
        answer.status == 200 && answer.body.contains("https://two.example.test/here")
    });
    let after = server.get("/here");
    assert!(
        !after.body.contains("one.example.test"),
        "a request after the reload still linked from the old origin: {after:?}"
    );
}
