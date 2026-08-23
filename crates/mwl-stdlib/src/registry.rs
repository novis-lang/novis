//! The `Core` signature registry: what the compiler resolves a
//! `Core\Class::member(...)` call against.
//!
//! [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md) is
//! authoritative for every signature; this is that file in the one form a
//! compiler can read. A row here is a *promise* — `mwl-types` seeds its own
//! signature table from [`CLASSES`], so a `Core` call goes through exactly the
//! arity check, the assignability check and the `ResolvedCall` recording a
//! user-declared static call already does, with no second code path.
//!
//! # Why a small type enum rather than a type spelling
//!
//! [`CoreTy`] is a closed enum, not a `&'static str` the compiler re-parses.
//! Two reasons, both structural: a spelling would put a second (partial)
//! parser for MWL's type grammar in the build, and a typo in it would be a
//! *runtime* surprise in `mwl check` rather than a compile error here. The
//! cost is that widening the registry to a type this enum cannot express —
//! a union, a nullable, a shape — is a change to this file rather than a
//! string edit, which is the correct amount of friction for something the
//! whole language resolves against.
//!
//! # Known gap
//!
//! The enum covers exactly what the members registered so far need. §§ 1–12
//! of the spec also use unions (`int|string`), nullables (`?T`), `decimal`,
//! shapes (an option bag) and a `callable` — each is a variant to add here
//! plus a lowering arm in `mwl_types`, and none has a member registered yet
//! that would exercise it.

/// One type in a `Core` member's signature.
///
/// Deliberately smaller than `mwl_types::ty::Ty`: this describes what the
/// *spec* wrote, not what the checker interns. `mwl-types` lowers each of
/// these into its own interner, which is where qualifiers, unions and class
/// identity live.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum CoreTy {
    /// `bool`
    Bool,
    /// `int`
    Int,
    /// `uint` — ADR 0007 § 4.
    Uint,
    /// `float`
    Float,
    /// `string`
    Str,
    /// `bytes` — ADR 0009.
    Bytes,
    /// `void`, return position only.
    Void,
    /// `mixed` — ADR 0007 § 3's one unchecked position.
    Mixed,
    /// `array<T>`, whose element type is the wrapped one.
    Array(&'static CoreTy),
    /// A type *variable*, named — `T` in `count(array<T> $a): uint`.
    ///
    /// The spec's `Core\Arr` section states the rule this exists for: "`T` is
    /// a type variable — the stdlib is parametric where user code is not." A
    /// variable is bound by unifying the declared parameter types against the
    /// call's actual argument types and then substituted through the whole
    /// signature; `mwl_types` owns both halves. Nothing user-written can
    /// declare one, which is `.claude/loop-goal.md`'s standing decision that
    /// type variables stay compiler-owned.
    Var(&'static str),
}

/// One `Core` member.
#[derive(Clone, Copy, Debug)]
pub struct CoreMethod {
    /// The member's own name, `camelCase` per ADR 0029.
    pub name: &'static str,
    /// Each parameter's declared type, positional. ADR 0063 R1 puts the
    /// subject first.
    pub params: &'static [CoreTy],
    /// The declared return type.
    pub return_ty: CoreTy,
    /// The linker symbol its implementation is reachable at — what
    /// [`crate::symbols`] hands the JIT and what `mwl-ir` records in the
    /// instruction it lowers a call to. Prefixed `mwl_core_` so a `Core`
    /// member is never mistakable for a `mwl_runtime` primitive in a
    /// disassembly.
    pub symbol: &'static str,
}

/// One `Core` domain class — ADR 0011's "every callable is a class member,"
/// with `Core` as the reserved namespace.
#[derive(Clone, Copy, Debug)]
pub struct CoreClass {
    /// The fully-qualified name, backslash-separated exactly as written in
    /// source (`Core\Arr`).
    pub name: &'static str,
    /// Its members, in the spec's own order.
    pub methods: &'static [CoreMethod],
}

/// Every `Core` class the compiler knows, and every member on it.
///
/// The single home for "does `Core\X::y` exist, and what shape is it" — see
/// [`crate`]'s own known gap 1 for how much of the spec is here so far.
pub const CLASSES: &[CoreClass] = &[CoreClass {
    name: r"Core\Arr",
    methods: &[CoreMethod {
        name: "count",
        params: &[CoreTy::Array(&CoreTy::Var("T"))],
        return_ty: CoreTy::Uint,
        symbol: "mwl_core_arr_count",
    }],
}];

/// Looks a class up by its fully-qualified name.
#[must_use]
pub fn class(name: &str) -> Option<&'static CoreClass> {
    CLASSES.iter().find(|class| class.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every registered name is one the spec's own naming rules allow: a
    /// class under `Core`, a `camelCase` member (ADR 0029), and no leading
    /// underscore anywhere (ADR 0030). Cheap, and it catches a paste error in
    /// a table that will grow to several hundred rows.
    #[test]
    fn every_registered_name_follows_the_casing_rules() {
        for class in CLASSES {
            assert!(
                class.name.starts_with(r"Core\"),
                "{} is not under Core",
                class.name
            );
            for segment in class.name.split('\\') {
                assert!(
                    segment.starts_with(|c: char| c.is_ascii_uppercase()),
                    "{segment} is not PascalCase"
                );
            }
            for method in class.methods {
                assert!(
                    method.name.starts_with(|c: char| c.is_ascii_lowercase()),
                    "{}::{} is not camelCase",
                    class.name,
                    method.name
                );
            }
        }
    }

    /// No class is registered twice — [`class`] returns the first match, so a
    /// duplicate would silently hide every member on the second entry.
    #[test]
    fn no_class_is_registered_twice() {
        let mut names: Vec<&str> = CLASSES.iter().map(|class| class.name).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total);
    }

    #[test]
    fn a_registered_class_is_found_by_name_and_an_unregistered_one_is_not() {
        assert!(class(r"Core\Arr").is_some());
        assert!(class(r"Core\Nope").is_none());
        assert!(class("Arr").is_none());
    }
}
