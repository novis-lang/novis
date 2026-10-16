//! `rule:concurrency/cross-request-state-is-explicit` and
//! `rule:statements/an-isolate-has-its-own-statics`, from outside the process: the built
//! `nvs serve` over a program that writes a class static from the query, and
//! the value the next request reads before it writes its own. And
//! `rule:packaging/a-guest-runs-under-the-requests-budget`: a guest that loops
//! forever reaches the request's limit handler, and the next request is served.
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
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
        let (status, body, closes) = self.answer(path);
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
    /// the body, and whether the answer closes the connection.
    ///
    /// # Panics
    ///
    /// When the answer is cut off, or does not arrive within [`BOUND`].
    fn answer(&mut self, path: &str) -> (String, String, bool) {
        write!(
            self.reader.get_mut(),
            "GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n"
        )
        .expect("the request is sent");
        let status = self.line(path);
        let mut closes = false;
        let mut length = None;
        let mut chunked = false;
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
        (status, body, closes)
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

    let (status, _, _) = Client::connect(server.addr).answer("/?spin=yes");
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
