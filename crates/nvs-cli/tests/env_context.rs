//! `rule:core-classes/env-context` from outside the process: each kind of run
//! `Core\Env::context()` names, started by the host that starts it for real.
//!
//! A `.nvst` case reaches three of the seven kinds — a command, an in-process
//! request and a spawned script. The other four, and a request served over a
//! socket, need the host itself: `nvs serve` for a request, a scheduled fire, a
//! queued job and both connection doors, `nvs run` for a `#[Command]` method
//! and `nvs test` for a `#[Test]` method.
//!
//! Every program prints its kind through [`KIND`], so an answer is the case's
//! name. A run with no reader of its output — a job and a scheduled fire —
//! throws the name instead, because a throw is the one outcome both report on
//! standard error.
//!
//! Each case's program lives in a directory of its own under
//! `CARGO_TARGET_TMPDIR`, named for the case and the process, and is removed
//! when the case ends.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// How long a served answer, a job or a connection has to arrive.
const BOUND: Duration = Duration::from_secs(30);

/// How long a scheduled fire has to arrive: its cron fires once a minute.
const FIRE: Duration = Duration::from_secs(90);

/// The time between two looks at standard error.
const POLL: Duration = Duration::from_millis(50);

/// A class that names a `Core\Env::Context` case, put at the top of every
/// program below.
const KIND: &str = r#"class Kind {
    public static function of(Core\Env::Context $context): string {
        return match ($context) {
            Core\Env::Context::Request => "Request",
            Core\Env::Context::Command => "Command",
            Core\Env::Context::Isolate => "Isolate",
            Core\Env::Context::Schedule => "Schedule",
            Core\Env::Context::Job => "Job",
            Core\Env::Context::Test => "Test",
            Core\Env::Context::Connection => "Connection",
        };
    }
}
"#;

/// A program: `<?nvs`, [`KIND`], then `body`.
fn program(body: &str) -> String {
    format!("<?nvs\n{KIND}\n{body}")
}

/// A program that throws `kind=` and its own kind of run.
fn throwing_its_kind() -> String {
    program("throw new RuntimeError(\"kind=\" . Kind::of(Core\\Env::context()));\n")
}

/// A fresh directory named for `case` with `files` written into it.
fn directory(case: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("env-context-{case}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, text) in files {
        let path = dir.join(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the file's directory is created");
        }
        std::fs::write(&path, text).unwrap_or_else(|error| {
            panic!("`{}` could not be written: {error}", path.display());
        });
    }
    dir
}

/// Runs `nvs` with `args` in a fresh directory holding `files`, and removes
/// the directory afterwards.
fn nvs(case: &str, files: &[(&str, &str)], args: &[&str]) -> Output {
    let dir = directory(case, files);
    let data = nvs_repo::scratch_private("nvsdata");
    let ran = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .args(args)
        .current_dir(&dir)
        .stdin(Stdio::null())
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let _ = std::fs::remove_dir_all(&dir);
    ran
}

/// A running `nvs serve app.nvs` over a directory of its own.
///
/// Dropping it stops the process and removes the directory.
struct Server {
    child: Child,
    addr: SocketAddr,
    dir: PathBuf,
    /// The private data folder the server is given.
    _data: nvs_repo::Scratch,
    stderr: Arc<Mutex<String>>,
}

impl Server {
    /// Writes `files` into a fresh directory named for `case`, runs `nvs`
    /// with `first` as its arguments there unless it is empty, and starts
    /// `nvs serve app.nvs` on a free loopback port.
    ///
    /// # Panics
    ///
    /// When the first command fails, or the server does not print its
    /// `listening on` line within [`BOOT`].
    fn start(case: &str, files: &[(&str, &str)], first: &[&str]) -> Self {
        let dir = directory(case, files);
        let data = nvs_repo::scratch_private("nvsdata");
        if !first.is_empty() {
            let ran = Command::new(env!("CARGO_BIN_EXE_nvs"))
                .arg("--data")
                .arg(&*data)
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
            .arg("--data")
            .arg(&*data)
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
            _data: data,
            stderr,
        }
    }

    /// The body of one `GET /`, on an HTTP/1.0 connection of its own, so the
    /// body ends where the connection does.
    ///
    /// # Panics
    ///
    /// When the server does not answer `200`.
    fn get(&self) -> String {
        let mut stream = TcpStream::connect(self.addr)
            .unwrap_or_else(|error| panic!("{} does not answer: {error}", self.addr));
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        write!(stream, "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n").expect("the request is sent");
        let mut raw = Vec::new();
        stream
            .read_to_end(&mut raw)
            .unwrap_or_else(|error| panic!("the answer was cut off: {error}"));
        let text = String::from_utf8_lossy(&raw);
        let (head, body) = text
            .split_once("\r\n\r\n")
            .unwrap_or_else(|| panic!("the answer has no header block: {text}"));
        assert!(
            head.starts_with("HTTP/1.0 200") || head.starts_with("HTTP/1.1 200"),
            "the request was not answered `200`: {head}; the server wrote: {}",
            self.said()
        );
        body.to_owned()
    }

    /// What the server has written to standard error so far.
    fn said(&self) -> String {
        self.stderr.lock().expect("no reader panicked").clone()
    }

    /// Waits until the server has written `text` to standard error.
    ///
    /// # Panics
    ///
    /// When `bound` runs out first.
    fn awaits_said(&self, text: &str, bound: Duration) {
        let started = Instant::now();
        while !self.said().contains(text) {
            assert!(
                started.elapsed() <= bound,
                "the server did not write `{text}` within {bound:?}; it wrote: {}",
                self.said()
            );
            thread::sleep(POLL);
        }
    }

    /// Opens a WebSocket on `/`, sends one short text frame and returns the
    /// text of the frame that answers it.
    ///
    /// # Panics
    ///
    /// When the server does not answer `101`, or the answer is not one short
    /// unmasked text frame.
    fn websocket_answer(&self) -> String {
        const MASK: [u8; 4] = [0x37, 0xfa, 0x21, 0x3d];
        let mut stream = TcpStream::connect(self.addr)
            .unwrap_or_else(|error| panic!("{} does not answer: {error}", self.addr));
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        write!(
            stream,
            "GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: \
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
                .unwrap_or_else(|error| panic!("the upgrade was cut off: {error}"));
            head.push(byte[0]);
        }
        let head = String::from_utf8_lossy(&head);
        assert!(
            head.starts_with("HTTP/1.1 101 "),
            "the request did not switch protocols: {head}; the server wrote: {}",
            self.said()
        );

        // A final text frame, masked: a client must mask every frame it sends.
        let text = b"hello";
        let mut frame = vec![0x81, 0x80 | 5];
        frame.extend_from_slice(&MASK);
        frame.extend(text.iter().zip(MASK.iter().cycle()).map(|(b, m)| b ^ m));
        stream.write_all(&frame).expect("the frame is sent");

        let mut head = [0; 2];
        stream
            .read_exact(&mut head)
            .unwrap_or_else(|error| panic!("no frame answered: {error}"));
        assert!(
            head[0] == 0x81 && head[1] < 126,
            "the answer is a frame this client does not read: {head:?}"
        );
        let mut body = vec![0; usize::from(head[1])];
        stream
            .read_exact(&mut body)
            .unwrap_or_else(|error| panic!("the answer was cut off: {error}"));
        String::from_utf8(body).expect("a text frame is UTF-8")
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A deployment with a SQLite queue and one worker, and a `[[schedule]]`
/// entry that fires every minute. Both run `jobs/kind.nvs`. `max_attempts =
/// 1` sends a job that throws to the dead-letter table at once.
const QUEUED: &str = r#"[capabilities.script]
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
script = "jobs/kind.nvs"
scope = "host"
"#;

#[test]
fn a_served_request_is_a_request() {
    let app = program("echo Kind::of(Core\\Env::context());\n");
    let server = Server::start("request", &[("app.nvs", &app)], &[]);
    assert_eq!(server.get(), "Request");
}

#[test]
fn a_command_method_is_a_command() {
    let app = program(
        "class Shop {
    #[Core\\Command(name: \"show\", about: \"Prints the kind of run\")]
    public static function show(): uint {
        echo Kind::of(Core\\Env::context()), \"\\n\";
        return 0;
    }
}
uint $status = Core\\Command::run();
",
    );
    let ran = nvs("command", &[("app.nvs", &app)], &["run", "app.nvs", "show"]);
    assert!(
        ran.status.success(),
        "`nvs run` failed: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "Command\n");
}

#[test]
fn a_test_method_is_a_test() {
    let app = program(
        "final class ShopTest {
    #[Core\\Test]
    public function knowsItsKind(): void {
        echo \"kind=\", Kind::of(Core\\Env::context()), \"\\n\";
        Core\\Test::assertSame(Kind::of(Core\\Env::context()), \"Test\");
    }
}
",
    );
    // `--show-output`, because a passing test's output is otherwise dropped.
    let ran = nvs(
        "test",
        &[("app.nvs", &app)],
        &["test", "--show-output", "app.nvs"],
    );
    let stdout = String::from_utf8_lossy(&ran.stdout);
    assert!(
        ran.status.success(),
        "`nvs test` failed: {stdout}{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert!(stdout.contains("kind=Test"), "the test printed: {stdout}");
}

/// The entry fires at the start of the next minute, so this case may wait
/// for most of one.
#[test]
fn a_scheduled_script_is_a_schedule() {
    let server = Server::start(
        "schedule",
        &[
            ("nvs.toml", QUEUED),
            ("app.nvs", "<?nvs\necho \"idle\";\n"),
            ("jobs/kind.nvs", &throwing_its_kind()),
        ],
        &["queue", "migrate"],
    );
    server.awaits_said("kind=Schedule", FIRE);
}

#[test]
fn a_queued_job_is_a_job() {
    let server = Server::start(
        "job",
        &[
            ("nvs.toml", QUEUED),
            (
                "app.nvs",
                "<?nvs\nCore\\Queue::push(\"jobs/kind.nvs\");\necho \"pushed\";\n",
            ),
            ("jobs/kind.nvs", &throwing_its_kind()),
        ],
        &["queue", "migrate"],
    );
    assert_eq!(server.get(), "pushed");
    server.awaits_said("kind=Job", BOUND);
}

#[test]
fn a_socket_connection_is_a_connection() {
    let server = Server::start(
        "socket",
        &[
            (
                "nvs.toml",
                "[capabilities.script]\nspawn = [\"sockets/\"]\n",
            ),
            (
                "app.nvs",
                "<?nvs\nCore\\Socket::upgrade(\"sockets/kind.nvs\");\n",
            ),
            (
                "sockets/kind.nvs",
                &program(
                    "var $conn = Core\\Socket::current();\nvar $msg = $conn->receive();\n\
                     $conn->send(Kind::of(Core\\Env::context()));\n",
                ),
            ),
        ],
        &[],
    );
    assert_eq!(server.websocket_answer(), "Connection");
}

#[test]
fn an_event_stream_connection_is_a_connection() {
    let app = program(
        "class Feed {
    public static function run(): void {
        Core\\Sse::current()->send(Kind::of(Core\\Env::context()));
    }
}
Core\\Sse::upgrade(Feed::run);
",
    );
    let server = Server::start("event-stream", &[("app.nvs", &app)], &[]);
    let body = server.get();
    assert!(
        body.contains("data: Connection"),
        "the stream sent: {body}; the server wrote: {}",
        server.said()
    );
}
