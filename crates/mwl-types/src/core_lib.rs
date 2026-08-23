//! Seeding the checker's signature table with `Core`.
//!
//! `mwl_stdlib::registry` is the one home for what a `Core` member's signature
//! *is*; this is the one place that turns it into the same
//! [`ClassSignature`](crate::signatures::ClassSignature) a user-declared class
//! produces. Everything after that point is unchanged machinery:
//! `resolve_method` finds `Core\Arr::count` the way it finds `Animal::name`,
//! the arity and assignability checks are the same ones, and the recorded
//! `ResolvedCall` has the same shape — which is the whole reason to seed a
//! table rather than special-case `Core` at each call site.
//!
//! Two properties fall out of that and are worth naming, because a
//! special-cased design would have had to state them:
//!
//! * A `Core` member is `static` and never `private` — [`seed`] sets both, so
//!   `Core\Arr::count($a)` is the only reachable spelling and
//!   `$a->count()` resolves to nothing, as ADR 0063 R20 ("no operation is
//!   reachable two ways") requires.
//! * A name `mwl_stdlib` does not register does not exist. Before this
//!   existed, `mwl_hir::QName::is_core` made every `Core\…` reference trusted
//!   and unchecked; a registered class is now checked like any other, while
//!   an unregistered one stays trusted so the rest of the spec's §§ 1–12 can
//!   still be *written* in a fixture before it is implemented. That trust is
//!   the thing to remove once the registry is complete.

use mwl_hir::QName;
use mwl_stdlib::registry::{CLASSES, CoreTy};
use rustc_hash::FxHashMap;

use crate::signatures::{MethodSig, SignatureTable};
use crate::ty::{TypeId, TypeInterner};

/// Adds every `mwl_stdlib::registry::CLASSES` entry to `table`.
///
/// Called once, at the head of
/// [`build_signatures`](crate::signatures::build_signatures), so a `Core`
/// signature is in place before the first user declaration is collected and
/// long before any body is checked.
pub(crate) fn seed(table: &mut SignatureTable, interner: &mut TypeInterner) {
    for class in CLASSES {
        let qname = QName::parse(class.name);
        let mut methods = FxHashMap::default();
        for method in class.methods {
            methods.insert(
                method.name.to_owned(),
                MethodSig {
                    params: method
                        .params
                        .iter()
                        .map(|param| lower(param, interner))
                        .collect(),
                    variadic: false,
                    return_ty: lower(&method.return_ty, interner),
                    is_static: true,
                    interface_private: false,
                    // Native Rust behind a helper symbol, not a compiled MWL
                    // function — but it is code, so a call never needs to go
                    // looking for an override.
                    has_body: true,
                },
            );
        }
        table.seed_class(qname, FxHashMap::default(), methods);
    }
}

/// The symbol a resolved `Core` call is reachable at, or `None` if `qname`
/// names no registered class or `method` no registered member.
///
/// `mwl-ir` reads this to lower a resolved static call whose target is a
/// `Core` member into the helper-shaped instruction that reaches it — the one
/// piece of a `Core` call that is genuinely not the same as a user-declared
/// one, since there is no compiled MWL function to name.
#[must_use]
pub fn symbol_of(qname: &QName, method: &str) -> Option<&'static str> {
    let class = mwl_stdlib::registry::class(&qname.to_string())?;
    class
        .methods
        .iter()
        .find(|candidate| candidate.name == method)
        .map(|found| found.symbol)
}

/// One registry type into an interned one. The registry's enum is
/// deliberately smaller than [`crate::ty::Ty`] — see its own docs — so this
/// is a total match with no failure case.
fn lower(ty: &CoreTy, interner: &mut TypeInterner) -> TypeId {
    match ty {
        CoreTy::Bool => interner.bool_ty(),
        CoreTy::Int => interner.int(),
        CoreTy::Uint => interner.uint(),
        CoreTy::Float => interner.float(),
        CoreTy::Str => interner.string(),
        CoreTy::Bytes => interner.bytes(),
        CoreTy::Void => interner.void(),
        CoreTy::Array(elem) => {
            let elem = lower(elem, interner);
            interner.array(elem)
        }
        CoreTy::Var(name) => interner.type_var(*name),
        // `Mixed` and anything a later registry variant adds: `mixed` is the
        // registry's own "unchecked position" spelling, and is the only safe
        // answer for a variant this arm has not learned yet, since `Ty` and
        // `CoreTy` are both `#[non_exhaustive]`.
        _ => interner.mixed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::resolve_method;
    use mwl_hir::ClassGraph;

    #[test]
    fn a_registered_member_resolves_with_its_declared_shape() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);

        let (owner, sig) = resolve_method(
            &QName::parse(r"Core\Arr"),
            "count",
            &table,
            &ClassGraph::default(),
        )
        .expect("Core\\Arr::count is registered");
        assert_eq!(owner.to_string(), r"Core\Arr");
        assert!(sig.is_static);
        assert!(!sig.variadic);
        assert_eq!(sig.params.len(), 1);
        assert_eq!(interner.describe(sig.params[0]), "array<T>");
        assert_eq!(interner.describe(sig.return_ty), "uint");
    }

    #[test]
    fn an_unregistered_member_resolves_to_nothing() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        assert!(
            resolve_method(
                &QName::parse(r"Core\Arr"),
                "nope",
                &table,
                &ClassGraph::default()
            )
            .is_none()
        );
    }

    #[test]
    fn a_registered_member_names_the_symbol_its_implementation_lives_at() {
        assert_eq!(
            symbol_of(&QName::parse(r"Core\Arr"), "count"),
            Some("mwl_core_arr_count")
        );
        assert_eq!(symbol_of(&QName::parse(r"Core\Arr"), "nope"), None);
        assert_eq!(symbol_of(&QName::parse("Animal"), "count"), None);
    }
}
