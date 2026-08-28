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
//! - **A class using ADR 0043 § 4's `by $field` delegation is exempt, whole.**
//!   Delegation supplies members from a property's own type, so checking such
//!   a class would report members the synthesized forwards provide. It is
//!   exempt *whole* rather than per-member because the one thing that would
//!   make the check exact — § 4's `E_DELEGATE_TYPE_MISMATCH`, "does `$field`'s
//!   type satisfy this interface" — is not built yet: without it, a member no
//!   forward covers is indistinguishable from one whose field cannot answer
//!   it. [`resolve_delegations`] runs in that exemption's place, and its own
//!   doc comment owns what a forward is and which members get one.
//!
//! The four compiler-declared global interfaces are *inside* it, and reach it
//! the same way a source-declared one does: [`crate::iter_lib`] seeds every
//! member on [`nvs_hir::interfaces`]'s roster, so `implements Comparable`
//! owes `compareTo` and `implements Stringable` owes `toString` here, rather
//! than only at the use sites (`crate::expr`'s `require_stringable` and the
//! object comparison check) that were the whole of the guarantee before.

use nvs_diagnostics::{Diagnostic, code};
use nvs_hir::QName;
use nvs_syntax::ast::{ClassDecl, Modifier};
use rustc_hash::FxHashSet;

use crate::Env;
use crate::expr_table::Delegation;
use crate::signatures::resolve_method;
use crate::{span_text, strip_sigil};

/// Reports one `E0449` per member of `decl`'s interfaces that nothing
/// answers. See the module docs for the three exemptions.
pub(crate) fn check_class_conformance(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    if decl.modifiers.contains(&Modifier::Abstract) {
        return;
    }
    if decl.implements.iter().any(|c| c.by_field.is_some()) {
        resolve_delegations(decl, qname, env);
        return;
    }

    let mut seen = FxHashSet::default();
    let mut owed: Vec<(QName, String)> = Vec::new();
    collect_obligations(qname, env, &mut seen, &mut owed);

    for (interface, method) in owed {
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
/// twice. Each still reaches `nvs_runtime::nvs_abstract_method` if it is
/// called; the handoff's backlog names all three.
fn resolve_delegations(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    let Some(links) = env.graph.get(qname) else {
        return;
    };
    // The graph drops an `implements` entry that resolved to nothing, so the
    // two lists only line up on a program with no name-resolution error in it.
    // One that has one is already reported and will never be lowered, so there
    // is nothing to resolve for it — and guessing which clause is which would
    // synthesize a forward against the wrong interface.
    if links.implements.len() != decl.implements.len() {
        return;
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
            if sig.is_static || sig.variadic || sig.inout.iter().any(|inout| *inout) {
                continue;
            }
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
