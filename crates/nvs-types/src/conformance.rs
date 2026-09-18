//! Interface conformance: every method a class's interfaces declare without
//! a body, the class has to answer.
//!
//! # Why this exists
//!
//! An interface is what a call dispatches through. `rule:iteration/two-interfaces`'s `Iterator<T>` is the case that needs it: its members are bodiless *on purpose*
//! ([`crate::iter_lib`]'s own docs own why — a call resolving to a bodiless
//! declaration has no compiled function to name, so it dispatches on the
//! receiver's runtime class, which is exactly what driving a cursor of
//! unknown concrete class needs). A class claiming `implements Iterator<int>`
//! and forgetting `advance` therefore ends in a dispatch to nothing rather
//! than in a call the author wrote by hand.
//!
//! # What it checks, and what it deliberately does not
//!
//! One walk of every `extends`/`implements` ancestor. Each ancestor's own
//! bodiless, non-`interface_private` methods are the obligation; the class
//! discharges one by having [`resolve_method`] land on a declaration that
//! *does* have a body — its own, an inherited one, or an `rule:classes/interface-default-methods`
//! interface default.
//!
//! The same walk asks one question of the answers it accepts: a member the
//! ancestor declared `: static` has to be answered by a declaration that also
//! writes `static` (`E0404`, [`reject_dropped_static_return`]). Nothing else
//! about the two signatures is compared here — argument and return
//! assignability is a rule of its own that nothing has decided yet.
//!
//! These are outside it, each for its own reason:
//!
//! - **An `abstract` class is exempt.** Leaving a member to a subclass is
//!   what the modifier means.
//! - **A member `rule:classes/delegation-by-field`'s `by $field` delegation supplies is exempt, one
//!   member at a time.** Delegation answers a member from a property's own
//!   type, so a synthesized forward discharges the obligation exactly as a
//!   written body does — but only for the members that actually get one.
//!   The exemption is per-member rather than whole-class because § 4's own
//!   first bullet is checked here: [`check_delegate_field`] asks whether
//!   `$field` is a declared property of a non-nullable type that satisfies
//!   the interface (`E0720`), so a member no forward covers can be told from
//!   one whose field could not have answered it. [`resolve_delegations`] is
//!   what runs both halves, and its own doc comment owns what a forward is
//!   and which members get one.
//!
//! The compiler-declared global interfaces are *inside* it, and reach it
//! the same way a source-declared one does: [`crate::iter_lib`] seeds every
//! member on [`nvs_hir::interfaces`]'s roster, so `implements Comparable`
//! owes `compareTo` and `implements Stringable` owes `toString` here, rather
//! than only at the use sites (`crate::expr`'s `require_stringable` and the
//! object comparison check).

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::{QName, SymbolKind, implements_interface};
use nvs_syntax::ast::{ClassDecl, ClassMemberKind, Modifier};
use rustc_hash::FxHashSet;

use crate::Env;
use crate::derive::stands_in_for_a_class;
use crate::expr::is_assignable;
use crate::expr_table::{ArgSlot, Delegation};
use crate::signatures::{resolve_method, resolve_property};
use crate::ty::{Ty, TypeId};
use crate::{span_text, strip_sigil};

/// Reports one `E0449` per member of `decl`'s interfaces that nothing
/// answers, and one `E0404` per member the answer weakens. See the module
/// docs for the exemptions.
pub(crate) fn check_class_conformance(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    if decl.modifiers.contains(&Modifier::Abstract) {
        return;
    }
    let delegated = if decl.implements.iter().any(|c| c.by_field.is_some()) {
        match resolve_delegations(decl, qname, env) {
            Some(covered) => covered,
            // The clauses could not be lined up with the resolved graph, so
            // there is no delegation to subtract and no program to lower —
            // see `resolve_delegations`' own comment.
            None => return,
        }
    } else {
        FxHashSet::default()
    };

    let mut seen = FxHashSet::default();
    let mut owed: Vec<(QName, String)> = Vec::new();
    collect_obligations(qname, env, &mut seen, &mut owed);

    for (interface, method) in owed {
        if delegated.contains(&method) {
            continue;
        }
        let answer = resolve_method(qname, &method, env.signatures, env.graph)
            .map(|(_, sig)| (sig.has_body, sig.returns_static));
        if let Some((true, keeps_static)) = answer {
            if !keeps_static {
                reject_dropped_static_return(decl, qname, &interface, &method, env);
            }
            continue;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_INTERFACE_METHOD_MISSING,
                format!("`{qname}` does not declare `{method}`, which `{interface}` requires"),
            )
            .with_primary(
                decl.name.span,
                format!("this class implements `{interface}`"),
            )
            .with_help(format!(
                "declare `{method}` here, or give `{interface}` a default body for it \
                 (`rule:classes/interface-default-methods`)"
            )),
        );
    }
}

/// `E0404` for an answer that drops the `static` its declaration promised.
///
/// A member declared `: static` promises the *called* class, and that promise
/// is what a caller reads: `Slug::parse($text)` is a `Slug` only because
/// [`MethodSig::returns_static`](crate::signatures::MethodSig::returns_static)
/// makes the call site substitute the class the call named. An answer written
/// `: Slug`, `: self` or `: string` keeps the assignability rule — every one of
/// those is assignable to the interface's own class, or is refused elsewhere —
/// and still breaks that substitution for every subclass, so assignability is
/// not the question here and a check of its own is.
///
/// This is the shape `Parses` needs and the only one this reports: `parse` is
/// declared `parse(tainted string $s): static`, and a binding site that hands
/// a route segment to it reads the answer as the class the parameter named.
/// An implementor answering anything else would put a value of one class where
/// the site had already decided another, which is priority 2 in AGENTS.md's
/// ordering.
///
/// The primary lands on the class's own declaration of the member where it has
/// one; a class that inherits the weakened answer gets its own name instead,
/// which is the only span it wrote.
fn reject_dropped_static_return(
    decl: &ClassDecl,
    qname: &QName,
    interface: &QName,
    method: &str,
    env: &mut Env<'_>,
) {
    let requires_static = env
        .signatures
        .get(interface)
        .and_then(|sig| sig.methods.get(method))
        .is_some_and(|sig| sig.returns_static);
    if !requires_static {
        return;
    }
    let written = decl.members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Method(m) if span_text(env.src, m.name) == method => Some(
            m.return_type
                .as_ref()
                .map_or(m.name, |declared| declared.span),
        ),
        _ => None,
    });
    env.diags.report(
        Diagnostic::error(
            code::E_INCOMPATIBLE_OVERRIDE,
            format!("`{qname}::{method}` does not return `static`, which `{interface}` declares"),
        )
        .with_primary(
            written.unwrap_or(decl.name.span),
            "this answers a class of its own",
        )
        .with_help(format!(
            "write `: static` here — `{interface}::{method}` promises the class the call named, \
             which a subclass may be"
        )),
    );
}

/// The promises `final` makes, checked where the declaration that breaks
/// one is written: no class extends a `final` class (`E0783`), and no class
/// redeclares a method an ancestor declared `final` (`E0784`).
///
/// Here rather than in a module of its own because it asks this module's own
/// question from the other side — what a class's ancestors impose on it — and
/// reads the same tables ([`resolve_method`] and the class graph) to do
/// it. The modifier itself is recorded per declaration
/// ([`crate::signatures::ClassSignature::final_methods`]), so both halves are
/// a lookup against the declaring class rather than a walk of the source.
///
/// The **class** half asks the direct superclass and nothing further: a
/// `final` class cannot be extended at all, so an ancestor two links up was
/// already refused where it was named, and reporting again here would name a
/// class the author never wrote.
pub(crate) fn check_class_finality(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    let Some(links) = env.graph.get(qname) else {
        return;
    };
    let parent = links.extends.first().cloned();
    let ancestors: Vec<QName> = links
        .extends
        .iter()
        .chain(links.implements.iter())
        .cloned()
        .collect();

    if let (Some(parent), Some(written)) = (parent, decl.extends.as_ref())
        && crate::signatures::class_is_final(&parent, env.signatures)
    {
        env.diags.report(
            Diagnostic::error(
                code::E_FINAL_CLASS_EXTENDED,
                format!("`{parent}` is `final`, so no class extends it"),
            )
            .with_primary(
                written.span,
                format!("`{qname}` names it as its superclass"),
            )
            .with_help(format!(
                "drop the `final` from `{parent}`'s own declaration if it was meant to be a base \
                 class, or hold a `{parent}` in a property of `{qname}` and forward to it — \
                 `implements … by $field` (`rule:classes/delegation-by-field`) writes the forwards for you"
            )),
        );
    }

    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        let name = span_text(env.src, m.name).to_owned();
        let owner = ancestors
            .iter()
            .find_map(|ancestor| resolve_method(ancestor, &name, env.signatures, env.graph))
            .map(|(owner, _)| owner);
        let Some(owner) = owner else { continue };
        if !crate::signatures::method_is_final(&owner, &name, env.signatures) {
            continue;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_FINAL_METHOD_OVERRIDDEN,
                format!("`{owner}::{name}` is `final`, so no subclass redeclares it"),
            )
            .with_primary(m.name, format!("`{qname}` declares it again here"))
            .with_help(format!(
                "give this method a name of its own, or drop the `final` from `{owner}`'s \
                 declaration of it"
            )),
        );
    }
}

/// Reports one `E0786` per method of `decl` written without a body, when
/// `decl` itself is not `abstract`.
///
/// The other half of the rule `expr::calls`' `reject_abstract_instantiation`
/// enforces at `new`, and here rather than there because this is where the
/// declaration is: a class that leaves a member open must say so, and a
/// reader fixing this one is editing the class body rather than a call site.
///
/// **The question is the body and not the modifier.** `abstract` is what an
/// author writes, but what breaks is the missing body: `nvs-ir` skips a
/// bodiless method (`lower/mod.rs`), so codegen substitutes
/// `nvs_runtime::nvs_abstract_method` and the call is a run-time `FATAL`
/// naming no source at all. Both spellings therefore report, and only the
/// wording differs.
///
/// An interface never reaches this — its bodiless members are the contract
/// [`check_class_conformance`] holds implementors to — and neither does an
/// `abstract` class, where leaving a member to a subclass is what the
/// modifier means.
pub(crate) fn check_abstract_members(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    if decl.modifiers.contains(&Modifier::Abstract) {
        return;
    }
    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        if m.body.is_some() {
            continue;
        }
        let name = span_text(env.src, m.name).to_owned();
        let written_abstract = m.modifiers.contains(&Modifier::Abstract);
        let message = if written_abstract {
            format!("`{qname}::{name}` is `abstract`, but `{qname}` is not")
        } else {
            format!("`{qname}::{name}` has no body, but `{qname}` is not `abstract`")
        };
        env.diags.report(
            Diagnostic::error(code::E_ABSTRACT_METHOD_IN_CONCRETE_CLASS, message)
                .with_primary(m.name, "declared here, with nothing to call")
                .with_help(format!(
                    "give it a body, or declare `{qname}` itself `abstract` and instantiate a \
                     subclass that fills this member in"
                )),
        );
    }
}

/// Every `(declaring interface, method)` pair `qname` inherits without a
/// body, in a deterministic order: each ancestor's own methods sorted by
/// name, ancestors in `extends`-then-`implements` order.
///
/// Sorted rather than left in [`rustc_hash::FxHashMap`]'s bucket order for
/// the reason every other ordered walk in this crate is: a diagnostic's
/// position in the report must depend only on the source.
fn collect_obligations(
    qname: &QName,
    env: &Env<'_>,
    seen: &mut FxHashSet<QName>,
    out: &mut Vec<(QName, String)>,
) {
    let Some(links) = env.graph.get(qname) else {
        return;
    };
    let parents: Vec<QName> = links
        .extends
        .iter()
        .chain(links.implements.iter())
        .cloned()
        .collect();
    for parent in parents {
        if !seen.insert(parent.clone()) {
            continue;
        }
        own_obligations(&parent, env, out);
        collect_obligations(&parent, env, seen, out);
    }
}

/// Records one [`Delegation`] per member an `implements I by $field;` clause
/// makes the compiler synthesize a forward for — `rule:classes/delegation-by-field`.
///
/// The obligation set is [`collect_obligations`]', asked of the *interface*
/// rather than of the class: every bodiless, non-`interface_private` member it
/// and its own ancestors declare. A member the class already answers with a
/// body is skipped, which is § 4's "a class may still write its own method
/// with the same name as a delegated one — that is an ordinary override" — and
/// the same subtraction covers an inherited body and an `rule:classes/interface-default-methods` interface
/// default, both of which `resolve_method` finds and neither of which a
/// forward should displace.
///
/// These member shapes get no forward, and each is a hole rather than a rule:
/// a `static` member has no receiver to forward through (`rule:statements/static-is-a-member-modifier` gives class
/// storage none), and a variadic or `inout` parameter list is packed and
/// written back at the *call site*, so passing it straight on would pack it
/// twice. Each is refused where the clause is written (`E0721`) rather than
/// silently left to reach `nvs_runtime::nvs_abstract_method`, and each counts
/// as *covered* below so that [`check_class_conformance`]'s per-member walk
/// does not then report the same member a second time under `E0449`: one
/// member, one diagnostic, and the one that says what the author can do about
/// it.
///
/// Returns the set of member names the delegation discharges, or [`None`]
/// where the clauses could not be lined up with the resolved graph at all —
/// which the caller reads as "check nothing", the program having a
/// name-resolution error in it already.
fn resolve_delegations(
    decl: &ClassDecl,
    qname: &QName,
    env: &mut Env<'_>,
) -> Option<FxHashSet<String>> {
    let mut covered = FxHashSet::default();
    let links = env.graph.get(qname)?;
    // The graph drops an `implements` entry that resolved to nothing, so the
    // two lists only line up on a program with no name-resolution error in it.
    // One that has one is already reported and will never be lowered, so there
    // is nothing to resolve for it — and guessing which clause is which would
    // synthesize a forward against the wrong interface.
    if links.implements.len() != decl.implements.len() {
        return None;
    }
    let interfaces: Vec<QName> = links.implements.clone();
    let label = qname.to_string();
    for (clause, interface) in decl.implements.iter().zip(interfaces) {
        let Some(field_span) = clause.by_field else {
            continue;
        };
        let field = strip_sigil(span_text(env.src, field_span)).to_owned();
        let (line, _) = env.src.line_col(field_span.start);
        let mut seen = FxHashSet::default();
        seen.insert(qname.clone());
        let mut owed: Vec<(QName, String)> = Vec::new();
        own_obligations(&interface, env, &mut owed);
        seen.insert(interface.clone());
        collect_obligations(&interface, env, &mut seen, &mut owed);
        // § 4 bullet 1, asked once per clause rather than once per member:
        // a field that cannot answer the interface at all cannot answer any
        // of it, so every member it owes is marked covered and the author
        // reads one diagnostic about the field instead of one per member.
        if !check_delegate_field(qname, &field, field_span, &interface, env) {
            covered.extend(owed.into_iter().map(|(_, method)| method));
            continue;
        }
        for (_, method) in owed {
            if resolve_method(qname, &method, env.signatures, env.graph)
                .is_some_and(|(_, sig)| sig.has_body)
            {
                continue;
            }
            let Some((_, sig)) = resolve_method(&interface, &method, env.signatures, env.graph)
            else {
                continue;
            };
            let unforwardable = if sig.is_static {
                Some("a `static` member has no receiver to read the delegate off")
            } else if sig.variadic {
                Some(
                    "a variadic parameter list is packed at the call site, so a forward would pack it twice",
                )
            } else if sig.inout.iter().any(|inout| *inout) {
                Some(
                    "an `inout` parameter is written back at the call site, so a forward would stage it twice",
                )
            } else {
                None
            };
            if let Some(why) = unforwardable {
                env.diags.report(
                    Diagnostic::error(
                        code::E_DELEGATE_MEMBER_NOT_FORWARDABLE,
                        format!(
                            "`{interface}::{method}` cannot be forwarded to `${field}`, so \
                             `{qname}` has to declare it"
                        ),
                    )
                    .with_primary(field_span, why.to_owned())
                    .with_help(format!(
                        "write `{method}` on `{qname}` by hand — `rule:classes/delegation-by-field` already lets an own \
                         method stand in for a delegated one"
                    )),
                );
                covered.insert(method);
                continue;
            }
            covered.insert(method.clone());
            env.exprs.record_delegation(Delegation {
                class: label.clone(),
                method: method.clone(),
                field: field.clone(),
                params: sig.params.clone(),
                return_ty: sig.return_ty,
                frame: format!("{label}::{method}() at {}:{}", env.src.name(), line + 1),
                span: field_span,
            });
        }
    }
    Some(covered)
}

/// `rule:classes/delegation-by-field` bullet 1: whether `field` can carry `interface`'s members at
/// all — a declared property of `qname` (its own or an inherited one) whose
/// type is a **non-nullable** class or interface that satisfies `interface`.
/// Reports one `E0720` and answers `false` where it cannot.
///
/// Each half is a failure the synthesized forward has no answer for.
/// `nvs_ir::lower::call::delegation_forward` reads the slot, retains it and
/// dispatches late on whatever class the value's own header names: a name
/// that is no property has no slot to read, a `null` in the slot has no class
/// to dispatch on, and a type that does not reach `interface` names no member
/// for the call to land on. The judgement is
/// [`nvs_hir::implements_interface`]'s, which is reflexive — a field declared
/// *as* the interface satisfies it in zero steps, and that is the spelling
/// § 4's own worked example uses.
fn check_delegate_field(
    qname: &QName,
    field: &str,
    field_span: nvs_diagnostics::Span,
    interface: &QName,
    env: &mut Env<'_>,
) -> bool {
    let Some(ty) = resolve_property(qname, field, env.signatures, env.graph) else {
        env.diags.report(
            Diagnostic::error(
                code::E_DELEGATE_TYPE_MISMATCH,
                format!("`{qname}` delegates `{interface}` to `${field}`, which is not a property"),
            )
            .with_primary(
                field_span,
                "no property of this name is declared".to_owned(),
            )
            .with_help(format!(
                "declare a property `${field}` typed `{interface}` (`rule:classes/delegation-by-field`)"
            )),
        );
        return false;
    };
    let described = env.interner.describe(ty);
    let nullable = env.interner.is_nullable(ty);
    let target = match env.interner.get(ty) {
        Ty::Class(name, _) => Some(name.clone()),
        _ => None,
    };
    if !nullable && target.is_some_and(|name| implements_interface(&name, interface, env.graph)) {
        return true;
    }
    let why = if nullable {
        "a delegate is dispatched on at every forwarded call, so it may not be nullable"
    } else {
        "the delegate's type does not satisfy this interface"
    };
    env.diags.report(
        Diagnostic::error(
            code::E_DELEGATE_TYPE_MISMATCH,
            format!("`${field}` is `{described}`, which cannot satisfy `{interface}`"),
        )
        .with_primary(field_span, why.to_owned())
        .with_help(format!(
            "declare `${field}` as `{interface}`, or as a non-nullable class that implements it \
             (`rule:classes/delegation-by-field`)"
        )),
    );
    false
}

/// One declaration's own bodiless, non-`interface_private` members, appended
/// to `out` in the sorted order [`collect_obligations`] reads them in — the
/// half of that walk that looks at the named declaration itself rather than at
/// its ancestors.
fn own_obligations(qname: &QName, env: &Env<'_>, out: &mut Vec<(QName, String)>) {
    let Some(sig) = env.signatures.get(qname) else {
        return;
    };
    let mut names: Vec<&String> = sig
        .methods
        .iter()
        .filter(|(_, m)| !m.has_body && !m.interface_private)
        .map(|(name, _)| name)
        .collect();
    names.sort();
    out.extend(names.into_iter().map(|n| (qname.clone(), n.clone())));
}

/// Reports `E0827` where what a double names is not an interface, one `E0825`
/// per method that interface requires and the shape of answers leaves out, one
/// `E0826` per field that shape names which the interface does not declare,
/// and one `E0828` per field that names a method it cannot stand in for —
/// `rule:testing/doubles`, checked at the call rather than at a declaration
/// because a double declares nothing.
///
/// The obligation set is [`check_class_conformance`]'s, asked of the interface
/// instead of of a class: what a double owes is exactly what an implementor
/// would have had to write, so an `rule:classes/interface-default-methods`
/// default is owed by neither — the interface answers that one itself — and a
/// `private` interface method is part of no contract, so naming one is the
/// *second* refusal rather than an accepted answer.
///
/// A `partial` owes nothing of the first half: the real implementation behind
/// it answers every method its shape leaves out, which is the whole of what
/// makes it a partial. It is held to the second half unchanged, since a field
/// naming no method of the interface overrides nothing whichever member built
/// it.
///
/// **What is compared is the argument's own inferred type**, which
/// [`crate::expr::literals::check_object_literal`] interned from the fields as
/// written — the declared parameter is `object`
/// (`rule:types/object-top`), because the constraint this position really
/// carries is the interface the *call site* wrote and no registry row can name
/// that. An argument that is not a shape at all therefore names no method,
/// which for a `double` is every method missing at once: there is no answer it
/// could default to.
pub(crate) fn check_double_answers(
    owner: &QName,
    method: &str,
    written: &[TypeId],
    arg_types: &[TypeId],
    slots: &[ArgSlot],
    call_span: Span,
    env: &mut Env<'_>,
) {
    // `Core\Test` is the only owner that stands in for a class rather than
    // building one ([`crate::derive::stands_in_for_a_class`]), and its two
    // members are the whole roster — every other `Core` call reaches this with
    // nothing to ask. The parameter index is the member's own: `$answers` is
    // `double`'s first and `partial`'s second, behind the `$real` it delegates
    // to.
    let answers_at = match (stands_in_for_a_class(&owner.to_string()), method) {
        (true, "double") => 0,
        (true, "partial") => 1,
        _ => return,
    };
    let Some(first) = written.first().copied() else {
        return;
    };
    // A type argument that names no declaration at all is
    // `E_TYPE_ARG_NOT_A_CLASS`, reported where the written class is read
    // (`crate::expr::args::written_class_of`), so this asks only the narrower
    // question that refusal leaves open: the name resolved, and to what kind.
    let Ty::Class(interface, _) = env.interner.get(first) else {
        return;
    };
    let interface = interface.clone();
    if env.symbols.get(&interface).map(|symbol| symbol.kind) != Some(SymbolKind::Interface) {
        env.diags.report(
            Diagnostic::error(
                code::E_DOUBLE_TYPE_ARG_NOT_AN_INTERFACE,
                format!("`{owner}::{method}` stands in for an interface, and `{interface}` is not one"),
            )
            .with_primary(call_span, format!("`{interface}` written here"))
            .with_help(
                "`rule:testing/doubles`: a shape of closures can answer an interface's declarations \
                 and nothing else — a class also carries state and bodies of its own, which a \
                 double neither holds nor runs",
            ),
        );
        return;
    }
    let named = answered_methods(answers_at, arg_types, slots, env);

    // Every field first, so a shape that both misnames one method and omits
    // another reads as the two mistakes it is, in the order ADR 0079 § 10's
    // own example prints them.
    let unknown: Vec<String> = named
        .iter()
        .filter(|(name, _)| {
            !resolve_method(&interface, name, env.signatures, env.graph)
                .is_some_and(|(_, sig)| !sig.interface_private)
        })
        .map(|(name, _)| name.clone())
        .collect();
    for name in unknown {
        env.diags.report(
            Diagnostic::error(
                code::E_DOUBLE_METHOD_UNKNOWN,
                format!("`{interface}` declares no method `{name}`"),
            )
            .with_primary(call_span, format!("this stands in for `{interface}`"))
            .with_help(format!(
                "drop the `{name}:` field, or declare `{name}` on `{interface}` \
                 (`rule:testing/doubles`)"
            )),
        );
    }
    check_answer_signatures(&interface, &named, call_span, env);
    if method == "partial" {
        return;
    }

    let mut seen = FxHashSet::default();
    seen.insert(interface.clone());
    let mut owed: Vec<(QName, String)> = Vec::new();
    own_obligations(&interface, env, &mut owed);
    collect_obligations(&interface, env, &mut seen, &mut owed);
    // One diagnostic per *method*, not per declaration that names it: an
    // interface redeclaring a parent's member owes it once, and the author has
    // one field to write either way.
    let mut reported = FxHashSet::default();
    for (declaring, name) in owed {
        if named.iter().any(|(field, _)| *field == name) || !reported.insert(name.clone()) {
            continue;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_DOUBLE_METHOD_MISSING,
                format!("`{declaring}::{name}` is not implemented by this double"),
            )
            .with_primary(call_span, format!("this stands in for `{interface}`"))
            .with_help(format!(
                "give the shape a `{name}:` field whose closure answers it, or build a \
                 `Core\\Test::partial` and delegate it to a real implementation \
                 (`rule:testing/doubles`)"
            )),
        );
    }
}

/// One `E0828` per field of `named` that answers a method `interface` declares
/// with a value that cannot stand in for it — the third of
/// `rule:testing/doubles`' three questions, asked of both members, since a
/// `partial`'s override answers its method exactly as a `double`'s field does.
///
/// A field naming no method of the interface is not asked: `E0826` has already
/// named it, and there is no signature to compare it against.
///
/// # Why a code of its own rather than the argument's own mismatch
///
/// The position a bad answer is written in is `$answers`, and that parameter
/// is declared `object` (`rule:types/object-top`), so
/// [`crate::expr::assign::report_mismatch`] there would report that a shape is
/// not an object — which is false, and names neither the field nor the method
/// it was supposed to answer. The constraint is the interface the *call site*
/// wrote, and this walk is the only place holding both halves of it.
///
/// The relation is ordinary assignability over [`Ty::CallableSig`], so
/// `rule:types/callable-arity`'s prefix arity and
/// `rule:types/callable-variance`'s contravariant parameters and covariant
/// return are what a double is held to, with no comparison of its own: a
/// closure declaring fewer parameters than the method is accepted here for the
/// reason `nvs_runtime::closure` accepts it at the call.
fn check_answer_signatures(
    interface: &QName,
    named: &[(String, TypeId)],
    call_span: Span,
    env: &mut Env<'_>,
) {
    for (name, field) in named {
        let Some((declaring, sig)) = resolve_method(interface, name, env.signatures, env.graph)
        else {
            continue;
        };
        if sig.interface_private {
            continue;
        }
        // Bare `callable` carries no parameter list to compare
        // (`rule:types/callable-is-a-closure`), and that is the accepted case
        // rather than a refusal: the field promises a closure and nothing more,
        // which is what `nvs_runtime::closure` checks a tag at a time.
        if matches!(env.interner.get(*field), Ty::Callable) {
            continue;
        }
        let expected = env.interner.callable_sig(sig.params.clone(), sig.return_ty);
        if is_assignable(*field, expected, env.interner, env.graph, env.signatures) {
            continue;
        }
        let found = env.interner.describe(*field);
        let wanted = env.interner.describe(expected);
        env.diags.report(
            Diagnostic::error(
                code::E_DOUBLE_METHOD_SIGNATURE,
                format!("`{name}:` answers `{declaring}::{name}` with `{found}`"),
            )
            .with_primary(call_span, format!("`{declaring}::{name}` is `{wanted}`"))
            .with_help(format!(
                "write the field as a closure the interface's own call sites can use: it may \
                 declare fewer parameters than `{name}`, but each one it does declare has to \
                 accept what they pass, and its result has to satisfy what they were promised \
                 (`rule:testing/doubles`)"
            )),
        );
    }
}

/// The method names one double's shape of answers implements paired with the
/// type of the field naming each, in the order the shape interned them — which
/// [`crate::ty::TypeInterner::shape`] sorts, so the refusals below it come out
/// in one order for one program.
///
/// The field's own type is carried rather than its name alone because
/// [`check_answer_signatures`] is the one reader that needs both: what a double
/// owes is a method's *signature*, and the shape is where the answer's is
/// written.
///
/// The argument is found through its [`ArgSlot`] rather than by position, so a
/// call writing `answers:` by name is read exactly as the positional spelling
/// is. An argument that filled no parameter, or one whose type is not a shape,
/// answers nothing — see [`check_double_answers`] for what that then means.
fn answered_methods(
    param: usize,
    arg_types: &[TypeId],
    slots: &[ArgSlot],
    env: &Env<'_>,
) -> Vec<(String, TypeId)> {
    let Some(index) = slots.iter().position(|slot| *slot == ArgSlot::Param(param)) else {
        return Vec::new();
    };
    let Some(ty) = arg_types.get(index) else {
        return Vec::new();
    };
    match env.interner.get(*ty) {
        Ty::Shape(fields) => fields
            .iter()
            .map(|field| (field.name.clone(), field.ty))
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use nvs_diagnostics::{Diagnostics, SourceMap, code};
    use nvs_hir::resolve_file;
    use nvs_syntax::parse_file;

    use crate::check::check_program;
    use crate::expr_table::ExprTypeTable;
    use crate::ty::TypeInterner;

    fn check_src(src: &str) -> Diagnostics {
        let mut map = SourceMap::new();
        let file = map.add("t.nvs", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut interner = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        check_program(
            &[crate::ProgramFile {
                src: map.file(file),
                stmts: &stmts,
            }],
            &module,
            &mut interner,
            &mut exprs,
            &mut diags,
        );
        diags
    }

    fn missing(diags: &Diagnostics) -> bool {
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INTERFACE_METHOD_MISSING))
    }

    /// The case this module exists for: `rule:iteration/two-interfaces`'s members are bodiless
    /// by design, so a cursor missing one dispatches to nothing.
    #[test]
    fn a_cursor_class_missing_advance_is_diagnosed() {
        let diags = check_src(
            "<?nvs\n\
             class Nums implements Iterator<int> {\n\
             \x20 function current(): int { return 1; }\n\
             }\n",
        );
        assert!(missing(&diags), "{diags:?}");
    }

    /// ...and one declaring both is clean.
    #[test]
    fn a_cursor_class_declaring_both_members_is_clean() {
        let diags = check_src(
            "<?nvs\n\
             class Nums implements Iterator<int> {\n\
             \x20 function advance(): bool { return false; }\n\
             \x20 function current(): int { return 1; }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// A user-declared interface is checked the same way.
    #[test]
    fn a_user_interfaces_member_is_owed_too() {
        let diags = check_src(
            "<?nvs\n\
             interface Shape { function area(): int; }\n\
             class Square implements Shape {}\n",
        );
        assert!(missing(&diags), "{diags:?}");
    }

    /// `rule:classes/interface-default-methods`: a default body discharges the obligation.
    #[test]
    fn an_interface_default_body_discharges_the_obligation() {
        let diags = check_src(
            "<?nvs\n\
             interface Shape { public function area(): int { return 0; } }\n\
             class Square implements Shape {}\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// Leaving a member to a subclass is what `abstract` means — the
    /// subclass is what owes it.
    #[test]
    fn an_abstract_class_is_exempt_and_its_concrete_subclass_is_not() {
        let clean = check_src(
            "<?nvs\n\
             interface Shape { function area(): int; }\n\
             abstract class Partial implements Shape {}\n\
             class Done extends Partial { function area(): int { return 4; } }\n",
        );
        assert!(!clean.has_errors(), "{clean:?}");

        let owed = check_src(
            "<?nvs\n\
             interface Shape { function area(): int; }\n\
             abstract class Partial implements Shape {}\n\
             class Undone extends Partial {}\n",
        );
        assert!(missing(&owed), "{owed:?}");
    }

    /// A compiler-declared interface owes its members like any other: once
    /// [`crate::iter_lib`] seeds `compareTo`, a class claiming `Comparable`
    /// and declaring nothing is the same `E0449` as a source-declared
    /// interface left unanswered.
    #[test]
    fn a_reserved_interface_owes_its_seeded_members() {
        let owed = check_src("<?nvs\nclass Money implements Comparable {}\n");
        assert!(missing(&owed), "{owed:?}");

        let answered = check_src(
            "<?nvs\n\
             class Money implements Comparable {\n\
             public function compareTo(self $other): int { return 0; }\n\
             }\n",
        );
        assert!(!answered.has_errors(), "{answered:?}");
    }

    /// `Parses` is one required member, so a class claiming it and declaring
    /// nothing owes `parse` by exactly `Comparable`'s route above.
    #[test]
    fn a_class_implementing_parses_and_declaring_no_parse_names_the_member() {
        let owed = check_src("<?nvs\nclass Slug implements Parses {}\n");
        assert!(missing(&owed), "{owed:?}");
        assert!(
            owed.iter().any(|d| d.message.contains("`parse`")),
            "the diagnostic names the member it owes: {owed:?}"
        );
    }

    /// `rule:expressions/try-parse`: `tryParse` is a default body on the
    /// interface, not a second required member, so declaring `parse` alone is
    /// a complete implementation.
    #[test]
    fn a_class_implementing_parses_inherits_try_parse_without_declaring_it() {
        let diags = check_src(
            "<?nvs\n\
             class Slug implements Parses {\n\
             public static function parse(tainted string $s): static { return new static(); }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// ...and writing the second implementation that rule warns about is
    /// latitude, not a diagnostic — the same latitude `Comparable::compareTo`
    /// has.
    #[test]
    fn an_implementor_may_override_try_parse() {
        let diags = check_src(
            "<?nvs\n\
             class Slug implements Parses {\n\
             public static function parse(tainted string $s): static { return new static(); }\n\
             public static function tryParse(tainted string $s): ?Slug { return null; }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    /// `static` is what makes an implementor's `parse` answer its own class at
    /// the binding site — see [`super::reject_dropped_static_return`].
    #[test]
    fn a_parse_returning_something_other_than_static_is_refused() {
        for written in ["Slug", "self", "string"] {
            let refused = check_src(&format!(
                "<?nvs\n\
                 class Slug implements Parses {{\n\
                 public static function parse(tainted string $s): {written} \
                 {{ return new Slug(); }}\n\
                 }}\n",
            ));
            assert!(
                refused
                    .iter()
                    .any(|d| d.code == Some(code::E_INCOMPATIBLE_OVERRIDE)),
                "`{written}` is not `static`: {refused:?}"
            );
        }
    }
}
