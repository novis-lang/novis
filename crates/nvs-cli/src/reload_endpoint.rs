//! The reload endpoint of a running `nvs serve`: [`nvs_host::endpoint::Endpoint`], opened in the
//! data folder's `run/` before `READY=1` and removed when the process ends
//! (`rule:config/a-reload-command-checks-now-and-waits`).
//!
//! [`open`] first removes every endpoint in `run/` that nobody listens at, which is what a process
//! leaves behind when it was killed before its drain ended. Then it binds this process's own,
//! named `serve-<pid>-<start>`, and starts one thread that accepts on it. An endpoint that cannot
//! be opened is one warning, and the server serves without it: `nvs reload` does not reach that
//! process, and the timed check still does.
//!
//! **A Unix socket path has a length limit**, 108 bytes on Linux and 104 on macOS. A data folder
//! whose `run/` path is longer gets no endpoint and the warning names the path. A shorter
//! `--data` is the fix, and no second place for the socket exists to keep track of.
//!
//! **The wire is one line each way.** A connection sends [`REQUEST`], `reload 1\n`, and nothing
//! after it. The endpoint reads at most [`LINE`] bytes, and closes any other request without a
//! reply. A reload runs the configuration check ([`crate::reload::Process::reload_now`]) and then
//! checks every loaded program ([`crate::script::Compiler::revalidate_now`]), and the reply is one
//! JSON line once every compile it started has ended:
//!
//! ```text
//! {"config":"applied","restart":[],"programs":1,"failed":1,
//!  "refusals":[{"kind":"program","entry":"/srv/app/index.nvs","file":"/srv/app/lib.nvs","line":3,"message":"..."}]}
//! ```
//!
//! `config` is `applied`, `refused` or `unchanged`. `programs` counts the programs that serve a
//! new unit, and `failed` the programs that do not compile. Each refusal has its `kind`
//! (`configuration` or `program`), the program's `entry`, and the `file`, `line` and `message`
//! of its first error; `line` is `null` where the error has no place in a file. The reply carries
//! the error text because only the serving account or an administrator can connect, and both
//! can read the log already. A draining process replies [`STOPPING`], and a check that panics
//! replies [`PANICKED`], a refusal of kind `server`, so every request read in full gets a line.
//!
//! **Two reloads at once share one check.** A reply only comes from a check that began after the
//! reload arrived, because an earlier one may have read the files before the caller wrote them.
//! A reload that arrives while a reload's check runs waits for it, then one more check runs, and
//! every reload that arrived meanwhile shares that check and its reply ([`Rounds`]). A timed
//! check runs beside a reload: the configuration half waits on the one reload lock, and a program
//! both compile is compiled once and waited behind.
//!
//! Each connection is answered on a thread of its own, and at most [`CONNECTIONS`] are answered
//! at once. **One deadline bounds what the peer controls** ([`Deadline`]): the request must
//! arrive within [`PEER_DEADLINE`] of the accept, and the reply must be written within as long
//! again. The check between them has no deadline, because it waits for compiles. A peer past the
//! deadline is cut off by a [`nvs_host::endpoint::Breaker`], on Unix and on Windows alike. An
//! error from the accept is logged and the accept goes on.
//!
//! Cost: one thread per process, blocked in its accept, one socket or pipe instance, and two short
//! threads per connection while it is answered: the answer and its deadline. Nothing on the
//! request path. Each short thread hands its allocator's cached blocks back before it ends
//! (`nvs_runtime::budget::run_then_release_pooled_blocks`). The answer's thread compiles every
//! loaded program, so without that each reload kept up to about 1 MiB of freed blocks until the
//! process ended.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use nvs_host::endpoint::{self, Breaker, Connection, Endpoint};

use crate::reload::{Configured, Process};
use crate::script::Rechecked;

/// The most a request to the endpoint may send: [`REQUEST`] and room to spare.
pub(crate) const LINE: usize = 64;

/// The one request the endpoint answers.
pub(crate) const REQUEST: &[u8] = b"reload 1\n";

/// How many connections are answered at once. A connection over it is closed at once.
const CONNECTIONS: usize = 16;

/// How long a peer has to send its request, and again to take the reply.
const PEER_DEADLINE: Duration = Duration::from_secs(5);

/// How often a deadline that passed cuts the connection again, until its answer ends. On Windows
/// a cut ends only the read or write that waits at that moment.
const RECUT: Duration = Duration::from_millis(50);

/// The reply of a process that is draining: it checked nothing.
pub(crate) const STOPPING: &str = "{\"stopping\":true}\n";

/// The reply of a check that panicked. The panic itself is in the server's log.
const PANICKED: &str = "{\"config\":\"error\",\"restart\":[],\"programs\":0,\"failed\":0,\
    \"refusals\":[{\"kind\":\"server\",\"entry\":null,\"file\":null,\"line\":null,\
    \"message\":\"the server stopped its check with an internal error. Its log has the details.\"}]}\n";

/// This process's endpoint, removed when the value is dropped.
pub(crate) struct Opened {
    endpoint: Arc<Endpoint>,
}

impl Drop for Opened {
    fn drop(&mut self) {
        if let Err(err) = self.endpoint.remove() {
            eprintln!(
                "warning: the reload endpoint `{}` was not removed: {err}",
                self.endpoint.path().display()
            );
        }
    }
}

/// Opens this process's endpoint in the data folder's `run/`, answering for `process`, or warns
/// once and returns `None`. A process with no usable data folder has no endpoint and warns
/// nothing more: the data folder's own warning was printed at the start.
pub(crate) fn open(process: &Arc<Process>) -> Option<Opened> {
    let folder = nvs_config::data::current()?;
    let run = folder.run();
    match bind(&run, process) {
        Ok(endpoint) => Some(Opened { endpoint }),
        Err(err) => {
            eprintln!(
                "warning: `nvs reload` cannot reach this server, because its endpoint in `{}` \
                 could not be opened: {err}",
                run.display()
            );
            None
        }
    }
}

/// Removes the stale endpoints in `run`, binds this process's own and starts its thread.
fn bind(run: &Path, process: &Arc<Process>) -> std::io::Result<Arc<Endpoint>> {
    nvs_config::data::create_private_root(run)?;
    // Each live endpoint reached here closes this connection at once, and each stale one is
    // removed.
    drop(endpoint::reach_all(run)?);
    let start_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    let path = run.join(endpoint::name(std::process::id(), start_ms));
    let endpoint = Arc::new(Endpoint::bind(&path)?);
    let accepting = Arc::clone(&endpoint);
    let server = Arc::new(Server {
        process: Arc::clone(process),
        rounds: Rounds::default(),
        open: AtomicUsize::new(0),
    });
    std::thread::Builder::new()
        .name("nvs-reload".to_owned())
        .spawn(move || accept(&accepting, &server))?;
    Ok(endpoint)
}

/// What the endpoint's threads share.
struct Server {
    process: Arc<Process>,
    rounds: Rounds,
    /// The connections being answered now.
    open: AtomicUsize,
}

/// Accepts for as long as the process runs, and answers each connection on a thread of its own.
/// An accept error is logged and the accept goes on, after [`RECUT`] so a lasting error cannot
/// spin. The same error twice in a row is logged once.
fn accept(endpoint: &Endpoint, server: &Arc<Server>) {
    let mut last_error = None;
    loop {
        let connection = match endpoint.accept() {
            Ok(connection) => {
                last_error = None;
                connection
            }
            Err(err) => {
                let text = err.to_string();
                if last_error.as_ref() != Some(&text) {
                    eprintln!(
                        "warning: the reload endpoint `{}` could not accept a connection: {text}",
                        endpoint.path().display()
                    );
                    last_error = Some(text);
                }
                std::thread::sleep(RECUT);
                continue;
            }
        };
        if server.open.fetch_add(1, Ordering::AcqRel) >= CONNECTIONS {
            server.open.fetch_sub(1, Ordering::AcqRel);
            continue;
        }
        let answering = Arc::clone(server);
        let spawned = std::thread::Builder::new()
            .name("nvs-reload-answer".to_owned())
            .spawn(move || {
                nvs_runtime::budget::run_then_release_pooled_blocks(|| {
                    answer(&answering, connection);
                    answering.open.fetch_sub(1, Ordering::AcqRel);
                });
            });
        if spawned.is_err() {
            server.open.fetch_sub(1, Ordering::AcqRel);
        }
    }
}

/// Reads one request from `connection` and writes the reply of the check it shares, each under
/// the [`Deadline`]. Anything but [`REQUEST`] is closed without a reply, and so is a connection
/// whose deadline cannot be set.
fn answer(server: &Server, mut connection: Connection) {
    let Some(deadline) = connection.breaker().ok().and_then(Deadline::start) else {
        return;
    };
    deadline.set(Some(Instant::now() + PEER_DEADLINE));
    if read_line(&mut connection).as_deref() != Some(REQUEST) {
        return;
    }
    deadline.set(None);
    let reply = server.rounds.shared(|| check(&server.process));
    deadline.set(Some(Instant::now() + PEER_DEADLINE));
    drop(
        connection
            .write_all(reply.as_bytes())
            .and_then(|()| connection.flush()),
    );
}

/// The deadline of one connection: a thread that cuts the connection once the time set passes,
/// and again every [`RECUT`] until the answer ends. Dropping it ends the thread.
struct Deadline {
    shared: Arc<(Mutex<Watch>, Condvar)>,
}

/// What the deadline's thread waits for.
enum Watch {
    /// The connection is cut at this moment.
    Until(Instant),
    /// The check runs, and nothing is cut.
    Paused,
    /// The answer ended.
    Done,
}

impl Deadline {
    /// Starts the deadline's thread, paused, or `None` where the thread cannot start.
    fn start(breaker: Breaker) -> Option<Self> {
        let shared = Arc::new((Mutex::new(Watch::Paused), Condvar::new()));
        let watching = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("nvs-reload-deadline".to_owned())
            .spawn(move || {
                nvs_runtime::budget::run_then_release_pooled_blocks(|| watch(&watching, &breaker));
            })
            .ok()?;
        Some(Self { shared })
    }

    /// Cuts the connection at `at`, or never with `None`.
    fn set(&self, at: Option<Instant>) {
        let (state, changed) = &*self.shared;
        *state.lock().unwrap_or_else(PoisonError::into_inner) =
            at.map_or(Watch::Paused, Watch::Until);
        changed.notify_one();
    }
}

impl Drop for Deadline {
    fn drop(&mut self) {
        let (state, changed) = &*self.shared;
        *state.lock().unwrap_or_else(PoisonError::into_inner) = Watch::Done;
        changed.notify_one();
    }
}

/// The deadline's thread: waits for the time set and cuts the connection with `breaker` once it
/// passed, until the answer ends.
fn watch(shared: &(Mutex<Watch>, Condvar), breaker: &Breaker) {
    let (state, changed) = shared;
    let mut watch = state.lock().unwrap_or_else(PoisonError::into_inner);
    loop {
        let wait = match *watch {
            Watch::Done => return,
            Watch::Paused => None,
            Watch::Until(at) => {
                let now = Instant::now();
                if now >= at {
                    breaker.break_now();
                    Some(RECUT)
                } else {
                    Some(at - now)
                }
            }
        };
        watch = match wait {
            None => changed.wait(watch).unwrap_or_else(PoisonError::into_inner),
            Some(wait) => {
                changed
                    .wait_timeout(watch, wait)
                    .unwrap_or_else(PoisonError::into_inner)
                    .0
            }
        };
    }
}

/// The bytes up to and including the first newline, or `None` where the connection closes first,
/// sends more than [`LINE`] bytes before one, or sends anything after it.
fn read_line(connection: &mut impl Read) -> Option<Vec<u8>> {
    let mut buf = [0_u8; LINE];
    let mut len = 0;
    while len < LINE {
        let read = connection.read(&mut buf[len..]).ok()?;
        if read == 0 {
            return None;
        }
        len += read;
        if let Some(end) = buf[..len].iter().position(|byte| *byte == b'\n') {
            return (end + 1 == len).then(|| buf[..len].to_vec());
        }
    }
    None
}

/// One reload: the configuration, then every loaded program, as the reply line. A process that
/// is draining replies [`STOPPING`].
fn check(process: &Process) -> String {
    let Some(configured) = process.reload_now() else {
        return STOPPING.to_owned();
    };
    let programs = process.compiler().revalidate_now();
    let swapped = programs
        .iter()
        .filter(|program| matches!(program, Rechecked::Swapped { .. }))
        .count();
    process.reloaded(&configured, swapped, programs.len() - swapped);
    reply(&configured, &programs)
}

/// The reply line of one reload, ending in a newline.
fn reply(configured: &Configured, programs: &[Rechecked]) -> String {
    let mut refusals = Vec::new();
    let (config, restart) = match configured {
        Configured::Unchanged => ("unchanged", &[][..]),
        Configured::Applied { restart } => ("applied", &restart[..]),
        Configured::Refused { message, at } => {
            refusals.push(refusal("configuration", None, message, at.as_ref()));
            ("refused", &[][..])
        }
    };
    let mut swapped = 0_usize;
    let mut failed = 0_usize;
    for program in programs {
        match program {
            Rechecked::Swapped { .. } => swapped += 1,
            Rechecked::Failed {
                entry,
                message,
                first,
            } => {
                failed += 1;
                refusals.push(refusal("program", Some(entry), message, first.as_ref()));
            }
        }
    }
    let mut line = serde_json::json!({
        "config": config,
        "restart": restart,
        "programs": swapped,
        "failed": failed,
        "refusals": refusals,
    })
    .to_string();
    line.push('\n');
    line
}

/// One refusal of the reply. A program's error with no place in a file names its entry file.
fn refusal(
    kind: &str,
    entry: Option<&PathBuf>,
    message: &str,
    at: Option<&crate::Located>,
) -> serde_json::Value {
    let entry = entry.map(|path| path.display().to_string());
    let placed = at.filter(|at| !at.file.is_empty());
    serde_json::json!({
        "kind": kind,
        "entry": entry,
        "file": placed.map_or_else(|| entry.clone(), |at| Some(at.file.clone())),
        "line": placed.map(|at| at.line),
        "message": at.map_or(message, |at| at.message.as_str()),
    })
}

/// The reloads' checks, numbered, so a reload is answered only by a check that began after it
/// arrived and every reload waiting for the next check shares it.
#[derive(Default)]
struct Rounds {
    state: Mutex<Round>,
    finished: Condvar,
}

/// Where the checks stand.
#[derive(Default)]
struct Round {
    /// The number of the last check that began.
    started: u64,
    /// Whether that check is still running.
    running: bool,
    /// The number of the last check that ended, and its reply.
    finished: u64,
    reply: Arc<str>,
    /// How many reloads wait for a check to end.
    waiting: usize,
}

impl Rounds {
    /// The reply of the first check that begins after this call: `run` on this thread, where no
    /// check is running and nobody began that one yet, and otherwise the reply of the check this
    /// call waits for. A later check's reply also began after this call, so it answers too. A
    /// check that panics replies [`PANICKED`].
    fn shared(&self, run: impl FnOnce() -> String) -> Arc<str> {
        let mut round = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let mine = round.started + 1;
        loop {
            if round.finished >= mine {
                return round.reply.clone();
            }
            if !round.running && round.started < mine {
                round.started = mine;
                round.running = true;
                drop(round);
                // A panic in a check is that check's failure, replied as one, and ends no waiter.
                let reply = Arc::<str>::from(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run))
                        .unwrap_or_else(|_| PANICKED.to_owned()),
                );
                let mut round = self.state.lock().unwrap_or_else(PoisonError::into_inner);
                round.running = false;
                round.finished = mine;
                round.reply.clone_from(&reply);
                drop(round);
                self.finished.notify_all();
                return reply;
            }
            round.waiting += 1;
            round = self
                .finished
                .wait(round)
                .unwrap_or_else(PoisonError::into_inner);
            round.waiting -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn a_request_is_one_line_of_at_most_line_bytes() {
        let read = |bytes: &[u8]| read_line(&mut &bytes[..]);
        assert_eq!(read(b"reload 1\n").as_deref(), Some(REQUEST));
        assert_eq!(read(b"reload 1"), None, "no newline before the end");
        assert_eq!(read(b"reload 1\nmore"), None, "bytes after the line");
        assert_eq!(read(&[b'x'; LINE + 10]), None, "longer than the line");
    }

    #[test]
    fn reloads_that_arrive_during_a_check_share_the_next_one() {
        let rounds = Arc::new(Rounds::default());
        let (began, first_running) = mpsc::channel();
        let (release, hold) = mpsc::channel::<()>();
        let first = {
            let rounds = Arc::clone(&rounds);
            std::thread::spawn(move || {
                rounds.shared(|| {
                    began.send(()).expect("the test waits");
                    hold.recv().expect("the test releases the check");
                    "first".to_owned()
                })
            })
        };
        first_running.recv().expect("the first check began");
        let checks = Arc::new(AtomicUsize::new(0));
        let waiters: Vec<_> = (0..3)
            .map(|_| {
                let rounds = Arc::clone(&rounds);
                let checks = Arc::clone(&checks);
                std::thread::spawn(move || {
                    rounds.shared(|| {
                        checks.fetch_add(1, Ordering::SeqCst);
                        "second".to_owned()
                    })
                })
            })
            .collect();
        // The test waits on the round's state, not on time, until all three wait behind the
        // running check.
        while rounds.state.lock().expect("no check panicked").waiting < 3 {
            std::thread::yield_now();
        }
        release.send(()).expect("the first check waits");
        assert_eq!(&*first.join().expect("no panic"), "first");
        for waiter in waiters {
            assert_eq!(&*waiter.join().expect("no panic"), "second");
        }
        assert_eq!(
            checks.load(Ordering::SeqCst),
            1,
            "the three shared one check"
        );
    }

    /// A check that panics replies as a refused server, and the next check runs as usual.
    #[test]
    fn a_check_that_panics_is_a_reply() {
        let rounds = Rounds::default();
        let reply = rounds.shared(|| panic!("a check failed"));
        let doc: serde_json::Value = serde_json::from_str(&reply).expect("the reply is JSON");
        assert_eq!(doc["config"], "error");
        assert_eq!(doc["refusals"][0]["kind"], "server");
        assert_eq!(&*rounds.shared(|| "next".to_owned()), "next");
        let stopping: serde_json::Value = serde_json::from_str(STOPPING).expect("JSON");
        assert_eq!(stopping["stopping"], true);
    }

    #[test]
    fn a_reply_counts_swaps_and_failures_and_names_each_refusal() {
        let line = reply(
            &Configured::Applied {
                restart: vec!["server.workers".to_owned()],
            },
            &[
                Rechecked::Swapped {
                    entry: PathBuf::from("a.nvs"),
                    program: nvs_config::cache::Digest::from_bytes([1; 32]),
                    compiled: 1,
                    reused: 2,
                },
                Rechecked::Failed {
                    entry: PathBuf::from("b.nvs"),
                    message: "`b.nvs` could not be compiled; see the errors above".to_owned(),
                    first: Some(crate::Located {
                        file: "lib.nvs".to_owned(),
                        line: 3,
                        message: "unexpected `;`".to_owned(),
                    }),
                },
            ],
        );
        assert!(line.ends_with('\n') && line.matches('\n').count() == 1);
        let doc: serde_json::Value = serde_json::from_str(&line).expect("the reply is JSON");
        assert_eq!(doc["config"], "applied");
        assert_eq!(doc["restart"][0], "server.workers");
        assert_eq!(doc["programs"], 1);
        assert_eq!(doc["failed"], 1);
        let refusal = &doc["refusals"][0];
        assert_eq!(refusal["kind"], "program");
        assert_eq!(refusal["entry"], "b.nvs");
        assert_eq!(refusal["file"], "lib.nvs");
        assert_eq!(refusal["line"], 3);
        assert_eq!(refusal["message"], "unexpected `;`");
    }
}
