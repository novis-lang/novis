//! Stage 8's end gate: **every refusal this compiler still carries is either a
//! diagnostic the front end raises, a hole an open goal item owns, or an entry
//! on a frozen allowlist** — and nothing else.
//!
//! A refusal site is a `panic!`/`todo!`/`unimplemented!` naming a shape, or a
//! `CodegenError::Unsupported`: the places where a program that type-checks
//! stops working anyway. Counting conformance cases says how much of the
//! language is *exercised*; only this says how much of it is *there*.
//!
//! # Why this shells out to `tools/holes.py`
//!
//! That script is already the tree's one recognizer for a refusal site, and
//! `docs/implementation-plan.md`'s `Open now` names it as the worklist no
//! session re-derives. A second regex here would be a second answer to the
//! same question, drifting from the first the day either is edited — so this
//! test *is* the script, run over the repository it sits in, plus the two
//! judgements below that the script deliberately does not make.
//!
//! # What makes a standing site acceptable
//!
//! Two things, and the difference between them is who owns the site:
//!
//! *   **An open numbered item in `docs/agent/loop-goal.md` claims it.** That
//!     is a hole with a schedule: the item names the file and the function,
//!     and closing the item removes the site. `holes.py`'s own attribution
//!     decides this, so a site in a file no item anchors is *unattributed* and
//!     fails here. When the last item lands there is nothing left to claim a
//!     site, which is exactly when this gate starts refusing everything not on
//!     the allowlist — the end state the check's name, `nvs-ir (no refusal
//!     left)`, describes.
//! *   **[`ALLOWLIST`] names it**, because it is a refusal the language means
//!     to keep. **That list may never grow.** Every entry is a bullet in
//!     `docs/agent/loop-goal.md` § *Standing decisions*, and adding one to
//!     make a run go green is the single move that goal forbids outright. It
//!     is empty today, and the honest way to keep it empty is to make the
//!     refusal a diagnostic or to remove it.
//!
//! [`CEILING`] is the third half of the same rule: attribution is by file, so
//! a *new* refusal added beside an old one in a claimed file would be invisible
//! to the first test. The count ratchets down and never up.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A refusal the language keeps on purpose, as `(file, the start of its
/// message)`.
///
/// **This may never grow** — see this module's docs. A line number is
/// deliberately not part of the key: every edit above a site moves it, and a
/// key that rots is a key somebody eventually deletes.
const ALLOWLIST: &[(&str, &str)] = &[];

/// The number of refusal sites the tree is allowed to hold in total.
///
/// A ratchet, not a target: a session that closes one lowers this in the same
/// slice, and a session that adds one has to explain itself to a red test
/// first. `python tools/holes.py` prints the current number.
const CEILING: usize = 4;

/// The repository root — this crate is `crates/nvs-ir`.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `python tools/holes.py <args>`, run at the repository root.
///
/// The interpreter is looked up the way every other entry point into this
/// tree's tooling is invoked, and a machine with no Python fails the gate
/// rather than skipping it: `verify.py`, `loop.py` and `session.py` are all
/// Python, so a checkout that cannot run one cannot run this project's checks
/// at all.
fn holes(arg: &str) -> String {
    let script = root().join("tools/holes.py");
    assert!(script.is_file(), "{} is missing", script.display());
    let mut last = String::new();
    for exe in ["python3", "python", "py"] {
        let out = Command::new(exe)
            .arg(&script)
            .arg(arg)
            .current_dir(root())
            .output();
        match out {
            Ok(done) if done.status.success() => {
                return String::from_utf8_lossy(&done.stdout).replace("\r\n", "\n");
            }
            Ok(done) => {
                last = format!(
                    "{exe} exited {}: {}",
                    done.status,
                    String::from_utf8_lossy(&done.stderr)
                );
            }
            Err(err) => last = format!("{exe}: {err}"),
        }
    }
    panic!("could not run `tools/holes.py {arg}` — {last}");
}

/// The leading `N` of `holes.py`'s `"N … site(s)"` header.
///
/// Read rather than assumed, and every caller cross-checks it against what it
/// parsed: a format change that this file did not follow shows up as a
/// mismatch instead of as a vacuous pass.
fn counted(out: &str, what: &str) -> usize {
    let head = out.lines().next().unwrap_or_default();
    let (n, rest) = head.split_once(' ').unwrap_or_default();
    assert_eq!(
        rest,
        format!("{what} site(s)"),
        "`holes.py` printed a header this test does not know how to read: {head:?}"
    );
    n.parse().unwrap_or_else(|err| {
        panic!("`holes.py` header {head:?} does not start with a count: {err}")
    })
}

/// Each site in a `--sites`/`--unattributed` listing, as `(file, line,
/// message)`. The listing groups by file, so the file is carried down.
fn listed(out: &str) -> Vec<(String, String, String)> {
    let mut found = Vec::new();
    let mut file = String::new();
    for line in out.lines() {
        let body = line.trim_end();
        if let Some(rest) = body.strip_prefix("    :") {
            let (at, message) = rest.split_once("  ").unwrap_or((rest, ""));
            found.push((file.clone(), at.to_owned(), message.trim().to_owned()));
        } else if let Some(rest) = body.strip_prefix("  ")
            && rest.starts_with("crates/")
        {
            file = rest.to_owned();
        }
    }
    found
}

#[test]
fn every_refusal_is_a_diagnostic_or_decided() {
    for crate_src in ["crates/nvs-ir/src", "crates/nvs-codegen/src"] {
        assert!(
            root().join(crate_src).is_dir(),
            "{crate_src} is not where it was — this gate would pass vacuously"
        );
    }

    let all = holes("--sites");
    let total = counted(&all, "refusal");
    assert_eq!(
        total,
        listed(&all).len(),
        "read a different number of sites than the header claims"
    );

    let orphans = holes("--unattributed");
    let orphan_count = counted(&orphans, "unattributed refusal");
    let orphans = listed(&orphans);
    assert_eq!(
        orphan_count,
        orphans.len(),
        "read a different number of unattributed sites than the header claims"
    );

    let standing: Vec<String> = orphans
        .iter()
        .filter(|(file, _, message)| {
            !ALLOWLIST
                .iter()
                .any(|(at, start)| at == file && message.starts_with(start))
        })
        .map(|(file, line, message)| format!("  {file}:{line}  {message}"))
        .collect();
    assert!(
        standing.is_empty(),
        "{} refusal(s) belong to nobody — each is a shape that type-checks and then \
         refuses, with no open item in docs/agent/loop-goal.md claiming it and no entry \
         on this test's allowlist. Give it a diagnostic, close it, or take the decision \
         in loop-goal.md § Standing decisions — do NOT add it to the allowlist to make \
         this pass.\n{}",
        standing.len(),
        standing.join("\n")
    );

    assert!(
        total <= CEILING,
        "the tree now holds {total} refusal site(s) against a ceiling of {CEILING}. A new \
         refusal in a file an item already anchors is claimed by that item's attribution \
         and would otherwise be invisible here, so the count is the second half of the \
         gate. `python tools/holes.py` lists them."
    );
    assert!(
        total >= CEILING,
        "the tree is down to {total} refusal site(s) from a ceiling of {CEILING} — lower \
         CEILING to {total} in this file, in the slice that closed them, so the ratchet \
         cannot slip back."
    );
}
