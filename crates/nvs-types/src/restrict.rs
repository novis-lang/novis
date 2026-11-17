//! `rule:packaging/a-restriction-limits-where-a-declaration-is-used`: each use of a restricted
//! declaration is checked against every `[[restrict]]` entry on it, and a use outside every
//! allowed place is [`code::E_RESTRICTED_USE`].
//!
//! **The tables are read by `nvs_config::restrict`** and reach this crate through
//! [`nvs_config::grants::Grants::restrictions`], the application's entries first and then each
//! loaded package's. [`build`] indexes them once per check, after the signatures are built, and
//! checks the part of each entry only the program can answer: a `Core` target that names no `Core`
//! declaration and a `use` that names an override are [`code::E_BAD_RESTRICTION`], and an
//! application target that names nothing of this program is [`code::W_RESTRICTION_NAMES_NOTHING`].
//! A program with no entry gets `None`, and nothing below runs.
//!
//! **The reader is the typed tree.** [`check_uses`] runs when a body's check ends, in the same
//! [`crate::deprecated::window`] that warns about a deprecated use, and maps each expression table
//! entry the body recorded to the declarations it names
//! ([`crate::deprecated::resolved`]). So a call, a method reference, `new`, a property, a class
//! constant and an enum case are each a use, and a type position, `instanceof` and `catch` record
//! no such entry and are not. No side list is kept.
//!
//! **Which entries hold a use.** A use of a member is held by an entry on the member, on its class
//! and on the class's namespace (`'Ns'` or a `'Ns\*'` subtree above it). A use of a method is held
//! too by an entry on each ancestor's method of the same name, so an override and an
//! implementation are restricted with the method they replace. Every entry that holds a use must
//! allow it: nothing lifts an entry.
//!
//! **Where a use is.** Its place is the namespace it is written in, the class around it and the
//! method around it. An `allow` entry matches the namespace (exactly, or a subtree), the class, or
//! the `Class::method`. A use inside the restricted declaration itself, its own class or its own
//! namespace, is always allowed, so restricted code can call itself. A package's entry matches only
//! where the using file is inside that package by layout ([`crate::package`]), so a file that
//! writes the package's namespace from outside its folder is still refused.
//!
//! Cost: one pass over each body's new table entries, and a few hash lookups per entry, only when
//! a table exists. Memory: one index of the program's entries per check, never per request.

use nvs_config::restrict::{Restriction, Restrictions, Target};
use nvs_diagnostics::{Diagnostic, Diagnostics, SourceId, Span, code};
use nvs_hir::{ClassGraph, QName};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::Env;
use crate::deprecated::Member;
use crate::signatures::{SignatureTable, resolve_const_owned, resolve_method_ref};

/// The program's restrictions, indexed by the declaration each one names.
pub(crate) struct Restricted<'g> {
    entries: &'g [Restriction],
    /// `'Ns\Name'` entries, by the name: a class's or a namespace's.
    names: FxHashMap<&'g str, Vec<usize>>,
    /// `'Ns\Name::member'` entries, by the class and the member.
    members: FxHashMap<(&'g str, &'g str), Vec<usize>>,
    /// `'Ns\*'` entries, by the root.
    subtrees: Vec<(&'g str, usize)>,
}

/// Where one checked body is: its namespace, and the class and member around it.
#[derive(Clone, Copy)]
pub(crate) struct Place<'p> {
    pub namespace: &'p [String],
    pub class: Option<&'p QName>,
    pub member: Option<&'p Member>,
}

/// The index of `restrictions`, or `None` when there is no entry. Each entry's target is checked
/// against the program here, and what is wrong goes to `diags`.
pub(crate) fn build<'g>(
    restrictions: &'g Restrictions,
    signatures: &SignatureTable,
    graph: &ClassGraph,
    diags: &mut Diagnostics,
) -> Option<Restricted<'g>> {
    if restrictions.is_empty() {
        return None;
    }
    if let Err(diagnostic) =
        restrictions.check_core(|target| names_something(target, signatures, graph))
    {
        diags.report(diagnostic);
    }
    let entries = restrictions.entries();
    let mut out = Restricted {
        entries,
        names: FxHashMap::default(),
        members: FxHashMap::default(),
        subtrees: Vec::new(),
    };
    for (at, entry) in entries.iter().enumerate() {
        let target = entry.target();
        if let Target::Member { class, member } = target {
            report_override(entry, class, member, signatures, graph, diags);
        }
        if entry.package().is_none()
            && !target.is_core()
            && !names_something(target, signatures, graph)
        {
            let mut warning = Diagnostic::warning(
                code::W_RESTRICTION_NAMES_NOTHING,
                format!("`{target}` is not a declaration of this program"),
            )
            .with_help("check the name for a typo".to_owned());
            if let Some(span) = entry.span() {
                warning = warning.with_primary(span, "this name");
            }
            diags.report(warning);
        }
        match target {
            Target::Name(name) => out.names.entry(name).or_default().push(at),
            Target::Member { class, member } => {
                out.members.entry((class, member)).or_default().push(at);
            }
            Target::Subtree(root) => out.subtrees.push((root, at)),
        }
    }
    Some(out)
}

/// Whether `target` names a class, a member of one, or a namespace that declares one.
fn names_something(target: &Target, signatures: &SignatureTable, graph: &ClassGraph) -> bool {
    match target {
        Target::Name(name) => {
            signatures.get(&QName::parse(name)).is_some()
                || signatures
                    .iter()
                    .any(|(class, _)| namespace_of(class).is_some_and(|ns| under(&ns, name)))
        }
        Target::Subtree(root) => signatures
            .iter()
            .any(|(class, _)| namespace_of(class).is_some_and(|ns| under(&ns, root))),
        Target::Member { class, member } => {
            let qname = QName::parse(class);
            let Some(sig) = signatures.get(&qname) else {
                return false;
            };
            sig.methods.contains_key(member.as_str())
                || sig.properties.contains_key(member.as_str())
                || sig.static_properties.iter().any(|(name, _)| name == member)
                || resolve_const_owned(&qname, member, signatures, graph).is_some()
        }
    }
}

/// [`code::E_BAD_RESTRICTION`] when the method `class::member` overrides or implements a method
/// of an ancestor: the entry has to name that method instead.
fn report_override(
    entry: &Restriction,
    class: &str,
    member: &str,
    signatures: &SignatureTable,
    graph: &ClassGraph,
    diags: &mut Diagnostics,
) {
    let qname = QName::parse(class);
    let declares = |name: &QName| {
        signatures
            .get(name)
            .is_some_and(|sig| sig.methods.contains_key(member))
    };
    if member == "constructor" || !declares(&qname) {
        return;
    }
    let Some(parent) = ancestors(&qname, graph)
        .into_iter()
        .find(|ancestor| declares(ancestor))
    else {
        return;
    };
    let mut diagnostic = Diagnostic::error(
        code::E_BAD_RESTRICTION,
        format!(
            "`{class}::{member}` replaces `{parent}::{member}`, so it cannot be restricted alone"
        ),
    )
    .with_help(format!("restrict `{parent}::{member}` instead"));
    if let Some(span) = entry.span() {
        diagnostic = diagnostic.with_primary(span, "this method overrides another one");
    }
    diags.report(diagnostic);
}

/// Every class and interface `class` extends or implements, at any depth, without `class`.
fn ancestors(class: &QName, graph: &ClassGraph) -> Vec<QName> {
    let mut seen = FxHashSet::default();
    seen.insert(class.clone());
    let mut stack = vec![class.clone()];
    let mut out = Vec::new();
    while let Some(next) = stack.pop() {
        let Some(links) = graph.get(&next) else {
            continue;
        };
        for parent in links.extends.iter().chain(&links.implements) {
            if seen.insert(parent.clone()) {
                stack.push(parent.clone());
                out.push(parent.clone());
            }
        }
    }
    out
}

/// Reports [`code::E_RESTRICTED_USE`] at every use the table entries after `mark` name that an
/// entry holds and does not allow at `place`.
pub(crate) fn check_uses(place: Place<'_>, mark: usize, env: &mut Env<'_>) {
    let Some(restricted) = env.restricted else {
        return;
    };
    let mut found: Vec<(Span, usize)> = Vec::new();
    for (span, info) in env.exprs.since_at(mark) {
        for (owner, member, _) in crate::deprecated::resolved(info, env) {
            for at in restricted.holding(&owner, &member, env) {
                if !restricted.allows(at, &place, span.file, env) {
                    found.push((span, at));
                }
            }
        }
    }
    let mut seen = FxHashSet::default();
    for (span, at) in found {
        if seen.insert((span, at)) {
            env.diags.report(refusal(&restricted.entries[at], span));
        }
    }
}

impl Restricted<'_> {
    /// Every entry that holds a use of `member` of `owner`.
    fn holding(&self, owner: &QName, member: &Member, env: &Env<'_>) -> Vec<usize> {
        let mut out = Vec::new();
        let class = owner.to_string();
        let name = match member {
            Member::Method(name)
            | Member::Property(name)
            | Member::Const(name)
            | Member::Case(name) => Some(name.as_str()),
            Member::Type | Member::Param(..) => None,
        };
        if let Some(name) = name {
            out.extend(
                self.members
                    .get(&(class.as_str(), name))
                    .into_iter()
                    .flatten(),
            );
        }
        if let (Member::Method(name), true) = (member, !self.members.is_empty()) {
            for ancestor in ancestors(owner, env.graph) {
                let declares = resolve_method_ref(&ancestor, name, env.signatures, env.graph)
                    .is_some_and(|(declaring, _)| declaring == &ancestor);
                if declares {
                    let key = ancestor.to_string();
                    out.extend(
                        self.members
                            .get(&(key.as_str(), name.as_str()))
                            .into_iter()
                            .flatten(),
                    );
                }
            }
        }
        out.extend(self.names.get(class.as_str()).into_iter().flatten());
        if let Some(ns) = namespace_of(owner) {
            out.extend(self.names.get(ns.as_str()).into_iter().flatten());
            out.extend(
                self.subtrees
                    .iter()
                    .filter(|(root, _)| under(&ns, root))
                    .map(|&(_, at)| at),
            );
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whether entry `at` allows a use at `place`, in `file`.
    fn allows(&self, at: usize, place: &Place<'_>, file: SourceId, env: &Env<'_>) -> bool {
        let entry = &self.entries[at];
        if let Some(package) = entry.package() {
            let inside = env
                .packages
                .and_then(|packages| packages.of(file))
                .is_some_and(|of| of.name() == package);
            if !inside {
                return false;
            }
        }
        let namespace = place.namespace.join("\\");
        let class = place.class.map(QName::to_string);
        let method = match place.member {
            Some(Member::Method(name)) => Some(name.as_str()),
            _ => None,
        };
        let matches = |target: &Target| match target {
            Target::Name(name) => namespace == *name || class.as_deref() == Some(name.as_str()),
            Target::Subtree(root) => under(&namespace, root),
            Target::Member {
                class: owner,
                member,
            } => class.as_deref() == Some(owner.as_str()) && method == Some(member.as_str()),
        };
        let own = match entry.target() {
            Target::Name(name) => namespace == *name || class.as_deref() == Some(name.as_str()),
            Target::Subtree(root) => under(&namespace, root),
            Target::Member { class: owner, .. } => class.as_deref() == Some(owner.as_str()),
        };
        own || entry.allow().iter().any(matches)
    }
}

/// The error at a use `entry` does not allow: what is not allowed, where it is allowed, the note,
/// and the file and line of the entry.
fn refusal(entry: &Restriction, span: Span) -> Diagnostic {
    let mut text = match entry.allow() {
        [] => "It cannot be used anywhere.".to_owned(),
        places => {
            let list: Vec<String> = places.iter().map(|place| format!("`{place}`")).collect();
            format!("It is allowed only in {}.", list.join(", "))
        }
    };
    if let Some(note) = entry.note() {
        text.push(' ');
        text.push_str(note.trim());
    }
    let mut diagnostic = Diagnostic::error(
        code::E_RESTRICTED_USE,
        format!("`{}` cannot be used here", entry.target()),
    )
    .with_primary(span, "this use is not allowed here")
    .with_note(text)
    .with_note(format!(
        "This rule is in `{}`, line {}.",
        entry.file(),
        entry.line()
    ));
    if let Some(at) = entry.span() {
        diagnostic = diagnostic.with_secondary(at, "the restriction");
    }
    diagnostic
}

/// The namespace `class` is declared in, or `None` for a class in the global namespace.
fn namespace_of(class: &QName) -> Option<String> {
    let segments = class.segments();
    let (_, namespace) = segments.split_last()?;
    (!namespace.is_empty()).then(|| namespace.join("\\"))
}

/// Whether the namespace `name` is `root` or one below it.
fn under(name: &str, root: &str) -> bool {
    name.strip_prefix(root)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('\\'))
}
