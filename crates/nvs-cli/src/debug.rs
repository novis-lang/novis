//! `nvs debug` — the debug service for one data folder
//! (`rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it`).
//!
//! This module owns the command: the lock, the configuration it reads, and the loop. [`index`] is
//! the SQLite file and its deletion window, and [`ingest`] is what moves spool files into it.
//! [`listen`] serves the viewer on `[debug] listen` from its own threads, and the loop publishes
//! the index's watermark to it after every pass that read something.
//!
//! **A taken port.** A service started by hand stops with an error that names `[debug] listen` and
//! `--listen`, and never moves to another port, because a link to the viewer must outlive a
//! restart. A service a server started with `--parent` prints the same message as a warning and
//! ingests on without the viewer, because that server's requests still need their records.
//!
//! **One service per data folder.** The service holds an exclusive lock on `debug/service.lock`
//! for as long as it runs, because the index has one writer. A second `nvs debug` on the same
//! folder says so and exits. The operating system releases the lock when the process ends, however
//! it ends, so a killed service leaves no lock behind.
//!
//! **Sources in turn.** The spool is one source and each `--tail` file is one more. A pass gives
//! each source at most [`ingest::PASS_BYTES`], in the same order every time, so one enormous file
//! cannot keep the others waiting. A spool file the budget stopped in goes on where it stopped in
//! the next pass.
//!
//! **A failure waits, and nothing else does.** A source or a file that cannot be listed, opened,
//! read or deleted is quarantined under an [`ingest::Backoff`] and listed in `quarantine`, and every
//! other one keeps flowing. An index error, a full disk among them, stops every source under the
//! same backoff, and deletes no file
//! (`rule:testing/a-bad-line-degrades-and-a-bad-file-is-quarantined`).
//!
//! **A server's child ends with its parent.** A development `nvs serve` starts the service through
//! [`Child::start`] with the hidden `--parent` flag and a pipe as its standard input, and holds the
//! pipe's other end. The pipe closes when the server drops the [`Child`] or ends in any other way,
//! killed included, and the service then stops between two passes. A pipe is the signal because
//! every platform closes it for a process that is gone, with no process id to poll and none to be
//! reused. A child that finds the lock held exits quietly: the server then uses the service that
//! is already running.
//!
//! **A server's child knows where to replay.** Once its sockets are bound, the server writes the
//! first TCP address it answers on as one line on that same pipe ([`Child::target`]), and the
//! service keeps it as its [`Target`], the one address `POST /api/replay` sends to. A service
//! started by hand, or a server that binds only a Unix-domain socket, has none.
//!
//! **The loop waits on one channel.** Between two passes the loop waits on a channel of
//! [`Signal`]s, so the parent's end and the viewer's discard of the index each wake it at once. A
//! discard drops the loop's connection, deletes the file and creates it again
//! ([`Index::rebuild`]); the spool's leftovers and each tailed file are then read from their start,
//! and the quarantine list starts empty.
//!
//! **What it spends:** see [`index`], [`ingest`], [`tail`] and [`listen`]. Here, the lock's file handle, and one
//! entry per spool file stopped in or per file failing. Under `--parent`, one thread blocked on
//! standard input. In the server, the child's handle and its pipe.

pub(crate) mod api;
pub(crate) mod exceptions;
pub(crate) mod index;
pub(crate) mod ingest;
pub(crate) mod listen;
pub(crate) mod replay;
pub(crate) mod search;
pub(crate) mod secure;
pub(crate) mod settings;
pub(crate) mod source;
pub(crate) mod stream;
pub(crate) mod tail;

use std::collections::HashMap;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{BufRead as _, Write as _};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Stdio};
use std::sync::{Arc, OnceLock, mpsc};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use nvs_config::{Quantity, Reading, Unit};
use nvs_diagnostics::{Diagnostics, SourceMap};

use crate::config::Init;
use crate::render_diagnostics;
use index::{Index, Window};
use listen::{Listen, Watermark};

/// What wakes the loop between two passes.
pub(crate) enum Signal {
    /// The parent closed the pipe: the loop ends.
    Gone,
    /// The viewer asks for the index to be discarded. The new watermark, or why it failed, goes
    /// back on the channel.
    Discard(mpsc::Sender<Result<i64, String>>),
}

/// Run the debug service: read recorded requests into the debug index, and serve the debug viewer.
///
/// A program records its requests when `[debug] store = true` is set in `nvs.toml`. The records
/// are written to files in the data folder. This command reads those files into one database,
/// `debug/index.sqlite`, and then deletes them.
///
/// The debug viewer shows those requests in a browser. `nvs debug` prints its address, which is
/// `http://127.0.0.1:8137/` unless `[debug] listen` or `--listen` sets another one.
///
/// It runs until you stop it. Records older than `[debug] index_age` are deleted, and so are the
/// oldest records when the database grows past `[debug] index_max`. Only one `nvs debug` runs for
/// a data folder at a time.
///
/// In development, `nvs serve` starts `nvs debug` and stops it when the server stops. If one is
/// already running for the data folder, the server uses that one. In production, run it yourself.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Read the files that are ready once, then exit.
    #[arg(long)]
    once: bool,
    /// Also read the log file at this path, and every line that is added to it later.
    /// The file must use the `json` log format. `nvs debug` never changes the file.
    /// You can give this option more than once.
    #[arg(long, value_name = "FILE")]
    tail: Vec<PathBuf>,
    /// Serve the debug viewer on this IP address and port, such as `127.0.0.1:8137`. This
    /// replaces `[debug] listen` for this run.
    #[arg(long, value_name = "ADDRESS")]
    listen: Option<String>,
    /// Set by `nvs serve` for the service it starts: stop once standard input closes, and exit
    /// quietly when another service holds the lock.
    #[arg(long, hide = true)]
    parent: bool,
}

impl Args {
    /// Runs `nvs debug` with the arguments clap parsed, over the run's `--config` list.
    pub(crate) fn run(mut self, config: &[PathBuf]) -> ExitCode {
        let (window, mut listen) = match settings(config) {
            Ok(settings) => settings,
            Err(code) => return code,
        };
        if let Some(addr) = self.listen.take() {
            listen.addr = addr;
        }
        let Some(folder) = nvs_config::data::current() else {
            eprintln!("error: `nvs debug` needs a data folder, and Novis cannot use one");
            return ExitCode::FAILURE;
        };
        let spool = match folder.prepare_spool() {
            Ok(spool) => spool,
            Err(unusable) => {
                eprintln!("{}", unusable.warning());
                return ExitCode::FAILURE;
            }
        };
        let lock_path = folder.debug().join("service.lock");
        let _lock = match lock(&lock_path) {
            Ok(Some(lock)) => lock,
            Ok(None) if self.parent => return ExitCode::SUCCESS,
            Ok(None) => {
                eprintln!(
                    "error: another `nvs debug` is already running for the data folder `{}`",
                    folder.root().display()
                );
                return ExitCode::FAILURE;
            }
            Err(err) => {
                eprintln!("error: cannot lock `{}`: {err}", lock_path.display());
                return ExitCode::FAILURE;
            }
        };
        let index_path = folder.debug_index();
        let index = match Index::open(&index_path) {
            Ok(index) => index,
            Err(err) => {
                eprintln!("error: cannot open `{}`: {err}", index_path.display());
                return ExitCode::FAILURE;
            }
        };
        let tails = self
            .tail
            .iter()
            .map(|path| std::path::absolute(path).unwrap_or_else(|_| path.clone()))
            .collect();
        let failed = match quarantined(&index, Instant::now()) {
            Ok(failed) => failed,
            Err(err) => {
                eprintln!("error: cannot read `{}`: {err}", index_path.display());
                return ExitCode::FAILURE;
            }
        };
        let watermark = match index.watermark() {
            Ok(batch) => Watermark::new(batch),
            Err(err) => {
                eprintln!("error: cannot read `{}`: {err}", index_path.display());
                return ExitCode::FAILURE;
            }
        };
        let (signal, signals) = mpsc::channel();
        let target = Target::default();
        if !self.once {
            match listen::start(
                &listen,
                folder,
                Arc::clone(&watermark),
                signal.clone(),
                Arc::clone(&target),
            ) {
                Ok(link) => eprintln!("the debug viewer is at {link}"),
                Err(message) if self.parent => eprintln!("warning: {message}"),
                Err(message) => {
                    eprintln!("error: {message}");
                    return ExitCode::FAILURE;
                }
            }
        }
        if self.parent {
            parent_gone(signal.clone(), target);
        }
        let mut service = Service {
            index,
            index_path,
            spool,
            tails,
            resume: HashMap::new(),
            failed,
            stalled: None,
            watermark,
            signals,
            _signal: signal,
        };
        service.serve(window, self.once)
    }
}

/// The address a development server gave its child, the one place a replay is sent
/// (`rule:testing/two-requests-compare-and-a-replay-asks-first`). Empty for a service started by
/// hand.
pub(crate) type Target = Arc<OnceLock<SocketAddr>>;

/// Keeps the parent's first line as `target`, then sends [`Signal::Gone`] once standard input
/// reaches its end, which is the parent closing the pipe or ending.
fn parent_gone(signal: mpsc::Sender<Signal>, target: Target) {
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin().lock();
        let mut line = String::new();
        if stdin.read_line(&mut line).is_ok_and(|read| read > 0) {
            if let Ok(addr) = line.trim().parse() {
                let _ = target.set(addr);
            }
            let _ = std::io::copy(&mut stdin, &mut std::io::sink());
        }
        let _ = signal.send(Signal::Gone);
    });
}

/// The `nvs debug` a development server started, stopped when this value is dropped
/// (`rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it`).
pub(crate) struct Child(std::process::Child);

impl Child {
    /// Starts `nvs debug --parent` for this process's data folder, over the server's `config`
    /// list. `None` when there is no data folder or the program cannot start: the server says so
    /// and serves on, and its requests wait in the spool for the next service.
    pub(crate) fn start(config: &[PathBuf]) -> Option<Self> {
        let folder = nvs_config::data::current()?;
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(err) => {
                eprintln!("warning: cannot start `nvs debug`: {err}");
                return None;
            }
        };
        let mut command = std::process::Command::new(exe);
        // The child finds the default folder the same way, and naming it would check it as a
        // named one (`nvs_config::data`'s module doc).
        if !folder.is_default() {
            command.arg("--data").arg(folder.root());
        }
        for path in config {
            command.arg("--config").arg(path);
        }
        command
            .args(["debug", "--parent"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        match command.spawn() {
            Ok(child) => {
                eprintln!(
                    "`nvs debug` reads this server's recorded requests into `{}`",
                    folder.debug_index().display()
                );
                Some(Self(child))
            }
            Err(err) => {
                eprintln!("warning: cannot start `nvs debug`: {err}");
                None
            }
        }
    }

    /// Tells the service the address this server answers on, its one replay target. A child that
    /// found another service running has ended, and the write fails without a word.
    pub(crate) fn target(&mut self, addr: SocketAddr) {
        if let Some(stdin) = self.0.stdin.as_mut() {
            let _ = writeln!(stdin, "{addr}");
            let _ = stdin.flush();
        }
    }
}

impl Drop for Child {
    /// Closes the pipe and waits for the service to end its pass.
    fn drop(&mut self) {
        drop(self.0.stdin.take());
        let _ = self.0.wait();
    }
}

/// The running service: the index, its sources, and what the module doc's failures left waiting.
struct Service {
    index: Index,
    index_path: PathBuf,
    spool: PathBuf,
    tails: Vec<PathBuf>,
    /// Each spool file the budget stopped in, and where it goes on.
    resume: HashMap<PathBuf, u64>,
    /// Each source or file that failed, and when it is tried again.
    failed: HashMap<PathBuf, ingest::Backoff>,
    /// The index's own failure, which stops every source.
    stalled: Option<ingest::Backoff>,
    /// What the viewer's event streams are sent.
    watermark: Arc<Watermark>,
    /// What wakes the wait between two passes.
    signals: mpsc::Receiver<Signal>,
    /// Keeps `signals` connected when nothing else sends on it.
    _signal: mpsc::Sender<Signal>,
}

impl Service {
    /// The ingest loop: a pass, the deletion window after any pass that read something, and the
    /// wait [`ingest::wait`] names. With `once`, one pass and its deletion. A [`Signal`] ends the
    /// wait early: the loop stops on [`Signal::Gone`] and discards the index on
    /// [`Signal::Discard`].
    fn serve(&mut self, window: Window, once: bool) -> ExitCode {
        let mut last_arrival = Instant::now();
        let mut forget = true;
        loop {
            if self.pass(Instant::now()) > 0 {
                last_arrival = Instant::now();
                forget = true;
                if let Ok(batch) = self.index.watermark() {
                    self.watermark.publish(batch);
                }
            }
            if forget && self.stalled.is_none() {
                if let Err(err) = self.index.forget(window, now_us()) {
                    eprintln!("warning: the debug index could not delete old records: {err}");
                }
                forget = false;
            }
            if once {
                return ExitCode::SUCCESS;
            }
            let wait = ingest::wait(last_arrival, Instant::now());
            match self.signals.recv_timeout(wait) {
                Ok(Signal::Gone) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return ExitCode::SUCCESS;
                }
                Ok(Signal::Discard(reply)) => {
                    let _ = reply.send(self.rebuild());
                    last_arrival = Instant::now();
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }

    /// Discards the index and creates it again, and forgets every file's place and failure, which
    /// the old file held. Answers the new watermark.
    fn rebuild(&mut self) -> Result<i64, String> {
        let next = self.index.rebuild(&self.index_path)?;
        self.resume.clear();
        self.failed.clear();
        self.stalled = None;
        self.watermark.publish(next);
        eprintln!(
            "the debug index `{}` was discarded and is read again",
            self.index_path.display()
        );
        Ok(next)
    }

    /// One pass over every source in turn. Answers how many sources read something.
    fn pass(&mut self, now: Instant) -> u64 {
        if self.stalled.is_some_and(|stalled| now < stalled.until) {
            return 0;
        }
        match self.sources(now) {
            Ok(arrived) => {
                self.stalled = None;
                arrived
            }
            Err(err) => {
                let backoff = ingest::Backoff::after(self.stalled, now);
                eprintln!(
                    "warning: cannot write to the debug index: {err}. It is tried again in {} s.",
                    backoff.delay.as_secs()
                );
                self.stalled = Some(backoff);
                0
            }
        }
    }

    /// The spool, then each tailed file, each up to [`ingest::PASS_BYTES`].
    fn sources(&mut self, now: Instant) -> rusqlite::Result<u64> {
        let mut arrived = u64::from(self.spool(now)?);
        for tail in self.tails.clone() {
            if !self.due(&tail, now) {
                continue;
            }
            match tail::pass(&mut self.index, &tail, ingest::PASS_BYTES, now_us()) {
                Ok(read) => {
                    arrived += u64::from(read.bytes > 0);
                    self.recovered(&tail)?;
                }
                Err(ingest::Failed::Read(err) | ingest::Failed::Delete(err)) => {
                    self.fail(&tail, &err, now)?;
                }
                Err(ingest::Failed::Index(err)) => return Err(err),
            }
        }
        Ok(arrived)
    }

    /// The spool's share of a pass: every file [`ingest::ready`] lists, in order, until the budget
    /// is spent. Answers whether a file was read.
    fn spool(&mut self, now: Instant) -> rusqlite::Result<bool> {
        let spool = self.spool.clone();
        if !self.due(&spool, now) {
            return Ok(false);
        }
        let files = match ingest::ready(&spool, SystemTime::now()) {
            Ok(files) => files,
            Err(err) => {
                self.fail(&spool, &err, now)?;
                return Ok(false);
            }
        };
        self.recovered(&spool)?;
        self.resume.retain(|path, _| files.contains(path));
        let gone: Vec<PathBuf> = self
            .failed
            .keys()
            .filter(|path| path.parent() == Some(&spool) && !files.contains(path))
            .cloned()
            .collect();
        for path in gone {
            self.recovered(&path)?;
        }
        let mut budget = ingest::PASS_BYTES;
        let mut arrived = false;
        for path in files {
            if budget == 0 {
                break;
            }
            if !self.due(&path, now) {
                continue;
            }
            let from = self.resume.get(&path).copied().unwrap_or(0);
            match ingest::file(&mut self.index, &path, from, budget, now_us()) {
                Ok(read) => {
                    arrived = true;
                    budget = budget.saturating_sub(read.bytes);
                    match read.next {
                        Some(next) => self.resume.insert(path.clone(), next),
                        None => self.resume.remove(&path),
                    };
                    self.recovered(&path)?;
                }
                // The writer evicted it after the listing.
                Err(ingest::Failed::Read(err)) if err.kind() == std::io::ErrorKind::NotFound => {
                    self.resume.remove(&path);
                }
                // A failed delete was committed: the next try reads it again and adds nothing.
                Err(ingest::Failed::Read(err) | ingest::Failed::Delete(err)) => {
                    arrived = true;
                    self.resume.remove(&path);
                    self.fail(&path, &err, now)?;
                }
                Err(ingest::Failed::Index(err)) => return Err(err),
            }
        }
        Ok(arrived)
    }

    /// Whether `path` is not waiting out a failure at `now`.
    fn due(&self, path: &Path, now: Instant) -> bool {
        self.failed
            .get(path)
            .is_none_or(|backoff| now >= backoff.until)
    }

    /// Quarantines `path` after it failed with `err` at `now`.
    fn fail(&mut self, path: &Path, err: &std::io::Error, now: Instant) -> rusqlite::Result<()> {
        let backoff = ingest::Backoff::after(self.failed.get(path).copied(), now);
        eprintln!(
            "warning: cannot read `{}`: {err}. It is tried again in {} s.",
            path.display(),
            backoff.delay.as_secs()
        );
        self.failed.insert(path.to_owned(), backoff);
        ingest::quarantine(&self.index, path, &err.to_string(), backoff, now_us())
    }

    /// Releases `path` from quarantine, when it was in it.
    fn recovered(&mut self, path: &Path) -> rusqlite::Result<()> {
        if self.failed.remove(path).is_some() {
            ingest::release(&self.index, path)?;
        }
        Ok(())
    }
}

/// Each file an earlier run left in `quarantine`, due at `now`, so this run releases it once it
/// is read or gone and counts its failures on.
fn quarantined(index: &Index, now: Instant) -> rusqlite::Result<HashMap<PathBuf, ingest::Backoff>> {
    let mut rows = index.db.prepare("SELECT path, failures FROM quarantine")?;
    let rows = rows.query_map([], |row| {
        let path: String = row.get(0)?;
        let failures: u32 = row.get(1)?;
        Ok((
            PathBuf::from(path),
            ingest::Backoff {
                failures,
                delay: std::time::Duration::ZERO,
                until: now,
            },
        ))
    })?;
    rows.collect()
}

/// The lock on `path`, or `None` when another process holds it.
fn lock(path: &Path) -> std::io::Result<Option<File>> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(err)) => Err(err),
    }
}

/// `[debug] index_age` and `index_max`, then `listen` and the three files it serves with off
/// loopback, from the working directory's configuration.
fn settings(config: &[PathBuf]) -> Result<(Window, Listen), ExitCode> {
    let mut sources = SourceMap::new();
    let snapshot = match crate::config::boot_origins(config, None, &mut sources, Init::Never) {
        Ok((snapshot, _)) => snapshot,
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &sources);
            return Err(ExitCode::FAILURE);
        }
    };
    let request = nvs_config::Request::new(snapshot);
    let age_key = "debug.index_age";
    let max_key = "debug.index_max";
    let age_us = match Reading::of(age_key, Unit::Duration, request.get(age_key)) {
        Reading::Read(Quantity::Nanos(nanos)) => nanos / 1_000,
        Reading::Read(Quantity::Unbounded) => u64::MAX,
        _ => duration_us(nvs_config::defaults::DEBUG_INDEX_AGE),
    };
    let max_bytes = match Reading::of(max_key, Unit::Bytes, request.get(max_key)) {
        Reading::Read(Quantity::Bytes(bytes)) => bytes,
        Reading::Read(Quantity::Unbounded) => u64::MAX,
        _ => nvs_config::defaults::DEBUG_INDEX_MAX,
    };
    let path = |key: &str| request.get(key).map(PathBuf::from);
    let listen = Listen {
        addr: request
            .get("debug.listen")
            .unwrap_or_else(|| nvs_config::defaults::DEBUG_LISTEN.to_owned()),
        cert: path("debug.tls_cert_file"),
        key: path("debug.tls_key_file"),
        token: path("debug.token_file"),
    };
    Ok((Window { age_us, max_bytes }, listen))
}

fn duration_us(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

/// The wall clock in Unix microseconds, which the spool's times are in.
fn now_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, duration_us)
}
