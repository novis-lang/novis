//! `rule:config/reloadability-is-its-own-field`, from outside the process: the
//! built `nvs serve` over a program in a directory of its own, a changed
//! `nvs.toml` and `nvs ctl reload`, and the answer a request gets afterwards.
//!
//! Every case goes through [`Server`], which is `live_edit.rs`'s harness with
//! a control endpoint added. Its `nvs.toml` always names `[control] socket`, so
//! [`Server::reload`] can rewrite the file and ask the running process to read
//! it again. [`Server::save`] rewrites the file and asks nothing, because the
//! server checks its own files. A reload is observed by polling: [`Server::awaits`] asks until the
//! answer is the one wanted or [`BOUND`] runs out, and the failure message says
//! what the last answer was.
//!
//! The program lives under `CARGO_TARGET_TMPDIR`, inside the build directory,
//! and is removed when its [`Server`] is dropped.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
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

/// `[mode] default` written out, so a case does not depend on what a
/// configuration with nothing in it starts with.
const PRODUCTION: &str = "[mode]\ndefault = \"production\"\n";

/// The background check run every 100ms. [`controlled`] writes it unless the
/// case writes an `[opcache]` block of its own.
const QUICK_CHECKS: &str = "[opcache]\nrevalidate_freq = \"100ms\"\n";

/// One answer: the status code, the header lines and the body, as text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Answer {
    status: u16,
    head: String,
    body: String,
}

impl Answer {
    /// The value of the header `name`, compared without regard to case, or
    /// `None` when the answer does not carry it.
    fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim()
                .eq_ignore_ascii_case(name)
                .then_some(value.trim())
        })
    }
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
        Self::start_after(case, config, files, &[])
    }

    /// [`Server::start`], which first runs `nvs <first>` in the program's
    /// directory when `first` is not empty.
    ///
    /// # Panics
    ///
    /// As [`Server::start`] does, and when that first command fails.
    fn start_after(case: &str, config: &str, files: &[(&str, &str)], first: &[&str]) -> Self {
        Self::boot(case, config, files, first, false)
    }

    /// [`Server::start`], with the server's working directory an empty folder
    /// inside the program's directory. `--config` names `nvs.toml` and the
    /// program is named by its absolute path, so nothing the server runs is
    /// found through the working directory.
    ///
    /// # Panics
    ///
    /// As [`Server::start`] does.
    fn start_elsewhere(case: &str, config: &str, files: &[(&str, &str)]) -> Self {
        Self::boot(case, config, files, &[], true)
    }

    /// [`Server::start_after`], started from the folder [`Server::start_elsewhere`]
    /// names when `elsewhere` is set.
    fn boot(
        case: &str,
        config: &str,
        files: &[(&str, &str)],
        first: &[&str],
        elsewhere: bool,
    ) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("live-config-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        if config.contains("file_cache_dir") {
            private(&dir);
        }
        for (path, text) in files {
            write_file(&dir.join(path), text);
        }
        let socket = endpoint(&dir, case);
        write_file(&dir.join("nvs.toml"), &controlled(&socket, config));
        if !first.is_empty() {
            let ran = Command::new(env!("CARGO_BIN_EXE_nvs"))
                .args(first)
                .current_dir(&dir)
                .stdin(Stdio::null())
                .output()
                .expect("the `nvs` binary this test was built beside starts");
            assert!(
                ran.status.success(),
                "`nvs {}` failed: {}",
                first.join(" "),
                String::from_utf8_lossy(&ran.stderr)
            );
        }

        let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
        if elsewhere {
            let cwd = dir.join("elsewhere");
            std::fs::create_dir_all(&cwd).expect("the working directory is created");
            command
                .arg("--config")
                .arg(dir.join("nvs.toml"))
                .arg("serve")
                .arg(dir.join("app.nvs"))
                .current_dir(cwd);
        } else {
            command.args(["serve", "app.nvs"]).current_dir(&dir);
        }
        let mut child = command
            .args(["--listen", "127.0.0.1:0"])
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
    fn get(&self, path: &str) -> Answer {
        self.get_with(path, &[])
    }

    /// One `GET` of `path` that also sends `headers`, each a name and its
    /// value, on a connection of its own.
    ///
    /// # Panics
    ///
    /// When the server does not answer, or answers something that is not HTTP.
    fn get_with(&self, path: &str, headers: &[(&str, &str)]) -> Answer {
        self.send("GET", path, headers)
    }

    /// One `method` request for `path` with an empty body, sending `headers`,
    /// on a connection of its own.
    ///
    /// # Panics
    ///
    /// When the server does not answer, or answers something that is not HTTP.
    fn send(&self, method: &str, path: &str, headers: &[(&str, &str)]) -> Answer {
        let mut stream = TcpStream::connect(self.addr)
            .unwrap_or_else(|error| panic!("{} does not answer: {error}", self.addr));
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        let extra: String = headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}\r\n"))
            .collect();
        let length = if method == "GET" {
            ""
        } else {
            "Content-Length: 0\r\n"
        };
        write!(
            stream,
            "{method} {path} HTTP/1.0\r\nHost: localhost\r\n{length}{extra}\r\n"
        )
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
            head: head.to_owned(),
            body: body.to_owned(),
        }
    }

    /// Polls `path` until `wanted` holds of its answer, and returns that answer.
    fn awaits(&self, path: &str, what: &str, wanted: impl Fn(&Answer) -> bool) -> Answer {
        self.awaits_with(path, &[], what, wanted)
    }

    /// Polls `path`, sending `headers` each time, until `wanted` holds of its
    /// answer, and returns that answer.
    ///
    /// # Panics
    ///
    /// When [`BOUND`] runs out first. The message names `what` was awaited, the
    /// last answer and what the server wrote to standard error.
    fn awaits_with(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        what: &str,
        wanted: impl Fn(&Answer) -> bool,
    ) -> Answer {
        let started = Instant::now();
        loop {
            let answer = self.get_with(path, headers);
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

    /// What the server has written to standard error so far.
    fn said(&self) -> String {
        self.stderr.lock().expect("no reader panicked").clone()
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
        self.ctl("reload")
    }

    /// Rewrites `nvs.toml` as [`Server::reload`] does, and runs nothing. The
    /// server checks its own configuration files, so it reads the file itself.
    fn save(&self, config: &str) {
        write_file(
            &self.dir.join("nvs.toml"),
            &controlled(&self.socket, config),
        );
    }

    /// Waits until standard error contains `text`, and returns all of it.
    ///
    /// # Panics
    ///
    /// When [`BOUND`] runs out first.
    fn logs(&self, text: &str) -> String {
        let started = Instant::now();
        loop {
            let said = self.said();
            if said.contains(text) {
                return said;
            }
            assert!(
                started.elapsed() < BOUND,
                "`{text}` was not logged within {BOUND:?}; the server wrote: {said}"
            );
            thread::sleep(POLL);
        }
    }

    /// Runs `nvs ctl <request>` against this server and returns what it
    /// printed.
    ///
    /// # Panics
    ///
    /// When the request fails; the message carries what both processes wrote
    /// to standard error.
    fn ctl(&self, request: &str) -> String {
        self.ctl_on(&self.socket, request)
    }

    /// [`Server::ctl`] over the control endpoint `socket`.
    ///
    /// # Panics
    ///
    /// As [`Server::ctl`] does.
    fn ctl_on(&self, socket: &Path, request: &str) -> String {
        let ran = self.ctl_output(socket, request);
        assert!(
            ran.status.success(),
            "`nvs ctl {request}` failed: {}\nand the server wrote: {}",
            String::from_utf8_lossy(&ran.stderr),
            self.stderr.lock().expect("no reader panicked")
        );
        String::from_utf8_lossy(&ran.stdout).into_owned()
    }

    /// Runs `nvs ctl <request>` over the control endpoint `socket`, and
    /// returns how it ended, whether it succeeded or not.
    fn ctl_output(&self, socket: &Path, request: &str) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("ctl")
            .arg("--socket")
            .arg(socket)
            .arg(request)
            .current_dir(&self.dir)
            .stdin(Stdio::null())
            .output()
            .expect("the `nvs` binary this test was built beside starts")
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

/// `nvs.toml`: [`PRODUCTION`] where `config` has no `[mode]` block,
/// [`QUICK_CHECKS`] where it has no `[opcache]` block, `[control] socket`
/// naming `socket`, and then `config`.
fn controlled(socket: &Path, config: &str) -> String {
    let mode = if config.contains("[mode]") {
        ""
    } else {
        PRODUCTION
    };
    let checks = if config.contains("[opcache]") {
        ""
    } else {
        QUICK_CHECKS
    };
    format!(
        "{mode}\n{checks}\n[control]\nsocket = '{}'\n\n{config}",
        socket.display()
    )
}

/// Makes `dir` writable by this account alone. `[opcache] file_cache_dir` is
/// not used when it, or the directory that contains it, is writable by others.
/// [`Server::start_after`] calls this for a case that names one, before it
/// writes the case's files. On Windows a new directory lets every signed-in
/// account write to it, so this replaces its access list with one entry for
/// this account.
///
/// # Panics
///
/// When the access list or the mode cannot be changed.
fn private(dir: &Path) {
    #[cfg(windows)]
    {
        let me = nvs_repo::spawn("whoami", &[])
            .output()
            .expect("`whoami` starts");
        let me = String::from_utf8_lossy(&me.stdout).trim().to_owned();
        let ran = nvs_repo::spawn("icacls", &[])
            .arg(dir)
            .args(["/inheritance:r", "/grant:r", &format!("{me}:(OI)(CI)F")])
            .output()
            .expect("`icacls` starts");
        assert!(
            ran.status.success(),
            "`icacls` could not make `{}` private: {}",
            dir.display(),
            String::from_utf8_lossy(&ran.stdout)
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).unwrap_or_else(
            |error| panic!("`{}` could not be made private: {error}", dir.display()),
        );
    }
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
/// new origin, and none links from the old one any more. The reload's answer
/// names `app` as applied.
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

    let report = server.reload(&app_at("https://two.example.test"));
    assert!(
        report.contains("applied: app\n"),
        "the reload that moved `[[app]] origin` did not name `app` as applied: {report}"
    );
    server.awaits("/here", "a link from the reloaded origin", |answer| {
        answer.status == 200 && answer.body.contains("https://two.example.test/here")
    });
    let after = server.get("/here");
    assert!(
        !after.body.contains("one.example.test"),
        "a request after the reload still linked from the old origin: {after:?}"
    );
}

/// A program that answers every request with one word.
const PLAIN: &str = "<?nvs\necho \"ok\";\n";

/// A program that answers every request with `word`.
fn saying(word: &str) -> String {
    format!("<?nvs\necho \"{word}\";\n")
}

/// An `[opcache]` block with these two keys.
fn opcache(freq: &str, settle: &str) -> String {
    format!("[opcache]\nrevalidate_freq = \"{freq}\"\nsettle = \"{settle}\"\n")
}

/// `[opcache]` reloads into the unit cache. A long `settle` holds an edit back
/// until a reload shortens it. A long `revalidate_freq` holds the next edit
/// back until a reload shortens that too, and the reload does not wait out the
/// hour the server was already waiting.
#[test]
fn a_changed_opcache_block_reaches_the_unit_cache() {
    let app = |server: &Server| server.dir.join("app.nvs");
    let server = Server::start(
        "opcache",
        &opcache("100ms", "1h"),
        &[("app.nvs", &saying("one"))],
    );
    server.awaits("/", "the boot's program", |answer| {
        answer.status == 200 && answer.body == "one"
    });

    write_file(&app(&server), &saying("two"));
    thread::sleep(Duration::from_millis(500));
    let held = server.get("/");
    assert_eq!(
        held.body, "one",
        "an edit reached a request inside `settle`"
    );

    let report = server.reload(&opcache("1h", "0s"));
    for key in ["opcache.revalidate_freq", "opcache.settle"] {
        assert!(
            report.contains(&format!("applied: {key}\n")),
            "the reload did not name `{key}` as applied: {report}"
        );
    }
    server.awaits("/", "the edit, once `settle` is short", |answer| {
        answer.status == 200 && answer.body == "two"
    });

    write_file(&app(&server), &saying("three"));
    thread::sleep(Duration::from_millis(500));
    let held = server.get("/");
    assert_eq!(
        held.body, "two",
        "an edit was checked inside `revalidate_freq`"
    );

    server.reload(&opcache("100ms", "0s"));
    server.awaits("/", "the edit, once `revalidate_freq` is short", |answer| {
        answer.status == 200 && answer.body == "three"
    });
}

/// `[http.headers]` reloads: the header set every response carries is the one
/// the tree a request started under configures, so a reload that moves
/// `referrer_policy` moves the header on the next response.
#[test]
fn a_changed_http_headers_block_reaches_the_next_response() {
    let server = Server::start(
        "headers",
        "[http.headers]\nreferrer_policy = \"no-referrer\"\n",
        &[("app.nvs", PLAIN)],
    );
    server.awaits("/", "the boot's `Referrer-Policy`", |answer| {
        answer.status == 200 && answer.header("referrer-policy") == Some("no-referrer")
    });

    server.reload("[http.headers]\nreferrer_policy = \"same-origin\"\n");
    server.awaits("/", "the reloaded `Referrer-Policy`", |answer| {
        answer.status == 200 && answer.header("referrer-policy") == Some("same-origin")
    });
}

/// `[http.cors]` reloads: a server that booted closed grants an origin a reload
/// named, and takes the grant away again when a second reload removes it.
#[test]
fn a_changed_cors_block_reaches_the_next_response() {
    const ORIGIN: &str = "https://app.example.test";
    let crossing = [("Origin", ORIGIN)];
    let server = Server::start("cors", "", &[("app.nvs", PLAIN)]);
    let closed = server.get_with("/", &crossing);
    assert_eq!(closed.status, 200, "the boot's answer: {closed:?}");
    assert_eq!(
        closed.header("access-control-allow-origin"),
        None,
        "a server that booted closed granted an origin: {closed:?}"
    );

    server.reload(&format!("[http.cors]\norigins = [\"{ORIGIN}\"]\n"));
    server.awaits_with("/", &crossing, "the reloaded grant", |answer| {
        answer.status == 200 && answer.header("access-control-allow-origin") == Some(ORIGIN)
    });

    server.reload("");
    server.awaits_with("/", &crossing, "the grant taken away", |answer| {
        answer.status == 200 && answer.header("access-control-allow-origin").is_none()
    });
}

/// A program that answers `ok`, after twenty seconds when the query says
/// `hold=yes`.
const HOLDING: &str = r#"<?nvs
if (Core\Request::query("hold") == "yes") {
    Core\Time::sleep(20s);
}
echo "ok";
"#;

/// A `[limits]` block whose `memory` is `bytes`.
fn memory(bytes: u64) -> String {
    format!("[limits]\nmemory = {bytes}\n")
}

/// `limits.memory` is one input of the admission ceiling, so a reload that
/// moves it moves the ceiling. A cap of a pebibyte a request leaves this
/// machine room for one request, so while one is held in flight the next is
/// answered `503`.
#[test]
fn a_changed_memory_limit_moves_the_admission_ceiling() {
    const SMALL: u64 = 64 * 1024 * 1024;
    const HUGE: u64 = 1 << 50;
    the_ceiling_follows("memory", &memory(SMALL), &memory(HUGE), "limits.memory");
}

/// `[server] max_in_flight` is the other input of the admission ceiling. A
/// ceiling of one, with one request held in flight, answers the next `503`.
#[test]
fn a_changed_in_flight_ceiling_moves_the_admission_ceiling() {
    the_ceiling_follows(
        "in-flight",
        "[server]\nmax_in_flight = 8\n",
        "[server]\nmax_in_flight = 1\n",
        "server.max_in_flight",
    );
}

/// Holds one request in flight under `roomy`, reloads `tight`, and expects the
/// next request to be answered `503` and the reload to name `applied`. The held
/// request keeps its place across both reloads, and a second reload back to
/// `roomy` admits the next request again.
fn the_ceiling_follows(case: &str, roomy: &str, tight: &str, applied: &str) {
    let server = Server::start(case, roomy, &[("app.nvs", HOLDING)]);
    server.awaits("/", "the boot's program", |answer| {
        answer.status == 200 && answer.body == "ok"
    });

    // The held request's answer is never read: the server is stopped under
    // it when the case ends, so every error here is expected.
    let addr = server.addr;
    thread::spawn(move || {
        if let Ok(mut stream) = TcpStream::connect(addr) {
            let _ = write!(stream, "GET /?hold=yes HTTP/1.0\r\nHost: localhost\r\n\r\n");
            let _ = stream.read_to_end(&mut Vec::new());
        }
    });
    let started = Instant::now();
    while !server.ctl("status").contains("in_flight: 1\n") {
        assert!(
            started.elapsed() < BOUND,
            "the held request was not in flight within {BOUND:?}"
        );
        thread::sleep(POLL);
    }

    let report = server.reload(tight);
    assert!(
        report.contains(&format!("applied: {applied}\n")),
        "the reload did not name `{applied}` as applied: {report}"
    );
    let refused = server.awaits("/", "a refusal at the lowered ceiling", |answer| {
        answer.status == 503
    });
    assert_eq!(refused.header("retry-after"), Some("1"), "{refused:?}");
    assert!(
        server.ctl("status").contains("in_flight: 1\n"),
        "the held request lost its place in the count"
    );

    server.reload(roomy);
    server.awaits("/", "an admission at the raised ceiling", |answer| {
        answer.status == 200 && answer.body == "ok"
    });
}

/// A program that answers whether its server is draining, after five seconds
/// when the query says `hold=yes`.
const PAUSING: &str = r#"<?nvs
if (Core\Request::query("hold") == "yes") {
    Core\Time::sleep(5s);
}
echo Core\Server::isDraining() ? "draining" : "serving";
"#;

/// A connection to `server` with one whole request answered on it, so it is
/// kept alive and was accepted under the tree published now.
fn kept_alive(server: &Server) -> TcpStream {
    let mut stream = TcpStream::connect(server.addr).expect("the server accepts");
    stream
        .set_read_timeout(Some(BOUND))
        .expect("a read timeout can be set");
    write!(stream, "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").expect("the request is written");
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        stream
            .read_exact(&mut byte)
            .unwrap_or_else(|error| panic!("the first answer ended early: {error}"));
        head.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&head).to_lowercase();
    let length: usize = head
        .lines()
        .find_map(|line| line.strip_prefix("content-length:"))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or_else(|| panic!("the first answer has no length: {head}"));
    stream
        .read_exact(&mut vec![0; length])
        .expect("the first answer's body arrives");
    stream
}

/// A reload drains the connections the old tree accepted, and nothing else. A
/// kept-alive connection with no request is closed. A request in flight when
/// `nvs ctl reload` publishes a changed tree is answered. The reloaded health
/// probe, `nvs ctl status` and `Core\Server::isDraining()` all report a
/// server that is still accepting.
// covers: tools:server/stopping-and-reloading-the-drain
#[test]
fn a_reload_drains_the_connections_the_old_tree_accepted() {
    let server = Server::start("reload-drain", "", &[("app.nvs", PAUSING)]);
    server.awaits("/", "the boot's program", |answer| {
        answer.status == 200 && answer.body == "serving"
    });
    let mut idle = kept_alive(&server);

    thread::scope(|scope| {
        let held = scope.spawn(|| server.get("/?hold=yes"));
        let started = Instant::now();
        while !server.ctl("status").contains("in_flight: 1\n") {
            assert!(
                started.elapsed() < BOUND,
                "the held request was not in flight within {BOUND:?}"
            );
            thread::sleep(POLL);
        }

        // The probe's empty body is what tells it from the program, which
        // answers every path until the reload names this one.
        server.reload("[server]\nhealth_path = \"/up\"\n");
        // Before any new connection, so the close is the reload's own and not
        // the next accept's.
        let closed = idle.read(&mut [0_u8; 1]);
        assert!(
            matches!(closed, Ok(0)),
            "the kept-alive connection the old tree accepted was not closed cleanly: \
             {closed:?}"
        );
        server.awaits("/up", "the reloaded health path", |answer| {
            answer.status == 200 && answer.body.is_empty()
        });
        let status = server.ctl("status");
        assert!(
            status.contains("draining: false\n"),
            "the reload drained the server and not only its connections: {status}"
        );

        let answer = held.join().expect("the held request's thread panicked");
        assert_eq!(
            (answer.status, answer.body.as_str()),
            (200, "serving"),
            "the request in flight across the reload was not answered: {answer:?}"
        );
    });
}

/// A program that says whether the client address it was given is the one a
/// proxy forwarded.
const ADDRESSED: &str =
    "<?nvs\necho Core\\Request::clientIp() == \"203.0.113.9\" ? \"forwarded\" : \"direct\";\n";

/// `[server]`'s switches, as the boot writes them and as the reload does.
const SWITCHES_OFF: &str = "[server]\ndispatch = \"entry\"\nstatic = false\n";
const SWITCHES_ON: &str = "[server]\ndispatch = \"path\"\nstatic = true\nhealth_path = \"/up\"\n\
                           trusted_proxies = [\"127.0.0.1/32\"]\n";

/// The keys of `[server]` a request reads from the snapshot it cloned take the
/// reloaded value in the next request: `dispatch` runs the file the path names,
/// `static` sends a stylesheet, `health_path` answers the probe, and
/// `trusted_proxies` believes the proxy's `X-Forwarded-For`.
// covers: tools:server/behind-a-proxy-trusted-proxies, tools:server/the-server-block
#[test]
fn a_changed_server_block_reaches_the_next_request() {
    let server = Server::start(
        "server-block",
        SWITCHES_OFF,
        &[
            ("app.nvs", ADDRESSED),
            ("other.nvs", "<?nvs\necho \"other\";\n"),
            ("style.css", "body {}\n"),
        ],
    );
    let forwarded = [("x-forwarded-for", "203.0.113.9")];
    server.awaits("/", "the boot's program", |answer| {
        answer.status == 200 && answer.body == "direct"
    });
    for path in ["/other.nvs", "/style.css", "/up"] {
        let answer = server.get(path);
        assert_eq!(
            answer.body, "direct",
            "`{path}` before the reload: {answer:?}"
        );
    }
    let answer = server.get_with("/app.nvs", &forwarded);
    assert_eq!(answer.body, "direct", "a proxy nobody trusts was believed");

    let report = server.reload(SWITCHES_ON);
    for key in ["dispatch", "static", "health_path", "trusted_proxies"] {
        assert!(
            report.contains(&format!("applied: server.{key}\n")),
            "the reload did not name `server.{key}` as applied: {report}"
        );
    }
    server.awaits("/other.nvs", "the file the path names", |answer| {
        answer.status == 200 && answer.body == "other"
    });
    let sent = server.get("/style.css");
    assert_eq!(sent.body, "body {}\n", "{sent:?}");
    let probe = server.get("/up");
    assert!(
        probe.status == 200 && probe.body != "direct",
        "the health path did not answer the probe: {probe:?}"
    );
    let answer = server.get_with("/app.nvs", &forwarded);
    assert_eq!(
        answer.body, "forwarded",
        "the trusted proxy was not believed"
    );
}

/// A mount table of one block: `[server] root` is `root` and the block mounts
/// `app.nvs` under it at `prefix`.
fn mounted(root: &str, prefix: &str) -> String {
    format!(
        "[server]\nroot = \"{root}\"\n\n[[server.mount]]\nentry = \"app.nvs\"\nprefix = \"{prefix}\"\n"
    )
}

/// `[server] root` and `[[server.mount]]` are read from the tree a reload
/// published, and a change to either expands the table again: after the
/// reload the new prefix answers from the new root's file, and the old prefix
/// answers `404`. The reload names both keys as applied and neither as
/// ignored.
// covers: tools:server/mounts-which-file-answers-a-request
#[test]
fn a_changed_mount_table_is_expanded_again() {
    let server = Server::start(
        "mount-table",
        &mounted(".", "/one"),
        &[
            ("app.nvs", &saying("boot")),
            ("site/app.nvs", &saying("moved")),
        ],
    );
    server.awaits("/one", "the boot's mount", |answer| {
        answer.status == 200 && answer.body == "boot"
    });
    let before = server.get("/two");
    assert_eq!(
        before.status, 404,
        "a prefix nothing mounts answered: {before:?}"
    );

    let report = server.reload(&mounted("site", "/two"));
    for key in ["root", "mount"] {
        assert!(
            report.contains(&format!("applied: server.{key}\n")),
            "the reload did not name `server.{key}` as applied: {report}"
        );
        assert!(
            !report.contains(&format!("ignored: server.{key}\n")),
            "the reload named `server.{key}` as needing a restart: {report}"
        );
    }
    server.awaits("/two", "the reloaded mount", |answer| {
        answer.status == 200 && answer.body == "moved"
    });
    let gone = server.get("/one");
    assert_eq!(gone.status, 404, "the old prefix still answered: {gone:?}");
}

/// A SQLite queue with one worker, whose `[queue] visibility` is `visibility`
/// and whose `[queue] max_attempts` is `attempts`. The boot writes `1`, which
/// moves a job that throws to the dead-letter table at once, so no retry of a
/// job pushed before the reload writes a line after it.
fn queue(visibility: &str, attempts: u32) -> String {
    format!(
        "[capabilities.script]\nspawn = [\"jobs/\"]\n\n[db.jobs]\ndriver = \"sqlite\"\npath = \
         \"jobs.db\"\n\n[queue]\nconnection = \"jobs\"\nworkers = 1\nmax_attempts = \
         {attempts}\nvisibility = \"{visibility}\"\n"
    )
}

/// An entry file that pushes one job per request, and prints the `[queue]
/// max_attempts` the push read.
const PUSHING: &str = "<?nvs\nCore\\Queue::push(\"jobs/work.nvs\");\necho \"pushed \", \
                       Core\\Config::get('queue.max_attempts');\n";

/// A job that throws the `[queue] visibility` it reads. A throw is what the
/// worker reports on standard error.
const READING: &str =
    "<?nvs\nthrow new RuntimeError(\"visibility \" . Core\\Config::get('queue.visibility'));\n";

/// A queue job runs under the snapshot in force when a worker claims it, and
/// not under the one the worker started with. After a reload moves `[queue]
/// visibility`, a job pushed afterwards reads the new value. The same reload
/// moves `[queue] max_attempts`, and every push after it reads the new value.
#[test]
fn a_queue_job_runs_under_the_configuration_in_force_when_it_is_claimed() {
    // The worker names the job by the absolute path the pushed literal was joined to.
    const THREW: &str = "work.nvs` threw RuntimeError: visibility ";
    let server = Server::start_after(
        "queue",
        &queue("5m", 1),
        &[("app.nvs", PUSHING), ("jobs/work.nvs", READING)],
        &["queue", "migrate"],
    );
    server.awaits("/", "the first push", |answer| answer.body == "pushed 1");
    let started = Instant::now();
    while !server.said().contains(&format!("{THREW}5m")) {
        assert!(
            started.elapsed() <= BOUND,
            "no queued job read `5m` within {BOUND:?}; the server wrote: {}",
            server.said()
        );
        thread::sleep(POLL);
    }

    let report = server.reload(&queue("7m", 2));
    for key in ["queue.max_attempts", "queue.visibility"] {
        assert!(
            report.contains(&format!("applied: {key}\n")),
            "the reload did not name `{key}` as applied: {report}"
        );
    }

    // One job per push, pushed until a job runs, far slower than `POLL` so the
    // queue never holds more than a few.
    let seen = server.said().len();
    let started = Instant::now();
    let after = loop {
        let said = server.said();
        if let Some(line) = said[seen..].lines().find(|line| line.contains(THREW)) {
            break line.to_owned();
        }
        assert!(
            started.elapsed() <= BOUND,
            "no job pushed after the reload ran within {BOUND:?}; the server wrote: {said}"
        );
        server.awaits("/", "a push under the reloaded `max_attempts`", |answer| {
            answer.body == "pushed 2"
        });
        thread::sleep(POLL * 10);
    };
    assert!(
        after.ends_with(&format!("{THREW}7m")),
        "a job claimed after the reload did not read the reloaded `[queue] visibility`: {after}"
    );
}

/// Two SQLite blocks, `jobs` and `other`, and a queue on `connection` with
/// `workers` workers. A job that throws is dead-lettered at once, so each job
/// writes one line.
fn crew(connection: &str, workers: u32) -> String {
    format!(
        "[capabilities.script]\nspawn = [\"jobs/\"]\n\n[db.jobs]\ndriver = \"sqlite\"\npath = \
         \"jobs.db\"\n\n[db.other]\ndriver = \"sqlite\"\npath = \"other.db\"\n\n[queue]\n\
         connection = \"{connection}\"\nworkers = {workers}\nmax_attempts = 1\n"
    )
}

/// An entry file that pushes one slow job per request.
const PUSHING_SLOW: &str = "<?nvs\nCore\\Queue::push(\"jobs/slow.nvs\");\necho \"pushed\";\n";

/// A job that waits one second and then throws. A throw is what the worker
/// reports on standard error.
const SLOW: &str = "<?nvs\nCore\\Time::sleep(1s);\nthrow new RuntimeError(\"finished\");\n";

/// A reload that changes `[queue] workers` starts or stops workers, and one
/// that changes `[queue] connection` moves them. A stopped worker finishes
/// the job it holds first. A new connection whose tables are missing refuses
/// the reload, and the workers keep running where they were.
#[test]
fn a_changed_queue_worker_count_starts_and_stops_workers_after_their_current_job() {
    // The worker names the job by the absolute path the pushed literal was joined to.
    const FINISHED: &str = "slow.nvs` threw RuntimeError: finished";
    let server = Server::start_after(
        "crew",
        &crew("jobs", 1),
        &[("app.nvs", PUSHING_SLOW), ("jobs/slow.nvs", SLOW)],
        &["queue", "migrate"],
    );
    let finished = |count: usize, what: &str| {
        eventually(&server, what, || times(&server.said(), FINISHED) >= count);
    };
    let push = || server.awaits("/", "a push", |answer| answer.body == "pushed");

    push();
    finished(1, "the job the boot's worker claimed");

    // The worker is holding a job when the reload stops it, and the job ends.
    push();
    thread::sleep(POLL * 6);
    let report = server.reload(&crew("jobs", 0));
    assert!(
        report.contains("applied: queue.workers\n"),
        "the reload did not name `queue.workers` as applied: {report}"
    );
    server.logs("1 queue worker on `[db.jobs]` stops after the current job");
    finished(2, "the job the stopped worker was holding");

    // No worker runs now, so a job pushed now waits.
    push();
    thread::sleep(Duration::from_secs(3));
    assert_eq!(
        times(&server.said(), FINISHED),
        2,
        "a job ran with `[queue] workers = 0`: {}",
        server.said()
    );

    let report = server.reload(&crew("jobs", 2));
    assert!(
        report.contains("applied: queue.workers\n"),
        "the reload did not name `queue.workers` as applied: {report}"
    );
    server.logs("2 queue workers started on `[db.jobs]`");
    finished(3, "the waiting job, claimed by a started worker");

    // `other.db` has no queue tables yet, so the reload is refused whole.
    write_file(
        &server.dir.join("nvs.toml"),
        &controlled(&server.socket, &crew("other", 2)),
    );
    let refused = server.ctl_output(&server.socket, "reload");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(
        !refused.status.success() && said.contains("is behind the queue's schema"),
        "a reload onto a connection with no queue tables was not refused: {said}"
    );

    let migrated = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["queue", "migrate", "--connection", "other"])
        .current_dir(&server.dir)
        .stdin(Stdio::null())
        .output()
        .expect("the `nvs` binary this test was built beside starts");
    assert!(
        migrated.status.success(),
        "`nvs queue migrate --connection other` failed: {}",
        String::from_utf8_lossy(&migrated.stderr)
    );
    server.ctl("reload");
    server.logs("2 queue workers on `[db.jobs]` stop after the current job");
    server.logs("2 queue workers started on `[db.other]`");
    push();
    finished(4, "a job pushed onto the new connection");
}

/// A deployment whose scripts may live under `jobs/`, with no `[[schedule]]`
/// entry until `entry` adds one.
fn scheduling(entry: &str) -> String {
    format!("[capabilities.script]\nspawn = [\"jobs/\"]\n\n{entry}")
}

/// A `[[schedule]]` entry that runs `jobs/tick.nvs` every minute.
const MINUTELY: &str = "[[schedule]]\nname = \"minutely\"\ncron = \"* * * * *\"\nscript = \
                        \"jobs/tick.nvs\"\nscope = \"host\"\n";

/// A scheduled script that throws. A throw is what the server reports for a
/// fire on standard error.
const TICKING: &str = "<?nvs\nthrow new RuntimeError(\"ticked\");\n";

/// A server that booted with no `[[schedule]]` entry fires the first one a
/// reload adds. The entry fires at the next whole minute, so this case waits up
/// to a minute more than the others.
#[test]
fn a_changed_schedule_roster_is_armed_from_the_next_tick() {
    let server = Server::start(
        "schedule",
        &scheduling(""),
        &[("app.nvs", PLAIN), ("jobs/tick.nvs", TICKING)],
    );
    server.awaits("/", "the first answer", |answer| answer.body == "ok");

    let report = server.reload(&scheduling(MINUTELY));
    assert!(
        report.contains("applied: schedule\n"),
        "the reload did not name `schedule` as applied: {report}"
    );
    ticks(&server, "the entry a reload added");
}

/// Waits until `server` reports that `jobs/tick.nvs` ran for the entry
/// `minutely` and threw.
///
/// # Panics
///
/// When it does not within a minute more than [`BOUND`]. The message names
/// `entry` and carries what the server wrote.
fn ticks(server: &Server, entry: &str) {
    let fired = "the scheduled entry `minutely` threw RuntimeError: ticked";
    let started = Instant::now();
    while !server.said().contains(fired) {
        assert!(
            started.elapsed() <= BOUND + Duration::from_secs(60),
            "{entry} did not fire within a minute; the server wrote: {}",
            server.said()
        );
        thread::sleep(POLL * 10);
    }
}

/// `[[schedule]] script` is written relative in `nvs.toml`, and names the file
/// beside that configuration. The server is started from another folder, and
/// the entry still runs that file.
#[test]
fn a_schedule_script_beside_the_configuration_runs_from_any_working_directory() {
    let server = Server::start_elsewhere(
        "schedule-elsewhere",
        &scheduling(MINUTELY),
        &[("app.nvs", PLAIN), ("jobs/tick.nvs", TICKING)],
    );
    ticks(&server, "the entry beside the configuration");
}

/// A `[log] handler` that writes `jobs/reported.txt` beside itself.
fn reporting() -> String {
    format!(
        "{}\n[capabilities.fs]\nwrite = [\"jobs/\"]\n\n[log]\nhandler = \"jobs/report.nvs\"\n",
        scheduling("")
    )
}

/// A program that throws, so every request it answers reaches the handler.
const FAILING: &str = "<?nvs\nthrow new RuntimeError(\"broke\");\n";

/// The handler `reporting` names. Its literal path resolves beside this file.
const REPORT: &str = "<?nvs\nCore\\IO::write('reported.txt', 'reported');\n";

/// `[log] handler` is written relative in `nvs.toml`, and names the file beside
/// that configuration. The server is started from another folder, a request
/// throws, and the handler still runs.
#[test]
fn a_log_handler_beside_the_configuration_runs_from_any_working_directory() {
    let server = Server::start_elsewhere(
        "handler-elsewhere",
        &reporting(),
        &[("app.nvs", FAILING), ("jobs/report.nvs", REPORT)],
    );
    let failed = server.get("/");
    assert_eq!(failed.status, 500, "the throwing program did not fail");
    let report = server.dir.join("jobs").join("reported.txt");
    let started = Instant::now();
    while !report.exists() {
        assert!(
            started.elapsed() <= BOUND,
            "the handler beside the configuration did not run; the server wrote: {}",
            server.said()
        );
        thread::sleep(POLL);
    }
}

/// A stand-in shared store on a loopback port of its own. It reads each
/// command, answers `$-1` (nothing stored) to a `GET` and `+OK` to every
/// other one, and keeps the name of each command it read.
struct Store {
    addr: SocketAddr,
    commands: Arc<Mutex<Vec<String>>>,
}

impl Store {
    /// Binds a free port and answers every connection on a thread of its own.
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port is free");
        let addr = listener
            .local_addr()
            .expect("a bound socket has an address");
        let commands = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&commands);
        thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                let kept = Arc::clone(&kept);
                thread::spawn(move || answered(stream, &kept));
            }
        });
        Self { addr, commands }
    }

    /// The URL `[cache.shared] url` names it by.
    fn url(&self) -> String {
        format!("redis://{}", self.addr)
    }

    /// Whether a command named `name` has arrived.
    fn received(&self, name: &str) -> bool {
        self.commands
            .lock()
            .expect("no reader panicked")
            .iter()
            .any(|arrived| arrived == name)
    }
}

/// Reads commands from `stream` until it closes, answers each as [`Store`]
/// does, and keeps each command's name in `kept`.
fn answered(stream: TcpStream, kept: &Mutex<Vec<String>>) {
    /// The number after `marker` on one line, or `None` at the end of the
    /// stream or on a line that is not one.
    fn counted(reader: &mut impl BufRead, marker: char) -> Option<usize> {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        line.trim_end().strip_prefix(marker)?.parse().ok()
    }

    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    while let Some(count) = counted(&mut reader, '*') {
        let mut parts = Vec::new();
        for _ in 0..count {
            let Some(length) = counted(&mut reader, '$') else {
                return;
            };
            let mut part = vec![0; length + 2];
            if reader.read_exact(&mut part).is_err() {
                return;
            }
            part.truncate(length);
            parts.push(part);
        }
        let name = parts
            .first()
            .map(|name| String::from_utf8_lossy(name).to_uppercase())
            .unwrap_or_default();
        let reply: &[u8] = if name == "GET" {
            b"$-1\r\n"
        } else {
            b"+OK\r\n"
        };
        kept.lock().expect("no reader panicked").push(name);
        if writer.write_all(reply).is_err() {
            return;
        }
    }
}

/// A deployment whose shared store is `store`, granted to `app.nvs`, with
/// `[session] backend` written as `backend`.
fn sessions(store: &Store, backend: &str) -> String {
    format!(
        "[[app]]\nentry = \"app.nvs\"\n\n[app.capabilities.cache]\nshared = true\n\n\
         [cache.shared]\nurl = \"{}\"\n\n[session]\nbackend = \"{backend}\"\n",
        store.url()
    )
}

/// An entry file that starts a session, and prints `started`, or `refused`
/// when `Core\Session::start` throws.
const SESSIONED: &str = "<?nvs\ntry {\n    Core\\Session::start();\n    echo \"started\";\n} \
                         catch (RuntimeError $refused) {\n    echo \"refused\";\n}\n";

/// `Core\Session::start` reads `[session]` from the snapshot its request
/// cloned. A server that booted with `backend = "db"`, which this build does
/// not serve, refuses every session. After a reload names `shared`, the next
/// request starts one, and the record reaches the shared store.
#[test]
fn a_changed_session_backend_applies_to_new_requests() {
    let store = Store::start();
    let server = Server::start(
        "session",
        &sessions(&store, "db"),
        &[("app.nvs", SESSIONED)],
    );
    server.awaits("/", "the boot's refusal", |answer| answer.body == "refused");
    assert!(
        !store.received("SET"),
        "a backend that refuses every session wrote a record"
    );

    let report = server.reload(&sessions(&store, "shared"));
    assert!(
        report.contains("applied: session.backend\n"),
        "the reload did not name `session.backend` as applied: {report}"
    );
    server.awaits("/", "a session on the reloaded backend", |answer| {
        answer.body == "started"
    });
    assert!(
        store.received("SET"),
        "a session started after the reload wrote no record to the shared store"
    );
}

/// A deployment whose shared store is `store`, granted to `app.nvs`, with a
/// `scope = "fleet"` entry that runs `jobs/tick.nvs` every minute.
fn fleet(store: &Store) -> String {
    format!(
        "[[app]]\nentry = \"app.nvs\"\n\n[app.capabilities.cache]\nshared = true\n\n\
         [cache.shared]\nurl = \"{}\"\n\n{}\n[[schedule]]\nname = \"fleet-minutely\"\ncron = \
         \"* * * * *\"\nscript = \"jobs/tick.nvs\"\nscope = \"fleet\"\n",
        store.url(),
        scheduling("")
    )
}

/// An entry file that reads one key from the shared store and prints `asked`.
const ASKING: &str = "<?nvs\nCore\\Cache::shared()->get(\"live\");\necho \"asked\";\n";

/// A reload that moves `[cache.shared] url` reaches both of its readers. The
/// next request reads from the new store, and the next fire of a fleet entry
/// takes its lease there. The fire comes at the next whole minute, so this case
/// waits up to a minute more than the others.
#[test]
fn a_changed_shared_store_takes_the_next_request_and_the_next_fleet_lease() {
    let first = Store::start();
    let second = Store::start();
    let server = Server::start(
        "shared-store",
        &fleet(&first),
        &[("app.nvs", ASKING), ("jobs/tick.nvs", TICKING)],
    );
    server.awaits("/", "the first answer", |answer| answer.body == "asked");
    assert!(
        first.received("GET"),
        "the request before the reload did not reach the first store"
    );

    let report = server.reload(&fleet(&second));
    assert!(
        report.contains("applied: cache.shared.url\n"),
        "the reload did not name `cache.shared.url` as applied: {report}"
    );
    server.awaits("/", "an answer after the reload", |answer| {
        answer.body == "asked" && second.received("GET")
    });

    let started = Instant::now();
    while !second.received("SET") {
        assert!(
            started.elapsed() <= BOUND + Duration::from_secs(60),
            "no fleet fire took its lease in the new store within a minute; the server wrote: {}",
            server.said()
        );
        thread::sleep(POLL * 10);
    }
}

/// A second control endpoint for the server in `dir`: a socket beside the
/// first on Unix, and a pipe name of its own on Windows.
fn moved_endpoint(dir: &Path) -> PathBuf {
    #[cfg(unix)]
    {
        dir.join("moved.sock")
    }
    #[cfg(windows)]
    {
        let _ = dir;
        PathBuf::from(format!(
            r"\\.\pipe\nvs-live-config-{}-moved",
            std::process::id()
        ))
    }
}

/// `[control] socket` moves without a restart. A reload pushed over the old
/// endpoint is answered there and names the key as applied. After it, the new
/// endpoint answers `nvs ctl` and the old one does not. A name whose endpoint
/// cannot be created is logged and named as not applied, and the endpoint in
/// force keeps answering.
// covers: tools:server/nvs-ctl
#[test]
fn a_changed_control_socket_moves_the_control_endpoint() {
    let server = Server::start("moves", "", &[("app.nvs", PLAIN)]);
    server.awaits("/", "the boot's answer", |answer| answer.status == 200);

    let moved = moved_endpoint(&server.dir);
    write_file(&server.dir.join("nvs.toml"), &controlled(&moved, ""));
    let report = server.ctl("reload");
    assert!(
        report.contains("applied: control.socket\n"),
        "the reload did not name `control.socket` as applied: {report}"
    );
    let status = server.ctl_on(&moved, "status");
    assert!(
        status.contains("in_flight:"),
        "the moved endpoint did not answer `status`: {status}"
    );
    let started = Instant::now();
    while server.ctl_output(&server.socket, "status").status.success() {
        assert!(
            started.elapsed() < BOUND,
            "the old endpoint still answers after the reload moved it; the server wrote: {}",
            server.said()
        );
        thread::sleep(POLL);
    }

    // A directory that does not exist, which no endpoint can be created in.
    let missing = server.dir.join("missing").join("control.sock");
    write_file(&server.dir.join("nvs.toml"), &controlled(&missing, ""));
    let report = server.ctl_on(&moved, "reload");
    assert!(
        report.contains("ignored: control.socket\n"),
        "the reload did not name `control.socket` as not applied: {report}"
    );
    let said = server.logs("configuration key not applied");
    assert!(
        said.contains("missing"),
        "the record does not name the endpoint that was written: {said}"
    );
    let status = server.ctl_on(&moved, "status");
    assert!(
        status.contains("in_flight:"),
        "the endpoint in force stopped answering after a move that failed: {status}"
    );
}

/// `nvs ctl` from outside the process, one request after another. `config`
/// lists a key the server runs with beside the file that wrote it. A `reload`
/// over a file that does not parse fails: it prints the file and the line,
/// says that the running configuration is unchanged, and `config` still lists
/// the value from before. A `reload` over the repaired file is answered with
/// its report, and `config` lists the new value. The same three requests sent
/// to the HTTP port reach the program, as any other path does.
// covers: tools:server/nvs-ctl
#[test]
fn nvs_ctl_reads_and_reloads_a_running_server_and_the_http_port_does_neither() {
    const POLICY: &str = "http.headers.referrer_policy";
    let server = Server::start(
        "ctl",
        "[http.headers]\nreferrer_policy = \"no-referrer\"\n",
        &[("app.nvs", PLAIN)],
    );
    server.awaits("/", "the boot's answer", |answer| answer.status == 200);
    let held = |listing: &str| {
        listing
            .lines()
            .find(|line| line.starts_with(POLICY))
            .unwrap_or_else(|| panic!("`nvs ctl config` does not list `{POLICY}`: {listing}"))
            .to_owned()
    };

    let row = held(&server.ctl("config"));
    assert!(
        row.contains("\"no-referrer\"") && row.trim_end().ends_with("nvs.toml"),
        "the row does not give the value and the file that wrote it: {row}"
    );

    server.save("[http.headers]\nreferrer_policy = \"same-ori");
    let refused = server.ctl_output(&server.socket, "reload");
    let said = String::from_utf8_lossy(&refused.stderr).into_owned();
    assert!(
        !refused.status.success() && refused.stdout.is_empty(),
        "a reload over a file that does not parse was reported as done: {said}"
    );
    assert!(
        said.contains("nvs.toml:"),
        "the refusal does not name the file and the line: {said}"
    );
    assert!(
        said.contains("note: the running configuration is unchanged"),
        "the refusal does not say what the server runs with now: {said}"
    );
    let row = held(&server.ctl("config"));
    assert!(
        row.contains("\"no-referrer\""),
        "a refused reload changed the running configuration: {row}"
    );

    let report = server.reload("[http.headers]\nreferrer_policy = \"same-origin\"\n");
    assert!(
        report.contains("invalidated: "),
        "the reload over the repaired file printed no report: {report}"
    );
    let row = held(&server.ctl("config"));
    assert!(
        row.contains("\"same-origin\""),
        "`nvs ctl config` does not list the reloaded value: {row}"
    );

    for (method, path, control) in [
        ("POST", "/reload", "invalidated:"),
        ("GET", "/config", POLICY),
        ("GET", "/status", "in_flight:"),
    ] {
        let answer = server.send(method, path, &[]);
        assert!(
            !answer.body.contains(control),
            "`{method} {path}` on the HTTP port was answered by the control endpoint: {answer:?}"
        );
    }
}

/// A stand-in OTLP collector on a loopback port of its own. It answers every
/// request with `200` and keeps the request target of each.
struct Collector {
    addr: SocketAddr,
    targets: Arc<Mutex<Vec<String>>>,
}

impl Collector {
    /// Binds a free port and starts answering on a thread of its own.
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port is free");
        let addr = listener
            .local_addr()
            .expect("a bound socket has an address");
        let targets = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&targets);
        thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                if let Some(target) = collected(stream) {
                    kept.lock().expect("no reader panicked").push(target);
                }
            }
        });
        Self { addr, targets }
    }

    /// The URL `[trace] endpoint` or `[metrics] endpoint` names it by.
    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Whether a request to `target` has arrived.
    fn received(&self, target: &str) -> bool {
        self.targets
            .lock()
            .expect("no reader panicked")
            .iter()
            .any(|arrived| arrived == target)
    }
}

/// Reads one request from `stream`, answers `200`, and returns its target.
fn collected(mut stream: TcpStream) -> Option<String> {
    stream.set_read_timeout(Some(BOUND)).ok()?;
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let target = line.split_whitespace().nth(1)?.to_owned();
    let mut length = 0;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).ok()?;
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().ok()?;
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    stream
        .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
        .ok()?;
    Some(target)
}

/// A loopback port nothing listens on at the moment it is asked.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("a loopback port is free")
        .port()
}

/// The body a scrape of `port` answers with, or `None` when nothing answers
/// there.
fn scraped(port: u16) -> Option<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream.set_read_timeout(Some(BOUND)).ok()?;
    stream
        .write_all(b"GET /metrics HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .ok()?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).ok()?;
    Some(String::from_utf8_lossy(&raw).into_owned())
}

/// Checks `done` until it holds or [`BOUND`] runs out, and fails naming `what`
/// and what `server` wrote to standard error.
fn eventually(server: &Server, what: &str, mut done: impl FnMut() -> bool) {
    let started = Instant::now();
    while !done() {
        assert!(
            started.elapsed() <= BOUND,
            "{what} did not happen within {BOUND:?}; the server wrote: {}",
            server.said()
        );
        thread::sleep(POLL);
    }
}

/// `[metrics]` written as `metrics`, and `[trace]` sampling every request and
/// pushing it to `traces`.
fn exporting(metrics: &str, traces: &Collector) -> String {
    format!(
        "[metrics]\n{metrics}\n[trace]\nexporter = \"otlp\"\nsample = 1.0\nendpoint = \"{}\"\n",
        traces.url()
    )
}

/// A Prometheus exporter answering scrapes on `port`.
fn scraped_on(port: u16) -> String {
    format!("exporter = \"prometheus\"\nlisten = \"127.0.0.1:{port}\"\n")
}

/// A server that booted with no metrics exporter starts one when a reload adds
/// it, and counts requests into it. A reload that then moves the scrape port
/// and the trace collector reaches both: the new port answers, the old one
/// closes, and spans go to the new collector. A last reload that switches
/// `[metrics]` to `otlp` closes the scrape port and pushes the series to a
/// collector.
#[test]
fn changed_metrics_and_trace_blocks_rebuild_their_exporters() {
    let first = Collector::start();
    let second = Collector::start();
    let (old_port, new_port) = (free_port(), free_port());
    let server = Server::start("exporters", &exporting("", &first), &[("app.nvs", PLAIN)]);
    server.awaits("/", "the first answer", |answer| answer.body == "ok");
    eventually(&server, "a span pushed to the boot's collector", || {
        server.get("/");
        first.received("/v1/traces")
    });

    server.reload(&exporting(&scraped_on(old_port), &first));
    eventually(&server, "a scrape that counts requests", || {
        server.get("/");
        scraped(old_port).is_some_and(|body| body.contains("nvs_requests_total"))
    });

    let report = server.reload(&exporting(&scraped_on(new_port), &second));
    assert!(
        report.contains("metrics") && report.contains("trace"),
        "the reload did not name `metrics` and `trace` as applied: {report}"
    );
    eventually(&server, "a scrape of the reloaded port", || {
        scraped(new_port).is_some_and(|body| body.contains("nvs_requests_total"))
    });
    eventually(&server, "the boot's scrape port closing", || {
        scraped(old_port).is_none()
    });
    eventually(&server, "a span pushed to the reloaded collector", || {
        server.get("/");
        second.received("/v1/traces")
    });

    server.reload(&exporting(
        &format!("exporter = \"otlp\"\nendpoint = \"{}\"\n", second.url()),
        &second,
    ));
    eventually(&server, "series pushed to the reloaded collector", || {
        second.received("/v1/metrics")
    });
    eventually(&server, "the scrape port closing", || {
        scraped(new_port).is_none()
    });
}

/// The directives a request reads out of the snapshot it started under, each
/// with the value the boot writes and the value a reload writes.
const REQUEST_READ: &[(&str, &str, &str)] = &[
    ("limits.fatal_reserve_memory", "\"1M\"", "\"2M\""),
    ("limits.fatal_reserve_time", "\"50ms\"", "\"60ms\""),
    ("limits.max_script_depth", "64", "32"),
    ("limits.max_decompressed", "\"64M\"", "\"32M\""),
    ("limits.max_decompression_ratio", "1000", "500"),
    ("limits.memory_high_water", "0.5", "0.9"),
    ("limits.disconnect_grace", "\"30s\"", "\"20s\""),
    ("limits.hard.memory", "\"1G\"", "\"2G\""),
    ("log.format", "\"json\"", "\"text\""),
    ("log.handler_reserve_memory", "\"16M\"", "\"8M\""),
    ("log.handler_reserve_time", "\"5s\"", "\"4s\""),
    ("debug.keep_temporary", "false", "true"),
    ("http.client.pool_idle", "16", "8"),
    ("http.client.pool_idle_timeout", "\"30s\"", "\"20s\""),
    ("http.client.socket.send_timeout", "\"30s\"", "\"20s\""),
    ("deferred.max_concurrent", "256", "128"),
    ("deferred.deadline", "\"30s\"", "\"20s\""),
    ("cache.local.max_size", "\"32M\"", "\"16M\""),
    ("cache.process.max_size", "\"32M\"", "\"16M\""),
    ("cache.process.fill_wait", "\"5s\"", "\"4s\""),
    ("log.handler", "\"jobs/one.nvs\"", "\"jobs/two.nvs\""),
    ("log.target", "\"stderr\"", "\"file:app.log\""),
    ("debug.inline", "false", "true"),
    ("errors.deprecated", "\"ignore\"", "\"throw\""),
    (
        "http.client.proxy.url",
        "\"http://127.0.0.1:3128\"",
        "\"http://127.0.0.1:3129\"",
    ),
    ("http.client.proxy.resolve", "\"local\"", "\"proxy\""),
];

/// `nvs.toml` writing every [`REQUEST_READ`] key, with its reloaded value when
/// `reloaded` is set. Keys of one block are written under one header.
fn request_read(reloaded: bool) -> String {
    let mut blocks: Vec<(&str, String)> = Vec::new();
    for (key, boot, reload) in REQUEST_READ {
        let (block, name) = key.rsplit_once('.').expect("every key is in a block");
        let value = if reloaded { reload } else { boot };
        let line = format!("{name} = {value}\n");
        match blocks.iter_mut().find(|(written, _)| *written == block) {
            Some((_, body)) => body.push_str(&line),
            None => blocks.push((block, line)),
        }
    }
    blocks
        .iter()
        .map(|(block, body)| format!("[{block}]\n{body}\n"))
        .collect()
}

/// A program that prints each [`REQUEST_READ`] key and the value
/// `Core\Config::get` returns for it, one per line.
fn reading_back() -> String {
    let lines: String = REQUEST_READ
        .iter()
        .map(|(key, _, _)| {
            format!("echo \"{key}=\", Core\\Config::get(\"{key}\") ?? \"none\", \"\\n\";\n")
        })
        .collect();
    format!("<?nvs\n{lines}")
}

/// Every directive a request reads out of its own snapshot takes the
/// reloaded value from the next request. A request reads each of them through
/// the context it started under, so the value `Core\Config::get` returns is the
/// value its reader sees. Every line changes after the reload, and none of them
/// is missing.
#[test]
fn every_request_read_directive_takes_the_reloaded_value_in_the_next_request() {
    let server = Server::start(
        "request-read",
        &request_read(false),
        &[("app.nvs", &reading_back())],
    );
    let boot = server.awaits("/", "the boot's values", |answer| {
        answer.status == 200 && answer.body.lines().count() == REQUEST_READ.len()
    });
    for line in boot.body.lines() {
        assert!(!line.ends_with("=none"), "the boot did not set `{line}`");
    }

    server.reload(&request_read(true));
    let reloaded = server.awaits("/", "every reloaded value", |answer| {
        answer.status == 200
            && answer.body.lines().count() == REQUEST_READ.len()
            && answer
                .body
                .lines()
                .zip(boot.body.lines())
                .all(|(now, then)| now != then)
    });
    for line in reloaded.body.lines() {
        assert!(!line.ends_with("=none"), "the reload unset `{line}`");
    }
}

/// A `[mode]` block with this default and this ceiling.
fn mode(default: &str, ceiling: &str) -> String {
    format!("[mode]\ndefault = \"{default}\"\nceiling = \"{ceiling}\"\n")
}

/// A program that prints the mode a request starts in and the ceiling it may
/// select.
const MODES: &str = "<?nvs\necho Core\\Config::get(\"mode.default\"), \" \", \
                     Core\\Config::get(\"mode.ceiling\");\n";

/// `[mode]` reloads: a server that booted in `development` starts the next
/// request in `production` once a reload writes it, under the reloaded ceiling.
#[test]
fn a_changed_mode_block_reaches_the_next_request() {
    let server = Server::start(
        "mode",
        &mode("development", "development"),
        &[("app.nvs", MODES)],
    );
    server.awaits("/", "the boot's mode", |answer| {
        answer.status == 200 && answer.body == "development development"
    });

    server.reload(&mode("production", "production"));
    server.awaits("/", "the reloaded mode", |answer| {
        answer.status == 200 && answer.body == "production production"
    });
}

/// An `[[include]]` entry naming `more.toml`.
const INCLUDING: &str = "[[include]]\npath = \"more.toml\"\n";

/// `more.toml` setting `[limits] max_script_depth` to `depth`.
fn included(depth: u32) -> String {
    format!("[limits]\nmax_script_depth = {depth}\n")
}

/// `[[include]]` is read again on every reload: a key that changes in the
/// included file, and nowhere else, reaches the next request.
#[test]
fn a_changed_included_file_reaches_the_next_request() {
    let server = Server::start(
        "include",
        INCLUDING,
        &[
            ("more.toml", &included(64)),
            (
                "app.nvs",
                "<?nvs\necho Core\\Config::get(\"limits.max_script_depth\");\n",
            ),
        ],
    );
    server.awaits("/", "the included file's value", |answer| {
        answer.status == 200 && answer.body == "64"
    });

    write_file(&server.dir.join("more.toml"), &included(32));
    server.reload(INCLUDING);
    server.awaits("/", "the included file's reloaded value", |answer| {
        answer.status == 200 && answer.body == "32"
    });
}

/// A grant to read the files under `data/`.
const READ_GRANT: &str = "[capabilities.fs]\nread = [\"data\"]\n";

/// `[capabilities]` reloads: a call a grant allowed fails in the next request
/// once a reload removes the grant.
#[test]
fn a_grant_a_reload_removes_fails_the_next_call() {
    let server = Server::start(
        "capabilities",
        READ_GRANT,
        &[
            ("data/note.txt", "hello"),
            (
                "app.nvs",
                "<?nvs\necho Core\\IO::read(\"data/note.txt\");\n",
            ),
        ],
    );
    server.awaits("/", "a read under the grant", |answer| {
        answer.status == 200 && answer.body == "hello"
    });

    server.reload("");
    let refused = server.awaits("/", "the read failing without the grant", |answer| {
        answer.status != 200
    });
    assert!(
        !refused.body.contains("hello"),
        "a read with no grant returned the file: {refused:?}"
    );
}

/// An `[opcache]` block that writes compiled programs to `cache/`, with the
/// `[[extension]]` entry `extension` after it.
fn file_cache(extension: &str) -> String {
    format!("[opcache]\nrevalidate_freq = \"100ms\"\nfile_cache_dir = \"cache\"\n\n{extension}")
}

/// How many compiled programs `dir` holds, in its subdirectories.
fn artifacts(dir: &Path) -> usize {
    let Ok(shards) = std::fs::read_dir(dir) else {
        return 0;
    };
    shards
        .filter_map(Result::ok)
        .filter_map(|shard| std::fs::read_dir(shard.path()).ok())
        .flatten()
        .filter_map(Result::ok)
        .filter(|file| file.path().extension().is_some_and(|ext| ext == "nvsc"))
        .count()
}

/// `[[extension]]` reloads into both caches of compiled programs. A reload
/// that adds an extension writes the compiled extension to the file cache,
/// then compiles the program again, and the file cache gets a second program,
/// written for the new set of extensions: three entries in all. The case
/// writes a file into `cache/`, so the directory exists inside the private one.
#[test]
fn a_changed_extension_set_compiles_every_program_again() {
    let shelf = Shelf::new("extension");
    let geo = extension("Shop\\Geo");
    let server = Server::start(
        "extension",
        &file_cache(""),
        &[("app.nvs", PLAIN), ("cache/.keep", "")],
    );
    let cache = server.dir.join("cache");
    server.awaits("/", "the first answer", |answer| answer.body == "ok");
    assert_eq!(
        artifacts(&cache),
        1,
        "the boot did not write one program to the file cache; the server wrote: {}",
        server.said()
    );

    let report = server.reload(&file_cache(&shelf.put("one.nvsx", &geo, &geo)));
    assert!(
        report.contains("applied: extension\n"),
        "the reload did not name `extension` as applied: {report}"
    );
    server.awaits("/", "an answer after the reload", |answer| {
        answer.body == "ok" && artifacts(&cache) == 3
    });
}

/// An `[opcache]` block that writes compiled programs to `dir`.
fn cache_dir(dir: &str) -> String {
    format!("[opcache]\nrevalidate_freq = \"100ms\"\nsettle = \"0s\"\nfile_cache_dir = \"{dir}\"\n")
}

/// Lets every account write to `dir`. `[opcache] file_cache_dir` is not used
/// there.
///
/// # Panics
///
/// When the access list or the mode cannot be changed.
fn open_to_everyone(dir: &Path) {
    #[cfg(windows)]
    {
        let ran = nvs_repo::spawn("icacls", &[])
            .arg(dir)
            .args(["/grant", "*S-1-1-0:(OI)(CI)F"])
            .output()
            .expect("`icacls` starts");
        assert!(
            ran.status.success(),
            "`icacls` could not open `{}`: {}",
            dir.display(),
            String::from_utf8_lossy(&ran.stdout)
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o777))
            .unwrap_or_else(|error| panic!("`{}` could not be opened: {error}", dir.display()));
    }
}

/// `[opcache] file_cache_dir` reloads. After a reload that moves it, the next
/// compile writes its program to the new directory, and nothing more is
/// written to the old one. A directory that every account may write to is
/// not used: the reload logs the key and names it as not applied, and the
/// directory in force stays.
#[test]
fn a_changed_file_cache_dir_applies_to_the_next_compile() {
    let server = Server::start(
        "cache-dir",
        &cache_dir("one"),
        &[
            ("app.nvs", &saying("one")),
            ("one/.keep", ""),
            ("two/.keep", ""),
            ("open/.keep", ""),
        ],
    );
    let dir = |name: &str| server.dir.join(name);
    server.awaits("/", "the boot's program", |answer| {
        answer.status == 200 && answer.body == "one"
    });
    assert_eq!(
        artifacts(&dir("one")),
        1,
        "the boot did not write one program to its directory; the server wrote: {}",
        server.said()
    );

    let report = server.reload(&cache_dir("two"));
    assert!(
        report.contains("applied: opcache.file_cache_dir\n"),
        "the reload did not name `opcache.file_cache_dir` as applied: {report}"
    );
    write_file(&dir("app.nvs"), &saying("two"));
    server.awaits("/", "the edit, written to the new directory", |answer| {
        answer.body == "two" && artifacts(&dir("two")) == 1
    });
    assert_eq!(
        artifacts(&dir("one")),
        1,
        "a compile after the reload wrote to the old directory"
    );

    open_to_everyone(&dir("open"));
    let report = server.reload(&cache_dir("open"));
    assert!(
        report.contains("ignored: opcache.file_cache_dir\n"),
        "the reload did not name `opcache.file_cache_dir` as not applied: {report}"
    );
    let said = server.logs("configuration key not applied");
    assert!(
        said.contains("open"),
        "the record does not name the directory that was written: {said}"
    );
    write_file(&dir("app.nvs"), &saying("three"));
    server.awaits(
        "/",
        "the edit, written to the directory in force",
        |answer| answer.body == "three" && artifacts(&dir("two")) == 2,
    );
    assert_eq!(
        artifacts(&dir("open")),
        0,
        "a directory every account may write to was used"
    );
}

/// A loopback HTTPS peer whose certificate is self-signed for `127.0.0.1`. It
/// answers every request with `ok`, and [`Peer::pem`] is the certificate a
/// `roots` entry names to trust it.
struct Peer {
    addr: SocketAddr,
    pem: String,
}

impl Peer {
    /// Issues the certificate, binds a free port and answers every connection
    /// on a thread of its own.
    fn start() -> Self {
        let issued = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()])
            .expect("the certificate is issued");
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
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port is free");
        let addr = listener
            .local_addr()
            .expect("a bound socket has an address");
        thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                let config = Arc::clone(&config);
                thread::spawn(move || served(stream, config));
            }
        });
        Self {
            addr,
            pem: issued.cert.pem(),
        }
    }
}

/// Reads one request head from `stream` over TLS and answers `ok`. A client
/// that refuses the certificate ends the handshake, and nothing is answered.
fn served(stream: TcpStream, config: Arc<rustls::ServerConfig>) {
    let Ok(conn) = rustls::ServerConnection::new(config) else {
        return;
    };
    if stream.set_read_timeout(Some(BOUND)).is_err() {
        return;
    }
    let mut tls = rustls::StreamOwned::new(conn, stream);
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match tls.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => return,
        }
    }
    drop(tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"));
    tls.conn.send_close_notify();
    drop(tls.flush());
}

/// A program that calls `peer` and prints its answer, or `refused` when the
/// call throws.
fn calling(peer: &Peer) -> String {
    format!(
        "<?nvs\ntry {{\n    echo Core\\Http\\Client::get(\"https://{}/\")->text();\n}} catch \
         (Throwable $refused) {{\n    echo \"refused\";\n}}\n",
        peer.addr
    )
}

/// `[http.client.tls]` trusting the one file `roots` names, or the compiled-in
/// set when it is `None`, beside a grant to call the loopback address.
fn anchors(roots: Option<&Path>) -> String {
    let block = roots.map_or_else(String::new, |path| {
        format!("[http.client.tls]\nroots = ['{}']\n\n", path.display())
    });
    format!("{block}[capabilities.net]\nconnect = [\"127.0.0.1\"]\ninternal = [\"127.0.0.1\"]\n")
}

/// `[http.client.tls]` reloads. The compiled-in anchors refuse the peer's
/// certificate. After a reload that trusts it, the next call is answered. A
/// file that holds no certificate is not used: the reload logs the key and
/// names it as not applied, and the anchors in force stay.
#[test]
fn changed_anchors_judge_the_next_outbound_connection() {
    let peer = Peer::start();
    let server = Server::start(
        "tls-client",
        &anchors(None),
        &[
            ("app.nvs", &calling(&peer)),
            ("peer.pem", &peer.pem),
            ("empty.pem", "not a certificate\n"),
        ],
    );
    server.awaits(
        "/",
        "the call refused by the compiled-in anchors",
        |answer| answer.status == 200 && answer.body == "refused",
    );

    let report = server.reload(&anchors(Some(&server.dir.join("peer.pem"))));
    assert!(
        report.contains("applied: http.client.tls.roots\n"),
        "the reload did not name `http.client.tls.roots` as applied: {report}"
    );
    server.awaits("/", "the call answered under the new anchors", |answer| {
        answer.status == 200 && answer.body == "ok"
    });

    let report = server.reload(&anchors(Some(&server.dir.join("empty.pem"))));
    assert!(
        report.contains("ignored: http.client.tls\n"),
        "the reload did not name `http.client.tls` as not applied: {report}"
    );
    let said = server.logs("configuration key not applied");
    assert!(
        said.contains("empty.pem"),
        "the record does not name the file that holds no certificate: {said}"
    );
    server.awaits(
        "/",
        "the call answered under the anchors in force",
        |answer| answer.status == 200 && answer.body == "ok",
    );
}

/// A program with one `POST` route, which the door checks for a CSRF token.
const FORM: &str = r#"<?nvs
class Form {
    #[Core\Route(path: "/form", method: Core\Http\Method::Post, name: "Form::send")]
    #[Core\Access(allow: Core\Audience::Public)]
    public function send(): string { return "sent"; }
}

echo "ok";
"#;

/// A CSRF key of 32 octets, each `octet`, written as URL-safe base64 with no
/// padding.
fn csrf_text(octet: u8) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let bytes = [octet; nvs_runtime::csrf::KEY_LEN];
    let mut text = String::new();
    for chunk in bytes.chunks(3) {
        let word = chunk.iter().enumerate().fold(0_u32, |word, (at, byte)| {
            word | u32::from(*byte) << (16 - 8 * at)
        });
        for at in 0..=chunk.len() {
            let index = (word >> (18 - 6 * at)) & 63;
            text.push(char::from(ALPHABET[index as usize]));
        }
    }
    text
}

/// A token for a request with no session cookie, signed with the key of 32
/// octets, each `octet`.
fn csrf_token(octet: u8) -> String {
    nvs_runtime::csrf::Key::new(&[octet; nvs_runtime::csrf::KEY_LEN])
        .expect("32 octets are a key")
        .issue(&[9; nvs_runtime::csrf::NONCE_LEN], "", "live_config")
        .expect("an empty session seals")
}

/// `[http] csrf_key` and `csrf_key_file` reload at the door: a token signed
/// with the key a reload replaced is refused in the next request, and one
/// signed with the new key is accepted. A reload reads the key file again.
#[test]
fn a_rotated_csrf_key_refuses_the_old_token_at_the_door() {
    let post = |server: &Server, octet: u8, what: &str, status: u16| {
        let token = csrf_token(octet);
        let started = Instant::now();
        loop {
            let answer = server.send("POST", "/form", &[("X-CSRF-Token", &token)]);
            if answer.status == status {
                return;
            }
            assert!(
                started.elapsed() <= BOUND,
                "{what} did not happen within {BOUND:?}; the last answer was {answer:?}, and \
                 the server wrote: {}",
                server.said()
            );
            thread::sleep(POLL);
        }
    };
    let server = Server::start(
        "csrf",
        &format!("[http]\ncsrf_key = \"{}\"\n", csrf_text(1)),
        &[("app.nvs", FORM)],
    );
    post(
        &server,
        1,
        "a token signed with the boot's key passing",
        200,
    );
    let bare = server.send("POST", "/form", &[]);
    assert_eq!(bare.status, 403, "a post with no token passed: {bare:?}");

    let file = "[http]\ncsrf_key_file = \"csrf.key\"\n";
    write_file(&server.dir.join("csrf.key"), &csrf_text(2));
    server.reload(file);
    post(&server, 1, "the boot's token refused after the reload", 403);
    post(
        &server,
        2,
        "a token signed with the file's key passing",
        200,
    );

    write_file(&server.dir.join("csrf.key"), &csrf_text(3));
    server.reload(file);
    post(
        &server,
        2,
        "the old file's token refused after the reload",
        403,
    );
    post(
        &server,
        3,
        "a token signed with the rewritten file's key passing",
        200,
    );
}

/// How many times `text` appears in `said`.
fn times(said: &str, text: &str) -> usize {
    said.matches(text).count()
}

/// The server checks its own configuration files: a saved `nvs.toml` reaches
/// the next response with no `nvs ctl reload`, and the record the check writes
/// names the file that changed.
#[test]
fn a_saved_configuration_file_is_applied_without_a_reload() {
    let server = Server::start(
        "saved",
        "[http.headers]\nreferrer_policy = \"no-referrer\"\n",
        &[("app.nvs", PLAIN)],
    );
    server.awaits("/", "the boot's `Referrer-Policy`", |answer| {
        answer.status == 200 && answer.header("referrer-policy") == Some("no-referrer")
    });

    server.save("[http.headers]\nreferrer_policy = \"same-origin\"\n");
    server.awaits("/", "the saved `Referrer-Policy`", |answer| {
        answer.status == 200 && answer.header("referrer-policy") == Some("same-origin")
    });
}

/// A file cut off in the middle of a line does not parse. The check logs it
/// once, with the file and the line, and the running configuration keeps
/// serving. The complete file is then applied as usual.
#[test]
fn a_half_written_configuration_file_is_logged_and_the_running_one_kept() {
    const REFUSED: &str = "configuration reload refused";
    let server = Server::start(
        "half",
        "[http.headers]\nreferrer_policy = \"no-referrer\"\n",
        &[("app.nvs", PLAIN)],
    );
    server.awaits("/", "the boot's `Referrer-Policy`", |answer| {
        answer.status == 200 && answer.header("referrer-policy") == Some("no-referrer")
    });

    server.save("[http.headers]\nreferrer_policy = \"same-ori");
    let said = server.logs(REFUSED);
    assert!(
        said.contains("nvs.toml"),
        "the refusal does not name the file: {said}"
    );
    let kept = server.get("/");
    assert_eq!(
        kept.header("referrer-policy"),
        Some("no-referrer"),
        "a file that does not parse changed the running configuration: {kept:?}"
    );
    // Three more checks of the same file log nothing more.
    thread::sleep(Duration::from_secs(6));
    assert_eq!(
        times(&server.said(), REFUSED),
        1,
        "one broken file was logged more than once: {}",
        server.said()
    );

    server.save("[http.headers]\nreferrer_policy = \"same-origin\"\n");
    server.awaits("/", "the completed file's `Referrer-Policy`", |answer| {
        answer.status == 200 && answer.header("referrer-policy") == Some("same-origin")
    });
}

/// A changed restart key is logged once, with the value in force and the
/// value written, and `nvs ctl status` lists it until the file is changed
/// back. A later save that changes another key does not log it again.
// covers: tools:server/nvs-ctl
#[test]
fn a_changed_restart_key_is_logged_as_pending_and_listed_by_ctl_status() {
    const PENDING: &str = "configuration restart pending";
    let server = Server::start("pending", "", &[("app.nvs", PLAIN)]);
    server.awaits("/", "the boot's answer", |answer| answer.status == 200);
    let status = server.ctl("status");
    assert!(
        !status.contains("restart pending"),
        "a server nobody changed has a pending restart: {status}"
    );

    server.save("[server]\nworkers = 1\n");
    let said = server.logs(PENDING);
    assert!(
        said.contains("not written"),
        "the record does not give the running value: {said}"
    );
    let status = server.ctl("status");
    let line = status
        .lines()
        .find(|line| line.starts_with("restart pending: server.workers"))
        .unwrap_or_else(|| panic!("`nvs ctl status` does not list the key: {status}"));
    assert!(
        line.contains("running not written") && line.contains("written 1)"),
        "the pending line does not give both values: {line}"
    );

    server.save("[server]\nworkers = 1\n\n[http.headers]\nreferrer_policy = \"same-origin\"\n");
    server.awaits("/", "the saved `Referrer-Policy`", |answer| {
        answer.header("referrer-policy") == Some("same-origin")
    });
    assert_eq!(
        times(&server.said(), PENDING),
        1,
        "one written value was logged more than once: {}",
        server.said()
    );

    server.save("[http.headers]\nreferrer_policy = \"same-origin\"\n");
    let started = Instant::now();
    loop {
        let status = server.ctl("status");
        if !status.contains("restart pending") {
            break;
        }
        assert!(
            started.elapsed() < BOUND,
            "a key changed back is still pending: {status}"
        );
        thread::sleep(POLL);
    }
}

/// A program that makes a temporary directory and prints its path.
const TEMPORARY: &str = "<?nvs\necho Core\\IO::temporaryDir();\n";

/// `[io] temp_root` at `root`, with a write grant over both `roots`.
fn temp_root(root: &Path, roots: &[&Path]) -> String {
    let granted: Vec<String> = roots
        .iter()
        .map(|root| format!("'{}'", root.display()))
        .collect();
    format!(
        "[io]\ntemp_root = '{}'\n\n[capabilities.fs]\nwrite = [{}]\n",
        root.display(),
        granted.join(", ")
    )
}

/// A reload that moves `[io] temp_root` applies to the next temporary
/// directory: it is made under the new root.
#[test]
fn a_changed_temp_root_applies_to_the_next_temporary_directory() {
    let server = Server::start("temp-root", "", &[("app.nvs", TEMPORARY)]);
    let one = server.dir.join("scratch-one");
    let two = server.dir.join("scratch-two");
    let roots = [one.as_path(), two.as_path()];

    server.reload(&temp_root(&one, &roots));
    server.awaits(
        "/",
        "a temporary directory under the first root",
        |answer| answer.status == 200 && Path::new(answer.body.trim()).starts_with(&one),
    );

    server.reload(&temp_root(&two, &roots));
    server.awaits(
        "/",
        "a temporary directory under the second root",
        |answer| answer.status == 200 && Path::new(answer.body.trim()).starts_with(&two),
    );
}

/// The three counts on `nvs ctl status`'s `config_check:` line: the passes the
/// configuration check has taken, the `stat` calls they made, and the paths
/// each pass takes.
fn check_counts(server: &Server) -> (u64, u64, u64) {
    let status = server.ctl("status");
    let line = status
        .lines()
        .find_map(|line| line.strip_prefix("config_check: "))
        .unwrap_or_else(|| panic!("`nvs ctl status` has no `config_check:` line: {status}"));
    let counts: Vec<u64> = line
        .split(", ")
        .map(|part| {
            part.split(' ')
                .next()
                .and_then(|count| count.parse().ok())
                .unwrap_or_else(|| panic!("`{part}` in `{line}` is not a count"))
        })
        .collect();
    match counts[..] {
        [passes, stats, paths] => (passes, stats, paths),
        _ => panic!("`{line}` is not three counts"),
    }
}

/// The configuration check runs on its own thread, and a request reads only
/// the snapshot in force. Many requests between two readings of the counts
/// add no pass and no `stat` call beyond what the time between them allows.
#[test]
fn watching_the_configuration_costs_no_request_a_filesystem_call() {
    const REQUESTS: u64 = 200;
    let server = Server::start("watched", "", &[("app.nvs", PLAIN)]);
    server.awaits("/", "the boot's answer", |answer| answer.status == 200);
    // One pass before the thread starts and one of its own: the check runs.
    eventually(&server, "a pass of the configuration check", || {
        check_counts(&server).0 >= 2
    });
    let (passes, stats, paths) = check_counts(&server);
    assert!(paths >= 1, "the check takes no path, so it proves nothing");

    let started = Instant::now();
    for _ in 0..REQUESTS {
        let answer = server.get("/");
        assert_eq!(answer.status, 200, "a request failed: {answer:?}");
    }
    let took = started.elapsed();
    let (passes_after, stats_after, _) = check_counts(&server);

    // One pass every two seconds, and one more for a pass that was under way
    // when the counts were read.
    let allowed = took.as_secs() / 2 + 2;
    let passed = passes_after - passes;
    assert!(
        passed <= allowed,
        "{REQUESTS} requests in {took:?} came with {passed} passes of the check, and the time \
         allows {allowed}"
    );
    let statted = stats_after - stats;
    assert!(
        statted <= (passed + 1) * paths,
        "{REQUESTS} requests came with {statted} `stat` calls, and {passed} passes over {paths} \
         paths make at most {}",
        (passed + 1) * paths
    );
}

/// The waits are read when a connection is accepted. After a reload shortens
/// `[server] header_timeout`, a new connection that sends nothing is closed
/// within the new wait. A connection accepted before the reload keeps the
/// wait it started with, so a head it began before the reload and finishes
/// slowly is still answered.
#[test]
fn a_changed_server_timeout_applies_to_the_next_connection() {
    let server = Server::start(
        "timeout",
        "[server]\nheader_timeout = \"60s\"\n",
        &[("app.nvs", PLAIN)],
    );
    server.awaits("/", "the boot's answer", |answer| answer.status == 200);
    let mut before = kept_alive(&server);
    // The head is begun before the reload, so the reload's drain finds a
    // request in progress and lets it finish.
    write!(before, "GET / HTTP/1.1\r\n").expect("the first line is written");

    server.reload("[server]\nheader_timeout = \"1s\"\n");

    let mut after = TcpStream::connect(server.addr).expect("the server accepts");
    after
        .set_read_timeout(Some(BOUND))
        .expect("a read timeout can be set");
    let started = Instant::now();
    let _ = after.read_to_end(&mut Vec::new());
    let held = started.elapsed();
    assert!(
        held < Duration::from_secs(20),
        "a connection accepted after the reload was held {held:?} with no head; the server \
         wrote: {}",
        server.said()
    );

    // At least three seconds between the head's first line and the rest of
    // it: past the reloaded wait, and well inside the one this connection
    // started with and the drain period.
    thread::sleep(Duration::from_secs(3));
    write!(before, "Host: localhost\r\nConnection: close\r\n\r\n")
        .expect("the rest of the head is written");
    let mut answer = String::new();
    let _ = before.read_to_string(&mut answer);
    assert!(
        answer.starts_with("HTTP/1.1 200"),
        "a connection accepted before the reload lost the wait it started with: {answer:?}; the \
         server wrote: {}",
        server.said()
    );
}

/// A component exporting `shop:geo/api` with one function, `distance-km`, its
/// `error` imported from `nvs:ext/types` — `nvs-ext`'s own load test guest.
fn guest() -> Vec<u8> {
    let text = r#"(component
  (import "nvs:ext/types@1.0.0" (instance $types
    (type $e (variant (case "invalid" string) (case "parse" string) (case "runtime" string)))
    (export "error" (type (eq $e)))))
  (alias export $types "error" (type $error))
  (core module $m
    (memory (export "memory") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 8)
    (func (export "distance-km") (param i32 i32 i32) (result i32) i32.const 16))
  (core instance $i (instantiate $m))
  (alias core export $i "memory" (core memory $mem))
  (alias core export $i "realloc" (core func $realloc))
  (func $f (param "from" string) (param "round" bool) (result (result f64 (error $error)))
    (canon lift (core func $i "distance-km") (memory $mem) (realloc $realloc)))
  (instance $api (export "distance-km" (func $f)))
  (export "shop:geo/api" (instance $api)))"#;
    wat::parse_str(text).expect("the test component compiles")
}

/// A `.nvsx` declaring `class` over [`guest`], with its manifest and no source.
fn extension(class: &str) -> Vec<u8> {
    let class = class.replace('\\', "\\\\");
    let manifest = format!(
        r#"{{"manifest": 1, "world": "1.0.0", "class": "{class}", "interface": "shop:geo/api", "methods": [{{"name": "distanceKm", "params": [{{"name": "from", "type": "string"}}, {{"name": "round", "type": "bool"}}], "returns": "float"}}]}}"#
    );
    nvs_ext::pack::append_section(guest(), nvs_ext::section::MANIFEST, manifest.as_bytes())
}

/// A directory of `.nvsx` files beside a case's own, removed when dropped.
/// [`Server::start`] empties the case's directory, so the files a boot loads
/// are written here first and named by their absolute paths.
struct Shelf {
    dir: PathBuf,
}

impl Shelf {
    fn new(case: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("live-config-shelf-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the shelf is created");
        Self { dir }
    }

    /// Writes `bytes` as `name` on the shelf and returns its `[[extension]]`
    /// entry, pinned to `pinned`.
    fn put(&self, name: &str, bytes: &[u8], pinned: &[u8]) -> String {
        let path = self.dir.join(name);
        std::fs::write(&path, bytes)
            .unwrap_or_else(|error| panic!("`{}` could not be written: {error}", path.display()));
        format!(
            "[[extension]]\npath = '{}'\nsha256 = \"{}\"\n",
            path.display(),
            nvs_ext::load::pin(pinned)
        )
    }
}

impl Drop for Shelf {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// `[http.headers] referrer_policy`, which a case changes beside its
/// extensions to see whether a reload published anything at all.
fn policy(value: &str) -> String {
    format!("[http.headers]\nreferrer_policy = \"{value}\"\n")
}

/// The referrer policy the running configuration holds, as `nvs ctl config`
/// lists it.
fn running_policy(server: &Server) -> String {
    let listing = server.ctl("config");
    listing
        .lines()
        .find(|line| line.starts_with("http.headers.referrer_policy"))
        .unwrap_or_else(|| panic!("`nvs ctl config` lists no referrer policy: {listing}"))
        .to_owned()
}

/// Runs `nvs ctl reload` and returns its standard error, asserting it failed.
fn refused_reload(server: &Server) -> String {
    let ran = server.ctl_output(&server.socket, "reload");
    let said = String::from_utf8_lossy(&ran.stderr).into_owned();
    assert!(
        !ran.status.success() && ran.stdout.is_empty(),
        "the reload was reported as done: {}; the server wrote: {}",
        String::from_utf8_lossy(&ran.stdout),
        server.said()
    );
    said
}

/// The `invalidated` count a reload's report gives.
fn invalidated(report: &str) -> u64 {
    report
        .lines()
        .find_map(|line| line.strip_prefix("invalidated: "))
        .and_then(|count| count.trim().parse().ok())
        .unwrap_or_else(|| panic!("the report gives no `invalidated` count: {report}"))
}

/// A boot whose one `[[extension]]` file differs from its pin does not start:
/// it exits with `E0652`, naming the file and the entry's line, and never
/// listens.
#[test]
fn a_boot_with_an_extension_entry_that_does_not_load_refuses_to_start() {
    let shelf = Shelf::new("boot-refused");
    let geo = extension("Shop\\Geo");
    let entry = shelf.put("geo.nvsx", &geo, b"another file");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("live-config-boot-refused-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_file(&dir.join("app.nvs"), PLAIN);
    write_file(
        &dir.join("nvs.toml"),
        &controlled(&endpoint(&dir, "boot-refused"), &entry),
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the `nvs` binary this test was built beside starts");
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("the child can be waited on") {
            break status;
        }
        if started.elapsed() > BOOT {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_dir_all(&dir);
            panic!("`nvs serve` with an extension that does not load kept running");
        }
        thread::sleep(POLL);
    };
    let mut said = String::new();
    let mut out = String::new();
    let _ = child
        .stderr
        .take()
        .expect("standard error is piped")
        .read_to_string(&mut said);
    let _ = child
        .stdout
        .take()
        .expect("standard output is piped")
        .read_to_string(&mut out);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!status.success(), "the boot succeeded: {said}");
    assert!(
        !out.contains("listening on"),
        "the boot listened before it refused: {out}"
    );
    assert!(
        said.contains("E0652") && said.contains("geo.nvsx") && said.contains("nvs.toml:"),
        "the refusal does not name the code, the file and the entry's line: {said}"
    );
}

/// A file rewritten under its pin is refused at the next reload, even though
/// its entry did not change: nothing in that reload is published, and the
/// running server keeps its configuration. Once the file is back, the same
/// reload goes through.
#[test]
fn a_reload_whose_extension_pin_does_not_match_is_refused_whole_and_the_previous_set_stays_live() {
    let shelf = Shelf::new("pin-refused");
    let geo = extension("Shop\\Geo");
    let entry = shelf.put("geo.nvsx", &geo, &geo);
    let server = Server::start(
        "pin-refused",
        &format!("{}\n{entry}", policy("no-referrer")),
        &[("app.nvs", PLAIN)],
    );
    server.awaits("/", "the boot's answer", |answer| answer.status == 200);

    shelf.put("geo.nvsx", &extension("Shop\\Blog"), &geo);
    server.save(&format!("{}\n{entry}", policy("same-origin")));
    let said = refused_reload(&server);
    assert!(
        said.contains("E0652") && said.contains("geo.nvsx") && said.contains("sha256"),
        "the refusal does not name the code, the file and its digest: {said}"
    );
    assert!(
        said.contains("note: the running configuration is unchanged"),
        "the refusal does not say what the server runs with now: {said}"
    );
    assert!(
        running_policy(&server).contains("\"no-referrer\""),
        "a refused reload changed the running configuration"
    );
    let answer = server.get("/");
    assert_eq!(answer.status, 200, "{answer:?}");

    shelf.put("geo.nvsx", &geo, &geo);
    server.ctl("reload");
    assert!(
        running_policy(&server).contains("\"same-origin\""),
        "the reload over the restored file was not published"
    );
}

/// One entry that does not load among two that do refuses the whole reload,
/// naming that entry; the two that load are not published either.
#[test]
fn a_reload_with_one_bad_extension_entry_among_good_ones_is_refused_whole() {
    let shelf = Shelf::new("one-bad");
    let geo = extension("Shop\\Geo");
    let blog = extension("Shop\\Blog");
    let broken = b"not a component".to_vec();
    let first = shelf.put("geo.nvsx", &geo, &geo);
    let server = Server::start(
        "one-bad",
        &format!("{}\n{first}", policy("no-referrer")),
        &[("app.nvs", PLAIN)],
    );
    server.awaits("/", "the boot's answer", |answer| answer.status == 200);

    let second = shelf.put("blog.nvsx", &blog, &blog);
    let third = shelf.put("broken.nvsx", &broken, &broken);
    server.save(&format!(
        "{}\n{first}\n{second}\n{third}",
        policy("same-origin")
    ));
    let said = refused_reload(&server);
    assert!(
        said.contains("E0652") && said.contains("broken.nvsx") && said.contains("nvs.toml:"),
        "the refusal does not name the code, the bad entry and its line: {said}"
    );
    assert!(
        !said.contains("blog.nvsx"),
        "the refusal names an entry that loads: {said}"
    );
    assert!(
        running_policy(&server).contains("\"no-referrer\""),
        "a refused reload changed the running configuration"
    );
    let answer = server.get("/");
    assert_eq!(answer.status, 200, "{answer:?}");
}

/// A reload that changes the extension set rekeys every compiled unit, and
/// one that changes something else and keeps the set rekeys none.
#[test]
fn a_reload_that_changes_the_extension_set_rekeys_the_unit_cache() {
    let shelf = Shelf::new("rekey");
    let geo = extension("Shop\\Geo");
    let blog = extension("Shop\\Blog");
    let first = shelf.put("geo.nvsx", &geo, &geo);
    let server = Server::start(
        "rekey",
        &format!("{}\n{first}", policy("no-referrer")),
        &[("app.nvs", PLAIN)],
    );
    server.awaits("/", "the boot's answer", |answer| answer.status == 200);

    let second = shelf.put("blog.nvsx", &blog, &blog);
    let report = server.reload(&format!("{}\n{first}\n{second}", policy("no-referrer")));
    assert!(
        invalidated(&report) > 0,
        "a reload that added an extension kept every compiled unit: {report}"
    );
    server.awaits("/", "the answer after the reload", |answer| {
        answer.status == 200
    });

    let report = server.reload(&format!("{}\n{first}\n{second}", policy("same-origin")));
    assert_eq!(
        invalidated(&report),
        0,
        "a reload that kept the extension set dropped compiled units: {report}"
    );
}
