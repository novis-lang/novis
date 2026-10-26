//! `nvs debug`'s request list over a real index
//! (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).
//!
//! The case starts one service on a data folder of its own and a port the OS picks, renames one
//! spool file of several requests into its spool, and waits on `/api/events` for the batch that
//! read it. It then reads `/api/requests` the ways the viewer does.
//!
//! **The viewer's fixture comes from here.** The newest page is written to
//! `viewer/test/fixtures/requests.json`, which `tsc` checks against `viewer/src/api.ts` and the
//! browser tests serve. When the committed file differs, the case writes the new one and fails, so
//! a change to the API's shape reaches the viewer's types in the same commit.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

/// How long the case waits for something the service does.
const PATIENCE: Duration = Duration::from_secs(60);

/// What the service prints before its link.
const LINK: &str = "the debug viewer is at ";

/// The trace id two requests share, as an inbound `traceparent` gives them.
const SHARED: &str = "0af7651916cd43dd8448eb211c80319c";

/// A running `nvs debug` on its own data folder.
struct Service {
    child: Child,
    data: nvs_repo::Scratch,
    addr: SocketAddr,
}

impl Service {
    fn start() -> Self {
        let data = nvs_repo::scratch_private("nvsdata");
        // The requests start in 2023, so the deletion window must reach that far back.
        std::fs::write(data.join("nvs.toml"), "[debug]\nindex_age = \"36500d\"\n")
            .expect("the configuration is written");
        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .current_dir(&*data)
            .arg("--data")
            .arg(&*data)
            .arg("--no-init")
            .args(["debug", "--listen", "127.0.0.1:0"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the `nvs` binary this test was built beside starts");
        let err = child.stderr.take().expect("standard error is piped");
        let (sent, link) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(err).lines().map_while(Result::ok) {
                if let Some(rest) = line.strip_prefix(LINK) {
                    let _ = sent.send(rest.to_owned());
                }
            }
        });
        let Ok(link) = link.recv_timeout(PATIENCE) else {
            let _ = child.kill();
            let _ = child.wait();
            panic!("`nvs debug` printed no link");
        };
        let port = link
            .rsplit_once(':')
            .and_then(|(_, rest)| rest.split('/').next()?.parse::<u16>().ok())
            .expect("the link names a port");
        Self {
            child,
            data,
            addr: SocketAddr::from(([127, 0, 0, 1], port)),
        }
    }

    /// `GET path`, and the status and the body.
    fn get(&self, path: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(self.addr).expect("the service accepts");
        stream
            .set_read_timeout(Some(PATIENCE))
            .expect("a timeout is set");
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.addr
        );
        stream
            .write_all(request.as_bytes())
            .expect("the request is sent");
        let mut bytes = Vec::new();
        let _ = stream.read_to_end(&mut bytes);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        let status = head
            .strip_prefix("HTTP/1.1 ")
            .and_then(|rest| rest.get(..3)?.parse().ok())
            .unwrap_or(0);
        (status, body.to_owned())
    }

    /// `GET path` as JSON, which must be a `200`.
    fn json(&self, path: &str) -> Value {
        let (status, body) = self.get(path);
        assert_eq!(status, 200, "{path}: {body}");
        serde_json::from_str(&body).unwrap_or_else(|err| panic!("{path}: {err}: {body}"))
    }

    /// Waits on `/api/events` until a batch at least `batch` is pushed.
    fn wait_for(&self, events: &mut BufReader<TcpStream>, batch: i64) {
        let mut line = String::new();
        loop {
            line.clear();
            assert!(
                events.read_line(&mut line).expect("the stream is open") > 0,
                "the stream ended"
            );
            if let Some(pushed) = line.trim_end().strip_prefix("data: ")
                && pushed.parse::<i64>().expect("the event is a number") >= batch
            {
                return;
            }
        }
    }

    fn events(&self) -> BufReader<TcpStream> {
        let mut stream = TcpStream::connect(self.addr).expect("the service accepts");
        stream
            .set_read_timeout(Some(PATIENCE))
            .expect("a timeout is set");
        let request = format!("GET /api/events HTTP/1.1\r\nHost: {}\r\n\r\n", self.addr);
        stream
            .write_all(request.as_bytes())
            .expect("the request is sent");
        BufReader::new(stream)
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// One request the case records: its trace id, span id, method, route, path, status and duration.
type Sent<'a> = (
    &'a str,
    &'a str,
    &'a str,
    Option<&'a str>,
    &'a str,
    u16,
    u64,
);

/// The spool lines of the case's requests: every column the list shows takes more than one
/// value, one request threw, and two share a trace id.
fn spool() -> String {
    let writer = "4242-1700000000000-0";
    let base: u64 = 1_700_000_000_000_000;
    let mut lines = Vec::new();
    let mut seq = 0;
    let mut line = |trace: &str, at_us: u64, record: Value| {
        let mut line = json!({ "writer": writer, "seq": seq, "trace": trace, "at_us": at_us });
        let fields = line.as_object_mut().expect("an object");
        for (key, value) in record.as_object().expect("an object") {
            fields.insert(key.clone(), value.clone());
        }
        seq += 1;
        lines.push(line.to_string());
    };
    let requests: [Sent<'_>; 6] = [
        (
            "4bf92f3577b34da6a3ce929d0e0e4736",
            "00f067aa0ba902b7",
            "GET",
            Some("/"),
            "/",
            200,
            840,
        ),
        (
            "5c1e8a0d2b7f4e9a8c3d6b1f0e2a4c7d",
            "b7ad6b7169203331",
            "GET",
            Some("/items/{id}"),
            "/items/7",
            200,
            2_310,
        ),
        (
            "6d2f9b1e3c8a4f0b9d4e7c2a1f3b5d8e",
            "e457b5a2e4d86bd1",
            "POST",
            Some("/orders"),
            "/orders",
            500,
            15_400,
        ),
        (
            SHARED,
            "53995c3f42cd8ad8",
            "GET",
            None,
            "/missing",
            404,
            120,
        ),
        (
            SHARED,
            "a3ce929d0e0e4736",
            "PUT",
            Some("/items/{id}"),
            "/items/7",
            204,
            3_900,
        ),
        (
            "7e3a0c2f4d9b4a1c8e5f8d3b2a4c6e9f",
            "c4d8e0f1a2b3c4d5",
            "DELETE",
            Some("/items/{id}"),
            "/items/9",
            403,
            610,
        ),
    ];
    for (n, (trace, span, method, route, path, status, duration_us)) in requests.iter().enumerate()
    {
        let start_us = base + 1_000_000 * n as u64;
        if *status == 500 {
            line(
                trace,
                start_us + 100,
                json!({
                    "kind": "event", "event": "query", "label": "sqlite main SELECT * FROM items WHERE id = ?",
                    "start_us": start_us + 100, "end_us": start_us + 900,
                    "query": { "driver": "sqlite", "sql": "SELECT * FROM items WHERE id = ?", "rows": 1 }
                }),
            );
            line(
                trace,
                start_us + 1_000,
                json!({
                    "kind": "dump", "file": "src/Shop/Orders.nvs", "line": 40,
                    "tree": { "class": "Shop\\Item", "fields": { "id": 7, "stock": 0 } }, "elided": false
                }),
            );
            line(
                trace,
                start_us + 1_200,
                json!({
                    "kind": "log", "level": "warning", "message": "stock is empty",
                    "fields": { "item": 7 }, "elided": false
                }),
            );
        }
        let mut request = json!({
            "kind": "request", "span": span, "start_us": start_us, "duration_us": duration_us,
            "method": method, "path": path, "status": status
        });
        if let Some(route) = route {
            request["route"] = json!(route);
        }
        if *status == 500 {
            request["exception"] = json!({
                "class": "Shop\\OutOfStock", "message": "item 7 is out of stock",
                "file": "src/Shop/Orders.nvs", "line": 42
            });
        }
        line(trace, start_us + duration_us, request);
    }
    lines.join("\n") + "\n"
}

#[test]
fn the_request_list_pages_newest_first_and_writes_the_viewers_fixture() {
    let service = Service::start();
    let mut events = service.events();
    service.wait_for(&mut events, 0);
    let debug = service.data.join("debug");
    // Written beside the spool and then renamed into it, so the service never sees half a file.
    let staged = debug.join("staged.spool");
    std::fs::write(&staged, spool()).expect("the debug folder is writable");
    std::fs::rename(&staged, debug.join("spool").join("1-1-0.0.spool"))
        .expect("the spool is writable");
    service.wait_for(&mut events, 1);

    let newest = service.json("/api/requests");
    assert_eq!(newest["watermark"], 1);
    let rows = newest["requests"].as_array().expect("a list");
    assert_eq!(rows.len(), 6);
    assert_eq!(newest["older"], Value::Null, "one page holds every request");
    let ids: Vec<i64> = rows
        .iter()
        .map(|row| row["id"].as_i64().expect("an id"))
        .collect();
    assert!(
        ids.windows(2).all(|pair| pair[0] > pair[1]),
        "newest first: {ids:?}"
    );
    let threw = rows
        .iter()
        .find(|row| row["status"] == 500)
        .expect("the request that threw");
    assert_eq!(threw["exception"]["class"], "Shop\\OutOfStock");
    assert_eq!(threw["exception"]["line"], 42);
    assert_eq!(
        (&threw["queries"], &threw["dumps"], &threw["logs"]),
        (&json!(1), &json!(1), &json!(1))
    );
    assert_eq!(rows[0]["exception"], Value::Null);

    let first = service.json("/api/requests?limit=4");
    let older = first["older"]
        .as_i64()
        .expect("a page of four has an older page");
    assert_eq!(first["requests"].as_array().map(Vec::len), Some(4));
    let rest = service.json(&format!("/api/requests?before={older}"));
    assert_eq!(rest["requests"].as_array().map(Vec::len), Some(2));
    assert_eq!(rest["older"], Value::Null);

    let since = service.json("/api/requests?since=1");
    assert_eq!(
        since["requests"],
        json!([]),
        "nothing arrived after batch 1"
    );
    let shared = service.json(&format!("/api/requests?trace={SHARED}"));
    assert_eq!(shared["requests"].as_array().map(Vec::len), Some(2));
    let one = service.json(&format!(
        "/api/requests?trace={SHARED}&span=53995c3f42cd8ad8"
    ));
    assert_eq!(one["requests"][0]["path"], "/missing");

    let (status, body) = service.get("/api/requests?order=asc");
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("`order`"), "{body}");

    let fixture = nvs_repo::path("viewer/test/fixtures/requests.json");
    let written = serde_json::to_string_pretty(&newest).expect("JSON") + "\n";
    let committed = std::fs::read_to_string(&fixture).unwrap_or_default();
    if committed.replace("\r\n", "\n") != written {
        std::fs::create_dir_all(fixture.parent().expect("a folder")).expect("the folder is made");
        std::fs::write(&fixture, &written).expect("the fixture is writable");
        panic!(
            "{} differed from the API's newest page and is written again: commit it",
            fixture.display()
        );
    }
}
