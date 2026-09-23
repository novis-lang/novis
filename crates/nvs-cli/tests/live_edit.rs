//! `rule:config/an-edit-reaches-the-next-request-without-a-restart`, from outside
//! the process: the built `nvs serve` over a program in a directory of its own,
//! an edit to that directory, and the answer a request gets afterwards.
//!
//! Every case goes through [`Server`]. It starts the binary this test was built
//! beside on a port the platform chose, reads the port back from the
//! `listening on` line, and answers each request over a fresh HTTP/1.0
//! connection, so the body ends where the connection does and no framing is
//! parsed. A WebSocket is opened by [`Server::websocket`], which reads short
//! text frames and nothing else. An edit is observed by polling:
//! [`Server::awaits`] asks until the answer is the one wanted or [`BOUND`] runs
//! out, and the failure message says what the last answer was. The bound is far above what the rule allows an
//! idle server — `revalidate_freq` plus `settle` — so a pass is never a race
//! against a slow machine, and a fail means the edit never arrived.
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

/// How long an edit has to reach a request before the case fails.
const BOUND: Duration = Duration::from_secs(30);

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// The time between two requests of one poll.
const POLL: Duration = Duration::from_millis(50);

/// `[mode] default` written out, so a case about production does not depend on
/// what a configuration with nothing in it starts in.
const PRODUCTION: &str = "[mode]\ndefault = \"production\"\n";

/// One answer: the status code and the body, as text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Answer {
    status: u16,
    body: String,
}

/// A running `nvs serve` over a program in its own directory.
///
/// Dropping it stops the process and removes the directory.
struct Server {
    child: Child,
    addr: SocketAddr,
    dir: PathBuf,
    stderr: Arc<Mutex<String>>,
}

impl Server {
    /// Writes `files` — each a path relative to the program's directory and its
    /// text — into a fresh directory named for `case`, and starts `nvs serve
    /// app.nvs` there, on a free loopback port.
    ///
    /// The server runs with that directory as its working directory, so an
    /// `nvs.toml` among `files` is the configuration it reads.
    ///
    /// # Panics
    ///
    /// When the process cannot start, or does not print its `listening on`
    /// line within [`BOOT`]; the message carries what it wrote to standard
    /// error.
    fn start(case: &str, files: &[(&str, &str)]) -> Self {
        Self::start_after(case, files, &[])
    }

    /// [`Server::start`], which first runs `nvs` with `first` as its arguments
    /// in the program's directory, unless `first` is empty.
    ///
    /// # Panics
    ///
    /// As [`Server::start`] does, and when that first command fails.
    fn start_after(case: &str, files: &[(&str, &str)], first: &[&str]) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("live-edit-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        for (path, text) in files {
            write_file(&dir.join(path), text);
        }
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

    /// Polls `path` until it answers `200` with exactly `body`.
    fn awaits_body(&self, path: &str, body: &str) -> Answer {
        self.awaits(path, &format!("`{path}` answering `{body}`"), |answer| {
            answer.status == 200 && answer.body == body
        })
    }

    /// What the server has written to standard error so far.
    fn said(&self) -> String {
        self.stderr.lock().expect("no reader panicked").clone()
    }

    /// Waits until the server has written `text` to standard error. Standard
    /// error is one ordered stream, so every line written before `text` has
    /// arrived too.
    ///
    /// # Panics
    ///
    /// When [`BOUND`] runs out first.
    fn awaits_said(&self, text: &str) {
        let started = Instant::now();
        while !self.said().contains(text) {
            assert!(
                started.elapsed() <= BOUND,
                "the server did not write `{text}` within {BOUND:?}; it wrote: {}",
                self.said()
            );
            thread::sleep(POLL);
        }
    }

    /// Writes `text` to `path`, relative to the program's directory, creating
    /// any directory it needs.
    fn write(&self, path: &str, text: &str) {
        write_file(&self.dir.join(path), text);
    }

    /// Opens a WebSocket on `path`, on a connection of its own.
    ///
    /// # Panics
    ///
    /// When the server does not answer `101`.
    fn websocket(&self, path: &str) -> WebSocket {
        let mut stream = TcpStream::connect(self.addr)
            .unwrap_or_else(|error| panic!("{} does not answer: {error}", self.addr));
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: \
             Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: \
             13\r\n\r\n"
        )
        .expect("the upgrade is sent");
        // Read a byte at a time, so no frame after the header block is taken
        // with it.
        let mut head = Vec::new();
        let mut byte = [0; 1];
        while !head.ends_with(b"\r\n\r\n") {
            stream
                .read_exact(&mut byte)
                .unwrap_or_else(|error| panic!("the upgrade of `{path}` was cut off: {error}"));
            head.push(byte[0]);
        }
        let head = String::from_utf8_lossy(&head);
        assert!(
            head.starts_with("HTTP/1.1 101 "),
            "`{path}` did not switch protocols: {head}"
        );
        WebSocket { stream }
    }

    /// Deletes `path`, relative to the program's directory.
    fn remove(&self, path: &str) {
        let path = self.dir.join(path);
        std::fs::remove_file(&path).unwrap_or_else(|error| {
            panic!("`{}` could not be removed: {error}", path.display());
        });
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// One open WebSocket, which sends and reads short text frames only.
///
/// Dropping it closes the connection.
struct WebSocket {
    stream: TcpStream,
}

impl WebSocket {
    /// Sends `text` as one frame and returns the text of the next frame.
    ///
    /// # Panics
    ///
    /// When `text` or the answer is 126 bytes or longer, or the answer is not
    /// one unmasked text frame.
    fn ask(&mut self, text: &str) -> String {
        const MASK: [u8; 4] = [0x37, 0xfa, 0x21, 0x3d];
        let len = u8::try_from(text.len())
            .ok()
            .filter(|len| *len < 126)
            .expect("a frame this client sends is shorter than 126 bytes");
        // A final text frame, masked: a client must mask every frame it sends.
        let mut frame = vec![0x81, 0x80 | len];
        frame.extend_from_slice(&MASK);
        frame.extend(text.bytes().zip(MASK.iter().cycle()).map(|(b, m)| b ^ m));
        self.stream.write_all(&frame).expect("the frame is sent");

        let mut head = [0; 2];
        self.stream
            .read_exact(&mut head)
            .unwrap_or_else(|error| panic!("no frame answered `{text}`: {error}"));
        assert!(
            head[0] == 0x81 && head[1] < 126,
            "`{text}` was answered by a frame this client does not read: {head:?}"
        );
        let mut body = vec![0; usize::from(head[1])];
        self.stream
            .read_exact(&mut body)
            .unwrap_or_else(|error| panic!("the answer to `{text}` was cut off: {error}"));
        String::from_utf8(body).expect("a text frame is UTF-8")
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

/// A program that prints `word` and nothing else.
fn printing(word: &str) -> String {
    format!("<?nvs\necho \"{word}\";\n")
}

/// The entry file is the one file today's resolve-time check has always seen,
/// so this case holds the harness to the half of the rule that is on disk.
#[test]
fn an_edit_to_the_entry_file_reaches_the_next_request_in_production() {
    let server = Server::start(
        "entry",
        &[("nvs.toml", PRODUCTION), ("app.nvs", &printing("before"))],
    );
    server.awaits_body("/", "before");

    server.write("app.nvs", &printing("after"));
    server.awaits_body("/", "after");
}

/// A broken entry file fails every request that reaches it, and the fix that
/// follows is served without a restart.
#[test]
fn a_broken_edit_fails_requests_and_its_fix_recovers_them() {
    let server = Server::start(
        "broken",
        &[("nvs.toml", PRODUCTION), ("app.nvs", &printing("good"))],
    );
    server.awaits_body("/", "good");

    server.write("app.nvs", "<?nvs\necho \"broken\"\n");
    let failed = server.awaits("/", "the broken edit failing `/`", |answer| {
        answer.status != 200
    });
    assert!(
        !failed.body.contains("good"),
        "the last good version is never served in place of a broken one: {failed:?}"
    );

    server.write("app.nvs", &printing("fixed"));
    server.awaits_body("/", "fixed");
}

/// An entry file that requires `lib.nvs` and prints nothing itself.
const REQUIRING: &str = "<?nvs\nrequire './lib.nvs';\n";

/// The entry file never changes here, so only a check of the file it requires
/// can bring the edit to a request.
#[test]
fn an_edit_to_a_required_file_reaches_the_next_request() {
    let server = Server::start(
        "required",
        &[
            ("nvs.toml", PRODUCTION),
            ("app.nvs", REQUIRING),
            ("lib.nvs", &printing("before")),
        ],
    );
    server.awaits_body("/", "before");

    server.write("lib.nvs", &printing("after"));
    server.awaits_body("/", "after");
}

/// A class reached through `autoload` is a file of the program like any other,
/// though nothing names its path.
#[test]
fn an_edit_to_an_autoloaded_class_reaches_the_next_request() {
    let server = Server::start(
        "autoloaded",
        &[
            ("nvs.toml", PRODUCTION),
            ("app.nvs", &saying("Word", &["./src"])),
            ("src/Word.nvs", &class("Word", "before")),
        ],
    );
    server.awaits_body("/", "before");

    server.write("src/Word.nvs", &class("Word", "after"));
    server.awaits_body("/", "after");
}

/// A class that no file declares yet fails the program, and writing its file
/// under the `autoload` root is found without a restart. The entry file does not
/// change after the class is named, so only the path the resolution looked for
/// and missed can bring the new file to a request.
#[test]
fn a_new_class_file_under_an_autoload_root_is_found_without_a_restart() {
    let server = Server::start(
        "new-class",
        &[
            ("nvs.toml", PRODUCTION),
            ("app.nvs", &saying("Word", &["./src"])),
            ("src/Word.nvs", &class("Word", "first")),
        ],
    );
    server.awaits_body("/", "first");

    server.write("app.nvs", &saying("Other", &["./src"]));
    server.awaits("/", "naming a class with no file failing `/`", |answer| {
        answer.status != 200
    });

    server.write("src/Other.nvs", &class("Other", "found"));
    server.awaits_body("/", "found");
}

/// A class file written under the first `autoload` root takes over from the one
/// under the second root. No file the program read has changed, so only the
/// path the resolution probed and missed can bring it to a request.
#[test]
fn a_file_that_shadows_an_autoload_probe_miss_takes_over_without_a_restart() {
    let server = Server::start(
        "shadow",
        &[
            ("nvs.toml", PRODUCTION),
            ("app.nvs", &saying("Word", &["./src", "./vendor"])),
            ("vendor/Word.nvs", &class("Word", "vendor")),
            ("src/.keep", ""),
        ],
    );
    server.awaits_body("/", "vendor");

    server.write("src/Word.nvs", &class("Word", "src"));
    server.awaits_body("/", "src");
}

/// A module written into a directory under the root that already holds one
/// joins what `implementing` returns. Nothing names the new class and no file
/// the program read changes, so only the listing of that directory can bring
/// it to a request.
#[test]
fn a_new_module_under_a_discovery_directory_joins_implementing_without_a_restart() {
    let server = Server::start(
        "discovery",
        &[
            ("nvs.toml", PRODUCTION),
            ("app.nvs", IMPLEMENTING),
            ("src/Said.nvs", SAID),
            ("src/One.nvs", &module("", "One", "one")),
            ("src/Plugins/Two.nvs", &module("\\Plugins", "Two", "two")),
        ],
    );
    server.awaits_body("/", "one;two;");

    server.write(
        "src/Plugins/Three.nvs",
        &module("\\Plugins", "Three", "three"),
    );
    server.awaits_body("/", "one;three;two;");
}

/// An entry file that maps `App` to `./src` and prints what `say` returns for
/// every class implementing `App\Said`, in the order `implementing` gives them.
const IMPLEMENTING: &str = "<?nvs\nautoload 'App' from './src';\nforeach (Core\\Program::implementing<App\\Said>() as App\\Said $it) {\n    echo $it->say(), \";\";\n}\n";

/// The file that declares `App\Said`.
const SAID: &str = "<?nvs\nnamespace App;\ninterface Said { public function say(): string; }\n";

/// The file that declares `App<namespace>\<name>`, implementing `App\Said` with
/// a `say` that returns `word`.
fn module(namespace: &str, name: &str, word: &str) -> String {
    format!(
        "<?nvs\nnamespace App{namespace};\nclass {name} implements App\\Said {{ public function say(): string {{ return '{word}'; }} }}\n"
    )
}

/// An entry file that maps `App` to `roots` and prints what `App\<name>::say`
/// returns.
fn saying(name: &str, roots: &[&str]) -> String {
    let roots = roots
        .iter()
        .map(|root| format!("'{root}'"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("<?nvs\nautoload 'App' from {roots};\nvar $it = new App\\{name}();\necho $it->say();\n")
}

/// The file that declares `App\<name>`, whose `say` returns `word`.
fn class(name: &str, word: &str) -> String {
    format!(
        "<?nvs\nnamespace App;\nclass {name} {{ public function say(): string {{ return '{word}'; }} }}\n"
    )
}

/// A required file that is deleted fails every request that reaches it, and
/// putting it back is served without a restart.
#[test]
fn a_deleted_required_file_fails_the_requests_that_reach_it() {
    let server = Server::start(
        "deleted",
        &[
            ("nvs.toml", PRODUCTION),
            ("app.nvs", REQUIRING),
            ("lib.nvs", &printing("present")),
        ],
    );
    server.awaits_body("/", "present");

    server.remove("lib.nvs");
    let failed = server.awaits("/", "the deletion failing `/`", |answer| {
        answer.status != 200
    });
    assert!(
        !failed.body.contains("present"),
        "the program compiled before the deletion is never served in its place: {failed:?}"
    );

    server.write("lib.nvs", &printing("restored"));
    server.awaits_body("/", "restored");
}

/// A program that prints `word`, and whose every compile writes one warning to
/// standard error: its `/** … */` comment documents nothing. The warning
/// quotes the comment, so `marker` counts the compiles of this version.
fn warned(marker: &str, word: &str) -> String {
    format!(
        "<?nvs\nclass Word {{\n    /** {marker} */\n    public static function say(): string {{ return '{word}'; }}\n}}\necho Word::say();\n"
    )
}

/// Undoing the last edit is answered from the unit compiled before it, so the
/// first version's warning is not written a second time.
#[test]
fn a_reverted_edit_is_answered_from_the_unit_already_compiled() {
    let first = warned("Compiled first.", "first");
    let server = Server::start("reverted", &[("nvs.toml", PRODUCTION), ("app.nvs", &first)]);
    server.awaits_body("/", "first");

    server.write("app.nvs", &warned("Compiled second.", "second"));
    server.awaits_body("/", "second");
    server.awaits_said("Compiled second.");
    let compiled = server.said().matches("Compiled first.").count();
    assert!(
        compiled > 0,
        "the first version compiled without its warning"
    );

    server.write("app.nvs", &first);
    server.awaits_body("/", "first");

    // A third version whose warning comes after anything the revert wrote.
    server.write("app.nvs", &warned("Compiled third.", "third"));
    server.awaits_body("/", "third");
    server.awaits_said("Compiled third.");
    assert_eq!(
        server.said().matches("Compiled first.").count(),
        compiled,
        "the reverted version was compiled again: {}",
        server.said()
    );
}

/// A deployment with a SQLite queue, one worker and a `[[schedule]]` entry that
/// fires every minute, where the queued job and the scheduled script are one
/// file. `max_attempts = 1` moves a job that throws to the dead-letter table at
/// once, so no retry of an older job writes a line after the edit.
const QUEUED: &str = r#"[mode]
default = "production"

[capabilities.script]
spawn = ["jobs/"]

[db.jobs]
driver = "sqlite"
path = "jobs.db"

[queue]
connection = "jobs"
workers = 1
max_attempts = 1

[[schedule]]
name = "nightly"
cron = "* * * * *"
script = "jobs/work.nvs"
scope = "host"
"#;

/// An entry file that pushes one job per request.
const PUSHING: &str = "<?nvs\nCore\\Queue::push(\"jobs/work.nvs\");\necho \"pushed\";\n";

/// A job whose every run throws `ran <word>`. A job that returns writes
/// nothing, and a throw is the one outcome both a worker and a fire report on
/// standard error.
fn throwing(word: &str) -> String {
    format!("<?nvs\nthrow new RuntimeError(\"ran {word}\");\n")
}

/// A queue worker and a scheduled fire resolve their script through the
/// compiler requests use. The first job compiles the script before the edit, so
/// a worker or a fire holding that unit past the edit reports `ran first`.
///
/// The fire comes at the next minute, so this case waits up to a minute more
/// than the others. Only fires written after a job ran the edit are read: until
/// then the edit may not have reached the compiler yet.
#[test]
fn a_queue_job_and_a_scheduled_fire_run_the_edited_code() {
    let server = Server::start_after(
        "queued",
        &[
            ("nvs.toml", QUEUED),
            ("app.nvs", PUSHING),
            ("jobs/work.nvs", &throwing("first")),
        ],
        &["queue", "migrate"],
    );
    server.awaits_body("/", "pushed");
    server.awaits_said("`jobs/work.nvs` threw RuntimeError: ran first");

    // One job per push, pushed until one runs the edit, far slower than
    // `POLL` so the queue never holds more than a few.
    server.write("jobs/work.nvs", &throwing("second"));
    let started = Instant::now();
    while !server
        .said()
        .contains("`jobs/work.nvs` threw RuntimeError: ran second")
    {
        assert!(
            started.elapsed() <= BOUND,
            "no queued job ran the edit within {BOUND:?}; the server wrote: {}",
            server.said()
        );
        server.awaits_body("/", "pushed");
        thread::sleep(POLL * 10);
    }

    let seen = server.said().len();
    let fired = "the scheduled entry `nightly` threw";
    let started = Instant::now();
    let line = loop {
        let said = server.said();
        if let Some(line) = said[seen..].lines().find(|line| line.contains(fired)) {
            break line.to_owned();
        }
        assert!(
            started.elapsed() <= BOUND + Duration::from_secs(60),
            "the entry did not fire within a minute of the edit; the server wrote: {said}"
        );
        thread::sleep(POLL * 10);
    };
    assert!(
        line.ends_with("ran second"),
        "a fire after a job ran the edit ran the code from before it: {line}"
    );
}

/// A deployment that may open the connection scripts under `sockets/`.
const SOCKETS: &str =
    "[mode]\ndefault = \"production\"\n\n[capabilities.script]\nspawn = [\"sockets/\"]\n";

/// An entry file that turns every request into a WebSocket run by
/// `sockets/echo.nvs`.
const UPGRADING: &str = "<?nvs\nCore\\Socket::upgrade(\"sockets/echo.nvs\");\n";

/// A connection script that answers every frame with `word`.
fn answering(word: &str) -> String {
    format!(
        "<?nvs\nvar $conn = Core\\Socket::current();\nvar $msg = $conn->receive();\nwhile ($msg != \
         null) {{\n    $conn->send(\"{word}\");\n    $msg = $conn->receive();\n}}\n"
    )
}

/// A connection runs the code it was opened with until it closes. A connection
/// opened after the edit runs the edit, and the one opened before it still
/// answers from the old code.
#[test]
fn a_running_websocket_keeps_the_code_it_started_with() {
    let server = Server::start(
        "websocket",
        &[
            ("nvs.toml", SOCKETS),
            ("app.nvs", UPGRADING),
            ("sockets/echo.nvs", &answering("first")),
        ],
    );
    let mut running = server.websocket("/");
    assert_eq!(running.ask("hello"), "first");

    server.write("sockets/echo.nvs", &answering("second"));
    let started = Instant::now();
    loop {
        let answer = server.websocket("/").ask("hello");
        if answer == "second" {
            break;
        }
        assert!(
            started.elapsed() <= BOUND,
            "no new connection ran the edit within {BOUND:?}; the last one answered `{answer}`, \
             and the server wrote: {}",
            server.said()
        );
        thread::sleep(POLL);
    }

    assert_eq!(
        running.ask("hello"),
        "first",
        "a connection opened before the edit changed code while it ran"
    );
}
