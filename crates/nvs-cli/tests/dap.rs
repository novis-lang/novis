//! `nvs dap` driven by a scripted client: a launched program's output arrives as `output` events,
//! its end as `exited` and `terminated`, and every byte the adapter writes on stdout belongs to a
//! frame (`rule:ide/stdout-belongs-to-the-protocol`), even when the program prints one.
//!
//! Through the built binary, because the property is the process's: the program runs inside the
//! adapter, and what it writes to "standard output" must reach the client as text and never as a
//! frame.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::thread::JoinHandle;

use serde_json::{Value, json};

/// A running `nvs dap` and the frames it has written so far.
struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    frames: Receiver<Value>,
    reader: JoinHandle<Result<usize, String>>,
    seq: i64,
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
        let reader = std::thread::spawn(move || {
            let mut stdout = BufReader::new(stdout);
            let mut count = 0;
            loop {
                match strict_frame(&mut stdout)? {
                    Some(frame) => {
                        count += 1;
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
            reader,
            seq: 1,
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

    /// Every message up to and including the first event named `event`.
    fn until_event(&self, event: &str) -> Vec<Value> {
        let mut seen = Vec::new();
        loop {
            let frame = self
                .frames
                .recv()
                .unwrap_or_else(|_| panic!("stdout ended before a `{event}` event: {seen:#?}"));
            let done = frame["type"] == "event" && frame["event"] == event;
            seen.push(frame);
            if done {
                return seen;
            }
        }
    }

    /// Sends one request and gives its response's body, with every message before the response.
    /// Fails if the request failed.
    fn ask(&mut self, command: &str, arguments: Value) -> (Value, Vec<Value>) {
        self.request(command, arguments);
        let mut seen = Vec::new();
        loop {
            let frame = self.frames.recv().unwrap_or_else(|_| {
                panic!("stdout ended before the `{command}` response: {seen:#?}")
            });
            if frame["type"] == "response" && frame["command"] == command {
                assert_eq!(frame["success"], true, "`{command}` failed: {frame:#?}");
                return (frame["body"].clone(), seen);
            }
            seen.push(frame);
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
        (self.frames.try_iter().collect(), status.success())
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
    let (set, _) = client.ask(
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

    let (threads, _) = client.ask("threads", json!({}));
    let thread = threads["threads"][0]["id"].clone();
    let (trace, _) = client.ask("stackTrace", json!({ "threadId": thread }));
    let frame = &trace["stackFrames"][0];
    assert_eq!(frame["line"], 4, "{trace:#?}");
    assert!(
        frame["source"]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("main.nvs")),
        "{trace:#?}"
    );
    let (scopes, _) = client.ask("scopes", json!({ "frameId": frame["id"] }));
    let reference = scopes["scopes"][0]["variablesReference"].clone();
    let (variables, _) = client.ask("variables", json!({ "variablesReference": reference }));
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
