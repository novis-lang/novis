//! Interface conformance: every method a class's interfaces declare without
//! a body, the class has to answer.
//!
//! # Why this exists now
//!
//! It was harmless while nothing dispatched through an interface. ADR 0053
//! § 1's `Iterator<T>` changed that: its members are bodiless *on purpose*
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
//! *does* have a body — its own, an inherited one, or an ADR 0043 § 2
//! interface default.
//!
//! Two things are outside it, each for its own reason:
//!
//! - **An `abstract` class is exempt.** Leaving a member to a subclass is
//!   what the modifier means.
//! - **A member ADR 0043 § 4's `by $field` delegation supplies is exempt, one
//!   member at a time.** Delegation answers a member from a property's own
//!   type, so a synthesized forward discharges the obligation exactly as a
//!   written body does — but only for the members that actually get one.
//!   The exemption is per-member rather than whole-class because § 4's own
//!   first bullet is checked now: [`check_delegate_field`] asks whether
//!   `$field` is a declared property of a non-nullable type that satisfies
//!   the interface (`E0720`), so a member no forward covers can be told from
//!   one whose field could not have answered it. [`resolve_delegations`] is
//!   what runs both halves, and its own doc comment owns what a forward is
//!   and which members get one.
//!
//! The four compiler-declared global interfaces are *inside* it, and reach it
//! the same way a source-declared one does: [`crate::iter_lib`] seeds every
//! member on [`nvs_hir::interfaces`]'s roster, so `implements Comparable`
//! owes `compareTo` and `implements Stringable` owes `toString` here, rather
//! than only at the use sites (`crate::expr`'s `require_stringable` and the
//! object comparison check) that were the whole of the guarantee before.

use nvs_diagnostics::{Diagnostic, code};
use nvs_hir::{QName, implements_interface};
use nvs_syntax::ast::{ClassDecl, Modifier};
use rustc_hash::FxHashSet;

use crate::Env;
use crate::expr_table::Delegation;
use crate::signatures::{resolve_method, resolve_property};
use crate::ty::Ty;
use crate::{span_text, strip_sigil};

/// Reports one `E0449` per member of `decl`'s interfaces that nothing
/// answers. See the module docs for the two exemptions.
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
        let answered = resolve_method(qname, &method, env.signatures, env.graph)
            .is_some_and(|(_, sig)| sig.has_body);
        if answered {
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
                 (ADR 0043 § 2)"
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
/// makes the compiler synthesize a forward for — ADR 0043 § 4.
///
/// The obligation set is [`collect_obligations`]', asked of the *interface*
/// rather than of the class: every bodiless, non-`interface_private` member it
/// and its own ancestors declare. A member the class already answers with a
/// body is skipped, which is § 4's "a class may still write its own method
/// with the same name as a delegated one — that is an ordinary override" — and
/// the same subtraction covers an inherited body and an ADR 0043 § 2 interface
/// default, both of which `resolve_method` finds and neither of which a
/// forward should displace.
///
/// Three member shapes get no forward, and each is a hole rather than a rule:
/// a `static` member has no receiver to forward through (ADR 0008 gives class
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
                        "write `{method}` on `{qname}` by hand — ADR 0043 § 4 already lets an own \
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
            });
        }
    }
    Some(covered)
}

/// ADR 0043 § 4 bullet 1: whether `field` can carry `interface`'s members at
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
                "declare a property `${field}` typed `{interface}` (ADR 0043 § 4)"
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
             (ADR 0043 § 4)"
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

    /// The case this module exists for: ADR 0053 § 1's members are bodiless
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

    /// ADR 0043 § 2: a default body discharges the obligation.
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
}
