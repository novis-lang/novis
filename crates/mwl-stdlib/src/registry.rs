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
//! of the spec also use unions (`int|string`), nullables (`?T`), `decimal` and
//! shapes (an option bag) — each is a variant to add here plus a lowering arm
//! in `mwl_types`, and none has a member registered yet that would exercise
//! it. [`Const`] has the same shape of gap: no `null`, so a spec signature
//! ending `= null` cannot be stated here until `mwl_types::defaults` can emit
//! one.

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
    /// `callable` — [ADR 0031](../../../../docs/adr/0031-callable-is-the-only-closure-type.md)
    /// § 4's one closure type, and opaque: it says nothing about the
    /// parameters or the result of the closure that satisfies it. What a
    /// `Core` member actually hands a callback is stated by
    /// [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
    /// § 2 — `($value, $key)`, with fewer parameters allowed — and enforced
    /// at the call by `mwl_runtime::call_closure`, not by this type.
    Callable,
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

/// One optional parameter's default value.
///
/// The registry's counterpart of `mwl_types::defaults::ConstArg`, kept
/// separate for the reason [`CoreTy`] is kept separate from
/// `mwl_types::ty::Ty`: this states what the *spec* wrote, and `mwl-types`
/// translates it into the one representation the checker and `mwl-ir` share.
/// Only the shapes that enum can already emit are expressible — a member whose
/// spec signature defaults to `null` cannot be registered until
/// `mwl_types::defaults` grows that variant, which is exactly the friction
/// this crate wants around a signature the whole language resolves against.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Const {
    /// A `bool` default.
    Bool(bool),
    /// An `int` default.
    Int(i64),
    /// A `uint` default.
    Uint(u64),
    /// A `float` default.
    Float(f64),
    /// A `string` default, already cooked — a registry row writes the bytes it
    /// means, so there is no escape grammar here at all.
    Str(&'static str),
}

/// One `Core` member.
#[derive(Clone, Copy, Debug)]
pub struct CoreMethod {
    /// The member's own name, `camelCase` per ADR 0029.
    pub name: &'static str,
    /// Each parameter's declared type, positional. ADR 0063 R1 puts the
    /// subject first.
    pub params: &'static [CoreTy],
    /// Defaults for the *trailing* optional parameters, aligned to the end of
    /// [`Self::params`] — so `params.len() - defaults.len()` is how many
    /// arguments a call must supply, and an empty slice means every parameter
    /// is required.
    ///
    /// Aligned to the end rather than carrying one entry per parameter because
    /// that is the only arrangement the language allows: a required parameter
    /// can never follow an optional one (`E_PARAM_DEFAULT_ORDER`), so a
    /// per-parameter list would be a run of `None` followed by a run of `Some`
    /// and every row would spell out the `None`s.
    pub defaults: &'static [Const],
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
pub const CLASSES: &[CoreClass] = &[
    CoreClass {
        name: r"Core\Str",
        methods: &[
            CoreMethod {
                name: "isEmpty",
                params: &[CoreTy::Str],
                defaults: &[],
                return_ty: CoreTy::Bool,
                symbol: "mwl_core_str_is_empty",
            },
            CoreMethod {
                name: "contains",
                params: &[CoreTy::Str, CoreTy::Str],
                defaults: &[],
                return_ty: CoreTy::Bool,
                symbol: "mwl_core_str_contains",
            },
            CoreMethod {
                name: "startsWith",
                params: &[CoreTy::Str, CoreTy::Str],
                defaults: &[],
                return_ty: CoreTy::Bool,
                symbol: "mwl_core_str_starts_with",
            },
            CoreMethod {
                name: "endsWith",
                params: &[CoreTy::Str, CoreTy::Str],
                defaults: &[],
                return_ty: CoreTy::Bool,
                symbol: "mwl_core_str_ends_with",
            },
            CoreMethod {
                name: "join",
                params: &[CoreTy::Array(&CoreTy::Str), CoreTy::Str],
                defaults: &[Const::Str("")],
                return_ty: CoreTy::Str,
                symbol: "mwl_core_str_join",
            },
            CoreMethod {
                name: "padStart",
                params: &[CoreTy::Str, CoreTy::Uint, CoreTy::Str],
                defaults: &[Const::Str(" ")],
                return_ty: CoreTy::Str,
                symbol: "mwl_core_str_pad_start",
            },
            CoreMethod {
                name: "padEnd",
                params: &[CoreTy::Str, CoreTy::Uint, CoreTy::Str],
                defaults: &[Const::Str(" ")],
                return_ty: CoreTy::Str,
                symbol: "mwl_core_str_pad_end",
            },
            CoreMethod {
                name: "repeat",
                params: &[CoreTy::Str, CoreTy::Uint],
                defaults: &[],
                return_ty: CoreTy::Str,
                symbol: "mwl_core_str_repeat",
            },
            CoreMethod {
                name: "lower",
                params: &[CoreTy::Str],
                defaults: &[],
                return_ty: CoreTy::Str,
                symbol: "mwl_core_str_lower",
            },
            CoreMethod {
                name: "upper",
                params: &[CoreTy::Str],
                defaults: &[],
                return_ty: CoreTy::Str,
                symbol: "mwl_core_str_upper",
            },
            CoreMethod {
                name: "upperFirst",
                params: &[CoreTy::Str],
                defaults: &[],
                return_ty: CoreTy::Str,
                symbol: "mwl_core_str_upper_first",
            },
            CoreMethod {
                name: "lowerFirst",
                params: &[CoreTy::Str],
                defaults: &[],
                return_ty: CoreTy::Str,
                symbol: "mwl_core_str_lower_first",
            },
        ],
    },
    CoreClass {
        name: r"Core\Arr",
        methods: &[
            CoreMethod {
                name: "count",
                params: &[CoreTy::Array(&CoreTy::Var("T"))],
                defaults: &[],
                return_ty: CoreTy::Uint,
                symbol: "mwl_core_arr_count",
            },
            CoreMethod {
                name: "filter",
                params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Callable],
                defaults: &[],
                return_ty: CoreTy::Array(&CoreTy::Var("T")),
                symbol: "mwl_core_arr_filter",
            },
            CoreMethod {
                name: "isEmpty",
                params: &[CoreTy::Array(&CoreTy::Var("T"))],
                defaults: &[],
                return_ty: CoreTy::Bool,
                symbol: "mwl_core_arr_is_empty",
            },
        ],
    },
];

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

    /// No member declares more defaults than it has parameters —
    /// [`CoreMethod::defaults`] is aligned to the end of `params`, so a longer
    /// slice has nowhere to align to and would make the required count
    /// underflow.
    #[test]
    fn no_member_declares_more_defaults_than_parameters() {
        for class in CLASSES {
            for method in class.methods {
                assert!(
                    method.defaults.len() <= method.params.len(),
                    "{}::{} declares {} defaults for {} parameters",
                    class.name,
                    method.name,
                    method.defaults.len(),
                    method.params.len()
                );
            }
        }
    }

    #[test]
    fn a_registered_class_is_found_by_name_and_an_unregistered_one_is_not() {
        assert!(class(r"Core\Arr").is_some());
        assert!(class(r"Core\Nope").is_none());
        assert!(class("Arr").is_none());
    }
}
