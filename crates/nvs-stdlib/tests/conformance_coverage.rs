//! The coverage gate `docs/agent/loop-goal.md`'s Stage 4 names: every `Core`
//! member this crate registers is called by at least one `.nvst` case under
//! `tests/conformance/`.
//!
//! A registry row and an implementation together still prove nothing about
//! *behaviour* — `crates/nvs-stdlib/src/lib.rs` says a half-added member
//! cannot link, and this says a member cannot be added without a case that
//! runs it. It is the one check that reads across the crate boundary, which is
//! why it is an integration test rather than a `#[cfg(test)]` module: it needs
//! the repository, not the crate.
//!
//! **The enumerable set is what is registered, not what the spec owes.**
//! [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md) §§ 1-12
//! is the work list and `nvs-stdlib`'s known gap 1 tracks how much of it is on
//! disk; a member the registry does not hold is not yet a member, and the
//! compiler refuses to resolve a call to one, so there is nothing here to
//! check. What this forbids is the other order: registering a member, then
//! never writing the case.
//!
//! **The second gate is depth, and it is the same corpus read a second way.**
//! One case per member says a member runs; it says nothing about whether it
//! was asked anything it could get wrong. Stage 5's item 10 puts a floor of
//! three cases under every member, and
//! [`every_core_class_has_a_conformance_floor_of_three`] is that floor —
//! landed as a ratchet over the twenty-six members that were already below it,
//! for the reason [`BELOW_THE_FLOOR`] states.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use nvs_stdlib::registry::{self, CoreTy};
use regex::Regex;

/// Every `.nvst` file under `dir`, recursively, in no particular order.
fn cases(dir: &Path, into: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display()));
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            cases(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "nvst") {
            into.push(path);
        }
    }
}

/// Whether `haystack` writes `needle` as a whole name — the same match
/// `haystack.contains(needle)` performs, plus the one boundary an identifier
/// needs on its right.
///
/// A `\` is part of that boundary, not past it: `Core\Time` names
/// `Core\Time::now` and `Core\Time $t`, never `Core\Time\Duration`, which is a
/// different class with its own registry row.
fn mentions(haystack: &str, needle: &str) -> bool {
    haystack.match_indices(needle).any(|(at, _)| {
        haystack[at + needle.len()..]
            .chars()
            .next()
            .is_none_or(|next| !next.is_alphanumeric() && next != '_' && next != '\\')
    })
}

/// The `--FILE--` section of every case under `tests/conformance/`, one string
/// each.
///
/// The section alone, so a member named in a title or in an expected
/// diagnostic is not mistaken for one a case calls. Per case rather than
/// concatenated because the floor below counts *cases*, not occurrences, and
/// the two checks here read the same corpus.
fn case_sources() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance");
    let mut paths = Vec::new();
    cases(&root, &mut paths);
    assert!(
        paths.len() > 100,
        "{} holds {} cases, which is too few to be the conformance suite — \
         a check over it would pass vacuously",
        root.display(),
        paths.len()
    );
    paths
        .iter()
        .map(|path| {
            let text =
                fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
            nvs_test::case::parse(path, &text)
                .unwrap_or_else(|err| panic!("{}: {err}", path.display()))
                .file
        })
        .collect()
}

#[test]
fn every_part_one_member_has_a_conformance_case() {
    let source = case_sources().join("\n");

    let mut uncovered = BTreeSet::new();
    for class in registry::CLASSES {
        for method in class.methods {
            // The call spelling, with the open parenthesis, so `Str::trim`
            // does not answer for `Str::trimStart`. A member declaring a
            // written type parameter is spelled `decodeAs<User>(` at every
            // call site, so its own boundary is the `<`.
            let opener = if method.written().is_empty() {
                '('
            } else {
                '<'
            };
            let call = format!("{}::{}{opener}", class.name, method.name);
            if !source.contains(&call) {
                uncovered.insert(call);
            }
        }
        for method in class.instance {
            // An instance member is reached through a value, so what a case
            // writes is `->name(` and the class name appears nowhere. That is
            // weaker than the static spelling above — any receiver's `->text(`
            // answers for `Core\Regex\Match::text` — and deliberately so: the
            // alternative is inferring a receiver's type here, which would
            // mean a second checker rather than a coverage gate.
            let call = format!("->{}(", method.name);
            if !source.contains(&call) {
                uncovered.insert(format!("{}::{}", class.name, method.name));
            }
        }
        for declared in class.constants {
            // A constant has no parenthesis to bound it, so the boundary is
            // checked instead — otherwise `Core\Math::E` would be covered by
            // any case that writes `Core\Math::EPSILON`.
            let write = format!("{}::{}", class.name, declared.name);
            if !mentions(&source, &write) {
                uncovered.insert(write);
            }
        }
    }

    assert!(
        uncovered.is_empty(),
        "{} registered `Core` member(s) are never used by a conformance case: {}\n\
         Write one under tests/conformance/ — never with an `--ORACLE--` section, which \
         makes the Linux leg skip the case entirely.",
        uncovered.len(),
        uncovered
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// Which classes a case exercises, and which of their members it calls.
///
/// This is `python tools/gaps.py`'s `coverage` attribution, ported rather than
/// re-invented: that tool's docstring owns *why* a case is attributed this way
/// and is the only home for the reasoning. The floor below is the gate over
/// the same figure, so the two must agree — a member the tool ranks at two
/// cases has to fail here, or the worklist and the gate send a session in
/// different directions. Anything about the rule that changes changes in both.
struct Attribution {
    /// class -> member -> the class an instance of which that member answers.
    builds: BTreeMap<&'static str, BTreeMap<&'static str, &'static str>>,
    /// `Core\X::member`, with the qualified name and the member apart.
    qualified: Regex,
    /// `->member(` — an instance call, whose receiver's type is not written.
    arrow: Regex,
}

impl Attribution {
    fn new() -> Self {
        let mut builds: BTreeMap<&'static str, BTreeMap<&'static str, &'static str>> =
            BTreeMap::new();
        for class in registry::CLASSES {
            for method in class.methods.iter().chain(class.instance) {
                // The top-level type only. A `?Instance` is what `tryParse`
                // answers, and a case holding one has said `if ($x !== null)`
                // about it before calling anything — the tool draws the line
                // in the same place.
                if let CoreTy::Instance(made) = &method.return_ty {
                    builds
                        .entry(class.name)
                        .or_default()
                        .insert(method.name, made);
                }
            }
        }
        Self {
            builds,
            qualified: Regex::new(r"(Core(?:\\[A-Za-z][A-Za-z0-9]*)*)::([A-Za-z][A-Za-z0-9]*)")
                .expect("the qualified-call pattern"),
            arrow: Regex::new(r"->([a-z][A-Za-z0-9]*)\s*\(").expect("the instance-call pattern"),
        }
    }

    /// Every class whose values one case handles, named or not.
    ///
    /// A case names a class outright, or it holds one because something it
    /// called answers an instance of it — and then an instance call on *that*
    /// answers a third, to a fixed point. Half the registry is reached that
    /// way: `var $d = Core\Time::fromIso($text)->in($zone);` exercises three
    /// classes and spells one.
    fn holders(&self, text: &str) -> BTreeSet<&'static str> {
        let mut held: BTreeSet<&'static str> = registry::CLASSES
            .iter()
            .map(|class| class.name)
            .filter(|name| mentions(text, name))
            .collect();
        for found in self.qualified.captures_iter(text) {
            if let Some(made) = self
                .builds
                .get(&found[1])
                .and_then(|members| members.get(&found[2]))
            {
                held.insert(made);
            }
        }
        let arrows = self.arrows(text);
        let mut growing = true;
        while growing {
            growing = false;
            for name in held.iter().copied().collect::<Vec<_>>() {
                for (member, made) in self.builds.get(name).into_iter().flatten() {
                    if arrows.contains(*member) && held.insert(made) {
                        growing = true;
                    }
                }
            }
        }
        held
    }

    /// The member name of every `->member(` in one case.
    fn arrows<'a>(&self, text: &'a str) -> BTreeSet<&'a str> {
        self.arrow
            .captures_iter(text)
            .map(|found| found.get(1).expect("the captured member name").as_str())
            .collect()
    }
}

/// The members that were below the floor when the gate was written, and are
/// the whole of Stage 5 item 10's remaining worklist.
///
/// **This list may only shrink.** It is not a permission to be thin: the test
/// below fails on a member that is *not* here and below the floor, and equally
/// on one that is here and has reached it, so the list cannot go stale in
/// either direction and nothing can be added to it without deleting this
/// sentence. A gate introduced over an existing violation set either arrives
/// as a ratchet or does not arrive, and what it buys on the day it lands is
/// that there is never a twenty-seventh.
///
/// `python tools/gaps.py --coverage` ranks these by class and names each one's
/// anchor; a case that asks one of them a *second question* — not the same
/// question again — is what removes a line.
const BELOW_THE_FLOOR: &[&str] = &[
    r"Core\Attributes::all",
    r"Core\Math::atan2",
    r"Core\Program::implementing",
    r"Core\Regex::quote",
    r"Core\Router::urlAbsolute",
    r"Core\Time\TimeOfDay::compareTo",
];

/// Stage 5's item 10: no `Core` member is asked by fewer than three cases.
///
/// The floor, not the median, and per class rather than per corpus — a median
/// rises by writing a fourth case for a member that already has three, which
/// is the metric a corpus grows *around* rather than into, and a corpus total
/// rises without any member moving at all. What this forbids is a member that
/// ships with one case pinning the happy path and nothing on either side of
/// it; the four shapes a third case takes are in
/// [docs/agent/conventions.md](../../../docs/agent/conventions.md).
///
/// Three is the smallest floor that cannot be met by the happy path alone: a
/// member with three cases has been asked something it refuses, or something
/// at a boundary, or the same question its siblings were asked. Constants are
/// not counted — a constant is one value, and the sibling check above already
/// requires a case to name it.
#[test]
fn every_core_class_has_a_conformance_floor_of_three() {
    const FLOOR: usize = 3;

    let sources = case_sources();
    let rules = Attribution::new();
    let held: Vec<BTreeSet<&'static str>> =
        sources.iter().map(|text| rules.holders(text)).collect();

    let mut thin: Vec<String> = Vec::new();
    let mut closed: Vec<String> = Vec::new();
    for class in registry::CLASSES {
        let mut asked: BTreeMap<&'static str, usize> = class
            .methods
            .iter()
            .chain(class.instance)
            .map(|method| (method.name, 0))
            .collect();
        for (text, holds) in sources.iter().zip(&held) {
            if !holds.contains(class.name) {
                continue;
            }
            let mut named = rules.arrows(text);
            for found in rules.qualified.captures_iter(text) {
                if &found[1] == class.name {
                    named.insert(found.get(2).expect("the captured member name").as_str());
                }
            }
            for (member, count) in &mut asked {
                if named.contains(*member) {
                    *count += 1;
                }
            }
        }
        for (member, count) in &asked {
            let spelling = format!("{}::{member}", class.name);
            let exempt = BELOW_THE_FLOOR.contains(&spelling.as_str());
            if *count < FLOOR && !exempt {
                thin.push(format!("{spelling} is asked by {count} case(s)"));
            }
            if *count >= FLOOR && exempt {
                closed.push(spelling);
            }
        }
    }

    assert!(
        thin.is_empty(),
        "{} `Core` member(s) are asked by fewer than {FLOOR} conformance cases, so their \
         class's floor is below it:\n  {}\n\
         `python tools/gaps.py --coverage` ranks every class by the same figure and names \
         the thinnest members of each with their anchors. A case that asks one of these a \
         second question closes it; a case that asks the same question again does not. \
         `BELOW_THE_FLOOR` in this file is not where a new one goes — it only shrinks.",
        thin.len(),
        thin.join("\n  ")
    );
    assert!(
        closed.is_empty(),
        "{} member(s) listed in `BELOW_THE_FLOOR` have reached the floor of {FLOOR}. \
         Delete these lines — the list is the worklist, and a stale entry is a member \
         nobody will look at again:\n  {}",
        closed.len(),
        closed.join("\n  ")
    );
}
