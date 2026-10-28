//! `nvs registry` — the commands a registry service runs: its build of each release
//! (`rule:packaging/the-registry-builds-the-archive-from-git`) and its transparency log
//! (`rule:packaging/the-registry-keeps-an-append-only-log`). They are operator surface, hidden from
//! the command list: a user of packages never types them.
//!
//! `log append` reads entries as JSON lines on standard input, checks each against
//! [`crate::package::entry::Entry`]'s closed list before anything is written, appends their
//! canonical form, and signs the new checkpoint with the key file. The log directory's newest checkpoint is the file
//! `checkpoint` in it, C2SP's name, opened under the key's own verifier; a directory without one
//! is an empty log. The tiles are written before the checkpoint replaces the old one, so a run that
//! stops part way leaves the old checkpoint, and the next append from it writes the same tiles
//! again. Appends to one directory run one at a time: the service serializes them, and an append
//! that races another from the same size fails on the first tile the other wrote differently.
//! After its summary line it prints the path of each file it wrote, relative to the directory and
//! `checkpoint` last, which is the order the service copies them to its stores in.
//!
//! The key file is read under `rule:config/ownership-is-the-trust-boundary`'s check: it and its
//! directory must be writable by no account but this one, an administrator and the system.
//!
//! `log verify` reads the checkpoint with a pinned verifier key and checks the whole directory
//! from scratch ([`log::verify`]), every entry included.
//!
//! `index sign` signs a directory's root file, `registry.json`, and every index file under its
//! `index/` again ([`index::sign`]), each with the key file, the time and the size and root hash of
//! the log's checkpoint, `log/checkpoint` in the directory unless `--log` names another log. The
//! service runs it after each publish and once a day over every file, so no client refuses an
//! index file as older than seven days. Each file is replaced through a rename, so a run that stops
//! part way leaves every file signed, some of them by the run before.
//!
//! `build` is [`crate::package::build`]: one release's archive, built from a git commit.
//!
//! `key` makes a new key: it writes the signer key to a file that did not exist, readable and
//! writable by this account alone on Unix, and prints the verifier key that clients pin as the
//! `key` of the registry's `[registries]` line. The key's name is the origin of every checkpoint
//! the log signs, so a client opens a checkpoint with the key's name as its origin.

use std::fs;
use std::io::{self, BufRead as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nvs_config::trust::{self, Untrusted};

use crate::package::entry::Entry;
use crate::package::note::{Checkpoint, Signer, Verifier};
use crate::package::{index, log};

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
    /// Build a package's archive from a commit of its git repository.
    ///
    /// The command fetches the one commit over HTTPS, packs the package's
    /// folder and checks it. It prints one JSON object. `"ok": true` has
    /// the name, the version, the digest and every file. `"ok": false` has
    /// the reason in `refusal`, and the command exits with an error.
    Build(crate::package::build::Build),
    /// Make a new signing key for the registry's log.
    ///
    /// The command writes the private key to a new file and prints the
    /// public key. Clients write the public key as the `key` of the
    /// registry's line under `[registries]`.
    Key {
        /// The file to write the private key to. It must not exist yet.
        file: PathBuf,
        /// The key's name. It is also the log's name, the first line of every
        /// checkpoint, such as `registry.example.com`.
        #[arg(long)]
        name: String,
    },
    /// Work with the registry's log, the list of every change to the registry.
    Log {
        #[command(subcommand)]
        command: LogCommand,
    },
    /// Work with the registry's index files, which list every package's versions.
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },
}

#[derive(clap::Subcommand)]
enum IndexCommand {
    /// Sign `registry.json` and every index file again.
    ///
    /// The command signs `registry.json` and each file under `index/` in the
    /// directory. Each file gets the time and the log's newest checkpoint.
    /// Clients do not use an index file signed more than seven days ago to
    /// choose a version, so run this command at least once a day.
    Sign {
        /// The directory with `registry.json` and `index/`.
        dir: PathBuf,
        /// The file with the signing key, `PRIVATE+KEY+<name>+...`. Only your
        /// account may be able to change it and its folder.
        #[arg(long, value_name = "FILE")]
        key: PathBuf,
        /// The log's directory. The default is `log` in the directory.
        #[arg(long, value_name = "DIR")]
        log: Option<PathBuf>,
        /// The time to sign with, in Unix seconds. The default is now.
        #[arg(long, value_name = "SECONDS")]
        at: Option<u64>,
    },
}

#[derive(clap::Subcommand)]
enum LogCommand {
    /// Add entries to the log and sign a new checkpoint.
    ///
    /// The command reads one JSON entry per line from standard input. Each
    /// entry must have `"v": 1` and a `kind` the log knows. If one entry is
    /// wrong, the command adds nothing. The first line it prints says how
    /// many entries the log has. Each next line is a file it wrote, as a
    /// path in the log's directory. The last one is `checkpoint`.
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
        let done = match self.command {
            Command::Build(build) => return crate::package::build::run(&build),
            Command::Key { file, name } => key(&file, &name),
            Command::Log {
                command: LogCommand::Append { dir, key, origin },
            } => append(&dir, &key, &origin),
            Command::Log {
                command: LogCommand::Verify { dir, vkey, origin },
            } => verify(&dir, &vkey, &origin),
            Command::Index {
                command: IndexCommand::Sign { dir, key, log, at },
            } => sign(&dir, &key, log.as_deref(), at),
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

/// `nvs registry key`: the verifier key of a new key, whose signer key is written to `file`.
fn key(file: &Path, name: &str) -> Result<String, String> {
    let (signer, skey) = Signer::generate(name).map_err(|err| format!("--name: {err}"))?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let written = options
        .open(file)
        .and_then(|mut opened| io::Write::write_all(&mut opened, format!("{skey}\n").as_bytes()));
    written.map_err(|err| format!("{}: {err}", file.display()))?;
    Ok(signer.verifier().to_string())
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
    let written = log::written_paths(old.size(), size);
    let note = Checkpoint::new(origin, size, root)
        .and_then(|checkpoint| checkpoint.sign(&signer))
        .map_err(|err| err.to_string())?;
    let path = dir.join(CHECKPOINT);
    let partial = dir.join(format!("{CHECKPOINT}.partial"));
    fs::write(&partial, note)
        .and_then(|()| fs::rename(&partial, &path))
        .map_err(|err| format!("{}: {err}", path.display()))?;
    let mut message = format!(
        "Added {} entries. The log has {size} entries.",
        entries.len()
    );
    for path in written.iter().map(String::as_str).chain([CHECKPOINT]) {
        message.push('\n');
        message.push_str(path);
    }
    Ok(message)
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

/// `nvs registry index sign`.
fn sign(dir: &Path, key: &Path, log: Option<&Path>, at: Option<u64>) -> Result<String, String> {
    let signer = read_key(key)?;
    let verifier = signer.verifier();
    let log = log.map_or_else(|| dir.join("log"), Path::to_path_buf);
    let checkpoint = read_checkpoint(&log, verifier.name(), std::slice::from_ref(&verifier))?
        .ok_or_else(|| format!("{} has no `{CHECKPOINT}` file", log.display()))?;
    let at = at.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs())
    });
    let mut files = Vec::new();
    let root = dir.join(index::ROOT_FILE);
    if root.is_file() {
        files.push(root);
    }
    json_files(&dir.join("index"), &mut files)?;
    for file in &files {
        let failed = |err: String| format!("{}: {err}", file.display());
        let bytes = fs::read(file).map_err(|err| failed(err.to_string()))?;
        let note = index::sign(&bytes, &signer, &checkpoint, at).map_err(failed)?;
        let partial = file.with_extension("json.partial");
        fs::write(&partial, note)
            .and_then(|()| fs::rename(&partial, file))
            .map_err(|err| failed(err.to_string()))?;
    }
    Ok(format!(
        "Signed {} files with the log of {} entries.",
        files.len(),
        checkpoint.size()
    ))
}

/// Every `.json` file under `dir`, added to `files` in path order. A missing `dir` has none.
fn json_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("{}: {err}", dir.display())),
    };
    let mut paths = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("{}: {err}", dir.display()))?;
    paths.sort();
    for path in paths {
        if path.is_dir() {
            json_files(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "json") {
            files.push(path);
        }
    }
    Ok(())
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
