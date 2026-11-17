//! `rule:config/an-edit-reaches-the-next-request-without-a-restart` on a
//! connection that stays open: the built `nvs serve`, one keep-alive
//! connection that sends a request as soon as the last one is answered, and an
//! entry file rewritten many times while it does.
//!
//! The connection speaks HTTP/1.1 and keeps one request in flight. Each answer
//! is read to the end its `content-length` or its chunked framing names, so the
//! next request goes on the same socket. An edit is observed by the answers
//! themselves: the next edit is written once an answer shows the last one, and
//! [`BOUND`] is how long one edit may take to arrive.
//!
//! The program lives under `CARGO_TARGET_TMPDIR` and is removed when the
//! server is dropped.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use nvs_test::serve::Served;

/// How long one edit has to reach the keep-alive connection.
const BOUND: Duration = Duration::from_secs(30);

/// How many times the entry is rewritten.
const EDITS: u32 = 60;

/// A check every 100 milliseconds, by content, with no wait for the program to
/// be quiet, so each edit is compiled as soon as the check sees it.
const CONFIG: &str =
    "[opcache]\nvalidate = \"hash\"\nrevalidate_freq = \"100ms\"\nsettle = \"0s\"\n";

/// The entry at version `n`: a class with one method per version, so each
/// edit makes a unit that differs from the last, and a page that prints `vN;`.
fn entry(n: u32) -> String {
    let methods: String = (0..=n)
        .map(|i| {
            format!("    public static function v{i}(): int\n    {{\n        return {i};\n    }}\n")
        })
        .collect();
    format!("<?nvs\nfinal class Page\n{{\n{methods}}}\necho \"v\", Page::v{n}(), \";\";\n")
}

/// One answer read off the keep-alive connection: the status and the body.
fn answer(reader: &mut BufReader<TcpStream>) -> (u16, String) {
    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .expect("the server answers on the kept connection");
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("the answer has no status: {status_line:?}"));
    let mut length = None;
    let mut chunked = false;
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .expect("the header block is read");
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            length = value.parse::<usize>().ok();
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            chunked = value.eq_ignore_ascii_case("chunked");
        }
    }
    let mut body = Vec::new();
    if chunked {
        loop {
            let mut size = String::new();
            reader.read_line(&mut size).expect("a chunk size is read");
            let size = usize::from_str_radix(size.trim(), 16).expect("a chunk size is hex");
            let mut chunk = vec![0; size + 2];
            reader.read_exact(&mut chunk).expect("a chunk is read");
            if size == 0 {
                break;
            }
            body.extend_from_slice(&chunk[..size]);
        }
    } else {
        let length = length.expect("an answer without chunks has a content-length");
        body.resize(length, 0);
        reader.read_exact(&mut body).expect("the body is read");
    }
    (status, String::from_utf8_lossy(&body).into_owned())
}

/// The version a page printed, from its `vN;`.
fn version(body: &str) -> Option<u32> {
    body.trim()
        .strip_prefix('v')?
        .strip_suffix(';')?
        .parse()
        .ok()
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text)
        .unwrap_or_else(|error| panic!("`{}` could not be written: {error}", path.display()));
}

#[test]
fn a_busy_keep_alive_connection_serves_the_newest_version_after_many_reloads() {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("serve-reload-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the case's directory is created");
    let app = dir.join("app.nvs");
    write(&dir.join("nvs.toml"), CONFIG);
    write(&app, &entry(0));
    let data = nvs_repo::scratch_private("nvsdata");
    let server = Served::spawn(
        Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(&*data)
            .args(["serve", "app.nvs", "--listen", "127.0.0.1:0"])
            .current_dir(&dir),
        dir.clone(),
    );

    let stream = TcpStream::connect(server.addr).expect("the server takes a connection");
    stream
        .set_read_timeout(Some(BOUND))
        .expect("a read timeout is set");
    let mut writer = stream.try_clone().expect("the socket is cloned");
    let mut reader = BufReader::new(stream);
    let mut ask = || {
        write!(writer, "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .expect("the request is sent on the kept connection");
        answer(&mut reader)
    };

    let mut answers = 0_u32;
    for edit in 0..=EDITS {
        if edit > 0 {
            write(&app, &entry(edit));
        }
        let started = Instant::now();
        loop {
            let (status, body) = ask();
            answers += 1;
            assert_eq!(
                status,
                200,
                "answer {answers}, after edit {edit}, was not a 200: {body:?}\n{}",
                server.said()
            );
            let seen = version(&body)
                .unwrap_or_else(|| panic!("answer {answers} printed no version: {body:?}"));
            assert!(
                seen == edit || seen + 1 == edit,
                "answer {answers}, after edit {edit}, printed version {seen}"
            );
            if seen == edit {
                break;
            }
            assert!(
                started.elapsed() < BOUND,
                "edit {edit} did not reach the kept connection within {BOUND:?}\n{}",
                server.said()
            );
        }
    }
}
