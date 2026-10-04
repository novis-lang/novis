//! `nvs tmp clean` — `rule:core-classes/temporary-dir-orphan-sweep`'s orphan sweep, as the operator's own door.
//!
//! The runtime deletes a temporary directory when its script ends
//! (`nvs_runtime::sweep`, § 3) and reclaims a hard-killed script's leftovers at
//! `nvs serve` boot (`crate::serve::sweep_orphans`, § 4). This command is the
//! other half of § 4, and it exists because those two do not cover a machine
//! that never runs a server: on one of those, a leftover waits here or waits
//! forever.
//!
//! # The predicate is not this command's, deliberately
//!
//! Which entries go is [`nvs_runtime::sweep::orphans`]'s answer and nothing is
//! re-decided here — liveness, never age, and every ambiguity resolved toward
//! leaving the entry alone. That module's doc owns the whole rule. What is here
//! is the operator's half of it: the configured root, one line per path, and
//! `--dry-run`.
//!
//! **There is no force flag.** § 4 says so in as many words, and it is the one
//! flag an operator would eventually reach for: a sweep that could be told to
//! ignore liveness is a sweep that can delete a running process's directory out
//! from under it, which is precisely what keying on liveness rather than age
//! bought. The worst this command can do is nothing.
//!
//! # It reads the tree, and it names no application
//!
//! `[io] temp_root` is `System`-class (`nvs_config::directive`), so the root is
//! the host's and no request can have moved it. This command
//! resolves the configuration tree the way `nvs config check` does and reads
//! that key straight off it: there is no entry file to layer
//! `rule:config/every-matching-app-block-applies-least-specific-first`'s
//! `[[app]]` blocks with, and an operator clearing the host's root should not
//! have to name one of the applications sharing it.
//!
//! # The exit status is about the configuration and nothing else
//!
//! A tree that will not resolve is a failure, because the command could not tell
//! which root it was being asked about. Everything after that succeeds: a root
//! that is not there has nothing in it, and a deletion the platform refuses — on
//! Windows, routinely a handle an indexer is holding — is a printed line and an
//! entry that waits for the next sweep, exactly as § 4 states. An operator's
//! cleanup command that exits non-zero because a scanner had a file open is a
//! failing deployment step over nothing.
//!

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nvs_diagnostics::{Diagnostics, SourceMap};

use crate::config::{LocalFiles, working_directory};
use crate::render_diagnostics;

/// `nvs tmp clean [--dry-run]` — resolve the tree, walk the owned root, and
/// report every entry whose owner is gone.
pub(crate) fn clean(config: &[PathBuf], dry_run: bool) -> ExitCode {
    let files = LocalFiles;
    let mut sources = SourceMap::new();
    // `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`'s roots, resolved as `config check` resolves them and for the
    // same reason: this command reads one key and never boots anything, so it
    // wants the tree as written rather than a snapshot built against an entry.
    let resolved = working_directory().and_then(|cwd| {
        let roots = nvs_config::resolve::roots(config, &cwd, &files);
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

    clean_root(
        &nvs_runtime::capability::temp_root(Some(&resolved.config)),
        dry_run,
        &mut std::io::stdout(),
    )
}

/// [`clean`] with the root already decided and the output already chosen — the
/// whole of what this command *does*, separated from where it learns the root so
/// that a case can point it at a directory it built.
///
/// **Every path is named, whichever way it went.** § 4 asks for the removals to
/// be printed; a refusal is printed too, because an operator who was told
/// nothing would read the silence as a cleared root and the entry would sit
/// there unaccounted for. A dry run prints the same list under a different verb
/// and touches nothing at all — not even the entries it would certainly have
/// removed, which is the only way the flag is worth having.
fn clean_root(root: &Path, dry_run: bool, out: &mut impl Write) -> ExitCode {
    let orphans = nvs_runtime::sweep::orphans(root);
    if orphans.is_empty() {
        // Said rather than left silent: an operator running this wants to know
        // it looked, and at which root, since `[io] temp_root` is the key most
        // likely to be pointing somewhere they did not mean.
        let _ = writeln!(out, "nothing to clean in {}", root.display());
        return ExitCode::SUCCESS;
    }
    if dry_run {
        for path in &orphans {
            let _ = writeln!(out, "would remove {}", path.display());
        }
        return ExitCode::SUCCESS;
    }

    // Attempted first and reported after, so a path that could not be removed is
    // never printed as though it had been. The lookup is quadratic over one
    // directory listing's worth of paths, which is the cheaper thing to be than
    // a second shape for the sweep to hand back.
    let refused = nvs_runtime::sweep::refusals(orphans.clone());
    for path in &orphans {
        match refused.iter().find(|(refused, _)| refused == path) {
            Some((_, error)) => {
                let _ = writeln!(
                    out,
                    "note: {} could not be removed and waits for the next sweep: {error}",
                    path.display()
                );
            }
            None => {
                let _ = writeln!(out, "removed {}", path.display());
            }
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{Path, PathBuf, clean_root};

    /// A scratch root this case alone is using, deleted when the guard drops.
    fn root_of(case: &str) -> nvs_repo::Scratch {
        nvs_repo::scratch(&format!("tmp-clean-{case}"))
    }

    /// An entry in `root` owned by `pid`, spelled exactly as
    /// `nvs_runtime::capability::temp_dir` spells one.
    fn entry(root: &Path, pid: u32, nonce: &str) -> PathBuf {
        let path = root.join(format!("nvs-{pid}-{nonce}"));
        std::fs::create_dir_all(&path).expect("the scratch root is writable");
        path
    }

    /// Above every platform's pid ceiling, so no process can be holding it and
    /// the answer is not a race with anything this machine is running.
    const DEAD: u32 = i32::MAX as u32 - 1;

    fn printed(root: &Path, dry_run: bool) -> String {
        let mut out = Vec::new();
        clean_root(root, dry_run, &mut out);
        String::from_utf8(out).expect("this command writes paths and English")
    }

    /// § 4's rule at the operator's door, asserted from both sides at once: the
    /// dead owner's entry goes and everything else stays.
    ///
    /// The live owner's entry is the one that matters — it is a *deletion* if
    /// the liveness question is asked the wrong way round — and the operator's
    /// own directory matters beside it: the root is Novis's, but a name this
    /// runtime never wrote is still not this command's to remove. The printed
    /// lines are asserted as well as the disk, because § 4 asks for the removals
    /// to be named and a sweep that cleared the root silently would pass every
    /// assertion about the filesystem.
    #[test]
    fn tmp_clean_removes_a_dead_owners_entry_and_skips_a_live_one() {
        let root = root_of("mixed");
        let dead = entry(&root, DEAD, "0123456789abcdef");
        let live = entry(&root, std::process::id(), "0123456789abcdef");
        let theirs = root.join("notes");
        std::fs::create_dir_all(&theirs).expect("the scratch root is writable");

        let said = printed(&root, false);

        assert!(!dead.exists(), "the owner of {} is gone", dead.display());
        assert!(
            live.is_dir() && theirs.is_dir(),
            "a live owner's entry and a name this runtime never wrote both stay"
        );
        assert_eq!(
            said,
            format!("removed {}\n", dead.display()),
            "one line, naming the one entry that went"
        );

        std::fs::remove_dir_all(&root).expect("the case removes what it made");
    }

    /// § 4's `--dry-run`, asserted as *the same answer without the effect*: the
    /// flag prints the entry, leaves it standing, and the run that follows
    /// removes exactly what the dry run said it would.
    ///
    /// The second call is what makes the first assertion mean something. A dry
    /// run that printed a path it would in fact never have touched is the
    /// failure this shape catches and a case asserting the flag alone cannot.
    #[test]
    fn tmp_clean_dry_run_prints_and_deletes_nothing() {
        let root = root_of("dry-run");
        let dead = entry(&root, DEAD, "0123456789abcdef");

        let rehearsed = printed(&root, true);
        assert_eq!(
            rehearsed,
            format!("would remove {}\n", dead.display()),
            "the flag names what it would do and says so in a different verb"
        );
        assert!(
            dead.is_dir(),
            "and does none of it: {} is still there",
            dead.display()
        );

        assert_eq!(
            printed(&root, false),
            format!("removed {}\n", dead.display()),
            "the real run removes exactly what the rehearsal named"
        );
        assert!(!dead.exists(), "and this time it is gone");

        std::fs::remove_dir_all(&root).expect("the case removes what it made");
    }

    /// § 4's "liveness, **never** age", which is the whole reason the predicate
    /// is what it is: an entry old enough that any age rule would take it, whose
    /// owner is this very process, survives.
    ///
    /// Both § 4 doors are covered by one fixture, because both are
    /// [`nvs_runtime::sweep::orphans`] plus what they do with the answer — the
    /// boot sweep hands it to `refusals` (`crate::serve::sweep_orphans`) and
    /// this command prints it — so an entry the walk never answers is one no
    /// door can delete. Asserting the walk's answer is *empty* rather than
    /// "does not contain this path" is what makes that reasoning hold for the
    /// whole root.
    ///
    /// The backdating is on a file inside the entry rather than on the entry
    /// itself: a directory's own timestamp cannot be set portably (`File::open`
    /// on a directory is refused on Windows without a backup-semantics flag),
    /// and an age rule reaching for either would find this one.
    #[test]
    fn an_old_entry_with_a_live_owner_survives_every_sweep() {
        let root = root_of("old-and-live");
        let mine = entry(&root, std::process::id(), "0123456789abcdef");
        let long_ago = std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(400 * 24 * 60 * 60))
            .expect("this machine's clock is well past 1971");
        let stamp = std::fs::File::create(mine.join("written-long-ago"))
            .expect("the case writes into its own entry");
        stamp
            .set_modified(long_ago)
            .expect("a file this frame just created carries a settable time");
        drop(stamp);

        assert_eq!(
            nvs_runtime::sweep::orphans(&root),
            Vec::<PathBuf>::new(),
            "the walk both § 4 doors take answers nothing at all in {}",
            root.display()
        );
        assert_eq!(
            printed(&root, false),
            format!("nothing to clean in {}\n", root.display()),
            "so the operator's door says the root is clear"
        );
        assert!(
            mine.join("written-long-ago").is_file(),
            "and a year-old file under a live owner's entry is untouched"
        );

        std::fs::remove_dir_all(&root).expect("the case removes what it made");
    }
}
