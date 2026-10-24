//! `rule:tooling/the-telemetry-collector-is-a-novis-program` end to end: the real client against
//! the real collector, on one machine.
//!
//! The test copies `apps/telemetry/` into a scratch folder, so its database is a fresh
//! `telemetry.sqlite` there, and adds `[log] target = "stderr"` and `access = true` to the copy's
//! `nvs.toml`, so every line the server writes reaches the test. It starts `nvs serve` on a free
//! loopback port and runs `nvs telemetry upload` with `NVS_TELEMETRY_URL` at a relay in front of
//! the server, which keeps the bytes the client sent.
//!
//! Last week's report is accepted with `204`, and `/data/totals.json` and `/` show it, and the
//! footer of `/` names the commit in the copy's `REVISION`. The app's `[capabilities.fs] read`
//! grants both files a request reads outside `public/`, and only a request under `nvs serve`
//! is held to that list's default, so this is the test that fails on a missing grant. A second
//! week waiting in the same week is not sent, and a week the collector refuses with `400` is
//! dropped. The bytes the relay saw are the ones `nvs telemetry show` printed, and `127.0.0.1`
//! is in no file of the copy and no line the server wrote.
//!
//! Every run removes `CI` and `DO_NOT_TRACK`, which turn telemetry off and which a CI machine
//! sets.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// How long the server has to write the log line of a request it answered.
const LOGGED: Duration = Duration::from_secs(10);

/// The seconds in one day.
const DAY: i64 = 86_400;

/// The commit the copy's `REVISION` names, which the footer of `/` shows.
const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

/// The files of the application the copy needs.
const APP: [&str; 4] = ["public", "src", "known.toml", "nvs.toml"];

/// `nvs` with `args`, the data folder `data`, the environment `env` and no `CI` or
/// `DO_NOT_TRACK`, run in `dir`.
fn nvs(data: &Path, dir: &Path, env: &[(&str, &str)], args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    command
        .arg("--data")
        .arg(data)
        .args(args)
        .current_dir(dir)
        .env_remove("CI")
        .env_remove("DO_NOT_TRACK")
        .envs(env.iter().copied())
        .stdin(Stdio::null());
    command
}

/// What `command` printed, once it exited `0`.
fn ran(mut command: Command) -> String {
    let output: Output = command.output().expect("the `nvs` binary runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("text")
}

/// The proleptic Gregorian date of `days` since 1970-01-01, as year, month and day.
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The ISO week label, such as `2026-W42`, of the week that starts on the Monday `monday`, in
/// days since 1970-01-01.
fn label(monday: i64) -> String {
    // The Thursday of an ISO week is always in the week's own year.
    let thursday = monday + 3;
    let (year, _, _) = civil(thursday);
    let mut january = thursday;
    while civil(january - 1).0 == year {
        january -= 1;
    }
    format!("{year}-W{:02}", (thursday - january) / 7 + 1)
}

/// The Monday of the current UTC week, in days since 1970-01-01.
fn this_monday() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("a clock after 1970")
        .as_secs();
    let today = i64::try_from(now).expect("a time in range") / DAY;
    // 1970-01-01 was a Thursday, so a Monday is 3 days after a multiple of 7.
    today - (today + 3).rem_euclid(7)
}

/// A counter file with nothing uploaded and the finished week `week` waiting, due at `due`
/// seconds since 1970.
fn waiting(week: &str, due: i64) -> String {
    format!(
        "uploads 0\nbytes 0\ndue {due}\npending {week}\ndays 1110000\ninvocation.run 41\ninvocation.serve 2\n"
    )
}

/// A private data folder with telemetry on and `counters` as its counter file.
fn client(counters: &str) -> nvs_repo::Scratch {
    let data = nvs_repo::scratch_private("telemetry-e2e-client");
    let _ = ran(nvs(&data, &data, &[], &["telemetry", "on"]));
    std::fs::write(data.join("telemetry").join("counters"), counters).expect("the counter file");
    data
}

/// The counter file in `data`.
fn counters(data: &Path) -> String {
    std::fs::read_to_string(data.join("telemetry").join("counters")).expect("the counter file")
}

/// Copies the file or folder `from` to `to`.
fn copy(from: &Path, to: &Path) {
    if from.is_dir() {
        std::fs::create_dir_all(to).expect("a folder of the copy");
        for entry in std::fs::read_dir(from).expect("a folder of the app") {
            let entry = entry.expect("an entry of the app");
            copy(&entry.path(), &to.join(entry.file_name()));
        }
    } else {
        std::fs::copy(from, to).expect("a file of the copy");
    }
}

/// The server, with every line it wrote so far and the address it listens on.
struct Server {
    child: Child,
    addr: String,
    lines: Arc<Mutex<Vec<String>>>,
    logged: Receiver<String>,
}

impl Server {
    /// Starts `nvs serve` on the copy of the application in `app`.
    fn start(app: &Path) -> Self {
        let data = nvs_repo::scratch_private("telemetry-e2e-server");
        let config = app.join("nvs.toml");
        let mut child = nvs(
            &data,
            app,
            &[],
            &[
                "--config",
                &config.to_string_lossy(),
                "serve",
                "public/index.nvs",
                "--listen",
                "127.0.0.1:0",
            ],
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the `nvs` binary starts");
        let lines = Arc::new(Mutex::new(Vec::new()));
        let (bound, addr) = mpsc::channel();
        let (wrote, logged) = mpsc::channel();
        let out = child.stdout.take().expect("standard output is piped");
        let err = child.stderr.take().expect("standard error is piped");
        for (stream, bound, wrote) in [
            (Box::new(out) as Box<dyn Read + Send>, Some(bound), None),
            (Box::new(err), None, Some(wrote)),
        ] {
            let lines = Arc::clone(&lines);
            thread::spawn(move || {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    if let (Some(bound), Some(rest)) =
                        (&bound, line.strip_prefix("listening on http://"))
                    {
                        let _ = bound.send(
                            rest.split_whitespace()
                                .next()
                                .unwrap_or_default()
                                .to_owned(),
                        );
                    }
                    if let Some(wrote) = &wrote {
                        let _ = wrote.send(line.clone());
                    }
                    lines.lock().expect("the lines").push(line);
                }
            });
        }
        let Ok(addr) = addr.recv_timeout(BOOT) else {
            let _ = child.kill();
            panic!(
                "`nvs serve` did not start listening within {BOOT:?}: {:?}",
                lines.lock().expect("the lines")
            );
        };
        Self {
            child,
            addr,
            lines,
            logged,
        }
    }

    /// The body of the answer to `GET path`, once its status is `200`.
    fn get(&self, path: &str) -> String {
        let mut stream = TcpStream::connect(&self.addr).expect("the server answers");
        write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost\r\n\r\n").expect("the request");
        let mut answer = String::new();
        stream.read_to_string(&mut answer).expect("the answer");
        let (head, body) = answer.split_once("\r\n\r\n").expect("a head and a body");
        assert!(
            head.starts_with("HTTP/1.1 200") || head.starts_with("HTTP/1.0 200"),
            "{head}"
        );
        body.to_owned()
    }

    /// Waits until the server wrote a line that contains `text`.
    fn wait_for(&self, text: &str) {
        loop {
            match self.logged.recv_timeout(LOGGED) {
                Ok(line) if line.contains(text) => return,
                Ok(_) => {}
                Err(_) => panic!(
                    "the server wrote no line with {text:?} within {LOGGED:?}: {:?}",
                    self.lines.lock().expect("the lines")
                ),
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A relay on loopback in front of `server` for one connection, and its URL. The thread gives
/// back the bytes of the request it relayed.
fn relay(server: &str) -> (String, JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let url = format!(
        "http://{}/v1/report",
        listener.local_addr().expect("the bound address")
    );
    let server = server.to_owned();
    let relayed = thread::spawn(move || {
        let (mut from, _) = listener.accept().expect("the client connects");
        let mut to = TcpStream::connect(&server).expect("the server answers");
        let (mut back_from, mut back_to) = (
            to.try_clone().expect("the stream"),
            from.try_clone().expect("the stream"),
        );
        let answers = thread::spawn(move || {
            let _ = std::io::copy(&mut back_from, &mut back_to);
            let _ = back_to.shutdown(Shutdown::Write);
        });
        let mut sent = Vec::new();
        let mut chunk = [0; 4096];
        while let Ok(read) = from.read(&mut chunk) {
            if read == 0 {
                break;
            }
            sent.extend_from_slice(&chunk[..read]);
            to.write_all(&chunk[..read]).expect("the relayed request");
        }
        let _ = to.shutdown(Shutdown::Write);
        answers.join().expect("the answer thread");
        sent
    });
    (url, relayed)
}

/// The body of the HTTP request `request`, by its `Content-Length`.
fn body(request: &[u8]) -> String {
    let text = String::from_utf8_lossy(request);
    let (head, rest) = text.split_once("\r\n\r\n").expect("a head and a body");
    let length: usize = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().expect("a length"))
        })
        .expect("a `Content-Length`");
    rest[..length].to_owned()
}

/// The database files in `dir`, `telemetry.sqlite` and its `-wal` and `-shm`, each with whether
/// `text` is in its bytes.
fn database(dir: &Path, text: &[u8]) -> Vec<(String, bool)> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).expect("the folder of the copy") {
        let path = entry.expect("an entry of the copy").path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if name.starts_with("telemetry.sqlite") {
            let bytes = std::fs::read(&path).expect("a database file");
            files.push((name, bytes.windows(text.len()).any(|window| window == text)));
        }
    }
    files
}

// covers: tools:cli/nvs-telemetry
#[test]
fn the_client_uploads_to_the_collector_and_the_totals_show_it() {
    let app = nvs_repo::scratch("telemetry-e2e-app");
    for name in APP {
        copy(
            &nvs_repo::path(&format!("apps/telemetry/{name}")),
            &app.join(name),
        );
    }
    let config = std::fs::read_to_string(app.join("nvs.toml")).expect("the app's `nvs.toml`");
    let config = config.replacen(
        "client_ip = false\n",
        "client_ip = false\ntarget = \"stderr\"\naccess = true\n",
        1,
    );
    assert!(config.contains("access = true"), "{config}");
    std::fs::write(app.join("nvs.toml"), config).expect("the copy's `nvs.toml`");
    std::fs::write(app.join("REVISION"), format!("{REVISION}\n")).expect("the commit");
    let server = Server::start(&app);

    // Last week ended at this Monday, so the collector takes it.
    let monday = this_monday();
    let week = label(monday - 7);
    let data = client(&waiting(&week, monday * DAY));
    let shown = ran(nvs(&data, &data, &[], &["telemetry", "show"]));
    let (url, relayed) = relay(&server.addr);
    let env = [("NVS_TELEMETRY_URL", url.as_str())];
    let out = ran(nvs(&data, &data, &env, &["telemetry", "upload"]));
    let sent = relayed.join().expect("the relay thread");
    let received = body(&sent);
    assert_eq!(
        out,
        format!("telemetry: sent week {week}, {} bytes\n", received.len()),
        "{:?}",
        server.lines.lock().expect("the lines")
    );
    assert_eq!(format!("{received}\n"), shown);
    server.wait_for("/v1/report");

    let totals: serde_json::Value =
        serde_json::from_str(&server.get("/data/totals.json")).expect("the totals as JSON");
    let reports = totals["reports"].as_array().expect("the reports");
    assert_eq!(reports.len(), 1, "{totals}");
    assert_eq!(reports[0]["week"], week.as_str(), "{totals}");
    assert_eq!(reports[0]["reports"], 1, "{totals}");
    let run = totals["counters"]
        .as_array()
        .expect("the counters")
        .iter()
        .find(|row| row["counter"] == "invocation.run")
        .unwrap_or_else(|| panic!("no `invocation.run` row: {totals}"));
    assert_eq!(run["week"], week.as_str(), "{totals}");
    assert_eq!(run["total"], 41, "{totals}");
    let home = server.get("/");
    assert!(home.contains(&week), "{home}");
    assert!(home.contains(REVISION), "{home}");

    // The upload was this week, so a second finished week waits until next week.
    let mut file = counters(&data);
    file.push_str(&format!(
        "pending {}\ndays 1000000\ninvocation.run 3\n",
        label(monday - 14)
    ));
    std::fs::write(data.join("telemetry").join("counters"), &file).expect("the counter file");
    let out = ran(nvs(&data, &data, &env, &["telemetry", "upload"]));
    assert!(
        out.starts_with(&format!(
            "telemetry: week {} is sent from ",
            label(monday - 14)
        )),
        "{out}"
    );

    // The collector takes no week more than four weeks old, so the client drops this one.
    let old = client(&waiting("2020-W01", 0));
    let direct = format!("http://{}/v1/report", server.addr);
    let env = [("NVS_TELEMETRY_URL", direct.as_str())];
    let out = ran(nvs(&old, &old, &env, &["telemetry", "upload"]));
    assert!(
        out.starts_with("telemetry: refused by the collector (400"),
        "{out}"
    );
    assert!(out.trim_end().ends_with("; dropped"), "{out}");
    assert!(!counters(&old).contains("pending"), "{}", counters(&old));
    server.wait_for("/v1/report");

    let totals: serde_json::Value =
        serde_json::from_str(&server.get("/data/totals.json")).expect("the totals as JSON");
    assert_eq!(
        totals["reports"].as_array().map(Vec::len),
        Some(1),
        "{totals}"
    );

    // Every request came from 127.0.0.1, and the address is in no table and no log line.
    server.wait_for("/data/totals.json");
    let lines = server.lines.lock().expect("the lines").clone();
    for line in &lines {
        assert!(
            !line.contains("127.0.0.1") || line.starts_with("listening on "),
            "{line}"
        );
    }
    let files = database(&app, b"127.0.0.1");
    assert!(
        files.iter().any(|(name, _)| name == "telemetry.sqlite"),
        "{files:?}"
    );
    assert!(files.iter().all(|(_, found)| !found), "{files:?}");
}
