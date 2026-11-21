//! `nvs reload` — `rule:config/a-reload-command-checks-now-and-waits`'s command: every running
//! `nvs serve` of this data folder checks its configuration and every loaded program now, and
//! the command prints what each one replied.
//!
//! **The servers are the endpoints in the data folder's `run/`.** The folder resolves as
//! `nvs serve`'s does, from `--data` or beside the binary, and [`nvs_host::endpoint::reach_all`]
//! connects to each live endpoint there and removes every stale one. The command sends
//! [`crate::reload_endpoint::REQUEST`] to every server first and then reads each reply, so
//! several servers of one folder check at the same time. A connection that closes before its
//! line is a server that was stopping, and is reported as stopped.
//!
//! **Exit codes.** `0` when every server replied with nothing refused, `1` when a configuration
//! file or a program was refused or a server could not be reached, and `2` when no server
//! replied. There is no timeout: each compile has its own limits, and a server that stops
//! closes the connection.
//!
//! Cost: one connection per server and one short line each way. Nothing here runs in a server.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nvs_host::endpoint::{self, Connection};
use serde::{Deserialize, Serialize};

use crate::reload_endpoint::REQUEST;

/// Make every running server use the files on disk now.
///
/// Every `nvs serve` process that uses this data folder checks its
/// configuration and every program it has loaded. The command waits until each
/// changed program is compiled, then prints what each server did. It exits with
/// 0 when everything is in use, with 1 when a file has an error, and with 2 when
/// no server is running.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Print the result as JSON.
    #[arg(long)]
    json: bool,
}

impl Args {
    /// Reloads every server of this process's data folder and prints what each replied.
    pub(crate) fn run(self) -> ExitCode {
        let Some(folder) = nvs_config::data::current() else {
            eprintln!(
                "error: no server was reloaded, because this program has no data folder. \
                 Use `--data` to name the folder of the server."
            );
            return print(&[], self.json, &mut std::io::stdout());
        };
        let run = folder.run();
        let servers = match reload_all(&run) {
            Ok(servers) => servers,
            Err(error) => {
                eprintln!("error: could not read the folder `{}`: {error}", run.display());
                return print(&[], self.json, &mut std::io::stdout());
            }
        };
        if servers.is_empty() {
            eprintln!(
                "error: no `nvs serve` process uses the data folder `{}`.",
                folder.root().display()
            );
        }
        print(&servers, self.json, &mut std::io::stdout())
    }
}

/// Sends a reload to every live endpoint in `run`, then reads each reply.
fn reload_all(run: &Path) -> std::io::Result<Vec<Server>> {
    let sent: Vec<(PathBuf, Result<Connection, Outcome>)> = endpoint::reach_all(run)?
        .into_iter()
        .map(|(path, connected)| {
            let sent = connected
                .map_err(|error| Outcome::Unreachable(error.to_string()))
                .and_then(|mut connection| {
                    connection
                        .write_all(REQUEST)
                        .and_then(|()| connection.flush())
                        .map(|()| connection)
                        .map_err(|error| closed_or(&error))
                });
            (path, sent)
        })
        .collect();
    Ok(sent
        .into_iter()
        .map(|(path, sent)| Server {
            pid: pid_of(&path),
            outcome: sent.map_or_else(|outcome| outcome, read_reply),
        })
        .collect())
}

/// The reply line on `connection`, or how the connection ended without one.
fn read_reply(connection: Connection) -> Outcome {
    let mut line = String::new();
    match BufReader::new(connection).read_line(&mut line) {
        Ok(_) if !line.ends_with('\n') => Outcome::Stopped,
        Ok(_) => serde_json::from_str(&line).map_or_else(
            |error| Outcome::Unreachable(format!("the reply is not valid: {error}")),
            Outcome::Replied,
        ),
        Err(error) => closed_or(&error),
    }
}

/// `Stopped` where `error` is the server closing the connection, and `Unreachable` otherwise.
fn closed_or(error: &std::io::Error) -> Outcome {
    use std::io::ErrorKind;
    match error.kind() {
        ErrorKind::BrokenPipe | ErrorKind::ConnectionReset | ErrorKind::UnexpectedEof => {
            Outcome::Stopped
        }
        _ => Outcome::Unreachable(error.to_string()),
    }
}

/// The pid in an endpoint's name, `serve-<pid>-<start>`.
fn pid_of(path: &Path) -> Option<u32> {
    path.file_name()?
        .to_str()?
        .strip_prefix(endpoint::PREFIX)?
        .split('-')
        .next()?
        .parse()
        .ok()
}

/// One server the command reached.
struct Server {
    pid: Option<u32>,
    outcome: Outcome,
}

/// How one server's reload ended.
enum Outcome {
    /// The server replied.
    Replied(Reply),
    /// The connection closed with no reply: the server was stopping.
    Stopped,
    /// The server could not be reached, or its reply could not be read.
    Unreachable(String),
}

/// A server's reply line, as `crate::reload_endpoint` writes it.
#[derive(Deserialize, Serialize)]
struct Reply {
    config: String,
    restart: Vec<String>,
    programs: u64,
    failed: u64,
    refusals: Vec<Refusal>,
}

impl Reply {
    /// Whether anything in the reply was refused.
    fn refused(&self) -> bool {
        self.config == "refused" || self.failed > 0 || !self.refusals.is_empty()
    }
}

/// One refused file of a reply.
#[derive(Deserialize, Serialize)]
struct Refusal {
    kind: String,
    entry: Option<String>,
    file: Option<String>,
    line: Option<u64>,
    message: String,
}

/// One server in the `--json` document. The fields are in this order on purpose: a derived
/// `Serialize` writes them as declared, and `serde_json`'s map would sort them.
#[derive(Serialize)]
struct ServerJson<'a> {
    pid: Option<u32>,
    state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
    #[serde(flatten)]
    reply: Option<&'a Reply>,
}

/// The `--json` document.
#[derive(Serialize)]
struct Document<'a> {
    schema: u32,
    servers: Vec<ServerJson<'a>>,
}

/// The exit code for `servers`: `2` when none replied, `1` when any refused something or could
/// not be reached, and `0` otherwise.
fn exit_code(servers: &[Server]) -> u8 {
    let mut replied = false;
    let mut failed = false;
    for server in servers {
        match &server.outcome {
            Outcome::Replied(reply) => {
                replied = true;
                failed |= reply.refused();
            }
            Outcome::Unreachable(_) => failed = true,
            Outcome::Stopped => {}
        }
    }
    if !replied {
        2
    } else if failed {
        1
    } else {
        0
    }
}

/// Prints `servers` as text or, with `json`, as one document on `out`, and returns the exit code.
fn print(servers: &[Server], json: bool, out: &mut impl Write) -> ExitCode {
    let text = if json { document(servers) } else { lines(servers) };
    let _ = out.write_all(text.as_bytes());
    ExitCode::from(exit_code(servers))
}

/// The `--json` document, ending in a newline.
fn document(servers: &[Server]) -> String {
    let doc = Document {
        schema: 1,
        servers: servers
            .iter()
            .map(|server| {
                let (state, error, reply) = match &server.outcome {
                    Outcome::Replied(reply) => ("replied", None, Some(reply)),
                    Outcome::Stopped => ("stopped", None, None),
                    Outcome::Unreachable(error) => ("unreachable", Some(error.as_str()), None),
                };
                ServerJson {
                    pid: server.pid,
                    state,
                    error,
                    reply,
                }
            })
            .collect(),
    };
    let mut text = serde_json::to_string(&doc).unwrap_or_default();
    text.push('\n');
    text
}

/// The text a person reads: one line per server, then one line per error and restart.
fn lines(servers: &[Server]) -> String {
    use std::fmt::Write as _;
    let mut text = String::new();
    for server in servers {
        let name = server
            .pid
            .map_or_else(|| "server".to_owned(), |pid| format!("server {pid}"));
        match &server.outcome {
            Outcome::Stopped => {
                let _ = writeln!(text, "{name}: stopped before it replied");
            }
            Outcome::Unreachable(error) => {
                let _ = writeln!(text, "{name}: could not be reached: {error}");
            }
            Outcome::Replied(reply) => {
                let _ = writeln!(
                    text,
                    "{name}: configuration {}, {} reloaded, {} with errors",
                    reply.config,
                    count(reply.programs, "program"),
                    count(reply.failed, "program"),
                );
                for refusal in &reply.refusals {
                    let place = match (&refusal.file, refusal.line) {
                        (Some(file), Some(line)) => format!("{file}:{line}"),
                        (Some(file), None) => file.clone(),
                        (None, _) => refusal.kind.clone(),
                    };
                    let _ = writeln!(text, "  error: {place}: {}", refusal.message);
                }
                if !reply.restart.is_empty() {
                    let _ = writeln!(
                        text,
                        "  a restart is needed to change: {}",
                        reply.restart.join(", ")
                    );
                }
            }
        }
    }
    text
}

/// `n` and `noun`, with an `s` when `n` is not one.
fn count(n: u64, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replied(failed: u64, config: &str) -> Server {
        Server {
            pid: Some(7),
            outcome: Outcome::Replied(Reply {
                config: config.to_owned(),
                restart: Vec::new(),
                programs: 1,
                failed,
                refusals: Vec::new(),
            }),
        }
    }

    /// The exit code is `0` when every reply refused nothing, `1` with any refusal or
    /// unreachable server, and `2` when no server replied.
    #[test]
    fn the_exit_code_follows_the_replies() {
        assert_eq!(exit_code(&[]), 2);
        let stopped = Server {
            pid: None,
            outcome: Outcome::Stopped,
        };
        assert_eq!(exit_code(&[stopped]), 2);
        assert_eq!(exit_code(&[replied(0, "applied")]), 0);
        assert_eq!(exit_code(&[replied(1, "unchanged")]), 1);
        assert_eq!(exit_code(&[replied(0, "refused")]), 1);
        let unreachable = Server {
            pid: Some(8),
            outcome: Outcome::Unreachable("denied".to_owned()),
        };
        assert_eq!(exit_code(&[replied(0, "applied"), unreachable]), 1);
    }

    /// An endpoint's name gives its server's pid.
    #[test]
    fn the_pid_is_read_from_the_endpoint_name() {
        assert_eq!(pid_of(Path::new("run/serve-42-1700000000000")), Some(42));
        assert_eq!(pid_of(Path::new("run/other-42-1")), None);
    }
}
