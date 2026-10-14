//! `nvs run --request <file>`: the program answers the request that file
//! describes, and refuses to invent one where no file was named.
//!
//! Through the built binary rather than against `inbound_from` directly, for
//! the reason the `bundle` suite already writes down — `nvs-cli` has no
//! library target — and because what is asserted here is the whole crossing
//! and not one link of it: `nvs_test::request`'s file is read into an
//! `Inbound`, the `Ctx` carries it, and `Core\Request` answers off it. A test
//! that stopped at the carrier would pass while the program still saw
//! nothing.
//!
//! The format itself is `nvs_test::request`'s and is asserted there;
//! `rule:security/request-state-throws-in-an-isolate` is what the second test
//! pins.
//!
//! The last two cases run `nvs serve` itself, because the `Host` a request
//! arrives with is checked at the door, before a request file could describe
//! it.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// The directory every fixture in this file lives in.
fn fixtures() -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "request"]
        .iter()
        .collect()
}

/// Runs the fixture program, answering the named request file where there is
/// one, and hands back the whole result: one test wants the output and the
/// other wants the refusal.
fn run(request: Option<&str>) -> Output {
    let data = nvs_repo::scratch_private("nvsdata");
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    command
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("run");
    if let Some(file) = request {
        command.arg("--request").arg(fixtures().join(file));
    }
    command
        .arg(fixtures().join("reads-the-request.nvs"))
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

#[test]
fn a_request_file_becomes_the_inbound_the_program_answers() {
    let out = run(Some("reads-the-request.nvsr"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "the program answers it: {stderr}");
    // Every line is a section of the file: the verb, the path, one field of
    // the query, one header, one cookie off the `cookie` field, and the body
    // verbatim to its trailing newline.
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "method: POST\npath: /users\nquery: novis\nheader: 7\ncookie: abc123\nbody: name=ada\n"
    );
}

#[test]
fn a_run_without_a_request_file_answers_no_request() {
    let out = run(None);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "there is no request to answer");
    // Refused rather than answered empty, and refused as the program's own
    // mistake: "no request arrived" and "the request sent nothing" are
    // different facts, and a peer must never be able to raise this.
    assert!(
        stderr.contains("LogicError"),
        "the program's mistake: {stderr}"
    );
    assert!(
        stderr.contains("there is no request here"),
        "it says which of the two states this is: {stderr}"
    );
    assert!(
        out.stdout.is_empty(),
        "nothing is answered before the refusal"
    );
}

/// Runs the event-stream fixture as the answer to the fixture request, with
/// the events file beside it where `events` says so.
fn stream(events: bool) -> Output {
    let data = nvs_repo::scratch_private("nvsdata");
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    command
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("run")
        .arg("--request")
        .arg(fixtures().join("reads-the-request.nvsr"));
    if events {
        command
            .arg("--events")
            .arg(fixtures().join("opens-an-event-stream.nvse"));
    }
    command
        .arg(fixtures().join("opens-an-event-stream.nvs"))
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// `rule:concurrency/two-doors-one-isolate`: every request is offered the SSE
/// cell, so the stream its `Core\Sse::upgrade` prepared starts once it has
/// ended, and `--events` feeds that stream rather than the request. Only the
/// topic the stream subscribed to reaches it, and the connection's own `echo`
/// follows the stream.
// covers: Core\Sse::upgrade
#[test]
fn a_request_that_upgrades_to_an_event_stream_runs_it_fed_from_the_events_file() {
    let out = stream(true);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "the request ended normally: {stderr}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "data: hello ada\n\nevent: news\ndata: first\n\nevent: news\ndata: second\n\nclient left\n"
    );
}

/// With no `--events` the feed is empty, so the stream's first wait ends it
/// as a client that left does, and the run still ends.
// covers: Core\Sse::upgrade
#[test]
fn an_upgraded_event_stream_with_no_events_file_ends_at_its_first_wait() {
    let out = stream(false);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "the request ended normally: {stderr}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "data: hello ada\n\nclient left\n"
    );
}

/// A running `nvs serve app.nvs` whose program prints `ran`, in a directory
/// named for the case. Dropping it stops the process and removes the directory.
struct Served {
    child: Child,
    addr: SocketAddr,
    dir: PathBuf,
    _data: nvs_repo::Scratch,
}

impl Served {
    /// Starts the server on a free loopback port.
    ///
    /// # Panics
    ///
    /// When it does not print its `listening on` line within a minute.
    fn start(case: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("request-{case}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the case's directory is created");
        std::fs::write(dir.join("app.nvs"), "<?nvs\necho \"ran\";\n")
            .expect("the program is written");
        let data = nvs_repo::scratch_private("nvsdata");
        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(&*data)
            .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the `nvs` binary this test was built beside starts");
        let out = child.stdout.take().expect("standard output is piped");
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
        let Ok(addr) = bound.recv_timeout(Duration::from_secs(60)) else {
            let _ = child.kill();
            let _ = child.wait();
            panic!("`nvs serve` did not start listening");
        };
        Self {
            child,
            addr: addr.parse().expect("the server printed an address"),
            dir,
            _data: data,
        }
    }

    /// The status line the server answers `raw` with.
    fn status_of(&self, raw: &[u8]) -> String {
        let mut stream = TcpStream::connect(self.addr).expect("the server answers");
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .expect("a read timeout is set");
        stream.write_all(raw).expect("the request is sent");
        let mut line = String::new();
        BufReader::new(stream)
            .read_line(&mut line)
            .expect("an answer arrives");
        line.trim_end().to_owned()
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// `rule:http-server/host-matching-is-on-the-host-part-only`: HTTP/1.1 requires
/// a `Host` line, so the door answers `400` before any mount is chosen. HTTP/1.0
/// does not, and reaches the mount that declares no host.
#[test]
fn an_http_1_1_request_with_no_host_is_a_400() {
    let served = Served::start("no-host");
    assert_eq!(
        served.status_of(b"GET / HTTP/1.1\r\nConnection: close\r\n\r\n"),
        "HTTP/1.1 400 Bad Request"
    );
    assert_eq!(
        served.status_of(b"GET / HTTP/1.1\r\nHost: \r\nConnection: close\r\n\r\n"),
        "HTTP/1.1 400 Bad Request",
        "an empty line names no host either"
    );
    assert_eq!(
        served.status_of(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"),
        "HTTP/1.1 200 OK"
    );
    assert!(
        served
            .status_of(b"GET / HTTP/1.0\r\n\r\n")
            .ends_with(" 200 OK"),
        "HTTP/1.0 may leave the line out"
    );
}

/// A `Host` with a byte that is not ASCII is refused at the door. It does not
/// reach the mount that declares no host, as it did when it was read as no
/// host at all. Two `Host` lines are refused the same way.
#[test]
fn a_request_with_a_non_ascii_host_is_a_400() {
    let served = Served::start("non-ascii-host");
    for raw in [
        &b"GET / HTTP/1.1\r\nHost: bl\xc3\xb6g.example.com\r\nConnection: close\r\n\r\n"[..],
        b"GET / HTTP/1.1\r\nHost: bl\xf6g.example.com\r\nConnection: close\r\n\r\n",
        b"GET / HTTP/1.0\r\nHost: bl\xc3\xb6g.example.com\r\n\r\n",
        b"GET / HTTP/1.1\r\nHost: a.example.com\r\nHost: b.example.com\r\nConnection: close\r\n\r\n",
    ] {
        assert!(
            served.status_of(raw).ends_with(" 400 Bad Request"),
            "for {}",
            String::from_utf8_lossy(raw)
        );
    }
    assert_eq!(
        served.status_of(
            b"GET / HTTP/1.1\r\nHost: xn--blg-una.example.com\r\nConnection: close\r\n\r\n"
        ),
        "HTTP/1.1 200 OK",
        "punycode is ASCII"
    );
}
