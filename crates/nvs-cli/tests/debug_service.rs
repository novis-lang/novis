//! A development `nvs serve` with `[debug] store` on starts `nvs debug`, and its requests reach the
//! index (`rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it`,
//! `rule:testing/writing-a-debug-record-to-disk-is-its-own-gate`).
//!
//! Each case serves one program from a directory and a data folder of its own. The program dumps a
//! value before it answers, and a dump with `store` on is written to the spool as it is made, so
//! the spool folder exists before the answer is sent. That is what makes "nothing was written"
//! something a case can check once its answer has arrived. The server says on standard error that
//! it started the service, before it binds a socket, so a case reads the whole of standard error
//! once the server and its child have ended.
//!
//! A server runs no probe (`rule:testing/debug-mode-directive`), and `[debug] store` sets no probe
//! bit: `Ctx::records_spans` is the one place a run reads it. So with `store` off, what a served
//! case can compare with a run that writes no `[debug]` block is its answer and its data folder.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use rusqlite::{Connection, OpenFlags};

/// Dumps a value, then answers with the path it was asked for.
const PROGRAM: &str = "<?nvs\nCore\\Debug::dump(['order' => 1042]);\n\
                       echo Core\\Request::path();\n";

const DEVELOPMENT: &str = "[mode]\ndefault = \"development\"\n";

/// What the server printed when it started the service.
const STARTED: &str = "`nvs debug` reads this server's recorded requests into";

/// How long a case waits for something the server or the service does.
const PATIENCE: Duration = Duration::from_secs(60);

/// A running `nvs serve app.nvs` in a directory named for the case.
struct Served {
    child: Child,
    addr: SocketAddr,
    dir: PathBuf,
    data: nvs_repo::Scratch,
    /// Everything the server and its child write to standard error, once both have closed it.
    stderr: mpsc::Receiver<String>,
}

impl Served {
    /// Starts [`PROGRAM`] on a free loopback port, with `config` as the directory's `nvs.toml`.
    fn start(case: &str, config: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("debug-service-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        std::fs::write(dir.join("app.nvs"), PROGRAM).expect("the program is written");
        std::fs::write(dir.join("nvs.toml"), config).expect("the configuration is written");
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
        let out = child.stdout.take().expect("standard output is piped");
        let (sent, bound) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if let Some(rest) = line.strip_prefix("listening on http://") {
                    let addr = rest.split_whitespace().next().unwrap_or_default();
                    let _ = sent.send(addr.to_owned());
                }
            }
        });
        let mut err = child.stderr.take().expect("standard error is piped");
        let (said, stderr) = mpsc::channel();
        thread::spawn(move || {
            let mut text = String::new();
            let _ = err.read_to_string(&mut text);
            let _ = said.send(text);
        });
        let Ok(addr) = bound.recv_timeout(PATIENCE) else {
            let _ = child.kill();
            let _ = child.wait();
            panic!("`nvs serve` did not start listening");
        };
        Self {
            child,
            addr: addr.parse().expect("the server printed an address"),
            dir,
            data,
            stderr,
        }
    }

    /// The whole answer to `GET path`, which closes the connection.
    fn get(&self, path: &str) -> String {
        let mut stream = TcpStream::connect(self.addr).expect("the server answers");
        stream
            .set_read_timeout(Some(PATIENCE))
            .expect("a read timeout is set");
        let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        stream
            .write_all(request.as_bytes())
            .expect("the request is sent");
        let mut answer = Vec::new();
        stream.read_to_end(&mut answer).expect("an answer arrives");
        String::from_utf8_lossy(&answer).into_owned()
    }

    fn debug(&self) -> PathBuf {
        self.data.join("debug")
    }

    /// Kills the server, and returns its standard error once every process holding it has ended.
    fn stop(&mut self) -> String {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.stderr
            .recv_timeout(PATIENCE)
            .expect("standard error closes once the server and its child have ended")
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = self.stderr.recv_timeout(PATIENCE);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// What the program echoed, before the dump `development` renders into the page.
fn echoed(answer: &str) -> &str {
    let (_, body) = answer.split_once("\r\n\r\n").expect("an answer has a head");
    body.split('<').next().unwrap_or_default()
}

/// The trace id in an answer's `traceresponse` header.
fn trace_of(answer: &str) -> String {
    answer
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("traceresponse")
                .then(|| value.trim().split('-').nth(1).map(str::to_owned))
                .flatten()
        })
        .unwrap_or_else(|| panic!("no traceresponse header: {answer}"))
}

/// The lock on `path`, once nothing else holds it.
fn locked(path: &Path) -> File {
    let until = Instant::now() + PATIENCE;
    loop {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .expect("the lock file opens");
        if file.try_lock().is_ok() {
            return file;
        }
        assert!(Instant::now() < until, "`{}` is still held", path.display());
        thread::sleep(Duration::from_millis(20));
    }
}

/// `query` over the index, once it answers `want`.
fn index_answers(index: &Path, query: &str, want: i64) {
    let until = Instant::now() + PATIENCE;
    loop {
        let read = Connection::open_with_flags(index, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .and_then(|db| db.query_row(query, [], |row| row.get::<_, i64>(0)));
        if read.as_ref().is_ok_and(|got| *got == want) {
            return;
        }
        assert!(
            Instant::now() < until,
            "`{query}` answers {read:?}, not {want}"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

/// The `.spool` files in `spool`, which are the sealed ones.
fn sealed(spool: &Path) -> usize {
    std::fs::read_dir(spool)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "spool"))
        .count()
}

/// A development server with `store` on starts the service. Each request reaches the index under
/// the trace id its answer carried, with its input and its dump, and every sealed spool file is
/// read and deleted. Once the server is killed, the service ends and frees its lock.
#[test]
fn a_development_server_starts_the_service_and_its_requests_reach_the_index() {
    let mut served = Served::start(
        "development",
        &format!("{DEVELOPMENT}\n[debug]\nstore = true\n"),
    );
    let traces: Vec<String> = ["/first", "/second"]
        .iter()
        .map(|path| {
            let answer = served.get(path);
            assert_eq!(echoed(&answer), *path, "{answer}");
            trace_of(&answer)
        })
        .collect();
    let index = served.debug().join("index.sqlite");
    for (trace, path) in traces.iter().zip(["/first", "/second"]) {
        index_answers(
            &index,
            &format!(
                "SELECT count(*) FROM requests r JOIN inputs i ON i.request = r.id \
                 WHERE r.trace = '{trace}' AND i.trace = '{trace}' AND r.path = '{path}' \
                 AND r.status = 200 AND r.dumps = 1"
            ),
            1,
        );
        index_answers(
            &index,
            &format!("SELECT count(*) FROM dumps WHERE trace = '{trace}'"),
            1,
        );
    }
    let spool = served.debug().join("spool");
    let until = Instant::now() + PATIENCE;
    while sealed(&spool) > 0 {
        assert!(Instant::now() < until, "a sealed spool file was never read");
        thread::sleep(Duration::from_millis(50));
    }

    let stderr = served.stop();
    assert!(stderr.contains(STARTED), "{stderr}");
    drop(locked(&served.debug().join("service.lock")));
}

/// A production server never starts the service, though `store` on records its requests.
#[test]
fn a_production_server_records_and_starts_no_service() {
    let mut served = Served::start(
        "production",
        "[mode]\ndefault = \"production\"\n\n[debug]\nstore = true\n",
    );
    let answer = served.get("/order");
    assert_eq!(echoed(&answer), "/order", "{answer}");
    assert!(
        served.debug().join("spool").is_dir(),
        "the dump reached the spool"
    );
    let stderr = served.stop();
    assert!(!stderr.contains(STARTED), "{stderr}");
    assert!(!served.debug().join("index.sqlite").exists());
    assert!(!served.debug().join("service.lock").exists());
}

/// With `store` off, a development server writes nothing to the data folder's `debug` folder, and
/// answers as a server whose configuration has no `[debug]` block.
#[test]
fn with_store_off_nothing_is_written_and_the_answer_is_a_run_without_debug() {
    // The status line, each header's name and what the program echoed. A header's value and the
    // page's nonce differ on every run.
    let body_of = |answer: &str| {
        let (head, _) = answer.split_once("\r\n\r\n").expect("an answer has a head");
        let mut lines = head.lines();
        let status = lines.next().unwrap_or_default().to_owned();
        let names: Vec<String> = lines
            .filter_map(|line| line.split_once(':').map(|(name, _)| name.to_owned()))
            .collect();
        (status, names, echoed(answer).to_owned())
    };
    let mut answers = Vec::new();
    for (case, config) in [
        ("store-off", format!("{DEVELOPMENT}\n[debug]\nstore = false\n")),
        ("no-debug", DEVELOPMENT.to_owned()),
    ] {
        let mut served = Served::start(case, &config);
        let answer = served.get("/order");
        assert!(
            !served.debug().exists(),
            "{case}: the `debug` folder was created"
        );
        let stderr = served.stop();
        assert!(!stderr.contains(STARTED), "{case}: {stderr}");
        answers.push(body_of(&answer));
    }
    assert_eq!(answers[0], answers[1]);
}
