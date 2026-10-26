//! `nvs debug` — the debug service for one data folder
//! (`rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it`).
//!
//! This module owns the command: the lock, the configuration it reads, and the loop. [`index`] is
//! the SQLite file and its deletion window, and [`ingest`] is what moves spool files into it. The
//! viewer half of the service, with its listener, is `debug-viewer`'s and is not built here.
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
//! **What it spends:** see [`index`], [`ingest`] and [`tail`]. Here, the lock's file handle, and one
//! entry per spool file stopped in or per file failing.

pub(crate) mod index;
pub(crate) mod ingest;
pub(crate) mod tail;

use std::collections::HashMap;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use nvs_config::{Quantity, Reading, Unit};
use nvs_diagnostics::{Diagnostics, SourceMap};

use crate::config::Init;
use crate::render_diagnostics;
use index::{Index, Window};

/// Run the debug service: read recorded requests into the debug index.
///
/// A program records its requests when `[debug] store = true` is set in `nvs.toml`. The records
/// are written to files in the data folder. This command reads those files into one database,
/// `debug/index.sqlite`, and then deletes them.
///
/// It runs until you stop it. Records older than `[debug] index_age` are deleted, and so are the
/// oldest records when the database grows past `[debug] index_max`. Only one `nvs debug` runs for
/// a data folder at a time.
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
}

impl Args {
    /// Runs `nvs debug` with the arguments clap parsed, over the run's `--config` list.
    pub(crate) fn run(self, config: &[PathBuf]) -> ExitCode {
        let window = match window(config) {
            Ok(window) => window,
            Err(code) => return code,
        };
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
        let mut service = Service {
            index,
            spool,
            tails,
            resume: HashMap::new(),
            failed,
            stalled: None,
        };
        service.serve(window, self.once)
    }
}

/// The running service: the index, its sources, and what the module doc's failures left waiting.
struct Service {
    index: Index,
    spool: PathBuf,
    tails: Vec<PathBuf>,
    /// Each spool file the budget stopped in, and where it goes on.
    resume: HashMap<PathBuf, u64>,
    /// Each source or file that failed, and when it is tried again.
    failed: HashMap<PathBuf, ingest::Backoff>,
    /// The index's own failure, which stops every source.
    stalled: Option<ingest::Backoff>,
}

impl Service {
    /// The ingest loop: a pass, the deletion window after any pass that read something, and the
    /// wait [`ingest::wait`] names. With `once`, one pass and its deletion.
    fn serve(&mut self, window: Window, once: bool) -> ExitCode {
        let mut last_arrival = Instant::now();
        let mut forget = true;
        loop {
            if self.pass(Instant::now()) > 0 {
                last_arrival = Instant::now();
                forget = true;
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
            std::thread::sleep(ingest::wait(last_arrival, Instant::now()));
        }
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

/// `[debug] index_age` and `[debug] index_max` from the working directory's configuration.
fn window(config: &[PathBuf]) -> Result<Window, ExitCode> {
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
    Ok(Window { age_us, max_bytes })
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
