//! `nvs pkg diff <old> <new>` — every change between two versions of a package's public API,
//! each classified as breaking or additive (`rule:packaging/only-a-major-may-break-the-api`).
//!
//! Each side is a package directory, whose API [`super::api::extract`] reads, or a document
//! `nvs pkg api` printed. A file of another `schema` is refused rather than misread. The walk is over
//! the two documents alone: a declaration is matched by its `name`, a member by its `name` within
//! its declaration, and a parameter by its position.
//!
//! ## The classification
//!
//! * **Breaking** — a removed declaration, member, parameter or enum case, which is also what a
//!   rename is; a lowered visibility, or a lowered write visibility on a property; a declaration of
//!   another kind; an addition a caller or an implementing class must follow: a new method or
//!   property on an interface, a new abstract method, a new parent interface, a new enum case
//!   (an exhaustive `match` no longer covers it), a new required parameter; a new restriction:
//!   `abstract`, `final` or `readonly` set, a parameter that is no longer optional; a flip either
//!   way of `static`, `variadic` or `inout`; a renamed parameter, since a call may name it.
//! * **Additive** — a new declaration, a new concrete method, constant or property on a class, a new
//!   optional parameter, a new implemented interface, a raised visibility, a dropped restriction.
//!
//! **It errs towards breaking**, as [`crate::api_diff`] does, and there is no way to suppress a
//! break. A new optional parameter is a break on
//! an interface's method and on an abstract one, because every class that writes the method must
//! add it. A key the walk does not know whose value differs, or a value of the wrong shape, is a
//! break: a newer document is never compared as if its new keys were not there.
//!
//! ## Types
//!
//! A parameter's type may widen and a method's return type may narrow; the other way is a break. A
//! property's, a constant's and an alias's type may not change at all, since a property is both
//! read and written. The comparison is over the two documents alone, never the checker's: a type
//! is split on `|`, and then on `&`, and one type covers another when each member of the second
//! is covered by a member of the first. A member covers itself, `mixed` covers anything, `never`
//! is covered by anything, `bool` covers `true` and `false`, and a class is covered by every
//! parent and interface the document lists for it, closed over theirs. A parent the document does
//! not declare is known by its name alone. The answer must hold in both documents.
//!
//! Anything else, such as a generic argument, a shape, a signature, or a class whose parents the
//! document cannot list, is undecidable. An undecidable change is a break, and its line says the
//! comparison could not tell.
//!
//! There is no cosmetic class: the document is shape alone, so every change in it is a change to
//! what a caller may write.
//!
//! ## The version step
//!
//! A break fails the command only when the step from the old version to the new one does not
//! allow it ([`Step`]). Each side's version is its directory's `package.toml` `version`, which
//! `--from` and `--to` replace; a side given as a document has none unless they give it. A
//! `0.0.x` version on either side allows anything, and so does a higher major. A higher minor or
//! patch allows no break, and neither does a new version that is not higher. A side with no version
//! allows no break, which is what a bare `nvs pkg diff` of two documents checks.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::ExitCode;

use serde_json::{Map, Value};

use super::api;
use super::manifest::{self, Version};

/// One JSON object of the document: a declaration, a member or a parameter.
type Obj = Map<String, Value>;

/// How a single change is classified. Ordered worst-first, which is the order the report prints in.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Class {
    Breaking,
    Additive,
}

impl Class {
    /// The word the report prints.
    const fn label(self) -> &'static str {
        match self {
            Self::Breaking => "breaking",
            Self::Additive => "additive",
        }
    }
}

/// One classified change.
#[derive(Debug)]
pub(crate) struct Change {
    class: Class,
    /// Where it is, in the way a caller writes it: `Shop\Cart`, `Shop\Cart::add()`,
    /// `Shop\Cart::add($count)`, `Shop\Cart::$id`, `Shop\Cart::LIMIT`.
    at: String,
    /// What changed, in a clause.
    what: String,
}

/// What the step from the old version to the new one allows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Step {
    /// A side has no version, so no break is allowed.
    Unknown,
    /// A side is `0.0.x`, so nothing is checked.
    Experimental,
    Major,
    Minor,
    Patch,
    /// The new version is the old one or lower, so no break is allowed.
    NotHigher,
}

impl Step {
    /// The step between two sides' versions.
    fn between(old: Option<Version>, new: Option<Version>) -> Self {
        let (Some(old), Some(new)) = (old, new) else {
            return Self::Unknown;
        };
        let experimental = |v: Version| v.major == 0 && v.minor == 0;
        if experimental(old) || experimental(new) {
            Self::Experimental
        } else if new <= old {
            Self::NotHigher
        } else if new.major > old.major {
            Self::Major
        } else if new.minor > old.minor {
            Self::Minor
        } else {
            Self::Patch
        }
    }

    /// Whether a breaking change passes.
    const fn allows_a_break(self) -> bool {
        matches!(self, Self::Experimental | Self::Major)
    }

    /// The word the JSON form prints.
    const fn label(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Experimental => "experimental",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Patch => "patch",
            Self::NotHigher => "not-higher",
        }
    }

    /// The report's last line: which step was read, and what it allows.
    fn sentence(self, old: Option<Version>, new: Option<Version>) -> String {
        let (Some(old), Some(new)) = (old, new) else {
            let missing = match (old, new) {
                (None, None) => "Neither version is known",
                (None, Some(_)) => "The old version is not known",
                _ => "The new version is not known",
            };
            return format!("{missing}, so no breaking change is allowed.");
        };
        match self {
            Self::Experimental => {
                format!(
                    "{old} to {new}: a 0.0.x version is experimental, so every change is allowed."
                )
            }
            Self::Major => {
                format!("{old} to {new} is a major release, so every change is allowed.")
            }
            Self::Minor => {
                format!("{old} to {new} is a minor release, so no breaking change is allowed.")
            }
            Self::Patch => {
                format!("{old} to {new} is a patch release, so no breaking change is allowed.")
            }
            Self::NotHigher | Self::Unknown => {
                format!("{new} is not higher than {old}, so no breaking change is allowed.")
            }
        }
    }
}

/// `nvs pkg diff <old> <new> [--from <version>] [--to <version>] [--json]` — the whole
/// subcommand: one line per change worst-first and the summary, or the JSON form, and a non-zero
/// exit if a change is breaking and the version step does not allow it. A side that cannot be
/// read, or whose version cannot, is a failure naming it, never an empty diff.
pub(crate) fn run(
    old: &Path,
    new: &Path,
    from: Option<Version>,
    to: Option<Version>,
    json: bool,
) -> ExitCode {
    let (Ok(from), Ok(to)) = (version(old, from), version(new, to)) else {
        return ExitCode::FAILURE;
    };
    let (Ok(old), Ok(new)) = (side(old), side(new)) else {
        return ExitCode::FAILURE;
    };
    let step = Step::between(from, to);
    let changes = diff(&old, &new);
    let count = |class: Class| changes.iter().filter(|c| c.class == class).count();
    let breaking = count(Class::Breaking);
    let allowed = breaking == 0 || step.allows_a_break();
    if json {
        let list = changes
            .iter()
            .map(|change| {
                let mut entry = Map::new();
                entry.insert("class".into(), Value::from(change.class.label()));
                entry.insert("at".into(), Value::from(change.at.as_str()));
                entry.insert("what".into(), Value::from(change.what.as_str()));
                Value::Object(entry)
            })
            .collect();
        let named = |v: Option<Version>| v.map_or(Value::Null, |v| Value::from(v.to_string()));
        let mut out = Map::new();
        out.insert("schema".into(), Value::from(api::SCHEMA));
        out.insert("from".into(), named(from));
        out.insert("to".into(), named(to));
        out.insert("step".into(), Value::from(step.label()));
        out.insert("allowed".into(), Value::from(allowed));
        out.insert("changes".into(), Value::Array(list));
        let text = serde_json::to_string_pretty(&Value::Object(out))
            .expect("a tree of strings serialises");
        println!("{text}");
    } else if changes.is_empty() {
        println!("no change");
    } else {
        for change in &changes {
            println!("{}: {} — {}", change.class.label(), change.at, change.what);
        }
        println!(
            "{} change(s): {breaking} breaking, {} additive",
            changes.len(),
            count(Class::Additive),
        );
        println!("{}", step.sentence(from, to));
    }
    if allowed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// One side's version: the one `--from` or `--to` gave, else a directory's `package.toml`'s, else
/// none. A version that cannot be read has already reported on standard error.
fn version(path: &Path, given: Option<Version>) -> Result<Option<Version>, ExitCode> {
    match given {
        Some(version) => Ok(Some(version)),
        None if path.is_dir() => manifest::version(path),
        None => Ok(None),
    }
}

/// One side's API document: a directory's extracted, or a file's read. Each way to fail has
/// already reported on standard error.
fn side(path: &Path) -> Result<Value, ExitCode> {
    if path.is_dir() {
        return api::extract(path);
    }
    let text = std::fs::read_to_string(path).map_err(|err| {
        eprintln!("cannot read {}: {err}", path.display());
        ExitCode::FAILURE
    })?;
    let document: Value = serde_json::from_str(&text).map_err(|err| {
        eprintln!("{} is not valid JSON: {err}", path.display());
        ExitCode::FAILURE
    })?;
    match document.get("schema").and_then(Value::as_u64) {
        Some(api::SCHEMA) => Ok(document),
        Some(other) => {
            eprintln!(
                "{} has API schema {other}. This version of `nvs` reads schema {}.",
                path.display(),
                api::SCHEMA
            );
            Err(ExitCode::FAILURE)
        }
        None => {
            eprintln!(
                "{} is not a file that `nvs pkg api` printed. It has no `schema`.",
                path.display()
            );
            Err(ExitCode::FAILURE)
        }
    }
}

/// Every change between two API documents, worst class first and then by location.
pub(crate) fn diff(old: &Value, new: &Value) -> Vec<Change> {
    let mut out = Vec::new();
    let lineage = Lineage {
        old: parents(old),
        new: parents(new),
    };
    match (
        by_name(old.get("declarations")),
        by_name(new.get("declarations")),
    ) {
        (Some(old), Some(new)) => matched(
            &old,
            &new,
            |name, _| name.to_owned(),
            |at, decl| breaking(at, &format!("{} removed", kind_word(decl))),
            |at, decl| additive(at, &format!("new {}", kind_word(decl))),
            |at, old, new, out| declaration(at, old, new, &lineage, out),
            &mut out,
        ),
        _ => out.push(unreadable("", "declarations")),
    }
    rest(
        "",
        obj(old),
        obj(new),
        &["schema", "declarations"],
        &mut out,
    );
    out.sort_by(|a, b| (a.class, &a.at, &a.what).cmp(&(b.class, &b.at, &b.what)));
    out
}

/// The keys each kind of declaration has, which [`rest`] does not compare again.
const CLASS_KEYS: &[&str] = &[
    "name",
    "kind",
    "abstract",
    "final",
    "readonly",
    "extends",
    "implements",
    "constants",
    "properties",
    "methods",
];
const INTERFACE_KEYS: &[&str] = &[
    "name",
    "kind",
    "extends",
    "constants",
    "properties",
    "methods",
];
const ENUM_KEYS: &[&str] = &[
    "name",
    "kind",
    "cases",
    "backing",
    "implements",
    "constants",
    "properties",
    "methods",
];
const ALIAS_KEYS: &[&str] = &["name", "kind", "type"];
const CONSTANT_KEYS: &[&str] = &["name", "visibility", "type", "final"];
const PROPERTY_KEYS: &[&str] = &["name", "visibility", "set", "type", "static", "readonly"];
const METHOD_KEYS: &[&str] = &[
    "name",
    "visibility",
    "static",
    "abstract",
    "final",
    "params",
    "returns",
];
const PARAM_KEYS: &[&str] = &["name", "type", "optional", "variadic", "inout"];

/// One declaration present on both sides.
fn declaration(at: &str, old: &Obj, new: &Obj, lineage: &Lineage<'_>, out: &mut Vec<Change>) {
    let kind = old.get("kind").and_then(Value::as_str);
    if kind != new.get("kind").and_then(Value::as_str) {
        out.push(breaking(
            at,
            &format!("was {} and is now {}", kind_word(old), kind_word(new)),
        ));
        return;
    }
    let keys = match kind {
        Some("class") => {
            flag(
                at,
                "abstract",
                old,
                new,
                Class::Breaking,
                Class::Additive,
                out,
            );
            flag(at, "final", old, new, Class::Breaking, Class::Additive, out);
            flag(
                at,
                "readonly",
                old,
                new,
                Class::Breaking,
                Class::Additive,
                out,
            );
            same(at, "extends", old, new, out);
            names(at, "implements", old, new, Class::Additive, out);
            CLASS_KEYS
        }
        // A new parent interface brings methods every implementing class must write.
        Some("interface") => {
            names(at, "extends", old, new, Class::Breaking, out);
            INTERFACE_KEYS
        }
        Some("enum") => {
            same(at, "backing", old, new, out);
            names(at, "implements", old, new, Class::Additive, out);
            cases(at, old, new, out);
            ENUM_KEYS
        }
        Some("type") => {
            same(at, "type", old, new, out);
            ALIAS_KEYS
        }
        _ => {
            out.push(unreadable(at, "kind"));
            return;
        }
    };
    if kind != Some("type") {
        let interface = kind == Some("interface");
        members(at, "constants", old, new, out, |at, old, new, out| {
            visibility(at, "visibility", old, new, out);
            same(at, "type", old, new, out);
            flag(at, "final", old, new, Class::Breaking, Class::Additive, out);
            rest(at, Some(old), Some(new), CONSTANT_KEYS, out);
        });
        members(at, "properties", old, new, out, |at, old, new, out| {
            visibility(at, "visibility", old, new, out);
            visibility(at, "set", old, new, out);
            same(at, "type", old, new, out);
            flag(
                at,
                "static",
                old,
                new,
                Class::Breaking,
                Class::Breaking,
                out,
            );
            flag(
                at,
                "readonly",
                old,
                new,
                Class::Breaking,
                Class::Additive,
                out,
            );
            rest(at, Some(old), Some(new), PROPERTY_KEYS, out);
        });
        members(at, "methods", old, new, out, |at, old, new, out| {
            method(at, old, new, interface, lineage, out);
        });
    }
    rest(at, Some(old), Some(new), keys, out);
}

/// One method present on both sides. `interface` is whether it is an interface's.
fn method(
    at: &str,
    old: &Obj,
    new: &Obj,
    interface: bool,
    lineage: &Lineage<'_>,
    out: &mut Vec<Change>,
) {
    visibility(at, "visibility", old, new, out);
    flag(
        at,
        "static",
        old,
        new,
        Class::Breaking,
        Class::Breaking,
        out,
    );
    flag(
        at,
        "abstract",
        old,
        new,
        Class::Breaking,
        Class::Additive,
        out,
    );
    flag(at, "final", old, new, Class::Breaking, Class::Additive, out);
    directed(at, "returns", old, new, Side::Returned, lineage, out);
    let implemented = interface || is_set(old, "abstract") || is_set(new, "abstract");
    params(at, old, new, implemented, lineage, out);
    rest(at, Some(old), Some(new), METHOD_KEYS, out);
}

/// A method's parameters, matched by position. `implemented` is whether a class other than the
/// declaring one writes the method, so that a new optional parameter is one it must add.
fn params(
    at: &str,
    old: &Obj,
    new: &Obj,
    implemented: bool,
    lineage: &Lineage<'_>,
    out: &mut Vec<Change>,
) {
    let (Some(old), Some(new)) = (objects(old.get("params")), objects(new.get("params"))) else {
        out.push(unreadable(at, "params"));
        return;
    };
    let base = at.strip_suffix("()").unwrap_or(at);
    for index in 0..old.len().max(new.len()) {
        match (old.get(index), new.get(index)) {
            (Some(old), Some(new)) => {
                let old_name = name(old).unwrap_or("?");
                let at = format!("{base}({old_name})");
                if Some(old_name) != name(new) {
                    out.push(breaking(
                        &at,
                        &format!(
                            "parameter is renamed `{}`, and a call may name it",
                            name(new).unwrap_or("?")
                        ),
                    ));
                }
                directed(&at, "type", old, new, Side::Passed, lineage, out);
                flag(
                    &at,
                    "optional",
                    old,
                    new,
                    Class::Additive,
                    Class::Breaking,
                    out,
                );
                flag(
                    &at,
                    "variadic",
                    old,
                    new,
                    Class::Breaking,
                    Class::Breaking,
                    out,
                );
                flag(
                    &at,
                    "inout",
                    old,
                    new,
                    Class::Breaking,
                    Class::Breaking,
                    out,
                );
                rest(&at, Some(old), Some(new), PARAM_KEYS, out);
            }
            (Some(old), None) => out.push(breaking(
                at,
                &format!("parameter `{}` removed", name(old).unwrap_or("?")),
            )),
            (None, Some(new)) => {
                let param = name(new).unwrap_or("?");
                let change = if !(is_set(new, "optional") || is_set(new, "variadic")) {
                    breaking(at, &format!("new required parameter `{param}`"))
                } else if implemented {
                    breaking(
                        at,
                        &format!(
                            "new optional parameter `{param}`, which every class that writes this method must add"
                        ),
                    )
                } else {
                    additive(at, &format!("new optional parameter `{param}`"))
                };
                out.push(change);
            }
            (None, None) => {}
        }
    }
}

/// An enum's cases: a removed one breaks a caller that names it, a new one breaks an exhaustive
/// `match`.
fn cases(at: &str, old: &Obj, new: &Obj, out: &mut Vec<Change>) {
    let (Some(old), Some(new)) = (strings(old.get("cases")), strings(new.get("cases"))) else {
        out.push(unreadable(at, "cases"));
        return;
    };
    for case in old.iter().filter(|case| !new.contains(case)) {
        out.push(breaking(at, &format!("case `{case}` removed")));
    }
    for case in new.iter().filter(|case| !old.contains(case)) {
        out.push(breaking(
            at,
            &format!("new case `{case}`, which a `match` over every case does not cover"),
        ));
    }
}

/// One member list, `constants`, `properties` or `methods`: a removed member is a break, a new one
/// as [`new_member`] says, and `pair` compares the
/// members on both sides.
fn members(
    at: &str,
    key: &str,
    old: &Obj,
    new: &Obj,
    out: &mut Vec<Change>,
    pair: impl Fn(&str, &Obj, &Obj, &mut Vec<Change>),
) {
    let (Some(old_list), Some(new_list)) = (by_name(old.get(key)), by_name(new.get(key))) else {
        out.push(unreadable(at, key));
        return;
    };
    let interface = new.get("kind").and_then(Value::as_str) == Some("interface");
    matched(
        &old_list,
        &new_list,
        |name, _| member_at(at, key, name),
        |at, _| breaking(at, &format!("{} removed", member_word(key))),
        |at, member| new_member(at, key, member, interface),
        pair,
        out,
    );
}

/// A member only the new side has. On an interface, a new method or property is one more thing
/// every implementing class must write.
fn new_member(at: &str, key: &str, member: &Obj, interface: bool) -> Change {
    if interface && key != "constants" {
        return breaking(
            at,
            &format!(
                "new {} on an interface, which every implementing class must write",
                member_word(key)
            ),
        );
    }
    if key == "methods" && is_set(member, "abstract") {
        return breaking(at, "new abstract method");
    }
    additive(at, &format!("new {}", member_word(key)))
}

/// Walks two lists keyed by name: `removed` for a name only the old side has, `added` for one
/// only the new side has, `pair` for one both have. `place` writes where each is.
fn matched(
    old: &BTreeMap<&str, &Obj>,
    new: &BTreeMap<&str, &Obj>,
    place: impl Fn(&str, &Obj) -> String,
    removed: impl Fn(&str, &Obj) -> Change,
    added: impl Fn(&str, &Obj) -> Change,
    pair: impl Fn(&str, &Obj, &Obj, &mut Vec<Change>),
    out: &mut Vec<Change>,
) {
    for (name, old_item) in old {
        let at = place(name, old_item);
        match new.get(name) {
            Some(new_item) => pair(&at, old_item, new_item, out),
            None => out.push(removed(&at, old_item)),
        }
    }
    for (name, new_item) in new.iter().filter(|(name, _)| !old.contains_key(*name)) {
        out.push(added(&place(name, new_item), new_item));
    }
}

/// A flag on both sides. Setting it is `set`, clearing it is `cleared`, and a value that is not a
/// flag is a break.
fn flag(
    at: &str,
    key: &str,
    old: &Obj,
    new: &Obj,
    set: Class,
    cleared: Class,
    out: &mut Vec<Change>,
) {
    match (old.get(key), new.get(key)) {
        (before, after) if before == after => {}
        (Some(Value::Bool(false)), Some(Value::Bool(true))) => out.push(Change {
            class: set,
            at: at.to_owned(),
            what: format!("is now `{key}`"),
        }),
        (Some(Value::Bool(true)), Some(Value::Bool(false))) => out.push(Change {
            class: cleared,
            at: at.to_owned(),
            what: format!("is no longer `{key}`"),
        }),
        _ => out.push(unreadable(at, key)),
    }
}

/// A visibility on both sides: lowered is a break, raised additive.
fn visibility(at: &str, key: &str, old: &Obj, new: &Obj, out: &mut Vec<Change>) {
    let rank = |value: Option<&Value>| match value.and_then(Value::as_str) {
        Some("public") => Some(2),
        Some("protected") => Some(1),
        Some("private") => Some(0),
        _ => None,
    };
    let (before, after) = (old.get(key), new.get(key));
    if before == after {
        return;
    }
    let what = |class| Change {
        class,
        at: at.to_owned(),
        what: format!("`{key}` changed from {} to {}", show(before), show(after)),
    };
    match (rank(before), rank(after)) {
        (Some(b), Some(a)) if a < b => out.push(what(Class::Breaking)),
        (Some(_), Some(_)) => out.push(what(Class::Additive)),
        _ => out.push(unreadable(at, key)),
    }
}

/// A value that must not change: any change is a break.
fn same(at: &str, key: &str, old: &Obj, new: &Obj, out: &mut Vec<Change>) {
    let (before, after) = (old.get(key), new.get(key));
    if before != after {
        out.push(breaking(
            at,
            &format!("`{key}` changed from {} to {}", show(before), show(after)),
        ));
    }
}

/// Which side of a call a type is on.
#[derive(Clone, Copy)]
enum Side {
    /// A parameter's: the caller writes the value, so the type may widen.
    Passed,
    /// A return's: the caller reads the value, so the type may narrow.
    Returned,
}

/// A type on both sides, classified by the way it moved: widening a parameter and narrowing a
/// return are additive, and every other change, an undecidable one included, is a break.
fn directed(
    at: &str,
    key: &str,
    old: &Obj,
    new: &Obj,
    side: Side,
    lineage: &Lineage<'_>,
    out: &mut Vec<Change>,
) {
    let (before, after) = (old.get(key), new.get(key));
    if before == after {
        return;
    }
    let changed = format!("`{key}` changed from {} to {}", show(before), show(after));
    let (wide, narrow) = match side {
        Side::Passed => (after, before),
        Side::Returned => (before, after),
    };
    let covered = match (wide.and_then(Value::as_str), narrow.and_then(Value::as_str)) {
        (Some(wide), Some(narrow)) => lineage.covers(wide, narrow),
        _ => None,
    };
    out.push(match (covered, side) {
        (Some(true), Side::Passed) => additive(at, &format!("{changed}, which is wider")),
        (Some(true), Side::Returned) => additive(at, &format!("{changed}, which is narrower")),
        (Some(false), Side::Passed) => breaking(
            at,
            &format!("{changed}, which no longer accepts every value it did"),
        ),
        (Some(false), Side::Returned) => breaking(
            at,
            &format!("{changed}, which may return a value it did not before"),
        ),
        (None, _) => breaking(
            at,
            &format!("{changed}, and the comparison cannot tell which way it moved"),
        ),
    });
}

/// Each side's declarations, by name, with the parents each lists: a class's `extends` and
/// `implements`, an interface's `extends`, an enum's `implements`.
struct Lineage<'a> {
    old: BTreeMap<&'a str, Vec<&'a str>>,
    new: BTreeMap<&'a str, Vec<&'a str>>,
}

impl Lineage<'_> {
    /// Whether every value of `narrow` is a value of `wide`, in both documents: `None` when either
    /// cannot tell.
    fn covers(&self, wide: &str, narrow: &str) -> Option<bool> {
        if wide == narrow {
            return Some(true);
        }
        // A signature's return type is written after its `)` unbracketed, so its `|` is not one
        // the split can tell from the outer union's.
        if wide.contains("callable(") || narrow.contains("callable(") {
            return None;
        }
        match (
            covers(wide, narrow, &self.old),
            covers(wide, narrow, &self.new),
        ) {
            (Some(true), Some(true)) => Some(true),
            (Some(false), _) | (_, Some(false)) => Some(false),
            _ => None,
        }
    }
}

/// Every declaration's name in `document` and the parents it lists.
fn parents(document: &Value) -> BTreeMap<&str, Vec<&str>> {
    let mut map = BTreeMap::new();
    for decl in objects(document.get("declarations")).unwrap_or_default() {
        let Some(name) = name(decl) else { continue };
        let mut listed = Vec::new();
        for key in ["extends", "implements"] {
            match decl.get(key) {
                Some(Value::String(parent)) => listed.push(parent.as_str()),
                Some(Value::Array(list)) => listed.extend(list.iter().filter_map(Value::as_str)),
                _ => {}
            }
        }
        map.insert(name, listed);
    }
    map
}

/// Whether type `wide` covers type `narrow` over one document's `parents`: each member of `narrow`
/// covered by some member of `wide`.
fn covers(wide: &str, narrow: &str, parents: &BTreeMap<&str, Vec<&str>>) -> Option<bool> {
    let wide = split(wide, '|');
    all(split(narrow, '|').into_iter().map(|member| {
        any(wide
            .iter()
            .map(|option| covers_member(option, member, parents)))
    }))
}

/// One union member against another. An intersection `A&B` is covered when one of its parts is,
/// and covers when each of its parts does.
fn covers_member(wide: &str, narrow: &str, parents: &BTreeMap<&str, Vec<&str>>) -> Option<bool> {
    if wide == narrow || wide == "mixed" || narrow == "never" {
        return Some(true);
    }
    if wide.contains('&') {
        return all(split(wide, '&')
            .into_iter()
            .map(|part| covers_member(part, narrow, parents)));
    }
    if narrow.contains('&') {
        // A value of `A&B` is also a value of a type neither part covers, `A&B` itself, so a
        // miss is undecidable.
        return match any(split(narrow, '&')
            .into_iter()
            .map(|part| covers_member(wide, part, parents)))
        {
            Some(true) => Some(true),
            _ => None,
        };
    }
    match (atom(wide), atom(narrow)) {
        (Atom::Other, _) | (_, Atom::Other) => None,
        (Atom::Keyword, Atom::Keyword | Atom::Literal) => {
            if wide == "bool" && matches!(narrow, "true" | "false") {
                return Some(true);
            }
            // These hold values of other kinds, which the document does not list.
            if matches!(wide, "iterable" | "object" | "callable" | "static") {
                return None;
            }
            Some(false)
        }
        (Atom::Keyword, Atom::Class) => match wide {
            "object" | "iterable" | "callable" => None,
            _ => Some(false),
        },
        (Atom::Class | Atom::Literal, Atom::Keyword | Atom::Literal) => {
            if narrow == "static" {
                None
            } else {
                Some(false)
            }
        }
        (Atom::Literal, Atom::Class) => Some(false),
        (Atom::Class, Atom::Class) => {
            let (ancestors, complete) = ancestors(narrow, parents)?;
            if ancestors.contains(wide) {
                Some(true)
            } else if complete {
                Some(false)
            } else {
                None
            }
        }
    }
}

/// What one union member is, as far as the comparison reads it.
#[derive(Clone, Copy)]
enum Atom {
    /// A built-in type's keyword: `int`, `null`, `tainted string`.
    Keyword,
    /// One value written as a type: `'draft'`, `3`.
    Literal,
    /// A class, an interface or an enum, by its fully qualified name.
    Class,
    /// Anything with structure: a generic argument, a shape, a member, a group.
    Other,
}

fn atom(member: &str) -> Atom {
    const KEYWORDS: &[&str] = &[
        "null", "bool", "int", "uint", "float", "decimal", "string", "bytes", "object", "mixed",
        "void", "never", "true", "false", "iterable", "callable", "static", "array",
    ];
    let plain = member
        .trim_start_matches("secret ")
        .trim_start_matches("tainted ");
    if KEYWORDS.contains(&plain) {
        Atom::Keyword
    } else if member.starts_with(['\'', '"', '-'])
        || member.starts_with(|c: char| c.is_ascii_digit())
    {
        Atom::Literal
    } else if member
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '\\')
    {
        Atom::Class
    } else {
        Atom::Other
    }
}

/// Every parent `name` has, closed over the parents of each: `None` when the document does not
/// declare `name`, and whether the set is complete, which it is not when a parent is one the
/// document does not declare.
fn ancestors<'a>(
    name: &'a str,
    parents: &BTreeMap<&'a str, Vec<&'a str>>,
) -> Option<(BTreeSet<&'a str>, bool)> {
    let mut stack = parents.get(name)?.clone();
    let mut seen = BTreeSet::new();
    let mut complete = true;
    while let Some(parent) = stack.pop() {
        if !seen.insert(parent) {
            continue;
        }
        match parents.get(parent) {
            Some(more) => stack.extend(more),
            None => complete = false,
        }
    }
    Some((seen, complete))
}

/// `text` split on `separator` where it is not inside brackets or quotes.
fn split(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut depth, mut quote, mut start) = (0_i32, None, 0);
    for (index, c) in text.char_indices() {
        match (quote, c) {
            (Some(open), _) if c == open => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') => quote = Some(c),
            (None, '<' | '(' | '{' | '[') => depth += 1,
            (None, '>' | ')' | '}' | ']') => depth -= 1,
            (None, _) if c == separator && depth == 0 => {
                parts.push(&text[start..index]);
                start = index + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

/// Every answer `true`: `false` if one is, else `None` if one cannot tell.
fn all(answers: impl Iterator<Item = Option<bool>>) -> Option<bool> {
    let mut result = Some(true);
    for answer in answers {
        match answer {
            Some(false) => return Some(false),
            None => result = None,
            Some(true) => {}
        }
    }
    result
}

/// Some answer `true`: `true` if one is, else `None` if one cannot tell.
fn any(answers: impl Iterator<Item = Option<bool>>) -> Option<bool> {
    let mut result = Some(false);
    for answer in answers {
        match answer {
            Some(true) => return Some(true),
            None => result = None,
            Some(false) => {}
        }
    }
    result
}

/// A list of names, `implements` or an interface's `extends`: a removed one is a break, a new one
/// is `added`.
fn names(at: &str, key: &str, old: &Obj, new: &Obj, added: Class, out: &mut Vec<Change>) {
    let (Some(old), Some(new)) = (strings(old.get(key)), strings(new.get(key))) else {
        out.push(unreadable(at, key));
        return;
    };
    for name in old.iter().filter(|name| !new.contains(name)) {
        out.push(breaking(at, &format!("no longer `{key}` `{name}`")));
    }
    for name in new.iter().filter(|name| !old.contains(name)) {
        out.push(Change {
            class: added,
            at: at.to_owned(),
            what: format!("now `{key}` `{name}`"),
        });
    }
}

/// Every key neither side's walk compared: one whose value differs is a break.
fn rest(at: &str, old: Option<&Obj>, new: Option<&Obj>, known: &[&str], out: &mut Vec<Change>) {
    let empty = Obj::new();
    let (old, new) = (old.unwrap_or(&empty), new.unwrap_or(&empty));
    let mut keys: Vec<&String> = old.keys().chain(new.keys()).collect();
    keys.sort();
    keys.dedup();
    for key in keys
        .into_iter()
        .filter(|key| !known.contains(&key.as_str()))
    {
        same(at, key, old, new, out);
    }
}

/// Where a member is, as a caller writes it.
fn member_at(owner: &str, key: &str, name: &str) -> String {
    match key {
        "methods" => format!("{owner}::{name}()"),
        _ => format!("{owner}::{name}"),
    }
}

/// The word for one entry of a member list.
fn member_word(key: &str) -> &'static str {
    match key {
        "constants" => "constant",
        "properties" => "property",
        _ => "method",
    }
}

/// The word for a declaration's kind.
fn kind_word(decl: &Obj) -> &str {
    match decl.get("kind").and_then(Value::as_str) {
        Some("type") => "type alias",
        Some(kind) => kind,
        None => "declaration",
    }
}

/// A value as the report shows it: a string in backticks, nothing as `none`.
fn show(value: Option<&Value>) -> String {
    match value {
        None => "none".to_owned(),
        Some(Value::String(text)) => format!("`{text}`"),
        Some(other) => format!("`{other}`"),
    }
}

/// A list of objects keyed by their `name`, or `None` when it is not one.
fn by_name(list: Option<&Value>) -> Option<BTreeMap<&str, &Obj>> {
    objects(list)?
        .into_iter()
        .map(|item| Some((name(item)?, item)))
        .collect()
}

/// A list of objects, or `None` when it is not one.
fn objects(list: Option<&Value>) -> Option<Vec<&Obj>> {
    list?.as_array()?.iter().map(Value::as_object).collect()
}

/// A list of strings, or `None` when it is not one.
fn strings(list: Option<&Value>) -> Option<Vec<&str>> {
    list?.as_array()?.iter().map(Value::as_str).collect()
}

fn obj(value: &Value) -> Option<&Obj> {
    value.as_object()
}

fn name(item: &Obj) -> Option<&str> {
    item.get("name").and_then(Value::as_str)
}

fn is_set(item: &Obj, key: &str) -> bool {
    item.get(key) == Some(&Value::Bool(true))
}

/// A change the walk could not compare, because a value is missing or of the wrong shape.
fn unreadable(at: &str, key: &str) -> Change {
    breaking(at, &format!("`{key}` cannot be compared"))
}

fn breaking(at: &str, what: &str) -> Change {
    Change {
        class: Class::Breaking,
        at: at.to_owned(),
        what: what.to_owned(),
    }
}

fn additive(at: &str, what: &str) -> Change {
    Change {
        class: Class::Additive,
        at: at.to_owned(),
        what: what.to_owned(),
    }
}
