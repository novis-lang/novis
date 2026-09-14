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
//! same question, drifting from the first the day either is edited — so these
//! tests *are* the script, run over the repository they sit in, plus the
//! judgements below that the script deliberately does not make.
//!
//! # What makes a standing site acceptable
//!
//! Two things, and the difference between them is who owns the site:
//!
//! *   **An open numbered item claims it** — one in
//!     `docs/agent/loop-goal.md`, or one in `docs/agent/carried-refusals.md`,
//!     which is where a hole an earlier milestone left is owned once no current
//!     goal can be judged on it (a goal switch carries a check forward and the
//!     items that made it green not at all, so that inventory has to live
//!     somewhere the switch does not rewrite). That
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
//!
//! # What makes a site leave the count
//!
//! Lowering the shape, or a `lower::guarded_by!` naming the diagnostic that
//! refuses it where it is written — `nvs_ir`'s own § *Known gaps* preamble is
//! where the two are contrasted. A guard is a claim about the front end rather
//! than a hole, so `holes.py` does not count it; what holds it honest is
//! [`every_guarded_site_names_a_code_a_conformance_case_expects`], which
//! requires a conformance case to expect the code each guard names. Rewording a
//! `panic!` is neither close, and a guard naming a code nothing raises is that
//! rewording wearing a macro.

use std::collections::HashSet;
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
///
/// **This number may only fall**, with one exception this file records rather
/// than hides: a goal whose *own* design lands an operator in stages leaves a
/// hole between them, and refusing to write the number down would only make
/// the count a floor. A count is a ratchet only when it is the truth rather
/// than a floor on it, which is why `holes.py` reads a site from the construct
/// that carries it rather than from a set of fixed phrasings: a phrasing match
/// leaves sites unseen, and a ratchet set from a blind count is not a ratchet.
///
/// A site leaves this count two ways, and `nvs_ir`'s own § *Known gaps*
/// preamble is where they are contrasted: the shape lowers, or the site becomes
/// a `lower::guarded_by!` naming the diagnostic that refuses the shape where it
/// is written, which is a front-end guarantee rather than a hole.
///
/// One site here is open on purpose rather than unowned: `$x is T`, whose stage
/// lowers a scalar, `null`, `object`, a bare `array`, a class and an `array<T>`
/// and leaves a literal, an enum case, a shape, a union, `iterable` and
/// `callable` for the stage after it. That item's own attribution is what keeps
/// the first half of this gate green while it stands.
const CEILING: usize = 8;

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

/// Each site in a `--guarded` listing, as `(file, line, code)`. The listing
/// groups by file the way `--sites` does, and the code is the one token after
/// the line number — `no-such-constant` where the guard names a constant the
/// registry does not declare, which reads as a code nothing expects and fails
/// below rather than needing a rule of its own.
fn guarded(out: &str) -> Vec<(String, String, String)> {
    let mut found = Vec::new();
    let mut file = String::new();
    for line in out.lines() {
        let body = line.trim_end();
        if let Some(rest) = body.strip_prefix("    :") {
            let (at, rest) = rest.split_once("  ").unwrap_or((rest, ""));
            let code = rest.split_whitespace().next().unwrap_or_default();
            found.push((file.clone(), at.trim().to_owned(), code.to_owned()));
        } else if let Some(rest) = body.strip_prefix("  ")
            && rest.starts_with("crates/")
        {
            file = rest.to_owned();
        }
    }
    found
}

/// Every diagnostic code a `.nvst` case under `dir` expects: `error[E0700]`
/// contributes `E0700`.
fn expected_codes(dir: &Path, found: &mut HashSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            expected_codes(&path, found);
        } else if path.extension().is_some_and(|kind| kind == "nvst") {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            for (at, _) in text.match_indices("error[") {
                let rest = &text[at + "error[".len()..];
                if let Some(end) = rest.find(']') {
                    found.insert(rest[..end].to_owned());
                }
            }
        }
    }
}

/// Every `lower::guarded_by!` names a code some conformance case expects.
///
/// A guard is a claim about the front end — *this shape never arrives, because
/// that diagnostic refuses it where it is written* — and it buys a close off
/// [`CEILING`] on the strength of it. The claim is worth that only if something
/// proves the diagnostic is raised, so each code named has to be one a `.nvst`
/// case under `tests/conformance` expects. Without this, converting a `panic!`
/// to a guard would be the rewording the goal forbids, wearing a macro.
#[test]
fn every_guarded_site_names_a_code_a_conformance_case_expects() {
    let cases = root().join("tests/conformance");
    assert!(
        cases.is_dir(),
        "{} is not where it was — this gate would pass vacuously",
        cases.display()
    );

    let out = holes("--guarded");
    let total = counted(&out, "guarded");
    let sites = guarded(&out);
    assert_eq!(
        total,
        sites.len(),
        "read a different number of guarded sites than the header claims"
    );

    let mut expects = HashSet::new();
    expected_codes(&cases, &mut expects);
    assert!(
        !expects.is_empty(),
        "no conformance case under {} expects any diagnostic at all, so this gate would \
         refuse every guard rather than checking one",
        cases.display()
    );

    let unproven: Vec<String> = sites
        .iter()
        .filter(|(_, _, code)| !expects.contains(code.as_str()))
        .map(|(file, line, code)| format!("  {file}:{line}  names {code}"))
        .collect();
    assert!(
        unproven.is_empty(),
        "{} guarded site(s) name a diagnostic no conformance case expects. A guard says \
         the front end refuses the shape where it is written; a code nothing raises is a \
         refusal closed on paper only. Write the case, or name the code that is really \
         raised — `python tools/holes.py --guarded` lists them.\n{}",
        unproven.len(),
        unproven.join("\n")
    );
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
         refuses, with no open item in docs/agent/loop-goal.md or \
         docs/agent/carried-refusals.md claiming it and no entry on this test's \
         allowlist. Give it a diagnostic, close it, or take the decision in loop-goal.md \
         § Standing decisions — do NOT add it to the allowlist to make this pass.\n{}",
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
