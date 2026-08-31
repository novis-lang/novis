//! [ADR 0118] § 2's check, and § 5's refusal: the one function a door to the operating system calls
//! before it opens.
//!
//! [`require`] is deliberately the only *decision* here. The decision procedure is
//! [`nvs_config::capability`] and is pure; this is the half that knows about a request — where the
//! snapshot comes from, and what a denial looks like to the program that hit it. Eight below are
//! § 2's filesystem doors — [`open_read`], [`metadata`], [`exists`] and [`canonicalize`] behind
//! `fs.read`, [`write()`],
//! [`remove_file`], [`remove_dir`] and [`temp_dir`] behind `fs.write` — [`exec`] is the process
//! door behind `process.exec`, and `connect` arrives
//! with the first `Core` member that needs it; each of them calls [`require`] before it names a
//! spelling that
//! performs the effect, which is what makes § 2's claim structural rather than a convention: a member
//! reaches the OS through a door or not at all, and every door has already asked.
//!
//! **A context with no configuration grants nothing.** That is not a special case for tests — it is the
//! same deny-by-default the absent block gets, and a request path that reached a capability check
//! without a snapshot has a bug that should fail closed rather than quietly succeed.
//!
//! [ADR 0118]: ../../../docs/adr/0118-a-capability-is-checked-at-the-door-to-the-effect.md

use std::fs::{File, Metadata};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use nvs_config::capability::{Cap, Scope};

use crate::ctx::Ctx;
use crate::{Fault, ThrownClass};

/// § 1's question, asked of `ctx`'s own snapshot, and § 5's `RuntimeError` when the answer is no.
///
/// `member` is what the message names as the thing that wanted the capability — `Core\IO::write`,
/// or `spawn script` for the language construct, which is not a `Core` member at all and is checked
/// through the same function for exactly that reason.
///
/// # Errors
///
/// [`Fault::thrown`] — a `RuntimeError`, catchable, naming the capability in the spelling `nvs.toml`
/// grants it under and, for a scoped check, the argument that fell outside the grant. It is never a
/// `FATAL`: a denial is known before any work is done and leaves nothing behind, so a program that
/// degrades when a capability is missing is a reasonable program (ADR 0118 § 5).
pub fn require(ctx: &Ctx, cap: Cap, scope: Scope<'_>, member: &str) -> Result<(), Fault> {
    match refusal(ctx, cap, scope, member) {
        Some(message) => Err(Fault::thrown(message)),
        None => Ok(()),
    }
}

/// [`require`]'s answer as data: `None` when the capability covers `scope`, and § 5's message
/// otherwise.
///
/// For the one door that cannot hand back a [`Fault`] — [`crate::script::resolve`] owns an error
/// type of its own, because a spawn's two other ways of failing are not capability questions. It
/// asks this rather than re-deriving the sentence, so the message a denial prints has exactly one
/// author whichever door produced it.
pub(crate) fn refusal(ctx: &Ctx, cap: Cap, scope: Scope<'_>, member: &str) -> Option<String> {
    let granted = ctx.config().is_some_and(|config| {
        config
            .snapshot()
            .config
            .capabilities
            .as_ref()
            .is_some_and(|caps| caps.allows(cap, scope, &nvs_config::resolve::Disk))
    });
    if granted {
        return None;
    }
    Some(denial(cap, scope, member))
}

/// § 5's message. The capability's name comes first after the member because the reader is usually
/// the operator, and that string is what they are about to paste into a configuration file.
fn denial(cap: Cap, scope: Scope<'_>, member: &str) -> String {
    let name = cap.name();
    match scope {
        Scope::Unscoped => format!("{member} needs the capability `{name}`, which is not granted"),
        Scope::Path(path) => format!(
            "{member} needs the capability `{name}` for {}, which is not granted",
            path.display()
        ),
        Scope::Host(host) | Scope::Name(host) => {
            format!("{member} needs the capability `{name}` for {host}, which is not granted")
        }
    }
}

/// § 2's read door: the file at `path`, open for reading, once [`Cap::FsRead`] has been shown to
/// cover it.
///
/// The open handle rather than the bytes, because one door has to serve a whole-file read and an
/// incremental one alike, and the capability question belongs to the *handle*: a descriptor already
/// open is a descriptor already checked, so nothing downstream of this call has to ask again.
///
/// `member` is what a refusal names — see [`require`].
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when the open itself fails. The order is the point: a path
/// outside the grant is refused as a capability whether or not it exists, so a program cannot use
/// the difference between the two messages to probe a directory it was never allowed to read.
pub fn open_read(ctx: &Ctx, path: &Path, member: &str) -> Result<File, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    File::open(path).map_err(|err| io_failure(member, path, &err))
}

/// § 2's write door: `bytes` become the whole content of `path`, once [`Cap::FsWrite`] has been
/// shown to cover it.
///
/// The finished effect rather than an open handle, unlike [`open_read`], because a write *creates*
/// the path it names: `nvs_config::capability` § 4's argument side resolves the deepest existing
/// ancestor precisely so this check can happen before anything is created, and handing back a
/// handle would put the create on the caller's side of the door.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// `path`, or [`io_failure`]'s `IOError` when the write itself fails.
pub fn write(ctx: &Ctx, path: &Path, bytes: &[u8], member: &str) -> Result<(), Fault> {
    require(ctx, Cap::FsWrite, Scope::Path(path), member)?;
    std::fs::write(path, bytes).map_err(|err| io_failure(member, path, &err))
}

/// § 2's metadata door: what the operating system knows about `path`, once [`Cap::FsRead`] has been
/// shown to cover it.
///
/// Reading a file's size, kind or timestamps is reading the file, so this is the same capability
/// [`open_read`] asks for and not a weaker one: a program that can measure a path it was not granted
/// can enumerate a directory it was never allowed to open.
///
/// The whole [`Metadata`] rather than the one field a caller wants, because every question a
/// `Core\IO` metadata member asks is answered by one `stat` and a second door per field would be a
/// second syscall for the same permission.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when the `stat` itself fails — a path that is not there is
/// a failure here, because a member asking for a size has no answer for one.
pub fn metadata(ctx: &Ctx, path: &Path, member: &str) -> Result<Metadata, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    std::fs::metadata(path).map_err(|err| io_failure(member, path, &err))
}

/// § 2's existence door: whether anything is at `path`, once [`Cap::FsRead`] has been shown to cover
/// it.
///
/// Separate from [`metadata`] for the one reason that matters to a caller: **absence is an answer
/// here, not a failure.** Everything else about the two is the same, including which capability is
/// asked and that it is asked first — so a path outside the grant is refused whether or not it
/// exists, and the difference between a missing file and an unreadable one leaks nothing.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when the operating system could answer neither yes nor no —
/// a parent directory it will not traverse, for instance, which is not the same as "no".
pub fn exists(ctx: &Ctx, path: &Path, member: &str) -> Result<bool, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    path.try_exists()
        .map_err(|err| io_failure(member, path, &err))
}

/// § 2's resolution door: what `path` actually names, once [`Cap::FsRead`] has been shown to cover
/// it — every `..` collapsed by the operating system and every symlink followed.
///
/// Behind `fs.read` and not behind nothing, because resolving a name *reads* the directories above
/// it: a program that can canonicalize a path it was never granted can learn which of its
/// components exist, which is the enumeration [`exists`] is refused for.
///
/// The walk is [`nvs_config::capability::resolved`] and not a second one. It answers for a path that
/// does not exist yet by pinning its deepest existing ancestor, which is what a `Core\IO::within`
/// over a name about to be created needs — and, more importantly, it is the **same** resolution
/// [`require`] just compared against the grant, so a member cannot prove containment about a
/// different path than the one it was allowed to touch.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.read` for
/// `path`, or [`io_failure`]'s `IOError` when nothing about the path could be resolved — not even
/// an ancestor of it exists, or a still-unresolved component is `..`, which the walk refuses rather
/// than collapsing textually.
pub fn canonicalize(ctx: &Ctx, path: &Path, member: &str) -> Result<PathBuf, Fault> {
    require(ctx, Cap::FsRead, Scope::Path(path), member)?;
    nvs_config::capability::resolved(path, &nvs_config::resolve::Disk).ok_or_else(|| {
        io_failure(
            member,
            path,
            &std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no ancestor of this path could be resolved",
            ),
        )
    })
}

/// § 2's unlink door: `path` stops existing, once [`Cap::FsWrite`] has been shown to cover it.
///
/// Removal is a write and not a fifth capability, for the reason § 3 gives for not splitting one:
/// an account that may replace a file's whole content can already destroy it, so a separate grant
/// would name a distinction the filesystem does not make.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// `path`, or [`io_failure`]'s `IOError` when the unlink itself fails — the path is not there, or is
/// a directory, which [`remove_dir`] is the door for.
pub fn remove_file(ctx: &Ctx, path: &Path, member: &str) -> Result<(), Fault> {
    require(ctx, Cap::FsWrite, Scope::Path(path), member)?;
    std::fs::remove_file(path).map_err(|err| io_failure(member, path, &err))
}

/// § 2's rmdir door: the **empty** directory at `path` stops existing, once [`Cap::FsWrite`] has
/// been shown to cover it.
///
/// Empty deliberately, and this is the door's own decision rather than the caller's: a recursive
/// removal is one grant check standing in for a whole tree of them, so a single wrong argument
/// deletes everything under it. A program that means to empty a directory first walks it, and every
/// entry it removes is a path the capability was asked about.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for
/// `path`, or [`io_failure`]'s `IOError` when the removal itself fails — the directory is not there,
/// is not a directory, or still has entries in it.
pub fn remove_dir(ctx: &Ctx, path: &Path, member: &str) -> Result<(), Fault> {
    require(ctx, Cap::FsWrite, Scope::Path(path), member)?;
    std::fs::remove_dir(path).map_err(|err| io_failure(member, path, &err))
}

/// § 2's temporary-directory door: a new, empty, private directory under the system temporary root,
/// once [`Cap::FsWrite`] has been shown to cover **the path it is about to create**.
///
/// The check is the ordinary one and the argument is the ordinary argument, which is the whole
/// decision here: a member that creates a directory the program never named could plausibly have
/// been exempt from the grant, or have widened it to cover what it created, and both would make a
/// capability something a running program can enlarge. So the name is chosen first and asked about
/// second, exactly as [`write()`] asks about a path that does not exist yet, and an operator grants
/// the temporary root — or `true` — or the member does not run.
///
/// **Nothing here relies on the name being unpredictable.** The defence is that creating a directory
/// is atomic: a name an attacker has already taken, including as a symlink, fails with
/// `AlreadyExists` and is retried rather than adopted. On Unix the mode is `0o700` at creation
/// rather than after it, so there is no window in which the directory is readable by anyone else; on
/// Windows the per-user temporary root already carries that ACL and the directory inherits it.
///
/// The caller owns what it gets. Nothing here registers the directory for later cleanup — a program
/// removes what it made, and a runtime that swept temporary directories at request end would be
/// deciding the lifetime of data it knows nothing about.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `fs.write` for the
/// temporary root, or [`io_failure`]'s `IOError` when every attempt to create one failed.
pub fn temp_dir(ctx: &Ctx, member: &str) -> Result<PathBuf, Fault> {
    /// Enough attempts that exhausting them means something other than a collision — a full disk, a
    /// root that is not writable, a temporary directory someone has filled with our names.
    const ATTEMPTS: u32 = 16;

    let root = std::env::temp_dir();
    for _ in 0..ATTEMPTS {
        let path = root.join(format!("nvs-{}-{:016x}", std::process::id(), nonce()));
        require(ctx, Cap::FsWrite, Scope::Path(&path), member)?;
        match create_private_dir(&path) {
            Ok(()) => return Ok(path),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(io_failure(member, &path, &err)),
        }
    }
    Err(io_failure(
        member,
        &root,
        &std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("no unused name after {ATTEMPTS} attempts"),
        ),
    ))
}

/// A value unlikely to repeat within a process or between two of them, for [`temp_dir`]'s candidate
/// name. Not a secret and not required to be one — see that function's own paragraph on why.
fn nonce() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let ticks = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| u64::from(since.subsec_nanos()));
    let counted = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // An odd multiplier so consecutive counter values do not produce consecutive names, which is
    // what would let one process's directories be guessed from another's.
    ticks.wrapping_add(counted.wrapping_mul(0x9e37_79b9_7f4a_7c15))
}

/// [`temp_dir`]'s one create, with the mode applied by the create itself rather than after it.
fn create_private_dir(path: &Path) -> std::io::Result<()> {
    private_builder().create(path)
}

/// A builder that creates owner-only directories: `0o700` from the moment the directory exists, so
/// there is no window in which anyone else on the machine can read it.
#[cfg(unix)]
fn private_builder() -> std::fs::DirBuilder {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700);
    builder
}

/// The same builder where the mode is not a concept: Windows has no `mode` bits to set, and the
/// per-user temporary root already carries the ACL a new directory under it inherits.
#[cfg(not(unix))]
fn private_builder() -> std::fs::DirBuilder {
    std::fs::DirBuilder::new()
}

/// § 2's process door: `program` started as a child with `argv`, once [`Cap::ProcessExec`] has been
/// shown to cover it and [ADR 0044] § 4's shell targets have been refused.
///
/// The started child rather than its output, for [`open_read`]'s reason: one door has to serve
/// `Core\Process::run`'s captured wait and `::spawn`'s streamed handle alike, and the capability
/// question belongs to the *child* — a process already started is a process already checked, so
/// nothing downstream of this call has to ask again.
///
/// `argv` is what the program receives after its own name, which the operating system supplies:
/// there is no command line anywhere in this function, and so nothing for a quoting rule to be
/// wrong about. ADR 0044 § 1 is why that is the only shape offered.
///
/// **All three standard streams are pipes, and that is this door's decision rather than the
/// caller's.** A child that inherited them would read the server's own stdin and write to the
/// server's own stdout — a request reaching a descriptor no capability named, and one that no
/// `Core\Process` member would have to ask for.
///
/// **A shell target is refused here, on every platform**, [ADR 0044] § 4: a `.bat`, `.cmd` or `.ps1`
/// runs by handing a command line to `cmd.exe` or `powershell.exe`, which re-parses the arguments
/// this door never built, so an argv Novis passed correctly becomes a shell string again by the time
/// the target sees it. The check runs on Unix too, where the risk is not real — a `#!` line is read
/// by the same `execve` that already has the split argv — because a refusal that exists on one
/// platform only is a behaviour no test on the other can pin.
///
/// The capability is asked **first**, before the target's kind, so the rule every other door here
/// states holds without an exception: the grant is consulted before anything else is looked at.
/// Both refusals are the same catchable class, since neither is a condition a program can recover
/// from by trying something adjacent.
///
/// # Errors
///
/// [`require`]'s catchable `RuntimeError` when the configuration does not grant `process.exec` for
/// `program`, a `RuntimeError` naming the extension for § 4's refused target kinds, or
/// [`io_failure`]'s `IOError` when the spawn itself fails — nothing is at `program`, or it is not
/// executable.
///
/// [ADR 0044]: ../../../docs/adr/0044-core-process-argv-only-no-shell.md
pub fn exec(ctx: &Ctx, program: &Path, argv: &[&str], member: &str) -> Result<Child, Fault> {
    require(ctx, Cap::ProcessExec, Scope::Path(program), member)?;
    if let Some(extension) = shell_target(program) {
        return Err(Fault::thrown(format!(
            "{member} will not run {}: a `.{extension}` target is started by handing a command line \
             to a second parser, which re-quotes an argv this API passed across whole — start that \
             interpreter yourself if it is what you mean",
            program.display()
        )));
    }
    Command::new(program)
        .args(argv)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| io_failure(member, program, &err))
}

/// The lower-cased extension of a target [`exec`] refuses, or `None` for one it will start.
///
/// By extension and not by content, ADR 0044 § 4: what makes a `.bat` unsafe to start is which
/// program the operating system hands the command line to, and that is decided by the name alone —
/// so this answers the same way for a file that is not there, which is what lets the refusal be
/// about the kind of target rather than about the filesystem.
fn shell_target(program: &Path) -> Option<String> {
    let extension = program.extension()?.to_str()?.to_ascii_lowercase();
    matches!(extension.as_str(), "bat" | "cmd" | "ps1").then_some(extension)
}

/// What a door reports when the operating system refuses something the capability allowed: an
/// `IOError`, catchable, naming the member, the path and what the OS said.
///
/// Public because [`open_read`] hands back a handle rather than a result, so the caller that reads
/// from it owes the same message for the same kind of failure; one function is how the two agree
/// rather than drifting into two spellings of "could not read".
#[must_use]
pub fn io_failure(member: &str, path: &Path, err: &std::io::Error) -> Fault {
    Fault::thrown_as(
        ThrownClass::Io,
        format!("{member} failed on {}: {err}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Cap, Ctx, Fault, Path, Scope, ThrownClass, exec, require, shell_target};

    /// The member a case refuses on behalf of. `run` and not `spawn` for no reason beyond being the
    /// one the fixture calls; the door does not know which it is serving.
    const MEMBER: &str = "Core\\Process::run";

    /// A snapshot built from the text an operator would have written, rather than from the typed
    /// tree — the boot path deserializes, so a case that constructed the struct directly would pin
    /// a grant no configuration file can express.
    fn snapshot_of(written: &str) -> Arc<nvs_config::Snapshot> {
        let table: toml::Table = written.parse().expect("the case writes valid TOML");
        Arc::new(nvs_config::Snapshot {
            config: table
                .clone()
                .try_into()
                .expect("the case writes a block this tree has"),
            table,
            ..nvs_config::Snapshot::default()
        })
    }

    /// ADR 0118 § 5, asked of the process door: an unconfigured context starts nothing, and the
    /// message names the capability in the spelling `nvs.toml` grants it under.
    #[test]
    fn a_child_starts_only_where_process_exec_is_granted() {
        let ctx = Ctx::buffered();
        let denied = exec(&ctx, Path::new("/usr/bin/convert"), &["-version"], MEMBER)
            .expect_err("a context with no configuration grants nothing");
        let Fault::Thrown(class, message) = denied else {
            panic!("a denial is a throw and never a fatal — ADR 0118 § 5");
        };
        assert_eq!(class, ThrownClass::Runtime);
        assert!(
            message.contains("process.exec")
                && message.contains(MEMBER)
                && message.contains("convert"),
            "the denial names the capability, who wanted it and what for: {message}"
        );
    }

    /// ADR 0044 § 4, on a context that grants everything: the refusal is about the kind of target,
    /// so a grant cannot buy it and no platform is exempt from it.
    #[test]
    fn a_shell_target_is_refused_however_wide_the_grant_is() {
        let mut ctx = Ctx::buffered();
        ctx.set_config(snapshot_of("[capabilities.process]\nexec = true\n"));
        // The positive control: without it every refusal below could be the capability denial in
        // disguise, which is the same class and would satisfy a weaker assertion.
        require(
            &ctx,
            Cap::ProcessExec,
            Scope::Path(Path::new("examples/process/say.bat")),
            MEMBER,
        )
        .expect("`exec = true` covers every program, this one included");

        for (target, named) in [
            ("examples/process/say.bat", "bat"),
            ("C:/deploy/RELEASE.CMD", "cmd"),
            ("./build.ps1", "ps1"),
        ] {
            let refused = exec(&ctx, Path::new(target), &[], MEMBER)
                .expect_err("a second command-line parser is not a target this API has");
            let Fault::Thrown(class, message) = refused else {
                panic!("§ 4's refusal is catchable, like every other one this module writes");
            };
            assert_eq!(class, ThrownClass::Runtime);
            assert!(
                message.contains(&format!(".{named}")) && message.contains(MEMBER),
                "the refusal names the extension and the member: {message}"
            );
            assert!(
                !message.contains("process.exec"),
                "and is not the capability denial, which this context does not produce: {message}"
            );
        }
    }

    /// The other half of the same rule, which the cases above cannot show: every other target kind
    /// reaches the spawn, including the shebang script Unix runs through `execve` itself.
    #[test]
    fn nothing_but_those_three_extensions_is_a_shell_target() {
        for allowed in [
            "/usr/bin/convert",
            "target/debug/nvs.exe",
            "tools/deploy.sh",
            "batch",
            "archive.bat.gz",
        ] {
            assert!(
                shell_target(Path::new(allowed)).is_none(),
                "`{allowed}` is started by the operating system, not by a command-line parser"
            );
        }
    }
}
