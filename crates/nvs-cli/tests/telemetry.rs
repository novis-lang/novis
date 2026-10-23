//! `rule:tooling/a-serving-process-never-uploads` from outside the process: the `nvs` binary
//! with telemetry on, a finished week waiting in the counter file and `NVS_TELEMETRY_URL` at a
//! listener on loopback.
//!
//! `nvs serve` answers many requests and the listener accepts no connection, while the counter
//! file gains `invocation.serve`. `nvs telemetry upload` and `nvs update-check` each send the week
//! once, and the bytes the listener receives are the ones `nvs telemetry show` printed.
//!
//! The two attacks under `tests/hostile/tools/cli/` run here too: a forged counter file reaches
//! no payload, and a release reply with control characters is an error that prints none of them.
//!
//! Every run removes `CI` and `DO_NOT_TRACK`, which turn telemetry off and which a CI machine
//! sets.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// How long `nvs serve` has to compile its program and bind its port.
const BOOT: Duration = Duration::from_secs(60);

/// How many requests the server answers while the listener waits.
const REQUESTS: usize = 50;

/// The counter file the tests start from: nothing uploaded, and one finished week from 2020.
const WAITING: &str = "uploads 0\nbytes 0\npending 2020-W01\ndays 1000000\ninvocation.run 3\n";

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

/// A private data folder with telemetry on and [`WAITING`] as its counter file.
fn waiting() -> nvs_repo::Scratch {
    let data = nvs_repo::scratch_private("telemetry");
    let _ = ran(nvs(&data, &data, &[], &["telemetry", "on"]));
    std::fs::write(data.join("telemetry").join("counters"), WAITING).expect("the counter file");
    data
}

/// The counter file in `data`.
fn counters(data: &Path) -> String {
    std::fs::read_to_string(data.join("telemetry").join("counters")).expect("the counter file")
}

/// A collector on loopback that answers one report `204`, and its URL. The thread gives back
/// the report's body.
fn collector() -> (String, JoinHandle<Vec<u8>>) {
    serving("HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".to_owned())
}

/// A listener on loopback that answers one request with `reply`, and its URL. The thread gives
/// back the request's body.
fn serving(reply: String) -> (String, JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let url = format!(
        "http://{}/v1/report",
        listener.local_addr().expect("the bound address")
    );
    let served = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("the request connects");
        let mut reader = BufReader::new(stream.try_clone().expect("the stream"));
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("the head");
            if line.trim().is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                length = value.trim().parse().expect("a length");
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).expect("the body");
        stream.write_all(reply.as_bytes()).expect("the reply");
        body
    });
    (url, served)
}

/// What the attack `tests/hostile/tools/cli/<feature>/<name>` prints when `nvs run` runs it.
fn attack(feature: &str, name: &str) -> String {
    let path = nvs_repo::path(&format!("tests/hostile/tools/cli/{feature}/{name}"));
    let data = nvs_repo::scratch_private("telemetry-attack");
    ran(nvs(&data, &data, &[], &["run", &path.to_string_lossy()]))
}

/// Whether `text` has a control character other than a line break, the C1 set included.
fn controls(text: &str) -> bool {
    text.chars().any(|c| c != '\n' && c.is_control())
}

#[test]
fn a_serving_process_records_and_never_uploads() {
    let data = waiting();
    let app = nvs_repo::scratch("telemetry-serve");
    std::fs::write(app.join("app.nvs"), "<?nvs\necho \"ok\";\n").expect("the program");

    // A listener that counts every connection it accepts.
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    listener
        .set_nonblocking(true)
        .expect("a non-blocking listener");
    let url = format!(
        "http://{}/v1/report",
        listener.local_addr().expect("the bound address")
    );
    let accepted = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let watcher = {
        let (accepted, stop) = (Arc::clone(&accepted), Arc::clone(&stop));
        thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                if listener.accept().is_ok() {
                    accepted.fetch_add(1, Ordering::SeqCst);
                }
                thread::park_timeout(Duration::from_millis(10));
            }
        })
    };

    let mut server = nvs(
        &data,
        &app,
        &[("NVS_TELEMETRY_URL", &url)],
        &["serve", "app.nvs", "--listen", "127.0.0.1:0"],
    )
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .expect("the `nvs` binary starts");
    let out = server.stdout.take().expect("standard output is piped");
    let (sent, bound) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(rest) = line.strip_prefix("listening on http://") {
                let _ = sent.send(
                    rest.split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
        }
    });
    let Ok(addr) = bound.recv_timeout(BOOT) else {
        let _ = server.kill();
        panic!("`nvs serve` did not start listening within {BOOT:?}");
    };

    for _ in 0..REQUESTS {
        let mut stream = TcpStream::connect(&addr).expect("the server answers");
        write!(stream, "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n").expect("the request");
        let mut answer = String::new();
        stream.read_to_string(&mut answer).expect("the answer");
        assert!(answer.ends_with("ok"), "{answer}");
    }
    let _ = server.kill();
    let _ = server.wait();
    stop.store(true, Ordering::SeqCst);
    watcher.join().expect("the watcher thread");

    assert_eq!(accepted.load(Ordering::SeqCst), 0);
    let file = counters(&data);
    assert!(file.contains("\ninvocation.serve 1\n"), "{file}");
    assert!(file.contains("\npending 2020-W01\n"), "{file}");
    assert!(file.contains("\nuploads 0\n"), "{file}");
}

// covers: tools:cli/nvs-telemetry
#[test]
fn telemetry_upload_sends_what_show_prints_once() {
    let data = waiting();
    let shown = ran(nvs(&data, &data, &[], &["telemetry", "show"]));

    let (url, served) = collector();
    let env = [("NVS_TELEMETRY_URL", url.as_str())];
    let out = ran(nvs(&data, &data, &env, &["telemetry", "upload"]));
    let body = served.join().expect("the collector thread");
    assert_eq!(format!("{}\n", String::from_utf8_lossy(&body)), shown);
    assert_eq!(
        out,
        format!("telemetry: sent week 2020-W01, {} bytes\n", body.len())
    );
    assert!(
        counters(&data).contains(&format!("\nuploads 1\nbytes {}\n", body.len())),
        "{}",
        counters(&data)
    );

    // The week is gone, so the second run sends nothing and connects to nobody.
    let out = ran(nvs(&data, &data, &env, &["telemetry", "upload"]));
    assert_eq!(out, "telemetry: no finished week is waiting to be sent\n");
}

// covers: tools:cli/nvs-update-check
#[test]
fn update_check_carries_the_upload_and_prints_its_history() {
    let data = waiting();
    // Nothing listens on the release URL, so the release read fails at once.
    let port = TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("a loopback port")
        .port();
    let release = format!("http://127.0.0.1:{port}/latest");
    let (url, served) = collector();
    let env = [
        ("NVS_TELEMETRY_URL", url.as_str()),
        ("NVS_UPDATE_URL", release.as_str()),
    ];
    let out = ran(nvs(&data, &data, &env, &["update-check"]));
    let body = served.join().expect("the collector thread");
    assert!(out.starts_with("telemetry: on, "), "{out}");
    assert!(out.contains("could not read the latest release: "), "{out}");
    assert!(
        out.contains(&format!(
            "\ntelemetry: sent week 2020-W01, {} bytes\nlast upload: ",
            body.len()
        )),
        "{out}"
    );
    assert!(
        out.ends_with(&format!(
            "uploads accepted: 1, bytes sent: {}\n",
            body.len()
        )),
        "{out}"
    );
}

/// The attack under `tests/hostile/tools/cli/nvs-telemetry/`: the counter file its program prints,
/// with forged names, control characters and numbers at their limits. `status`, `show` and
/// `upload` exit `0`, the payload names only the two real counters of the finished week, and the
/// totals stop at `u64::MAX`.
// covers: tools:cli/nvs-telemetry
#[test]
fn a_forged_counter_file_reaches_no_payload_and_overflows_nothing() {
    let forged = attack("nvs-telemetry", "01-a-counter-file-with-forged-lines.nvs");
    let data = nvs_repo::scratch_private("telemetry-forged");
    let _ = ran(nvs(&data, &data, &[], &["telemetry", "on"]));
    std::fs::write(data.join("telemetry").join("counters"), &forged).expect("the counter file");

    let status = ran(nvs(&data, &data, &[], &["telemetry", "status"]));
    assert!(!controls(&status), "{status:?}");
    let shown = ran(nvs(&data, &data, &[], &["telemetry", "show"]));
    let document: serde_json::Value = serde_json::from_str(&shown).expect("one JSON document");
    assert_eq!(document["week"], "2020-W01");
    assert_eq!(document["days_active"], 7);
    assert_eq!(document["first"], false);
    assert_eq!(
        document["counters"],
        serde_json::json!({"invocation.run": u64::MAX, "internal-error.check": 2})
    );

    let (url, served) = collector();
    let env = [("NVS_TELEMETRY_URL", url.as_str())];
    let out = ran(nvs(&data, &data, &env, &["telemetry", "upload"]));
    let body = served.join().expect("the collector thread");
    assert_eq!(format!("{}\n", String::from_utf8_lossy(&body)), shown);
    assert!(out.starts_with("telemetry: sent week 2020-W01, "), "{out}");
    let file = counters(&data);
    assert!(
        file.contains(&format!("\nuploads {0}\nbytes {0}\n", u64::MAX)),
        "{file}"
    );
    for forged in ["ssh", "hunter2", "extra", "invocation.serve", "9223372036854775807"] {
        assert!(!file.contains(forged), "{forged}: {file}");
    }
    assert!(!controls(&file), "{file:?}");
}

/// The attack under `tests/hostile/tools/cli/nvs-update-check/`: each line its program prints is
/// one Releases API reply. `nvs update-check` exits `0` for each, says it could not read the
/// release, and prints no control character.
// covers: tools:cli/nvs-update-check
#[test]
fn a_release_that_writes_to_the_terminal_is_an_error() {
    let replies = attack(
        "nvs-update-check",
        "01-a-release-that-writes-to-the-terminal.nvs",
    );
    let data = nvs_repo::scratch_private("update-check-hostile");
    for body in replies.lines() {
        let (url, served) = serving(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ));
        let env = [("NVS_UPDATE_URL", url.as_str())];
        let out = ran(nvs(&data, &data, &env, &["update-check"]));
        served.join().expect("the release thread");
        assert!(!controls(&out), "{out:?}");
        assert!(
            out.lines()
                .last()
                .is_some_and(|line| line.starts_with("could not read the latest release: ")),
            "{out}"
        );
    }
    assert_eq!(replies.lines().count(), 4, "{replies}");
}
