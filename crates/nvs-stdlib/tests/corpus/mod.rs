//! The conformance corpus, as every gate over it reads it: the `.nvst` cases
//! under `tests/conformance/`, and the one rule that says which members a case
//! asks of which classes.
//!
//! This is a module rather than a section of one of the two test files because
//! Cargo compiles every `tests/*.rs` as its own crate, and both of them ask the
//! corpus the same question from opposite ends —
//! `conformance_coverage.rs` walks [`registry::CLASSES`] and asks the
//! repository for a case, `spec_registry_coverage.rs` walks the spec and asks
//! it for the same thing. A rule that lived in one of them would have to be
//! restated in the other, and two gates that disagree about what "a case calls
//! this member" means are worse than one gate.
//!
//! [`Attribution::asked`] is that rule's only home. Everything about *why* a
//! case is attributed to a class it never names is in `tools/nv/cmd/gaps.ts`'s
//! own header comment, which this is a port of rather than a second design — the
//! tool ranks the worklist and these gates close it, so the two must agree
//! member for member.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use nvs_stdlib::registry::{self, CoreTy};
use regex::Regex;

/// Every `.nvst` file under `dir`, recursively, in no particular order.
pub(crate) fn cases(dir: &Path, into: &mut Vec<PathBuf>) {
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
pub(crate) fn mentions(haystack: &str, needle: &str) -> bool {
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
/// concatenated because the gates over this count *cases*, not occurrences,
/// and every one of them reads the same corpus.
pub(crate) fn sources() -> Vec<String> {
    let root = nvs_repo::path("tests/conformance");
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

/// Which classes a case exercises, and which of their members it calls.
///
/// This is `tools/nv/cmd/gaps.ts`'s `coverage` attribution, ported rather than
/// re-invented: that command's header comment owns *why* a case is attributed this way
/// and is the only home for the reasoning. The gates over the same figure have
/// to agree with it — a member the tool ranks at two cases has to fail a floor
/// of three, or the worklist and the gate send a session in different
/// directions. Anything about the rule that changes changes in both.
pub(crate) struct Attribution {
    /// class -> member -> the class an instance of which that member answers.
    builds: BTreeMap<&'static str, BTreeMap<&'static str, &'static str>>,
    /// `Core\X::member`, with the qualified name and the member apart.
    qualified: Regex,
    /// `->member(` — an instance call, whose receiver's type is not written.
    arrow: Regex,
}

impl Attribution {
    pub(crate) fn new() -> Self {
        let mut builds: BTreeMap<&'static str, BTreeMap<&'static str, &'static str>> =
            BTreeMap::new();
        for class in registry::CLASSES {
            for method in class.methods.iter().chain(class.instance) {
                // The top-level type only. A `?Instance` is what `tryParse`
                // answers, and a case holding one has said `if ($x !== null)`
                // about it before calling anything — the tool draws the line
                // in the same place.
                // A generic class at written arguments counts the same: what
                // `Core\Db\Connection::query` answers is a `Core\Db\Rows`
                // whether or not the row spells its `T`, and reading only the
                // bare variant would drop every case that reaches a result set
                // without naming the class.
                let made = match &method.return_ty {
                    CoreTy::Instance(made) | CoreTy::InstanceAt(made, _) => Some(made),
                    _ => None,
                };
                if let Some(made) = made {
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
            // The optional `<...>` is `Core\Db\Queryable::queryAs<T>`'s: a
            // member declaring a written type parameter has no spelling
            // without one, so an arrow that stopped at the `<` would read
            // every case of it as asking nothing.
            arrow: Regex::new(r"->([a-z][A-Za-z0-9]*)\s*(?:<[^<>()\n]*>\s*)?\(")
                .expect("the instance-call pattern"),
        }
    }

    /// Every class whose values one case handles, named or not.
    ///
    /// A case names a class outright, or it holds one because something it
    /// called answers an instance of it — and then an instance call on *that*
    /// answers a third, to a fixed point. Half the registry is reached that
    /// way: `var $d = Core\Time::fromIso($text)->in($zone);` exercises three
    /// classes and spells one.
    pub(crate) fn holders(&self, text: &str) -> BTreeSet<&'static str> {
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

    /// What one case asks, indexed the way both gates read it: every class the
    /// case holds, mapped to every member name it asks of that class.
    ///
    /// The static half is exact — a written `Core\X::member` names its own
    /// class. The instance half is not, and deliberately: an `->text(` is
    /// attributed to *every* class the case holds, because inferring a
    /// receiver's type here would mean a second type checker rather than a
    /// coverage gate. What keeps that from attributing a member to a class the
    /// case never touches is [`Self::holders`], which is the whole reason the
    /// index is per class rather than a flat set of names.
    pub(crate) fn asked<'a>(&self, text: &'a str) -> BTreeMap<&'static str, BTreeSet<&'a str>> {
        let arrows = self.arrows(text);
        let mut index: BTreeMap<&'static str, BTreeSet<&'a str>> = self
            .holders(text)
            .into_iter()
            .map(|class| (class, arrows.clone()))
            .collect();
        for found in self.qualified.captures_iter(text) {
            let class = found.get(1).expect("the captured class name").as_str();
            let member = found.get(2).expect("the captured member name").as_str();
            if let Some(named) = index.get_mut(class) {
                named.insert(member);
            }
        }
        index
    }

    /// The member name of every `->member(` in one case.
    fn arrows<'a>(&self, text: &'a str) -> BTreeSet<&'a str> {
        self.arrow
            .captures_iter(text)
            .map(|found| found.get(1).expect("the captured member name").as_str())
            .collect()
    }
}
