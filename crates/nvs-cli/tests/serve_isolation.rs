//! `rule:concurrency/cross-request-state-is-explicit` and
//! `rule:statements/an-isolate-has-its-own-statics`, from outside the process: the built
//! `nvs serve` over a program that writes a class static from the query, and
//! the value the next request reads before it writes its own. And
//! `rule:packaging/a-guest-runs-under-the-requests-budget`: a guest that loops
//! forever reaches the request's limit handler, and the next request is served.
//! And `rule:http-server/admission-is-arithmetic-not-a-number` under overload: a
//! flood past `max_in_flight` beside requests that reach their limits or lose
//! their client, and a server that gives every place and byte back once it ends. And
//! `rule:packaging/a-fresh-instance-per-request`: an extension instance ends with its request, so
//! requests past the process's instance places are all served. And
//! `rule:http-server/four-idle-waits-all-finite`: a program that works past the
//! body wait once its body is read is still answered.
//!
//! [`Server`] starts the binary this test was built beside on a port the
//! platform chose and reads the port back from the `listening on` line, as
//! `live_edit.rs` does. Requests go over HTTP/1.1 keep-alive connections, so
//! one connection carries many requests and the body is framed by
//! `Content-Length` or by chunks. Several connections run at once, each on a
//! thread of its own, against one worker and against several.
//!
//! The program lives under `CARGO_TARGET_TMPDIR`, inside the build directory,
//! and is removed when its [`Server`] is dropped.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How long one answer has to arrive before the case fails.
const BOUND: Duration = Duration::from_secs(30);

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// The program every case serves. It prints the static as it found it, then
/// the value it wrote from the query.
const PROGRAM: &str = "<?nvs\n\
class Seen {\n    public static int $value = 0;\n}\n\
echo Seen::$value, \" \";\n\
Seen::$value = Core\\Request::query(\"v\") as int;\n\
echo Seen::$value;\n";

/// Connections opened at once against each server.
const CONNECTIONS: u32 = 8;

/// Requests sent over each connection, one after another.
const PER_CONNECTION: u32 = 25;

/// A running `nvs serve` over [`PROGRAM`] in its own directory.
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
    /// Writes [`PROGRAM`] and an `nvs.toml` in production mode with `workers`
    /// accept loops into a fresh directory named for `case`, and starts `nvs
    /// serve app.nvs` there, on a free loopback port.
    ///
    /// # Panics
    ///
    /// When the process cannot start, or does not print its `listening on`
    /// line within [`BOOT`]; the message carries what it wrote to standard
    /// error.
    fn start(case: &str, workers: u32) -> Self {
        Self::serving(case, workers, PROGRAM, "", &[])
    }

    /// Writes `program` as `app.nvs`, each of `files`, and an `nvs.toml` in production mode
    /// with `workers` accept loops followed by `config`, into a fresh directory named for
    /// `case`, and starts `nvs serve app.nvs` there, on a free loopback port.
    ///
    /// # Panics
    ///
    /// As [`Server::start`] does.
    fn serving(
        case: &str,
        workers: u32,
        program: &str,
        config: &str,
        files: &[(&str, &[u8])],
    ) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("serve-isolation-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        write_file(&dir.join("app.nvs"), program);
        write_file(
            &dir.join("nvs.toml"),
            &format!(
                "[mode]\ndefault = \"production\"\n\n[server]\nworkers = {workers}\n\n{config}"
            ),
        );
        for (name, bytes) in files {
            let path = dir.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("the file's folder is created");
            }
            std::fs::write(path, bytes).expect("the case's file is written");
        }
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
        let collected = Arc::clone(&stderr);
        thread::spawn(move || {
            for line in BufReader::new(out_pipe).lines().map_while(Result::ok) {
                {
                    let mut all = collected.lock().expect("no reader panicked");
                    all.push_str(&line);
                    all.push('\n');
                }
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

    /// Opens [`CONNECTIONS`] keep-alive connections at once and sends
    /// [`PER_CONNECTION`] requests over each, every one with a value no other
    /// request sends. Returns each request's value beside the body it got.
    fn exchange(&self) -> Vec<(u32, String)> {
        let threads: Vec<_> = (0..CONNECTIONS)
            .map(|connection| {
                let addr = self.addr;
                thread::spawn(move || {
                    let mut client = Client::connect(addr);
                    (1..=PER_CONNECTION)
                        .map(|request| {
                            let value = connection * 1000 + request;
                            (value, client.get(&format!("/?v={value}")))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        threads
            .into_iter()
            .flat_map(|thread| thread.join().expect("no connection panicked"))
            .collect()
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

/// One HTTP/1.1 connection that stays open between requests.
struct Client {
    reader: BufReader<TcpStream>,
}

impl Client {
    /// # Panics
    ///
    /// When the server does not accept the connection.
    fn connect(addr: SocketAddr) -> Self {
        let stream = TcpStream::connect(addr)
            .unwrap_or_else(|error| panic!("{addr} does not answer: {error}"));
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        Self {
            reader: BufReader::new(stream),
        }
    }

    /// One `GET` of `path` on this connection. Returns the body of a `200`.
    ///
    /// # Panics
    ///
    /// When the answer is not a `200`, is cut off, or closes the connection.
    fn get(&mut self, path: &str) -> String {
        let (status, body, closes, _) = self.answer(path);
        assert!(
            status.split_whitespace().nth(1) == Some("200"),
            "`{path}` answered `{status}`"
        );
        assert!(
            !closes,
            "`{path}` closed the connection, so the next request would not reuse it"
        );
        body
    }

    /// One `GET` of `path` on this connection, whatever its status. Returns the status line,
    /// the body, whether the answer closes the connection, and its `Retry-After` header.
    ///
    /// # Panics
    ///
    /// When the answer is cut off, or does not arrive within [`BOUND`].
    fn answer(&mut self, path: &str) -> (String, String, bool, Option<String>) {
        write!(
            self.reader.get_mut(),
            "GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n"
        )
        .expect("the request is sent");
        let status = self.line(path);
        let mut closes = false;
        let mut length = None;
        let mut chunked = false;
        let mut retry_after = None;
        loop {
            let header = self.line(path);
            if header.is_empty() {
                break;
            }
            let (name, value) = header.split_once(':').unwrap_or((&header, ""));
            let (name, value) = (name.to_ascii_lowercase(), value.trim());
            if name == "content-length" {
                length = value.parse::<usize>().ok();
            } else if name == "transfer-encoding" && value.eq_ignore_ascii_case("chunked") {
                chunked = true;
            } else if name == "connection" && value.eq_ignore_ascii_case("close") {
                closes = true;
            } else if name == "retry-after" {
                retry_after = Some(value.to_owned());
            }
        }
        let mut body = Vec::new();
        if chunked {
            loop {
                let size = self.line(path);
                let size = usize::from_str_radix(size.split(';').next().unwrap_or("").trim(), 16)
                    .unwrap_or_else(|error| panic!("`{size}` is not a chunk size: {error}"));
                if size == 0 {
                    while !self.line(path).is_empty() {}
                    break;
                }
                self.exact(path, size, &mut body);
                self.line(path);
            }
        } else {
            let length = length.unwrap_or_else(|| panic!("the answer to `{path}` has no framing"));
            self.exact(path, length, &mut body);
        }
        let body = String::from_utf8(body).expect("the body is UTF-8");
        (status, body, closes, retry_after)
    }

    /// One line of the answer to `path`, without its line ending.
    fn line(&mut self, path: &str) -> String {
        let mut line = String::new();
        let read = self
            .reader
            .read_line(&mut line)
            .unwrap_or_else(|error| panic!("the answer to `{path}` was cut off: {error}"));
        assert!(
            read > 0,
            "the connection closed while `{path}` was answered"
        );
        line.trim_end_matches(['\r', '\n']).to_owned()
    }

    /// Appends exactly `count` bytes of the answer to `path` to `body`.
    fn exact(&mut self, path: &str, count: usize, body: &mut Vec<u8>) {
        let mut buffer = vec![0; count];
        self.reader
            .read_exact(&mut buffer)
            .unwrap_or_else(|error| panic!("the body of `{path}` was cut off: {error}"));
        body.extend_from_slice(&buffer);
    }
}

fn write_file(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap_or_else(|error| {
        panic!("`{}` could not be written: {error}", path.display());
    });
}

/// Every answer shows the static at its initial `0`, then its own value. A
/// value another request wrote, on this connection or any other, never shows.
fn assert_isolated(server: &Server) {
    for (value, body) in server.exchange() {
        assert_eq!(
            body,
            format!("0 {value}"),
            "the request that sent {value} saw another request's static; the server wrote: {}",
            server.said()
        );
    }
}

#[test]
fn a_static_set_by_one_request_is_not_seen_by_the_next() {
    assert_isolated(&Server::start("one-worker", 1));
    assert_isolated(&Server::start("four-workers", 4));
}

/// An extension `Shop\Spin` whose one method, `forever`, loops and never returns, as a core
/// module the packer componentizes.
const SPIN: &str = r#"(module
  (memory (export "memory") 1)
  (func (export "shop:spin/api#forever") (result i32)
    (loop $again (br $again))
    (i32.const 0)))"#;

const SPIN_WIT: &str = "package shop:spin;\n\ninterface api {\n    use nvs:ext/types@1.0.0.{error};\n\n    forever: func() -> result<_, error>;\n}\n";

const SPIN_MANIFEST: &str = r#"{"manifest": 1, "world": "1.0.0", "class": "Shop\\Spin", "interface": "shop:spin/api",
  "methods": [{"name": "forever", "help": "Loops and never returns."}]}"#;

/// A program that calls the looping guest when the query asks for it, and greets otherwise.
const SPINNING: &str = "<?nvs\n\
Core\\Fatal::onLimit(fn (array<string> $report): void => {\n    \
Core\\IO::write('marks/limit.txt', $report['limit']);\n});\n\
if (Core\\Request::query(\"spin\") == \"yes\") {\n    Shop\\Spin::forever();\n}\n\
echo \"served\";\n";

/// A guest that loops forever runs under the request's own `[limits] cpu_time`. The request
/// reaches its `Core\Fatal::onLimit` handler and ends, and the same worker serves the next
/// request (`rule:packaging/a-guest-runs-under-the-requests-budget`).
#[test]
fn a_looping_guest_reaches_on_limit_and_the_next_request_is_served() {
    let nvsx = nvs_ext::pack::pack(&nvs_ext::pack::Inputs {
        wasm: &wat::parse_str(SPIN).expect("the module's text parses"),
        wit: &[("api.wit", SPIN_WIT)],
        manifest: SPIN_MANIFEST.as_bytes(),
        files: &[],
    })
    .expect("the module packs");
    let config = format!(
        "[limits]\ncpu_time = \"300ms\"\n\n[capabilities.fs]\nwrite = [\"marks/\"]\n\n[[extension]]\npath = 'spin.nvsx'\nsha256 = \"{}\"\n",
        nvs_ext::load::pin(&nvsx)
    );
    let server = Server::serving(
        "looping-guest",
        1,
        SPINNING,
        &config,
        &[("spin.nvsx", &nvsx), ("marks/.keep", b"")],
    );

    let (status, _, _, _) = Client::connect(server.addr).answer("/?spin=yes");
    assert!(
        status.split_whitespace().nth(1) == Some("500"),
        "the looping request answered `{status}`; the server wrote: {}",
        server.said()
    );
    let mark = server.dir.join("marks").join("limit.txt");
    assert_eq!(
        std::fs::read_to_string(&mark).ok().as_deref(),
        Some("cpu_time"),
        "the limit handler did not run; the server wrote: {}",
        server.said()
    );
    assert_eq!(Client::connect(server.addr).get("/"), "served");
}

/// The program the overload case serves. `?do=hold` waits 300 ms, `?do=grow` allocates until it
/// reaches `[limits] memory`, `?do=spin` loops until it reaches `[limits] cpu_time`, and anything
/// else answers `ok` at once.
const OVERLOADED: &str = "<?nvs\n\
?tainted string $what = Core\\Request::query(\"do\");\n\
if ($what == \"hold\") {\n    Core\\Time::sleep(Core\\Time\\Duration::milliseconds(300));\n\
} elseif ($what == \"grow\") {\n    string $chunk = \"x\";\n    \
for (int $i = 0; $i < 16; $i++) {\n        $chunk = $chunk . $chunk;\n    }\n    \
array<string> $parts = [];\n    while (true) {\n        $parts[] = $chunk . \"y\";\n    }\n\
} elseif ($what == \"spin\") {\n    int $turns = 0;\n    while (true) {\n        $turns = $turns + 1;\n    }\n\
}\n\
echo \"ok\";\n";

/// The in-flight ceiling the overload case serves under.
const CEILING: u32 = 50;

/// The accept loops the overload case serves with.
const WORKERS: u32 = 2;

/// Connections the flood opens at once, four times [`CEILING`]. Each sends [`FLOOD_ROUNDS`]
/// holds, one after another, each on a connection of its own.
const FLOOD: u32 = 4 * CEILING;

/// Holds each flood connection sends.
const FLOOD_ROUNDS: u32 = 3;

/// Requests of each kind sent beside the flood: those that reach the memory limit, those that
/// reach the CPU limit, and those whose client leaves before the answer.
const BESIDE: u32 = 10;

/// How long a probe may wait for its answer. Longer is a hang.
const PROBE_BOUND: Duration = Duration::from_secs(10);

/// How far `nvs_memory_bytes{scope="request"}` may stand above its reading before the flood once
/// the flood has ended: two mebibytes per worker. A core publishes its balance a mebibyte at a
/// time, so its gauge is stale by up to one, and the second covers what a core keeps in its pools.
const SETTLED: f64 = 2.0 * WORKERS as f64 * 1_048_576.0;

/// A program that reads its body, works for longer than the body wait below,
/// and then answers with the body's length.
const READS_THEN_WORKS: &str = "<?nvs\n\
tainted string $sent = Core\\Request::body();\n\
Core\\Time::sleep(1s);\n\
echo 'read ', Core\\Str::length($sent);\n";

/// `rule:http-server/four-idle-waits-all-finite`'s body wait ends with the
/// body: a program that has read its body, or had none, and works for five
/// times `body_idle_timeout` is still answered. The client waits on the reply
/// itself, so a connection the body wait closes reads as an empty answer.
#[test]
fn a_program_that_works_past_the_body_wait_after_its_body_is_answered() {
    let server = Server::serving(
        "body-wait",
        1,
        READS_THEN_WORKS,
        "body_idle_timeout = \"200ms\"\n",
        &[],
    );
    for request in [
        "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
        "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    ] {
        let mut stream = TcpStream::connect(server.addr)
            .unwrap_or_else(|error| panic!("{} does not answer: {error}", server.addr));
        stream
            .set_read_timeout(Some(BOUND))
            .expect("a read timeout is set");
        stream
            .write_all(request.as_bytes())
            .expect("the request is sent");
        let mut raw = Vec::new();
        let read = stream.read_to_end(&mut raw);
        let text = String::from_utf8_lossy(&raw);
        assert!(
            read.is_ok() && text.starts_with("HTTP/1.1 200 OK\r\n"),
            "a program that read its body and then worked was not answered: {read:?} {text:?}\n\
             the server wrote: {}",
            server.said()
        );
        let length = if request.starts_with("POST") { 5 } else { 0 };
        assert!(
            text.ends_with(&format!("read {length}")),
            "the program answered something else: {text}"
        );
    }
}

/// A loopback port nothing listens on at the moment it is asked.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("a loopback port is free")
        .port()
}

/// One `GET` of `path` on a connection of its own. Returns the status code and whether the answer
/// carried `Retry-After`.
fn once(addr: SocketAddr, path: &str) -> (String, bool) {
    let (status, _, _, retry_after) = Client::connect(addr).answer(path);
    let code = status
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_owned();
    (code, retry_after.is_some())
}

/// `nvs_memory_bytes{scope="request"}` as a scrape of `port` reads it. The scrape port may open a
/// moment after the server's own, so a refused connection is tried again for up to [`BOOT`].
fn request_memory(port: u16, server: &Server) -> f64 {
    let started = Instant::now();
    let mut stream = loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(stream) => break stream,
            Err(error) => assert!(
                started.elapsed() <= BOOT,
                "the scrape port {port} does not answer: {error}; the server wrote: {}",
                server.said()
            ),
        }
        thread::sleep(Duration::from_millis(50));
    };
    stream
        .set_read_timeout(Some(BOUND))
        .expect("a read timeout is set");
    stream
        .write_all(b"GET /metrics HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .expect("the scrape is sent");
    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .expect("the scrape is answered");
    raw.lines()
        .find_map(|line| line.strip_prefix("nvs_memory_bytes{scope=\"request\"} "))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or_else(|| panic!("the scrape has no request memory: {raw}"))
}

/// A flood of valid requests four times `max_in_flight` runs beside requests that reach the memory
/// limit, reach the CPU limit, and lose their client, while a probe sends ordinary requests. Every
/// refusal is a `503` with `Retry-After`, every probe is answered within [`PROBE_BOUND`], and the
/// process lives. Once the flood ends, the request memory comes back within [`SETTLED`] of its
/// reading before it, [`CEILING`] requests held open together are all admitted, and the next
/// request answers `200` (`rule:http-server/admission-is-arithmetic-not-a-number`).
#[test]
fn a_flood_past_capacity_is_refused_and_every_place_and_byte_comes_back() {
    let port = free_port();
    let config = format!(
        "max_in_flight = {CEILING}\n\n[limits]\nmemory = \"24M\"\ncpu_time = \"200ms\"\n\n\
         [metrics]\nexporter = \"prometheus\"\nlisten = \"127.0.0.1:{port}\"\n"
    );
    let mut server = Server::serving("overload", WORKERS, OVERLOADED, &config, &[]);
    let addr = server.addr;

    // Each path runs once first, so what it compiles and pools is held before the reading.
    for path in ["/", "/?do=hold", "/?do=grow", "/?do=spin"] {
        once(addr, path);
    }
    let before = request_memory(port, &server);

    let flooding = Arc::new(AtomicBool::new(true));
    let probe = {
        let flooding = Arc::clone(&flooding);
        thread::spawn(move || {
            let mut probes = 0_u32;
            while flooding.load(Ordering::Relaxed) {
                let started = Instant::now();
                let (code, retry_after) = once(addr, "/");
                assert!(
                    started.elapsed() <= PROBE_BOUND,
                    "a probe waited {:?} for its answer",
                    started.elapsed()
                );
                assert!(
                    code == "200" || (code == "503" && retry_after),
                    "a probe answered `{code}`, with Retry-After: {retry_after}"
                );
                probes += 1;
                thread::sleep(Duration::from_millis(20));
            }
            probes
        })
    };

    let refused = Arc::new(AtomicU32::new(0));
    let mut threads = Vec::new();
    for _ in 0..FLOOD {
        let refused = Arc::clone(&refused);
        threads.push(thread::spawn(move || {
            for _ in 0..FLOOD_ROUNDS {
                match once(addr, "/?do=hold") {
                    (code, _) if code == "200" => {}
                    (code, true) if code == "503" => {
                        refused.fetch_add(1, Ordering::Relaxed);
                    }
                    (code, retry_after) => {
                        panic!("a hold answered `{code}`, with Retry-After: {retry_after}")
                    }
                }
            }
        }));
    }
    for path in ["/?do=grow", "/?do=spin"] {
        for _ in 0..BESIDE {
            threads.push(thread::spawn(move || {
                let (code, retry_after) = once(addr, path);
                assert!(
                    code == "500" || (code == "503" && retry_after),
                    "`{path}` answered `{code}`, with Retry-After: {retry_after}"
                );
            }));
        }
    }
    for _ in 0..BESIDE {
        threads.push(thread::spawn(move || {
            // The client sends a hold and leaves before the answer.
            let mut stream = TcpStream::connect(addr).expect("the server accepts");
            stream
                .write_all(b"GET /?do=hold HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .expect("the request is sent");
        }));
    }
    for thread in threads {
        thread.join().expect("no flood connection failed");
    }
    flooding.store(false, Ordering::Relaxed);
    let probes = probe.join().expect("no probe failed");
    assert!(probes > 0, "no probe ran during the flood");
    assert!(
        refused.load(Ordering::Relaxed) > 0,
        "no hold was refused, so the flood never passed the ceiling"
    );
    assert!(
        server
            .child
            .try_wait()
            .expect("the process is asked")
            .is_none(),
        "the server ended during the flood; it wrote: {}",
        server.said()
    );

    let started = Instant::now();
    loop {
        let now = request_memory(port, &server);
        if now <= before + SETTLED {
            break;
        }
        assert!(
            started.elapsed() <= BOUND,
            "the request memory stayed at {now} bytes, more than {SETTLED} above {before}; \
             the server wrote: {}",
            server.said()
        );
        thread::sleep(Duration::from_millis(50));
    }

    // The ceiling's requests are sent together, once every connection is open.
    let together = Arc::new(Barrier::new(CEILING as usize));
    let held: Vec<_> = (0..CEILING)
        .map(|_| {
            let together = Arc::clone(&together);
            thread::spawn(move || {
                let mut client = Client::connect(addr);
                together.wait();
                client.answer("/?do=hold").0
            })
        })
        .collect();
    for thread in held {
        let status = thread.join().expect("no held request failed");
        assert!(
            status.split_whitespace().nth(1) == Some("200"),
            "a request within the ceiling answered `{status}` after the flood; the server wrote: {}",
            server.said()
        );
    }
    assert_eq!(Client::connect(addr).get("/"), "ok");
    assert!(
        server
            .child
            .try_wait()
            .expect("the process is asked")
            .is_none(),
        "the server ended after the flood; it wrote: {}",
        server.said()
    );
}

/// The program the instance case serves: one call into the built-in `Novis\Intl` component.
const LISTING: &str = "<?nvs\n\
use Novis\\Intl\\ListFormat;\n\
echo ListFormat::join([[\"Shop\", \"Blog\", \"Wiki\"]], \"en\")[0];\n";

/// Requests the instance case sends on each kind of connection. Together they are past the
/// process's 64 instance places three times over.
const LISTINGS: u32 = 100;

/// How far `nvs_memory_bytes{scope="request"}` may move between the reading after a few requests
/// and the reading after all of them. One instance of the component is several mebibytes, so a
/// kept instance per request moves it by far more.
const LISTED: f64 = 4.0 * 1_048_576.0;

/// Every request that calls an extension gets a fresh instance, and the instance ends with the
/// request (`rule:packaging/a-fresh-instance-per-request`). 100 requests on one keep-alive
/// connection and 100 more each on a connection of its own all answer correctly, past the
/// process's 64 instance places, and the request memory stays where it was after the first few.
#[test]
fn every_request_that_calls_an_extension_ends_its_instance() {
    let port = free_port();
    let config = format!("[metrics]\nexporter = \"prometheus\"\nlisten = \"127.0.0.1:{port}\"\n");
    let server = Server::serving("instances", 1, LISTING, &config, &[]);
    let want = "Shop, Blog, and Wiki";

    let mut client = Client::connect(server.addr);
    for _ in 0..4 {
        assert_eq!(client.get("/"), want, "the server wrote: {}", server.said());
    }
    let before = request_memory(port, &server);
    for request in 4..LISTINGS {
        assert_eq!(
            client.get("/"),
            want,
            "request {request} on one connection; the server wrote: {}",
            server.said()
        );
    }
    drop(client);
    for request in 0..LISTINGS {
        assert_eq!(
            Client::connect(server.addr).get("/"),
            want,
            "request {request} on a connection of its own; the server wrote: {}",
            server.said()
        );
    }
    let after = request_memory(port, &server);
    assert!(
        after - before <= LISTED,
        "the request memory went from {before} to {after} bytes over {} requests",
        2 * LISTINGS
    );
}

/// The program the held-instance case serves: one call into `Novis\Intl`, then a wait, so the
/// request keeps its instance while the others start.
const LISTING_HELD: &str = "<?nvs\n\
use Novis\\Intl\\ListFormat;\n\
var $listed = ListFormat::join([[\"Shop\", \"Blog\", \"Wiki\"]], \"en\")[0];\n\
Core\\Time::sleep(Core\\Time\\Duration::milliseconds(500));\n\
echo $listed;\n";

/// Requests the held-instance case keeps in flight together: past the 64 instances the host once
/// had room for, and under its `max_in_flight`.
const HELD_LISTINGS: usize = 100;

/// Every request in flight can hold an extension instance at once, because the host's pool is
/// sized from `max_in_flight` (`rule:packaging/a-fresh-instance-per-request`). 100 requests that
/// each keep an instance while the others start all answer correctly.
#[test]
fn every_request_in_flight_can_hold_an_extension_instance() {
    let server = Server::serving(
        "held-instances",
        2,
        LISTING_HELD,
        "max_in_flight = 200\n",
        &[],
    );
    let addr = server.addr;
    let threads: Vec<_> = (0..HELD_LISTINGS)
        .map(|_| thread::spawn(move || Client::connect(addr).get("/")))
        .collect();
    for (request, thread) in threads.into_iter().enumerate() {
        assert_eq!(
            thread.join().expect("the client thread panicked"),
            "Shop, Blog, and Wiki",
            "request {request} of {HELD_LISTINGS} in flight together; the server wrote: {}",
            server.said()
        );
    }
}
