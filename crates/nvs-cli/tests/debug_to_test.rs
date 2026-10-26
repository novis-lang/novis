//! A request a development server recorded becomes a test, and that test passes against the program
//! that answered it and fails when the program answers another status, another `Content-Type` or
//! another JSON field (`rule:testing/a-recorded-request-becomes-a-test`).
//!
//! The case serves one program from a directory and a data folder of its own, sends it one request,
//! and waits until the request and its response are in the debug index. The server is then stopped,
//! `nvs debug to-test` writes the test into the program's directory, and `nvs test .` runs the
//! directory as one program, the way a project runs its tests.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use rusqlite::{Connection, OpenFlags};

/// The JSON body the recorded program answers with.
const JSON: &str = "Core\\Response::json({ id: 7, items: ['tea', 'cake'] });";

/// The program, with its status and the line that writes its body left open, so each change the
/// test must catch is one argument.
fn program(status: u32, body: &str) -> String {
    format!(
        "<?nvs\nif (Core\\Env::context() == Core\\Env::Context::Request) {{\n    \
         Core\\Response::setStatus({status});\n    {body}\n}}\n"
    )
}

/// The program as it answered the recorded request.
fn recorded() -> String {
    program(201, JSON)
}

const CONFIG: &str = "[mode]\ndefault = \"development\"\n\n[debug]\nstore = true\n";

/// How long the case waits for something the server or the service does.
const PATIENCE: Duration = Duration::from_secs(60);

fn nvs(dir: &Path, data: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(data)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A new directory for `case`, holding `program` as `app.nvs` and [`CONFIG`] as `nvs.toml`.
fn case_dir(case: &str, program: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("debug-to-test-{case}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the case's directory is created");
    std::fs::write(dir.join("app.nvs"), program).expect("the program is written");
    std::fs::write(dir.join("nvs.toml"), CONFIG).expect("the configuration is written");
    dir
}

/// Serves `app.nvs` in `dir`, sends each of `requests` as written, and returns the trace id and
/// span id of each once every request and its response are in the index.
fn record(dir: &Path, data: &Path, requests: &[&str]) -> Vec<(String, String)> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(data)
        .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
        .current_dir(dir)
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
                let addr = rest.split_whitespace().next().unwrap_or_default();
                let _ = sent.send(addr.to_owned());
            }
        }
    });
    let Ok(addr) = bound.recv_timeout(PATIENCE) else {
        let _ = child.kill();
        let _ = child.wait();
        panic!("`nvs serve` did not start listening");
    };
    let addr: SocketAddr = addr.parse().expect("the server printed an address");
    let traced: Vec<(String, String)> = requests
        .iter()
        .map(|request| traced(&exchange(addr, request)))
        .collect();
    let index = data.join("debug").join("index.sqlite");
    for (trace, span) in &traced {
        let query = format!(
            "SELECT count(*) FROM requests r JOIN inputs i ON i.request = r.id \
             WHERE r.trace = '{trace}' AND r.span = '{span}' AND i.response_read = 1"
        );
        let until = Instant::now() + PATIENCE;
        loop {
            let read = Connection::open_with_flags(&index, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .and_then(|db| db.query_row(&query, [], |row| row.get::<_, i64>(0)));
            if read.as_ref().is_ok_and(|got| *got == 1) {
                break;
            }
            assert!(Instant::now() < until, "`{query}` answers {read:?}, not 1");
            thread::sleep(Duration::from_millis(50));
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    traced
}

/// The whole answer to `request`, sent as written to `addr`, which closes the connection.
fn exchange(addr: SocketAddr, request: &str) -> String {
    let mut stream = TcpStream::connect(addr).expect("the address answers");
    stream
        .set_read_timeout(Some(PATIENCE))
        .expect("a read timeout is set");
    stream
        .write_all(request.as_bytes())
        .expect("the request is sent");
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).expect("the answer is read");
    String::from_utf8_lossy(&answer).into_owned()
}

/// The trace id and span id in an answer's `traceresponse` header.
fn traced(answer: &str) -> (String, String) {
    answer
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if !name.eq_ignore_ascii_case("traceresponse") {
                return None;
            }
            let mut parts = value.trim().split('-');
            Some((parts.nth(1)?.to_owned(), parts.next()?.to_owned()))
        })
        .unwrap_or_else(|| panic!("no traceresponse header: {answer}"))
}

// covers: tools:cli/nvs-debug-to-test
#[test]
fn a_recorded_request_becomes_a_test_that_fails_when_the_response_changes() {
    let dir = case_dir("changes", &recorded());
    let data = nvs_repo::scratch_private("nvsdata");
    let [(trace, span)] = <[_; 1]>::try_from(record(
        &dir,
        &data,
        &["GET /orders?page=2 HTTP/1.1\r\nHost: localhost\r\n\
           Authorization: Bearer recorded-value\r\nConnection: close\r\n\r\n"],
    ))
    .expect("one request was recorded");

    let written = nvs(
        &dir,
        &data,
        &[
            "debug",
            "to-test",
            &trace,
            "--span",
            &span,
            "--write",
            "recorded.nvs",
        ],
    );
    assert!(written.status.success(), "{}", said(&written));
    let test = std::fs::read_to_string(dir.join("recorded.nvs")).expect("the test is written");
    assert!(test.contains("'authorization' => 'redacted',"), "{test}");
    assert!(!test.contains("recorded-value"), "{test}");
    assert!(test.contains("Core\\Json::decode("), "{test}");

    let again = nvs(
        &dir,
        &data,
        &["debug", "to-test", &trace, "--write", "recorded.nvs"],
    );
    assert!(!again.status.success(), "{}", said(&again));
    assert!(
        said(&again).contains("never replaces a file"),
        "{}",
        said(&again)
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("recorded.nvs")).expect("the test is still there"),
        test
    );

    let passed = nvs(&dir, &data, &["test", "."]);
    assert!(
        passed.status.success() && said(&passed).contains("0 failed, 1 passed"),
        "{test}\n{}",
        said(&passed)
    );

    for (changed, what) in [
        (program(200, JSON), "the status"),
        (
            program(
                201,
                "Core\\Response::text('{\"id\":7,\"items\":[\"tea\",\"cake\"]}');",
            ),
            "the `Content-Type`",
        ),
        (
            program(201, &JSON.replace("id: 7", "id: 8")),
            "one JSON field",
        ),
    ] {
        std::fs::write(dir.join("app.nvs"), &changed).expect("the program is changed");
        let failed = nvs(&dir, &data, &["test", "."]);
        assert!(
            !failed.status.success(),
            "the test passed though {what} changed:\n{changed}\n{}",
            said(&failed)
        );
        assert!(
            said(&failed).contains("assertEquals"),
            "the test failed for another reason than {what}:\n{}",
            said(&failed)
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The attack under `tests/hostile/tools/cli/nvs-debug-to-test/`: responses and a request header
/// whose text tries to end a string literal of the printed test and declare a class. Each printed
/// test compiles and passes, so every literal holds exactly the bytes that were recorded, and none
/// declares the class.
#[test]
fn a_response_that_tries_to_add_code_to_its_test_is_only_text_in_it() {
    let attack = std::fs::read_to_string(nvs_repo::path(
        "tests/hostile/tools/cli/nvs-debug-to-test/01-a-response-that-tries-to-add-code-to-its-test.nvs",
    ))
    .expect("the attack is in the repository");
    let dir = case_dir("attack", &attack);
    let data = nvs_repo::scratch_private("nvsdata");
    let header = "X-Note: it's \\' . exit(1) . ' $name {$name} ?> \"end\"\r\n";
    let requests = ["/text", "/json?q=%27%5C%24", "/bytes"].map(|path| {
        format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n{header}Connection: close\r\n\r\n")
    });
    let traced = record(&dir, &data, &requests.each_ref().map(String::as_str));
    for (n, (trace, span)) in traced.iter().enumerate() {
        let file = format!("recorded-{n}.nvs");
        let written = nvs(
            &dir,
            &data,
            &["debug", "to-test", trace, "--span", span, "--write", &file],
        );
        assert!(written.status.success(), "{}", said(&written));
        let test = std::fs::read_to_string(dir.join(&file)).expect("the test is written");
        assert!(
            !test
                .lines()
                .any(|line| line.trim_start().starts_with("class Injected")),
            "{test}"
        );
    }
    let passed = nvs(&dir, &data, &["test", "."]);
    assert!(
        passed.status.success() && said(&passed).contains("0 failed, 3 passed"),
        "{}",
        said(&passed)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
