//! `rule:core-classes/temporary-dir-sweep`'s end-of-script sweep: the directories `Core\IO::temporaryDir` handed
//! this script, deleted when the script ends.
//!
//! The program is never asked to clean up and never asked to remember. What it
//! was handed, [`Ctx::track_temporary_dir`] wrote down; what is written down,
//! this module deletes.
//!
//! # It runs from [`Drop`], which is the whole of "after the last user code"
//!
//! § 3 puts the sweep after `rule:observability/script-on-exit`'s
//! `onExit` queue on a CLI ending and after
//! `rule:concurrency/after-response-outlives-the-connection`'s
//! `afterResponse` work on a request, and has it cover every ending the process
//! survives — normal, `exit`, an uncaught throw, and a request that died
//! mid-flight. A context's teardown is every one of those at once and is
//! *structurally* last: both queues run against a live [`Ctx`], so anything that
//! ran them has already returned by the time this does. So [`at_script_end`] is
//! called from `Ctx::drop` and from nowhere else, and there is no ending a
//! caller can forget to wire up. A `FATAL` on the CLI kills the process without
//! unwinding and leaves § 4's orphan case, which is the one this module cannot
//! reach.
//!
//! # It never throws, and there is nothing for it to throw *into*
//!
//! § 3's "never throws and never alters a response" is not a rule this module
//! remembers — by the time it runs there is no `Fault` to raise, no frame to
//! raise it on and no response left to change. What is left is the log, so a
//! refusal is one line naming the path and the error ([`at_script_end`]) and the
//! directory waits in the owned root for the next sweep to try again. No retry
//! loop and no accumulated error state: § 4's `nvs tmp clean` and the `nvs serve`
//! boot are the next attempts.
//!
//! # The one thing that stops it, and it is the operator's
//!
//! § 5's `[debug] keep_temporary` turns every deletion into a log line naming
//! the path that was kept ([`keep_temporary`], [`kept`]). The list is still
//! drained, so the keeping is per script rather than a queue that grows: what
//! is kept is on disk under the owned root, where § 4's `nvs tmp clean` is how
//! a debugging session's keepings are cleared. There is no in-language setter
//! and no per-call persist — a program able to exempt its own files from
//! cleanup is a program that can be made to hoard them, which
//! `rule:concurrency/cross-request-state-is-explicit` already
//! answers with storage.
//!
//! # § 4's orphan sweep decides here and acts elsewhere
//!
//! A script killed outright ran no [`Drop`] and so ran nothing above. What it
//! left behind is § 4's, and that sweep runs from the `nvs serve` boot and from
//! `nvs tmp clean`, neither of which is in this crate. What *is* here is
//! everything they share: [`owner_is_alive`], the predicate over one path, and
//! [`orphans`], the walk over one root that applies it. The doors differ only
//! in what they do with the list — the boot hands it to [`refusals`],
//! `tmp clean` prints it and, with `--dry-run`, does nothing else — so there is
//! no reading of "whose entry is this" for them to come to disagree about.
//!
//! [`refusals`] is the part with no logging in it, which is what makes the
//! behaviour testable without a sink to read back.

use std::path::PathBuf;

use nvs_render::Level;

use crate::ctx::Ctx;

/// Deletes every temporary directory this script was handed, and logs the ones
/// the operating system refused — `rule:core-classes/temporary-dir-sweep`, called from `Ctx::drop`.
///
/// Draining, so a second call has nothing to do: a request that died mid-flight
/// and then ended ordinarily is swept once
/// ([`Ctx::take_temporary_dirs`](Ctx::take_temporary_dirs)).
///
/// A refusal is `warn` and not `error`: nothing is lost, no request was
/// affected, and the directory is still inside the root Novis owns where the
/// next sweep will find it. On Windows a held handle — an indexer, a scanner —
/// is the routine reason and it is not a fault of the program's, which is why
/// this reads as a note rather than as a failure. The path is a field rather
/// than part of the message so that a log pipeline can count refusals per root
/// (`rule:errors/record-transformations`).
///
pub fn at_script_end(ctx: &mut Ctx) {
    let taken = ctx.take_temporary_dirs();
    if taken.is_empty() {
        // The ordinary script: no allocation was made for the list and no clock
        // is read for the sweep.
        return;
    }
    if keep_temporary(ctx) {
        for path in taken {
            let record = kept(&path);
            crate::floor::report(ctx, &record);
        }
        return;
    }
    for (path, error) in refusals(taken) {
        let record = note(&path, &error);
        crate::floor::report(ctx, &record);
    }
}

/// Whether `[debug] keep_temporary` is on — `rule:core-classes/temporary-dir-sweep`'s one escape hatch,
/// and the operator's alone.
///
/// Read off the snapshot, like `capability`'s `temp_root` beside it, because
/// § 5 gives the key no in-language setter: it is `System`-class, so no request
/// has a spelling by which it could have written one, and a context carrying no
/// configuration at all sweeps rather than keeps. `Reload` rather than `Boot`
/// (`nvs_config::directive`), which is what makes "flipped on around one
/// problematic request and off again" true — the next script to end reads the
/// snapshot the reload published.
///
fn keep_temporary(ctx: &Ctx) -> bool {
    ctx.config()
        .and_then(|config| config.snapshot().config.debug.as_ref())
        .and_then(|debug| debug.keep_temporary)
        .unwrap_or(false)
}

/// Builds the one record a *kept* directory writes — § 5's "always deliberate
/// and always visible, never a silent leak".
///
/// `info` and not `warn`, which is the whole difference from [`note`] beside
/// it: a refusal is something the runtime tried and could not do, while this is
/// something an operator asked for, and a debugging session that filled the log
/// with warnings would teach the operator to stop reading them. The path is a
/// field for [`note`]'s reason, and it is the only field: what the sweep would
/// have done is in the message and needs no second key to be counted by.
fn kept(path: &std::path::Path) -> nvs_render::Record {
    let mut record = crate::floor::note(
        Level::Info,
        "a temporary directory was kept rather than removed: `[debug] keep_temporary` is on",
    );
    record.envelope.fields.push((
        "path".to_owned(),
        crate::floor::text(&path.display().to_string()),
    ));
    record
}

/// Builds the one record a refused deletion writes, so that
/// [`at_script_end`]'s loop reads as what it does rather than as how a record
/// is shaped.
fn note(path: &std::path::Path, error: &std::io::Error) -> nvs_render::Record {
    let mut record = crate::floor::note(
        Level::Warn,
        "a temporary directory could not be removed and was left in the owned root",
    );
    record.envelope.fields.push((
        "path".to_owned(),
        crate::floor::text(&path.display().to_string()),
    ));
    record
        .envelope
        .fields
        .push(("error".to_owned(), crate::floor::text(&error.to_string())));
    record
}

/// Deletes each of `paths` and answers the ones that would not go, with the
/// error each refused with.
///
/// **A path that is already gone is not a refusal.** § 3 calls that the goal
/// state reached early: a program may remove its own temporary directory
/// whenever it likes, and a sweep that remarked on it would turn tidy programs
/// into log noise. `NotFound` is therefore success, and it is the only error
/// kind this reads — everything else is handed back exactly as the operating
/// system gave it.
///
/// **Recursive**, unlike the `Core\IO::removeDir` a program can call. That
/// member is non-recursive as a security decision, because it is pointed at a
/// path the program named; this is pointed only at a directory the runtime
/// itself created, under the root it owns, whose whole contract is that it goes
/// away with the script. A sweep that refused a non-empty directory would delete
/// nothing a real program ever made.
///
/// Every path is attempted even after one fails, so one held handle does not
/// strand the rest of a script's directories.
#[must_use]
pub fn refusals(paths: Vec<PathBuf>) -> Vec<(PathBuf, std::io::Error)> {
    let mut refused = Vec::new();
    for path in paths {
        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => refused.push((path, error)),
        }
    }
    refused
}

/// Whether the process that created the entry at `path` is still running —
/// `rule:core-classes/temporary-dir-orphan-sweep`'s predicate, and the only thing the orphan sweep decides on.
///
/// **A function over a path and nothing else**: no context, no configuration,
/// no directory walk and no clock. [`orphans`] is the walk that asks it of one
/// entry at a time, and § 4's doors reach it through that, so the rule they
/// share is one function rather than separate walks that agree today.
///
/// **Liveness, never age.** An age rule is precisely what deletes a
/// long-running process's files out from under it; this cannot, because a live
/// owner's entry is skipped however old it is. The pid is read out of the name
/// `capability::temp_dir` wrote (`nvs-<pid>-<16 hex>`) — that name is the only
/// record of who owns an entry, which is why it carries the pid at all.
///
/// **Every ambiguity answers `true`**, which is *skip*. A name this did not
/// write, a pid that will not parse, a `0`, a process another account owns, an
/// error the platform did not name: all of them read as alive, so the sweep
/// may under-delete and can never over-delete. The one failure that remains is
/// a recycled pid, which makes a dead owner's entry look alive and leaks it
/// until a later sweep — § 4 takes that direction deliberately, and the goal's
/// own standing decisions settle it rather than leaving it to a caller.
///
#[must_use]
pub fn owner_is_alive(path: &std::path::Path) -> bool {
    let Some(pid) = owning_pid(path) else {
        return true;
    };
    pid_is_alive(pid)
}

/// `rule:core-classes/temporary-dir-orphan-sweep`'s walk: the entries of the owned root whose owner is dead,
/// sorted, and nothing deleted.
///
/// **It answers rather than acts**, because that is the whole of what § 4's
/// doors have in common. The `nvs serve` boot hands this list to [`refusals`];
/// `nvs tmp clean` prints each path and hands over the same list, or, under
/// `--dry-run`, prints it and hands over nothing. A walk that deleted as it went
/// could not serve that dry run, and each door would grow its own predicate —
/// which is the one thing § 4 cannot afford, since doors that disagree mean
/// over-deleting.
///
/// **A root, not a context.** Each door knows its own root
/// ([`crate::capability::temp_root`]) and neither has a [`Ctx`] to be asked
/// through; taking the path keeps the root decided in one place and leaves this
/// as testable as [`owner_is_alive`] is.
///
/// **Every ambiguity skips, exactly as the predicate does.** A root that cannot
/// be listed answers "nothing to sweep" — a root nothing has created yet is the
/// ordinary state of a first boot, and one the platform refuses to read is an
/// ambiguity, which § 4 resolves toward under-deleting like every other. An
/// entry whose kind cannot be read is skipped. So is anything that is not a
/// directory: `capability::temp_dir` creates directories and only directories,
/// so a file or a symlink wearing the name is something this runtime did not
/// write — and following a link out of the owned root is the one move that
/// would turn the sweep into the deletion primitive an attacker wanted.
///
/// Sorted so that every door reporting the same root reports it in the same
/// order, rather than in whatever order the filesystem enumerated. The cost is
/// a sort over one directory, paid only where a door runs.
///
#[must_use]
pub fn orphans(root: &std::path::Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut dead: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .filter(|path| !owner_is_alive(path))
        .collect();
    dead.sort();
    dead
}

/// The pid out of an entry name `capability::temp_dir` wrote, or `None` for
/// anything else in the root.
///
/// The nonce is checked as well as the prefix, because the name is what stands
/// in for ownership: a directory an operator made called `nvs-1-notes` should
/// not be read as process 1's and swept when process 1 exits. `0` is refused
/// for a sharper reason — on Unix it is the *process group* to `kill`, and a
/// name carrying it must not become a question about anything but one process.
fn owning_pid(path: &std::path::Path) -> Option<u32> {
    let (pid, nonce) = path
        .file_name()?
        .to_str()?
        .strip_prefix("nvs-")?
        .split_once('-')?;
    if nonce.len() != 16 || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    pid.parse().ok().filter(|pid| *pid != 0)
}

/// Whether `pid` names a running process — `kill(pid, 0)`, which sends no
/// signal and asks only whether it could.
///
/// `ESRCH` is the one answer that means dead. `EPERM` is a process this account
/// may not signal, which is very much alive, and every other errno is an
/// ambiguity resolved toward skip per [`owner_is_alive`]. A zombie the parent
/// has not reaped answers alive here, which is the safe direction: its pid is
/// not reusable yet either.
#[cfg(unix)]
#[expect(
    unsafe_code,
    reason = "asking whether a pid exists has no `std` spelling, and signal 0 \
              is the ordinary way to ask it"
)]
fn pid_is_alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return true;
    };
    // SAFETY: `kill` takes two integers by value, writes nothing, and delivers
    // nothing at all for signal 0 — it sets `errno` and returns.
    if unsafe { libc::kill(pid, 0) } == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}

/// Whether `pid` names a running process — the smallest handle the question
/// needs, opened and closed.
///
/// `PROCESS_QUERY_LIMITED_INFORMATION` because a sweep asks whether a process
/// is there and nothing else; it is also the right that survives across
/// integrity levels, so an entry belonging to a service is answered rather than
/// refused. `ERROR_INVALID_PARAMETER` is the one answer that means dead — there
/// is no such pid. `ERROR_ACCESS_DENIED` is a process this account may not
/// open, which exists, and every other error is an ambiguity resolved toward
/// skip per [`owner_is_alive`]. A process that has exited while somebody still
/// holds a handle to it opens successfully and reads as alive, which is the
/// safe direction for the same reason a zombie is on Unix.
#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "asking whether a pid exists has no `std` spelling, and these two \
              calls are the platform's own way to ask it"
)]
fn pid_is_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    // SAFETY: three integers by value; the call writes nothing of ours and
    // answers a handle this frame owns or a null.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return std::io::Error::last_os_error().raw_os_error()
            != i32::try_from(ERROR_INVALID_PARAMETER).ok();
    }
    // SAFETY: the handle was just opened by this frame and is not used again.
    unsafe { CloseHandle(handle) };
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory this case alone is using, deleted when the guard drops.
    fn scratch(name: &str) -> nvs_repo::Scratch {
        nvs_repo::scratch(&format!("sweep-{name}"))
    }

    /// The lines a context's diagnostic sink has taken — where
    /// [`crate::floor::report`] leaves a record for a context that configured
    /// no `[log] target`, which is every context in this file.
    fn diagnostic(ctx: &mut Ctx) -> Vec<String> {
        let written = ctx
            .take_buffered_diagnostic()
            .expect("the case asked for a buffered diagnostic sink");
        String::from_utf8(written)
            .expect("a record renders as text")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// § 3's ordinary ending, through the only caller [`at_script_end`] has:
    /// the context's own teardown.
    ///
    /// The directory the program already removed is tracked **first** on
    /// purpose, and its sibling case reads the same list from the other side: a
    /// sweep that gave up at the first path it could not find would leave the
    /// standing one behind and still pass every assertion naming that one
    /// alone.
    #[test]
    fn a_temporary_dir_still_standing_at_script_end_is_removed() {
        let base = scratch("standing");
        let already_gone = base.join("already-gone");
        let standing = base.join("standing");
        std::fs::create_dir(&standing).expect("the base exists");
        std::fs::write(standing.join("note.txt"), b"what the program wrote")
            .expect("the directory was just created");

        let mut ctx = Ctx::buffered();
        ctx.track_temporary_dir(already_gone);
        ctx.track_temporary_dir(standing.clone());
        drop(ctx);

        assert!(
            !standing.exists(),
            "the script ended, so what it was handed goes — including what the program put \
             inside it, since the sweep is recursive over a directory the runtime made"
        );
    }

    /// § 3's goal state reached early: a program may remove its own temporary
    /// directory whenever it likes, and a sweep that remarked on it would turn
    /// tidy programs into log noise.
    ///
    /// Driven through [`at_script_end`] rather than through the teardown its
    /// sibling drops, because the claim is about what was *not* written and a
    /// context that is gone has no sink left to read back.
    #[test]
    fn a_temporary_dir_the_program_already_removed_is_not_an_error() {
        let base = scratch("already-gone");
        let already_gone = base.join("already-gone");
        let behind_it = base.join("behind-it");
        std::fs::create_dir(&behind_it).expect("the base exists");

        let mut ctx = Ctx::buffered();
        ctx.set_diagnostic_sink(crate::OutputSink::Buffer(Vec::new()));
        ctx.track_temporary_dir(already_gone);
        ctx.track_temporary_dir(behind_it.clone());
        at_script_end(&mut ctx);

        let written = diagnostic(&mut ctx);
        assert!(
            written.is_empty(),
            "a directory that is already gone is not worth a line: {written:?}"
        );
        assert!(
            ctx.pending().is_none(),
            "and it is not worth a fault either: {:?}",
            ctx.pending()
        );
        assert!(
            !behind_it.exists(),
            "and the sweep carried on past it to the rest of the script's list"
        );
    }

    /// A snapshot built from the text an operator would have written, rather
    /// than from the typed tree — the boot path deserializes, so a case that
    /// constructed the struct directly would pin a key no configuration file
    /// can express. `capability`'s own tests carry the same helper for the same
    /// reason, and neither is reachable from the other.
    fn snapshot_of(written: &str) -> std::sync::Arc<nvs_config::Snapshot> {
        let table: toml::Table = written.parse().expect("the case writes valid TOML");
        std::sync::Arc::new(nvs_config::Snapshot {
            config: table
                .clone()
                .try_into()
                .expect("the case writes a block this tree has"),
            table,
            ..nvs_config::Snapshot::default()
        })
    }

    /// § 4's predicate over a name the runtime itself wrote — the agreement no
    /// hand-built fixture can make.
    ///
    /// `capability::temp_dir` is the only writer of these names and this reads
    /// one it actually produced, so a name whose shape drifted from the parse
    /// fails here rather than silently answering *skip* forever, which is what
    /// an orphan sweep that never deleted anything would look like.
    #[test]
    fn an_entry_this_process_wrote_is_read_as_a_live_owners() {
        let root = scratch("live-owner");
        let mut ctx = Ctx::buffered();
        ctx.set_config(snapshot_of(&format!(
            "[capabilities.fs]\nwrite = true\n\n[io]\ntemp_root = '{}'\n",
            root.display()
        )));
        let made = crate::capability::temp_dir(&mut ctx, "Core\\IO::temporaryDir")
            .expect("`write = true` covers the root the case configured");

        assert!(
            owner_is_alive(&made),
            "this process wrote {} and is still running",
            made.display()
        );

        drop(ctx);
    }

    /// The runners' door: [`crate::capability::private_dir`] creates a root
    /// that is not there yet, and the entry it makes carries this process's
    /// pid, so the walk reads it as a live owner's and a killed run's as an
    /// orphan.
    #[test]
    fn a_runner_dir_is_private_and_named_for_the_orphan_sweep() {
        let scratch = scratch("runner-dir");
        let root = scratch.join("not-there-yet");
        let made = crate::capability::private_dir(&root)
            .expect("a writable scratch directory takes a root and an entry");

        assert!(made.is_dir(), "{} was created", made.display());
        assert_eq!(made.parent(), Some(root.as_path()));
        assert_eq!(owning_pid(&made), Some(std::process::id()));
        assert!(orphans(&root).is_empty(), "a live owner's entry is skipped");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for dir in [&root, &made] {
                let mode = std::fs::metadata(dir)
                    .expect("it exists")
                    .permissions()
                    .mode();
                assert_eq!(mode & 0o777, 0o700, "{} is owner-only", dir.display());
            }
        }
        let other = crate::capability::private_dir(&root).expect("a second entry");
        assert_ne!(made, other, "each call makes a new directory");
    }

    /// § 4's remaining answers, in one case because they are one rule read from
    /// two sides: a pid no process can hold is dead, and everything this module
    /// did not name is skipped.
    ///
    /// The skipped names are the half that matters. Each of them is a
    /// *deletion* if the parse is loose — an operator's own directory in the
    /// root, a name with no nonce, a name whose pid field is the Unix process
    /// group — and over-deleting is the one failure § 4 does not allow.
    #[test]
    fn a_dead_owners_entry_is_swept_and_every_ambiguity_is_skipped() {
        // Above every platform's pid ceiling — Linux's `pid_max` tops out at
        // 2^22 and Windows hands out far smaller numbers — so no process can be
        // holding it and the answer is not a race.
        let dead = PathBuf::from(format!("nvs-{}-0123456789abcdef", i32::MAX - 1));
        assert!(
            !owner_is_alive(&dead),
            "no process holds that pid, so the entry is an orphan: {}",
            dead.display()
        );

        for (what, name) in [
            ("a name this module never wrote", "notes"),
            ("one carrying the prefix and nothing else", "nvs-99999999"),
            ("one whose nonce is not a nonce", "nvs-99999999-notes"),
            (
                "one whose nonce is the wrong width",
                "nvs-99999999-0123456789abcde",
            ),
            (
                "one naming the Unix process group",
                "nvs-0-0123456789abcdef",
            ),
        ] {
            let path = PathBuf::from(name);
            assert!(
                owner_is_alive(&path),
                "{what} is not this runtime's to delete, so it reads as alive: {name}"
            );
        }
    }

    /// § 4's walk, asserted by **what the whole root came back as** rather than
    /// by one path being in the answer.
    ///
    /// Every entry below is a *deletion* if the walk is one filter too loose,
    /// so a case that only asked "is the orphan listed" would pass against a
    /// walk that listed the live owner's directory and the operator's notes
    /// beside it. Two orphans rather than one because the answer is sorted and
    /// one path cannot show an order. And the entries are still on disk
    /// afterwards, which is the walk's other half: it answers, and the door
    /// deletes.
    #[test]
    fn the_walk_answers_every_dead_owners_directory_and_nothing_else() {
        let root = scratch("orphans");
        // Above every platform's pid ceiling, for the reason the predicate's
        // own case gives: no process can be holding it, so this is not a race.
        let dead = i32::MAX - 1;
        let first = root.join(format!("nvs-{dead}-0000000000000001"));
        let second = root.join(format!("nvs-{dead}-0000000000000002"));
        let live = root.join(format!("nvs-{}-0123456789abcdef", std::process::id()));
        for path in [&first, &second, &live, &root.join("notes")] {
            std::fs::create_dir_all(path).expect("the scratch root is writable");
        }
        // A file wearing a dead owner's name: `temp_dir` writes directories and
        // only directories, so this is not one of ours however it is spelled.
        std::fs::write(root.join(format!("nvs-{dead}-0000000000000003")), b"")
            .expect("the scratch root is writable");

        assert_eq!(
            orphans(&root),
            vec![first.clone(), second.clone()],
            "the two dead owners' directories, in order, and nothing else in {}",
            root.display()
        );
        assert!(
            first.is_dir() && second.is_dir(),
            "the walk answers what is dead and deletes none of it"
        );
        assert_eq!(
            orphans(&root.join("never-created")),
            Vec::<PathBuf>::new(),
            "a root nothing has created yet has nothing to sweep"
        );
    }

    /// § 5's escape hatch, both halves in one case: **each** kept path is named
    /// and **none** of them is deleted.
    ///
    /// Two directories rather than one, because § 5's promise is that the
    /// absence of cleanup is always visible — a sweep that logged the first and
    /// kept the rest silently would leave exactly the untraceable leftovers the
    /// key exists to make traceable, and would pass every assertion about one
    /// path.
    #[test]
    fn keep_temporary_logs_each_kept_path_and_deletes_none() {
        let base = scratch("kept");
        let first = base.join("first");
        let second = base.join("second");
        std::fs::create_dir(&first).expect("the base exists");
        std::fs::create_dir(&second).expect("the base exists");

        let mut ctx = Ctx::buffered();
        ctx.set_config(snapshot_of("[debug]\nkeep_temporary = true\n"));
        ctx.set_diagnostic_sink(crate::OutputSink::Buffer(Vec::new()));
        ctx.track_temporary_dir(first.clone());
        ctx.track_temporary_dir(second.clone());
        at_script_end(&mut ctx);

        let written = diagnostic(&mut ctx);
        assert_eq!(written.len(), 2, "one line per kept path: {written:?}");
        assert!(
            written.iter().any(|line| line.contains("first"))
                && written.iter().any(|line| line.contains("second")),
            "and each names the path it kept: {written:?}"
        );
        assert!(
            first.is_dir() && second.is_dir(),
            "and nothing was deleted — that is what the operator asked for"
        );

        // The list is drained even so, so the keeping is per script rather than
        // a queue that grows: the teardown behind this call finds nothing.
        drop(ctx);
        assert!(
            first.is_dir() && second.is_dir(),
            "including at the teardown, which has nothing left to sweep"
        );
    }

    /// § 3's refusal, from the side [`refusals`] cannot show: what the *sweep*
    /// does with one, which is a line and nothing else.
    ///
    /// The refusal is a path that is not a directory, because that is the one
    /// reading every platform agrees on — a held handle is Windows's routine
    /// reason and there is no portable way to hold one.
    #[test]
    fn a_refused_deletion_logs_and_never_throws() {
        let base = scratch("logs");
        let not_a_directory = base.join("not-a-directory");
        std::fs::write(&not_a_directory, b"a file where a directory was expected")
            .expect("the base exists");

        let mut ctx = Ctx::buffered();
        ctx.set_diagnostic_sink(crate::OutputSink::Buffer(Vec::new()));
        ctx.track_temporary_dir(not_a_directory.clone());
        at_script_end(&mut ctx);

        let written = diagnostic(&mut ctx);
        assert_eq!(written.len(), 1, "one refusal is one line: {written:?}");
        let line = &written[0];
        assert!(
            line.to_lowercase().contains("warn"),
            "nothing was lost and no request was affected, so it is a note: {line}"
        );
        assert!(
            line.contains("not-a-directory"),
            "naming the path left standing, as its own field so a pipeline can count \
             refusals per root: {line}"
        );
        assert!(
            line.contains("error"),
            "and what the operating system answered: {line}"
        );
        assert!(
            ctx.pending().is_none(),
            "and § 3's never-throws: there is no frame left to raise on: {:?}",
            ctx.pending()
        );
        assert!(
            not_a_directory.exists(),
            "the entry waits in the owned root for § 4's next attempt"
        );
    }

    /// § 3's refusal, from the one side that is the same on every platform: a
    /// path that is not a directory at all cannot be removed as one, and what
    /// the sweep does about it is answer rather than fail.
    ///
    /// The other paths in the same call are the point. A sweep that gave up at
    /// the first error would leave a script's remaining directories standing for
    /// § 4 to find, and every deletion here would still look correct on its own.
    #[test]
    fn a_refused_deletion_is_answered_rather_than_raised_and_the_rest_still_go() {
        let base = scratch("refused");
        let not_a_directory = base.join("not-a-directory");
        let after_it = base.join("after-it");
        std::fs::write(&not_a_directory, b"a file where a directory was expected")
            .expect("the base exists");
        std::fs::create_dir(&after_it).expect("the base exists");

        let refused = refusals(vec![not_a_directory.clone(), after_it.clone()]);

        assert_eq!(
            refused.len(),
            1,
            "one path refused and the sweep answered with it: {refused:?}"
        );
        assert_eq!(refused[0].0, not_a_directory, "and named which one");
        assert!(
            !after_it.exists(),
            "the entry behind the refusal is still swept"
        );
    }
}
