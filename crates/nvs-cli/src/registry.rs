//! `nvs registry` — the commands a registry service runs over its transparency log
//! (`rule:packaging/the-registry-keeps-an-append-only-log`). They are operator surface, hidden from
//! the command list: a user of packages never types them.
//!
//! `log append` reads entries as JSON lines on standard input, checks each against
//! [`entry::Entry`]'s closed list before anything is written, appends their canonical form, and
//! signs the new checkpoint with the key file. The log directory's newest checkpoint is the file
//! `checkpoint` in it, C2SP's name, opened under the key's own verifier; a directory without one
//! is an empty log. The tiles are written before the checkpoint replaces the old one, so a run that
//! stops part way leaves the old checkpoint, and the next append from it writes the same tiles
//! again. Appends to one directory run one at a time: the service serializes them, and an append
//! that races another from the same size fails on the first tile the other wrote differently.
//!
//! The key file is read under `rule:config/ownership-is-the-trust-boundary`'s check: it and its
//! directory must be writable by no account but this one, an administrator and the system.
//!
//! `log verify` reads the checkpoint with a pinned verifier key and checks the whole directory
//! from scratch ([`log::verify`]), every entry included.

use std::fs;
use std::io::{self, BufRead as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nvs_config::trust::{self, Untrusted};

use crate::package::entry::Entry;
use crate::package::log;
use crate::package::note::{Checkpoint, Signer, Verifier};

/// The file in a log directory that holds its newest checkpoint.
const CHECKPOINT: &str = "checkpoint";

/// Commands for the server of a package registry.
#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Work with the registry's log, the list of every change to the registry.
    Log {
        #[command(subcommand)]
        command: LogCommand,
    },
}

#[derive(clap::Subcommand)]
enum LogCommand {
    /// Add entries to the log and sign a new checkpoint.
    ///
    /// The command reads one JSON entry per line from standard input. Each
    /// entry must have `"v": 1` and a `kind` the log knows. If one entry is
    /// wrong, the command adds nothing.
    Append {
        /// The log's directory.
        dir: PathBuf,
        /// The file with the signing key, `PRIVATE+KEY+<name>+...`. Only your
        /// account may be able to change it and its folder.
        #[arg(long, value_name = "FILE")]
        key: PathBuf,
        /// The log's name, the first line of every checkpoint.
        #[arg(long)]
        origin: String,
    },
    /// Check every file of the log against its signed checkpoint.
    ///
    /// The command exits with an error if a file was changed, an entry is not
    /// valid, or the checkpoint's signature is wrong.
    Verify {
        /// The log's directory.
        dir: PathBuf,
        /// The public key the checkpoint is signed with, `<name>+<id>+<key>`.
        #[arg(long)]
        vkey: String,
        /// The log's name, the first line of every checkpoint.
        #[arg(long)]
        origin: String,
    },
}

impl Args {
    /// Runs the `nvs registry` subcommand clap parsed.
    pub(crate) fn run(self) -> ExitCode {
        let Command::Log { command } = self.command;
        let done = match command {
            LogCommand::Append { dir, key, origin } => append(&dir, &key, &origin),
            LogCommand::Verify { dir, vkey, origin } => verify(&dir, &vkey, &origin),
        };
        match done {
            Ok(message) => {
                println!("{message}");
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("error: {message}");
                ExitCode::FAILURE
            }
        }
    }
}

/// `nvs registry log append`.
fn append(dir: &Path, key: &Path, origin: &str) -> Result<String, String> {
    let signer = read_key(key)?;
    let mut entries = Vec::new();
    for (n, line) in io::stdin().lock().lines().enumerate() {
        let line = line.map_err(|err| format!("standard input: {err}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let entry = Entry::parse(line.as_bytes())
            .map_err(|why| format!("line {}: {why}. Nothing was added.", n + 1))?;
        entries.push(entry.canonical());
    }
    let old = match read_checkpoint(dir, origin, &[signer.verifier()])? {
        Some(old) => old,
        None => Checkpoint::empty(origin).map_err(|err| format!("--origin: {err}"))?,
    };
    let (size, root) = log::append(dir, old.size(), &entries).map_err(|err| err.to_string())?;
    let note = Checkpoint::new(origin, size, root)
        .and_then(|checkpoint| checkpoint.sign(&signer))
        .map_err(|err| err.to_string())?;
    let path = dir.join(CHECKPOINT);
    let partial = dir.join(format!("{CHECKPOINT}.partial"));
    fs::write(&partial, note)
        .and_then(|()| fs::rename(&partial, &path))
        .map_err(|err| format!("{}: {err}", path.display()))?;
    Ok(format!(
        "Added {} entries. The log has {size} entries.",
        entries.len()
    ))
}

/// `nvs registry log verify`.
fn verify(dir: &Path, vkey: &str, origin: &str) -> Result<String, String> {
    let verifier = Verifier::parse(vkey).map_err(|err| format!("--vkey: {err}"))?;
    let checkpoint = read_checkpoint(dir, origin, &[verifier])?
        .ok_or_else(|| format!("{} has no `{CHECKPOINT}` file", dir.display()))?;
    log::verify(
        dir,
        checkpoint.size(),
        checkpoint.root(),
        |_, record: &[u8]| {
            let entry = Entry::parse(record)?;
            if entry.canonical() != record {
                return Err("the entry is not in its canonical form".to_owned());
            }
            Ok(())
        },
    )
    .map_err(|err| err.to_string())?;
    Ok(format!(
        "The log `{origin}` has {} entries, and every file matches its checkpoint.",
        checkpoint.size()
    ))
}

/// The signer in the key file at `path`, read once the file passes the ownership check.
fn read_key(path: &Path) -> Result<Signer, String> {
    let checked = trust::check(path).map_err(|why| {
        let head = format!("the key file {}: {}", path.display(), why.message());
        match why {
            Untrusted::Breach { help, .. } => format!("{head}\n  help: {help}"),
            Untrusted::Unreadable(_) => head,
        }
    })?;
    let text = fs::read_to_string(&checked).map_err(|err| format!("{}: {err}", path.display()))?;
    Signer::parse(text.trim()).map_err(|err| format!("{}: {err}", path.display()))
}

/// The checkpoint in `dir`, opened with `verifiers`, or `None` when `dir` has none.
fn read_checkpoint(
    dir: &Path,
    origin: &str,
    verifiers: &[Verifier],
) -> Result<Option<Checkpoint>, String> {
    let path = dir.join(CHECKPOINT);
    match fs::read(&path) {
        Ok(note) => Checkpoint::open(&note, origin, verifiers)
            .map(Some)
            .map_err(|err| format!("{}: {err}", path.display())),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("{}: {err}", path.display())),
    }
}
