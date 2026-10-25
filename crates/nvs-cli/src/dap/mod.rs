//! `nvs dap` — the debug adapter, on standard input and output
//! (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`).
//!
//! A client starts `nvs dap`, sends `initialize`, then `launch` and `configurationDone` in either
//! order, and the program starts once both have arrived. It runs in this process, on a thread of
//! its own, through the same [`crate::run_run`] `nvs run` uses, so a launched program compiles and
//! runs exactly as it would from a terminal. There is no `attach`: nothing ever listens for a
//! debugger.
//!
//! stdout carries frames and nothing else (`rule:ide/stdout-belongs-to-the-protocol`).
//! [`stdio`] moves the process's own streams into pipes before anything runs, and a pump thread per
//! pipe turns what the program writes into `output` events. The program's end is an `exited` and a
//! `terminated` event, written once both pumps have drained, so no output arrives after them.
//!
//! **Ending a session.** `disconnect` ends the adapter, and the program with it, because the
//! program is this process. `terminate` answers with `terminated` and leaves the program running
//! until the client's `disconnect` follows, which every client sends.
//!
//! **What it spends:** the two pump threads and their 8 KiB buffers, one thread for the program,
//! and one frame in flight at a time. Nothing grows with the length of the session.

mod protocol;
mod stdio;

use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use serde_json::{Value, json};

use crate::config;
use protocol::{Capabilities, Incoming, LaunchArguments, Outgoing};

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
        let mut session = Session {
            writer: Arc::new(Writer {
                output: Mutex::new((streams.output, 1)),
            }),
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

/// One session's state: what the client has sent so far, and the program once it runs.
struct Session {
    writer: Arc<Writer>,
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
            "threads" => self.succeed(
                request,
                Some(json!({ "threads": [{ "id": 1, "name": "main" }] })),
            ),
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

    /// Reads a `launch` request's arguments, or the message the client shows when they cannot run.
    fn launch(&mut self, request: &Incoming) -> Result<(), String> {
        if self.launch.is_some() {
            return Err("This session has already launched a program.".to_owned());
        }
        let arguments: LaunchArguments = serde_json::from_value(request.arguments.clone())
            .map_err(|error| format!("The launch configuration is not valid: {error}"))?;
        match arguments.mode.as_deref() {
            None | Some("run") => {}
            Some(mode) => {
                return Err(format!(
                    "`nvs dap` cannot launch in `{mode}` mode yet. Use `run`."
                ));
            }
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
        let spawned = std::thread::Builder::new()
            .name("nvs dap program".to_owned())
            // The main thread's own size (`build.rs`'s `main_thread_stack`), because the
            // program is compiled on this thread as `nvs run` compiles it on the main one.
            .stack_size(8 << 20)
            .spawn(move || {
                let code = run(&launch, &config, init);
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

/// Runs the launched program as `nvs run` would, and gives its exit status.
fn run(launch: &LaunchArguments, config: &[PathBuf], init: config::Init) -> i64 {
    if let Some(cwd) = &launch.cwd
        && let Err(error) = std::env::set_current_dir(cwd)
    {
        eprintln!(
            "error: cannot change to the folder {}: {error}",
            cwd.display()
        );
        return 1;
    }
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        crate::run_run(
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
        )
    }));
    match ran {
        Ok(code) => exit_number(code),
        // The panic hook has already written the message to stderr, which is an `output`
        // event. 101 is the status a Rust process ends with on a panic.
        Err(_) => 101,
    }
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
