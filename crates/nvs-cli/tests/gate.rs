//! `rule:routing/a-gate-answers-around-every-handler`'s refusal mode, from
//! outside the process: the built `nvs serve` over a program that calls
//! `Core\Router::dispatch`, and the door's three refusals — CSRF, a body over
//! the cap and an unknown verb — answered by the program's gate and never by
//! its route method. And its limit: a run stopped by its memory or CPU-time
//! limit answered by the gate's `failed`, from a second run under the same
//! limits.
//!
//! [`Server`] starts the binary this test was built beside on a port the
//! platform chose and reads the port back from the `listening on` line, as
//! `serve_isolation.rs` does. Each request goes over a connection of its own
//! with `Connection: close`, so the answer is everything read to the end.
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
use std::time::Duration;

/// How long one answer has to arrive before the case fails.
const BOUND: Duration = Duration::from_secs(30);

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// The route every program declares: a `POST`, so the CSRF check covers it.
const ROUTE: &str = "class Notes {\n\
    #[Core\\Route(path: \"/notes\", method: Core\\Http\\Method::Post)]\n\
    #[Core\\Access(allow: Core\\Audience::Public)]\n\
    public static function create(): void {\n\
        echo \"handler ran\";\n\
    }\n\
}\n";

/// A gate whose `refused` prints the reason, the route it was given and the
/// route `Core\Request::route()` returns. `{refused}` is the line it ends with.
const GATE: &str = "class Gate implements Core\\Router\\Gate {\n\
    public function allows(Core\\Router\\Match $route): bool {\n\
        return true;\n\
    }\n\
    public function refused(Core\\Router\\Refusal $reason, ?Core\\Router\\Match $route): void {\n\
        string $why = match ($reason) {\n\
            Core\\Router\\Refusal::UnknownMethod => \"UnknownMethod\",\n\
            Core\\Router\\Refusal::Csrf => \"Csrf\",\n\
            Core\\Router\\Refusal::BodyTooLarge => \"BodyTooLarge\",\n\
            default => \"other\",\n\
        };\n\
        echo \"refused \", $why, \" for \", $route == null ? \"no route\" : ($route->is(Notes::create) ? \"create\" : \"another route\");\n\
        ?Core\\Router\\Match $current = Core\\Request::route();\n\
        echo \" with \", $current == null ? \"no match\" : ($current->is(Notes::create) ? \"create\" : \"another route\");\n\
        {refused}\n\
    }\n\
    public function failed(?Core\\Router\\Match $route, ?Throwable $error): void {\n\
        echo \"failed\";\n\
    }\n\
}\n\
Core\\Router::dispatch(new Gate());\n";

/// A program that dispatches with a gate whose `refused` ends with `last`.
fn dispatching(last: &str) -> String {
    format!("<?nvs\n{ROUTE}{}", GATE.replace("{refused}", last))
}

/// A program that declares the same route and never calls `dispatch`.
fn by_hand() -> String {
    format!("<?nvs\n{ROUTE}echo \"program ran\";\n")
}

/// A running `nvs serve` over one program in its own directory.
///
/// Dropping it stops the process and removes the directory.
struct Server {
    child: Child,
    addr: SocketAddr,
    dir: PathBuf,
    /// The private data folder every `nvs` run of this server is given.
    _data: nvs_repo::Scratch,
    stderr: Arc<Mutex<String>>,
}

impl Server {
    /// Writes `program` and an `nvs.toml` in production mode into a fresh
    /// directory named for `case`, and starts `nvs serve app.nvs` there, on a
    /// free loopback port.
    ///
    /// # Panics
    ///
    /// When the process cannot start, or does not print its `listening on`
    /// line within [`BOOT`]; the message carries what it wrote to standard
    /// error.
    fn start(case: &str, program: &str) -> Self {
        Self::start_with(case, program, "")
    }

    /// [`Self::start`], with `more` appended to the configuration.
    fn start_with(case: &str, program: &str, more: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("gate-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        std::fs::write(dir.join("app.nvs"), program).expect("the program is written");
        std::fs::write(
            dir.join("nvs.toml"),
            format!("[mode]\ndefault = \"production\"\n\n[server]\nworkers = 1\n{more}"),
        )
        .expect("the configuration is written");
        let data = nvs_repo::scratch_private("nvsdata");

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

    /// Sends `verb /notes` with the extra header lines `headers` on a
    /// connection of its own, and returns the status and the body.
    ///
    /// # Panics
    ///
    /// When the server does not accept the connection or the answer has no
    /// status line.
    fn send(&self, verb: &str, headers: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(self.addr)
            .unwrap_or_else(|error| panic!("{} does not answer: {error}", self.addr));
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        write!(
            stream,
            "{verb} /notes HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n{headers}\r\n",
            self.addr
        )
        .expect("the request is sent");
        let mut answer = Vec::new();
        let _ = stream.read_to_end(&mut answer);
        let answer = String::from_utf8_lossy(&answer).into_owned();
        let (head, body) = answer
            .split_once("\r\n\r\n")
            .unwrap_or_else(|| panic!("`{verb}` got no whole answer: {answer:?}\n{}", self.said()));
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or_else(|| panic!("`{verb}` got no status line: {head:?}"));
        let chunked = head
            .to_ascii_lowercase()
            .contains("transfer-encoding: chunked");
        let body = if chunked {
            unchunk(body)
        } else {
            body.to_owned()
        };
        (status, body)
    }

    /// What the server has written to standard error so far.
    fn said(&self) -> String {
        self.stderr.lock().expect("no reader panicked").clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The body a chunked answer carries, joined.
fn unchunk(mut rest: &str) -> String {
    let mut body = String::new();
    while let Some((size, after)) = rest.split_once("\r\n") {
        let Ok(size) = usize::from_str_radix(size.trim(), 16) else {
            break;
        };
        if size == 0 || after.len() < size {
            break;
        }
        body.push_str(&after[..size]);
        rest = after[size..].trim_start_matches("\r\n");
    }
    body
}

/// A write whose `Origin` is another site, which the door's CSRF check refuses.
const FORGED: &str = "Origin: http://elsewhere.example\r\nContent-Length: 0\r\n";

/// A body whose declared length is over the server's cap. No byte of it is
/// sent, because the door refuses it on the length alone.
const OVER_THE_CAP: &str = "Content-Length: 300000000\r\n";

/// A verb that is not a `Core\Http\Method` case.
const UNKNOWN: &str = "PURGE";

#[test]
fn a_csrf_refusal_reaches_the_gate_and_no_handler() {
    let server = Server::start("csrf", &dispatching(""));
    let (status, body) = server.send("POST", FORGED);
    assert_eq!(
        (status, body.as_str()),
        (403, "refused Csrf for no route with no match"),
        "{}",
        server.said()
    );
}

#[test]
fn a_body_over_the_cap_reaches_the_gate_and_no_handler() {
    let server = Server::start("body", &dispatching(""));
    let (status, body) = server.send("POST", OVER_THE_CAP);
    assert_eq!(
        (status, body.as_str()),
        (413, "refused BodyTooLarge for no route with no match"),
        "{}",
        server.said()
    );
}

#[test]
fn an_unknown_method_reaches_the_gate_and_no_handler() {
    let server = Server::start("verb", &dispatching(""));
    let (status, body) = server.send(UNKNOWN, "");
    assert_eq!(
        (status, body.as_str()),
        (501, "refused UnknownMethod for no route with no match"),
        "{}",
        server.said()
    );
    // The same server still runs the route method for a request it accepts.
    let (status, body) = server.send("POST", "Content-Length: 0\r\n");
    assert_eq!((status, body.as_str()), (200, "handler ran"));
}

#[test]
fn a_gate_that_throws_leaves_the_empty_answer() {
    let server = Server::start(
        "throws",
        &dispatching("throw new RuntimeError(\"the gate failed\");"),
    );
    // What the gate wrote before it threw is not sent, and neither is a `500`.
    assert_eq!(server.send("POST", FORGED), (403, String::new()));
    assert_eq!(server.send("POST", OVER_THE_CAP), (413, String::new()));
    assert_eq!(server.send(UNKNOWN, ""), (501, String::new()));
}

/// A program whose `GET` route method runs `{body}` after it writes a line,
/// and whose gate's `failed` runs `{failed}` and then prints the route and
/// the error it was given.
const OVER: &str = "<?nvs\n\
class Endless {\n\
    public static function counter(): Iterator<int> {\n\
        int $i = 0;\n\
        while ($i < 4000000) {\n\
            yield $i;\n\
            $i = $i + 1;\n\
        }\n\
    }\n\
}\n\
class Notes {\n\
    #[Core\\Route(path: \"/notes\", method: Core\\Http\\Method::Get)]\n\
    #[Core\\Access(allow: Core\\Audience::Public)]\n\
    public static function list(): void {\n\
        echo \"handler started\";\n\
        {body}\n\
    }\n\
}\n\
class Gate implements Core\\Router\\Gate {\n\
    public function allows(Core\\Router\\Match $route): bool {\n\
        return true;\n\
    }\n\
    public function refused(Core\\Router\\Refusal $reason, ?Core\\Router\\Match $route): void {\n\
        echo \"refused\";\n\
    }\n\
    public function failed(?Core\\Router\\Match $route, ?Throwable $error): void {\n\
        {failed}\n\
        echo \"failed for \", $route == null ? \"no route\" : ($route->is(Notes::list) ? \"list\" : \"another route\");\n\
        echo \" with \", $error?->message ??\"no error\";\n\
    }\n\
}\n\
Core\\Router::dispatch(new Gate());\n";

/// Builds an array of four million numbers, which is over a 16 MB limit.
const GROWS: &str = "array<int> $all = Core\\Arr::from(Endless::counter());\n\
        echo Core\\Arr::count($all);";

/// Counts for as long as it may, and never ends by itself.
const SPINS: &str = "int $spins = 0;\n\
        while ($spins >= 0) {\n\
            $spins = $spins + 1;\n\
        }";

/// A program whose route method runs `body` and whose `failed` runs `failed`.
fn over(body: &str, failed: &str) -> String {
    OVER.replace("{body}", body).replace("{failed}", failed)
}

#[test]
fn a_program_over_its_memory_limit_reaches_failed() {
    let server = Server::start_with("memory", &over(GROWS, ""), "\n[limits]\nmemory = \"16M\"\n");
    // The first run's line is not sent: the answer is the second run's page.
    let (status, body) = server.send("GET", "");
    assert_eq!(
        (status, body.as_str()),
        (500, "failed for no route with no error"),
        "{}",
        server.said()
    );
}

#[test]
fn a_program_over_its_time_limit_reaches_failed() {
    let server = Server::start_with(
        "time",
        &over(SPINS, ""),
        "\n[limits]\ncpu_time = \"200ms\"\n",
    );
    let (status, body) = server.send("GET", "");
    assert_eq!(
        (status, body.as_str()),
        (500, "failed for no route with no error"),
        "{}",
        server.said()
    );
}

#[test]
fn a_failed_page_over_the_limit_leaves_the_empty_500() {
    let server = Server::start_with(
        "failed-over",
        &over(SPINS, SPINS),
        "\n[limits]\ncpu_time = \"200ms\"\n",
    );
    // The second run is stopped too, and nothing is run a third time.
    assert_eq!(server.send("GET", ""), (500, String::new()));
}

#[test]
fn a_program_that_does_not_dispatch_keeps_the_empty_answer() {
    let server = Server::start("by-hand", &by_hand());
    // The program does not run for any of the three, so nothing it would
    // print is in the answer.
    assert_eq!(server.send("POST", FORGED), (403, String::new()));
    assert_eq!(server.send("POST", OVER_THE_CAP), (413, String::new()));
    assert_eq!(server.send(UNKNOWN, ""), (501, String::new()));
    let (status, body) = server.send("POST", "Content-Length: 0\r\n");
    assert_eq!((status, body.as_str()), (200, "program ran"));
}
