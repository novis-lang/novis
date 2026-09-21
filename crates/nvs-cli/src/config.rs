//! `nvs config` — `rule:config/check-and-dump-audit-the-tree-offline`'s offline audit of the configuration tree, and
//! the reader every caller in this binary resolves that tree through.
//!
//! § 9 puts `check` beside `dump` and `ctl config` for one stated reason: § 3's
//! later-wins precedence "is only safe while it is auditable", so the reporting
//! is part of that decision rather than tooling around it. [`check`] answers
//! whether a tree resolves at all and what it holds in summary; [`dump`] answers
//! what every key resolved to and where it was written, which is the obligation
//! § 3 attaches to letting an include override the file that pulled it in. Both
//! are offline and need no server, which is what lets a tree be validated in CI
//! before it is deployed.
//!
//! ## Why the audit does not apply § 6's ownership check
//!
//! [`LocalFiles`] reads without that check, and `nvs config check` is a second
//! caller that wants it that way — for a reason of its own rather than by
//! inheriting `nvs run`'s. § 6 asks whether a file is owned by **the account the
//! runtime runs as** and unwritable by anyone else. A machine auditing a tree
//! before deployment is not that account and usually not that host, so the check
//! run there answers a different question than the one it exists for: it refuses
//! trees the server would accept and passes trees the server will refuse. A
//! green `nvs config check` that means neither is worse than one that does not
//! claim to have looked.
//!
//! So the boundary is asserted where it can be answered — `nvs serve` and
//! `nvs ctl reload`, through `nvs_config::resolve::Disk`, on the host and as the
//! account that will serve — and `check` reports what is decidable offline:
//! syntax, unknown keys, a duplicate key within one file, an include cycle, a
//! missing include, and the precedence the tree flattens to.
//!

use std::ffi::OsStr;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceMap};

use crate::render_diagnostics;

/// The reader this binary resolves the configuration tree through: the real
/// filesystem, with `rule:config/ownership-is-the-trust-boundary`'s **ownership check not applied**.
///
/// `nvs_config::resolve::Files` exists for exactly this split — its own doc
/// says § 6's check belongs on the reader so that "a caller that has one and a
/// caller that does not are two implementations of one interface rather than a
/// flag threaded through the resolver". This is the caller that does not, and
/// that is the split itself rather than a weakening of the boundary:
///
/// - § 6 defends a runtime that grants **configured capabilities to requests
///   nobody at the keyboard wrote**. `nvs serve` and `nvs ctl reload` are that
///   runtime and they use `Disk`, which checks.
/// - A `nvs run` has no such boundary to defend. The program is named on argv
///   and executed as the invoking account, and the configuration is `./nvs.toml`
///   in a working directory that same person chose. Whoever can write that file
///   is in a position to be writing the program too.
/// - And the check would refuse nearly every Windows checkout. § 6 names the
///   reason itself: Windows grants `Authenticated Users` modify rights by
///   default on a non-system drive's root and on everything inheriting from it,
///   so a repository on `D:` fails the check until an operator breaks that
///   inheritance. That is the right price for a served host and the wrong one
///   for `nvs run examples/hello.nvs`.
///
/// [`check`] reads through it for a reason of its own, which is this module's
/// own doc comment.
///
/// **This is not the whole answer, and the rest is Stage 4's.** Where a
/// capability check sits so that no member can route around it is the one ADR
/// slot this milestone reserved, and whether a CLI run may be granted anything
/// out of an unchecked file belongs in it. Until then nothing here grants
/// anything: `Core\Config` reads values and `[capabilities]` is enforced
/// nowhere, so the split above costs no right that is currently checked.
pub(crate) struct LocalFiles;

impl nvs_config::resolve::Files for LocalFiles {
    /// Canonicalization without the ownership check — see the type's own docs.
    /// The canonical path still comes from `nvs_config::trust::canonical`,
    /// because the resolver's cycle test compares files rather than spellings
    /// and a second canonicalizer is how a symlinked cycle gets through.
    fn trust(&self, path: &Path) -> Result<PathBuf, nvs_config::trust::Untrusted> {
        nvs_config::trust::canonical(path)
            .map_err(|err| nvs_config::trust::Untrusted::Unreadable(err.to_string()))
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        nvs_config::resolve::Disk.canonical(path)
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        nvs_config::resolve::Disk.read(path)
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        nvs_config::resolve::Disk.read_bytes(path)
    }

    fn exposure(&self, path: &Path) -> Option<String> {
        nvs_config::resolve::Disk.exposure(path)
    }

    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        nvs_config::resolve::Disk.list(dir)
    }

    fn exists(&self, path: &Path) -> bool {
        nvs_config::resolve::Disk.exists(path)
    }
}

/// The working directory, as the diagnostic a caller reports when it cannot be
/// read.
///
/// Both entry points below need it — `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in` resolves a `--config` against
/// it and § 1 step 2 looks for `./nvs.toml` in it — and neither can proceed
/// without it, so the failure is one shape rather than two.
pub(crate) fn working_directory() -> Result<PathBuf, Diagnostic> {
    std::env::current_dir().map_err(|err| {
        Diagnostic::error(
            nvs_diagnostics::code::E_UNREADABLE_CONFIG,
            format!("the working directory could not be read: {err}"),
        )
    })
}

/// Whether reaching step 3 of `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
/// writes step 2's file before resolving.
///
/// The lookup itself is unchanged — still `./nvs.toml` in the working directory, still never a walk
/// upward. What a project command adds is that arriving at step 3 creates the file step 2 looks for,
/// in the one directory it already looks in, and then reads it. **Which commands those are is one
/// table in [`crate::initializes`]**, so this type is what that table produces rather than a
/// decision taken here: a subcommand's arm hands it down, and every path that never resolves a tree
/// for a program to run on hands down [`Never`](Self::Never).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Init {
    /// Write the shipped default file into the working directory and resolve that.
    Write,
    /// Take step 3's shipped defaults and leave the directory as it was found.
    Never,
}

/// The environment variable that disables the write, beside `--no-init`.
///
/// A container image built by running the binary once must not bake in a configuration file nobody
/// wrote, and the build step that runs it is usually not in a position to add a flag.
pub(crate) const NO_INIT: &str = "NOVIS_NO_INIT";

/// [`Init`] for this run: a project command writes unless something asked it not to.
///
/// The variable disables on **presence**, whatever it is set to — `NO_COLOR`'s convention, and the
/// fail-closed direction. A shell that exports it unconditionally then leaves the directory holding
/// what it already held, where a `NOVIS_NO_INIT=0` that wrote after all would be a surprise nobody
/// can see in the environment they set.
///
/// A named `--config` is not tested here: naming files makes step 1 the answer whether or not they
/// exist, so step 3 is never reached and there is nothing to gate.
pub(crate) fn init_gate(project_command: bool, no_init: bool, environment: Option<&OsStr>) -> Init {
    if project_command && !no_init && environment.is_none() {
        Init::Write
    } else {
        Init::Never
    }
}

/// Why step 3's write did not happen.
///
/// None of these is an error. The shipped defaults are a complete configuration
/// (`rule:config/no-configuration-file-is-a-complete-configuration`), so a directory that would not
/// take the file leaves the run exactly where it stood before this step existed — the artifact
/// cache's discipline ([`crate::cache`] § 4), for its reason: a run that refused to start because it
/// could not write a file nobody asked for would be a regression against every deployment that works
/// today. What the reason buys is that the question can be *asked*: [`init`] renders it as the one
/// line [`Self::note_about`] gives, because there an operator asked for the file and a silent failure
/// would be the whole answer withheld. A run keeps it to itself until there is a boot line to carry
/// it (`rule:config/the-resolved-root-is-announced-and-stored`).
#[derive(Debug)]
pub(crate) enum Declined {
    /// The directory fails `rule:config/ownership-is-the-trust-boundary`, so a file created there
    /// would be one another local account can rewrite before `nvs serve` reads it back.
    Untrusted(nvs_config::trust::Untrusted),
    /// The directory already holds one, and an operator's own file outranks a template. Reaching
    /// this from a project command means a concurrent `nvs` won the race between step 2's look and
    /// this write, and nothing is wrong either way: the directory holds a tree, and it is the one
    /// the next run reads at step 2.
    Exists,
    /// The filesystem refused — a read-only working directory, a full disk, or whatever else it
    /// answered with, in its own words.
    Unwritable(String),
}

impl Declined {
    /// The reason as the one line a run prints before carrying on, which is all a declined write
    /// ever amounts to from outside the process.
    ///
    /// A breach carries the remedy and the explicit door with it, because that is the only one of
    /// these an operator is expected to act on: the others describe a machine's state rather than
    /// a decision anybody made. It also says that two directories were examined, because
    /// `nvs_config::trust::check` covers the directory the file goes into **and the one containing
    /// it**, and a breach naming the parent otherwise reads as the file having been aimed at the
    /// wrong place.
    ///
    /// A directory that could not be examined at all gets neither: the remedy is an answer to a
    /// DACL or a mode that was read, and what stopped this one is whatever the reader said.
    ///
    /// `target` is the file that was to be written, named in full so that a note about a
    /// `--config` path and one about the working directory's `nvs.toml` read the same way.
    pub(crate) fn note_about(&self, target: &Path) -> String {
        let file = target.display();
        match self {
            Self::Untrusted(nvs_config::trust::Untrusted::Unreadable(why)) => {
                format!("`{file}` was not written: its directory could not be examined: {why}")
            }
            Self::Untrusted(why) => format!(
                "`{file}` was not written: {}\n  \
                 note: the ownership check covers the directory the file goes into and the \
                 directory that contains it\n  \
                 help: {}\n  \
                 help: then run `nvs init` again",
                why.message(),
                nvs_config::trust::REMEDY,
            ),
            Self::Exists => {
                format!("`{file}` was not written: it already exists, and it is never overwritten")
            }
            Self::Unwritable(why) => format!("`{file}` was not written: {why}"),
        }
    }
}

/// Step 3's write: the shipped default file into `dir`, under the name step 2 looks for, and the
/// path it now holds — or the [`Declined`] reason it does not.
///
/// `rule:config/ownership-is-the-trust-boundary` is asked first and about the **directory**, because
/// this is the one place in the binary that creates a file a later `nvs serve` will read as
/// configuration: writing into a directory another local account can write manufactures exactly the
/// surface that check exists to close, so refusing is the fail-closed direction and costs an
/// operator one explicit write.
fn write_default_file(dir: &Path) -> Result<PathBuf, Declined> {
    write_default_at(&dir.join(nvs_config::resolve::LOCAL_FILE))
}

/// [`write_default_file`] at a stated path, which is what `nvs init --config <path>` names.
///
/// The ownership check is the same one and falls on the directory `target` will be created in. That
/// directory has to exist: creating it here would create it with whatever the directory above it
/// lets every child inherit, which is the state the check refuses.
fn write_default_at(target: &Path) -> Result<PathBuf, Declined> {
    let (Some(dir), Some(name)) = (target.parent(), target.file_name()) else {
        return Err(Declined::Unwritable(format!(
            "`{}` does not name a file",
            target.display()
        )));
    };
    let dir = nvs_config::trust::check(dir).map_err(Declined::Untrusted)?;
    let path = dir.join(name);
    // `create_new` is the whole of never overwriting: the file is created by this call or it is not,
    // with no window between asking whether one exists and writing it, so a second `nvs` in the same
    // directory loses the race rather than landing on top of the winner.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::AlreadyExists => Declined::Exists,
            _ => Declined::Unwritable(err.to_string()),
        })?;
    if let Err(err) = file.write_all(nvs_config::default_file().as_bytes()) {
        // A half-written template is a tree that refuses to parse, which would turn a failed write
        // into a refusal to start. What this run had a moment ago is no file, so that is what it
        // gets back, and the leftovers are this process's own.
        drop(std::fs::remove_file(&path));
        return Err(Declined::Unwritable(err.to_string()));
    }
    Ok(path)
}

/// `nvs init` — the same file, written because an operator asked for it.
///
/// The write is [`write_default_file`]'s, so this is the file a project command would have created
/// and the ownership check in front of it is the same check. **What differs is what a refusal
/// means.** Nobody asked for the implicit write, so declining it is a note and the run carries on;
/// this command exists only to produce the file, so a refusal is the answer to the question that was
/// asked — an `error:` line and a non-zero exit, which is what a script that runs this can act on.
///
/// It is not a project command and is not in [`crate::initializes`]: it resolves no tree and runs
/// nothing, so there is no step 3 to reach. It is instead where every refusal of the implicit write
/// points.
///
/// **Where it writes is `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
/// read backwards**: the file the next run will find. With no `--config` that is step 2's
/// `nvs.toml` in this process's own working directory. With one it is that path, resolved against
/// the working directory as step 1 resolves it, so `nvs init --config <path>` and
/// `nvs serve --config <path>` name the same file. More than one is refused: the flag is
/// repeatable because a tree may have several roots, and a template is one file.
pub(crate) fn init(config: &[PathBuf]) -> ExitCode {
    let cwd = match working_directory() {
        Ok(cwd) => cwd,
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &SourceMap::new());
            return ExitCode::FAILURE;
        }
    };
    let target = match config {
        [] => cwd.join(nvs_config::resolve::LOCAL_FILE),
        [named] => cwd.join(named),
        several => {
            eprintln!(
                "error: `nvs init` writes one file, and {} `--config` paths were given",
                several.len()
            );
            return ExitCode::FAILURE;
        }
    };
    match write_default_at(&target) {
        Ok(written) => {
            println!("wrote `{}`", written.display());
            ExitCode::SUCCESS
        }
        Err(declined) => {
            eprintln!("error: {}", declined.note_about(&target));
            ExitCode::FAILURE
        }
    }
}

/// The snapshot this run's request reads — `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`'s roots, § 3's ordered
/// stream, `rule:config/every-matching-app-block-applies-least-specific-first`'s `[[app]]` fold for `entry`, and `rule:config/the-config-is-an-immutable-snapshot`'s
/// immutable result.
///
/// `rule:routing/an-absolute-link-takes-a-configured-origin`'s origin arrives
/// through that `[[app]]` fold rather than out of any single block in the file,
/// so nothing in this binary scans `nvs.toml` for a key of its own.
///
/// `sources` is the caller's so that a refusal can be rendered with the line it
/// came from: a `nvs.toml` diagnostic carries a span into a file this map is
/// the only holder of.
pub(crate) fn boot_snapshot(
    config: &[PathBuf],
    entry: &Path,
    sources: &mut SourceMap,
    init: Init,
) -> Result<Arc<nvs_config::Snapshot>, Diagnostic> {
    boot_origins(config, entry, sources, init).map(|(snapshot, _)| snapshot)
}

/// [`boot_snapshot`], keeping the map of **where each key was written**.
///
/// A snapshot is the effective tree and holds no spans, so a boot pass that has
/// to refuse a value — `nvs_config::server::waits_for`, `listen_on`, and
/// `nvs_config::mount::expand`, which resolves a relative `[server] root`
/// against the file that wrote it — needs this beside it. Callers that only
/// read values take [`boot_snapshot`] and never see it.
pub(crate) fn boot_origins(
    config: &[PathBuf],
    entry: &Path,
    sources: &mut SourceMap,
    init: Init,
) -> Result<
    (
        Arc<nvs_config::Snapshot>,
        std::collections::BTreeMap<String, nvs_config::resolve::Origin>,
    ),
    Diagnostic,
> {
    let cwd = working_directory()?;
    boot_in(&cwd, config, entry, sources, init)
}

/// Builds the process's one outbound TLS client from `[http.client.tls]` and
/// installs it, so the first `https` call a program makes verifies against the
/// anchors this deployment chose (`rule:security/one-tls-client`).
///
/// **Every run site that starts a program calls this, and no other caller
/// does.** It is not folded into [`boot_in`], which resolves the tree for
/// `nvs check` and `nvs config dump` as well: those open no socket, and one of
/// them creating a key log file would be the audit writing secrets nobody
/// asked it for. `nvs_host::tls::configure` settles a `OnceLock`, so a second
/// call reports `AlreadyExists` rather than replacing anything, and putting
/// the call where a process is owned is what keeps it a single one.
///
/// The block has already been checked — an empty `roots` (`E0638`), a floor
/// this build cannot speak (`E0639`) and a `keylog` on a `production` host
/// (`E0640`) are all refused while the tree resolves, and every `roots` file
/// is absolute and trust-checked by then (`nvs_config::http::canonicalize`).
/// What is left for here is the files themselves.
///
/// It also reports what the tree relaxed: [`relaxed_grants`]'s line for every
/// `[capabilities.tls]` grant and host, every start, because a weakening nobody is
/// reminded of outlives the incident it was added for
/// (`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`). The lines go out
/// once the client is built, so a boot that refuses to start reports the refusal
/// alone.
///
/// # Errors
///
/// `E0641`, naming the file, when an anchor bundle will not open or holds no
/// certificate, or the key log will not open.
pub(crate) fn install_tls_client(snapshot: &nvs_config::Snapshot) -> Result<(), Diagnostic> {
    nvs_host::tls::configure(&policy_of(snapshot)).map_err(|err| {
        Diagnostic::error(
            nvs_diagnostics::code::E_TLS_CLIENT_UNBUILDABLE,
            format!("`[http.client.tls]` does not build an outbound TLS client: {err}"),
        )
    })?;
    for line in relaxed_grants(snapshot) {
        eprintln!("{line}");
    }
    Ok(())
}

/// One line per `[capabilities.tls]` grant and host, naming the grant in the
/// spelling an operator writes it in.
///
/// Split from [`install_tls_client`] for [`policy_of`]'s reason: what a tree
/// relaxed is assertable on its own, where installing settles a `OnceLock` and is
/// answerable once per process. Which hosts a grant names is
/// `nvs_config::tree::Capabilities::tls_relaxations`, so the boot's reading of a
/// grant is the same one the call that asks for it is measured against.
fn relaxed_grants(snapshot: &nvs_config::Snapshot) -> Vec<String> {
    let Some(caps) = snapshot.config.capabilities.as_ref() else {
        return Vec::new();
    };
    caps.tls_relaxations()
        .into_iter()
        .map(|(cap, host)| {
            format!(
                "note: [capabilities.tls] {} relaxes outbound TLS verification for {host}",
                cap.name()
            )
        })
        .collect()
}

/// `[http.client.tls]` as `nvs_host` asks for it.
///
/// Split from [`install_tls_client`] so the reading can be asserted on its own:
/// installing settles a `OnceLock` and is therefore answerable once per
/// process, which a test of what the block resolved to would otherwise have to
/// spend.
fn policy_of(snapshot: &nvs_config::Snapshot) -> nvs_host::tls::ClientPolicy {
    let block = snapshot
        .config
        .http
        .as_ref()
        .and_then(|http| http.client.as_ref())
        .and_then(|client| client.tls.as_ref());
    nvs_host::tls::ClientPolicy {
        roots: block.and_then(|tls| tls.roots.clone()).unwrap_or_default(),
        min_version: block.and_then(|tls| tls.min_version.clone()),
        keylog: block.and_then(|tls| tls.keylog.as_ref()).map(PathBuf::from),
    }
}

/// [`boot_origins`] against a stated directory rather than this process's own.
///
/// The directory is a parameter because both halves of § 1 read it — step 2 looks for its
/// `nvs.toml` and step 3's write creates one there — and a process has exactly one working
/// directory, which is a fact about the process and not about the tree being resolved.
fn boot_in(
    cwd: &Path,
    config: &[PathBuf],
    entry: &Path,
    sources: &mut SourceMap,
    init: Init,
) -> Result<
    (
        Arc<nvs_config::Snapshot>,
        std::collections::BTreeMap<String, nvs_config::resolve::Origin>,
    ),
    Diagnostic,
> {
    let files = LocalFiles;
    // `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`: every `--config` in the order given, else `./nvs.toml`,
    // else the shipped defaults. `roots` owns all three steps, so this call is
    // the whole of the CLI's part in choosing what is read.
    let mut roots = nvs_config::resolve::roots(config, cwd, &files);
    // Step 3, reached by a project command: write step 2's file and then resolve the file just
    // written, so what the run reads is what the directory now holds and what the next run will
    // find at step 2. A write that does not happen changes nothing — `roots` still says
    // `Defaults` and the run takes them.
    //
    // The [`Declined`] reason is dropped here rather than printed, and that is where
    // `rule:config/the-resolved-root-is-announced-and-stored` lands once it ships: there is no boot
    // line for this to join yet, and a run that printed one of its own would repeat it on every
    // invocation in a directory that fails the ownership check — which is every checkout under a
    // Windows drive root that grants `Authenticated Users` write. `nvs init` is where an operator
    // asks this question, and it is where the answer is reported.
    if init == Init::Write
        && matches!(roots, nvs_config::Roots::Defaults)
        && write_default_file(cwd).is_ok()
    {
        roots = nvs_config::resolve::roots(config, cwd, &files);
    }
    let resolved = nvs_config::resolve::resolve(&roots, sources, &files)?;
    let snapshot = nvs_config::Snapshot::build(&resolved, entry, &files)?;
    Ok((snapshot, resolved.origins))
}

/// The `[capabilities]` block the machine that is **compiling** reads — `rule:core-classes/db-literal-query-checking`'s second sentence, which is what makes a literal `Core\Db::open` host
/// matching no `db.open` grant a check-time diagnostic rather than only a
/// refusal at the door.
///
/// The tree is resolved exactly as [`boot_snapshot`] resolves it for a run: § 1's
/// roots in the same order, § 3's later-wins, and `rule:config/every-matching-app-block-applies-least-specific-first`'s `[[app]]` blocks
/// folded for this entry file — so the answer `nvs check` gives is the one this
/// deployment would give the same program. A tree that does not resolve is
/// reported here and stops the check, because a capability question answered
/// against half a tree is worse than one not asked.
///
/// **An empty tree answers `None`, and `None` is not an empty grant set.**
/// `nvs_types::check_program_granted`'s own doc owns that distinction: a program
/// checked outside any project root has no configuration to be measured against,
/// and refusing it would make deny-by-default mean "deny with nothing written".
/// **A file this very run wrote is that case and not the other one**, which is why the question
/// below is asked of the roots as they were *found* rather than of the snapshot's file list: step
/// 3's write manufactures a tree, and a program measured against a file created by the command
/// measuring it is measured against nobody's decision. `nvs_config::default_file`'s own doc is the
/// claim that keeps — a file in which nothing is uncommented resolves to what a host with no file
/// at all resolves to, so writing one does not change the run that takes it.
/// A tree that *was* read and simply grants nothing is the other case — the
/// operator wrote a configuration and it says no — so it answers
/// `Some(Capabilities::default())`.
///
/// § 7's advisories are deliberately not printed here. They are an audit of the
/// tree, which is `nvs config check`'s subject; this call's subject is the
/// program, and the tree is only being asked one question.
///
/// # Errors
///
/// The exit code to return when the tree does not resolve. The diagnostic is
/// rendered before it comes back, against the source map this call owns.
pub(crate) fn grants(
    config: &[PathBuf],
    entry: &Path,
    init: Init,
) -> Result<Option<nvs_config::tree::Capabilities>, ExitCode> {
    let mut sources = SourceMap::new();
    // Asked before the call below, because that call may create the very file this is asking
    // about. A working directory that cannot be read answers "no tree" here and is reported as the
    // refusal it is a line later, by the resolve that hits the same failure.
    let found_a_tree = matches!(
        working_directory().map(|cwd| nvs_config::resolve::roots(config, &cwd, &LocalFiles)),
        Ok(nvs_config::Roots::Files(_))
    );
    let snapshot = match boot_snapshot(config, entry, &mut sources, init) {
        Ok(snapshot) => snapshot,
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &sources);
            return Err(ExitCode::FAILURE);
        }
    };
    Ok(found_a_tree.then(|| snapshot.config.capabilities.clone().unwrap_or_default()))
}

/// `nvs config check [<file>...]` — resolve the tree and report what it holds,
/// exiting non-zero on any refusal.
///
/// The paths are `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 1's root list given positionally: naming one
/// disables step 2 exactly as `--config` does, so an audit of `/etc/nvs` on a
/// developer's machine never quietly merges the `./nvs.toml` beside the
/// checkout. With none named, § 1's own search runs — step 2's `./nvs.toml`,
/// else step 3's shipped defaults, which resolve and are reported as a tree of
/// no files rather than as a failure to find one.
///
/// It stops at the **first** refusal, because that is what `resolve` reports:
/// the tree is an ordered stream and a file that does not parse has no keys to
/// carry into the rest of it, so a second refusal found after the first would
/// be a guess about a tree that was never built.
///
/// The summary line is § 9's, and its counts are the ones that make § 3's
/// precedence auditable: how many files the tree reached, how many keys are in
/// force, how many of those overrode an earlier assignment, and how many
/// advisories (§ 7's readable secret file today) were raised. `dump --origin`
/// is where each override is named; this line is what CI reads.
pub(crate) fn check(config: &[PathBuf], paths: &[PathBuf]) -> ExitCode {
    let files = LocalFiles;
    let mut sources = SourceMap::new();
    let named = named_roots(config, paths);
    let resolved = working_directory().and_then(|cwd| {
        let roots = nvs_config::resolve::roots(&named, &cwd, &files);
        nvs_config::resolve::resolve(&roots, &mut sources, &files)
    });
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &sources);
            return ExitCode::FAILURE;
        }
    };

    // An advisory is printed in full and does not change the verdict — `rule:config/a-secret-is-a-file-whose-content-is-the-value`
    // says so, and `Resolved::warnings`' own doc says a refusal is never
    // here. The count on the summary line is so a green run still says one was
    // raised.
    if !resolved.warnings.is_empty() {
        let mut diags = Diagnostics::new();
        for warning in &resolved.warnings {
            diags.report(warning.clone());
        }
        render_diagnostics(&mut diags, &sources);
    }

    let set = nvs_config::audit::Audit::of_files(&resolved)
        .listing()
        .len();
    println!(
        "ok: {} file{}, {} directive{} set, {} override{}, {} warning{}",
        resolved.files.len(),
        plural(resolved.files.len()),
        set,
        plural(set),
        resolved.overrides.len(),
        plural(resolved.overrides.len()),
        resolved.warnings.len(),
        plural(resolved.warnings.len()),
    );
    ExitCode::SUCCESS
}

/// `nvs config dump [--origin] [--toml] [<file>...]` — every key in force, one
/// per line, in dotted-key order.
///
/// The roots are read exactly as [`check`] reads them. This is § 9's other
/// offline half and § 3's own condition: later-wins is acceptable *because*
/// every override is recoverable, and `--origin` is where it is recovered in
/// full rather than summarized.
///
/// What a row looks like and what each column means is
/// [`nvs_config::audit`]'s, because `nvs ctl config` renders the same listing
/// off the live snapshot and `rule:config/ctl-config-reports-the-live-snapshot`
/// has an operator diff the two. What is decided *here* is only the stream:
/// `--toml` is one canonical file for diffing two environments, so it carries
/// the table and nothing else, while the listing carries the origin column an
/// audit is read for.
pub(crate) fn dump(config: &[PathBuf], paths: &[PathBuf], origin: bool, as_toml: bool) -> ExitCode {
    let files = LocalFiles;
    let mut sources = SourceMap::new();
    let named = named_roots(config, paths);
    let resolved = working_directory().and_then(|cwd| {
        let roots = nvs_config::resolve::roots(&named, &cwd, &files);
        nvs_config::resolve::resolve(&roots, &mut sources, &files)
    });
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &sources);
            return ExitCode::FAILURE;
        }
    };

    // `--toml` is one canonical file for diffing two environments, so it is the
    // table serialized and nothing else: no origins, no alignment, no advisory
    // on standard output. Anything added to that stream is something the diff
    // reports as a change.
    if as_toml {
        return match toml::to_string(&resolved.table) {
            Ok(text) => {
                print!("{text}");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("error: the resolved configuration could not be written as TOML: {err}");
                ExitCode::FAILURE
            }
        };
    }

    print!(
        "{}",
        nvs_config::audit::Audit::of_files(&resolved).render(origin)
    );
    ExitCode::SUCCESS
}

/// The root list an audit reads: every `--config` in the order given, then
/// every file named positionally.
///
/// Two spellings for one of `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`'s steps, because the audit and the
/// runtime are asking about the same files from different directions. `--config`
/// is the runtime's, so `nvs config check --config /etc/nvs/nvs.toml` audits
/// exactly the argv the server will be given — § 9's own example is that
/// spelling. The positional list is the shorthand a person types at a prompt.
/// Naming both reads both, flag first, because that is the order a copied
/// server argv wants to keep.
pub(crate) fn named_roots(config: &[PathBuf], paths: &[PathBuf]) -> Vec<PathBuf> {
    config.iter().chain(paths).cloned().collect()
}

/// The `s` on a count, so the one-file case does not read as a template.
fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use nvs_diagnostics::SourceMap;

    use super::{Declined, Init, boot_in, policy_of, relaxed_grants, write_default_file};
    use crate::testing::{open_to_the_world, refuse_new_files};

    /// A directory of this test's own, under a per-process root.
    ///
    /// Nested rather than placed directly in the temp directory for
    /// `rule:config/ownership-is-the-trust-boundary`'s reason, which
    /// [`super::write_default_file`] asks about the directory it writes into: a Unix `/tmp` is mode
    /// `1777` and fails that check outright, while a root this process created carries the umask's
    /// ordinary bits and is the parent the check is meant to see.
    fn scratch(name: &str) -> PathBuf {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("nvs-init-{}", std::process::id()));
        let dir = root.join(format!("{unique}-{name}"));
        drop(fs::remove_dir_all(&dir));
        fs::create_dir_all(&dir).expect("a scratch directory under the temp dir is creatable");
        dir
    }

    /// A program for the snapshot to be folded for: `Snapshot::build` examines its entry, so a
    /// path that does not exist is `E0605` before anything this module is about is reached.
    fn entry(dir: &std::path::Path) -> PathBuf {
        let path = dir.join("app.nvs");
        fs::write(&path, "<?nvs\n").expect("a scratch directory takes a file");
        path
    }

    /// What the run sites hand `nvs_host::tls::configure`: the three keys of `[http.client.tls]`,
    /// off the booted snapshot rather than off the file.
    ///
    /// The `roots` assertion is the one worth making twice. `bundled` names no file and survives
    /// the boot verbatim, while a PEM entry is resolved against the file that wrote it — so a
    /// policy carrying the spelling an operator typed would be one that made `nvs serve` believe a
    /// bundle relative to whatever directory it was started from.
    #[test]
    fn the_tls_block_reaches_the_host_policy_with_its_roots_resolved() {
        let dir = scratch("tls");
        let entry = entry(&dir);
        let keylog = dir.join("keys.log");
        fs::write(dir.join("corp.pem"), "").expect("a scratch directory takes a file");
        fs::write(
            dir.join("nvs.toml"),
            format!(
                "[mode]\ndefault = \"development\"\n\n[http.client.tls]\n\
                 roots = [\"bundled\", \"corp.pem\"]\nmin_version = \"1.3\"\nkeylog = {}\n",
                toml::Value::from(keylog.to_string_lossy().into_owned())
            ),
        )
        .expect("a scratch directory takes a file");

        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(&dir, &[], &entry, &mut sources, Init::Never)
            .expect("the tree names a readable bundle and a mode that allows a key log");
        let policy = policy_of(&snapshot);

        let [bundled, corp] = policy.roots.as_slice() else {
            panic!(
                "both `roots` entries survive the boot, not {:?}",
                policy.roots
            );
        };
        assert_eq!(
            bundled,
            nvs_host::tls::BUNDLED,
            "the compiled-in set is named, not resolved"
        );
        assert!(
            std::path::Path::new(corp).is_absolute() && corp.ends_with("corp.pem"),
            "the PEM entry is resolved against the file that wrote it, not left as `{corp}`"
        );
        assert_eq!(policy.min_version.as_deref(), Some("1.3"));
        assert_eq!(policy.keylog.as_deref(), Some(keylog.as_path()));
    }

    /// The reminder the boot writes: one line per `[capabilities.tls]` grant and host, in the
    /// spelling the operator granted it under.
    ///
    /// The `insecure = true` entry is the case worth asserting from here rather than from
    /// `nvs-config`'s own tests. It is the spelling these grants do not have
    /// (`nvs_config::Cap::takes_true_spelling`) and it fails closed, so a boot printing a line for
    /// it would be telling an operator a weakening is in force that no call can actually reach.
    #[test]
    fn the_boot_reports_one_line_per_relaxed_tls_grant_and_host() {
        let dir = scratch("relaxed");
        let entry = entry(&dir);
        fs::write(
            dir.join("nvs.toml"),
            "[capabilities.tls]\nanchors = [\"corp.internal\"]\n\
             any_name = [\"a.example\", \"b.example\"]\ninsecure = true\n",
        )
        .expect("a scratch directory takes a file");

        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(&dir, &[], &entry, &mut sources, Init::Never)
            .expect("a tree that is one capability block resolves");

        assert_eq!(
            relaxed_grants(&snapshot),
            [
                "note: [capabilities.tls] tls.anchors relaxes outbound TLS verification for corp.internal",
                "note: [capabilities.tls] tls.any_name relaxes outbound TLS verification for a.example",
                "note: [capabilities.tls] tls.any_name relaxes outbound TLS verification for b.example",
            ],
            "one line per grant and host, in roster order, and `insecure = true` names no host"
        );
    }

    /// `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 3 for a
    /// project command: the file is written into the one directory step 2 looks in, and the same
    /// call then reads it back — so what the run holds is the tree on disk rather than the shipped
    /// defaults that were about to answer.
    ///
    /// The second assertion is the whole of "and then resolves it": a snapshot built from step 3's
    /// defaults reaches no files at all (`rule:config/no-configuration-file-is-a-complete-configuration`),
    /// so a `files` list naming the written path is the only thing that distinguishes a run that
    /// wrote and read from a run that wrote and carried on regardless.
    #[test]
    fn a_run_in_a_directory_with_no_config_writes_one_and_then_resolves_it() {
        let dir = scratch("writes");
        let entry = entry(&dir);
        let written = dir.join("nvs.toml");
        assert!(!written.exists(), "the directory starts with no tree");

        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(&dir, &[], &entry, &mut sources, Init::Write)
            .expect("the file this call writes is a file it can read");

        assert!(
            written.exists(),
            "a project command with no tree writes one"
        );
        assert_eq!(
            snapshot.files.len(),
            1,
            "the run resolved the file it just wrote, not the shipped defaults"
        );
        assert!(
            snapshot.files[0].ends_with("nvs.toml"),
            "the one file read is the one written: {:?}",
            snapshot.files[0]
        );
    }

    /// The goal's first standing decision, on disk: every key is commented out, so the file a
    /// project command leaves behind is the shipped template and not a rendering of today's
    /// values. A byte-for-byte comparison is what keeps it that way — anything that formats,
    /// filters or fills in the template fails here rather than in a deployment that finds its
    /// defaults frozen at the version that wrote them.
    #[test]
    fn the_written_file_is_the_shipped_template_byte_for_byte() {
        let dir = scratch("template");
        let entry = entry(&dir);

        let mut sources = SourceMap::new();
        boot_in(&dir, &[], &entry, &mut sources, Init::Write).expect("a tree it wrote itself");

        let written = fs::read(dir.join("nvs.toml")).expect("the write happened");
        assert_eq!(
            written,
            nvs_config::default_file().as_bytes(),
            "the file written is `nvs_config::default_file` and nothing else"
        );
    }

    /// A file already in the directory is step 2's answer and is read, never replaced: the write
    /// exists to create the file that is missing, and an operator's own `nvs.toml` being silently
    /// overwritten by a `nvs run` would be the worst version of this stage.
    #[test]
    fn an_existing_nvs_toml_is_never_touched() {
        let dir = scratch("existing");
        let entry = entry(&dir);
        let existing = dir.join("nvs.toml");
        let hand_written = "# an operator's own, and the only key it sets is none\n";
        fs::write(&existing, hand_written).expect("a scratch directory takes a file");

        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(&dir, &[], &entry, &mut sources, Init::Write)
            .expect("a comment-only file is a tree that resolves");

        assert_eq!(
            fs::read_to_string(&existing).expect("the file is still there"),
            hand_written,
            "the file in the directory is the one the operator wrote"
        );
        assert_eq!(
            snapshot.files.len(),
            1,
            "and it is the file the run read: {:?}",
            snapshot.files
        );
    }

    /// A working directory any local account can write gets no file. Creating one there is what
    /// `rule:config/ownership-is-the-trust-boundary` exists to stop — a `nvs serve` in that
    /// directory reads its capability grants back out of a file anybody on the machine can rewrite
    /// first — so [`super::write_default_file`] asks about the directory before it creates
    /// anything, and refusing costs an operator one explicit `nvs init` instead.
    ///
    /// The run still starts, on step 3's shipped defaults, which reach no files at all
    /// (`rule:config/no-configuration-file-is-a-complete-configuration`): a declined write is
    /// silent, so an empty `files` list is what separates this from the directory that was written
    /// to and read back.
    #[test]
    fn a_working_directory_that_fails_the_ownership_check_is_not_written_to() {
        let dir = scratch("untrusted");
        let entry = entry(&dir);
        open_to_the_world(&dir);

        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(&dir, &[], &entry, &mut sources, Init::Write)
            .expect("a directory that may not be written into is still one to run in");

        assert!(
            !dir.join("nvs.toml").exists(),
            "the directory another account can write is the one directory never written into"
        );
        assert!(
            snapshot.files.is_empty(),
            "and the run carries on with the shipped defaults: {:?}",
            snapshot.files
        );
    }

    /// A directory that will not take the file is not a failure to start. The goal's § 5: the run
    /// keeps the shipped defaults it was about to take anyway — no diagnostic, no non-zero exit,
    /// and nothing on stderr that a deployment which has always run read-only now has to expect.
    ///
    /// Ownership is untouched here on purpose, so what refuses is the filesystem rather than
    /// `rule:config/ownership-is-the-trust-boundary`'s check in front of it — the two are different
    /// reasons and the next case is the one that separates them.
    #[test]
    fn a_read_only_working_directory_leaves_the_run_green_on_the_shipped_defaults() {
        let dir = scratch("read-only");
        let entry = entry(&dir);
        refuse_new_files(&dir);

        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(&dir, &[], &entry, &mut sources, Init::Write)
            .expect("a directory that will not take the file is still one to run in");

        assert!(
            !dir.join("nvs.toml").exists(),
            "the write could not happen, and nothing pretended otherwise"
        );
        assert!(
            snapshot.files.is_empty(),
            "and the run has the shipped defaults, which is what it came in with: {:?}",
            snapshot.files
        );
    }

    /// The goal's § 7, as far as it can land: a declined write says **which** reason applied, so an
    /// operator who expected a file and has none is not left comparing an empty directory against
    /// the documentation. [`super::init`] is what reports it today; the boot line will carry it
    /// once `rule:config/the-resolved-root-is-announced-and-stored` ships.
    ///
    /// The three reasons are the three things that can answer this call, and only the breach is
    /// something anybody decided — so only its note carries the remedy and the explicit door.
    #[test]
    fn a_declined_write_says_which_reason_applied() {
        let untrusted = scratch("reason-untrusted");
        open_to_the_world(&untrusted);
        let breach = write_default_file(&untrusted)
            .expect_err("a directory any local account can write is refused");
        assert!(
            matches!(breach, Declined::Untrusted(_)),
            "the ownership check answered first: {breach:?}"
        );
        let breach_note = breach.note_about(&untrusted.join("nvs.toml"));
        assert!(
            breach_note.contains("nvs init"),
            "and it points at the one explicit door: {breach_note}"
        );
        assert!(
            breach_note.contains("the directory that contains it"),
            "and says the check reaches one directory further up than the file's own, so a breach \
             naming the parent is not read as the file having gone to the wrong place: \
             {breach_note}"
        );

        let refusing = scratch("reason-unwritable");
        refuse_new_files(&refusing);
        let refused = write_default_file(&refusing)
            .expect_err("a directory that takes no new file writes none");
        assert!(
            matches!(refused, Declined::Unwritable(_)),
            "the filesystem's own answer, not the ownership check's: {refused:?}"
        );

        let occupied = scratch("reason-exists");
        fs::write(occupied.join("nvs.toml"), "").expect("a scratch directory takes a file");
        let already = write_default_file(&occupied).expect_err("the file is already there");
        assert!(
            matches!(already, Declined::Exists),
            "a file that is already there is its own reason: {already:?}"
        );

        for note in [
            breach_note,
            refused.note_about(&refusing.join("nvs.toml")),
            already.note_about(&occupied.join("nvs.toml")),
        ] {
            assert!(
                note.contains("nvs.toml"),
                "every note names the file that is missing: {note}"
            );
        }
    }

    /// Step 1's precedent, carried to step 3: an operator who named files never gets a surprise
    /// write in the working directory, **even when every named file is missing**. Naming one makes
    /// step 1 the answer whatever is on disk, so step 3 is not reached and there is nothing to
    /// write — the refusal the run gets is the missing file's, and the directory is as it was.
    #[test]
    fn a_config_flag_disables_the_write_even_when_every_named_file_is_missing() {
        let dir = scratch("named");
        let entry = entry(&dir);
        let absent = dir.join("absent.toml");

        let mut sources = SourceMap::new();
        boot_in(&dir, &[absent], &entry, &mut sources, Init::Write)
            .expect_err("a `--config` naming a file that does not exist is a hard refusal");

        assert!(
            !dir.join("nvs.toml").exists(),
            "naming a tree disables the write, missing or not"
        );
    }

    /// The two opt-outs, each on its own: `--no-init` for a person and `NOVIS_NO_INIT` for a build
    /// step that cannot add a flag. The variable is read for **presence**, so the last case is the
    /// one that matters — an empty value is still a request not to write.
    ///
    /// Against [`super::init_gate`] rather than the environment, because a process has one
    /// environment and tests share it: what is being pinned is which answers the two inputs
    /// produce, and that is a question with no process state in it.
    #[test]
    fn no_init_and_the_environment_variable_each_disable_the_write() {
        use std::ffi::OsStr;

        use super::init_gate;

        assert_eq!(init_gate(true, false, None), Init::Write);
        assert_eq!(init_gate(true, true, None), Init::Never);
        assert_eq!(init_gate(true, false, Some(OsStr::new("1"))), Init::Never);
        assert_eq!(init_gate(true, false, Some(OsStr::new(""))), Init::Never);
        assert_eq!(
            init_gate(false, false, None),
            Init::Never,
            "and a command that is not a project command never writes"
        );
    }
}
