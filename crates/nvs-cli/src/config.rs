//! `nvs config` — `rule:config/check-and-dump-audit-the-tree-offline`'s offline audit of the configuration tree, and
//! the reader every caller in this binary resolves that tree through.
//!
//! § 9 puts `check` beside `dump` for one stated reason: § 3's
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
//! So the boundary is asserted where it can be answered — `nvs serve` and each
//! reload it runs, through `nvs_config::resolve::Disk`, on the host and as the
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
///   nobody at the keyboard wrote**. `nvs serve` and its reloads are that
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
///
/// The language server reads a document's tree through the same type, for the
/// same reason: the editor is the developer's own.
pub(crate) use nvs_config::resolve::Unowned as LocalFiles;

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

/// Whether reaching step 4 of `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
/// writes step 3's file before resolving.
///
/// The lookup itself is [`nvs_config::resolve::roots`]'s and does not change here. What a project
/// command adds is that arriving at step 4 creates the data folder's `nvs.toml` — the file step 3
/// looks for — and then reads it. **The working directory is never written to**: a file there is
/// only ever one somebody put there, or `nvs init` wrote on request. **Which commands write is one
/// table in [`crate::initializes`]**, so this type is what that table produces rather than a
/// decision taken here: a subcommand's arm hands it down, and every path that never resolves a tree
/// for a program to run on hands down [`Never`](Self::Never).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Init {
    /// Write the shipped default file into the data folder and resolve that.
    Write,
    /// Take step 4's shipped defaults and write nothing.
    Never,
}

/// The environment variable that disables the write, beside `--no-init`.
///
/// A container image built by running the binary once must not bake in a configuration file nobody
/// wrote, and the build step that runs it is usually not in a position to add a flag. It disables
/// the file only: the data folder and its subfolders are still created.
pub(crate) const NO_INIT: &str = "NOVIS_NO_INIT";

/// [`Init`] for this run: a project command writes unless something asked it not to.
///
/// The variable disables on **presence**, whatever it is set to — `NO_COLOR`'s convention, and the
/// fail-closed direction. A shell that exports it unconditionally then leaves the data folder
/// without a file, where a `NOVIS_NO_INIT=0` that wrote after all would be a surprise nobody can
/// see in the environment they set.
///
/// A named `--config` is not tested here: naming files makes step 1 the answer whether or not they
/// exist, so step 4 is never reached and there is nothing to gate.
pub(crate) fn init_gate(project_command: bool, no_init: bool, environment: Option<&OsStr>) -> Init {
    if project_command && !no_init && environment.is_none() {
        Init::Write
    } else {
        Init::Never
    }
}

/// Why a write of the shipped default file did not happen.
///
/// None of these is an error for a project command. The shipped defaults are a complete
/// configuration (`rule:config/no-configuration-file-is-a-complete-configuration`), so a folder that
/// would not take the file leaves the run on them — the artifact cache's discipline
/// ([`crate::cache`] § 4), for its reason: a run that refused to start because it could not write a
/// file nobody asked for would be a regression against every deployment that works without one.
/// What the reason buys is that the question can be *asked*: [`init`] renders it as the one line
/// [`Self::note_about`] gives, because there an operator asked for the file and a silent failure
/// would be the whole answer withheld. A run keeps it to itself until there is a boot line to carry
/// it (`rule:config/the-resolved-root-is-announced-and-stored`).
#[derive(Debug)]
pub(crate) enum Declined {
    /// The directory fails `rule:config/ownership-is-the-trust-boundary`, so a file created there
    /// would be one another local account can rewrite before `nvs serve` reads it back.
    Untrusted(nvs_config::trust::Untrusted),
    /// The directory already holds one, and an operator's own file outranks a template. Reaching
    /// this from a project command means a concurrent `nvs` won the race between step 3's look and
    /// this write, and nothing is wrong either way: the folder holds a tree, and it is the one the
    /// next run reads at step 3.
    Exists,
    /// The filesystem refused — a read-only folder, a full disk, or whatever else it answered
    /// with, in its own words.
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
    /// `--config` path and one about `./nvs.toml` read the same way.
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

/// The shipped default file written at `target` — the data folder's `nvs.toml` for a project
/// command, `./nvs.toml` or the one `--config` path for [`init`] — and the path it now holds, or the
/// [`Declined`] reason it does not.
///
/// `rule:config/ownership-is-the-trust-boundary` is asked first and about the **directory**, because
/// this is the one place in the binary that creates a file a later `nvs serve` will read as
/// configuration: writing into a directory another local account can write manufactures exactly the
/// surface that check exists to close, so refusing is the fail-closed direction and costs an
/// operator one explicit write. That directory has to exist: creating it here would create it with
/// whatever the directory above it lets every child inherit, which is the state the check refuses.
/// The data folder is created privately by `nvs_config::data::prepare` before any of this runs, and
/// `nvs_config::data::check` is the check, so the default data folder is examined without the
/// binary's directory above it.
fn write_default_at(target: &Path) -> Result<PathBuf, Declined> {
    let (Some(dir), Some(name)) = (target.parent(), target.file_name()) else {
        return Err(Declined::Unwritable(format!(
            "`{}` does not name a file",
            target.display()
        )));
    };
    let dir = nvs_config::data::check(dir).map_err(Declined::Untrusted)?;
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
/// The write is [`write_default_at`]'s, so this is the template a project command writes into the
/// data folder and the ownership check in front of it is the same check. **What differs is what a
/// refusal means.** Nobody asked for the implicit write, so declining it is silent and the run
/// carries on; this command exists only to produce the file, so a refusal is the answer to the
/// question that was asked — an `error:` line and a non-zero exit, which is what a script that runs
/// this can act on.
///
/// It is not a project command and is not in [`crate::initializes`]: it resolves no tree and runs
/// nothing, so there is no step 4 to reach.
///
/// **Where it writes is the project's own file**: with no `--config` that is step 2's `nvs.toml`
/// in this process's working directory, never the data folder. With one it is that path, resolved
/// against the working directory as step 1 resolves it, so `nvs init --config <path>` and
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
    boot_origins(config, Some(entry), sources, init).map(|(snapshot, _)| snapshot)
}

/// [`boot_snapshot`], keeping the map of **where each key was written**.
///
/// A snapshot is the effective tree and holds no spans, so a boot pass that has
/// to refuse a value — `nvs_config::server::waits_for`, `listen_on`, and
/// `nvs_config::mount::expand`, which resolves a relative `[server] root`
/// against the file that wrote it — needs this beside it. Callers that only
/// read values take [`boot_snapshot`] and never see it.
///
/// `entry` is `None` for `nvs serve` started with no file named, and the
/// snapshot is then [`nvs_config::Snapshot::host`]'s: the global tree with no
/// `[[app]]` block folded.
pub(crate) fn boot_origins(
    config: &[PathBuf],
    entry: Option<&Path>,
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
    let data = nvs_config::data::config_file();
    boot_in(&cwd, data.as_deref(), config, entry, sources, init)
}

/// `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`'s roots for `cwd`, with
/// this process's data folder as step 3 — what every command that reads a tree without booting a
/// program resolves, and what [`grants`] asks before a write can change the answer.
pub(crate) fn roots_in(config: &[PathBuf], cwd: &Path) -> nvs_config::Roots {
    let data = nvs_config::data::config_file();
    nvs_config::resolve::roots(config, cwd, data.as_deref(), &LocalFiles)
}

/// Builds the process's one outbound TLS client from `[http.client.tls]` and
/// installs it, so the first `https` call a program makes verifies against the
/// anchors this deployment chose (`rule:security/one-tls-client`).
///
/// **Every run site that starts a program calls this, and no other caller
/// does.** It is not folded into [`boot_in`], which resolves the tree for
/// `nvs check` and `nvs config dump` as well: those open no socket, and one of
/// them creating a key log file would be the audit writing secrets nobody
/// asked it for. `nvs_host::tls::configure` reports `AlreadyExists` to a second
/// call rather than replacing anything, and putting the call where a process is
/// owned is what keeps it a single one. A reload replaces the client through
/// `nvs_host::tls::install` instead, from `crate::reload`.
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
/// relaxed is assertable on its own, where installing changes the process's one
/// client. Which hosts a grant names is
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
/// Split from [`install_tls_client`] so the reading can be asserted on its own,
/// since installing changes the process's one client, and so a reload can build
/// the client its tree names before it publishes (`crate::reload`).
pub(crate) fn policy_of(snapshot: &nvs_config::Snapshot) -> nvs_host::tls::ClientPolicy {
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

/// [`boot_origins`] against a stated directory and data file rather than this process's own.
///
/// Both are parameters because § 1 reads them — step 2 looks for `cwd`'s `nvs.toml`, step 3 for
/// `data`, and step 4's write creates `data` — and a process has exactly one of each, which is a
/// fact about the process and not about the tree being resolved. `data` is the data folder's
/// `nvs.toml`, and `None` when there is no usable data folder.
fn boot_in(
    cwd: &Path,
    data: Option<&Path>,
    config: &[PathBuf],
    entry: Option<&Path>,
    sources: &mut SourceMap,
    init: Init,
) -> Result<
    (
        Arc<nvs_config::Snapshot>,
        std::collections::BTreeMap<String, nvs_config::resolve::Origin>,
    ),
    Diagnostic,
> {
    let resolved = resolved_in(cwd, data, config, sources, init)?;
    let snapshot = match entry {
        Some(entry) => nvs_config::Snapshot::build(&resolved, entry, &LocalFiles)?,
        None => nvs_config::Snapshot::host(&resolved, &LocalFiles)?,
    };
    Ok((snapshot, resolved.origins))
}

/// [`boot_set`]'s host snapshot, roster and the origin of every key.
pub(crate) type BootSet = (
    Arc<nvs_config::Snapshot>,
    nvs_config::AppBlocks,
    std::collections::BTreeMap<String, nvs_config::resolve::Origin>,
);

/// What `nvs serve` boots from: the host's snapshot, with no `[[app]]` block folded, and the
/// roster each served entry's own snapshot is folded from (ADR 0271).
///
/// # Errors
///
/// As [`boot_origins`].
pub(crate) fn boot_set(
    config: &[PathBuf],
    sources: &mut SourceMap,
    init: Init,
) -> Result<BootSet, Diagnostic> {
    let data = nvs_config::data::config_file();
    let resolved = resolved_in(
        &working_directory()?,
        data.as_deref(),
        config,
        sources,
        init,
    )?;
    let snapshot = nvs_config::Snapshot::host(&resolved, &LocalFiles)?;
    Ok((
        snapshot,
        nvs_config::AppBlocks::of(&resolved),
        resolved.origins,
    ))
}

/// The tree [`boot_in`] and [`boot_set`] build their snapshots from, in `cwd`, with `data` as the
/// data folder's `nvs.toml`.
fn resolved_in(
    cwd: &Path,
    data: Option<&Path>,
    config: &[PathBuf],
    sources: &mut SourceMap,
    init: Init,
) -> Result<nvs_config::resolve::Resolved, Diagnostic> {
    let files = LocalFiles;
    // `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`: every `--config` in
    // the order given, else `./nvs.toml`, else the data folder's `nvs.toml`, else the shipped
    // defaults. `roots` owns all four steps, so this call is the whole of the CLI's part in
    // choosing what is read.
    let mut roots = nvs_config::resolve::roots(config, cwd, data, &files);
    // Step 4, reached by a project command: write step 3's file into the data folder and then
    // resolve the file just written, so what the run reads is what the folder now holds and what
    // the next run will find at step 3. Nothing is ever written into `cwd`. A write that does not
    // happen — no usable data folder, or one that declines — changes nothing: `roots` still says
    // `Defaults` and the run takes them.
    //
    // The [`Declined`] reason is dropped here rather than printed, and that is where
    // `rule:config/the-resolved-root-is-announced-and-stored` lands once it ships: there is no boot
    // line for this to join yet. An unusable data folder has already been reported, once, by the
    // warning `nvs_config::data::prepare` gives the command.
    if init == Init::Write
        && matches!(roots, nvs_config::Roots::Defaults)
        && data.is_some_and(|data| write_default_at(data).is_ok())
    {
        roots = nvs_config::resolve::roots(config, cwd, data, &files);
    }
    let mut resolved = nvs_config::resolve::resolve(&roots, sources, &files)?;
    crate::bundle::carry_extensions(&mut resolved);
    Ok(resolved)
}

/// The `[capabilities]` block the machine that is **compiling** reads — `rule:core-classes/db-compile-time-query-checking`'s second sentence, which is what makes a literal `Core\Db::open` host
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
/// 4's write manufactures a tree, and a program measured against a file created by the command
/// measuring it is measured against nobody's decision. `nvs_config::default_file`'s own doc is the
/// claim that keeps — a file in which nothing is uncommented resolves to what a host with no file
/// at all resolves to, so writing one does not change the run that takes it.
/// A tree that *was* read and simply grants nothing is the other case — the
/// operator wrote a configuration and it says no — so it answers
/// `Some(Capabilities::default())`.
///
/// § 7's advisories are deliberately not printed here. They are an audit of the
/// tree, which is `nvs check`'s subject; this call's subject is the
/// program, and the tree is only being asked what it grants and loads.
///
/// The second half is the manifests of the tree's extension set
/// ([`crate::extensions::manifests`]), which the program's extension calls are
/// typed against.
///
/// # Errors
///
/// The exit code to return when the tree does not resolve or an extension's
/// manifest does not read. The diagnostic is rendered before it comes back,
/// against the source map this call owns.
pub(crate) fn grants(
    config: &[PathBuf],
    entry: &Path,
    init: Init,
) -> Result<
    (
        Option<nvs_config::tree::Capabilities>,
        Vec<nvs_ext::manifest::Manifest>,
    ),
    ExitCode,
> {
    let mut sources = SourceMap::new();
    // Asked before the call below, because that call may create the very file this is asking
    // about. A working directory that cannot be read answers "no tree" here and is reported as the
    // refusal it is a line later, by the resolve that hits the same failure.
    let found_a_tree = matches!(
        working_directory().map(|cwd| roots_in(config, &cwd)),
        Ok(nvs_config::Roots::Files(_))
    );
    let refuse = |diagnostic: Diagnostic, sources: &SourceMap| {
        let mut diags = Diagnostics::new();
        diags.report(diagnostic);
        render_diagnostics(&mut diags, sources);
        ExitCode::FAILURE
    };
    let (snapshot, origins) = boot_origins(config, Some(entry), &mut sources, init)
        .map_err(|diagnostic| refuse(diagnostic, &sources))?;
    let manifests = crate::extensions::manifests(&snapshot.config, &origins, &sources)
        .map_err(|diagnostic| refuse(diagnostic, &sources))?;
    Ok((
        found_a_tree.then(|| snapshot.config.capabilities.clone().unwrap_or_default()),
        manifests,
    ))
}

/// The manifests of the extension set the tree resolves to for `entry`, for a
/// compile that asks the tree nothing else — `nvs run`'s, which reads the
/// tree for itself once the program has compiled.
///
/// A tree that does not resolve is no set here, and the run's own read of the
/// tree reports it.
///
/// # Errors
///
/// The exit code to return when an extension's manifest does not read. The
/// diagnostic is rendered before it comes back.
pub(crate) fn extension_set(
    config: &[PathBuf],
    entry: &Path,
) -> Result<Vec<nvs_ext::manifest::Manifest>, ExitCode> {
    let mut sources = SourceMap::new();
    let Ok((snapshot, origins)) = boot_origins(config, Some(entry), &mut sources, Init::Never)
    else {
        return Ok(Vec::new());
    };
    crate::extensions::manifests(&snapshot.config, &origins, &sources).map_err(|diagnostic| {
        let mut diags = Diagnostics::new();
        diags.report(diagnostic);
        render_diagnostics(&mut diags, &sources);
        ExitCode::FAILURE
    })
}

/// The extension set of the tree `config` and `entry` resolve, loaded, as the host a compiled
/// call into one reaches (`crate::extensions::Calls`). A tree with no `[[extension]]` is an empty
/// set, which still reaches the built-in components. `None` for a tree that does not resolve,
/// which the boot that follows reports.
///
/// # Errors
///
/// The exit code after `E0652` is printed for the first entry that does not load.
pub(crate) fn extension_calls(
    config: &[PathBuf],
    entry: &Path,
) -> Result<Option<crate::extensions::Calls>, ExitCode> {
    let mut sources = SourceMap::new();
    let Ok((snapshot, origins)) = boot_origins(config, Some(entry), &mut sources, Init::Never)
    else {
        return Ok(None);
    };
    match crate::extensions::loaded(&snapshot.config, &origins, &sources) {
        Ok(set) => Ok(Some(crate::extensions::Calls::new(set))),
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &sources);
            Err(ExitCode::FAILURE)
        }
    }
}

/// A resolved tree folded for one entry: the snapshot, where each key was written, and the files
/// it was read from, which a refusal is rendered against.
pub(crate) struct Tree {
    pub(crate) snapshot: Arc<nvs_config::Snapshot>,
    pub(crate) origins: std::collections::BTreeMap<String, nvs_config::resolve::Origin>,
    pub(crate) sources: SourceMap,
}

/// [`boot_origins`] for `entry`, kept whole as a [`Tree`].
///
/// # Errors
///
/// The exit code after the tree's refusal is rendered.
pub(crate) fn boot_tree(config: &[PathBuf], entry: &Path, init: Init) -> Result<Tree, ExitCode> {
    let mut sources = SourceMap::new();
    match boot_origins(config, Some(entry), &mut sources, init) {
        Ok((snapshot, origins)) => Ok(Tree {
            snapshot,
            origins,
            sources,
        }),
        Err(diagnostic) => Err(rendered(diagnostic, &sources)),
    }
}

/// The tree `nvs ext test` runs a project's tests under, folded for `entry`: the files `config`
/// names and never `./nvs.toml`, else the shipped defaults, with the `.nvsx` at `file` in its
/// extension set under the pin `sha256`.
///
/// An entry of `config` that names `file` is kept with its grants. With none, an entry granting
/// nothing is added after the others, so a test run with no `--config` holds no I/O.
///
/// # Errors
///
/// The exit code after the tree's refusal is rendered, or after one line naming an entry of
/// `config` that pins `file` to another digest.
pub(crate) fn extension_test_tree(
    config: &[PathBuf],
    entry: &Path,
    file: &Path,
    sha256: &str,
) -> Result<Tree, ExitCode> {
    let mut sources = SourceMap::new();
    let roots = if config.is_empty() {
        nvs_config::Roots::Defaults
    } else {
        roots_in(
            config,
            &working_directory().map_err(|d| rendered(d, &sources))?,
        )
    };
    let mut resolved = nvs_config::resolve::resolve(&roots, &mut sources, &LocalFiles)
        .map_err(|diagnostic| rendered(diagnostic, &sources))?;
    let canonical = |path: &Path| std::fs::canonicalize(path).ok();
    let built = canonical(file);
    let mut listed = false;
    for (index, written) in resolved.config.extension.iter().enumerate() {
        let path = nvs_config::extension::file(
            index,
            written.path.as_deref().unwrap_or_default(),
            &resolved.origins,
        );
        if built.is_none() || canonical(&path) != built {
            continue;
        }
        let pinned = written.sha256.as_deref().unwrap_or_default();
        if !pinned.eq_ignore_ascii_case(sha256) {
            eprintln!(
                "error: {}: the configuration pins it to sha256 \"{pinned}\", and the built file is \"{sha256}\". Run `nvs ext pin` and copy its `sha256` into the configuration.",
                file.display()
            );
            return Err(ExitCode::FAILURE);
        }
        listed = true;
    }
    if !listed {
        // The snapshot is typed again from the table, so the entry is added there.
        let mut added = toml::Table::new();
        added.insert("path".into(), file.display().to_string().into());
        added.insert("sha256".into(), sha256.into());
        let entries = resolved
            .table
            .entry("extension")
            .or_insert_with(|| toml::Value::Array(Vec::new()));
        if let toml::Value::Array(entries) = entries {
            entries.push(toml::Value::Table(added));
        }
    }
    let snapshot = nvs_config::Snapshot::build(&resolved, entry, &LocalFiles)
        .map_err(|diagnostic| rendered(diagnostic, &sources))?;
    Ok(Tree {
        snapshot,
        origins: resolved.origins,
        sources,
    })
}

/// The exit code after `diagnostic` is rendered against `sources`.
fn rendered(diagnostic: Diagnostic, sources: &SourceMap) -> ExitCode {
    let mut diags = Diagnostics::new();
    diags.report(diagnostic);
    render_diagnostics(&mut diags, sources);
    ExitCode::FAILURE
}

/// `nvs config check [<file>...]` — resolve the tree and report what it holds,
/// exiting non-zero on any refusal.
///
/// The paths are `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 1's root list given positionally: naming one
/// disables step 2 exactly as `--config` does, so an audit of `/etc/nvs` on a
/// developer's machine never quietly merges the `./nvs.toml` beside the
/// checkout. With none named, § 1's own search runs — step 2's `./nvs.toml`,
/// else step 3's data-folder `nvs.toml`, else step 4's shipped defaults, which
/// resolve and are reported as a tree of no files rather than as a failure to
/// find one. Like every read-only command, it never creates the data folder.
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
        let roots = roots_in(&named, &cwd);
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
    // The verdict is about every block of the roster, not the blocks one entry file matches.
    nvs_config::app::record_roster(&resolved.config);

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
/// [`nvs_config::audit`]'s. What is decided *here* is only the stream:
/// `--toml` is one canonical file for diffing two environments, so it carries
/// the table and nothing else, while the listing carries the origin column an
/// audit is read for.
pub(crate) fn dump(config: &[PathBuf], paths: &[PathBuf], origin: bool, as_toml: bool) -> ExitCode {
    let files = LocalFiles;
    let mut sources = SourceMap::new();
    let named = named_roots(config, paths);
    let resolved = working_directory().and_then(|cwd| {
        let roots = roots_in(&named, &cwd);
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
    // The dump prints every block of the roster, not the blocks one entry file matches.
    nvs_config::app::record_roster(&resolved.config);

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

    use nvs_diagnostics::SourceMap;

    use super::{Declined, Init, boot_in, policy_of, relaxed_grants, write_default_at};
    use crate::testing::{open_to_the_world, refuse_new_files};

    /// A directory of this test's own, locked to this account, because
    /// [`super::write_default_at`] runs `rule:config/ownership-is-the-trust-boundary`'s check on
    /// the directory it writes into and on its parent.
    fn scratch(name: &str) -> nvs_repo::Scratch {
        nvs_repo::scratch_private(name)
    }

    /// A data folder inside `dir`, created the way the scratch directory was so it inherits the
    /// same private permissions, and the `nvs.toml` path in it that step 3 reads and step 4 writes.
    fn data_folder(dir: &std::path::Path) -> (PathBuf, PathBuf) {
        let data = dir.join("data");
        fs::create_dir(&data).expect("a scratch directory takes a folder");
        let file = data.join("nvs.toml");
        (data, file)
    }

    /// Whether a file a snapshot read is the one in [`data_folder`]'s folder.
    fn in_data(file: &std::path::Path) -> bool {
        file.parent().is_some_and(|parent| parent.ends_with("data"))
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
        let (snapshot, _) = boot_in(&dir, None, &[], Some(&entry), &mut sources, Init::Never)
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
        let (snapshot, _) = boot_in(&dir, None, &[], Some(&entry), &mut sources, Init::Never)
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

    /// `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` step 4 for a
    /// project command: the file is written into the data folder, where step 3 looks, and the same
    /// call then reads it back — so what the run holds is the tree on disk rather than the shipped
    /// defaults that were about to answer. The working directory is never written to.
    ///
    /// The `files` assertion is the whole of "and then resolves it": a snapshot built from step 4's
    /// defaults reaches no files at all (`rule:config/no-configuration-file-is-a-complete-configuration`),
    /// so a `files` list naming the written path is the only thing that distinguishes a run that
    /// wrote and read from a run that wrote and carried on regardless.
    #[test]
    fn a_run_with_no_config_writes_one_into_the_data_folder_and_then_resolves_it() {
        let dir = scratch("writes");
        let entry = entry(&dir);
        let (data, written) = data_folder(&dir);
        assert!(!written.exists(), "the data folder starts with no tree");

        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(
            &dir,
            Some(&written),
            &[],
            Some(&entry),
            &mut sources,
            Init::Write,
        )
        .expect("the file this call writes is a file it can read");

        assert!(
            written.exists(),
            "a project command with no tree writes one into the data folder"
        );
        assert!(
            !dir.join("nvs.toml").exists(),
            "and never into the working directory"
        );
        assert_eq!(
            snapshot.files.len(),
            1,
            "the run resolved the file it just wrote, not the shipped defaults"
        );
        assert!(
            snapshot.files[0].ends_with("nvs.toml") && in_data(&snapshot.files[0]),
            "the one file read is the one written into `{}`: {:?}",
            data.display(),
            snapshot.files[0]
        );
    }

    /// Step 3 on its own: a data folder that already holds `nvs.toml` is read when the working
    /// directory has none, and the working directory's file wins when it has one.
    #[test]
    fn the_data_folder_s_file_is_read_unless_the_working_directory_has_one() {
        let dir = scratch("data-read");
        let entry = entry(&dir);
        let (_, data_file) = data_folder(&dir);
        fs::write(&data_file, "[mode]\ndefault = \"development\"\n")
            .expect("a scratch directory takes a file");

        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(
            &dir,
            Some(&data_file),
            &[],
            Some(&entry),
            &mut sources,
            Init::Never,
        )
        .expect("the data folder's file resolves");
        assert_eq!(snapshot.files.len(), 1, "{:?}", snapshot.files);
        assert!(
            in_data(&snapshot.files[0]),
            "the data folder's file is the one read: {:?}",
            snapshot.files[0]
        );

        fs::write(dir.join("nvs.toml"), "").expect("a scratch directory takes a file");
        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(
            &dir,
            Some(&data_file),
            &[],
            Some(&entry),
            &mut sources,
            Init::Never,
        )
        .expect("the working directory's file resolves");
        assert_eq!(snapshot.files.len(), 1, "{:?}", snapshot.files);
        assert!(
            !in_data(&snapshot.files[0]),
            "the working directory's file wins: {:?}",
            snapshot.files[0]
        );
    }

    /// A process with no usable data folder writes nothing and runs on the shipped defaults: the
    /// working directory is not a fallback for the write.
    #[test]
    fn no_data_folder_writes_nothing_and_takes_the_shipped_defaults() {
        let dir = scratch("no-data");
        let entry = entry(&dir);

        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(&dir, None, &[], Some(&entry), &mut sources, Init::Write)
            .expect("no data folder is still a run");

        assert!(
            !dir.join("nvs.toml").exists(),
            "the working directory is not written to"
        );
        assert!(snapshot.files.is_empty(), "{:?}", snapshot.files);
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
        let (_, data_file) = data_folder(&dir);

        let mut sources = SourceMap::new();
        boot_in(
            &dir,
            Some(&data_file),
            &[],
            Some(&entry),
            &mut sources,
            Init::Write,
        )
        .expect("a tree it wrote itself");

        let written = fs::read(&data_file).expect("the write happened");
        assert_eq!(
            written,
            nvs_config::default_file().as_bytes(),
            "the file written is `nvs_config::default_file` and nothing else"
        );
    }

    /// A file already in the working directory is step 2's answer and is read, never replaced, and
    /// the data folder gets no file either: the write exists to create the file that is missing,
    /// and an operator's own `nvs.toml` being silently overwritten by a `nvs run` would be the
    /// worst version of this stage. The same holds for a file already in the data folder.
    #[test]
    fn an_existing_nvs_toml_is_never_touched() {
        let dir = scratch("existing");
        let entry = entry(&dir);
        let (_, data_file) = data_folder(&dir);
        let existing = dir.join("nvs.toml");
        let hand_written = "# an operator's own, and the only key it sets is none\n";
        fs::write(&existing, hand_written).expect("a scratch directory takes a file");

        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(
            &dir,
            Some(&data_file),
            &[],
            Some(&entry),
            &mut sources,
            Init::Write,
        )
        .expect("a comment-only file is a tree that resolves");

        assert_eq!(
            fs::read_to_string(&existing).expect("the file is still there"),
            hand_written,
            "the file in the directory is the one the operator wrote"
        );
        assert!(
            !data_file.exists(),
            "a tree was found, so the data folder gets no file"
        );

        fs::remove_file(&existing).expect("the scratch file goes");
        fs::write(&data_file, hand_written).expect("a scratch directory takes a file");
        let mut sources = SourceMap::new();
        boot_in(
            &dir,
            Some(&data_file),
            &[],
            Some(&entry),
            &mut sources,
            Init::Write,
        )
        .expect("a comment-only file is a tree that resolves");
        assert_eq!(
            fs::read_to_string(&data_file).expect("the file is still there"),
            hand_written,
            "the data folder's file is the one the operator wrote"
        );
        assert_eq!(
            snapshot.files.len(),
            1,
            "and it is the file the run read: {:?}",
            snapshot.files
        );
    }

    /// A data folder any local account can write gets no file. Creating one there is what
    /// `rule:config/ownership-is-the-trust-boundary` exists to stop — a `nvs serve` reads its
    /// capability grants back out of a file anybody on the machine can rewrite first — so
    /// [`super::write_default_at`] asks about the folder before it creates anything.
    ///
    /// The run still starts, on step 4's shipped defaults, which reach no files at all
    /// (`rule:config/no-configuration-file-is-a-complete-configuration`): a declined write is
    /// silent, so an empty `files` list is what separates this from the folder that was written
    /// to and read back.
    #[test]
    fn a_data_folder_that_fails_the_ownership_check_is_not_written_to() {
        let dir = scratch("untrusted");
        let entry = entry(&dir);
        let (data, data_file) = data_folder(&dir);
        open_to_the_world(&data);

        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(
            &dir,
            Some(&data_file),
            &[],
            Some(&entry),
            &mut sources,
            Init::Write,
        )
        .expect("a folder that may not be written into is still no reason to stop");

        assert!(
            !data_file.exists(),
            "the folder another account can write is the one folder never written into"
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
    fn a_read_only_data_folder_leaves_the_run_green_on_the_shipped_defaults() {
        let dir = scratch("read-only");
        let entry = entry(&dir);
        let (data, data_file) = data_folder(&dir);
        refuse_new_files(&data);

        let mut sources = SourceMap::new();
        let (snapshot, _) = boot_in(
            &dir,
            Some(&data_file),
            &[],
            Some(&entry),
            &mut sources,
            Init::Write,
        )
        .expect("a folder that will not take the file is still no reason to stop");

        assert!(
            !data_file.exists(),
            "the write could not happen, and nothing pretended otherwise"
        );
        assert!(
            !dir.join("nvs.toml").exists(),
            "and nothing fell back to the working directory"
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
        let breach = write_default_at(&untrusted.join("nvs.toml"))
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
        let refused = write_default_at(&refusing.join("nvs.toml"))
            .expect_err("a directory that takes no new file writes none");
        assert!(
            matches!(refused, Declined::Unwritable(_)),
            "the filesystem's own answer, not the ownership check's: {refused:?}"
        );

        let occupied = scratch("reason-exists");
        fs::write(occupied.join("nvs.toml"), "").expect("a scratch directory takes a file");
        let already =
            write_default_at(&occupied.join("nvs.toml")).expect_err("the file is already there");
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

    /// Step 1's precedent, carried to step 4: an operator who named files never gets a surprise
    /// write, **even when every named file is missing**. Naming one makes step 1 the answer
    /// whatever is on disk, so step 4 is not reached and there is nothing to write — the refusal
    /// the run gets is the missing file's, and both folders are as they were.
    #[test]
    fn a_config_flag_disables_the_write_even_when_every_named_file_is_missing() {
        let dir = scratch("named");
        let entry = entry(&dir);
        let (_, data_file) = data_folder(&dir);
        let absent = dir.join("absent.toml");

        let mut sources = SourceMap::new();
        boot_in(
            &dir,
            Some(&data_file),
            &[absent],
            Some(&entry),
            &mut sources,
            Init::Write,
        )
        .expect_err("a `--config` naming a file that does not exist is a hard refusal");

        assert!(
            !dir.join("nvs.toml").exists() && !data_file.exists(),
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
