//! `nvs dap` driven by a scripted client: a launched program's output arrives as `output` events,
//! its end as `exited` and `terminated`, and every byte the adapter writes on stdout belongs to a
//! frame (`rule:ide/stdout-belongs-to-the-protocol`), even when the program prints one.
//!
//! Through the built binary, because the property is the process's: the program runs inside the
//! adapter, and what it writes to "standard output" must reach the client as text and never as a
//! frame.

use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{Value, json};

/// A running `nvs dap` and the frames it has written so far.
struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    frames: Receiver<Value>,
    /// The frames [`Client::ask`] read before the response it waited for. An event can arrive
    /// there, and [`Client::until_event`] reads these first, so it never waits for an event that
    /// has already arrived.
    pending: VecDeque<Value>,
    reader: JoinHandle<Result<usize, String>>,
    seq: i64,
    /// Every frame the adapter wrote so far, as its JSON text, in the order it arrived.
    transcript: Arc<Mutex<Vec<String>>>,
}

impl Client {
    /// Starts `nvs dap` in `dir`, with the data folder inside it.
    fn start(dir: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(dir.join(".nvsdata"))
            .arg("dap")
            .current_dir(dir)
            .env_remove("NOVIS_NO_INIT")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the `nvs` binary this test was built beside runs");
        let stdout = child.stdout.take().expect("stdout was piped");
        let stdin = child.stdin.take();
        let (sender, frames) = channel();
        let transcript = Arc::new(Mutex::new(Vec::new()));
        let written = Arc::clone(&transcript);
        let reader = std::thread::spawn(move || {
            let mut stdout = BufReader::new(stdout);
            let mut count = 0;
            loop {
                match strict_frame(&mut stdout)? {
                    Some(frame) => {
                        count += 1;
                        written
                            .lock()
                            .expect("the transcript is never poisoned")
                            .push(frame.to_string());
                        let _ = sender.send(frame);
                    }
                    None => return Ok(count),
                }
            }
        });
        Self {
            child,
            stdin,
            frames,
            pending: VecDeque::new(),
            reader,
            seq: 1,
            transcript,
        }
    }

    /// Sends one request.
    fn request(&mut self, command: &str, arguments: Value) {
        let body = json!({ "seq": self.seq, "type": "request", "command": command, "arguments": arguments })
            .to_string();
        self.seq += 1;
        let stdin = self.stdin.as_mut().expect("stdin is still open");
        write!(stdin, "Content-Length: {}\r\n\r\n{body}", body.len())
            .expect("the adapter reads stdin");
        stdin.flush().expect("the adapter reads stdin");
    }

    /// Every message up to and including the first event named `event`, the ones
    /// [`Client::ask`] already read included.
    fn until_event(&mut self, event: &str) -> Vec<Value> {
        let mut seen = Vec::new();
        loop {
            let frame = self.pending.pop_front().unwrap_or_else(|| {
                self.frames
                    .recv()
                    .unwrap_or_else(|_| panic!("stdout ended before a `{event}` event: {seen:#?}"))
            });
            let done = frame["type"] == "event" && frame["event"] == event;
            seen.push(frame);
            if done {
                return seen;
            }
        }
    }

    /// Sends one request and gives its response's body. Every message before the response is
    /// kept for [`Client::until_event`]. Fails if the request failed.
    fn ask(&mut self, command: &str, arguments: Value) -> Value {
        self.request(command, arguments);
        loop {
            let frame = self.frames.recv().unwrap_or_else(|_| {
                panic!(
                    "stdout ended before the `{command}` response: {:#?}",
                    self.pending
                )
            });
            if frame["type"] == "response" && frame["command"] == command {
                assert_eq!(frame["success"], true, "`{command}` failed: {frame:#?}");
                return frame["body"].clone();
            }
            self.pending.push_back(frame);
        }
    }

    /// Closes stdin, waits for the adapter to end, and gives the frames it wrote after the last
    /// read and its exit status. Fails if any byte on stdout was not part of a frame.
    fn finish(mut self) -> (Vec<Value>, bool) {
        drop(self.stdin.take());
        let status = self.child.wait().expect("the adapter ends");
        let read = self.reader.join().expect("the reader thread ends");
        if let Err(stray) = read {
            panic!("stdout carried bytes outside a frame: {stray}");
        }
        let pending = std::mem::take(&mut self.pending);
        (
            pending.into_iter().chain(self.frames.try_iter()).collect(),
            status.success(),
        )
    }
}

/// Reads one frame the way a strict client does: a `Content-Length` line, a blank line, and a JSON
/// body of exactly that length. Anything else on the stream is an error naming what was found.
fn strict_frame(stdout: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut header = String::new();
    if stdout.read_line(&mut header).map_err(|e| e.to_string())? == 0 {
        return Ok(None);
    }
    let length: usize = header
        .strip_prefix("Content-Length: ")
        .and_then(|rest| rest.strip_suffix("\r\n"))
        .and_then(|number| number.parse().ok())
        .ok_or_else(|| format!("a header line {header:?}"))?;
    let mut blank = String::new();
    stdout.read_line(&mut blank).map_err(|e| e.to_string())?;
    if blank != "\r\n" {
        return Err(format!("a second header line {blank:?}"));
    }
    let mut body = vec![0; length];
    stdout.read_exact(&mut body).map_err(|e| e.to_string())?;
    serde_json::from_slice(&body).map(Some).map_err(|_| {
        format!(
            "a body that is not JSON: {:?}",
            String::from_utf8_lossy(&body)
        )
    })
}

/// The text of every `output` event of `category` in `frames`, joined.
fn output(frames: &[Value], category: &str) -> String {
    frames
        .iter()
        .filter(|frame| frame["event"] == "output" && frame["body"]["category"] == category)
        .filter_map(|frame| frame["body"]["output"].as_str())
        .collect()
}

/// Initializes, launches `program` in `run` mode and configures, then reads up to `terminated`.
fn launch_and_run(dir: &Path, program: &str) -> (Client, Vec<Value>) {
    let mut client = Client::start(dir);
    client.request("initialize", json!({ "adapterID": "novis" }));
    let initialized = client.until_event("initialized");
    let capabilities = &initialized[0];
    assert_eq!(capabilities["command"], "initialize");
    assert_eq!(capabilities["success"], true);
    assert_eq!(
        capabilities["body"]["supportsConfigurationDoneRequest"],
        true
    );
    client.request(
        "launch",
        json!({ "mode": "run", "program": dir.join(program) }),
    );
    client.request("configurationDone", json!({}));
    let ran = client.until_event("terminated");
    (client, ran)
}

/// A launched script's stdout arrives as `output` events, a line shaped like a DAP frame among
/// it, and its end is `exited` with status 0 and then `terminated`. Every byte the adapter wrote
/// was part of a frame, and `disconnect` ends it.
// covers: tools:editor/nvs-dap
#[test]
fn a_launched_script_s_output_arrives_as_events_and_stdout_carries_frames_only() {
    let dir = nvs_repo::scratch_private("dap-output");
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\necho \"hello\\n\";\necho \"Content-Length: 2\\r\\n\\r\\n\", '{}';\n",
    )
    .unwrap();

    let (mut client, ran) = launch_and_run(&dir, "main.nvs");
    let stdout = output(&ran, "stdout");
    assert!(stdout.contains("hello\n"), "{ran:#?}");
    // `echo` writes a carriage return as `␍` on any terminal stream, so the line arrives as text.
    assert!(
        stdout.contains("Content-Length: 2\u{240D}\n\u{240D}\n{}"),
        "{ran:#?}"
    );
    let exited = ran
        .iter()
        .find(|frame| frame["event"] == "exited")
        .expect("an `exited` event before `terminated`");
    assert_eq!(exited["body"]["exitCode"], 0);

    client.request("disconnect", json!({}));
    let (rest, success) = client.finish();
    assert!(
        rest.iter()
            .any(|frame| frame["command"] == "disconnect" && frame["success"] == true),
        "{rest:#?}"
    );
    assert!(success, "the adapter ended with a failure");
}

/// A breakpoint set before the compile is verified once the unit loads, stops the program on its
/// line with the values the variables have there, and `continue` runs the program to its end.
#[test]
fn a_breakpoint_stops_a_launched_script_with_its_variables_and_continue_ends_it() {
    let dir = nvs_repo::scratch_private("dap-breakpoint");
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nint $price = 4;\nint $total = $price * 3;\necho $total, \"\\n\";\n",
    )
    .unwrap();
    let main = dir.join("main.nvs");

    let mut client = Client::start(&dir);
    client.request("initialize", json!({ "adapterID": "novis" }));
    client.until_event("initialized");
    let set = client.ask(
        "setBreakpoints",
        json!({ "source": { "path": main }, "breakpoints": [{ "line": 4 }] }),
    );
    let id = set["breakpoints"][0]["id"].clone();
    assert_eq!(set["breakpoints"][0]["verified"], false, "{set:#?}");
    client.request("launch", json!({ "mode": "run", "program": main }));
    client.request("configurationDone", json!({}));

    let ran = client.until_event("stopped");
    assert!(
        ran.iter().any(|frame| frame["event"] == "breakpoint"
            && frame["body"]["breakpoint"]["id"] == id
            && frame["body"]["breakpoint"]["verified"] == true),
        "the load did not verify the breakpoint: {ran:#?}"
    );
    let stopped = ran.last().expect("the `stopped` event");
    assert_eq!(stopped["body"]["reason"], "breakpoint");
    assert_eq!(
        output(&ran, "stdout"),
        "",
        "the program ran past its breakpoint"
    );

    let threads = client.ask("threads", json!({}));
    let thread = threads["threads"][0]["id"].clone();
    let trace = client.ask("stackTrace", json!({ "threadId": thread }));
    let frame = &trace["stackFrames"][0];
    assert_eq!(frame["line"], 4, "{trace:#?}");
    assert!(
        frame["source"]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("main.nvs")),
        "{trace:#?}"
    );
    let scopes = client.ask("scopes", json!({ "frameId": frame["id"] }));
    let reference = scopes["scopes"][0]["variablesReference"].clone();
    let variables = client.ask("variables", json!({ "variablesReference": reference }));
    let value = |name: &str| {
        variables["variables"]
            .as_array()
            .and_then(|all| all.iter().find(|variable| variable["name"] == name))
            .map(|variable| variable["value"].clone())
    };
    assert_eq!(value("$price"), Some(json!("4")), "{variables:#?}");
    assert_eq!(value("$total"), Some(json!("12")), "{variables:#?}");

    client.ask("continue", json!({ "threadId": thread }));
    let ran = client.until_event("terminated");
    assert_eq!(output(&ran, "stdout"), "12\n", "{ran:#?}");
    client.request("disconnect", json!({}));
    let (_, success) = client.finish();
    assert!(success, "the adapter ended with a failure");
}

/// A program that does not compile reaches the client as its diagnostic on stderr and an exit
/// status of 1.
#[test]
fn a_program_that_does_not_compile_reports_on_stderr_and_exits_1() {
    let dir = nvs_repo::scratch_private("dap-compile-error");
    fs::write(dir.join("broken.nvs"), "<?nvs\n$x = ;\n").unwrap();

    let (client, ran) = launch_and_run(&dir, "broken.nvs");
    assert!(!output(&ran, "stderr").is_empty(), "{ran:#?}");
    assert_eq!(output(&ran, "stdout"), "", "{ran:#?}");
    let exited = ran
        .iter()
        .find(|frame| frame["event"] == "exited")
        .expect("an `exited` event before `terminated`");
    assert_eq!(exited["body"]["exitCode"], 1);
    client.finish();
}

/// The first thread's innermost frame while the program is stopped, and its variables by name.
fn top_frame(client: &mut Client) -> (Value, Value) {
    let threads = client.ask("threads", json!({}));
    let thread = threads["threads"][0]["id"].clone();
    let trace = client.ask("stackTrace", json!({ "threadId": thread }));
    let frame = trace["stackFrames"][0].clone();
    let scopes = client.ask("scopes", json!({ "frameId": frame["id"] }));
    let reference = scopes["scopes"][0]["variablesReference"].clone();
    let variables = client.ask("variables", json!({ "variablesReference": reference }));
    let by_name = variables["variables"]
        .as_array()
        .map(|all| {
            all.iter()
                .map(|variable| {
                    (
                        variable["name"].as_str().unwrap_or_default().to_owned(),
                        variable["value"].clone(),
                    )
                })
                .collect::<serde_json::Map<_, _>>()
        })
        .unwrap_or_default();
    (frame, Value::Object(by_name))
}

/// `stopOnEntry` stops the program before its first statement runs, and the launch's `env`
/// reaches the program once it runs on.
#[test]
fn stop_on_entry_stops_before_the_first_statement_and_env_reaches_the_program() {
    let dir = nvs_repo::scratch_private("dap-entry");
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\necho Core\\Env::get('DAP_GREETING') ?? 'unset', \"\\n\";\n",
    )
    .unwrap();

    let mut client = Client::start(&dir);
    client.request("initialize", json!({ "adapterID": "novis" }));
    client.until_event("initialized");
    client.request(
        "launch",
        json!({
            "mode": "run",
            "program": dir.join("main.nvs"),
            "stopOnEntry": true,
            "env": { "DAP_GREETING": "hello" },
        }),
    );
    client.request("configurationDone", json!({}));
    let ran = client.until_event("stopped");
    assert_eq!(ran.last().unwrap()["body"]["reason"], "entry", "{ran:#?}");
    assert_eq!(output(&ran, "stdout"), "", "the program ran past its entry");
    let (frame, _) = top_frame(&mut client);
    assert_eq!(frame["line"], 2, "{frame:#?}");

    client.ask("continue", json!({}));
    let ran = client.until_event("terminated");
    assert_eq!(output(&ran, "stdout"), "hello\n", "{ran:#?}");
    client.request("disconnect", json!({}));
    let (_, success) = client.finish();
    assert!(success, "the adapter ended with a failure");
}

/// `pause` stops a program that never ends, inside its loop, and `disconnect` ends the program
/// with the adapter.
#[test]
fn pause_stops_a_program_in_a_loop() {
    let dir = nvs_repo::scratch_private("dap-pause");
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nvar $i = 0;\nwhile (true) {\n    $i = $i + 1;\n}\n",
    )
    .unwrap();

    let mut client = Client::start(&dir);
    client.request("initialize", json!({ "adapterID": "novis" }));
    client.until_event("initialized");
    client.request(
        "launch",
        json!({ "mode": "run", "program": dir.join("main.nvs") }),
    );
    client.request("configurationDone", json!({}));
    // A pause that arrives before the program runs stops it at its first statement.
    client.ask("pause", json!({ "threadId": 1 }));
    let ran = client.until_event("stopped");
    assert_eq!(ran.last().unwrap()["body"]["reason"], "pause", "{ran:#?}");
    let (frame, _) = top_frame(&mut client);
    assert!(
        frame["source"]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("main.nvs")),
        "{frame:#?}"
    );
    client.request("disconnect", json!({}));
    let (rest, success) = client.finish();
    assert!(
        rest.iter()
            .any(|frame| frame["command"] == "disconnect" && frame["success"] == true),
        "{rest:#?}"
    );
    assert!(success, "the adapter ended with a failure");
}

/// A `test` launch runs the program's `#[Test]` methods. A breakpoint in a test stops it with the
/// test's variables, the test's output arrives as `output` events, and stdout carries frames only.
#[test]
fn a_breakpoint_stops_a_launched_test_with_its_variables() {
    let dir = nvs_repo::scratch_private("dap-test");
    fs::write(
        dir.join("tests.nvs"),
        "<?nvs\nuse Core\\Test;\n\nfinal class PriceTest\n{\n    #[Test]\n    public function itTriples(): void\n    {\n        int $price = 4;\n        int $total = $price * 3;\n        echo $total, \"\\n\";\n        Test::assertSame($total, 12);\n    }\n}\n",
    )
    .unwrap();
    let tests = dir.join("tests.nvs");

    let mut client = Client::start(&dir);
    client.request("initialize", json!({ "adapterID": "novis" }));
    client.until_event("initialized");
    client.ask(
        "setBreakpoints",
        json!({ "source": { "path": tests }, "breakpoints": [{ "line": 11 }] }),
    );
    client.request(
        "launch",
        json!({ "mode": "test", "program": tests, "filter": "itTriples" }),
    );
    client.request("configurationDone", json!({}));
    let ran = client.until_event("stopped");
    assert_eq!(
        ran.last().unwrap()["body"]["reason"],
        "breakpoint",
        "{ran:#?}"
    );
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 11, "{frame:#?}");
    assert_eq!(variables["$price"], "4", "{variables:#?}");
    assert_eq!(variables["$total"], "12", "{variables:#?}");

    client.ask("continue", json!({}));
    let ran = client.until_event("terminated");
    assert!(output(&ran, "stdout").contains("12\n"), "{ran:#?}");
    let exited = ran
        .iter()
        .find(|frame| frame["event"] == "exited")
        .expect("an `exited` event before `terminated`");
    assert_eq!(exited["body"]["exitCode"], 0, "{ran:#?}");
    client.request("disconnect", json!({}));
    let (_, success) = client.finish();
    assert!(success, "the adapter ended with a failure");
}

/// How long a request to the launched server may take to answer before the test fails. A bound
/// on a failure, never a wait: every answer below arrives as soon as the server writes it.
const BOUND: Duration = Duration::from_secs(60);

/// Sends `GET path` to `addr` and gives the response's body.
fn get(addr: &str, path: &str) -> String {
    let mut stream =
        TcpStream::connect(addr).unwrap_or_else(|error| panic!("{addr} does not answer: {error}"));
    stream
        .set_read_timeout(Some(BOUND))
        .expect("a read timeout is set");
    write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost\r\n\r\n").expect("the request is sent");
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .unwrap_or_else(|error| panic!("the answer to `{path}` was cut off: {error}"));
    let text = String::from_utf8_lossy(&raw).into_owned();
    text.split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_else(|| panic!("the answer to `{path}` has no header block: {text}"))
}

/// A `serve` launch serves the program in `development`. A breakpoint stops the one request
/// that reaches it, a second request is answered while the first is stopped, and `continue` lets
/// the first one finish. The server's own lines arrive as `output` events, and stdout carries
/// frames only.
#[test]
fn a_breakpoint_stops_one_served_request_while_another_is_answered() {
    let dir = nvs_repo::scratch_private("dap-serve");
    fs::write(
        dir.join("app.nvs"),
        "<?nvs\nuse Core\\Env;\nuse Core\\Request;\n\nif (Request::path() == \"/stop\") {\n    int $price = 4;\n    int $total = $price * 3;\n    echo $total;\n} else {\n    echo Env::mode() == Env::Mode::Development ? \"development\" : \"production\";\n}\n",
    )
    .unwrap();
    let app = dir.join("app.nvs");

    let mut client = Client::start(&dir);
    client.request("initialize", json!({ "adapterID": "novis" }));
    client.until_event("initialized");
    client.ask(
        "setBreakpoints",
        json!({ "source": { "path": app }, "breakpoints": [{ "line": 8 }] }),
    );
    client.request(
        "launch",
        json!({ "mode": "serve", "program": app, "args": ["--port", "0"] }),
    );
    client.request("configurationDone", json!({}));
    let mut said = String::new();
    let mut seen = Vec::new();
    let addr = loop {
        let frame = client
            .frames
            .recv()
            .unwrap_or_else(|_| panic!("stdout ended before the server listened: {seen:#?}"));
        assert!(
            frame["event"] != "terminated",
            "the server ended before it listened: {seen:#?}"
        );
        said.push_str(&output(std::slice::from_ref(&frame), "stdout"));
        seen.push(frame);
        if let Some(rest) = said
            .split_once("listening on http://")
            .map(|(_, rest)| rest)
            && let Some((addr, _)) = rest.split_once(' ')
        {
            break addr.to_owned();
        }
    };

    let stopped_request = {
        let addr = addr.clone();
        std::thread::spawn(move || get(&addr, "/stop"))
    };
    let ran = client.until_event("stopped");
    assert_eq!(
        ran.last().unwrap()["body"]["reason"],
        "breakpoint",
        "{ran:#?}"
    );
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 8, "{frame:#?}");
    assert_eq!(variables["$price"], "4", "{variables:#?}");
    assert_eq!(variables["$total"], "12", "{variables:#?}");

    // The first request is still stopped, and this one is answered.
    assert_eq!(get(&addr, "/other"), "development");

    client.ask("continue", json!({}));
    assert_eq!(
        stopped_request.join().expect("the stopped request ends"),
        "12"
    );
    client.request("disconnect", json!({}));
    let (_, success) = client.finish();
    assert!(success, "the adapter ended with a failure");
}

/// Sends `command` for the first thread, and reads up to the `stopped` event it ends in. Fails
/// unless the stop's reason is `step`.
fn step(client: &mut Client, command: &str) {
    let threads = client.ask("threads", json!({}));
    let thread = threads["threads"][0]["id"].clone();
    client.ask(command, json!({ "threadId": thread }));
    let ran = client.until_event("stopped");
    let stopped = ran.last().expect("the `stopped` event");
    assert_eq!(stopped["body"]["reason"], "step", "{ran:#?}");
}

/// `stepIn` at a call stops at the method's first statement, `next` at its second, and `stepOut`
/// at the caller's next statement, where the frame shows the value the method returned.
/// A `next` from the last statement runs the program to its end.
#[test]
fn a_step_in_over_and_out_moves_one_statement_and_shows_the_returned_value() {
    let dir = nvs_repo::scratch_private("dap-step");
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nclass Shop {\n    public static function total(int $price, int $count): int {\n        int $sum = $price * $count;\n        int $withTax = $sum + 2;\n        return $withTax;\n    }\n}\nint $order = Shop::total(4, 3);\necho $order, \"\\n\";\n",
    )
    .unwrap();
    let main = dir.join("main.nvs");

    let mut client = Client::start(&dir);
    client.request("initialize", json!({ "adapterID": "novis" }));
    client.until_event("initialized");
    client.ask(
        "setBreakpoints",
        json!({ "source": { "path": main }, "breakpoints": [{ "line": 9 }] }),
    );
    client.request("launch", json!({ "mode": "run", "program": main }));
    client.request("configurationDone", json!({}));
    client.until_event("stopped");

    step(&mut client, "stepIn");
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 4, "{frame:#?}");
    assert_eq!(variables["$price"], "4", "{variables:#?}");

    step(&mut client, "next");
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 5, "{frame:#?}");
    assert_eq!(variables["$sum"], "12", "{variables:#?}");

    step(&mut client, "stepOut");
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 10, "{frame:#?}");
    assert_eq!(variables["Shop::total() returned"], "14", "{variables:#?}");

    let threads = client.ask("threads", json!({}));
    let thread = threads["threads"][0]["id"].clone();
    client.ask("next", json!({ "threadId": thread }));
    let ran = client.until_event("terminated");
    assert_eq!(output(&ran, "stdout"), "14\n", "{ran:#?}");
    client.request("disconnect", json!({}));
    let (_, success) = client.finish();
    assert!(success, "the adapter ended with a failure");
}

/// Starts `nvs dap` in `dir` with one breakpoint at `line` of `file`, launches `file` in `run`
/// mode with `launch`'s fields added, and reads up to the breakpoint's stop.
fn stopped_at(dir: &Path, file: &str, line: u32, launch: Value) -> Client {
    let program = dir.join(file);
    let mut client = Client::start(dir);
    client.request("initialize", json!({ "adapterID": "novis" }));
    client.until_event("initialized");
    client.ask(
        "setBreakpoints",
        json!({ "source": { "path": program }, "breakpoints": [{ "line": line }] }),
    );
    let mut arguments = json!({ "mode": "run", "program": program });
    if let (Some(all), Some(extra)) = (arguments.as_object_mut(), launch.as_object()) {
        all.extend(extra.clone());
    }
    client.request("launch", arguments);
    client.request("configurationDone", json!({}));
    let ran = client.until_event("stopped");
    assert_eq!(
        ran.last().unwrap()["body"]["reason"],
        "breakpoint",
        "{ran:#?}"
    );
    client
}

/// Sends `continue`, reads up to `terminated` and ends the adapter. Gives the program's stdout.
fn run_to_the_end(mut client: Client) -> String {
    client.ask("continue", json!({}));
    let ran = client.until_event("terminated");
    client.request("disconnect", json!({}));
    let (_, success) = client.finish();
    assert!(success, "the adapter ended with a failure");
    output(&ran, "stdout")
}

/// A `next` over a statement that awaits a task stops at the next statement of the same frame,
/// once the task has ended. The task runs while its parent waits, and its sites do not stop it.
#[test]
fn a_next_over_an_await_stops_at_the_same_frame_s_next_statement() {
    let dir = nvs_repo::scratch_private("dap-step-await");
    fs::write(
        dir.join("nvs.toml"),
        "[capabilities.script]\nspawn = true\n",
    )
    .unwrap();
    fs::write(
        dir.join("child.nvs"),
        "<?nvs\nint $count = 2;\necho \"child ran \", $count, \"\\n\";\n",
    )
    .unwrap();
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nvar $job = spawn script \"child.nvs\" with(output: \"capture\");\nvar $result = await $job;\necho $result->output;\n",
    )
    .unwrap();

    let mut client = stopped_at(&dir, "main.nvs", 3, json!({}));
    step(&mut client, "next");
    let (frame, _) = top_frame(&mut client);
    assert_eq!(frame["line"], 4, "{frame:#?}");
    assert!(
        frame["source"]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("main.nvs")),
        "the step stopped in the task: {frame:#?}"
    );
    assert_eq!(run_to_the_end(client), "child ran 2\n");
}

/// A `stepIn` at a statement whose only call is a `Core` method stops at the next statement of
/// the same frame. A `stepIn` at a call through a callable variable stops in the closure's body,
/// and so does one at a `Core` method that calls the closure it was given.
#[test]
fn a_step_in_reaches_a_closure_through_a_callable_and_steps_over_a_core_method() {
    let dir = nvs_repo::scratch_private("dap-step-callable");
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nvar $double = fn(int $n): int => {\n    int $twice = $n * 2;\n    return $twice;\n};\nstring $name = Core\\Str::upper('anna');\nint $four = $double(2);\narray<int> $all = Core\\Arr::map([5, 6], $double);\necho $name, \" \", $four, \" \", Core\\Arr::count($all), \"\\n\";\n",
    )
    .unwrap();

    let mut client = stopped_at(&dir, "main.nvs", 6, json!({}));
    step(&mut client, "stepIn");
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 7, "{frame:#?}");
    assert_eq!(variables["$name"], "\"ANNA\"", "{variables:#?}");

    step(&mut client, "stepIn");
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 3, "{frame:#?}");
    assert_eq!(variables["$n"], "2", "{variables:#?}");

    step(&mut client, "stepOut");
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 8, "{frame:#?}");
    assert_eq!(variables["$four"], "4", "{variables:#?}");

    step(&mut client, "stepIn");
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 3, "{frame:#?}");
    assert_eq!(variables["$n"], "5", "{variables:#?}");

    assert_eq!(run_to_the_end(client), "ANNA 4 2\n");
}

/// With `skipFiles` naming a second file, a `stepIn` at a call into it stops at the caller's next
/// statement.
#[test]
fn a_step_in_at_a_call_into_a_skipped_file_stops_at_the_caller_s_next_statement() {
    let dir = nvs_repo::scratch_private("dap-step-skip");
    fs::write(
        dir.join("lib.nvs"),
        "<?nvs\nclass Lib {\n    public static function twice(int $n): int {\n        int $twice = $n * 2;\n        return $twice;\n    }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nrequire 'lib.nvs';\nint $four = Lib::twice(2);\necho $four, \"\\n\";\n",
    )
    .unwrap();

    let mut client = stopped_at(&dir, "main.nvs", 3, json!({ "skipFiles": ["**/lib.nvs"] }));
    step(&mut client, "stepIn");
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 4, "{frame:#?}");
    assert!(
        frame["source"]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("main.nvs")),
        "the step stopped in the skipped file: {frame:#?}"
    );
    assert_eq!(variables["$four"], "4", "{variables:#?}");
    assert_eq!(run_to_the_end(client), "4\n");
}

/// A `stepOut` from a method declared `: secret string` shows the value it returned redacted, and
/// the secret's text is in no frame the adapter wrote.
#[test]
fn a_step_out_of_a_secret_method_shows_its_returned_value_redacted() {
    const KEY: &str = "dap-key-7f3a9c";
    let dir = nvs_repo::scratch_private("dap-step-secret");
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nclass Vault {\n    public static function key(): secret string {\n        secret string $key = Core\\Env::secret('DAP_KEY') ?? '';\n        return $key;\n    }\n}\nsecret string $got = Vault::key();\necho \"done\\n\";\n",
    )
    .unwrap();

    let mut client = stopped_at(&dir, "main.nvs", 8, json!({ "env": { "DAP_KEY": KEY } }));
    step(&mut client, "stepIn");
    let (frame, _) = top_frame(&mut client);
    assert_eq!(frame["line"], 4, "{frame:#?}");

    step(&mut client, "stepOut");
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 9, "{frame:#?}");
    let returned = variables["Vault::key() returned"]
        .as_str()
        .unwrap_or_else(|| panic!("no returned value: {variables:#?}"))
        .to_owned();
    assert!(!returned.contains(KEY), "{variables:#?}");

    let transcript = Arc::clone(&client.transcript);
    assert_eq!(run_to_the_end(client), "done\n");
    let written = transcript.lock().expect("the transcript is never poisoned");
    assert!(
        written.iter().all(|frame| !frame.contains(KEY)),
        "a frame carries the secret: {written:#?}"
    );
}

/// A condition stops a loop only where it holds, a logpoint writes each time and never stops,
/// and a condition that calls a method leaves its breakpoint unverified with the message.
#[test]
fn a_condition_stops_where_it_holds_and_a_logpoint_writes_without_stopping() {
    let dir = nvs_repo::scratch_private("dap-condition");
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nint $i = 0;\nwhile ($i < 4) {\n    $i = $i + 1;\n    echo $i, \"\\n\";\n}\n",
    )
    .unwrap();
    let main = dir.join("main.nvs");

    let mut client = Client::start(&dir);
    let initialized = client.ask("initialize", json!({ "adapterID": "novis" }));
    assert_eq!(initialized["supportsConditionalBreakpoints"], true);
    assert_eq!(initialized["supportsHitConditionalBreakpoints"], true);
    assert_eq!(initialized["supportsLogPoints"], true);
    let set = client.ask(
        "setBreakpoints",
        json!({ "source": { "path": main }, "breakpoints": [
            { "line": 2, "condition": "$i->size() > 1" },
            { "line": 4, "logMessage": "i is {$i}" },
            { "line": 5, "condition": "$i == 3" },
        ] }),
    );
    let call = set["breakpoints"][0]["id"].clone();
    client.request("launch", json!({ "mode": "run", "program": main }));
    client.request("configurationDone", json!({}));

    let ran = client.until_event("stopped");
    assert!(
        ran.iter().any(|frame| frame["event"] == "breakpoint"
            && frame["body"]["breakpoint"]["id"] == call
            && frame["body"]["breakpoint"]["verified"] == false
            && frame["body"]["breakpoint"]["message"]
                == "A condition cannot call a function or a method."),
        "the call left its breakpoint verified: {ran:#?}"
    );
    assert_eq!(output(&ran, "stdout"), "1\n2\n", "{ran:#?}");
    assert_eq!(
        output(&ran, "console"),
        "i is 0\ni is 1\ni is 2\n",
        "{ran:#?}"
    );
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 5, "{frame:#?}");
    assert_eq!(variables["$i"], "3", "{variables:#?}");

    client.ask("continue", json!({}));
    let ran = client.until_event("terminated");
    assert_eq!(output(&ran, "stdout"), "3\n4\n", "{ran:#?}");
    assert_eq!(output(&ran, "console"), "i is 3\n", "{ran:#?}");
    client.request("disconnect", json!({}));
    let (_, success) = client.finish();
    assert!(success, "the adapter ended with a failure");
}

/// A method that throws `RuntimeError` when its argument is over 1, called once inside a `try`
/// that catches it and once inside a `try` that catches only `LogicError`, whose `catch` re-raises
/// the same object.
const THROWS: &str = "<?nvs\nclass Shop {\n    public static function check(int $n): int {\n        if ($n > 1) {\n            throw new RuntimeError('too many');\n        }\n        return $n;\n    }\n}\ntry {\n    Shop::check(2);\n} catch (RuntimeError $e) {\n    echo \"caught\\n\";\n}\ntry {\n    Shop::check(3);\n} catch (LogicError $wrong) {\n    echo \"wrong\\n\";\n}\necho \"end\\n\";\n";

/// Starts `nvs dap` in `dir`, turns on the exception `filters` and launches `main.nvs`. Gives the
/// client and every message up to the first `stopped` or `terminated` event.
fn launch_with_filters(dir: &Path, filters: Value, launch: Value) -> (Client, Vec<Value>) {
    let program = dir.join("main.nvs");
    let mut client = Client::start(dir);
    client.request("initialize", json!({ "adapterID": "novis" }));
    client.until_event("initialized");
    client.ask("setExceptionBreakpoints", json!({ "filters": filters }));
    let mut arguments = json!({ "mode": "run", "program": program });
    if let (Some(all), Some(extra)) = (arguments.as_object_mut(), launch.as_object()) {
        all.extend(extra.clone());
    }
    client.request("launch", arguments);
    client.request("configurationDone", json!({}));
    let ran = until_stop_or_end(&mut client);
    (client, ran)
}

/// Every message up to and including the first `stopped` or `terminated` event.
fn until_stop_or_end(client: &mut Client) -> Vec<Value> {
    let mut seen = Vec::new();
    loop {
        let frame = client.pending.pop_front().unwrap_or_else(|| {
            client
                .frames
                .recv()
                .unwrap_or_else(|_| panic!("stdout ended before a stop or the end: {seen:#?}"))
        });
        let done = frame["type"] == "event"
            && (frame["event"] == "stopped" || frame["event"] == "terminated");
        seen.push(frame);
        if done {
            return seen;
        }
    }
}

/// Fails unless `ran` ends in a stop for an exception described as `description`, at `line`,
/// where `$n` is `n`. `exceptionInfo` names `RuntimeError` and its message.
fn assert_exception_stop(
    client: &mut Client,
    ran: &[Value],
    description: &str,
    line: u32,
    n: &str,
) {
    let stopped = ran.last().expect("a message");
    assert_eq!(stopped["event"], "stopped", "{ran:#?}");
    assert_eq!(stopped["body"]["reason"], "exception", "{ran:#?}");
    assert_eq!(stopped["body"]["description"], description, "{ran:#?}");
    assert_eq!(stopped["body"]["text"], "too many", "{ran:#?}");
    let info = client.ask("exceptionInfo", json!({ "threadId": 1 }));
    assert_eq!(info["exceptionId"], "RuntimeError", "{info:#?}");
    assert_eq!(info["description"], "too many", "{info:#?}");
    let (frame, variables) = top_frame(client);
    assert_eq!(frame["line"], line, "{frame:#?}");
    assert_eq!(variables["$n"], n, "{variables:#?}");
}

/// Under `uncaught`, the throw a `catch` handles does not stop. The throw no `catch` matches stops
/// once at the `throw`, and the `catch (LogicError …)` it passes does not stop it a second time.
#[test]
fn the_uncaught_filter_stops_once_at_a_throw_no_frame_catches() {
    let dir = nvs_repo::scratch_private("dap-uncaught");
    fs::write(dir.join("main.nvs"), THROWS).unwrap();

    let (mut client, ran) = launch_with_filters(&dir, json!(["uncaught"]), json!({}));
    assert_eq!(output(&ran, "stdout"), "caught\n", "{ran:#?}");
    assert_exception_stop(&mut client, &ran, "Uncaught RuntimeError", 5, "3");

    client.ask("continue", json!({}));
    let ran = client.until_event("terminated");
    assert!(
        ran.iter().all(|frame| frame["event"] != "stopped"),
        "the exception stopped a second time: {ran:#?}"
    );
    client.request("disconnect", json!({}));
    client.finish();
}

/// Under `thrown`, each of the two throws stops once at the `throw`, the caught one included.
#[test]
fn the_thrown_filter_stops_once_at_every_throw() {
    let dir = nvs_repo::scratch_private("dap-thrown");
    fs::write(dir.join("main.nvs"), THROWS).unwrap();

    let (mut client, ran) = launch_with_filters(&dir, json!(["thrown"]), json!({}));
    assert_exception_stop(&mut client, &ran, "Thrown RuntimeError", 5, "2");

    client.ask("continue", json!({}));
    let ran = until_stop_or_end(&mut client);
    assert_eq!(output(&ran, "stdout"), "caught\n", "{ran:#?}");
    assert_exception_stop(&mut client, &ran, "Uncaught RuntimeError", 5, "3");

    client.ask("continue", json!({}));
    let ran = client.until_event("terminated");
    assert!(
        ran.iter().all(|frame| frame["event"] != "stopped"),
        "an exception stopped a third time: {ran:#?}"
    );
    client.request("disconnect", json!({}));
    client.finish();
}

/// Starts `nvs dap` in `dir` with `breakpoints` set on `main.nvs`, launches it with `launch`'s
/// fields added, and gives the client with the `setBreakpoints` body.
fn launch_with_breakpoints(dir: &Path, breakpoints: Value, launch: Value) -> (Client, Value) {
    let main = dir.join("main.nvs");
    let mut client = Client::start(dir);
    client.request("initialize", json!({ "adapterID": "novis" }));
    client.until_event("initialized");
    let set = client.ask(
        "setBreakpoints",
        json!({ "source": { "path": main }, "breakpoints": breakpoints }),
    );
    let mut arguments = json!({ "mode": "run", "program": main });
    if let (Some(all), Some(extra)) = (arguments.as_object_mut(), launch.as_object()) {
        all.extend(extra.clone());
    }
    client.request("launch", arguments);
    client.request("configurationDone", json!({}));
    (client, set)
}

/// Continues from every stop up to the end. Gives each stop's line and `$i`, every message read,
/// and ends the adapter.
fn every_stop(mut client: Client) -> (Vec<(u64, String)>, Vec<Value>) {
    let mut stops = Vec::new();
    let mut all = Vec::new();
    loop {
        let ran = until_stop_or_end(&mut client);
        let end = ran
            .last()
            .is_some_and(|frame| frame["event"] == "terminated");
        all.extend(ran);
        if end {
            break;
        }
        let (frame, variables) = top_frame(&mut client);
        stops.push((
            frame["line"].as_u64().unwrap_or_default(),
            variables["$i"].as_str().unwrap_or_default().to_owned(),
        ));
        client.ask("continue", json!({}));
    }
    client.request("disconnect", json!({}));
    let (_, success) = client.finish();
    assert!(success, "the adapter ended with a failure");
    (stops, all)
}

/// Fails unless `ran` has a `breakpoint` event leaving breakpoint `id` unverified with `message`.
fn assert_unverified(ran: &[Value], id: &Value, message: &str) {
    assert!(
        ran.iter().any(|frame| frame["event"] == "breakpoint"
            && frame["body"]["breakpoint"]["id"] == *id
            && frame["body"]["breakpoint"]["verified"] == false
            && frame["body"]["breakpoint"]["message"] == message),
        "no unverified breakpoint {id} with {message:?}: {ran:#?}"
    );
}

/// A condition may read an array element, a property, arithmetic, `&&` and `!`, and stops only
/// where it holds. A condition that reads a `secret`, assigns or is not a `bool` leaves its
/// breakpoint unverified. A logpoint's hole shows a `secret` variable redacted.
#[test]
fn each_condition_form_stops_where_it_holds_and_a_logpoint_redacts_a_secret() {
    const KEY: &str = "dap-key-41c0d2";
    let dir = nvs_repo::scratch_private("dap-condition-forms");
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nclass Item {\n    public int $price = 0;\n}\nsecret string $key = Core\\Env::secret('DAP_KEY') ?? '';\nvar $item = new Item();\narray<int> $prices = [10, 20, 30];\nint $i = 0;\nwhile ($i < 3) {\n    $item->price = $prices[$i];\n    $i = $i + 1;\n    echo $item->price, \"\\n\";\n}\n",
    )
    .unwrap();

    let (client, set) = launch_with_breakpoints(
        &dir,
        json!([
            { "line": 6, "condition": "$key == 'x'" },
            { "line": 7, "condition": "$i = 2" },
            { "line": 8, "condition": "$i + 1" },
            { "line": 10, "condition": "$prices[$i] == 20" },
            { "line": 11, "condition": "$item->price * 2 > 50 && !($i == 0)" },
            { "line": 12, "logMessage": "price {$item->price} key {$key}" },
        ]),
        json!({ "env": { "DAP_KEY": KEY } }),
    );
    let ids: Vec<Value> = (0..3)
        .map(|n| set["breakpoints"][n]["id"].clone())
        .collect();
    let transcript = Arc::clone(&client.transcript);
    let (stops, ran) = every_stop(client);
    assert_unverified(
        &ran,
        &ids[0],
        "`$key` is secret, so a condition cannot read it.",
    );
    assert_unverified(&ran, &ids[1], "A condition cannot change a variable.");
    assert_unverified(
        &ran,
        &ids[2],
        "A condition must be a `bool`, and this one is `int`.",
    );
    assert_eq!(
        stops,
        [(10, "1".to_owned()), (11, "2".to_owned())],
        "{ran:#?}"
    );
    assert_eq!(output(&ran, "stdout"), "10\n20\n30\n", "{ran:#?}");
    assert_eq!(
        output(&ran, "console"),
        "price 10 key [redacted]\nprice 20 key [redacted]\nprice 30 key [redacted]\n",
        "{ran:#?}"
    );
    let written = transcript.lock().expect("the transcript is never poisoned");
    assert!(
        written.iter().all(|frame| !frame.contains(KEY)),
        "a frame carries the secret: {written:#?}"
    );
}

/// Each hit form counts its own breakpoint's hits: `3` from the third on, `>4` after the fourth,
/// `==2` at the second, `%3` at every third and `>=6` from the sixth. A hit condition that does
/// not parse, or `%0`, leaves its breakpoint unverified.
#[test]
fn each_hit_condition_form_stops_at_the_hits_it_names() {
    let dir = nvs_repo::scratch_private("dap-hit-forms");
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nint $i = 0;\nwhile ($i < 6) {\n    $i = $i + 1;\n    echo 'a';\n    echo 'b';\n    echo 'c';\n    echo 'd';\n}\necho \"\\n\";\n",
    )
    .unwrap();

    let (client, set) = launch_with_breakpoints(
        &dir,
        json!([
            { "line": 2, "hitCondition": "%0" },
            { "line": 4, "hitCondition": "3" },
            { "line": 5, "hitCondition": ">4" },
            { "line": 6, "hitCondition": "==2" },
            { "line": 7, "hitCondition": "%3" },
            { "line": 8, "hitCondition": ">=6" },
            { "line": 10, "hitCondition": "abc" },
        ]),
        json!({}),
    );
    let never = set["breakpoints"][0]["id"].clone();
    let wrong = set["breakpoints"][6]["id"].clone();
    let (stops, ran) = every_stop(client);
    assert_unverified(&ran, &never, "The hit condition `%0` never stops.");
    assert_unverified(
        &ran,
        &wrong,
        "The hit condition `abc` is not `N`, `>N`, `>=N`, `==N` or `%N`.",
    );
    let at = |line: u64, i: &str| (line, i.to_owned());
    assert_eq!(
        stops,
        [
            at(6, "2"),
            at(4, "2"),
            at(7, "3"),
            at(4, "3"),
            at(4, "4"),
            at(5, "5"),
            at(4, "5"),
            at(5, "6"),
            at(7, "6"),
            at(8, "6"),
        ],
        "{ran:#?}"
    );
}

/// Under `thrown`, a throw that a frame in a `skipFiles` file catches does not stop. Without
/// `skipFiles`, the same throw stops.
#[test]
fn a_throw_caught_in_a_skipped_file_does_not_stop_under_thrown() {
    let dir = nvs_repo::scratch_private("dap-thrown-skip");
    fs::write(
        dir.join("lib.nvs"),
        "<?nvs\nclass Lib {\n    public static function safely(callable(int): int $check, int $n): int {\n        try {\n            return $check($n);\n        } catch (RuntimeError $e) {\n            return 0;\n        }\n    }\n}\n",
    )
    .unwrap();
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nrequire 'lib.nvs';\nvar $check = fn(int $n): int => {\n    if ($n > 1) {\n        throw new RuntimeError('too many');\n    }\n    return $n;\n};\necho Lib::safely($check, 2), \"\\n\";\n",
    )
    .unwrap();

    let (client, ran) = launch_with_filters(
        &dir,
        json!(["thrown"]),
        json!({ "skipFiles": ["**/lib.nvs"] }),
    );
    let last = ran.last().expect("a message");
    assert_eq!(last["event"], "terminated", "the throw stopped: {ran:#?}");
    assert_eq!(output(&ran, "stdout"), "0\n", "{ran:#?}");
    client.finish();

    let (mut client, ran) = launch_with_filters(&dir, json!(["thrown"]), json!({}));
    assert_exception_stop(&mut client, &ran, "Thrown RuntimeError", 5, "2");
    client.ask("continue", json!({}));
    let ran = client.until_event("terminated");
    assert_eq!(output(&ran, "stdout"), "0\n", "{ran:#?}");
    client.request("disconnect", json!({}));
    client.finish();
}

/// Under `uncaught`, a `Core` method's error that nothing catches stops at the statement that
/// called it, with the frame's variables.
#[test]
fn a_core_method_s_uncaught_error_stops_at_the_calling_statement() {
    let dir = nvs_repo::scratch_private("dap-uncaught-core");
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nstring $text = '{';\nvar $parsed = Core\\Json::decode($text);\necho \"end\\n\";\n",
    )
    .unwrap();

    let (mut client, ran) = launch_with_filters(&dir, json!(["uncaught"]), json!({}));
    let stopped = ran.last().expect("a message");
    assert_eq!(stopped["event"], "stopped", "{ran:#?}");
    assert_eq!(stopped["body"]["reason"], "exception", "{ran:#?}");
    assert!(
        stopped["body"]["description"]
            .as_str()
            .is_some_and(|text| text.starts_with("Uncaught ")),
        "{ran:#?}"
    );
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(frame["line"], 3, "{frame:#?}");
    assert_eq!(variables["$text"], "\"{\"", "{variables:#?}");
    client.ask("continue", json!({}));
    let ran = client.until_event("terminated");
    assert!(
        ran.iter().all(|frame| frame["event"] != "stopped"),
        "the error stopped a second time: {ran:#?}"
    );
    client.request("disconnect", json!({}));
    client.finish();
}

/// Under `uncaught`, a throw a `catch` expression handles does not stop, and one whose `catch`
/// expression names another class stops at the `throw`.
#[test]
fn a_catch_expression_catches_for_the_uncaught_filter() {
    let dir = nvs_repo::scratch_private("dap-catch-expression");
    fs::write(
        dir.join("main.nvs"),
        "<?nvs\nclass Shop {\n    public static function check(int $n): int {\n        if ($n > 1) {\n            throw new RuntimeError('too many');\n        }\n        return $n;\n    }\n}\necho Shop::check(3) catch (RuntimeError $e) => 0, \"\\n\";\necho Shop::check(4) catch (LogicError $e) => 0, \"\\n\";\n",
    )
    .unwrap();

    let (mut client, ran) = launch_with_filters(&dir, json!(["uncaught"]), json!({}));
    assert_eq!(output(&ran, "stdout"), "0\n", "{ran:#?}");
    assert_exception_stop(&mut client, &ran, "Uncaught RuntimeError", 5, "4");
    client.ask("continue", json!({}));
    let ran = client.until_event("terminated");
    assert!(
        ran.iter().all(|frame| frame["event"] != "stopped"),
        "the exception stopped a second time: {ran:#?}"
    );
    client.request("disconnect", json!({}));
    client.finish();
}

/// With `pathMappings` pairing a folder that does not exist with the program's, a breakpoint the
/// editor set under its own folder before `launch` stops the program, the frame names the editor's
/// file, and a `skipFiles` glob written in the editor's terms keeps a step out of a file.
#[test]
fn path_mappings_map_every_path_in_and_out_of_the_adapter() {
    let dir = nvs_repo::scratch_private("dap-path-mappings");
    let program = dir.join("app");
    fs::create_dir_all(&program).unwrap();
    fs::write(
        program.join("lib.nvs"),
        "<?nvs\nclass Lib {\n    public static function twice(int $n): int {\n        return $n * 2;\n    }\n}\n",
    )
    .unwrap();
    fs::write(
        program.join("main.nvs"),
        "<?nvs\nrequire 'lib.nvs';\nint $four = Lib::twice(2);\necho $four, \"\\n\";\n",
    )
    .unwrap();
    let editor = dir.join("editor");
    let main = editor.join("main.nvs");

    let mut client = Client::start(&dir);
    client.request("initialize", json!({ "adapterID": "novis" }));
    client.until_event("initialized");
    let set = client.ask(
        "setBreakpoints",
        json!({ "source": { "path": main }, "breakpoints": [{ "line": 3 }] }),
    );
    let id = set["breakpoints"][0]["id"].clone();
    client.request(
        "launch",
        json!({
            "mode": "run",
            "program": main,
            "cwd": editor,
            "skipFiles": [format!("{}/lib.nvs", editor.display())],
            "pathMappings": [{ "localRoot": editor, "remoteRoot": program }],
        }),
    );
    client.request("configurationDone", json!({}));
    let ran = client.until_event("stopped");
    let stop = &ran.last().unwrap()["body"];
    assert_eq!(stop["reason"], "breakpoint", "{ran:#?}");
    assert!(
        ran.iter().any(|frame| frame["event"] == "breakpoint"
            && frame["body"]["breakpoint"]["id"] == id
            && frame["body"]["breakpoint"]["verified"] == true),
        "the breakpoint was not verified under its mapped file: {ran:#?}"
    );
    let (frame, _) = top_frame(&mut client);
    assert_eq!(frame["line"], 3, "{frame:#?}");
    assert_eq!(frame["source"]["path"], json!(main), "{frame:#?}");

    step(&mut client, "stepIn");
    let (frame, variables) = top_frame(&mut client);
    assert_eq!(
        frame["line"], 4,
        "the step stopped in the skipped file: {frame:#?}"
    );
    assert_eq!(frame["source"]["path"], json!(main), "{frame:#?}");
    assert_eq!(variables["$four"], "4", "{variables:#?}");
    assert_eq!(run_to_the_end(client), "4\n");
}
