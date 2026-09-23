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
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("live-config-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
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
        let mut stream = TcpStream::connect(self.addr)
            .unwrap_or_else(|error| panic!("{} does not answer: {error}", self.addr));
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        let extra: String = headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}\r\n"))
            .collect();
        write!(
            stream,
            "GET {path} HTTP/1.0\r\nHost: localhost\r\n{extra}\r\n"
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

    /// Runs `nvs ctl <request>` against this server and returns what it
    /// printed.
    ///
    /// # Panics
    ///
    /// When the request fails; the message carries what both processes wrote
    /// to standard error.
    fn ctl(&self, request: &str) -> String {
        let ran = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("ctl")
            .arg("--socket")
            .arg(&self.socket)
            .arg(request)
            .current_dir(&self.dir)
            .stdin(Stdio::null())
            .output()
            .expect("the `nvs` binary this test was built beside starts");
        assert!(
            ran.status.success(),
            "`nvs ctl {request}` failed: {}\nand the server wrote: {}",
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

/// `nvs.toml`: [`PRODUCTION`], [`QUICK_CHECKS`] where `config` has no
/// `[opcache]` block, `[control] socket` naming `socket`, and then `config`.
fn controlled(socket: &Path, config: &str) -> String {
    let checks = if config.contains("[opcache]") {
        ""
    } else {
        QUICK_CHECKS
    };
    format!(
        "{PRODUCTION}\n{checks}\n[control]\nsocket = '{}'\n\n{config}",
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
/// answered `503`. The held request keeps its place across both reloads, and
/// a second reload back to a small cap admits the next request again.
#[test]
fn a_changed_memory_limit_moves_the_admission_ceiling() {
    const SMALL: u64 = 64 * 1024 * 1024;
    const HUGE: u64 = 1 << 50;
    let server = Server::start("memory", &memory(SMALL), &[("app.nvs", HOLDING)]);
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

    let report = server.reload(&memory(HUGE));
    assert!(
        report.contains("applied: limits.memory\n"),
        "the reload did not name `limits.memory` as applied: {report}"
    );
    let refused = server.awaits("/", "a refusal at the lowered ceiling", |answer| {
        answer.status == 503
    });
    assert_eq!(refused.header("retry-after"), Some("1"), "{refused:?}");
    assert!(
        server.ctl("status").contains("in_flight: 1\n"),
        "the held request lost its place in the count"
    );

    server.reload(&memory(SMALL));
    server.awaits("/", "an admission at the raised ceiling", |answer| {
        answer.status == 200 && answer.body == "ok"
    });
}

/// A SQLite queue with one worker, whose `[queue] visibility` is `visibility`.
/// `max_attempts = 1` moves a job that throws to the dead-letter table at
/// once, so no retry of an older job writes a line after the reload.
fn queue(visibility: &str) -> String {
    format!(
        "[capabilities.script]\nspawn = [\"jobs/\"]\n\n[db.jobs]\ndriver = \"sqlite\"\npath = \
         \"jobs.db\"\n\n[queue]\nconnection = \"jobs\"\nworkers = 1\nmax_attempts = 1\nvisibility \
         = \"{visibility}\"\n"
    )
}

/// An entry file that pushes one job per request.
const PUSHING: &str = "<?nvs\nCore\\Queue::push(\"jobs/work.nvs\");\necho \"pushed\";\n";

/// A job that throws the `[queue] visibility` it reads. A throw is what the
/// worker reports on standard error.
const READING: &str =
    "<?nvs\nthrow new RuntimeError(\"visibility \" . Core\\Config::get('queue.visibility'));\n";

/// A queue job runs under the snapshot in force when a worker claims it, and
/// not under the one the worker started with. After a reload moves `[queue]
/// visibility`, a job pushed afterwards reads the new value.
#[test]
fn a_queue_job_runs_under_the_configuration_in_force_when_it_is_claimed() {
    const THREW: &str = "`jobs/work.nvs` threw RuntimeError: visibility ";
    let server = Server::start_after(
        "queue",
        &queue("5m"),
        &[("app.nvs", PUSHING), ("jobs/work.nvs", READING)],
        &["queue", "migrate"],
    );
    server.awaits("/", "the first push", |answer| answer.body == "pushed");
    let started = Instant::now();
    while !server.said().contains(&format!("{THREW}5m")) {
        assert!(
            started.elapsed() <= BOUND,
            "no queued job read `5m` within {BOUND:?}; the server wrote: {}",
            server.said()
        );
        thread::sleep(POLL);
    }

    let report = server.reload(&queue("7m"));
    assert!(
        report.contains("applied: queue.visibility\n"),
        "the reload did not name `queue.visibility` as applied: {report}"
    );

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
        server.awaits("/", "a push", |answer| answer.body == "pushed");
        thread::sleep(POLL * 10);
    };
    assert!(
        after.ends_with(&format!("{THREW}7m")),
        "a job claimed after the reload did not read the reloaded `[queue] visibility`: {after}"
    );
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
    let fired = "the scheduled entry `minutely` threw";
    let started = Instant::now();
    while !server.said().contains(fired) {
        assert!(
            started.elapsed() <= BOUND + Duration::from_secs(60),
            "the entry a reload added did not fire within a minute; the server wrote: {}",
            server.said()
        );
        thread::sleep(POLL * 10);
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
