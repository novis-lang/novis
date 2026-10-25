//! `nvs dap` — the debug adapter, on standard input and output
//! (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`).
//!
//! A client starts `nvs dap`, sends `initialize`, then `launch` and `configurationDone` in either
//! order, and the program starts once both have arrived. It runs in this process, on a thread of
//! its own, through the same code its command uses: [`crate::run_run`] for `run`,
//! [`crate::run_test`] for `test` and [`crate::serve::run`] for `serve`. A launched program
//! compiles and runs exactly as it would from a terminal, with the launch's `env` set in this
//! process before it starts. A `test` launch runs one test at a time and shows each test's output.
//! A `serve` launch starts in `development` whatever the tree writes, listens where its `args`
//! say (`--listen` or `--port`, as `nvs serve` takes them), and serves until the session ends. A
//! breakpoint stops the one request that reaches it, and every other request is served. There is
//! no `attach`: nothing ever listens for a debugger.
//!
//! `stopOnEntry` stops the program at its first statement, and a `pause` sent before the program
//! runs does the same ([`Debugger::stop_on_entry`], [`Debugger::pause`]).
//!
//! **Steps.** `next`, `stepIn` and `stepOut` step the one thread they name and resume every other
//! one ([`Debugger::step`]). A step does not stop in a file one of the launch's `skipFiles` globs
//! matches. A glob is matched against the file's path with `/` between its parts, and on Windows
//! with no regard to case.
//!
//! stdout carries frames and nothing else (`rule:ide/stdout-belongs-to-the-protocol`).
//! [`stdio`] moves the process's own streams into pipes before anything runs, and a pump thread per
//! pipe turns what the program writes into `output` events. The program's end is an `exited` and a
//! `terminated` event, written once both pumps have drained, so no output arrives after them.
//!
//! **Breakpoints and stops.** Every launch compiles under the debug switch and runs attached to
//! the session's [`Debugger`] (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`).
//! `setBreakpoints` sets a file's lines on it, and the client usually sends it before the program
//! has compiled, so the answer says no line is verified yet. Once the unit loads, a `breakpoint`
//! event gives each line's real answer. A file is named by [`file_key`] on both sides, so the path
//! an editor sends and the path the compiler read are the same text. A stop thread turns the
//! controller's stops into `stopped` events, and [`stopped`] answers what a client reads during
//! one.
//!
//! **Ending a session.** `disconnect` ends the adapter, and the program with it, because the
//! program is this process. `terminate` answers with `terminated` and leaves the program running
//! until the client's `disconnect` follows, which every client sends.
//!
//! **What it spends:** the two pump threads and their 8 KiB buffers, one thread for the program,
//! one for the stops, one frame in flight at a time, and one id per breakpoint the client set.
//! A stop holds what [`stopped`] says until its resume. Nothing grows with the length of the
//! session.

mod protocol;
mod stdio;
mod stopped;

use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

use nvs_host::debug::{Breakpoint, Debugger};
use nvs_runtime::StepKind;
use rustc_hash::FxHashMap;
use serde_json::{Value, json};

use crate::config;
use protocol::{Capabilities, Incoming, LaunchArguments, Mode, Outgoing};
use stopped::Stopped;

/// The text a file is named by in the session: its canonical path, so the path an editor sends
/// and the path the compiler read are the same. A path that does not resolve is kept as written.
pub(crate) fn file_key(path: &str) -> String {
    nvs_diagnostics::canonical_key(Path::new(path))
        .to_string_lossy()
        .into_owned()
}

/// Locks `mutex`, reading through a poison: every value behind one here is whole between writes.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The id the client knows each breakpoint by, and its line, per file key.
#[derive(Default)]
struct Breakpoints {
    next: i64,
    files: FxHashMap<String, Vec<(i64, u32)>>,
}

/// Speak the Debug Adapter Protocol on standard input and output.
///
/// An editor starts this command, not a person. It reads DAP messages on
/// stdin and writes them on stdout, so a terminal that runs it shows
/// nothing.
///
/// The editor's `launch` request names the program, and `nvs dap` runs
/// that program in its own process. Everything the program prints reaches
/// the editor as `output` events.
// `rule:ide/the-debug-adapter-does-not-wait-for-an-editor`: everything a launch configures arrives
// in the `launch` request, so the command has no options of its own.
#[derive(clap::Args)]
pub(crate) struct Args {}

impl Args {
    /// Runs `nvs dap` until the client disconnects or closes stdin.
    pub(crate) fn run(self, config: &[PathBuf], init: config::Init) -> ExitCode {
        let streams = match stdio::Streams::take() {
            Ok(streams) => streams,
            Err(error) => {
                eprintln!("error: nvs dap could not take its standard streams: {error}");
                return ExitCode::FAILURE;
            }
        };
        let writer = Arc::new(Writer {
            output: Mutex::new((streams.output, 1)),
        });
        let (debugger, stops) = Debugger::new();
        let stopped = Arc::new(Mutex::new(Stopped::default()));
        let breakpoints = Arc::new(Mutex::new(Breakpoints::default()));
        debugger.on_load({
            let writer = Arc::clone(&writer);
            let breakpoints = Arc::clone(&breakpoints);
            move |resolved| loaded(&writer, &breakpoints, resolved)
        });
        // Ends when the last handle of the session is dropped, which is the process's end.
        std::thread::spawn({
            let writer = Arc::clone(&writer);
            let stopped = Arc::clone(&stopped);
            move || {
                for stop in stops {
                    let event = lock(&stopped).file(stop);
                    if let Some(body) = event {
                        writer.event("stopped", Some(body));
                    }
                }
            }
        });
        let mut session = Session {
            writer,
            debugger,
            stopped,
            breakpoints,
            log: streams.log,
            launch: None,
            configured: false,
            program: None,
            pipes: Some(streams.pipes),
            pumps: Vec::new(),
            config: config.to_vec(),
            init,
        };
        session.pumps = vec![
            pump(streams.program_out, "stdout", Arc::clone(&session.writer)),
            pump(streams.program_err, "stderr", Arc::clone(&session.writer)),
        ];
        session.serve(BufReader::new(streams.input))
    }
}

/// The frames the adapter writes, numbered in the order they reach the wire.
struct Writer {
    output: Mutex<(File, i64)>,
}

impl Writer {
    /// Writes one message, numbered under the lock. A write that fails is a client that has gone,
    /// and the read loop sees that on its own side.
    fn send<'a>(&self, message: impl FnOnce(i64) -> Outgoing<'a>) {
        let mut output = self
            .output
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let seq = output.1;
        output.1 += 1;
        let Ok(body) = serde_json::to_vec(&message(seq)) else {
            return;
        };
        let _ = protocol::write_frame(&mut output.0, &body);
    }

    fn event(&self, event: &'static str, body: Option<Value>) {
        self.send(|seq| Outgoing::Event { seq, event, body });
    }
}

/// Tells the client the real answer for each breakpoint it set, once a unit has loaded. A line
/// is told only where the client's breakpoint at that place is still on that line, so a
/// `setBreakpoints` that crossed the load is answered by its own response.
fn loaded(
    writer: &Writer,
    breakpoints: &Mutex<Breakpoints>,
    resolved: &[(String, Vec<Breakpoint>)],
) {
    let breakpoints = lock(breakpoints);
    for (file, lines) in resolved {
        let Some(known) = breakpoints.files.get(file) else {
            continue;
        };
        for (&(id, line), resolved) in known.iter().zip(lines) {
            if line == resolved.line {
                writer.event(
                    "breakpoint",
                    Some(json!({
                        "reason": "changed",
                        "breakpoint": { "id": id, "verified": resolved.verified, "line": line },
                    })),
                );
            }
        }
    }
}

/// One session's state: what the client has sent so far, and the program once it runs.
struct Session {
    writer: Arc<Writer>,
    debugger: Debugger,
    stopped: Arc<Mutex<Stopped>>,
    breakpoints: Arc<Mutex<Breakpoints>>,
    log: File,
    launch: Option<LaunchArguments>,
    configured: bool,
    program: Option<JoinHandle<()>>,
    /// Taken by the program's thread, which closes them once the program ends.
    pipes: Option<stdio::Pipes>,
    pumps: Vec<JoinHandle<()>>,
    config: Vec<PathBuf>,
    init: config::Init,
}

impl Session {
    /// Reads and answers requests until `disconnect` or the end of stdin.
    fn serve(mut self, mut input: BufReader<File>) -> ExitCode {
        loop {
            let frame = match protocol::read_frame(&mut input) {
                Ok(Some(frame)) => frame,
                Ok(None) => return ExitCode::SUCCESS,
                // A broken frame leaves no way to find where the next one starts.
                Err(error) => {
                    let _ = writeln!(self.log, "error: nvs dap read a broken frame: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let request: Incoming = match serde_json::from_slice(&frame) {
                Ok(request) => request,
                Err(error) => {
                    let _ = writeln!(
                        self.log,
                        "error: nvs dap read a message that is not DAP: {error}"
                    );
                    continue;
                }
            };
            if request.kind != "request" {
                continue;
            }
            if self.answer(&request) == Flow::Stop {
                return ExitCode::SUCCESS;
            }
        }
    }

    /// Answers one request.
    fn answer(&mut self, request: &Incoming) -> Flow {
        match request.command.as_str() {
            "initialize" => {
                let capabilities = Capabilities {
                    supports_configuration_done_request: true,
                    supports_terminate_request: true,
                };
                self.succeed(request, serde_json::to_value(capabilities).ok());
                self.writer.event("initialized", None);
            }
            "launch" => match self.launch(request) {
                Ok(()) => {
                    self.succeed(request, None);
                    self.start();
                }
                Err(message) => self.fail(request, &message),
            },
            "configurationDone" => {
                self.configured = true;
                self.succeed(request, None);
                self.start();
            }
            "setBreakpoints" => match self.set_breakpoints(&request.arguments) {
                Ok(body) => self.succeed(request, Some(body)),
                Err(message) => self.fail(request, message),
            },
            "threads" => {
                let body = lock(&self.stopped).threads();
                self.succeed(request, Some(body));
            }
            "stackTrace" => {
                let body = lock(&self.stopped).stack_trace(&request.arguments);
                self.reply(request, body);
            }
            "scopes" => {
                let body = lock(&self.stopped).scopes(&request.arguments);
                self.reply(request, body);
            }
            "variables" => {
                let body = lock(&self.stopped).variables(&request.arguments);
                self.reply(request, body);
            }
            "continue" => {
                lock(&self.stopped).clear();
                self.debugger.resume();
                self.succeed(request, Some(json!({ "allThreadsContinued": true })));
            }
            "pause" => {
                self.debugger.pause();
                self.succeed(request, None);
            }
            "next" | "stepIn" | "stepOut" => {
                let kind = match request.command.as_str() {
                    "next" => StepKind::Over,
                    "stepIn" => StepKind::In,
                    _ => StepKind::Out,
                };
                let mut stopped = lock(&self.stopped);
                match stopped.stop_id(&request.arguments) {
                    Ok(stop) => {
                        stopped.clear();
                        drop(stopped);
                        self.debugger.step(stop, kind);
                        self.succeed(request, None);
                    }
                    Err(message) => {
                        drop(stopped);
                        self.fail(request, &message);
                    }
                }
            }
            "terminate" => {
                self.succeed(request, None);
                if self
                    .program
                    .as_ref()
                    .is_some_and(|program| !program.is_finished())
                {
                    self.writer.event("terminated", None);
                }
            }
            "disconnect" => {
                self.succeed(request, None);
                return Flow::Stop;
            }
            other => {
                let message = format!("`nvs dap` does not support the `{other}` request.");
                self.fail(request, &message);
            }
        }
        Flow::Continue
    }

    /// Sets one file's breakpoints, and gives the `setBreakpoints` body: one breakpoint per line,
    /// each with a new id.
    fn set_breakpoints(&self, arguments: &Value) -> Result<Value, &'static str> {
        let path = arguments["source"]["path"]
            .as_str()
            .ok_or("The breakpoints name no source file.")?;
        let lines: Vec<u32> = arguments["breakpoints"]
            .as_array()
            .map(|breakpoints| {
                breakpoints
                    .iter()
                    .filter_map(|breakpoint| breakpoint["line"].as_u64())
                    .filter_map(|line| u32::try_from(line).ok())
                    .collect()
            })
            .unwrap_or_default();
        let file = file_key(path);
        // Held across the set, so a load that follows it reads this set's ids.
        let mut breakpoints = lock(&self.breakpoints);
        let answer = self.debugger.set_breakpoints(&file, &lines);
        let first = breakpoints.next + 1;
        breakpoints.next += i64::try_from(lines.len()).unwrap_or(i64::MAX);
        let known: Vec<(i64, u32)> = (first..).zip(lines.iter().copied()).collect();
        let shown: Vec<Value> = known
            .iter()
            .zip(&answer)
            .map(|(&(id, line), set)| json!({ "id": id, "verified": set.verified, "line": line }))
            .collect();
        if known.is_empty() {
            breakpoints.files.remove(&file);
        } else {
            breakpoints.files.insert(file, known);
        }
        Ok(json!({ "breakpoints": shown }))
    }

    /// Reads a `launch` request's arguments, or the message the client shows when they cannot run.
    fn launch(&mut self, request: &Incoming) -> Result<(), String> {
        if self.launch.is_some() {
            return Err("This session has already launched a program.".to_owned());
        }
        let arguments: LaunchArguments = serde_json::from_value(request.arguments.clone())
            .map_err(|error| format!("The launch configuration is not valid: {error}"))?;
        if arguments.mode == Mode::Serve {
            serve_address(&arguments.args)?;
        }
        if !arguments.skip_files.is_empty() {
            let skip = skip_files(&arguments.skip_files)?;
            self.debugger
                .skip_files(move |file| skip.is_match(shown_file(file)));
        }
        if arguments.stop_on_entry {
            self.debugger.stop_on_entry();
        }
        self.launch = Some(arguments);
        Ok(())
    }

    /// Starts the program once `launch` and `configurationDone` have both arrived.
    fn start(&mut self) {
        if !self.configured || self.program.is_some() {
            return;
        }
        let (Some(launch), Some(pipes)) = (self.launch.take(), self.pipes.take()) else {
            return;
        };
        let writer = Arc::clone(&self.writer);
        let pumps = std::mem::take(&mut self.pumps);
        let config = self.config.clone();
        let init = self.init;
        let debugger = self.debugger.clone();
        set_env(&launch.env);
        let spawned = std::thread::Builder::new()
            .name("nvs dap program".to_owned())
            // The main thread's own size (`build.rs`'s `main_thread_stack`), because the
            // program is compiled on this thread as `nvs run` compiles it on the main one.
            .stack_size(8 << 20)
            .spawn(move || {
                let code = run(&launch, &config, init, &debugger);
                pipes.close();
                for pump in pumps {
                    let _ = pump.join();
                }
                writer.event("exited", Some(json!({ "exitCode": code })));
                writer.event("terminated", None);
            });
        match spawned {
            Ok(program) => self.program = Some(program),
            Err(error) => {
                let _ = writeln!(
                    self.log,
                    "error: nvs dap could not start the program: {error}"
                );
                self.writer.event("exited", Some(json!({ "exitCode": 1 })));
                self.writer.event("terminated", None);
            }
        }
    }

    fn succeed(&self, request: &Incoming, body: Option<Value>) {
        self.respond(request, true, None, body);
    }

    fn fail(&self, request: &Incoming, message: &str) {
        self.respond(request, false, Some(message), None);
    }

    fn reply(&self, request: &Incoming, body: Result<Value, String>) {
        match body {
            Ok(body) => self.succeed(request, Some(body)),
            Err(message) => self.fail(request, &message),
        }
    }

    fn respond(
        &self,
        request: &Incoming,
        success: bool,
        message: Option<&str>,
        body: Option<Value>,
    ) {
        self.writer.send(|seq| Outgoing::Response {
            seq,
            request_seq: request.seq,
            success,
            command: &request.command,
            message,
            body,
        });
    }
}

#[derive(PartialEq, Eq)]
enum Flow {
    Continue,
    Stop,
}

/// The launch's `skipFiles` globs as one set, or the message the client shows for one that is not
/// a glob. On Windows a `\` in a glob is a separator, as it is in the paths the glob is matched
/// against.
fn skip_files(globs: &[String]) -> Result<globset::GlobSet, String> {
    let mut set = globset::GlobSetBuilder::new();
    for glob in globs {
        let written = if cfg!(windows) {
            glob.replace('\\', "/")
        } else {
            glob.clone()
        };
        let parsed = globset::GlobBuilder::new(&written)
            .case_insensitive(cfg!(windows))
            .literal_separator(true)
            .build()
            .map_err(|error| format!("`skipFiles` has a glob that is not valid: {error}"))?;
        set.add(parsed);
    }
    set.build()
        .map_err(|error| format!("`skipFiles` has a glob that is not valid: {error}"))
}

/// A file key as a glob matches it: Windows's `\\?\` prefix, which a canonical path carries, left
/// off.
fn shown_file(file: &str) -> &str {
    file.strip_prefix(r"\\?\").unwrap_or(file)
}

/// Sets `env` in this process's environment, which is the program's because the program runs
/// here. A `None` removes the variable.
///
/// Called once, before the program's thread starts. The threads running then read pipes and
/// frames and never the environment, and the runtime reads it only once the program runs
/// (`nvs_runtime::environment`), so nothing reads it while it changes.
fn set_env(env: &std::collections::BTreeMap<String, Option<String>>) {
    for (name, value) in env {
        // A name `set_var` would panic on is one no program can read, so it is skipped.
        if name.is_empty() || name.contains(['=', '\0']) {
            continue;
        }
        match value {
            Some(value) if !value.contains('\0') => {
                #[expect(
                    unsafe_code,
                    reason = "`set_var` is unsafe while another thread may read the environment. \
                              No thread reads it now: the program has not started, and the \
                              adapter's own threads read pipes and frames only."
                )]
                // SAFETY: as the reason says, nothing reads the environment during the call.
                unsafe {
                    std::env::set_var(name, value);
                }
            }
            Some(_) => {}
            None => {
                #[expect(
                    unsafe_code,
                    reason = "`remove_var` is unsafe for `set_var`'s reason, and no thread reads \
                              the environment now, as above."
                )]
                // SAFETY: as the reason says, nothing reads the environment during the call.
                unsafe {
                    std::env::remove_var(name);
                }
            }
        }
    }
}

/// Runs the launched program as its mode's command would, under the debug switch and attached to
/// `debugger`, and gives its exit status.
fn run(
    launch: &LaunchArguments,
    config: &[PathBuf],
    init: config::Init,
    debugger: &Debugger,
) -> i64 {
    if let Some(cwd) = &launch.cwd
        && let Err(error) = std::env::set_current_dir(cwd)
    {
        eprintln!(
            "error: cannot change to the folder {}: {error}",
            cwd.display()
        );
        return 1;
    }
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match launch.mode {
        Mode::Run => crate::run_run(
            &launch.program,
            false,
            false,
            None,
            false,
            None,
            None,
            None,
            None,
            config,
            launch.args.clone(),
            init,
            Some(debugger),
        ),
        Mode::Test => test(launch, config, init, debugger),
        Mode::Serve => match serve_address(&launch.args) {
            Ok((listen, port)) => crate::serve::run(
                Some(&launch.program),
                listen.as_deref(),
                port,
                config,
                init,
                Some(debugger),
            ),
            // `launch` has read the same arguments already.
            Err(_) => ExitCode::FAILURE,
        },
    }));
    match ran {
        Ok(code) => exit_number(code),
        // The panic hook has already written the message to stderr, which is an `output`
        // event. 101 is the status a Rust process ends with on a panic.
        Err(_) => 101,
    }
}

/// Runs the program's `#[Test]` methods as `nvs test` would, with `filter` as its `--filter`.
///
/// The tests run one at a time, and each test's output is shown rather than captured. A stop
/// then names the one test that is running, and what each test prints arrives in the order the
/// tests ran.
fn test(
    launch: &LaunchArguments,
    config: &[PathBuf],
    init: config::Init,
    debugger: &Debugger,
) -> ExitCode {
    crate::run_test(
        std::slice::from_ref(&launch.program),
        crate::runner::Selection {
            filter: launch.filter.clone(),
            ..crate::runner::Selection::default()
        },
        None,
        crate::runner::Format::Human,
        crate::runner::Flags {
            update: false,
            list: false,
            jobs: 1,
            show_output: true,
            shared_worker: false,
            fail_fast: false,
            rerun_failed: false,
            shuffle: None,
            record_failed: None,
            mutate: false,
            profile: None,
            debug: Some(debugger.clone()),
        },
        &crate::coverage::Requested::default(),
        None,
        None,
        false,
        config,
        init,
    )
}

/// The address a `serve` launch listens on, read from its `args`: `--listen <address>` or
/// `--port <port>`, as `nvs serve` takes them, or neither for the configuration's own. An
/// argument that is not one of these is the message the client shows.
fn serve_address(args: &[String]) -> Result<(Option<String>, Option<u16>), String> {
    let mut listen = None;
    let mut port = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let (flag, value) = match arg.split_once('=') {
            Some((flag, value)) => (flag, Some(value.to_owned())),
            None => (arg.as_str(), None),
        };
        let value = match value {
            Some(value) => value,
            None if flag == "--listen" || flag == "--port" => rest
                .next()
                .cloned()
                .ok_or_else(|| format!("`{flag}` needs a value."))?,
            None => String::new(),
        };
        match flag {
            "--listen" => listen = Some(value),
            "--port" => {
                port = Some(value.parse::<u16>().map_err(|_| {
                    format!("`--port {value}` is not a port. Write a number from 0 to 65535.")
                })?);
            }
            _ => {
                return Err(format!(
                    "A `serve` launch takes `--listen <address>` or `--port <port>` in `args`, \
                     and `{arg}` is neither."
                ));
            }
        }
    }
    if listen.is_some() && port.is_some() {
        return Err("Write `--listen` or `--port` in `args`, not both.".to_owned());
    }
    Ok((listen, port))
}

/// The number an [`ExitCode`] stands for. `run_run` builds every code it returns from a byte, and
/// `ExitCode` has no reader for it, so the byte is found by comparison.
fn exit_number(code: ExitCode) -> i64 {
    (0..=u8::MAX)
        .find(|byte| ExitCode::from(*byte) == code)
        .map_or(1, i64::from)
}

/// Reads one of the program's pipes until it closes, and writes what arrives as `output` events.
fn pump(mut from: io::PipeReader, category: &'static str, writer: Arc<Writer>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = [0; 8192];
        let mut pending = Vec::new();
        loop {
            match from.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    pending.extend_from_slice(&buffer[..read]);
                    let text = complete_text(&mut pending);
                    if !text.is_empty() {
                        writer.event(
                            "output",
                            Some(json!({ "category": category, "output": text })),
                        );
                    }
                }
            }
        }
        if !pending.is_empty() {
            let text = String::from_utf8_lossy(&pending).into_owned();
            writer.event(
                "output",
                Some(json!({ "category": category, "output": text })),
            );
        }
    })
}

/// Takes the text in `pending` up to a character that a later read may still complete, and leaves
/// that character's bytes behind. A byte that is not UTF-8 becomes U+FFFD.
fn complete_text(pending: &mut Vec<u8>) -> String {
    let len = pending.len();
    let mut keep = 0;
    // A character is at most four bytes, so its first byte is among the last three of an
    // incomplete one.
    for back in 1..=len.min(3) {
        let byte = pending[len - back];
        if byte & 0xC0 == 0x80 {
            continue;
        }
        let needed = match byte {
            0xC0..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF7 => 4,
            _ => 1,
        };
        if needed > back {
            keep = back;
        }
        break;
    }
    let tail = pending.split_off(len - keep);
    let text = String::from_utf8_lossy(pending).into_owned();
    *pending = tail;
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A character split across two reads arrives whole, in the second event.
    #[test]
    fn a_character_split_across_reads_is_held_until_it_is_complete() {
        let bytes = "aé".as_bytes();
        let mut pending = bytes[..2].to_vec();
        assert_eq!(complete_text(&mut pending), "a");
        pending.extend_from_slice(&bytes[2..]);
        assert_eq!(complete_text(&mut pending), "é");
        assert!(pending.is_empty());
    }

    /// Every byte status maps back to its number.
    #[test]
    fn an_exit_code_reads_back_as_its_number() {
        assert_eq!(exit_number(ExitCode::SUCCESS), 0);
        assert_eq!(exit_number(ExitCode::from(3)), 3);
    }
}
