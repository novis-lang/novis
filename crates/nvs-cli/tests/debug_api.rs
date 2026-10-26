//! `nvs debug`'s API over a real index: the request list and request page
//! (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`), and the stream, the
//! search and the saved filters (`rule:testing/the-viewer-searches-every-record-and-saves-a-filter`).
//!
//! Each case starts one service on a data folder of its own and a port the OS picks, renames one
//! spool file of several requests into its spool, and waits on `/api/events` for the batch that
//! read it. It then reads the API the ways the viewer does.
//!
//! **The viewer's fixtures come from here.** The newest page is written to
//! `viewer/test/fixtures/requests.json`, the request that threw to `request.json`, the stream to
//! `stream.json`, a search to `search.json` and the saved settings to `settings.json`. `tsc` checks
//! them against `viewer/src/api.ts` and the browser tests serve them. When a committed file
//! differs, the case writes the new one and fails, so a change to the API's shape reaches the
//! viewer's types in the same commit.

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
    /// A service that has ingested [`spool`]'s requests.
    fn ingested() -> Self {
        let service = Self::start();
        let mut events = service.events();
        service.wait_for(&mut events, 0);
        let debug = service.data.join("debug");
        // Written beside the spool and then renamed into it, so the service never sees half a file.
        let staged = debug.join("staged.spool");
        std::fs::write(&staged, spool()).expect("the debug folder is writable");
        std::fs::rename(&staged, debug.join("spool").join("1-1-0.0.spool"))
            .expect("the spool is writable");
        service.wait_for(&mut events, 1);
        service
    }

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
        self.send("GET", path, None)
    }

    /// `method path` with a JSON `body` when there is one, from the page's own origin, and the
    /// status and the body.
    fn send(&self, method: &str, path: &str, body: Option<&str>) -> (u16, String) {
        let mut stream = TcpStream::connect(self.addr).expect("the service accepts");
        stream
            .set_read_timeout(Some(PATIENCE))
            .expect("a timeout is set");
        let addr = self.addr;
        let request = match body {
            None => {
                format!("{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n")
            }
            Some(body) => format!(
                "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nOrigin: http://{addr}\r\n\
                 Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ),
        };
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
            let event = |kind: &str, label: &str, from: u64, to: u64| {
                json!({
                    "kind": "event", "event": kind, "label": label,
                    "start_us": start_us + from, "end_us": start_us + to
                })
            };
            line(
                trace,
                start_us + 14_000,
                event("call", "Shop\\Orders::place", 50, 14_000),
            );
            line(
                trace,
                start_us + 900,
                json!({
                    "kind": "event", "event": "query", "label": "sqlite main SELECT * FROM items WHERE id = ?",
                    "start_us": start_us + 100, "end_us": start_us + 900,
                    "query": { "driver": "sqlite", "sql": "SELECT * FROM items WHERE id = ?", "rows": 1, "site": 12 }
                }),
            );
            line(
                trace,
                start_us + 1_000,
                dump(
                    40,
                    &json!({
                        "$class": "Shop\\Item", "$id": 1,
                        "$properties": { "id": 7, "stock": 0, "name": "Lamp" }
                    }),
                ),
            );
            line(
                trace,
                start_us + 1_200,
                json!({
                    "kind": "log", "level": "warning", "message": "stock is empty",
                    "fields": { "item": 7 }, "elided": false
                }),
            );
            line(
                trace,
                start_us + 3_000,
                event("gc", "cycle collection", 2_400, 3_000),
            );
            line(
                trace,
                start_us + 9_000,
                event("spawn", "Shop\\Mail::notify", 4_000, 9_000),
            );
            // A repeat names the node it repeats by its `$id`, and a cut list says how much is gone.
            line(
                trace,
                start_us + 13_000,
                dump(
                    41,
                    &json!({
                        "$class": "Shop\\Order", "$id": 2,
                        "$properties": {
                            "id": 31,
                            "card": { "$redacted": true },
                            "lines": [
                                { "$class": "Shop\\Line", "$id": 3, "$properties": {
                                    "order": { "$cycle": 2 }, "quantity": 2
                                } },
                                { "$elided": { "cut": "entries", "total": 40, "entries": 39 } }
                            ],
                            "note": { "$elided": { "cut": "text", "kept": "Leave at the", "bytes": 120 } },
                            "deep": { "$elided": { "cut": "depth" } }
                        }
                    }),
                ),
            );
        }
        // Two requests share a trace id, and each has one record of its own.
        if *span == "53995c3f42cd8ad8" {
            line(
                trace,
                start_us + 60,
                json!({
                    "kind": "log", "level": "info", "message": "no route matched",
                    "fields": {}, "elided": false
                }),
            );
        }
        if *span == "a3ce929d0e0e4736" {
            line(trace, start_us + 2_000, dump(17, &json!(7)));
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

/// A `dump` line from `src/Shop/Orders.nvs` at `line`.
fn dump(line: u32, tree: &Value) -> Value {
    json!({ "kind": "dump", "file": "src/Shop/Orders.nvs", "line": line, "tree": tree, "elided": false })
}

/// Writes `value` to `viewer/test/fixtures/<name>` when the committed file differs, and gives the
/// path it wrote.
fn fixture(name: &str, value: &Value) -> Option<String> {
    let fixture = nvs_repo::path(&format!("viewer/test/fixtures/{name}"));
    let written = serde_json::to_string_pretty(value).expect("JSON") + "\n";
    let committed = std::fs::read_to_string(&fixture).unwrap_or_default();
    if committed.replace("\r\n", "\n") == written {
        return None;
    }
    std::fs::create_dir_all(fixture.parent().expect("a folder")).expect("the folder is made");
    std::fs::write(&fixture, &written).expect("the fixture is writable");
    Some(fixture.display().to_string())
}

/// The number of groups a search gives.
fn groups(results: &Value) -> usize {
    results["groups"].as_array().map_or(0, Vec::len)
}

/// The kinds of the matches in a search's first group, sorted.
fn hit_kinds(results: &Value) -> Vec<String> {
    let mut kinds: Vec<String> = results["groups"][0]["hits"]
        .as_array()
        .expect("a list of matches")
        .iter()
        .map(|hit| hit["kind"].as_str().expect("a kind").to_owned())
        .collect();
    kinds.sort();
    kinds
}

#[test]
fn search_finds_words_phrases_and_field_terms_and_a_saved_filter_outlives_the_index() {
    let service = Service::ingested();

    // The writer replaced the card number with a redacted node, so the number is not found.
    let secret = service.json("/api/search?q=4111");
    assert_eq!(groups(&secret), 0, "{secret}");
    let word = service.json("/api/search?q=stock");
    assert_eq!(groups(&word), 1, "{word}");
    assert_eq!(word["groups"][0]["request"]["status"], 500);
    assert_eq!(hit_kinds(&word), ["dump", "exception", "log"]);
    let prefix = service.json("/api/search?q=Lam");
    assert_eq!(
        hit_kinds(&prefix),
        ["dump"],
        "a bare word matches the start of a word"
    );
    let phrase = service.json("/api/search?q=%22out+of+stock%22");
    assert_eq!(hit_kinds(&phrase), ["exception"]);
    let narrowed = service.json("/api/search?q=stock+kind%3Alog");
    assert_eq!(hit_kinds(&narrowed), ["log"]);

    for (terms, want) in [
        ("status%3A500", 1),
        ("status%3A2xx", 3),
        ("method%3Adelete", 1),
        ("route%3A%2Fitems%2F%7Bid%7D", 3),
        ("kind%3Aquery", 1),
        ("kind%3Aexception", 1),
        ("level%3Ainfo", 1),
        ("duration%3A%3E2ms", 3),
        ("duration%3A%3C%3D120us", 1),
        (&format!("trace%3A{SHARED}"), 2),
    ] {
        let found = service.json(&format!("/api/search?q={terms}"));
        assert_eq!(groups(&found), want, "{terms}: {found}");
        assert_eq!(found["groups"][0]["hits"], json!([]), "{terms}");
    }
    let pasted = service.json(&format!("/api/search?q=00-{SHARED}-53995c3f42cd8ad8-01"));
    assert_eq!(
        pasted["opens"],
        json!({ "trace": SHARED, "span": "53995c3f42cd8ad8" })
    );
    let first = service.json("/api/search?q=status%3A2xx&limit=2");
    let older = first["older"]
        .as_str()
        .expect("a page of two has a next page");
    let rest = service.json(&format!("/api/search?q=status%3A2xx&before={older}"));
    assert_eq!((groups(&first), groups(&rest)), (2, 1));
    assert_eq!(rest["older"], Value::Null);
    let (status, body) = service.get("/api/search?q=kind%3Acall");
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("kind:call"), "{body}");

    let stream = service.json("/api/stream");
    let records = stream["records"].as_array().expect("a list");
    assert_eq!(
        records.len(),
        15,
        "6 requests, 4 events, 3 dumps and 2 log lines"
    );
    let times: Vec<i64> = records
        .iter()
        .map(|record| record["at_us"].as_i64().expect("a time"))
        .collect();
    assert!(
        times.windows(2).all(|pair| pair[0] >= pair[1]),
        "newest first: {times:?}"
    );
    let logs = service.json("/api/stream?kind=log");
    assert_eq!(logs["records"].as_array().map(Vec::len), Some(2));
    assert_eq!(logs["records"][0]["request"]["path"], "/missing");
    assert_eq!(logs["records"][1]["request"]["path"], "/orders");
    let info = service.json("/api/stream?level=info");
    assert_eq!(info["records"][0]["text"], "no route matched");
    let shared = service.json(&format!("/api/stream?trace={SHARED}"));
    assert_eq!(shared["records"].as_array().map(Vec::len), Some(4));
    let routed = service.json("/api/stream?route=%2Forders");
    assert_eq!(routed["records"].as_array().map(Vec::len), Some(8));
    let page = service.json("/api/stream?limit=10");
    let older = page["older"]
        .as_str()
        .expect("a page of ten has an older page");
    let rest = service.json(&format!("/api/stream?before={older}"));
    assert_eq!(rest["records"].as_array().map(Vec::len), Some(5));
    assert_eq!(service.json("/api/quarantine"), json!({ "files": [] }));

    let defaults = service.json("/api/settings");
    assert_eq!(
        defaults,
        json!({ "editor": "vscode", "theme": "system", "filters": [] })
    );
    let saved = r#"{"editor":"vscode","theme":"dark","filters":[{"name":"Slow orders","view":"search","query":"q=route%3A%2Forders+duration%3A%3E10ms"}]}"#;
    let (status, body) = service.send("PUT", "/api/settings", Some(saved));
    assert_eq!(status, 200, "{body}");
    let (status, body) = service.send("PUT", "/api/settings", Some(r#"{"colour":"red"}"#));
    assert_eq!(status, 400, "{body}");
    let (status, body) = service.send("POST", "/api/index/discard", Some("{}"));
    assert_eq!((status, body.as_str()), (200, r#"{"watermark":2}"#));
    let after = service.json("/api/requests");
    assert_eq!(after["requests"], json!([]), "the index is empty again");
    assert_eq!(after["watermark"], 2, "the watermark goes on rising");
    let kept = service.json("/api/settings");
    assert_eq!(kept["filters"][0]["name"], "Slow orders", "{kept}");

    let written: Vec<String> = [
        ("stream.json", &stream),
        ("search.json", &word),
        ("settings.json", &kept),
    ]
    .into_iter()
    .filter_map(|(name, value)| fixture(name, value))
    .collect();
    assert!(
        written.is_empty(),
        "{written:?} differed from the API's answers and are written again: commit them"
    );
}

#[test]
fn the_request_list_pages_newest_first_and_writes_the_viewers_fixture() {
    let service = Service::ingested();

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
        (&json!(1), &json!(2), &json!(1))
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

    let thrown = format!(
        "/api/request?trace={}&span={}",
        threw["trace"].as_str().expect("a trace id"),
        threw["span"].as_str().expect("a span id")
    );
    let detail = service.json(&thrown);
    assert_eq!(detail["request"], *threw);
    assert_eq!(detail["exception"]["message"], "item 7 is out of stock");
    let kinds: Vec<&str> = detail["events"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|event| event["kind"].as_str().expect("a kind"))
        .collect();
    assert_eq!(kinds, ["call", "query", "gc", "spawn"], "in order of start");
    assert_eq!(detail["events"][1]["query"]["site"], 12);
    assert_eq!(detail["dumps"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        detail["dumps"][1]["tree"]["$properties"]["card"],
        json!({ "$redacted": true })
    );
    assert_eq!(detail["logs"][0]["message"], "stock is empty");
    assert_eq!(
        detail["cut"],
        json!({ "events": false, "dumps": false, "logs": false })
    );

    // Each request of a shared trace id shows its own records alone.
    let missing = service.json(&format!(
        "/api/request?trace={SHARED}&span=53995c3f42cd8ad8"
    ));
    assert_eq!(missing["logs"][0]["message"], "no route matched");
    assert_eq!(missing["dumps"], json!([]));
    let put = service.json(&format!(
        "/api/request?trace={SHARED}&span=a3ce929d0e0e4736"
    ));
    assert_eq!(put["dumps"][0]["tree"], 7);
    assert_eq!(put["logs"], json!([]));
    let (status, body) = service.get(&format!("/api/request?trace={SHARED}"));
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("`span`"), "{body}");
    let (status, body) = service.get("/api/request?trace=1fffffffffffffffffffffffffffffff");
    assert_eq!(status, 404, "{body}");

    let written: Vec<String> = [("requests.json", &newest), ("request.json", &detail)]
        .into_iter()
        .filter_map(|(name, value)| fixture(name, value))
        .collect();
    assert!(
        written.is_empty(),
        "{written:?} differed from the API's answers and are written again: commit them"
    );
}
