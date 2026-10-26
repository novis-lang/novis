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
//! **What it spends:** see [`index`] and [`ingest`]. Nothing here holds more than the lock's file
//! handle.

pub(crate) mod index;
pub(crate) mod ingest;

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
        let mut index = match Index::open(&index_path) {
            Ok(index) => index,
            Err(err) => {
                eprintln!("error: cannot open `{}`: {err}", index_path.display());
                return ExitCode::FAILURE;
            }
        };
        serve(&mut index, &spool, window, self.once)
    }
}

/// The ingest loop: a pass over `spool`, the deletion window after any pass that read a file,
/// and the wait [`ingest::wait`] names. With `once`, one pass and its deletion.
fn serve(index: &mut Index, spool: &Path, window: Window, once: bool) -> ExitCode {
    let mut last_arrival = Instant::now();
    let mut forget = true;
    loop {
        match pass(index, spool) {
            Ok(0) => {}
            Ok(_) => {
                last_arrival = Instant::now();
                forget = true;
            }
            Err(err) => {
                eprintln!("error: cannot list `{}`: {err}", spool.display());
                return ExitCode::FAILURE;
            }
        }
        if forget {
            if let Err(err) = index.forget(window, now_us()) {
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

/// One pass over `spool`: every file [`ingest::ready`] lists, ingested. Answers how many files it
/// read.
fn pass(index: &mut Index, spool: &Path) -> std::io::Result<u64> {
    let mut read = 0;
    for path in ingest::ready(spool, SystemTime::now())? {
        match ingest::file(index, &path, now_us()) {
            Ok(_) => read += 1,
            // Committed: the next pass reads the file again and adds nothing.
            Err(ingest::Failed::Delete(err)) => {
                eprintln!("warning: cannot delete `{}`: {err}", path.display());
                read += 1;
            }
            Err(ingest::Failed::Read(err)) => {
                eprintln!("warning: cannot read `{}`: {err}", path.display());
            }
            Err(ingest::Failed::Index(err)) => {
                eprintln!(
                    "warning: cannot write `{}` to the debug index: {err}",
                    path.display()
                );
            }
        }
    }
    Ok(read)
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
