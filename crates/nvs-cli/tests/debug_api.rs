//! `nvs debug`'s API over a real index: the request list and request page
//! (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`), the stream, the
//! search and the saved filters (`rule:testing/the-viewer-searches-every-record-and-saves-a-filter`),
//! and the exception groups (`rule:testing/an-exception-is-one-row-per-class-and-site`).
//!
//! Each case starts one service on a data folder of its own and a port the OS picks, renames one
//! spool file of several requests into its spool, and waits on `/api/events` for the batch that
//! read it. It then reads the API the ways the viewer does.
//!
//! **The viewer's fixtures come from here.** The newest page is written to
//! `viewer/test/fixtures/requests.json`, the request that threw to `request.json`, the stream to
//! `stream.json`, a search to `search.json`, the saved settings to `settings.json`, and the
//! exception groups and one group's requests to `exceptions.json` and `exception.json`, and one
//! request's coverage and a file it ran to `coverage.json` and `source.json`. The summary, the
//! routes, the statements and the health go to `summary.json`, `routes.json`, `statements.json`
//! and `health.json`. `tsc` checks
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
    /// The token the printed link carries, which every request sends as the loopback cookie.
    token: String,
}

impl Service {
    /// A service that has ingested [`spool`]'s requests.
    fn ingested() -> Self {
        Self::ingested_from(&spool())
    }

    /// A service that has ingested the spool file `text`.
    fn ingested_from(text: &str) -> Self {
        Self::ingested_with(|_| text.to_owned())
    }

    /// A service that has ingested the spool file `make` writes, given the service's data folder.
    fn ingested_with(make: impl FnOnce(&std::path::Path) -> String) -> Self {
        let service = Self::start();
        let mut events = service.events();
        service.wait_for(&mut events, 0);
        let debug = service.data.join("debug");
        // Written beside the spool and then renamed into it, so the service never sees half a file.
        let staged = debug.join("staged.spool");
        std::fs::write(&staged, make(&service.data)).expect("the debug folder is writable");
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
        Self::start_on(data)
    }

    /// A service on `data`, whose `nvs.toml` and index are already written.
    fn start_on(data: nvs_repo::Scratch) -> Self {
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
        let token = link
            .split_once("?token=")
            .expect("the link carries the token")
            .1
            .to_owned();
        Self {
            child,
            data,
            addr: SocketAddr::from(([127, 0, 0, 1], port)),
            token,
        }
    }

    /// The `Cookie` header line the printed link sets, with its token.
    fn cookie(&self) -> String {
        format!("Cookie: nvs-debug-{}={}\r\n", self.addr.port(), self.token)
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
        let cookie = self.cookie();
        let request = match body {
            None => format!(
                "{method} {path} HTTP/1.1\r\nHost: {addr}\r\n{cookie}Connection: close\r\n\r\n"
            ),
            Some(body) => format!(
                "{method} {path} HTTP/1.1\r\nHost: {addr}\r\n{cookie}Origin: http://{addr}\r\n\
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
        let request = format!(
            "GET /api/events HTTP/1.1\r\nHost: {}\r\n{}\r\n",
            self.addr,
            self.cookie()
        );
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
                start_us + 80,
                json!({ "kind": "profile", "path": "/srv/shop/orders.speedscope.json" }),
            );
            line(
                trace,
                start_us + 900,
                json!({
                    "kind": "event", "event": "query", "label": "sqlite main SELECT * FROM items WHERE id = ?",
                    "start_us": start_us + 100, "end_us": start_us + 900,
                    "query": { "driver": "sqlite", "sql": "SELECT * FROM items WHERE id = ?", "rows": 1, "site": { "file": "src/Shop/Items.nvs", "line": 12, "hash": "ab12" } }
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
        // The thrown request copied at two sites, and the list puts the larger one first.
        if *status == 500 {
            line(
                trace,
                start_us + 15_200,
                json!({
                    "kind": "copies", "span": span, "sites": [
                        { "file": "src/Shop/Orders.nvs", "line": 30, "hash": "ab12",
                          "reason": "still_used", "copies": 1, "units": 3 },
                        { "file": "src/Shop/Orders.nvs", "line": 36, "hash": "ab12",
                          "reason": "slot_kept", "copies": 4, "units": 120 }
                    ]
                }),
            );
        }
        // A loop runs one statement at one site twice and calls out once, and a third run of the
        // statement arrives after the request's line below.
        if *span == "b7ad6b7169203331" {
            for (from, to) in [(100, 400), (500, 700)] {
                line(trace, start_us + to, repeated(start_us, from, to));
            }
            line(
                trace,
                start_us + 1_800,
                json!({
                    "kind": "event", "event": "http", "label": "GET prices.example.com/items",
                    "start_us": start_us + 800, "end_us": start_us + 1_800,
                    "target": "prices.example.com/items"
                }),
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
        // A `body` part of the earlier request of the shared trace is not the later one's.
        if *span == "53995c3f42cd8ad8" {
            line(
                trace,
                start_us + 70,
                json!({ "kind": "body", "of": "request", "offset": 0, "text": "not the PUT's" }),
            );
        }
        if *span == "a3ce929d0e0e4736" {
            line(trace, start_us + 2_000, dump(17, &json!(7)));
            for (offset, text) in [(0, "{\"stock\":"), (9, "3}")] {
                line(
                    trace,
                    start_us + 2_100,
                    json!({ "kind": "body", "of": "request", "offset": offset, "text": text }),
                );
            }
        }
        // The thrown request's response body is two parts, and one is not UTF-8.
        if *status == 500 {
            line(
                trace,
                start_us + 15_000,
                json!({ "kind": "body", "of": "response", "offset": 0, "text": "Out of " }),
            );
            line(
                trace,
                start_us + 15_100,
                json!({ "kind": "body", "of": "response", "offset": 7, "base64": "/3N0b2Nr" }),
            );
        }
        let mut request = json!({
            "kind": "request", "span": span, "start_us": start_us, "duration_us": duration_us,
            "method": method, "path": path, "status": status
        });
        if let Some(route) = route {
            request["route"] = json!(route);
        }
        if *span == "b7ad6b7169203331" {
            request["memory_peak"] = json!(2_097_152);
            request["handler"] = json!("Shop\\Items::show");
        }
        if *status == 500 {
            request["memory_peak"] = json!(8_388_608);
            request["memory_limit"] = json!(134_217_728);
            request["handler"] = json!("Shop\\Orders::create");
            request["bytes_in"] = json!(23);
            request["bytes_out"] = json!(13);
            request["exception"] = json!({
                "class": "Shop\\OutOfStock", "message": "item 7 is out of stock",
                "file": "src/Shop/Orders.nvs", "line": 42,
                "frames": [
                    { "function": "Shop\\Stock::take", "file": "src/Shop/Stock.nvs", "line": 18 },
                    { "function": "Shop\\Orders::create", "file": "src/Shop/Orders.nvs", "line": 42 },
                    { "function": "Core\\Http\\Router::dispatch" }
                ],
                "previous": [{
                    "class": "Shop\\StockError", "message": "no row for item 7",
                    "file": "src/Shop/Stock.nvs", "line": 11,
                    "frames": [
                        { "function": "Shop\\Stock::row", "file": "src/Shop/Stock.nvs", "line": 11 }
                    ]
                }]
            });
            request["input"] = json!({
                "query": "coupon=SPRING",
                "headers": [["content-type", "application/json"], ["authorization", "[redacted]"]],
                "body": { "read": true, "text": "{\"item\":7,\"quantity\":2}" }
            });
            request["response"] = json!({
                "headers": [["content-type", "text/plain"], ["set-cookie", "[redacted]"]],
                "body": { "read": true, "chunks": 2 }
            });
        }
        if *span == "a3ce929d0e0e4736" {
            request["input"] = json!({
                "query": "",
                "headers": [["content-type", "application/json"]],
                "body": { "read": true, "chunks": 2 }
            });
        }
        line(trace, start_us + duration_us, request);
        if *span == "b7ad6b7169203331" {
            line(trace, start_us + 2_400, repeated(start_us, 1_900, 2_000));
        }
    }
    lines.join("\n") + "\n"
}

/// A `query` event of the statement the `GET /items/{id}` request runs in a loop, from `from` to
/// `to` microseconds after `start_us`.
fn repeated(start_us: u64, from: u64, to: u64) -> Value {
    json!({
        "kind": "event", "event": "query", "label": "sqlite main SELECT price FROM items WHERE id = ?",
        "start_us": start_us + from, "end_us": start_us + to,
        "query": { "driver": "sqlite", "sql": "SELECT price FROM items WHERE id = ?", "rows": 1,
                   "site": { "file": "src/Shop/Items.nvs", "line": 20, "hash": "ab12" } }
    })
}

/// The spool lines of requests that throw: three throw one class at one site under two routes,
/// one throws another class, and one throws nothing.
fn throwing_spool() -> String {
    let writer = "4242-1700000000000-0";
    let base: u64 = 1_700_000_000_000_000;
    let throws = [
        (
            "POST",
            "/orders",
            Some((
                "Shop\\OutOfStock",
                "item 7 is out of stock",
                "src/Shop/Orders.nvs",
                42,
            )),
        ),
        ("GET", "/", None),
        (
            "POST",
            "/cart",
            Some((
                "Shop\\OutOfStock",
                "item 9 is out of stock",
                "src/Shop/Orders.nvs",
                42,
            )),
        ),
        (
            "POST",
            "/pay",
            Some((
                "Shop\\PaymentDeclined",
                "the card was declined",
                "src/Shop/Pay.nvs",
                18,
            )),
        ),
        (
            "POST",
            "/orders",
            Some((
                "Shop\\OutOfStock",
                "item 3 is out of stock",
                "src/Shop/Orders.nvs",
                42,
            )),
        ),
    ];
    let mut lines = Vec::new();
    for (n, (method, route, exception)) in throws.into_iter().enumerate() {
        let start_us = base + 60_000_000 * n as u64;
        let mut request = json!({
            "writer": writer, "seq": n, "trace": format!("{:032x}", 0xabc0 + n), "at_us": start_us + 500,
            "kind": "request", "span": format!("{:016x}", 0xdef0 + n), "start_us": start_us,
            "duration_us": 500, "method": method, "route": route, "path": route,
            "status": if exception.is_some() { 500 } else { 200 }
        });
        if let Some((class, message, file, line)) = exception {
            request["exception"] = json!({
                "class": class, "message": message, "file": file, "line": line,
                "frames": [
                    { "function": "Shop\\Orders::create", "file": file, "line": line },
                    { "function": "Core\\Http\\Router::dispatch" }
                ]
            });
        }
        lines.push(request.to_string());
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
        ("kind%3Aquery", 2),
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
        19,
        "6 requests, 8 events, 3 dumps and 2 log lines"
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
    assert_eq!(rest["records"].as_array().map(Vec::len), Some(9));
    assert_eq!(service.json("/api/quarantine"), json!({ "files": [] }));

    let defaults = service.json("/api/settings");
    assert_eq!(
        defaults,
        json!({ "editor": "vscode", "path_mappings": [], "theme": "system", "filters": [] })
    );
    let saved = r#"{"editor":"custom","editor_template":"subl://open?url=file://{file}&line={line}","path_mappings":[{"from":"/app/","to":"/home/dev/shop/"}],"theme":"dark","filters":[{"name":"Slow orders","view":"search","query":"q=route%3A%2Forders+duration%3A%3E10ms"}]}"#;
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

// covers: tools:cli/the-debug-viewer
#[test]
fn an_exception_is_one_row_per_class_and_site_and_opens_to_its_requests() {
    let service = Service::ingested_from(&throwing_spool());

    let page = service.json("/api/exceptions");
    let groups = page["groups"].as_array().expect("a list");
    assert_eq!(groups.len(), 2, "{page}");
    assert_eq!(page["older"], Value::Null);
    // The newest throw is the first row's.
    let stock = &groups[0];
    assert_eq!(
        (
            &stock["class"],
            &stock["file"],
            &stock["line"],
            &stock["count"]
        ),
        (
            &json!("Shop\\OutOfStock"),
            &json!("src/Shop/Orders.nvs"),
            &json!(42),
            &json!(3)
        )
    );
    assert_eq!(stock["message"], "item 3 is out of stock");
    assert_eq!(stock["routes"], json!(["/orders", "/cart"]));
    assert_eq!(stock["sampled"], false);
    let chart: Vec<i64> = stock["chart"]
        .as_array()
        .expect("a chart")
        .iter()
        .map(|count| count.as_i64().expect("a count"))
        .collect();
    assert_eq!(chart.len(), 24);
    assert_eq!(chart.iter().sum::<i64>(), 3);
    assert_eq!(
        (chart[0], chart[23]),
        (1, 1),
        "the first and the last throw"
    );
    assert_eq!(groups[1]["class"], "Shop\\PaymentDeclined");
    assert_eq!(groups[1]["chart"][0], 1, "one throw is in the first count");

    let first = service.json("/api/exceptions?limit=1");
    let older = first["older"]
        .as_str()
        .expect("a page of one has a next page");
    let rest = service.json(&format!("/api/exceptions?before={older}"));
    assert_eq!(rest["groups"][0]["class"], "Shop\\PaymentDeclined");
    assert_eq!(rest["older"], Value::Null);

    let site = "class=Shop%5COutOfStock&file=src%2FShop%2FOrders.nvs&line=42";
    let one = service.json(&format!("/api/exceptions?{site}"));
    assert_eq!(one["group"]["count"], 3);
    assert_eq!(
        one["latest"]["frames"][0]["function"], "Shop\\Orders::create",
        "the group gives its latest throw's frames"
    );
    let paths: Vec<&str> = one["requests"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|row| row["path"].as_str().expect("a path"))
        .collect();
    assert_eq!(paths, ["/orders", "/cart", "/orders"], "newest first");
    let two = service.json(&format!("/api/exceptions?{site}&limit=2"));
    let older = two["older"]
        .as_i64()
        .expect("a page of two has an older page");
    let last = service.json(&format!("/api/exceptions?{site}&before={older}"));
    assert_eq!(last["requests"].as_array().map(Vec::len), Some(1));
    assert_eq!(last["older"], Value::Null);

    let (status, body) = service.get("/api/exceptions?class=Shop%5COutOfStock&file=x.nvs&line=1");
    assert_eq!(status, 404, "{body}");
    let (status, body) = service.get("/api/exceptions?class=Shop%5COutOfStock");
    assert_eq!(status, 400, "{body}");

    let written: Vec<String> = [("exceptions.json", &page), ("exception.json", &one)]
        .into_iter()
        .filter_map(|(name, value)| fixture(name, value))
        .collect();
    assert!(
        written.is_empty(),
        "{written:?} differed from the API's answers and are written again: commit them"
    );
}

#[test]
fn a_profile_the_request_named_downloads_by_its_row() {
    let speedscope = r#"{"$schema":"https://www.speedscope.app/file-format-schema.json"}"#;
    let service = Service::ingested_with(|data| {
        let path = data.join("run 1.speedscope.json");
        std::fs::write(&path, speedscope).expect("the profile is written");
        let trace = "4bf92f3577b34da6a3ce929d0e0e4736";
        let lines = [
            json!({ "writer": "1-1-0", "seq": 0, "trace": trace, "at_us": 1_000,
                    "kind": "profile", "path": path.display().to_string() }),
            json!({ "writer": "1-1-0", "seq": 1, "trace": trace, "at_us": 2_000,
                    "kind": "request", "span": "00f067aa0ba902b7", "start_us": 900,
                    "duration_us": 1_100, "method": "GET", "path": "/", "status": 200 }),
        ];
        lines.map(|line| line.to_string()).join("\n") + "\n"
    });
    let detail = service.json("/api/request?trace=4bf92f3577b34da6a3ce929d0e0e4736");
    let id = detail["profiles"][0]["id"].as_i64().expect("one profile");
    let response = service.send("GET", &format!("/api/profile?id={id}"), None);
    assert_eq!(response, (200, speedscope.to_owned()));
}

#[test]
fn coverage_reads_only_the_files_it_names_under_the_project_root() {
    let outside = nvs_repo::scratch_private("outside");
    let stray = outside.join("secret.nvs");
    std::fs::write(&stray, "$key = 'not for the viewer';\n").expect("the file is written");
    let stray = stray.display().to_string();
    let service = Service::ingested_with(|data| {
        std::fs::create_dir_all(data.join("shop")).expect("the folder is made");
        std::fs::write(
            data.join("shop").join("items.nvs"),
            "function stock(int $id): int {\n    if ($id > 5) {\n        return 0;\n    }\n    return 3;\n}\necho stock(7);\n",
        )
        .expect("the source is written");
        let trace = "4bf92f3577b34da6a3ce929d0e0e4736";
        let span = "00f067aa0ba902b7";
        let files = json!({
            "shop/items.nvs": { "2": 1, "3": 1, "7": 1 },
            stray.clone(): { "1": 1 },
        });
        let lines = [
            json!({ "writer": "1-1-0", "seq": 0, "trace": trace, "at_us": 2_000,
                    "kind": "request", "span": span, "start_us": 900,
                    "duration_us": 1_100, "method": "GET", "path": "/items/7", "status": 200 }),
            json!({ "writer": "1-1-0", "seq": 1, "trace": trace, "at_us": 2_000,
                    "kind": "coverage", "span": span, "files": files }),
        ];
        lines.map(|line| line.to_string()).join("\n") + "\n"
    });
    let detail = service.json("/api/request?trace=4bf92f3577b34da6a3ce929d0e0e4736");
    assert_eq!(detail["coverage"]["files"], 2, "{detail}");
    let id = detail["coverage"]["id"]
        .as_i64()
        .expect("the request has coverage");

    let coverage = service.json(&format!("/api/coverage?id={id}"));
    assert_eq!(coverage["start_us"], 900);
    assert_eq!(coverage["files"]["shop/items.nvs"]["3"], 1);
    let source = service.json(&format!("/api/source?coverage={id}&file=shop%2Fitems.nvs"));
    assert!(
        source["text"]
            .as_str()
            .is_some_and(|text| text.contains("return 0;")),
        "{source}"
    );
    assert_eq!(source["changed"], true, "written after the request started");

    let escaped = |file: &str| {
        file.bytes()
            .map(|byte| match byte {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' => {
                    char::from(byte).to_string()
                }
                byte => format!("%{byte:02X}"),
            })
            .collect::<String>()
    };
    let (status, body) = service.get(&format!(
        "/api/source?coverage={id}&file={}",
        escaped(&stray)
    ));
    assert_eq!(status, 403, "a file outside the project root: {body}");
    assert!(!body.contains("not for the viewer"), "{body}");
    let (status, body) = service.get(&format!(
        "/api/source?coverage={id}&file=..%2Foutside%2Fsecret.nvs"
    ));
    assert_eq!(status, 404, "a file the coverage does not name: {body}");
    let (status, body) = service.get(&format!("/api/source?coverage={id}"));
    assert_eq!(status, 400, "{body}");
    let (status, body) = service.get("/api/coverage?id=99");
    assert_eq!(status, 404, "{body}");

    let mut coverage_fixture = coverage.clone();
    if let Some(files) = coverage_fixture["files"].as_object_mut() {
        files.remove(&stray);
    }
    let written: Vec<String> = [
        ("coverage.json", &coverage_fixture),
        ("source.json", &source),
    ]
    .into_iter()
    .filter_map(|(name, value)| fixture(name, value))
    .collect();
    assert!(
        written.is_empty(),
        "{written:?} differed from the API's answers and are written again: commit them"
    );
}

// covers: tools:cli/the-debug-viewer
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
    assert_eq!(
        detail["exception"]["frames"][0]["function"],
        "Shop\\Stock::take"
    );
    assert_eq!(detail["exception"]["frames"][2]["file"], Value::Null);
    assert_eq!(
        detail["exception"]["previous"][0]["class"],
        "Shop\\StockError"
    );
    let kinds: Vec<&str> = detail["events"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|event| event["kind"].as_str().expect("a kind"))
        .collect();
    assert_eq!(kinds, ["call", "query", "gc", "spawn"], "in order of start");
    assert_eq!(
        detail["events"][1]["query"]["site"],
        json!({ "file": "src/Shop/Items.nvs", "line": 12, "hash": "ab12" })
    );
    assert_eq!(detail["dumps"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        detail["dumps"][1]["tree"]["$properties"]["card"],
        json!({ "$redacted": true })
    );
    assert_eq!(detail["logs"][0]["message"], "stock is empty");
    assert_eq!(detail["input"]["query"], "coupon=SPRING");
    assert_eq!(
        detail["input"]["headers"][1],
        json!(["authorization", "[redacted]"])
    );
    assert_eq!(
        detail["input"]["body"]["text"],
        "{\"item\":7,\"quantity\":2}"
    );
    assert_eq!(
        detail["input"]["response_body"]["base64"], "T3V0IG9mIP9zdG9jaw==",
        "the parts are joined, and bytes that are not UTF-8 are base64"
    );
    assert_eq!(
        detail["cut"],
        json!({ "events": false, "dumps": false, "logs": false, "profiles": false, "copies": false })
    );
    let lines: Vec<i64> = detail["copies"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|copy| copy["line"].as_i64().expect("a line"))
        .collect();
    assert_eq!(lines, [36, 30], "the site that copied the most comes first");
    assert_eq!(
        detail["copies"][0]["help"],
        nvs_ir::copies::Reason::SlotKept.help()
    );
    assert_eq!(
        detail["profiles"][0]["path"],
        "/srv/shop/orders.speedscope.json"
    );
    let profile = detail["profiles"][0]["id"].as_i64().expect("an id");
    let (status, body) = service.get(&format!("/api/profile?id={profile}"));
    assert_eq!(status, 404, "the file is not on disk: {body}");
    let (status, body) = service.get("/api/profile?id=x");
    assert_eq!(status, 400, "{body}");
    let (status, body) = service.get("/api/profile?path=/etc/passwd");
    assert_eq!(status, 400, "a page names a row, never a path: {body}");

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
    assert_eq!(put["profiles"], json!([]));
    assert_eq!(
        put["copies"],
        json!([]),
        "another trace's copies are not its own"
    );
    assert_eq!(put["logs"], json!([]));
    assert_eq!(
        put["input"]["body"],
        json!({ "text": "{\"stock\":3}", "base64": null, "read": true, "cut": false, "withheld": false }),
        "its body is its own two parts"
    );
    assert_eq!(missing["input"], Value::Null, "it recorded no input");
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

/// The paths of a filtered page of `GET /api/requests`, newest first.
fn paths_of(service: &Service, query: &str) -> Vec<String> {
    let page = service.json(&format!("/api/requests?{query}"));
    assert_eq!(page["older"], Value::Null, "{query}: one page holds them");
    page["requests"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|row| row["path"].as_str().expect("a path").to_owned())
        .collect()
}

#[test]
fn the_list_filters_and_each_row_has_its_time_by_kind_and_its_repeats() {
    let service = Service::ingested();

    assert_eq!(
        paths_of(&service, "method=GET"),
        ["/missing", "/items/7", "/"]
    );
    assert_eq!(paths_of(&service, "status=5xx"), ["/orders"]);
    assert_eq!(paths_of(&service, "status=404"), ["/missing"]);
    assert_eq!(
        paths_of(&service, "status=2xx"),
        ["/items/7", "/items/7", "/"]
    );
    assert_eq!(
        paths_of(&service, "route=%2Fitems%2F%7Bid%7D"),
        ["/items/9", "/items/7", "/items/7"]
    );
    assert_eq!(
        paths_of(&service, "method=PUT&route=%2Fitems%2F%7Bid%7D"),
        ["/items/7"]
    );
    assert_eq!(paths_of(&service, "min_us=3000"), ["/items/7", "/orders"]);
    assert_eq!(paths_of(&service, "min_memory=4000000"), ["/orders"]);
    assert_eq!(paths_of(&service, "min_memory=1"), ["/orders", "/items/7"]);
    assert_eq!(paths_of(&service, "threw=true"), ["/orders"]);
    assert_eq!(paths_of(&service, "threw=false").len(), 5);
    for wrong in [
        "status=6xx",
        "status=99",
        "method=get",
        "threw=yes",
        "min_us=-1",
        "min_memory=-1",
    ] {
        let (status, body) = service.get(&format!("/api/requests?{wrong}"));
        assert_eq!(status, 400, "{wrong}: {body}");
    }

    let rows = service.json("/api/requests");
    let row = |path: &str, method: &str| {
        rows["requests"]
            .as_array()
            .expect("a list")
            .iter()
            .find(|row| row["path"] == path && row["method"] == method)
            .cloned()
            .expect("the request")
    };
    let looped = row("/items/7", "GET");
    assert_eq!(
        looped["time_us"],
        json!({ "query": 600, "http": 1000, "cache": 0, "session": 0, "file": 0, "gc": 0, "spawn": 0 }),
        "a query that arrived after the request's line is counted too"
    );
    assert_eq!(
        (&looped["queries"], &looped["repeats"]),
        (&json!(3), &json!(3))
    );
    let threw = row("/orders", "POST");
    assert_eq!(
        threw["time_us"],
        json!({ "query": 800, "http": 0, "cache": 0, "session": 0, "file": 0, "gc": 600, "spawn": 5000 }),
        "a call is the program's own time and is not summed"
    );
    assert_eq!(threw["repeats"], 1);
    assert_eq!(row("/", "GET")["repeats"], 0);
    assert_eq!(
        (
            &threw["memory_peak"],
            &threw["memory_limit"],
            &threw["handler"],
            &threw["bytes_in"],
            &threw["bytes_out"],
        ),
        (
            &json!(8_388_608),
            &json!(134_217_728),
            &json!("Shop\\Orders::create"),
            &json!(23),
            &json!(13),
        ),
        "the row has the memory, the handler and the sizes its line recorded"
    );
    assert_eq!(
        (&looped["memory_peak"], &looped["memory_limit"]),
        (&json!(2_097_152), &Value::Null),
        "a request with no ceiling has a peak and no limit"
    );
    let missing = row("/missing", "GET");
    assert_eq!(
        (
            &missing["memory_peak"],
            &missing["handler"],
            &missing["bytes_out"]
        ),
        (&Value::Null, &Value::Null, &Value::Null),
        "a line without the fields reads with each null"
    );

    let until = 1_700_000_300_000_000_i64;
    let summary = service.json(&format!("/api/summary?until={until}"));
    assert_eq!(
        (&summary["count"], &summary["errors"], &summary["cut"]),
        (&json!(6), &json!(1), &json!(false))
    );
    assert_eq!(
        (&summary["p50_us"], &summary["p95_us"], &summary["max_us"]),
        (&json!(840), &json!(15_400), &json!(15_400))
    );
    let buckets = summary["buckets"].as_array().expect("a list");
    assert_eq!(buckets.len(), 12);
    assert_eq!(buckets[0], json!({ "count": 6, "errors": 1 }));
    let later = service.json(&format!("/api/summary?until={}", until + 600_000_000));
    assert_eq!(
        (&later["count"], &later["p50_us"]),
        (&json!(0), &Value::Null)
    );

    let routes = service.json("/api/routes");
    let listed = routes["routes"].as_array().expect("a list");
    assert_eq!(listed.len(), 6, "one row per method and route");
    let orders = listed
        .iter()
        .find(|route| route["route"] == "/orders")
        .expect("POST /orders");
    assert_eq!(
        (&orders["count"], &orders["errors"], &orders["p95_us"]),
        (&json!(1), &json!(1), &json!(15_400))
    );
    assert!(
        listed
            .iter()
            .any(|route| route["route"].is_null() && route["method"] == "GET"),
        "the requests no route matched are one row"
    );
    assert_eq!(
        orders["durations"], 1,
        "its percentiles stand on one request"
    );
    assert_eq!(
        (&routes["cut"], &routes["partial"]),
        (&json!(false), &json!(false))
    );

    let statements = service.json("/api/statements");
    assert_eq!(
        (&statements["cut"], &statements["partial"]),
        (&json!(false), &json!(false))
    );
    let sites: Vec<(i64, i64, i64, i64)> = statements["statements"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|site| {
            let n = |key: &str| site[key].as_i64().expect("a number");
            (n("line"), n("count"), n("total_us"), n("repeated"))
        })
        .collect();
    assert_eq!(
        sites,
        [(12, 1, 800, 0), (20, 3, 600, 1)],
        "the most total time first"
    );
    assert_eq!(
        statements["statements"][1]["sql"],
        "SELECT price FROM items WHERE id = ?"
    );

    let health = service.json("/api/health");
    assert_eq!(
        health,
        json!({ "dropped": 0, "dropped_records": 0, "unparseable": 0, "quarantine": 0, "replay": false })
    );
    for path in [
        "/api/routes?x=1",
        "/api/statements?x=1",
        "/api/health?x=1",
        "/api/summary?x=1",
    ] {
        let (status, body) = service.get(path);
        assert_eq!(status, 400, "{path}: {body}");
    }

    let written: Vec<String> = [
        ("summary.json", &summary),
        ("routes.json", &routes),
        ("statements.json", &statements),
        ("health.json", &health),
    ]
    .into_iter()
    .filter_map(|(name, value)| fixture(name, value))
    .collect();
    assert!(
        written.is_empty(),
        "{written:?} differed from the API's answers and are written again: commit them"
    );
}

/// A spool of three runs, one of each kind, each with an event, a dump and a log line under its
/// own trace, and one request. The queue job and the request throw the same exception.
fn runs_spool() -> String {
    let writer = "4343-1700000000000-0";
    let base: u64 = 1_700_000_000_000_000;
    let mail_down = json!({
        "class": "Shop\\MailDown", "message": "the mail server did not answer",
        "file": "jobs/receipt.nvs", "line": 9,
        "frames": [{ "function": "{main}", "file": "jobs/receipt.nvs", "line": 9 }]
    });
    let mut lines = Vec::new();
    let mut seq = 0_u64;
    let mut line = |mut value: Value, trace: &str, at_us: u64| {
        value["writer"] = json!(writer);
        value["seq"] = json!(seq);
        value["trace"] = json!(trace);
        value["at_us"] = json!(at_us);
        lines.push(value.to_string());
        seq += 1;
    };
    let runs = [
        ("schedule", "nightly-report", "ok", 2_000, 1_048_576),
        ("queue", "jobs/receipt.nvs", "failed", 8_400, 524_288),
        ("command", "run scripts/import.nvs", "ok", 30_000, 4_194_304),
    ];
    for (n, (of, name, outcome, duration_us, peak)) in runs.into_iter().enumerate() {
        let trace = format!("{:032x}", 0x5100 + n);
        let start_us = base + 60_000_000 * n as u64;
        line(
            json!({ "kind": "event", "event": "query", "label": "sqlite main SELECT 1",
                    "start_us": start_us + 100, "end_us": start_us + 400,
                    "query": { "driver": "sqlite", "sql": "SELECT 1", "rows": 1 } }),
            &trace,
            start_us + 400,
        );
        line(dump(7, &json!({ "run": n })), &trace, start_us + 500);
        line(
            json!({ "kind": "log", "level": "info", "message": format!("{of} step"),
                    "fields": {}, "elided": false }),
            &trace,
            start_us + 600,
        );
        let mut run = json!({
            "kind": "run", "of": of, "name": name, "start_us": start_us,
            "duration_us": duration_us, "outcome": outcome,
            "memory_peak": peak, "memory_limit": 67_108_864
        });
        if outcome == "failed" {
            run["exception"] = mail_down.clone();
        }
        line(run, &trace, start_us + duration_us);
    }
    let start_us = base + 300_000_000;
    let mut request = json!({
        "kind": "request", "span": "00000000000052ff", "start_us": start_us,
        "duration_us": 500, "method": "POST", "route": "/orders", "path": "/orders", "status": 500
    });
    request["exception"] = mail_down;
    line(request, &format!("{:032x}", 0x52ff), start_us + 500);
    lines.join("\n") + "\n"
}

// covers: tools:cli/the-debug-viewer
#[test]
fn the_runs_list_pages_and_filters_and_a_run_opens_to_its_records_and_its_exception() {
    let service = Service::ingested_from(&runs_spool());

    let page = service.json("/api/runs");
    let names: Vec<&str> = page["runs"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|run| run["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(
        names,
        [
            "run scripts/import.nvs",
            "jobs/receipt.nvs",
            "nightly-report"
        ],
        "newest first: {page}"
    );
    assert_eq!(page["older"], Value::Null);
    let queue = &page["runs"][1];
    assert_eq!(
        (
            &queue["of"],
            &queue["outcome"],
            &queue["memory_peak"],
            &queue["memory_limit"]
        ),
        (
            &json!("queue"),
            &json!("failed"),
            &json!(524_288),
            &json!(67_108_864)
        )
    );
    assert_eq!(
        (&queue["queries"], &queue["dumps"], &queue["logs"]),
        (&json!(1), &json!(1), &json!(1)),
        "the triggers count a run's records"
    );
    assert_eq!(queue["time_us"]["query"], 300);
    assert_eq!(queue["exception"]["class"], "Shop\\MailDown");

    let first = service.json("/api/runs?limit=1");
    let older = first["older"]
        .as_i64()
        .expect("a page of one has an older page");
    let rest = service.json(&format!("/api/runs?before={older}"));
    assert_eq!(rest["runs"].as_array().map(Vec::len), Some(2));

    let count = |query: &str| {
        service.json(&format!("/api/runs?{query}"))["runs"]
            .as_array()
            .map_or(0, Vec::len)
    };
    assert_eq!(count("of=schedule"), 1);
    assert_eq!(count("outcome=failed"), 1);
    assert_eq!(count("name=run%20scripts%2Fimport.nvs"), 1);
    assert_eq!(count("min_us=8000"), 2);
    assert_eq!(count("min_memory=1048576"), 2);
    for query in ["of=request", "outcome=crashed", "method=GET", "limit=0"] {
        let (status, body) = service.get(&format!("/api/runs?{query}"));
        assert_eq!(status, 400, "{query}: {body}");
    }

    let trace = queue["trace"].as_str().expect("a trace id");
    let one = service.json(&format!("/api/run?trace={trace}"));
    assert_eq!(one["run"]["name"], "jobs/receipt.nvs");
    assert_eq!(
        one["exception"]["message"],
        "the mail server did not answer"
    );
    assert_eq!(one["exception"]["frames"][0]["function"], "{main}");
    assert_eq!(one["events"][0]["kind"], "query");
    assert_eq!(one["dumps"][0]["tree"], json!({ "run": 1 }));
    assert_eq!(one["logs"][0]["message"], "queue step");
    assert_eq!(
        (
            one["events"].as_array().map(Vec::len),
            one["dumps"].as_array().map(Vec::len),
            one["logs"].as_array().map(Vec::len)
        ),
        (Some(1), Some(1), Some(1)),
        "a run has only its own trace's records"
    );
    let (status, body) = service.get(&format!("/api/run?trace={:032x}", 0x9999));
    assert_eq!(status, 404, "{body}");
    let (status, body) = service.get("/api/run?id=1");
    assert_eq!(status, 400, "{body}");

    // The run's throw and the request's are one group, and the group's page lists both.
    let site = "class=Shop%5CMailDown&file=jobs%2Freceipt.nvs&line=9";
    let group = service.json(&format!("/api/exceptions?{site}"));
    assert_eq!(group["group"]["count"], 2, "{group}");
    assert_eq!(
        group["group"]["chart"]
            .as_array()
            .map(|chart| chart.iter().filter_map(Value::as_i64).sum::<i64>()),
        Some(2)
    );
    assert_eq!(group["requests"].as_array().map(Vec::len), Some(1));
    assert_eq!(group["runs"][0]["name"], "jobs/receipt.nvs");

    let written: Vec<String> = [("runs.json", &page), ("run.json", &one)]
        .into_iter()
        .filter_map(|(name, value)| fixture(name, value))
        .collect();
    assert!(
        written.is_empty(),
        "{written:?} differed from the API's answers and are written again: commit them"
    );
}

/// `[debug] index_age` deletes a long run's records that are older than seven days, and records
/// their newest time as the index's cut. `/api/run` gives the cut for a run that started before
/// it, and `null` for a run that started after it. The data folder is ingested and its window
/// applied by `nvs debug --once` before the service starts, so the service reads a finished
/// deletion.
// covers: tools:cli/the-debug-viewer
#[test]
fn a_run_that_started_before_the_cut_returns_it() {
    const HOUR_US: u64 = 3_600_000_000;
    let now = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock is past 1970")
            .as_micros(),
    )
    .expect("the time fits");
    let writer = "4545-1700000000000-0";
    let long = format!("{:032x}", 0x5300);
    let short = format!("{:032x}", 0x5301);
    let mut lines = Vec::new();
    let mut seq = 0_u64;
    let mut line = |mut value: Value, trace: &str, at_us: u64| {
        value["writer"] = json!(writer);
        value["seq"] = json!(seq);
        value["trace"] = json!(trace);
        value["at_us"] = json!(at_us);
        lines.push(value.to_string());
        seq += 1;
    };
    let log = |message: &str| json!({ "kind": "log", "level": "info", "message": message, "fields": {}, "elided": false });
    let long_start = now - 240 * HOUR_US;
    let deleted_at = now - 216 * HOUR_US;
    line(log("an old step"), &long, deleted_at);
    line(log("a new step"), &long, now - 2 * HOUR_US);
    line(
        json!({ "kind": "run", "of": "command", "name": "run scripts/import.nvs",
                "start_us": long_start, "duration_us": 239 * HOUR_US, "outcome": "ok" }),
        &long,
        now - HOUR_US,
    );
    let short_start = now - HOUR_US / 2;
    line(log("a short step"), &short, short_start + 100);
    line(
        json!({ "kind": "run", "of": "schedule", "name": "nightly-report",
                "start_us": short_start, "duration_us": 1_000, "outcome": "ok" }),
        &short,
        short_start + 1_000,
    );
    let text = lines.join("\n") + "\n";

    let data = nvs_repo::scratch_private("nvsdata");
    std::fs::write(data.join("nvs.toml"), "[debug]\nindex_age = \"7d\"\n")
        .expect("the configuration is written");
    let once = || {
        let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .current_dir(&*data)
            .arg("--data")
            .arg(&*data)
            .arg("--no-init")
            .args(["debug", "--once"])
            .output()
            .expect("the `nvs` binary this test was built beside runs");
        assert!(
            out.status.success(),
            "`nvs debug --once` succeeds: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    once();
    std::fs::write(data.join("debug").join("spool").join("1-1-0.0.spool"), text)
        .expect("the spool is writable");
    once();
    let service = Service::start_on(data);

    let run = service.json(&format!("/api/run?trace={long}"));
    assert_eq!(run["cut_us"], json!(deleted_at), "{run}");
    let messages: Vec<&str> = run["logs"]
        .as_array()
        .expect("a list")
        .iter()
        .filter_map(|log| log["message"].as_str())
        .collect();
    assert_eq!(messages, ["a new step"], "only the old record is deleted");

    let run = service.json(&format!("/api/run?trace={short}"));
    assert_eq!(run["cut_us"], Value::Null, "{run}");
    assert_eq!(run["logs"].as_array().map(Vec::len), Some(1));
}

/// A one-pixel PNG, as base64.
const PIXEL: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

/// Two messages a capture block kept: a styled HTML receipt with an inline image and a file,
/// and a plain-text alert.
fn mails_spool() -> String {
    let writer = "4444-1700000000000-0";
    let base: u64 = 1_700_000_000_000_000;
    let receipt = json!({
        "kind": "mail", "endpoint": "outbox",
        "message_id": "5f0c2a9e4b7d41e3a8c6d2f1e0b9a7c3@example.com",
        "from": "noreply@example.com", "from_name": "Shop",
        "to": ["ada@example.com", "bo@example.com"], "cc": ["desk@example.com"],
        "bcc": ["archive@example.com"], "reply_to": "help@example.com",
        "subject": "Your receipt",
        "text": "Thank you for your order.\n\nView it online (https://example.com/o/7)\n",
        "html_written": "<style>p { color: #333 } .dark p { color: #eee }</style>\
                         <p>Thank you for your order.</p><img src=\"cid:4f1d@inline\" alt=\"Shop\">",
        "html": "<style>.dark p { color: #eee; }</style><p style=\"color: #333\">Thank you for \
                 your order.</p><img src=\"cid:4f1d@inline\" alt=\"Shop\">",
        "kept_css": [".dark p { color: #eee; }"],
        "unused_css": [".dark p { color: #eee; }"],
        "attachments": [
            { "name": "logo.png", "type": "image/png", "content_id": "4f1d@inline",
              "size": 70, "base64": PIXEL },
            { "name": "receipt.csv", "type": "text/csv", "size": 10,
              "base64": "ZGF5LHRvdGFsCg==" }
        ],
        "in_reply_to": "0a1b@example.com", "references": ["0a1b@example.com"],
        "unsubscribe": "https://example.com/unsubscribe/7",
        "auto_submitted": true, "bounce_tag": "order-7"
    });
    let alert = json!({
        "kind": "mail", "endpoint": "alerts",
        "message_id": "9e8d7c6b5a4f3e2d1c0b9a8f7e6d5c4b@example.com",
        "from": "alerts@example.com", "to": ["ops@example.com"],
        "subject": "Disk almost full", "text": "The disk is 91% full.\n",
        "auto_submitted": true
    });
    [receipt, alert]
        .into_iter()
        .enumerate()
        .map(|(n, mut line)| {
            line["writer"] = json!(writer);
            line["seq"] = json!(n);
            line["trace"] = json!(format!("{:032x}", 0x6100 + n));
            line["at_us"] = json!(base + 1_000_000 * n as u64);
            line.to_string() + "\n"
        })
        .collect()
}

// covers: tools:cli/the-debug-viewer
#[test]
fn the_mail_list_pages_newest_first_and_a_message_opens_to_its_whole_line() {
    let service = Service::ingested_from(&mails_spool());

    let page = service.json("/api/mails");
    let subjects: Vec<&str> = page["mails"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|mail| mail["subject"].as_str().expect("a subject"))
        .collect();
    assert_eq!(subjects, ["Disk almost full", "Your receipt"], "{page}");
    assert_eq!(page["older"], Value::Null);
    let receipt = &page["mails"][1];
    assert_eq!(receipt["endpoint"], "outbox");
    assert_eq!(receipt["recipients"], "ada@example.com, bo@example.com");
    assert!(
        receipt.get("message").is_none(),
        "the list carries no message body: {receipt}"
    );

    let first = service.json("/api/mails?limit=1");
    let older = first["older"]
        .as_i64()
        .expect("a page of one has an older page");
    let rest = service.json(&format!("/api/mails?before={older}"));
    assert_eq!(rest["mails"][0]["subject"], "Your receipt");
    for query in ["limit=0", "subject=hi", "before=x"] {
        let (status, body) = service.get(&format!("/api/mails?{query}"));
        assert_eq!(status, 400, "{query}: {body}");
    }

    let id = receipt["id"].as_i64().expect("an id");
    let one = service.json(&format!("/api/mail?id={id}"));
    assert_eq!(one["id"], id);
    let message = &one["message"];
    assert_eq!(message["from_name"], "Shop");
    assert_eq!(message["bcc"], json!(["archive@example.com"]));
    assert_eq!(message["kept_css"], json!([".dark p { color: #eee; }"]));
    assert_eq!(message["attachments"][0]["content_id"], "4f1d@inline");
    assert_eq!(message["attachments"][1]["base64"], "ZGF5LHRvdGFsCg==");
    let (status, body) = service.get("/api/mail?id=9999");
    assert_eq!(status, 404, "{body}");
    let (status, body) = service.get("/api/mail?trace=1");
    assert_eq!(status, 400, "{body}");

    let written: Vec<String> = [("mails.json", &page), ("mail.json", &one)]
        .into_iter()
        .filter_map(|(name, value)| fixture(name, value))
        .collect();
    assert!(
        written.is_empty(),
        "{written:?} differed from the API's answers and are written again: commit them"
    );
}
