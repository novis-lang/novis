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
//!   Delegation supplies members from a property's own type, and nothing
//!   resolves that yet (the plan lists `by`-delegation as open), so checking
//!   such a class would report members delegation is meant to provide.
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
use crate::signatures::resolve_method;

/// Reports one `E0449` per member of `decl`'s interfaces that nothing
/// answers. See the module docs for the three exemptions.
pub(crate) fn check_class_conformance(decl: &ClassDecl, qname: &QName, env: &mut Env<'_>) {
    if decl.modifiers.contains(&Modifier::Abstract) {
        return;
    }
    if decl.implements.iter().any(|c| c.by_field.is_some()) {
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
        if let Some(sig) = env.signatures.get(&parent) {
            let mut names: Vec<&String> = sig
                .methods
                .iter()
                .filter(|(_, m)| !m.has_body && !m.interface_private)
                .map(|(name, _)| name)
                .collect();
            names.sort();
            out.extend(names.into_iter().map(|n| (parent.clone(), n.clone())));
        }
        collect_obligations(&parent, env, seen, out);
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
