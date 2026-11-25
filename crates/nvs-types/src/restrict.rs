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
//! constant and an enum case are each a use, and a type position, an `is` test and `catch` record
//! no such entry and are not. No side list is kept.
//!
//! **Which entries hold a use.** A use of a member is held by an entry on the member, on its class
//! and on the class's namespace (`'Ns'` or a `'Ns\*'` subtree above it). A use of a method is held
//! too by an entry on each ancestor's method of the same name, so an override and an
//! implementation are restricted with the method they replace. Every entry that holds a use must
//! allow it: nothing lifts an entry. A call of a member an `extend` block adds resolves to the
//! block, so it is held by an entry on `'Block::member'`, on the block and on the block's
//! namespace, and never by one on the type the block extends.
//!
//! **Where a use is.** Its place is the namespace it is written in, the class around it and the
//! method around it. In an `extend` block's body the class is the block, never the type it
//! extends ([`crate::deprecated::window`]), so a block cannot use a member restricted on its type
//! from inside. An `allow` entry matches the namespace (exactly, or a subtree), the class, or
//! the `Class::method`. A use inside the restricted declaration itself, its own class or its own
//! namespace, is always allowed, so restricted code can call itself. A package's entry matches only
//! where the using file is inside that package by layout ([`crate::package`]), so a file that
//! writes the package's namespace from outside its folder is still refused.
//!
//! **A `replace` is compiled where it is written.** [`Restricted::replacements`] makes a copy of
//! the entry's `package.toml` in which the replacement is the only code, and [`check_replacement`]
//! infers it there, in the namespace of the file the table is in, with the target method's
//! parameters and `$this`, through [`crate::deprecated::check_restriction`]. What is wrong is
//! [`code::E_BAD_RESTRICTION_REPLACE`] at the value. At a refused use the entry's `replace` is
//! filled in as the fix ([`crate::deprecated::restriction_fix`]).
//!
//! **A reflective act is held to the same entries.** A `Core\Reflect\ClassInfo` member that calls
//! a method, reads or writes a property, reads a constant or constructs names its member at run
//! time, so [`record_acts`] lists every act on a restricted member that the call's place does not
//! allow, with its error text, and the call-site constant carries that list
//! (`nvs_runtime::source::encode_restricted`). The member throws when the act it makes is on it.
//! The list names the member on the class the act is made on: an inherited member is listed under
//! every class that inherits it, held by the entries a use of it there would be. A callable
//! reference to such a member is invoked somewhere not known here, so its list names every act an
//! entry restricts, allowed anywhere or not.
//!
//! **A call or property access through a `mixed` or `object` receiver is held the same way.**
//! Its class is known only at run time, so [`record_acts`] lists the acts of its kind on every
//! restricted member of its name that the place does not allow, and `nvs_ir` emits
//! `Helper::RestrictedErasedAct` before the access, which throws when the receiver's class is on
//! the list. A site whose name no entry restricts records nothing and emits nothing. A keyed
//! access, `$object->$key`, is a use of every public property of its key's class, and
//! [`check_uses`] refuses it at compile time where any of them is restricted here.
//!
//! Cost: one pass over each body's new table entries, and a few hash lookups per entry, only when
//! a table exists, and one inference of each `replace` per check. Memory: one index of the
//! program's entries per check, and one copy of a `package.toml` per `replace` while it is
//! inferred, never per request. A reflective call site in a program with a table carries one
//! list in the unit's data, and each act it makes scans it once.

use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use nvs_config::restrict::{Restriction, Restrictions, Target};
use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, SourceId, Span, code};
use nvs_hir::{ClassGraph, QName};
use nvs_runtime::source::Act;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::Env;
use crate::deprecated::Member;
use crate::expr_table::ExprInfo;
use crate::signatures::{
    SignatureTable, resolve_const_owned, resolve_method, resolve_method_ref, resolve_property_owned,
};

/// The program's restrictions, indexed by the declaration each one names.
pub(crate) struct Restricted<'g> {
    entries: &'g [Restriction],
    /// `'Ns\Name'` entries, by the name: a class's or a namespace's.
    names: FxHashMap<&'g str, Vec<usize>>,
    /// `'Ns\Name::member'` entries, by the class and the member.
    members: FxHashMap<(&'g str, &'g str), Vec<usize>>,
    /// `'Ns\*'` entries, by the root.
    subtrees: Vec<(&'g str, usize)>,
    /// Every reflective act an entry holds, built on the first reflective call ([`record_acts`]).
    acts: OnceCell<Vec<Held>>,
    /// The program's directory. An error names an entry's file relative to it, as the editor's
    /// hover does, so the text is the same on every machine.
    home: Option<PathBuf>,
}

/// One reflective act on a member of a class, and the entries that hold it.
struct Held {
    act: Act,
    class: String,
    /// The member's name, empty for [`Act::Construct`].
    member: String,
    entries: Vec<usize>,
}

/// Where one checked body is: its namespace, and the class and member around it.
#[derive(Clone, Copy)]
pub(crate) struct Place<'p> {
    pub namespace: &'p [String],
    pub class: Option<&'p QName>,
    pub member: Option<&'p Member>,
}

/// The index of `restrictions`, or `None` when there is no entry. Each entry's target is checked
/// against the program here, and what is wrong goes to `diags`. `home` is the program's
/// directory, which every error names an entry's file relative to.
pub(crate) fn build<'g>(
    restrictions: &'g Restrictions,
    home: Option<&Path>,
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
        acts: OnceCell::new(),
        home: home.map(Path::to_path_buf),
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
    let mut found: Vec<(Span, usize, QName, Member)> = Vec::new();
    for (span, info) in env.exprs.since_at(mark) {
        // A keyed access, `$object->$key`, names its property only at run time, and the key may
        // be any public property of its class. So it is a use of each of them.
        if let ExprInfo::KeyedProperty { class, .. } = info {
            let class = QName::parse(class);
            for name in crate::expr::members::public_property_names(&class, env) {
                let owner = crate::signatures::resolve_property_owned(
                    &class,
                    &name,
                    env.signatures,
                    env.graph,
                )
                .map_or_else(|| class.clone(), |(owner, _)| owner);
                let member = Member::Property(name);
                for at in restricted.holding(&owner, &member, env) {
                    if !restricted.allows(at, &place, span.file, env) {
                        found.push((span, at, owner.clone(), member.clone()));
                    }
                }
            }
            continue;
        }
        for (owner, member, _) in crate::deprecated::resolved(info, env) {
            for at in restricted.holding(&owner, &member, env) {
                if !restricted.allows(at, &place, span.file, env) {
                    found.push((span, at, owner.clone(), member.clone()));
                }
            }
        }
    }
    let mut seen = FxHashSet::default();
    for (span, at, owner, member) in found {
        if !seen.insert((span, at)) {
            continue;
        }
        let entry = &restricted.entries[at];
        let mut diagnostic = refusal(entry, span, restricted.home.as_deref());
        if let (Some(replace), Member::Method(name)) = (entry.replace(), &member) {
            diagnostic = crate::deprecated::restriction_fix(
                diagnostic,
                span,
                &owner,
                name,
                replace,
                &namespace(entry),
                env,
            );
        }
        env.diags.report(diagnostic);
    }
    record_acts(restricted, &place, mark, env);
}

/// Which acts one site's restricted-act list names.
enum ActSite {
    /// A `Core\Reflect\ClassInfo` acting call: every act the place does not allow.
    Reflective,
    /// A callable reference to one: every act an entry holds, allowed here or not.
    Anywhere,
    /// A call or property access through a `mixed` or `object` receiver: the acts of its one
    /// kind on members of its one name that the place does not allow.
    Erased(Act, String),
}

/// Records, for each `Core\Reflect\ClassInfo` acting call after `mark`, the list of acts on a
/// restricted member that `place` does not allow, for each callable reference to one, every act
/// an entry holds, and for each call or property access through a `mixed` or `object` receiver,
/// the acts on a member of its name that `place` does not allow. A site with nothing to refuse
/// records nothing.
fn record_acts(restricted: &Restricted<'_>, place: &Place<'_>, mark: usize, env: &mut Env<'_>) {
    let sites: Vec<(Span, ActSite)> = env
        .exprs
        .since_at(mark)
        .filter_map(|(span, info)| match info {
            ExprInfo::Call(call) | ExprInfo::ClassRefCall(call)
                if crate::core_takes_call_site(&call.class.to_string(), &call.method) =>
            {
                Some((span, ActSite::Reflective))
            }
            ExprInfo::CallableRef(call)
                if crate::core_takes_call_site(&call.class.to_string(), &call.method) =>
            {
                Some((span, ActSite::Anywhere))
            }
            ExprInfo::ErasedCall { name } => Some((span, ActSite::Erased(Act::Call, name.clone()))),
            ExprInfo::ShapeProperty { name, .. } => {
                Some((span, ActSite::Erased(Act::Property, name.clone())))
            }
            _ => None,
        })
        .collect();
    if sites.is_empty() {
        return;
    }
    let held = restricted.acts.get_or_init(|| restricted.held_acts(env));
    for (span, site) in sites {
        let refused: Vec<(Act, &str, &str, String)> =
            held.iter()
                .filter_map(|held| {
                    if let ActSite::Erased(act, name) = &site
                        && (held.act != *act || held.member != *name)
                    {
                        return None;
                    }
                    let anywhere = matches!(site, ActSite::Anywhere);
                    let at =
                        held.entries.iter().copied().find(|&at| {
                            anywhere || !restricted.allows(at, place, span.file, env)
                        })?;
                    let entry = &restricted.entries[at];
                    let text = format!(
                        "`{}` cannot be used here. {}",
                        entry.target(),
                        refusal_text(entry, restricted.home.as_deref())
                    );
                    Some((held.act, held.class.as_str(), held.member.as_str(), text))
                })
                .collect();
        if refused.is_empty() {
            continue;
        }
        let acts: Vec<(Act, &str, &str, &str)> = refused
            .iter()
            .map(|(act, class, member, text)| (*act, *class, *member, text.as_str()))
            .collect();
        let bytes = nvs_runtime::source::encode_restricted(&acts);
        env.exprs.record_restricted_acts(span, bytes.into());
    }
}

/// The namespace an entry's `replace` is written in: the package's name, or the global namespace
/// for the application's file.
fn namespace(entry: &Restriction) -> Vec<String> {
    entry
        .package()
        .map(|name| name.split('\\').map(str::to_owned).collect())
        .unwrap_or_default()
}

/// One entry's `replace`, ready to compile: the copy of `package.toml` its text is parsed in, and
/// the method it replaces.
pub(crate) struct Replacement {
    at: usize,
    /// The copy of the file in which the replacement is the only code
    /// ([`nvs_diagnostics::SourceFile::code_in`]).
    pub copy: SourceFile,
    owner: QName,
    method: String,
    start: u32,
    lit: Span,
}

impl Restricted<'_> {
    /// Every entry's `replace` that can be compiled, each in its copy of the file. A `replace` on
    /// a target that is not a method is [`code::E_BAD_RESTRICTION`], and one whose value is not
    /// its written text is [`code::E_BAD_RESTRICTION_REPLACE`], both reported to `diags`. An
    /// entry whose target names nothing has been reported already and is skipped.
    pub(crate) fn replacements(
        &self,
        signatures: &SignatureTable,
        graph: &ClassGraph,
        diags: &mut Diagnostics,
    ) -> Vec<Replacement> {
        let mut out = Vec::new();
        for (at, entry) in self.entries.iter().enumerate() {
            let (Some(text), Some(lit)) = (entry.replace(), entry.replace_span()) else {
                continue;
            };
            let method = match entry.target() {
                Target::Member { class, member } => {
                    let owner = QName::parse(class);
                    if resolve_method(&owner, member, signatures, graph).is_some() {
                        Some((owner, member.clone()))
                    } else if names_something(entry.target(), signatures, graph) {
                        None
                    } else {
                        continue;
                    }
                }
                Target::Name(_) | Target::Subtree(_) => None,
            };
            let Some((owner, method)) = method else {
                diags.report(
                    Diagnostic::error(
                        code::E_BAD_RESTRICTION,
                        format!(
                            "`{}` is not a method, so it has no replacement",
                            entry.target()
                        ),
                    )
                    .with_primary(lit, "here")
                    .with_help("a `replace` belongs to a method. Delete this line".to_owned()),
                );
                continue;
            };
            let copy = entry.replace_at().and_then(|(start, _)| {
                SourceFile::code_in(lit.file, entry.file(), start, text).map(|copy| (start, copy))
            });
            let Some((start, copy)) = copy else {
                diags.report(
                    Diagnostic::error(
                        code::E_BAD_RESTRICTION_REPLACE,
                        "the replacement is not the text written between its quotes",
                    )
                    .with_primary(lit, "this text has an escape sequence")
                    .with_help(
                        "write the replacement in single quotes, with no `\\` escape".to_owned(),
                    ),
                );
                continue;
            };
            out.push(Replacement {
                at,
                copy,
                owner,
                method,
                start,
                lit,
            });
        }
        out
    }
}

/// Compiles `job`'s replacement in `env`, whose source is `job.copy`, and reports
/// [`code::E_BAD_RESTRICTION_REPLACE`] when it does not compile, does not fit the method's return
/// type, or uses a declaration its own entry restricts.
pub(crate) fn check_replacement(job: &Replacement, env: &mut Env<'_>) {
    let Some(restricted) = env.restricted else {
        return;
    };
    let entry = &restricted.entries[job.at];
    let Some(text) = entry.replace() else {
        return;
    };
    let signatures = env.signatures;
    let Some((_, sig)) = resolve_method(&job.owner, &job.method, signatures, env.graph) else {
        return;
    };
    let namespace = namespace(entry);
    let lit = job.lit;
    let refused = |message: String| {
        Diagnostic::error(code::E_BAD_RESTRICTION_REPLACE, message)
            .with_primary(lit, "a use outside the allowed places is rewritten to this")
    };
    match crate::deprecated::check_restriction(text, job.start, &job.owner, &sig, &namespace, env) {
        Err(inner) => {
            let diagnostic = refused("the replacement does not compile".to_owned());
            let diagnostic = match inner.primary_span() {
                Some(span) if span != lit => diagnostic.with_secondary(span, inner.message),
                _ => diagnostic.with_note(inner.message),
            };
            env.diags.report(diagnostic);
        }
        Ok(named) => {
            let own = named
                .iter()
                .find(|(owner, member)| restricted.holding(owner, member, env).contains(&job.at));
            if let Some((owner, member)) = own {
                let what = match member {
                    Member::Type => owner.to_string(),
                    Member::Method(name)
                    | Member::Property(name)
                    | Member::Const(name)
                    | Member::Case(name) => format!("{owner}::{name}"),
                    Member::Param(..) => return,
                };
                env.diags.report(
                    refused(format!(
                        "the replacement uses `{what}`, which this entry restricts"
                    ))
                    .with_help(
                        "write the code that replaces it, which a use outside the allowed \
                             places may call"
                            .to_owned(),
                    ),
                );
            }
        }
    }
}

impl Restricted<'_> {
    /// Every reflective act on a member of a program class that an entry holds. Only a class whose
    /// own name, namespace or ancestors an entry names is walked.
    fn held_acts(&self, env: &Env<'_>) -> Vec<Held> {
        let mut out = Vec::new();
        for (class, _) in env.signatures.iter() {
            let mut chain = vec![class.clone()];
            chain.extend(ancestors(class, env.graph));
            if !chain.iter().any(|link| self.touches(link)) {
                continue;
            }
            let (mut methods, mut properties, mut constants) = (
                FxHashSet::default(),
                FxHashSet::default(),
                FxHashSet::default(),
            );
            for link in &chain {
                if let Some(sig) = env.signatures.get(link) {
                    methods.extend(sig.methods.keys().cloned());
                    properties.extend(sig.properties.keys().cloned());
                    constants.extend(sig.constants.keys().cloned());
                }
            }
            let mut push = |act: Act, member: String, entries: Vec<usize>| {
                if !entries.is_empty() {
                    out.push(Held {
                        act,
                        class: class.to_string(),
                        member,
                        entries,
                    });
                }
            };
            let mut construct = self.holding(class, &Member::Type, env);
            for name in methods {
                let Some((owner, _)) = resolve_method_ref(class, &name, env.signatures, env.graph)
                else {
                    continue;
                };
                let entries = self.holding(owner, &Member::Method(name.clone()), env);
                if name == "constructor" {
                    construct.extend(entries);
                } else {
                    push(Act::Call, name, entries);
                }
            }
            for name in properties {
                let owner = resolve_property_owned(class, &name, env.signatures, env.graph)
                    .map_or_else(|| class.clone(), |(owner, _)| owner);
                let entries = self.holding(&owner, &Member::Property(name.clone()), env);
                push(Act::Property, name, entries);
            }
            for name in constants {
                let owner = resolve_const_owned(class, &name, env.signatures, env.graph)
                    .map_or_else(|| class.clone(), |(owner, _)| owner);
                let entries = self.holding(&owner, &Member::Const(name.clone()), env);
                push(Act::Constant, name, entries);
            }
            construct.sort_unstable();
            construct.dedup();
            push(Act::Construct, String::new(), construct);
        }
        out
    }

    /// Whether an entry names `class`, a member of it, or its namespace.
    fn touches(&self, class: &QName) -> bool {
        let name = class.to_string();
        self.names.contains_key(name.as_str())
            || self.members.keys().any(|(owner, _)| *owner == name)
            || namespace_of(class).is_some_and(|ns| {
                self.names.contains_key(ns.as_str())
                    || self.subtrees.iter().any(|(root, _)| under(&ns, root))
            })
    }

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
        let namespace = place.namespace.join("\\");
        let class = place.class.map(QName::to_string);
        let method = match place.member {
            Some(Member::Method(name)) => Some(name.as_str()),
            _ => None,
        };
        let site = Site {
            namespace: &namespace,
            class: class.as_deref(),
            method,
            package: env
                .packages
                .and_then(|packages| packages.of(file))
                .map(nvs_config::package::Package::name),
            graph: env.graph,
        };
        allows_at(&self.entries[at], &site)
    }
}

/// Where one use is written: the namespace, the class and the method around it, and the package
/// its file is in by layout.
#[derive(Clone, Copy, Debug)]
pub struct Site<'s> {
    /// The namespace, its segments joined with `\`, and empty for the global one.
    pub namespace: &'s str,
    /// The class around the use, without a leading `\`.
    pub class: Option<&'s str>,
    /// The method around the use.
    pub method: Option<&'s str>,
    /// The name of the package the use's file is in, by layout ([`crate::package`]).
    pub package: Option<&'s str>,
    /// The program's classes. An allowed name that is a class allows that class alone, and
    /// never a namespace that has the same name.
    pub graph: &'s ClassGraph,
}

/// Whether `entry` allows a use at `site`: the check behind every [`code::E_RESTRICTED_USE`],
/// and the one the editor's completion asks of the cursor.
#[must_use]
pub fn allows_at(entry: &Restriction, site: &Site<'_>) -> bool {
    if let Some(package) = entry.package()
        && site.package != Some(package)
    {
        return false;
    }
    let Site {
        namespace,
        class,
        method,
        graph,
        ..
    } = *site;
    let names = |name: &str| {
        class == Some(name) || (namespace == name && graph.get(&QName::parse(name)).is_none())
    };
    let matches = |target: &Target| match target {
        Target::Name(name) => names(name),
        Target::Subtree(root) => under(namespace, root),
        Target::Member {
            class: owner,
            member,
        } => class == Some(owner.as_str()) && method == Some(member.as_str()),
    };
    let own = match entry.target() {
        Target::Name(name) => names(name),
        Target::Subtree(root) => under(namespace, root),
        Target::Member { class: owner, .. } => class == Some(owner.as_str()),
    };
    own || entry.allow().iter().any(matches)
}

/// The error at a use `entry` does not allow: what is not allowed, where it is allowed, the note,
/// and the file and line of the entry.
fn refusal(entry: &Restriction, span: Span, home: Option<&Path>) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(
        code::E_RESTRICTED_USE,
        format!("`{}` cannot be used here", entry.target()),
    )
    .with_primary(span, "this use is not allowed here")
    .with_note(allowed_text(entry))
    .with_note(source_text(entry, home));
    if let Some(at) = entry.span() {
        diagnostic = diagnostic.with_secondary(at, "the restriction");
    }
    diagnostic
}

/// Every entry of `restrictions` that names `class`, its `member` when one is given, or the
/// namespace `class` is declared in, in the order the tables were read. The editor's hover lists
/// them. An entry on a parent's method that an override replaces is not among them.
#[must_use]
pub fn entries_on<'r>(
    restrictions: &'r Restrictions,
    class: &QName,
    member: Option<&str>,
) -> Vec<&'r Restriction> {
    let name = class.to_string();
    let namespace = namespace_of(class).unwrap_or_default();
    restrictions
        .entries()
        .iter()
        .filter(|entry| match entry.target() {
            Target::Name(target) => *target == name || *target == namespace,
            Target::Subtree(root) => under(&namespace, root),
            Target::Member {
                class: owner,
                member: written,
            } => *owner == name && Some(written.as_str()) == member,
        })
        .collect()
}

/// Where `entry` allows its target, then its note: the first note of an `E0664`, and the text
/// the editor's hover shows.
#[must_use]
pub fn allowed_text(entry: &Restriction) -> String {
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
    text
}

/// The file and line `entry` is written at, the file relative to the program's directory `home`.
/// The editor's hover shows the same sentence.
#[must_use]
pub fn source_text(entry: &Restriction, home: Option<&Path>) -> String {
    format!(
        "This rule is in `{}`, line {}.",
        relative_to(home, entry.file()),
        entry.line()
    )
}

/// The error text a refused reflective act throws: [`refusal`]'s notes, after the message.
fn refusal_text(entry: &Restriction, home: Option<&Path>) -> String {
    format!("{} {}", allowed_text(entry), source_text(entry, home))
}

/// `file` relative to `home`, with `/` between its parts, or `file` as it is when it is not
/// inside `home`. An absolute path differs from machine to machine, and in an error it is noise.
/// `home` may be relative, and the two may differ in a symlink or in Windows' `\\?\` prefix, so
/// the comparison is tried as written, then made absolute, then with both canonicalized.
fn relative_to(home: Option<&Path>, file: &str) -> String {
    let path = Path::new(file);
    // An entry named by its bare file name has the empty path as its parent: the working directory.
    let home = home.map(|dir| {
        if dir.as_os_str().is_empty() {
            Path::new(".")
        } else {
            dir
        }
    });
    let inside = |file: &Path, dir: &Path| {
        file.strip_prefix(dir)
            .ok()
            .map(|rest| rest.to_string_lossy().replace('\\', "/"))
    };
    home.and_then(|dir| {
        inside(path, dir)
            .or_else(|| {
                std::path::absolute(dir)
                    .ok()
                    .and_then(|dir| inside(path, &dir))
            })
            .or_else(|| {
                let dir = nvs_hir::requires::canonicalize(dir)?;
                let file = nvs_hir::requires::canonicalize(path)?;
                inside(&file, &dir)
            })
    })
    .unwrap_or_else(|| file.to_owned())
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
