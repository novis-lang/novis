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
//! of the spec also use nullables (`?T`) and `decimal` — each is a variant to
//! add here plus a lowering arm in `mwl_types`, and neither has a member
//! registered yet that would exercise it.
//!
//! # A `Core` enum is declared here too
//!
//! The spec's own tables name enums as well as members — `Core\Order` at
//! § 2's *Ordering* is the first — so [`ENUMS`] is a second roster beside
//! [`CLASSES`], and [`CoreTy::Enum`] refers to one by name. Its own doc
//! comment owns why the two are separate; what belongs here is that
//! `mwl_types::enums` seeds them into the *same* table a declared `enum` goes
//! into, so nothing downstream of that point can tell the two apart.
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
    /// `callable`, plus the name of the type variable its **result** binds —
    /// `U` in `map(array<T> $a, callable $fn): array<U>`.
    ///
    /// The same opaque `callable` at the call site: it constrains nothing a
    /// [`Self::Callable`] parameter does not, and a closure value satisfies it
    /// by ADR 0027 § 2 exactly as before. What it adds is a *binding site* for
    /// a variable that appears at no argument position at all — `U` is the type
    /// of a value the callback produces, which the argument's own type
    /// (`callable`, and opaque) cannot say. `mwl_types::generics` binds it from
    /// the closure literal's recorded return type and owns the one case that
    /// still binds nothing: an argument that is not a written `fn` literal.
    ///
    /// **Parameter position only, and never nested.** It is a declaration of
    /// where a variable comes from, so it means nothing inside an
    /// [`Self::Array`], a [`Self::Union`], a [`CoreOption`] or a return type —
    /// `a_callback_result_type_is_only_ever_a_whole_parameter` holds that.
    CallableTo(&'static str),
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
    /// A `Core`-owned enum, named by its fully-qualified name — `Core\Order`
    /// in `sort(array<T> $a, {order?: Order, ...})`.
    ///
    /// The name is resolved against [`ENUMS`], not against the compiled
    /// program: an enum the spec's own tables name is part of `Core`'s
    /// surface exactly as a member is, so it is declared here and seeded into
    /// the checker's enum table alongside the user's own — see
    /// [`ENUMS`] for why that is one table rather than two.
    ///
    /// Carries no backing type. ADR 0010 § 2 makes `int` the default and
    /// there is no reason for a `Core` enum to be anything else: nothing
    /// stores one, so the only thing a `uint` backing could buy is a case
    /// past `i64::MAX`.
    Enum(&'static str),
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
    /// `null` — an option that was **not given**.
    ///
    /// ADR 0063 R2 makes every option optional, but the spec writes several
    /// whose type has no "absent" value in it: `Core\Arr::sort`'s
    /// `{by?: callable, comparator?: callable}` are the first two — a
    /// `callable` cannot be a "no callback" callable, and inventing a
    /// do-nothing one would silently change the answer. So the *declared*
    /// type stays what a call site may write, and the default an omitting
    /// call site passes is this: the helper reads `Tag::Null` and takes its
    /// own not-given path.
    ///
    /// This is the only default whose value is not of its option's declared
    /// type, and deliberately: `?callable` would be a union, and
    /// `a_union_is_only_ever_a_parameter` records why an option cannot be
    /// one.
    Null,
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
    /// A [`CoreTy::Enum`] case, by enum name and case name — the default for
    /// an option whose type is a `Core` enum.
    ///
    /// Named rather than written as the integer it is so that the default and
    /// the case cannot drift apart: ADR 0010 § 3 makes a case an integer
    /// constant, and [`ENUMS`] is the one place that constant is stated.
    /// `mwl_types::core_lib` resolves it there; `every_enum_case_default_names_a_real_case`
    /// holds that it resolves at all.
    EnumCase(&'static str, &'static str),
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
                name: "map",
                params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::CallableTo("U")],
                defaults: &[],
                return_ty: CoreTy::Array(&CoreTy::Var("U")),
                symbol: "mwl_core_arr_map",
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
                name: "isList",
                params: &[CoreTy::Array(&CoreTy::Var("T"))],
                defaults: &[],
                return_ty: CoreTy::Bool,
                symbol: "mwl_core_arr_is_list",
            },
            CoreMethod {
                name: "values",
                params: &[CoreTy::Array(&CoreTy::Var("T"))],
                defaults: &[],
                return_ty: CoreTy::Array(&CoreTy::Var("T")),
                symbol: "mwl_core_arr_values",
            },
            CoreMethod {
                name: "sort",
                params: &[
                    CoreTy::Array(&CoreTy::Var("T")),
                    CoreTy::Options(SORT_OPTIONS),
                ],
                defaults: &[],
                return_ty: CoreTy::Array(&CoreTy::Var("T")),
                symbol: "mwl_core_arr_sort",
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

/// `Core\Arr::sort`'s
/// `{by?: callable, order?: Order, comparator?: callable, preserveKeys?: bool}`
/// — the eleven PHP sort functions plus `array_multisort` in one bag, which is
/// what the spec's § 2 *Ordering* note means by "descending is
/// `{order: Order::Desc}`, key-preservation is an option rather than a letter
/// in the name."
///
/// `crate::arr::mwl_core_arr_sort`'s own docs own what each option does and
/// which combinations are refused. Two things about the *declaration* belong
/// here: `by` and `comparator` are the first options whose default is
/// [`Const::Null`] (there is no "no callback" callable — see that variant),
/// and `order` is the first use of [`CoreTy::Enum`].
const SORT_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "by",
        ty: CoreTy::Callable,
        default: Const::Null,
    },
    CoreOption {
        name: "order",
        ty: CoreTy::Enum(r"Core\Order"),
        default: Const::EnumCase(r"Core\Order", "Asc"),
    },
    CoreOption {
        name: "comparator",
        ty: CoreTy::Callable,
        default: Const::Null,
    },
    CoreOption {
        name: "preserveKeys",
        ty: CoreTy::Bool,
        default: Const::Bool(false),
    },
];

/// One `Core`-owned enum — [ADR 0010](../../../../docs/adr/0010-enums-are-a-value-type.md)'s
/// closed, named integer type, declared here rather than in MWL source.
#[derive(Clone, Copy, Debug)]
pub struct CoreEnum {
    /// The fully-qualified name, backslash-separated exactly as written in
    /// source (`Core\Order`).
    pub name: &'static str,
    /// Its cases, in declaration order. The value is each case's own
    /// constant, written out rather than auto-incremented: ADR 0010 § 1's
    /// auto-increment is a *source* convenience, and a table read by the
    /// compiler has nothing to gain from re-deriving what it could state.
    pub cases: &'static [(&'static str, i64)],
}

/// Every enum `Core` owns.
///
/// A second roster beside [`CLASSES`] rather than a member of it, because an
/// enum is not a class: [ADR 0011](../../../../docs/adr/0011-functions-and-constants-are-class-members.md)
/// puts every *callable* on a class, and an enum has none. `mwl_types::enums`
/// seeds its own table from this, so `Core\Order::Desc` resolves to an integer
/// constant through exactly the machinery a user-declared `enum` already goes
/// through — the same "seed a table rather than special-case `Core`" rule
/// `mwl_types::core_lib` states for members.
///
/// The spec's § 2 names a second one, `SetOn { Values, Keys, Both }`. It is
/// deliberately absent: no member that takes it is registered yet, and an
/// entry here is reachable from source the moment it exists.
pub const ENUMS: &[CoreEnum] = &[CoreEnum {
    // docs/spec/01-core-library.md § 2 *Ordering*: "Enums: `Order { Asc, Desc
    // }`". Ascending is `0` so that it is also the value a `{order: ...}` an
    // author never writes ends up meaning.
    name: r"Core\Order",
    cases: &[("Asc", 0), ("Desc", 1)],
}];

/// Looks a class up by its fully-qualified name.
#[must_use]
pub fn class(name: &str) -> Option<&'static CoreClass> {
    CLASSES.iter().find(|class| class.name == name)
}

/// Looks a `Core`-owned enum up by its fully-qualified name.
#[must_use]
pub fn core_enum(name: &str) -> Option<&'static CoreEnum> {
    ENUMS.iter().find(|found| found.name == name)
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

    /// [`CoreTy::CallableTo`] is a *binding site*, so it only means anything
    /// as a whole parameter: nested in an array, a union or an option it would
    /// name a variable nothing ever binds, and in return position it would name
    /// one at the moment it is meant to be read.
    #[test]
    fn a_callback_result_type_is_only_ever_a_whole_parameter() {
        fn nests_one(ty: &CoreTy) -> bool {
            match ty {
                CoreTy::CallableTo(_) => true,
                CoreTy::Array(elem) => nests_one(elem),
                CoreTy::Union(members) => members.iter().any(nests_one),
                CoreTy::Options(options) => options.iter().any(|option| nests_one(&option.ty)),
                _ => false,
            }
        }
        for class in CLASSES {
            for method in class.methods {
                assert!(
                    !nests_one(&method.return_ty),
                    "{}::{} returns a callback result type",
                    class.name,
                    method.name
                );
                for param in method.params {
                    if matches!(param, CoreTy::CallableTo(_)) {
                        continue;
                    }
                    assert!(
                        !nests_one(param),
                        "{}::{} nests a callback result type inside a parameter",
                        class.name,
                        method.name
                    );
                }
            }
        }
    }

    /// The variable a [`CoreTy::CallableTo`] binds is one the member actually
    /// reads back — a row naming `U` in the callback and `V` in the result
    /// would type-check every call to `mixed` with nothing to say why.
    #[test]
    fn a_callback_result_variable_is_mentioned_by_the_return_type() {
        fn mentions(ty: &CoreTy, name: &str) -> bool {
            match ty {
                CoreTy::Var(var) => *var == name,
                CoreTy::Array(elem) => mentions(elem, name),
                CoreTy::Union(members) => members.iter().any(|member| mentions(member, name)),
                _ => false,
            }
        }
        for class in CLASSES {
            for method in class.methods {
                for param in method.params {
                    let CoreTy::CallableTo(name) = param else {
                        continue;
                    };
                    assert!(
                        mentions(&method.return_ty, name),
                        "{}::{} binds `{name}` from its callback but never returns it",
                        class.name,
                        method.name
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

    /// A `Core` enum's name and cases follow ADR 0029's casing rules too —
    /// `PascalCase` for both, since a case is a type-level name (§ 1's
    /// enum-case row), not a member.
    #[test]
    fn every_core_enum_name_and_case_follows_the_casing_rules() {
        for declared in ENUMS {
            assert!(
                declared.name.starts_with(r"Core\"),
                "{} is not under Core",
                declared.name
            );
            assert!(!declared.cases.is_empty(), "{} has no cases", declared.name);
            for (case, _) in declared.cases {
                assert!(
                    case.starts_with(|c: char| c.is_ascii_uppercase()),
                    "{}::{case} is not PascalCase",
                    declared.name
                );
            }
        }
    }

    /// Every [`Const::EnumCase`] default names an enum this crate registers
    /// and a case that enum actually has — the check that keeps a default and
    /// its case from drifting apart, since `mwl_types::core_lib` resolves one
    /// against the other and panics if it cannot.
    #[test]
    fn every_enum_case_default_names_a_real_case() {
        for class in CLASSES {
            for method in class.methods {
                for option in method.options().unwrap_or(&[]) {
                    let Const::EnumCase(name, case) = option.default else {
                        continue;
                    };
                    let declared = core_enum(name).unwrap_or_else(|| {
                        panic!(
                            "{}::{} defaults `{}` to an unregistered enum `{name}`",
                            class.name, method.name, option.name
                        )
                    });
                    assert!(
                        declared.cases.iter().any(|(found, _)| *found == case),
                        "{}::{} defaults `{}` to `{name}::{case}`, which is not a case",
                        class.name,
                        method.name,
                        option.name
                    );
                }
            }
        }
    }

    /// An option typed as a `Core` enum names one this crate registers — the
    /// name is resolved rather than declared, so a typo would otherwise intern
    /// a type nothing can ever produce a value of.
    #[test]
    fn every_enum_typed_option_names_a_registered_enum() {
        for class in CLASSES {
            for method in class.methods {
                for option in method.options().unwrap_or(&[]) {
                    if let CoreTy::Enum(name) = option.ty {
                        assert!(
                            core_enum(name).is_some(),
                            "{}::{}'s option `{}` is typed as the unregistered enum `{name}`",
                            class.name,
                            method.name,
                            option.name
                        );
                    }
                }
            }
        }
    }

    /// `sort`'s bag spelled out, in ABI order — the one member whose options
    /// are all four kinds at once: two absent-by-default callbacks, an enum
    /// and a `bool`. The order is what
    /// `mwl_ir::lower::Lowering::lower_options_arg` flattens into, so a
    /// reordering here is a silently wrong call rather than a build failure.
    #[test]
    fn sort_declares_its_four_options_in_abi_order() {
        let sort = class(r"Core\Arr")
            .expect(r"Core\Arr is registered")
            .methods
            .iter()
            .find(|method| method.name == "sort")
            .expect("sort is registered");
        assert_eq!(sort.positional().len(), 1);
        let options = sort.options().expect("sort takes an options bag");
        let names: Vec<&str> = options.iter().map(|option| option.name).collect();
        assert_eq!(names, vec!["by", "order", "comparator", "preserveKeys"]);
        assert!(matches!(options[0].default, Const::Null));
        assert!(matches!(
            options[1].default,
            Const::EnumCase(r"Core\Order", "Asc")
        ));
        assert!(matches!(options[2].default, Const::Null));
        assert!(matches!(options[3].default, Const::Bool(false)));
    }

    #[test]
    fn a_registered_class_is_found_by_name_and_an_unregistered_one_is_not() {
        assert!(class(r"Core\Arr").is_some());
        assert!(class(r"Core\Nope").is_none());
        assert!(class("Arr").is_none());
    }
}
