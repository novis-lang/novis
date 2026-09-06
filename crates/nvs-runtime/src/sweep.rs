//! [ADR 0131](/docs/adr/0131-a-temporary-directory-dies-with-its-script-and-the-sweep-never-throws.md)
//! § 3's end-of-script sweep: the directories `Core\IO::temporaryDir` handed
//! this script, deleted when the script ends.
//!
//! The program is never asked to clean up and never asked to remember. What it
//! was handed, [`Ctx::track_temporary_dir`] wrote down; what is written down,
//! this module deletes.
//!
//! # It runs from [`Drop`], which is the whole of "after the last user code"
//!
//! § 3 puts the sweep after [ADR 0127](/docs/adr/0127-the-end-of-a-script-is-observable.md)'s
//! `onExit` queue on a CLI ending and after
//! [ADR 0072](/docs/adr/0072-core-task-structured-concurrency.md) § 6's
//! `afterResponse` work on a request, and has it cover every ending the process
//! survives — normal, `exit`, an uncaught throw, and a request that died
//! mid-flight. A context's teardown is all four of those at once and is
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
//! [`refusals`] is the part with no logging in it, which is what makes the
//! behaviour testable without a sink to read back.

use std::path::PathBuf;

use nvs_render::Level;

use crate::ctx::Ctx;

/// Deletes every temporary directory this script was handed, and logs the ones
/// the operating system refused — [ADR 0131] § 3, called from `Ctx::drop`.
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
/// (ADR 0092 § 5).
///
/// [ADR 0131]: ../../docs/adr/0131-a-temporary-directory-dies-with-its-script-and-the-sweep-never-throws.md
pub fn at_script_end(ctx: &mut Ctx) {
    let taken = ctx.take_temporary_dirs();
    if taken.is_empty() {
        // The ordinary script: no allocation was made for the list and no clock
        // is read for the sweep.
        return;
    }
    for (path, error) in refusals(taken) {
        let record = note(&path, &error);
        crate::floor::report(ctx, &record);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory under the platform root that this process alone is using —
    /// `capability`'s own test helper, which is not reachable from here.
    fn scratch(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("nvs-sweep-test-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&path).expect("the platform root is writable");
        path
    }

    /// § 3's ordinary case and its goal-state case in one, because what
    /// separates them is only which of the two directories is still on disk when
    /// the sweep runs — and a sweep that stopped at the first missing one would
    /// pass a test that asserted either alone.
    #[test]
    fn a_directory_still_standing_is_removed_and_one_already_gone_is_not_an_error() {
        let base = scratch("standing");
        let standing = base.join("standing");
        let with_contents = base.join("with-contents");
        let already_gone = base.join("already-gone");
        std::fs::create_dir(&standing).expect("the base exists");
        std::fs::create_dir(&with_contents).expect("the base exists");
        std::fs::write(with_contents.join("note.txt"), b"what the program wrote")
            .expect("the directory was just created");

        let refused = refusals(vec![
            already_gone.clone(),
            standing.clone(),
            with_contents.clone(),
        ]);

        assert!(
            refused.is_empty(),
            "a directory that is already gone is the goal state reached early, not a refusal: \
             {refused:?}"
        );
        assert!(!standing.exists(), "the empty one is deleted");
        assert!(
            !with_contents.exists(),
            "and so is the one holding what the program put in it — the sweep is recursive over \
             a directory the runtime itself created"
        );

        std::fs::remove_dir_all(&base).expect("the case removes what it made");
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

        std::fs::remove_dir_all(&base).expect("the case removes what it made");
    }
}
