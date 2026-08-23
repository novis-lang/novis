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
//! of the spec also use unions (`int|string`), nullables (`?T`) and `decimal`
//! — each is a variant to add here plus a lowering arm in `mwl_types`, and
//! none has a member registered yet that would exercise it. [`Const`] has the
//! same shape of gap: no `null`, so a spec signature ending `= null` cannot be
//! stated here until `mwl_types::defaults` can emit one.
//!
//! # The options bag
//!
//! [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) R2 makes a
//! trailing options shape (`{step?: int}`) the form of *every* optioned
//! member. It is [`CoreTy::Options`], and the four properties below are what
//! it costs and what it buys — recorded here because the fork is the
//! expensive part, not the code:
//!
//! * **A bag is its own type, not an ADR 0036 shape.** `mwl_types::ty::Ty`
//!   has an `Options` variant beside `Shape`, spellable only from here the
//!   way `TypeVar` already is. Reusing `Shape` would need an `optional` flag
//!   on its fields *and* a `?` in the surface type grammar, and would leave an
//!   unknown option accepted — ADR 0036 § 3's width subtyping allows an extra
//!   field on purpose, while a mistyped option name must be an error.
//! * **A bag is always last and always optional**, because every option is.
//!   Its `MethodSig::defaults` entry is a `ConstArg::Options(...)` carrying
//!   each option's own default, synthesized by `mwl_types::core_lib` from the
//!   type itself — so a row never states the bag twice, `MethodSig::required()`
//!   already excludes it, and the arity check needed no change at all.
//! * **A bag flattens at the ABI.** `mwl_ir::lower::lower_call_args` expands
//!   it into one argument per declared option, in the order [`CoreOption`]s
//!   are written here — the literal's value where written, the option's
//!   default where not — so `mwl_core_arr_range` is an ordinary `args: [3]`
//!   helper and no runtime representation of a shape exists. The rejected
//!   alternative was building an `array<mixed>` per call: it allocates on the
//!   common path, and needs a `null`/empty spelling the IR does not have.
//! * **The cost is one restriction:** an options argument must be written as a
//!   shape literal at the call site, or omitted — a diagnostic, never silence.
//!   That is exactly the set of programs that can run today, since
//!   `ExprKind::ObjectLiteral` has no lowering of its own at all.

/// One type in a `Core` member's signature.
///
/// Deliberately smaller than `mwl_types::ty::Ty`: this describes what the
/// *spec* wrote, not what the checker interns. `mwl-types` lowers each of
/// these into its own interner, which is where qualifiers, unions and class
/// identity live.
// No `PartialEq`/`Eq`: [`Self::Options`] carries [`CoreOption`]s, which carry
// [`Const`]s, which carry an `f64` — and there is nothing here to compare
// anyway, since a registry row is matched structurally and interned into
// `mwl_types::ty::Ty` before any consumer asks whether two types are equal.
#[derive(Clone, Copy, Debug)]
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
    /// `A|B|...` — ADR 0007 § 3's union, at least two members.
    ///
    /// **Parameter position only.** A helper's argument slot is a whole
    /// `mwl_runtime::Value`, and `mwl-codegen` writes its tag from the
    /// argument's own representation, so a union parameter needs no IR type of
    /// its own and the body decodes by tag. A union *return* would hand the
    /// caller a value whose representation `mwl_ir::ty::Ty::Mixed`'s own doc
    /// comment records as still undecided, so no row states one —
    /// `a_union_is_only_ever_a_parameter` holds that.
    Union(&'static [CoreTy]),
    /// ADR 0063 R2's trailing options shape — `{step?: int}`, one
    /// [`CoreOption`] per declared option, in the order the ABI passes them.
    /// See this module's own docs for why it is its own type rather than a
    /// [`CoreTy`] wrapping an ADR 0036 shape.
    ///
    /// Only ever the **last** entry of [`CoreMethod::params`], and never
    /// listed in [`CoreMethod::defaults`]: every option has a default of its
    /// own, so the bag itself is optional by construction rather than by
    /// declaration. `an_options_bag_is_last_and_never_empty` holds both.
    Options(&'static [CoreOption]),
}

/// One option inside a [`CoreTy::Options`] bag: its name, its type, and the
/// value a call that leaves it out passes.
///
/// The default is stated here rather than in [`CoreMethod::defaults`] because
/// an option is named, not positional — there is no end-alignment rule that
/// could relate a run of defaults to a set of names, and stating it beside the
/// name is the only arrangement in which the two cannot drift apart.
#[derive(Clone, Copy, Debug)]
pub struct CoreOption {
    /// The option's own name, `camelCase` per ADR 0029 — what a call site
    /// writes on the left of the `:` in `{step: 2}`.
    pub name: &'static str,
    /// Its declared type. Never itself a [`CoreTy::Options`]: a bag flattens
    /// to one ABI argument per option, and a nested one would have nothing to
    /// flatten into.
    pub ty: CoreTy,
    /// The constant a call that omits this option passes — materialized at the
    /// call site by `mwl_ir::lower::lower_call_args`, exactly as an omitted
    /// positional parameter's default is.
    pub default: Const,
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
    /// subject first, and R2 puts an options bag ([`CoreTy::Options`]) last if
    /// the member has one.
    pub params: &'static [CoreTy],
    /// Defaults for the *trailing* optional positional parameters, aligned to
    /// the end of [`Self::positional`] — so `positional().len() -
    /// defaults.len()` is how many arguments a call must supply, and an empty
    /// slice means every positional parameter is required.
    ///
    /// A trailing options bag is excluded on both sides of that subtraction:
    /// it is optional by construction and carries its own per-option defaults,
    /// so it never appears here. See [`CoreTy::Options`].
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

impl CoreMethod {
    /// This member's trailing options bag, or `None` for a member with none —
    /// the one place the "always last" rule of [`CoreTy::Options`] is read,
    /// so no consumer scans [`Self::params`] for it a second time.
    #[must_use]
    pub fn options(&self) -> Option<&'static [CoreOption]> {
        match self.params.last() {
            Some(CoreTy::Options(options)) => Some(options),
            _ => None,
        }
    }

    /// The positional parameters — [`Self::params`] without a trailing options
    /// bag. What [`Self::defaults`] aligns to the end of.
    #[must_use]
    pub fn positional(&self) -> &'static [CoreTy] {
        match self.options() {
            Some(_) => &self.params[..self.params.len() - 1],
            None => self.params,
        }
    }
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
                name: "split",
                params: &[CoreTy::Str, CoreTy::Str, CoreTy::Options(SPLIT_OPTIONS)],
                defaults: &[],
                return_ty: CoreTy::Array(&CoreTy::Str),
                symbol: "mwl_core_str_split",
            },
            CoreMethod {
                name: "replace",
                params: &[
                    CoreTy::Str,
                    CoreTy::Str,
                    CoreTy::Str,
                    CoreTy::Options(REPLACE_OPTIONS),
                ],
                defaults: &[],
                return_ty: CoreTy::Str,
                symbol: "mwl_core_str_replace",
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
                name: "trim",
                params: &[CoreTy::Str, CoreTy::Options(TRIM_OPTIONS)],
                defaults: &[],
                return_ty: CoreTy::Str,
                symbol: "mwl_core_str_trim",
            },
            CoreMethod {
                name: "trimStart",
                params: &[CoreTy::Str, CoreTy::Options(TRIM_OPTIONS)],
                defaults: &[],
                return_ty: CoreTy::Str,
                symbol: "mwl_core_str_trim_start",
            },
            CoreMethod {
                name: "trimEnd",
                params: &[CoreTy::Str, CoreTy::Options(TRIM_OPTIONS)],
                defaults: &[],
                return_ty: CoreTy::Str,
                symbol: "mwl_core_str_trim_end",
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
            CoreMethod {
                name: "hasKey",
                params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Union(ARRAY_KEY)],
                defaults: &[],
                return_ty: CoreTy::Bool,
                symbol: "mwl_core_arr_has_key",
            },
            CoreMethod {
                name: "range",
                params: &[CoreTy::Int, CoreTy::Int, CoreTy::Options(RANGE_OPTIONS)],
                defaults: &[],
                return_ty: CoreTy::Array(&CoreTy::Int),
                symbol: "mwl_core_arr_range",
            },
        ],
    },
];

/// `int|string` — ADR 0007 § 5's two array-key types, which the spec's § 2
/// writes at every member taking or producing a key.
const ARRAY_KEY: &[CoreTy] = &[CoreTy::Int, CoreTy::Str];

/// `Core\Str::split`'s `{limit?: int}` — `crate::str::mwl_core_str_split`'s
/// own docs own what each sign of it means and why the default is `int`'s
/// maximum.
const SPLIT_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "limit",
    ty: CoreTy::Int,
    default: Const::Int(i64::MAX),
}];

/// `Core\Str::trim`/`trimStart`/`trimEnd`'s `{characters?: string}`, shared by
/// all three — one bag, so the three members cannot drift apart on either the
/// option's name or its default.
///
/// The default is PHP's own `trim` set: space, tab, newline, carriage return,
/// NUL and vertical tab. `crate::str::trimmed` owns the two places the match
/// itself diverges from PHP's.
const TRIM_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "characters",
    ty: CoreTy::Str,
    default: Const::Str(" \t\n\r\0\u{0b}"),
}];

/// `Core\Str::replace`'s `{caseInsensitive?: bool, limit?: uint}`.
///
/// `crate::str::mwl_core_str_replace`'s own docs own both defaults — in
/// particular why "every occurrence" is spelled as `uint`'s maximum rather
/// than as a sentinel `0` or a `null` the registry cannot state yet.
const REPLACE_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "caseInsensitive",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
    CoreOption {
        name: "limit",
        ty: CoreTy::Uint,
        default: Const::Uint(u64::MAX),
    },
];

/// `Core\Arr::range`'s `{step?: int}` — the first options bag in the roster.
///
/// A named constant rather than an inline slice because a bag is referenced
/// twice in practice: once as a parameter type here, and once by
/// `crate::arr`'s own doc comment naming what its flattened arguments are.
const RANGE_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "step",
    ty: CoreTy::Int,
    default: Const::Int(1),
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

    /// No member declares more defaults than it has positional parameters —
    /// [`CoreMethod::defaults`] is aligned to the end of
    /// [`CoreMethod::positional`], so a longer slice has nowhere to align to
    /// and would make the required count underflow.
    #[test]
    fn no_member_declares_more_defaults_than_parameters() {
        for class in CLASSES {
            for method in class.methods {
                assert!(
                    method.defaults.len() <= method.positional().len(),
                    "{}::{} declares {} defaults for {} positional parameters",
                    class.name,
                    method.name,
                    method.defaults.len(),
                    method.positional().len()
                );
            }
        }
    }

    /// ADR 0063 R2, mechanically: at most one options bag per member, always
    /// last, never empty, and never nested inside another type. Every one of
    /// those is load-bearing — [`CoreMethod::options`] reads only the last
    /// parameter, and `mwl_types::core_lib` synthesizes exactly one
    /// `ConstArg::Options` entry from it.
    #[test]
    fn an_options_bag_is_last_and_never_empty() {
        for class in CLASSES {
            for method in class.methods {
                for (index, param) in method.params.iter().enumerate() {
                    let CoreTy::Options(options) = param else {
                        continue;
                    };
                    assert_eq!(
                        index,
                        method.params.len() - 1,
                        "{}::{} puts its options bag at {index} of {} parameters",
                        class.name,
                        method.name,
                        method.params.len()
                    );
                    assert!(
                        !options.is_empty(),
                        "{}::{} declares an empty options bag",
                        class.name,
                        method.name
                    );
                    for option in *options {
                        assert!(
                            option.name.starts_with(|c: char| c.is_ascii_lowercase()),
                            "{}::{}'s option `{}` is not camelCase",
                            class.name,
                            method.name,
                            option.name
                        );
                        assert!(
                            !matches!(option.ty, CoreTy::Options(_)),
                            "{}::{}'s option `{}` nests a second bag",
                            class.name,
                            method.name,
                            option.name
                        );
                    }
                }
            }
        }
    }

    /// A union is a **parameter** type and nothing else — see
    /// [`CoreTy::Union`], which owns why: a helper's argument slot is a tagged
    /// value written from the argument's own representation, while its result
    /// has to land in a caller-side value whose representation is still open.
    /// Checked over return types and array element types alike, since either
    /// would reach the caller.
    #[test]
    fn a_union_is_only_ever_a_parameter() {
        fn reaches_the_caller(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::Union(_) => true,
                CoreTy::Array(elem) => reaches_the_caller(elem),
                _ => false,
            }
        }
        for class in CLASSES {
            for method in class.methods {
                assert!(
                    !reaches_the_caller(&method.return_ty),
                    "{}::{} returns a union",
                    class.name,
                    method.name
                );
                for member in method.options().unwrap_or(&[]) {
                    assert!(
                        !matches!(member.ty, CoreTy::Union(_)),
                        "{}::{}'s option `{}` is a union, which no option-bag \
                         flattening rule covers",
                        class.name,
                        method.name,
                        member.name
                    );
                }
            }
        }
    }

    /// A union has at least two members — a one-member union is that member,
    /// and the interner collapses it, so writing one here would be a row that
    /// does not say what it looks like it says.
    #[test]
    fn a_union_has_at_least_two_members() {
        for class in CLASSES {
            for method in class.methods {
                for param in method.params {
                    if let CoreTy::Union(members) = param {
                        assert!(
                            members.len() >= 2,
                            "{}::{} declares a {}-member union",
                            class.name,
                            method.name,
                            members.len()
                        );
                    }
                }
            }
        }
    }

    /// The one member that has a bag today, spelled out — so a paste error
    /// that dropped `step` would fail here rather than only at
    /// `examples/core.mwl`.
    #[test]
    fn range_declares_one_step_option_defaulting_to_one() {
        let range = class(r"Core\Arr")
            .expect(r"Core\Arr is registered")
            .methods
            .iter()
            .find(|method| method.name == "range")
            .expect("range is registered");
        assert_eq!(range.positional().len(), 2);
        let options = range.options().expect("range takes an options bag");
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].name, "step");
        assert!(matches!(options[0].default, Const::Int(1)));
    }

    #[test]
    fn a_registered_class_is_found_by_name_and_an_unregistered_one_is_not() {
        assert!(class(r"Core\Arr").is_some());
        assert!(class(r"Core\Nope").is_none());
        assert!(class("Arr").is_none());
    }
}
