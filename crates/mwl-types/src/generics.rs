//! Type variables: binding them from a call's arguments, and substituting
//! them back through the signature.
//!
//! [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) parks
//! user-declared generics and `.claude/loop-goal.md` keeps type variables
//! **compiler-owned**, so exactly one thing in the whole compiler produces a
//! [`Ty::TypeVar`]: [`crate::core_lib`] lowering a `mwl_stdlib::registry`
//! signature. The spec's own `Core\Arr` section states why they exist at all
//! — "`T` is a type variable — the stdlib is parametric where user code is
//! not."
//!
//! # What this is, and what it deliberately is not
//!
//! It is *not* a solver. There is no constraint set, no occurs check, no
//! unification variable that can be bound to another unification variable —
//! nothing that could fail to terminate or need an error message of its own.
//! It is one structural walk, in one direction:
//!
//! 1. [`bind`] walks a *declared* parameter type alongside the *actual*
//!    argument type. Wherever the declared side is a variable, that variable
//!    is bound to whatever the actual side is at that position. First binding
//!    wins, so a member declaring the same variable twice
//!    (`contains(array<T> $a, T $needle)`) takes it from the subject, which is
//!    ADR 0063 R1's first parameter and therefore the argument a developer
//!    means to be authoritative.
//! 2. [`substitute`] rewrites the whole signature with those bindings. A
//!    variable no argument bound becomes `mixed` — the honest answer for "this
//!    position's type is unconstrained by this call," and the only one that
//!    keeps a later pass from meeting a variable it has no rule for.
//!
//! The consequence is the property everything downstream relies on:
//! **a type variable never survives a call site.** `crate::expr` substitutes
//! before it checks a single argument or records a `ResolvedCall`, so
//! `mwl-ir`, `mwl-codegen` and every diagnostic only ever see concrete types.
//!
//! # Known gap
//!
//! Binding is *positional and structural*, never inferential: it reads a
//! variable out of a matching position and no further. `map(array<T> $a,
//! callable $f): array<U>` cannot bind `U` from the closure's return type,
//! because a `callable` carries no signature in the type grammar at all
//! (ADR 0027 § 2 keeps it opaque). Such a member's result substitutes to
//! `mixed`, which is correct but weaker than the spec's row promises;
//! closing it needs a typed `callable`, which is its own decision.

use rustc_hash::FxHashMap;

use crate::ty::{Ty, TypeId, TypeInterner};

/// Which concrete type each named variable was bound to.
pub(crate) type Bindings = FxHashMap<String, TypeId>;

/// Whether `id` mentions a type variable anywhere inside it — the test that
/// decides whether a call site needs any of this machinery at all. A
/// user-declared signature never does, so the ordinary path is unchanged.
#[must_use]
pub(crate) fn mentions_type_var(id: TypeId, interner: &TypeInterner) -> bool {
    match interner.get(id) {
        Ty::TypeVar(_) => true,
        Ty::Array(elem) => mentions_type_var(*elem, interner),
        Ty::Union(members) | Ty::Intersection(members) => members
            .iter()
            .any(|member| mentions_type_var(*member, interner)),
        Ty::Shape(fields) => fields
            .iter()
            .any(|(_, field)| mentions_type_var(*field, interner)),
        _ => false,
    }
}

/// Binds every variable `declared` mentions against the matching position in
/// `actual`, recording into `out`. See this module's own docs for the
/// first-binding-wins rule and why there is no failure case: a shape the two
/// sides do not share simply binds nothing, and the unbound variable becomes
/// `mixed` in [`substitute`].
pub(crate) fn bind(declared: TypeId, actual: TypeId, interner: &TypeInterner, out: &mut Bindings) {
    match (interner.get(declared), interner.get(actual)) {
        (Ty::TypeVar(name), _) => {
            out.entry(name.clone()).or_insert(actual);
        }
        (Ty::Array(declared_elem), Ty::Array(actual_elem)) => {
            bind(*declared_elem, *actual_elem, interner, out);
        }
        (Ty::Shape(declared_fields), Ty::Shape(actual_fields)) => {
            let pairs: Vec<(TypeId, TypeId)> = declared_fields
                .iter()
                .filter_map(|(name, declared_field)| {
                    actual_fields
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, actual_field)| (*declared_field, *actual_field))
                })
                .collect();
            for (declared_field, actual_field) in pairs {
                bind(declared_field, actual_field, interner, out);
            }
        }
        _ => {}
    }
}

/// Rewrites `id` with `bindings` applied, interning whatever that produces.
/// A variable with no binding becomes `mixed`; anything mentioning no
/// variable is returned unchanged, so this allocates nothing on the ordinary
/// path.
pub(crate) fn substitute(id: TypeId, bindings: &Bindings, interner: &mut TypeInterner) -> TypeId {
    if !mentions_type_var(id, interner) {
        return id;
    }
    match interner.get(id).clone() {
        Ty::TypeVar(name) => bindings
            .get(&name)
            .copied()
            .unwrap_or_else(|| interner.mixed()),
        Ty::Array(elem) => {
            let elem = substitute(elem, bindings, interner);
            interner.array(elem)
        }
        Ty::Union(members) => {
            let members: Vec<TypeId> = members
                .iter()
                .map(|member| substitute(*member, bindings, interner))
                .collect();
            interner.make_union(members)
        }
        Ty::Intersection(members) => {
            let members: Vec<TypeId> = members
                .iter()
                .map(|member| substitute(*member, bindings, interner))
                .collect();
            interner.make_intersection(members)
        }
        Ty::Shape(fields) => {
            let fields: Vec<(String, TypeId)> = fields
                .iter()
                .map(|(name, field)| (name.clone(), substitute(*field, bindings, interner)))
                .collect();
            interner.shape(fields)
        }
        // Unreachable given the `mentions_type_var` guard above, which is
        // exactly the set of variants handled here — kept total rather than a
        // panic, since `Ty` is `#[non_exhaustive]` and a leaf variant added
        // later genuinely has nothing to substitute.
        _ => id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_variable_binds_to_whatever_it_is_matched_against() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let int = interner.int();
        let mut bindings = Bindings::default();
        bind(t, int, &interner, &mut bindings);
        assert_eq!(bindings.get("T"), Some(&int));
    }

    /// The shape `Core\Arr::count` needs: `array<T>` against `array<int>`.
    #[test]
    fn a_variable_inside_an_array_binds_to_the_element_type() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let declared = interner.array(t);
        let int = interner.int();
        let actual = interner.array(int);
        let mut bindings = Bindings::default();
        bind(declared, actual, &interner, &mut bindings);
        assert_eq!(bindings.get("T"), Some(&int));
        assert_eq!(substitute(declared, &bindings, &mut interner), actual);
    }

    #[test]
    fn the_first_binding_of_a_repeated_variable_wins() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let int = interner.int();
        let string = interner.string();
        let mut bindings = Bindings::default();
        bind(t, int, &interner, &mut bindings);
        bind(t, string, &interner, &mut bindings);
        assert_eq!(bindings.get("T"), Some(&int));
    }

    #[test]
    fn an_unbound_variable_substitutes_to_mixed() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let declared = interner.array(t);
        let bindings = Bindings::default();
        let mixed = interner.mixed();
        let expected = interner.array(mixed);
        assert_eq!(substitute(declared, &bindings, &mut interner), expected);
    }

    /// A mismatched shape binds nothing rather than failing — the "no error
    /// case" property this module's docs claim.
    #[test]
    fn a_non_matching_actual_type_binds_nothing() {
        let mut interner = TypeInterner::new();
        let t = interner.type_var("T");
        let declared = interner.array(t);
        let int = interner.int();
        let mut bindings = Bindings::default();
        bind(declared, int, &interner, &mut bindings);
        assert!(bindings.is_empty());
    }

    #[test]
    fn a_type_with_no_variable_is_returned_unchanged() {
        let mut interner = TypeInterner::new();
        let int = interner.int();
        let array = interner.array(int);
        assert!(!mentions_type_var(array, &interner));
        let bindings = Bindings::default();
        assert_eq!(substitute(array, &bindings, &mut interner), array);
    }
}
