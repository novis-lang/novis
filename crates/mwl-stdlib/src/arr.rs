//! `Core\Arr` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 2, over `mwl_runtime`'s insertion-ordered, copy-on-write `array<T>`.
//!
//! Every member here is pure (ADR 0063 R3) and borrows its subject rather
//! than consuming it — see [`crate`]'s own docs for why that falls out of
//! being a helper rather than being a rule this module states.
//!
//! # `array<T|U>` is written literally, not flattened to `array<mixed>`
//!
//! [ADR 0069](../../../../docs/adr/0069-array-combination-is-key-type-independent.md)
//! § 1 writes all four combination members as `array<T|U>`, and the registry
//! can state exactly that: a union is legal in either direction
//! ([`crate::registry::CoreTy::Union`]), so the return type is
//! [`COMBINED`] rather than the `array<mixed>` that would be the safe
//! fallback. It is never worse and usually better — `overlay` over two
//! `array<string>`s answers `array<string>`, which a call site can index and
//! read without a cast, where `array<mixed>` would force one at every use.
//!
//! What that costs is a *shape* rule the spec's signature already implies and
//! this is the one place it is written down: `T` binds from the base and `U`
//! from the **first** trailing layer (`mwl_types::generics` binds
//! first-occurrence-wins and substitutes before it checks a single argument),
//! so every later layer is checked against the first layer's element type.
//! Combining three arrays of three unrelated element types is therefore a
//! type error rather than an `array<mixed>`; a call that means it widens the
//! layers to `array<mixed>` itself, which element-covariance-on-read makes
//! free.
//!
//! # `on` selects, `by` maps what it selected
//!
//! `diff` and `intersect` take `{on?: SetOn, by?: callable, comparator?:
//! callable}`, and the twelve PHP functions they replace differ only in how
//! those three interact. The rule, decided here because neither the spec nor
//! ADR 0069 states it: **`on` selects the part of an entry that is compared —
//! the value, the key, or both — and `by` and `comparator` apply to the part
//! it selected.** So a `by` under `SetOn::Keys` replaces the *key*, not the
//! value; it is never an option that could not change the answer.
//! [`comparison_subject`] and [`set_member`] hold the mechanics and the cost.

use mwl_runtime::{Decimal, Fault, MwlArray, MwlStr, Tag, Value};

use crate::ordering::compare_values;
use crate::registry::{Const, CoreClass, CoreEnum, CoreMethod, CoreOption, CoreTy};

// ============================================================================
// Registration — this class's rows, its enum, and where its symbols live
// ============================================================================

/// `Core\Arr`'s registry rows, in the spec's own order.
///
/// Declared beside the implementations rather than in one flat table, so
/// adding a member touches this file and nothing else. [`crate::registry`]'s
/// `CLASSES` lists this const; that list grows one line per *class*, never one
/// per member.
pub const CLASS: CoreClass = CoreClass {
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
            name: "find",
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Callable],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_find",
        },
        CoreMethod {
            name: "findKey",
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Callable],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "mwl_core_arr_find_key",
        },
        CoreMethod {
            name: "any",
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Callable],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_arr_any",
        },
        CoreMethod {
            name: "all",
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Callable],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_arr_all",
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
            name: "contains",
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_arr_contains",
        },
        CoreMethod {
            name: "keyOf",
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "mwl_core_arr_key_of",
        },
        CoreMethod {
            name: "isList",
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_arr_is_list",
        },
        CoreMethod {
            name: "keys",
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "mwl_core_arr_keys",
        },
        CoreMethod {
            name: "values",
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_values",
        },
        CoreMethod {
            name: "first",
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_first",
        },
        CoreMethod {
            name: "last",
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_last",
        },
        CoreMethod {
            name: "firstKey",
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "mwl_core_arr_first_key",
        },
        CoreMethod {
            name: "lastKey",
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "mwl_core_arr_last_key",
        },
        CoreMethod {
            name: "slice",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Int,
                CoreTy::Nullable(&CoreTy::Int),
                CoreTy::Options(PRESERVE_KEYS),
            ],
            defaults: &[Const::Null],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_slice",
        },
        CoreMethod {
            name: "chunk",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Uint,
                CoreTy::Options(PRESERVE_KEYS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Array(&CoreTy::Var("T"))),
            symbol: "mwl_core_arr_chunk",
        },
        CoreMethod {
            name: "append",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Variadic(&CoreTy::Var("T")),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_append",
        },
        CoreMethod {
            name: "prepend",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Variadic(&CoreTy::Var("T")),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_prepend",
        },
        CoreMethod {
            name: "withoutFirst",
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_without_first",
        },
        CoreMethod {
            name: "withoutLast",
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_without_last",
        },
        CoreMethod {
            name: "padStart",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Uint,
                CoreTy::Var("T"),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_pad_start",
        },
        CoreMethod {
            name: "padEnd",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Uint,
                CoreTy::Var("T"),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_pad_end",
        },
        CoreMethod {
            name: "reverse",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Options(PRESERVE_KEYS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_reverse",
        },
        CoreMethod {
            name: "flip",
            params: &[CoreTy::Array(&CoreTy::Union(ARRAY_KEY))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "mwl_core_arr_flip",
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
            name: "fill",
            params: &[CoreTy::Uint, CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_fill",
        },
        CoreMethod {
            name: "fillKeys",
            params: &[CoreTy::Array(&CoreTy::Union(ARRAY_KEY)), CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_fill_keys",
        },
        CoreMethod {
            name: "range",
            params: &[CoreTy::Int, CoreTy::Int, CoreTy::Options(RANGE_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Int),
            symbol: "mwl_core_arr_range",
        },
        CoreMethod {
            name: "fromKeysAndValues",
            params: &[
                CoreTy::Array(&CoreTy::Union(ARRAY_KEY)),
                CoreTy::Array(&CoreTy::Var("T")),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_from_keys_and_values",
        },
        CoreMethod {
            name: "overlay",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Variadic(&CoreTy::Array(&CoreTy::Var("U"))),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Union(COMBINED)),
            symbol: "mwl_core_arr_overlay",
        },
        CoreMethod {
            name: "overlayDeep",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Variadic(&CoreTy::Array(&CoreTy::Var("U"))),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Union(COMBINED)),
            symbol: "mwl_core_arr_overlay_deep",
        },
        CoreMethod {
            name: "underlay",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Variadic(&CoreTy::Array(&CoreTy::Var("U"))),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Union(COMBINED)),
            symbol: "mwl_core_arr_underlay",
        },
        CoreMethod {
            name: "appendAll",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Variadic(&CoreTy::Array(&CoreTy::Var("U"))),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Union(COMBINED)),
            symbol: "mwl_core_arr_append_all",
        },
        CoreMethod {
            name: "diff",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Options(SET_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_diff",
        },
        CoreMethod {
            name: "intersect",
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Options(SET_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_intersect",
        },
        CoreMethod {
            name: "countBy",
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Options(BY_OPTION)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Uint),
            symbol: "mwl_core_arr_count_by",
        },
        CoreMethod {
            name: "unique",
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Options(BY_OPTION)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_unique",
        },
        CoreMethod {
            name: "min",
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_min",
        },
        CoreMethod {
            name: "max",
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "mwl_core_arr_max",
        },
        CoreMethod {
            name: "sum",
            params: &[CoreTy::Array(&CoreTy::Union(crate::math::NUMBER))],
            defaults: &[],
            return_ty: CoreTy::Union(crate::math::NUMBER),
            symbol: "mwl_core_arr_sum",
        },
        CoreMethod {
            name: "product",
            params: &[CoreTy::Array(&CoreTy::Union(crate::math::NUMBER))],
            defaults: &[],
            return_ty: CoreTy::Union(crate::math::NUMBER),
            symbol: "mwl_core_arr_product",
        },
        CoreMethod {
            name: "average",
            params: &[CoreTy::Array(&CoreTy::Union(crate::math::NUMBER))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Union(QUOTIENT)),
            symbol: "mwl_core_arr_average",
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `float|decimal` — what dividing spec § 2's `int|float|decimal` by a count
/// can land on, and the whole of what `average` answers under its `?`.
///
/// `int` is not on it, deliberately: an average is a quotient, and ADR 0007
/// § 4 already makes `int / int` yield `int|float` rather than an `int`. A
/// `decimal` subject stays exact (its quotient is a `decimal`), which is the
/// reason this is a union rather than plain `float`.
const QUOTIENT: &[CoreTy] = &[CoreTy::Float, CoreTy::Decimal];

/// `Core\Order` — the enum [`mwl_core_arr_sort`]'s `{order: ...}` option takes.
///
/// Declared here, beside its only consumer, for the same reason [`CLASS`] is.
/// [`crate::registry`]'s `ENUMS` lists it; the value of each case is written
/// out rather than auto-incremented, and `Asc` is `0` so it is also what an
/// omitted `{order: ...}` ends up meaning.
pub const ORDER: CoreEnum = CoreEnum {
    name: r"Core\Order",
    cases: &[("Asc", 0), ("Desc", 1)],
};

/// `Core\SetOn` — the enum [`mwl_core_arr_diff`] and [`mwl_core_arr_intersect`]
/// take, and the whole of what twelve PHP `array_diff*`/`array_intersect*`
/// functions differed by.
///
/// Declared here beside its two consumers, exactly as [`ORDER`] is, and
/// `Values` is `0` so it is also what an omitted `{on: ...}` ends up meaning.
/// This module's own docs hold what each case compares and how `by` and
/// `comparator` compose with it.
pub const SET_ON: CoreEnum = CoreEnum {
    name: r"Core\SetOn",
    cases: &[("Values", 0), ("Keys", 1), ("Both", 2)],
};

/// `int|string` — ADR 0007 § 5's two array-key types, which the spec's § 2
/// writes at every member taking or producing a key.
const ARRAY_KEY: &[CoreTy] = &[CoreTy::Int, CoreTy::Str];

/// `T|U` — the element type ADR 0069's four combination members answer, and
/// the spelling this module's own docs record as the one the registry can
/// state.
///
/// A union is legal in either direction ([`CoreTy::Union`]), so the spec's
/// `array<T|U>` is written literally rather than flattened to `array<mixed>`.
/// `T` binds from the base and `U` from the *first* trailing layer, and
/// `mwl_types::generics` substitutes both before a single argument is checked
/// — so every later layer is checked against the first one's type, and the
/// union the return type names is exactly what the result holds. A call with
/// no layers at all leaves `U` unbound, which substitutes to `mixed`, and
/// `T|mixed` is the honest answer for a combination whose second half the
/// call did not write.
const COMBINED: &[CoreTy] = &[CoreTy::Var("T"), CoreTy::Var("U")];

/// `{preserveKeys?: bool}` — the bag the spec's § 2 *Structure* rows share.
///
/// The default is `false` everywhere it appears, and it means what that
/// section says it means: **every** key is discarded and the result renumbered
/// from `"0"`, rather than PHP's renumber-integers-keep-strings, which is the
/// key-type-dependent behaviour
/// [ADR 0069](../../../../docs/adr/0069-array-combination-is-key-type-independent.md)
/// § 3 removes.
const PRESERVE_KEYS: &[CoreOption] = &[CoreOption {
    name: "preserveKeys",
    ty: CoreTy::Bool,
    default: Const::Bool(false),
}];

/// `Core\Arr::range`'s `{step?: int}` — the first options bag in the roster.
///
/// A named constant rather than an inline slice because a bag is referenced
/// twice in practice: once as a parameter type here, and once by this module's
/// own doc comment naming what its flattened arguments are.
const RANGE_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "step",
    ty: CoreTy::Int,
    default: Const::Int(1),
}];

/// `Core\Arr::countBy`'s `{by?: callable}` — the spec's § 2 *Combining* bag
/// that `unique`, `diff` and `intersect` share the `by` half of.
///
/// The same option [`SORT_OPTIONS`] declares and the same default: there is no
/// "no callback" callable, so absence is [`Const::Null`] and the body decodes
/// it through [`optional_callback`].
const BY_OPTION: &[CoreOption] = &[CoreOption {
    name: "by",
    ty: CoreTy::Callable,
    default: Const::Null,
}];

/// `Core\Arr::sort`'s
/// `{by?: callable, order?: Order, comparator?: callable, preserveKeys?: bool}`
/// — the eleven PHP sort functions plus `array_multisort` in one bag, which is
/// what the spec's § 2 *Ordering* note means by "descending is
/// `{order: Order::Desc}`, key-preservation is an option rather than a letter
/// in the name."
///
/// [`mwl_core_arr_sort`]'s own docs own what each option does and which
/// combinations are refused. Two things about the *declaration* belong here:
/// `by` and `comparator` are the first options whose default is
/// [`Const::Null`] (there is no "no callback" callable), and `order` is the
/// first use of [`CoreTy::Enum`].
/// `{on?: SetOn, by?: callable, comparator?: callable}` — the bag the spec's
/// § 2 set members share, in the order the ABI passes them.
///
/// `by` and `comparator` default to [`Const::Null`] for the reason that
/// variant's own docs give: a `callable` has no "no callback" value, so the
/// declared type stays what a call site may write and the helper reads
/// `Tag::Null` for not-given.
const SET_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "on",
        ty: CoreTy::Enum(r"Core\SetOn"),
        default: Const::EnumCase(r"Core\SetOn", "Values"),
    },
    CoreOption {
        name: "by",
        ty: CoreTy::Callable,
        default: Const::Null,
    },
    CoreOption {
        name: "comparator",
        ty: CoreTy::Callable,
        default: Const::Null,
    },
];

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

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain.
///
/// [`crate::symbols`] chains one of these per domain, so a new class adds an
/// arm here rather than to a single workspace-wide match.
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "mwl_core_arr_count" => (mwl_core_arr_count as *const ()).cast(),
        "mwl_core_arr_filter" => (mwl_core_arr_filter as *const ()).cast(),
        "mwl_core_arr_map" => (mwl_core_arr_map as *const ()).cast(),
        "mwl_core_arr_is_empty" => (mwl_core_arr_is_empty as *const ()).cast(),
        "mwl_core_arr_has_key" => (mwl_core_arr_has_key as *const ()).cast(),
        "mwl_core_arr_is_list" => (mwl_core_arr_is_list as *const ()).cast(),
        "mwl_core_arr_keys" => (mwl_core_arr_keys as *const ()).cast(),
        "mwl_core_arr_values" => (mwl_core_arr_values as *const ()).cast(),
        "mwl_core_arr_reverse" => (mwl_core_arr_reverse as *const ()).cast(),
        "mwl_core_arr_flip" => (mwl_core_arr_flip as *const ()).cast(),
        "mwl_core_arr_sort" => (mwl_core_arr_sort as *const ()).cast(),
        "mwl_core_arr_range" => (mwl_core_arr_range as *const ()).cast(),
        "mwl_core_arr_slice" => (mwl_core_arr_slice as *const ()).cast(),
        "mwl_core_arr_chunk" => (mwl_core_arr_chunk as *const ()).cast(),
        "mwl_core_arr_append" => (mwl_core_arr_append as *const ()).cast(),
        "mwl_core_arr_prepend" => (mwl_core_arr_prepend as *const ()).cast(),
        "mwl_core_arr_without_first" => (mwl_core_arr_without_first as *const ()).cast(),
        "mwl_core_arr_without_last" => (mwl_core_arr_without_last as *const ()).cast(),
        "mwl_core_arr_from_keys_and_values" => {
            (mwl_core_arr_from_keys_and_values as *const ()).cast()
        }
        "mwl_core_arr_pad_start" => (mwl_core_arr_pad_start as *const ()).cast(),
        "mwl_core_arr_pad_end" => (mwl_core_arr_pad_end as *const ()).cast(),
        "mwl_core_arr_fill" => (mwl_core_arr_fill as *const ()).cast(),
        "mwl_core_arr_fill_keys" => (mwl_core_arr_fill_keys as *const ()).cast(),
        "mwl_core_arr_overlay" => (mwl_core_arr_overlay as *const ()).cast(),
        "mwl_core_arr_overlay_deep" => (mwl_core_arr_overlay_deep as *const ()).cast(),
        "mwl_core_arr_underlay" => (mwl_core_arr_underlay as *const ()).cast(),
        "mwl_core_arr_append_all" => (mwl_core_arr_append_all as *const ()).cast(),
        "mwl_core_arr_diff" => (mwl_core_arr_diff as *const ()).cast(),
        "mwl_core_arr_intersect" => (mwl_core_arr_intersect as *const ()).cast(),
        "mwl_core_arr_count_by" => (mwl_core_arr_count_by as *const ()).cast(),
        "mwl_core_arr_first" => (mwl_core_arr_first as *const ()).cast(),
        "mwl_core_arr_last" => (mwl_core_arr_last as *const ()).cast(),
        "mwl_core_arr_first_key" => (mwl_core_arr_first_key as *const ()).cast(),
        "mwl_core_arr_last_key" => (mwl_core_arr_last_key as *const ()).cast(),
        "mwl_core_arr_find" => (mwl_core_arr_find as *const ()).cast(),
        "mwl_core_arr_find_key" => (mwl_core_arr_find_key as *const ()).cast(),
        "mwl_core_arr_any" => (mwl_core_arr_any as *const ()).cast(),
        "mwl_core_arr_all" => (mwl_core_arr_all as *const ()).cast(),
        "mwl_core_arr_contains" => (mwl_core_arr_contains as *const ()).cast(),
        "mwl_core_arr_key_of" => (mwl_core_arr_key_of as *const ()).cast(),
        "mwl_core_arr_unique" => (mwl_core_arr_unique as *const ()).cast(),
        "mwl_core_arr_min" => (mwl_core_arr_min as *const ()).cast(),
        "mwl_core_arr_max" => (mwl_core_arr_max as *const ()).cast(),
        "mwl_core_arr_sum" => (mwl_core_arr_sum as *const ()).cast(),
        "mwl_core_arr_product" => (mwl_core_arr_product as *const ()).cast(),
        "mwl_core_arr_average" => (mwl_core_arr_average as *const ()).cast(),
        _ => return None,
    })
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::count(array<T> $a): uint` — how many entries the array
    /// holds, replacing PHP's `count`/`sizeof`.
    ///
    /// `uint` rather than `int` because a count cannot be negative and ADR
    /// 0007 § 4 gives MWL a type that says so; the conversion below cannot
    /// fail, since `mwl_array_count` counts live entries of an allocation
    /// that fits in memory.
    fn mwl_core_arr_count(_ctx, args: [1]) {
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::count expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live \
                      allocation, so it is live for the length of this call"
        )]
        let count = unsafe { mwl_runtime::mwl_array_count(array) };
        Ok(Value::uint(count.cast_unsigned()))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::isEmpty(array<T> $a): bool` — whether the array holds no
    /// entries, replacing PHP's `empty($a)` and the `count($a) === 0` idiom.
    ///
    /// A member of its own rather than left to `count(…) === 0` because ADR
    /// 0063 R20's "no operation reachable two ways" is about *spellings the
    /// library offers*, and the spec's § 2 table lists this one: the question
    /// "is it empty" is answered without the caller having to know that a
    /// count is `uint` and therefore needs `0` written as one.
    fn mwl_core_arr_is_empty(_ctx, args: [1]) {
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::isEmpty expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        #[expect(
            unsafe_code,
            reason = "a Tag::Array argument owns a reference to a live \
                      allocation, so it is live for the length of this call"
        )]
        let count = unsafe { mwl_runtime::mwl_array_count(array) };
        Ok(Value::bool(count == 0))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::filter(array<T> $a, callable $predicate): array<T>` — the
    /// entries the predicate answers truthily for, replacing PHP's
    /// `array_filter` and both of its flags.
    ///
    /// **Keys are preserved**, exactly as `array_filter` preserves them: the
    /// spec's § 2 table names no `preserveKeys` option here, and a filtered
    /// result whose keys silently renumbered would make this the one member
    /// that changes what a following `Core\Arr::keys` answers.
    ///
    /// The predicate receives `($value, $key)` and may declare fewer
    /// parameters — the rule that removes `ARRAY_FILTER_USE_KEY` and
    /// `ARRAY_FILTER_USE_BOTH`. This member always offers both;
    /// `mwl_runtime::call_closure` trims them to what the closure wants, and
    /// is also where the retain/release around the call lives.
    ///
    /// Truthiness is [ADR 0035](../../../docs/adr/0035-truthy-boolean-context.md)'s
    /// table through `mwl_runtime::value_truthy`, so a predicate returning
    /// `0`, `""` or an empty array behaves here exactly as it would in a
    /// condition.
    ///
    /// The first `Core` member to call back into MWL code, and therefore the
    /// first that can fail partway through. Nothing special is needed for
    /// that: `MwlArray` and `MwlStr` both release on drop, so the partial
    /// result and the entry's own key are freed by the early return itself.
    fn mwl_core_arr_filter(ctx, args: [2]) {
        let subject = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::filter expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;

        let mut kept = MwlArray::new();
        let mut from = 0usize;
        loop {
            #[expect(
                unsafe_code,
                reason = "a Tag::Array argument owns a reference to a live \
                          allocation, so it is live for the length of this \
                          call, and `from` only ever advances past a slot \
                          this same cursor reported"
            )]
            let (slot, key, value) = unsafe {
                let slot = mwl_runtime::mwl_array_next_slot(subject, from);
                let Ok(slot) = usize::try_from(slot) else {
                    break;
                };
                let key = MwlStr::from_raw(mwl_runtime::mwl_array_key_at(subject, slot));
                let mut value = Value::null();
                mwl_runtime::mwl_array_value_at(subject, slot, &raw mut value);
                (slot, key, value)
            };
            from = slot + 1;

            // One reference for the duration of the call, released right
            // after: `call_closure` takes its own, and `key` itself is still
            // owed to either `kept` or its own drop below.
            let key_arg = Value::str(key.clone());
            let verdict = mwl_runtime::call_closure(ctx, args[1], &[value, key_arg]);
            #[expect(
                unsafe_code,
                reason = "this frame owns exactly the reference `key.clone()` \
                          just produced"
            )]
            unsafe {
                key_arg.release();
            }
            let verdict = verdict?;
            let truthy = mwl_runtime::value_truthy(verdict);
            #[expect(
                unsafe_code,
                reason = "the verdict is a fresh value this frame owns; a \
                          predicate returning a heap value would otherwise \
                          leak one reference per entry"
            )]
            unsafe {
                verdict.release();
            }

            if truthy {
                #[expect(
                    unsafe_code,
                    reason = "the entry is owned by the subject array, which \
                              outlives this call, so the copy stored here \
                              needs a reference of its own"
                )]
                unsafe {
                    value.retain();
                }
                kept.set(key, value);
            }
        }
        Ok(Value::array(kept))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::map(array<T> $a, callable $fn): array<U>` — every entry
    /// replaced by what the callback answers for it, replacing PHP's
    /// `array_map`.
    ///
    /// **Keys are preserved**, which is PHP's own single-array behaviour and
    /// the one `Core\Arr::filter` already keeps: `array_map` renumbers only in
    /// its multi-array form, which ADR 0063 R20 leaves no room for anyway.
    /// Re-keying is `mapKeys`, its own member in the spec's § 2 table.
    ///
    /// The callback receives `($value, $key)` and may declare fewer parameters,
    /// the same rule and the same `mwl_runtime::call_closure` trimming
    /// [`mwl_core_arr_filter`] documents.
    ///
    /// **The `U` in the signature is real.** `map`'s result type is the
    /// callback's own return type, bound at the call site from the `fn`
    /// literal's recorded return — `mwl_types::generics` owns that rule and the
    /// one argument shape that still leaves it `mixed`. Nothing here depends on
    /// it: the helper stores whatever `Value` the callback produced.
    ///
    /// The mapped value is *owned* by this frame — `call_closure` returns one
    /// fresh reference — so it is stored without a retain and never released.
    /// That is the difference from `filter`, which stores a value belonging to
    /// the subject array and therefore has to retain one first.
    fn mwl_core_arr_map(ctx, args: [2]) {
        let subject = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::map expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;

        let mut out = MwlArray::new();
        let mut from = 0usize;
        loop {
            #[expect(
                unsafe_code,
                reason = "a Tag::Array argument owns a reference to a live \
                          allocation, so it is live for the length of this \
                          call, and `from` only ever advances past a slot \
                          this same cursor reported"
            )]
            let (slot, key, value) = unsafe {
                let slot = mwl_runtime::mwl_array_next_slot(subject, from);
                let Ok(slot) = usize::try_from(slot) else {
                    break;
                };
                let key = MwlStr::from_raw(mwl_runtime::mwl_array_key_at(subject, slot));
                let mut value = Value::null();
                mwl_runtime::mwl_array_value_at(subject, slot, &raw mut value);
                (slot, key, value)
            };
            from = slot + 1;

            // One reference for the duration of the call, released right
            // after — `call_closure` takes its own. `key` itself is still owed
            // to `out.set` below.
            let key_arg = Value::str(key.clone());
            let mapped = mwl_runtime::call_closure(ctx, args[1], &[value, key_arg]);
            #[expect(
                unsafe_code,
                reason = "this frame owns exactly the reference `key.clone()` \
                          just produced"
            )]
            unsafe {
                key_arg.release();
            }
            // Unwrapped into a local of its own *before* `key` is moved: a
            // throw partway through then frees the partial result and this
            // entry's key by dropping two named locals, which `MwlArray` and
            // `MwlStr` both do by releasing.
            let mapped = mapped?;
            out.set(key, mapped);
        }
        Ok(Value::array(out))
    }
}

/// The `{preserveKeys?: bool}` option's value — [`PRESERVE_KEYS`] read back on
/// the runtime side.
///
/// The bag is flattened into one ordinary argument by
/// `mwl_ir::lower::lower_call_args`, so the slot is always present and a wrong
/// tag there means the checker let a call through it should have refused;
/// that is a contained `FATAL`, exactly as for a mistyped positional.
fn preserve_keys(value: &Value, member: &str) -> Result<bool, Fault> {
    value.as_bool().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Arr::{member} expected {:?} for `preserveKeys`, got tag {}",
            Tag::Bool,
            value.tag_byte()
        ))
    })
}

/// One `int` argument's value, as a contained `FATAL` if the tag is wrong —
/// the same "the checker let a call through it should have refused" failure
/// [`crate::str::text`] reports for a `string` position.
fn integer(value: &Value, member: &str, position: &str) -> Result<i64, Fault> {
    value.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Arr::{member} expected {:?} for {position}, got tag {}",
            Tag::Int,
            value.tag_byte()
        ))
    })
}

/// One `int|string` key argument, normalized to the bytes an array actually
/// stores it under.
///
/// ADR 0007 § 5 makes every stored key a `string`, and `mwl-ir` already
/// normalizes an `int` subscript to its decimal spelling on the way in
/// (`Lowering::lower_array_key`). A `Core` member reached through the helper
/// convention gets the argument *un*-normalized, because the tag is written
/// from the argument's own representation — so this is where the same
/// normalization happens for that path. Both spellings therefore find the same
/// entry, which is what makes `hasKey($a, 5)` and `$a["5"]` agree.
fn key_bytes(value: &Value, member: &str) -> Result<Vec<u8>, Fault> {
    if let Some(bytes) = value.as_str_bytes() {
        return Ok(bytes.to_vec());
    }
    if let Some(int) = value.as_int() {
        return Ok(int.to_string().into_bytes());
    }
    if let Some(uint) = value.as_uint() {
        return Ok(uint.to_string().into_bytes());
    }
    Err(Fault::fatal(format!(
        "Core\\Arr::{member} expected an `int|string` key, got tag {}",
        value.tag_byte()
    )))
}

/// A borrowed `MwlArray` handle over an argument's pointer, for the members
/// that want the safe handle API rather than the raw primitives.
///
/// `pub(crate)` because borrowing an array argument is not a `Core\Arr`
/// question: [`crate::regex`]'s `Match` reads the group array it was built
/// with through exactly this handle, under exactly this reasoning.
///
/// Deliberately never dropped: a helper's arguments are *borrowed* (see
/// [`crate`]'s own docs), so the reference this handle wraps belongs to the
/// caller and releasing it here would be a double free. Wrapping in
/// `ManuallyDrop` rather than retaining first keeps the borrow free — there is
/// no refcount traffic at all.
pub(crate) fn borrowed(array: *mut mwl_runtime::ArrayHeader) -> std::mem::ManuallyDrop<MwlArray> {
    #[expect(
        unsafe_code,
        reason = "a Tag::Array argument owns a reference to a live allocation, \
                  so it is live for the length of this call, and the handle is \
                  never dropped"
    )]
    std::mem::ManuallyDrop::new(unsafe { MwlArray::from_raw(array) })
}

/// Appends `times` copies of a borrowed `value` to a result being built.
///
/// The value belongs to the calling frame's argument, which outlives the call,
/// so every copy stored takes a reference of its own — the same rule
/// [`copy_entry`] applies to an entry, over a value that is repeated rather
/// than walked. `times` is a `u64` because it comes from a `uint` argument and
/// the loop is what turns it into allocations; a count that will not fit in
/// memory fails in [`MwlArray`], not by being truncated here.
fn append_copies(out: &mut MwlArray, value: Value, times: u64) {
    for _ in 0..times {
        append_borrowed(out, value);
    }
}

/// Appends one borrowed value to a result being built, under the next
/// integer key.
///
/// The value belongs to an argument of the calling frame, which outlives the
/// call, so the copy stored takes a reference of its own — [`copy_entry`]'s
/// rule without the key half. Shared by [`append_copies`], which is this
/// repeated, and by the two members whose variadic tail is a run of loose
/// values rather than of arrays.
fn append_borrowed(out: &mut MwlArray, value: Value) {
    #[expect(
        unsafe_code,
        reason = "the value is owned by the caller's argument, which \
                  outlives this call, so each copy stored here needs a \
                  reference of its own"
    )]
    unsafe {
        value.retain();
    }
    out.append(value);
}

/// Appends every value of a borrowed subject to a result being built, under
/// fresh `0, 1, …` keys — [`mwl_core_arr_values`]'s walk, shared by the two
/// padding members because each one wraps it in padding on a different side,
/// and by [`mwl_core_arr_append_all`], which is that walk over every argument
/// in turn and nothing else.
fn append_values(subject: &MwlArray, out: &mut MwlArray) {
    let mut from = 0usize;
    while let Some(slot) = subject.next_slot(from) {
        let value = subject
            .value_at(slot)
            .expect("next_slot only names live entries");
        #[expect(
            unsafe_code,
            reason = "the entry is owned by the subject array, which outlives \
                      this call, so the copy stored here needs a reference of \
                      its own"
        )]
        unsafe {
            value.retain();
        }
        out.append(value);
        from = slot + 1;
    }
}

/// Copies the entry at `slot` — key and value both — from a borrowed subject
/// into a result being built, under its own key.
///
/// The value belongs to the subject array, which outlives the call, so the
/// copy stored in the result takes a reference of its own; the key does not,
/// because [`MwlArray::key_at`] already hands back a fresh one. That asymmetry
/// is the single thing a key-and-value copy has to get right, so it lives here
/// once rather than in each member that walks entries through unchanged.
fn copy_entry(subject: &MwlArray, slot: usize, out: &mut MwlArray) {
    let key = subject
        .key_at(slot)
        .expect("next_slot only names live entries");
    let value = subject
        .value_at(slot)
        .expect("next_slot only names live entries");
    #[expect(
        unsafe_code,
        reason = "the entry is owned by the subject array, which outlives this \
                  call, so the copy stored here needs a reference of its own"
    )]
    unsafe {
        value.retain();
    }
    out.set(key, value);
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::hasKey(array<T> $a, int|string $key): bool` — replacing
    /// PHP's `array_key_exists` **and** `isset($a[$k])`, which differ in PHP
    /// only over a stored `null` and therefore cannot both survive ADR 0063
    /// R20.
    ///
    /// The first member with a **union** parameter. It needs no IR
    /// representation for one: the helper's argument slot is a tagged
    /// `mwl_runtime::Value` written from the argument's own type, so
    /// [`key_bytes`] decodes by tag. `mwl_ir::lower::ArgSig::helper` owns why
    /// that is a property of the helper convention rather than of this member.
    fn mwl_core_arr_has_key(_ctx, args: [2]) {
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::hasKey expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let key = key_bytes(&args[1], "hasKey")?;
        Ok(Value::bool(borrowed(array).has_key(&key)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::isList(array<T> $a): bool` — whether the keys are exactly
    /// `0, 1, …, n-1` in that order, replacing PHP's `array_is_list`. An empty
    /// array is a list, as it is in PHP.
    ///
    /// ADR 0007 § 5 stores every key as a `string`, so "is this an integer key"
    /// is a question about the *bytes*: the key must be the index's decimal
    /// spelling exactly, which rules out `"01"` and `"+1"` the way PHP's
    /// canonical-integer-key normalization already would. The expected spelling
    /// is written into one reused buffer rather than a `String` per entry — the
    /// member is O(n) and this keeps it one allocation rather than n.
    fn mwl_core_arr_is_list(_ctx, args: [1]) {
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::isList expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        Ok(Value::bool(is_list(&borrowed(array))))
    }
}

/// ADR 0007 § 5's `"0" … "n−1"` test, over a borrowed array.
///
/// Lifted out of [`mwl_core_arr_is_list`] rather than left inline because
/// [ADR 0069](../../../../docs/adr/0069-array-combination-is-key-type-independent.md)
/// § 1 states `overlayDeep`'s recursion rule in terms of it — two sides of a
/// key merge only where both hold an array and **neither is a list** — so the
/// member and the rule now read the same predicate rather than two spellings
/// of it.
fn is_list(subject: &MwlArray) -> bool {
    use std::fmt::Write as _;

    let mut expected = String::new();
    let mut from = 0usize;
    let mut index = 0usize;
    while let Some(slot) = subject.next_slot(from) {
        let key = subject
            .key_at(slot)
            .expect("next_slot only names live entries");
        expected.clear();
        write!(expected, "{index}").expect("writing a usize into a String never fails");
        if key.as_bytes() != expected.as_bytes() {
            return false;
        }
        from = slot + 1;
        index += 1;
    }
    true
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::keys(array<T> $a): array<string>` — every key in insertion
    /// order under fresh `0, 1, …` keys of its own, replacing PHP's
    /// `array_keys`.
    ///
    /// **`array<string>`, never `array<int|string>`.** ADR 0007 § 5 stores
    /// every key as a `string`, so a key that *looks* like an integer is one
    /// only in its spelling — `Core\Arr::keys(["10" => "x"])` yields `["10"]`,
    /// and PHP's `array_keys` yielding an `int` there is the key-type-dependent
    /// behaviour ADR 0069 removes. A caller who wants the number writes
    /// `$k as int`, which is the same thing a `foreach` key binding already
    /// does.
    ///
    /// PHP's `array_keys($a, $search)` search form is not reproduced: that is
    /// `Core\Arr::keyOf` for one and a `filter` for many, and folding two
    /// unrelated questions into one name is what ADR 0063 R20 refuses.
    fn mwl_core_arr_keys(_ctx, args: [1]) {
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::keys expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let subject = borrowed(array);

        let mut out = MwlArray::new();
        let mut from = 0usize;
        while let Some(slot) = subject.next_slot(from) {
            let key = subject
                .key_at(slot)
                .expect("next_slot only names live entries");
            // `key_at` hands back a fresh reference, which `append` takes over.
            out.append(Value::str(key));
            from = slot + 1;
        }
        Ok(Value::array(out))
    }
}

/// The entry positions an `int $offset` and a `?int $length` name over an
/// array of `count` entries, under ADR 0063 R8's sign rule.
///
/// Deliberately `crate::str::window`'s rule, one unit up: an array counts in
/// *entries* where a string counts in characters, and nothing else differs.
///
/// * A **negative offset** counts back from the last entry, and one reaching
///   past the first clamps to it.
/// * A **negative length** stops that many entries short of the end.
/// * A **null length** runs to the end. That is the type saying what a
///   sentinel would otherwise have to (ADR 0063 R5), and [`Const::Null`] is
///   what a call site materializes for a `slice` that omits it.
/// * The end never precedes the start, so a window that closes before it
///   opens is empty rather than reversed.
///
/// The positions are *ordinal*, not keys: entry 0 is whichever entry the walk
/// meets first. An ordered hash has no other reading, and it is what makes
/// `slice` answer the same entries whatever the keys happen to be — the
/// key-type independence
/// [ADR 0069](../../../../docs/adr/0069-array-combination-is-key-type-independent.md)
/// asks of every § 2 member, here reaching the *positions* rather than the
/// result's keys.
fn window(
    count: usize,
    offset: &Value,
    length: &Value,
    member: &str,
) -> Result<(usize, usize), Fault> {
    let offset = integer(offset, member, "the offset")?;
    let total = i64::try_from(count).unwrap_or(i64::MAX);

    let start = if offset < 0 {
        usize::try_from(total.saturating_add(offset)).unwrap_or(0)
    } else {
        usize::try_from(offset).unwrap_or(usize::MAX)
    }
    .min(count);

    let end = match length.tag() {
        Some(Tag::Null) => count,
        _ => {
            let length = integer(length, member, "the length")?;
            if length < 0 {
                usize::try_from(total.saturating_add(length)).unwrap_or(0)
            } else {
                usize::try_from(length)
                    .unwrap_or(usize::MAX)
                    .saturating_add(start)
            }
            .min(count)
        }
    };

    Ok((start, end.max(start)))
}

/// Writes the entry at `slot` into `out`, keeping its key or renumbering it —
/// the one line `slice` and `chunk` differ by, and the only thing their
/// `{preserveKeys?: bool}` option changes.
fn carry_entry(subject: &MwlArray, slot: usize, out: &mut MwlArray, preserve: bool) {
    if preserve {
        copy_entry(subject, slot, out);
    } else {
        let value = subject
            .value_at(slot)
            .expect("next_slot only names live entries");
        append_borrowed(out, value);
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::slice(array<T> $a, int $offset, ?int $length = null, {preserveKeys?: bool}): array<T>`
    /// — the entries in one window, replacing PHP's `array_slice`.
    ///
    /// [`window`] owns what each sign means, and [`PRESERVE_KEYS`] owns what
    /// the option's `false` default does: every key discarded and the result
    /// renumbered from `"0"`, rather than `array_slice`'s
    /// renumber-the-integers-keep-the-strings. That is the divergence to know
    /// about, and it is ADR 0069 § 3's rule rather than this member's opinion.
    ///
    /// An `args: [4]` helper for a three-parameter signature: the option bag
    /// flattens into one ordinary argument, as [`mwl_core_arr_range`]'s docs
    /// spell out.
    ///
    /// The walk stops at the window's end rather than running to the last
    /// entry — a slice of the first two of ten touches two entries, which is
    /// what makes this the member a paging loop calls.
    fn mwl_core_arr_slice(_ctx, args: [4]) {
        let base = subject(args, "slice")?;
        let preserve = preserve_keys(&args[3], "slice")?;
        let (start, end) = window(base.count(), &args[1], &args[2], "slice")?;

        let mut out = MwlArray::new();
        let mut from = 0usize;
        let mut position = 0usize;
        while position < end {
            let Some(slot) = base.next_slot(from) else {
                break;
            };
            from = slot + 1;
            if position >= start {
                carry_entry(&base, slot, &mut out, preserve);
            }
            position += 1;
        }
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::chunk(array<T> $a, uint $size, {preserveKeys?: bool}): array<array<T>>`
    /// — the entries in runs of `$size`, replacing PHP's `array_chunk`.
    ///
    /// The last run is short when the count does not divide, and an empty
    /// subject yields no runs at all rather than one empty one. Both are PHP's
    /// answers.
    ///
    /// **A `$size` of zero throws**, for [`mwl_core_arr_range`]'s reason and
    /// with the class that member's docs say the tree still owes: there is no
    /// run length that would terminate, so silently answering an empty array
    /// would turn a caller's arithmetic bug into a loop that does nothing.
    /// `$size` is a `uint`, so a negative one is a diagnostic rather than a
    /// throw — PHP's `ValueError` covers both.
    ///
    /// [`PRESERVE_KEYS`] governs the *inner* arrays' keys; the outer one is
    /// always a list, since a run has no key of its own to keep. That is the
    /// same question [`mwl_core_arr_slice`] answers and it is answered the
    /// same way, which is why the two share [`carry_entry`].
    fn mwl_core_arr_chunk(_ctx, args: [3]) {
        let base = subject(args, "chunk")?;
        let size = args[1].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::chunk expected {:?} for `size`, got tag {}",
                Tag::Uint,
                args[1].tag_byte()
            ))
        })?;
        let preserve = preserve_keys(&args[2], "chunk")?;
        if size == 0 {
            return Err(Fault::thrown(
                "Core\\Arr::chunk(): the `size` argument must be greater than 0, got 0",
            ));
        }

        let mut out = MwlArray::new();
        let mut run = MwlArray::new();
        let mut held = 0u64;
        let mut from = 0usize;
        while let Some(slot) = base.next_slot(from) {
            from = slot + 1;
            carry_entry(&base, slot, &mut run, preserve);
            held += 1;
            if held == size {
                out.append(Value::array(std::mem::replace(&mut run, MwlArray::new())));
                held = 0;
            }
        }
        if held > 0 {
            out.append(Value::array(run));
        }
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::append(array<T> $a, T ...$values): array<T>` — the subject
    /// with every trailing value after its last entry, replacing PHP's
    /// `array_push` and giving `$a[] = $v` a form that is an expression.
    ///
    /// **Every key of the subject is kept**, and each added value lands under
    /// the next free integer key — which is exactly what `$a[] = $v` does, and
    /// is the whole reason this member can claim to replace it. That is not a
    /// key-type-dependent rule of the kind
    /// [ADR 0069](../../../../docs/adr/0069-array-combination-is-key-type-independent.md)
    /// § 3 removes: the key chosen is one counter's next value whatever the
    /// existing keys look like, so it never has to ask what type they were.
    /// It is also why this member takes no `preserveKeys` option — appending
    /// invents no key that could collide with one already there, so there is
    /// nothing to choose. [`mwl_core_arr_prepend`] is the side where there is.
    ///
    /// A call with no trailing values at all is the subject, entry for entry.
    fn mwl_core_arr_append(_ctx, args: [2]) {
        let base = subject(args, "append")?;
        let mut out = MwlArray::new();
        copy_all(&base, &mut out);
        for_each_trailing(&args[1], "append", |value| {
            append_borrowed(&mut out, value);
            Ok(())
        })?;
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::prepend(array<T> $a, T ...$values): array<T>` — the trailing
    /// values in written order, then the subject's, replacing PHP's
    /// `array_unshift`.
    ///
    /// **The result is always a list**, whatever the subject's keys were, for
    /// [`mwl_core_arr_pad_start`]'s reason and no other: an entry put in
    /// *front* of an existing one has no non-arbitrary key. `"0"` is the only
    /// candidate and the subject may already hold it, at which point the
    /// alternatives are to overwrite an entry the call never mentioned or to
    /// pick `"1"` and store it ahead of `"0"` — an order that contradicts the
    /// keys. PHP escapes that by renumbering the integer keys and keeping the
    /// string ones, which is the key-type-dependent behaviour ADR 0069 § 3
    /// removes; renumbering *every* key is the same answer applied uniformly,
    /// and it is what spec § 2 means by `{preserveKeys: false}` wherever the
    /// option appears. The option is not declared here because, as in the
    /// padding members, keeping a key is not a choice that can be offered.
    ///
    /// So the two members are **not** mirror images, and the asymmetry is
    /// inherited rather than invented: appending has a free key to use and
    /// [`mwl_core_arr_append`] therefore keeps the subject's, while prepending
    /// does not. A call with no trailing values is `Core\Arr::values` of the
    /// subject.
    fn mwl_core_arr_prepend(_ctx, args: [2]) {
        let base = subject(args, "prepend")?;
        let mut out = MwlArray::new();
        for_each_trailing(&args[1], "prepend", |value| {
            append_borrowed(&mut out, value);
            Ok(())
        })?;
        append_values(&base, &mut out);
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::withoutFirst(array<T> $a): array<T>` — every entry but the
    /// first, replacing what is left of the subject after PHP's `array_shift`.
    ///
    /// PHP splits that question across a statement that answers two at once:
    /// `array_shift($a)` hands back the element *and* mutates `$a`. ADR 0063 R3
    /// makes nothing mutate, so the spec's § 2 table splits it in two —
    /// `Core\Arr::first` gets the element, this gets the remainder — and an
    /// empty array yields an empty array rather than a `null` and a warning.
    ///
    /// **Every surviving key is kept.** The member declares no `preserveKeys`
    /// option, and the spec's § 2 rule that the option's `false` default
    /// discards every key applies where it *appears*; here there is nothing to
    /// choose, so this behaves like [`mwl_core_arr_filter`], which also drops
    /// entries without renumbering the ones that remain. That is deliberately
    /// not `array_shift`'s behaviour, which renumbers integer keys and keeps
    /// string ones — the key-type-dependent rule
    /// [ADR 0069](../../../../docs/adr/0069-array-combination-is-key-type-independent.md)
    /// § 3 removes. A caller who wants `0, 1, …` writes `Core\Arr::values` and
    /// says so; a caller who wanted to keep a map's keys has no way to get them
    /// back once a member has thrown them away, so keeping is the direction
    /// that loses nothing.
    fn mwl_core_arr_without_first(_ctx, args: [1]) {
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::withoutFirst expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let subject = borrowed(array);

        // The one entry this member drops is whatever the cursor names first,
        // so skipping it is a cursor start rather than a comparison per entry.
        let mut from = match subject.next_slot(0) {
            Some(first) => first + 1,
            None => return Ok(Value::array(MwlArray::new())),
        };
        let mut out = MwlArray::new();
        while let Some(slot) = subject.next_slot(from) {
            copy_entry(&subject, slot, &mut out);
            from = slot + 1;
        }
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::withoutLast(array<T> $a): array<T>` — every entry but the
    /// last, replacing what is left of the subject after PHP's `array_pop`.
    ///
    /// [`mwl_core_arr_without_first`]'s own docs own the split from PHP's
    /// mutate-and-return pair and the rule that every surviving key is kept.
    ///
    /// The last entry cannot be named without walking, since the ordered hash
    /// has no backward cursor — the same property
    /// [`mwl_core_arr_reverse`] collects slots for. This one needs no `Vec`:
    /// it copies each entry only once it has seen that another follows.
    fn mwl_core_arr_without_last(_ctx, args: [1]) {
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::withoutLast expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let subject = borrowed(array);

        let mut out = MwlArray::new();
        let mut pending = subject.next_slot(0);
        while let Some(slot) = pending {
            pending = subject.next_slot(slot + 1);
            if pending.is_some() {
                copy_entry(&subject, slot, &mut out);
            }
        }
        Ok(Value::array(out))
    }
}

/// The subject, the target size and the padding value of a `padStart`/`padEnd`
/// call, decoded once for both.
///
/// The two differ only in which side the padding lands on, so everything up to
/// that point — including the "already long enough" answer, which is `0`
/// further entries rather than a second return path — is shared.
fn padding(
    args: &[Value],
    member: &str,
) -> Result<(std::mem::ManuallyDrop<MwlArray>, u64, Value), Fault> {
    let array = args[0].array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Arr::{member} expected {:?}, got tag {}",
            Tag::Array,
            args[0].tag_byte()
        ))
    })?;
    let size = args[1].as_uint().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Arr::{member} expected {:?} for `size`, got tag {}",
            Tag::Uint,
            args[1].tag_byte()
        ))
    })?;
    let subject = borrowed(array);
    let missing = size.saturating_sub(subject.count() as u64);
    Ok((subject, missing, args[2]))
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::padStart(array<T> $a, uint $size, T $value): array<T>` —
    /// enough copies of `$value` in front to reach `$size` entries, replacing
    /// PHP's `array_pad` with a *negative* length.
    ///
    /// **The result is always a list**, whatever the subject's keys were, and
    /// that is the one place these two members diverge from
    /// [`mwl_core_arr_without_first`]'s rule that every surviving key is kept.
    /// The difference is that padding *adds* entries, and there is no
    /// non-arbitrary key for an added one beside an existing map's keys —
    /// prepending to `["a" => 1]` would have to invent `"0"`, which the
    /// subject may already hold. PHP resolves that by renumbering the integer
    /// keys and keeping the string ones, which is exactly the
    /// key-type-dependent rule
    /// [ADR 0069](../../../../docs/adr/0069-array-combination-is-key-type-independent.md)
    /// § 3 removes; renumbering *every* key is the same answer applied
    /// uniformly, and it is what spec § 2 says `{preserveKeys: false}` means
    /// wherever the option appears. Neither member declares the option,
    /// because keeping a key here is not a choice that can be offered.
    ///
    /// A subject already at or past `$size` is returned as a list of its own
    /// values, unpadded — PHP returns it unchanged, and the difference is only
    /// the keys, which this member has already said it does not keep. `$size`
    /// is a `uint`, so PHP's "negative length pads on the left" overload is a
    /// member name here rather than a sign.
    fn mwl_core_arr_pad_start(_ctx, args: [3]) {
        let (subject, missing, value) = padding(args, "padStart")?;
        let mut out = MwlArray::new();
        append_copies(&mut out, value, missing);
        append_values(&subject, &mut out);
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::padEnd(array<T> $a, uint $size, T $value): array<T>` —
    /// enough copies of `$value` after the last entry to reach `$size`,
    /// replacing PHP's `array_pad` with a positive length.
    ///
    /// [`mwl_core_arr_pad_start`]'s own docs own why the result is always a
    /// list and why neither member takes a `preserveKeys` option. The two are
    /// the same walk with the padding on the other side.
    fn mwl_core_arr_pad_end(_ctx, args: [3]) {
        let (subject, missing, value) = padding(args, "padEnd")?;
        let mut out = MwlArray::new();
        append_values(&subject, &mut out);
        append_copies(&mut out, value, missing);
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::reverse(array<T> $a, {preserveKeys?: bool}): array<T>` —
    /// the entries in the opposite order, replacing PHP's `array_reverse`.
    ///
    /// The option is [`PRESERVE_KEYS`], whose docs own what `false` means:
    /// every key discarded and the result renumbered, rather than PHP's
    /// renumber-the-integers-keep-the-strings. With `true` each entry keeps
    /// its own key and only the order changes.
    ///
    /// The walk is forward and the writes are prepends-by-collection: the
    /// ordered hash has no backward cursor, so this collects the slots first
    /// and then writes them out in reverse. One `Vec<usize>` of scratch, which
    /// ADR 0004's ordering buys without discussion.
    fn mwl_core_arr_reverse(_ctx, args: [2]) {
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::reverse expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let preserve_keys = preserve_keys(&args[1], "reverse")?;
        let subject = borrowed(array);

        let mut slots: Vec<usize> = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = subject.next_slot(from) {
            slots.push(slot);
            from = slot + 1;
        }

        let mut out = MwlArray::new();
        for slot in slots.into_iter().rev() {
            let value = subject
                .value_at(slot)
                .expect("next_slot only names live entries");
            #[expect(
                unsafe_code,
                reason = "the entry is owned by the subject array, which \
                          outlives this call, so the copy stored here needs a \
                          reference of its own"
            )]
            unsafe {
                value.retain();
            }
            if preserve_keys {
                let key = subject
                    .key_at(slot)
                    .expect("next_slot only names live entries");
                out.set(key, value);
            } else {
                out.append(value);
            }
        }
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::flip(array<int|string> $a): array<string>` — each value
    /// becomes a key and each key becomes a value, replacing PHP's
    /// `array_flip`.
    ///
    /// Duplicate values collapse, **the last occurrence winning**, which is
    /// PHP's rule and the spec's § 2 *Structure* note. The result is keyed in
    /// first-occurrence order all the same, because a re-`set` of an existing
    /// key overwrites in place rather than moving the entry to the end —
    /// ADR 0007 § 5's insertion order is a property of the *key*, not of the
    /// most recent write.
    ///
    /// The values become keys through [`key_bytes`], the same normalization
    /// `hasKey` uses, so an `int` value and its decimal spelling flip to the
    /// same key. A value of any other type is the checker having let an
    /// `array<int|string>` position take something else, and is reported as
    /// such rather than skipped the way PHP's warning-and-continue does.
    fn mwl_core_arr_flip(_ctx, args: [1]) {
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::flip expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let subject = borrowed(array);

        let mut out = MwlArray::new();
        let mut from = 0usize;
        while let Some(slot) = subject.next_slot(from) {
            let key = subject
                .key_at(slot)
                .expect("next_slot only names live entries");
            let value = subject
                .value_at(slot)
                .expect("next_slot only names live entries");
            // The old key becomes the new *value*, and `key_at`'s fresh
            // reference is exactly the one `set` takes over.
            out.set(MwlStr::new(&key_bytes(&value, "flip")?), Value::str(key));
            from = slot + 1;
        }
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::values(array<T> $a): array<T>` — the values in insertion
    /// order under fresh `0, 1, …` keys, replacing PHP's `array_values`.
    ///
    /// Each value belongs to the subject array, which outlives this call, so
    /// the copy stored here takes a reference of its own — the opposite of
    /// [`mwl_core_arr_map`], whose values are the callback's and are already
    /// owned. `MwlArray::append` is what assigns the new keys, so this member
    /// states no key rule of its own.
    fn mwl_core_arr_values(_ctx, args: [1]) {
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::values expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let subject = borrowed(array);

        let mut out = MwlArray::new();
        let mut from = 0usize;
        while let Some(slot) = subject.next_slot(from) {
            let value = subject
                .value_at(slot)
                .expect("next_slot only names live entries");
            #[expect(
                unsafe_code,
                reason = "the entry is owned by the subject array, which \
                          outlives this call, so the copy stored here needs a \
                          reference of its own"
            )]
            unsafe {
                value.retain();
            }
            out.append(value);
            from = slot + 1;
        }
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::fill(uint $count, T $value): array<T>` — `$count` copies of
    /// one value under `0, 1, …`, replacing PHP's `array_fill`.
    ///
    /// **PHP's `$start_index` is dropped.** `array_fill(5, 3, 'v')` produces
    /// keys `5, 6, 7`, which is a list that does not start at zero — a shape
    /// [`mwl_core_arr_is_list`] answers `false` for, and one every caller then
    /// has to reason about. The two real uses of the parameter are covered
    /// without it: `0` is this member, and any other keys are
    /// [`mwl_core_arr_fill_keys`] over the keys the caller actually wants.
    ///
    /// `$count` of zero yields an empty array rather than throwing, which is
    /// PHP's own answer and the one that lets a computed count through
    /// unguarded.
    fn mwl_core_arr_fill(_ctx, args: [2]) {
        let count = args[0].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::fill expected {:?} for `count`, got tag {}",
                Tag::Uint,
                args[0].tag_byte()
            ))
        })?;
        let mut out = MwlArray::new();
        append_copies(&mut out, args[1], count);
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::fillKeys(array<int|string> $keys, T $value): array<T>` — one
    /// value stored under every key named, replacing PHP's `array_fill_keys`.
    ///
    /// The `$keys` array contributes its *values* and nothing else, exactly as
    /// [`mwl_core_arr_from_keys_and_values`] takes its keys, and each goes
    /// through [`key_bytes`] — so `1` and `"1"` name one entry, and a duplicate
    /// key collapses in its first occurrence's position. Every entry holds the
    /// same value, so which of two duplicates "wins" is not observable; what is
    /// observable is the count, and it counts distinct keys the way PHP's does.
    fn mwl_core_arr_fill_keys(_ctx, args: [2]) {
        let keys = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::fillKeys expected {:?} for the keys, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let keys = borrowed(keys);

        let mut out = MwlArray::new();
        let mut from = 0usize;
        while let Some(slot) = keys.next_slot(from) {
            let key = keys
                .value_at(slot)
                .expect("next_slot only names live entries");
            from = slot + 1;
            // Normalized *before* the retain: a key of the wrong type leaves
            // through `?`, and a reference taken first would have no owner.
            let bytes = key_bytes(&key, "fillKeys")?;
            #[expect(
                unsafe_code,
                reason = "the value is owned by the caller's argument, which \
                          outlives this call, so each copy stored here needs a \
                          reference of its own"
            )]
            unsafe {
                args[1].retain();
            }
            out.set(MwlStr::new(&bytes), args[1]);
        }
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::range(int $start, int $end, {step?: int}): array<int>` — the
    /// integers from `$start` to `$end` inclusive, replacing PHP's `range`.
    ///
    /// The first member with an ADR 0063 R2 options bag, and therefore the
    /// first whose arity says something the spec's signature does not:
    /// `{step?: int}` is flattened into one ordinary argument by
    /// `mwl_ir::lower::lower_call_args`, so this is an `args: [3]` helper and
    /// `args[2]` is always present — the call site materialized the default
    /// where it was not written. `mwl_stdlib::registry`'s own docs own why.
    ///
    /// Three behaviours, all PHP 8.5's and all verified against it:
    ///
    /// * **The direction comes from the arguments, not the step.** `$start >
    ///   $end` counts down; there is no negative step.
    /// * **A `step` of zero or less throws**, rather than looping forever or
    ///   silently reversing. PHP raises `ValueError`; this raises the tree's
    ///   root, because a `Core` helper cannot yet name the class it throws
    ///   (`mwl_runtime::Ctx::set_runtime_error_class`) — spec § 10's
    ///   `LogicError` is the class this owes once it can.
    /// * **A step that overshoots stops at the last in-range value**, so
    ///   `range(1, 10, {step: 3})` is `1, 4, 7, 10` and
    ///   `range(1, 10, {step: 4})` is `1, 5, 9`.
    ///
    /// The cursor advances by `checked_add`/`checked_sub` rather than by
    /// multiplying an index: a range whose next step would leave `int` stops
    /// instead of wrapping, which is ADR 0007 § 4's rule applied to a loop
    /// this member owns rather than to arithmetic a program wrote.
    fn mwl_core_arr_range(_ctx, args: [3]) {
        let start = integer(&args[0], "range", "the start")?;
        let end = integer(&args[1], "range", "the end")?;
        let step = integer(&args[2], "range", "the `step` option")?;
        if step <= 0 {
            return Err(Fault::thrown(format!(
                "Core\\Arr::range(): the `step` option must be greater than 0, got {step}"
            )));
        }

        let mut out = MwlArray::new();
        let mut cursor = start;
        loop {
            out.append(Value::int(cursor));
            let next = if start <= end {
                match cursor.checked_add(step) {
                    Some(next) if next <= end => next,
                    _ => break,
                }
            } else {
                match cursor.checked_sub(step) {
                    Some(next) if next >= end => next,
                    _ => break,
                }
            };
            cursor = next;
        }
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::fromKeysAndValues(array<int|string> $keys, array<T> $values): array<T>`
    /// — one array's values used as the keys of another's, replacing PHP's
    /// `array_combine`.
    ///
    /// **Two arrays of different lengths throw**, which is the spec's § 2
    /// *Structure* note and ADR 0063 R4: PHP 8 raises a `ValueError` here, and
    /// the shorter-wins alternative would silently drop data. The two are
    /// walked by their own cursors, so it is each array's *entry count* that
    /// has to match — neither side's keys are looked at, and the `$keys`
    /// array's own keys are discarded exactly as [`mwl_core_arr_values`]
    /// discards them.
    ///
    /// Each key goes through [`key_bytes`], the normalization `hasKey` and
    /// `flip` share, so an `int` key and its decimal spelling name the same
    /// entry. Duplicate keys collapse with the **last** value winning, in the
    /// **first** occurrence's position — `flip`'s rule, and for the same reason:
    /// re-`set`ting an existing key overwrites in place.
    fn mwl_core_arr_from_keys_and_values(_ctx, args: [2]) {
        let keys = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::fromKeysAndValues expected {:?} for the keys, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let values = args[1].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::fromKeysAndValues expected {:?} for the values, got tag {}",
                Tag::Array,
                args[1].tag_byte()
            ))
        })?;
        let keys = borrowed(keys);
        let values = borrowed(values);
        if keys.count() != values.count() {
            return Err(Fault::thrown(format!(
                "Core\\Arr::fromKeysAndValues(): the two arrays must be the same length, \
                 got {} against {}",
                keys.count(),
                values.count()
            )));
        }

        let mut out = MwlArray::new();
        let mut key_from = 0usize;
        let mut value_from = 0usize;
        while let (Some(key_slot), Some(value_slot)) =
            (keys.next_slot(key_from), values.next_slot(value_from))
        {
            let key = keys
                .value_at(key_slot)
                .expect("next_slot only names live entries");
            let value = values
                .value_at(value_slot)
                .expect("next_slot only names live entries");
            // Normalized *before* the retain: a key of the wrong type leaves
            // through `?`, and a reference taken first would have no owner.
            let bytes = key_bytes(&key, "fromKeysAndValues")?;
            #[expect(
                unsafe_code,
                reason = "the entry is owned by the values array, which outlives \
                          this call, so the copy stored here needs a reference \
                          of its own"
            )]
            unsafe {
                value.retain();
            }
            out.set(MwlStr::new(&bytes), value);
            key_from = key_slot + 1;
            value_from = value_slot + 1;
        }
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::countBy(array<T> $a, {by?: callable}): array<uint>` — how
    /// many entries fall under each distinct value, replacing PHP's
    /// `array_count_values` and the userland group-and-count loop.
    ///
    /// Without `by` the value itself is the bucket, which is
    /// `array_count_values`; with it, the callback names the bucket and
    /// receives `($value, $key)` like every other `Core\Arr` callback, which is
    /// the half PHP has no function for at all. The result is keyed in
    /// **first-occurrence** order and its values are `uint`, for the reason
    /// [`mwl_core_arr_count`] returns one: a count cannot be negative.
    ///
    /// A bucket is named through [`key_bytes`], so `1` and `"1"` count as one
    /// bucket — the same normalization every other key position uses. A value
    /// that is neither an `int` nor a `string` and has no `by` to name it is a
    /// throw rather than PHP's warn-and-skip, which is `flip`'s treatment of
    /// the identical situation.
    fn mwl_core_arr_count_by(ctx, args: [2]) {
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::countBy expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let by = optional_callback(&args[1], "countBy", "by")?;
        let subject = borrowed(array);

        let mut out = MwlArray::new();
        let mut from = 0usize;
        while let Some(slot) = subject.next_slot(from) {
            let value = subject
                .value_at(slot)
                .expect("next_slot only names live entries");
            from = slot + 1;

            let bucket = match by {
                None => key_bytes(&value, "countBy")?,
                Some(callback) => {
                    let key = subject
                        .key_at(slot)
                        .expect("next_slot only names live entries");
                    // One reference for the duration of the call, released
                    // right after: `call_closure` takes its own.
                    let key_arg = Value::str(key);
                    let named = mwl_runtime::call_closure(ctx, callback, &[value, key_arg]);
                    #[expect(
                        unsafe_code,
                        reason = "this frame owns exactly the reference `key_at` \
                                  just handed back"
                    )]
                    unsafe {
                        key_arg.release();
                    }
                    let named = named?;
                    let bucket = key_bytes(&named, "countBy");
                    #[expect(
                        unsafe_code,
                        reason = "the bucket name is a fresh value this frame \
                                  owns; a callback returning a string would \
                                  otherwise leak one reference per entry"
                    )]
                    unsafe {
                        named.release();
                    }
                    bucket?
                }
            };

            let seen = out.get(&bucket).and_then(|count| count.as_uint()).unwrap_or(0);
            out.set(MwlStr::new(&bucket), Value::uint(seen + 1));
        }
        Ok(Value::array(out))
    }
}

/// What a `{by: ...}` option extracted, owned by the frame that extracted it —
/// [`mwl_core_arr_sort`]'s sort keys and [`mwl_core_arr_unique`]'s identity
/// keys, which are the same obligation under two names.
///
/// `mwl_runtime::call_closure` hands back one fresh reference per call, so the
/// extracted values are this frame's to free — unlike the entries themselves,
/// which belong to the subject array. Held in a guard rather than released at
/// the end of the member because a `by` closure, a comparator or a wrong tag
/// can all leave part-way through, and a `Drop` is the only release every one
/// of those paths runs.
struct Extracted(Vec<Value>);

impl Drop for Extracted {
    fn drop(&mut self) {
        for value in self.0.drain(..) {
            #[expect(
                unsafe_code,
                reason = "each of these is exactly the one reference \
                          `call_closure` returned to this frame"
            )]
            unsafe {
                value.release();
            }
        }
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::sort(array<T> $a, {by?: callable, order?: Order, comparator?: callable, preserveKeys?: bool}): array<T>`
    /// — the spec's § 2 *Ordering* member that replaces `sort`, `rsort`,
    /// `asort`, `arsort`, `usort`, `uasort`, `natsort`, `natcasesort` and
    /// `array_multisort`.
    ///
    /// # The four options
    ///
    /// * **`by`** extracts the value each entry is *compared by*, receiving
    ///   `($value, $key)` like every other `Core\Arr` callback. It is called
    ///   exactly once per entry, before any comparison — decorate-sort-
    ///   undecorate — so an expensive extractor costs `n` calls rather than
    ///   the `n log n` a comparator doing the same work would.
    /// * **`order`** is `Core\Order::Asc` (the default) or `Core\Order::Desc`.
    ///   `Desc` reverses the *comparison*, not the result, so equal entries
    ///   keep their original relative order either way.
    /// * **`comparator`** replaces the natural ordering with `($a, $b): int`,
    ///   negative/zero/positive — `usort`'s callback, unchanged.
    /// * **`preserveKeys`** defaults to `false`, so the result is renumbered
    ///   `0..n-1` (PHP's `sort`/`usort`); `true` keeps each entry's key (PHP's
    ///   `asort`/`uasort`). That option is the whole of the difference the
    ///   `a`-prefixed half of PHP's roster spelled into eight extra names.
    ///
    /// **`by` and `comparator` compose** rather than conflicting: `by` decides
    /// *what* is compared and `comparator` decides *how*, so a comparator sees
    /// the extracted keys when both are given. No combination of the four is
    /// refused, which is one fewer rule than a caller would otherwise have to
    /// remember and costs nothing to allow.
    ///
    /// # The sort is stable, and hand-written
    ///
    /// Stable, like every PHP 8 sort. A bottom-up merge sort over an index
    /// permutation rather than `slice::sort_by`, for one reason: a comparison
    /// here can **fail** — the callback can throw, and two values of
    /// incomparable types are a throw of this member's own. Rust's sorts take
    /// an infallible comparator, so the alternatives were swallowing the fault
    /// until the sort finished (leaving a comparator that is no longer a total
    /// order, which those sorts are documented to be allowed to panic on) or
    /// this. It costs one `Vec<usize>` of scratch space, which
    /// [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md)'s ordering
    /// buys without discussion.
    ///
    /// # Natural ordering, and where it diverges from PHP
    ///
    /// [`compare_values`] owns the table. The one deliberate divergence:
    /// **two `string`s always compare bytewise**, never numerically. PHP
    /// compares `"10"` and `"9"` as numbers, which is the same
    /// changes-type-by-itself behaviour
    /// [ADR 0007](../../../docs/adr/0007-explicit-type-system.md) rejects
    /// everywhere else; a caller who wants a numeric order over numeric
    /// strings writes `{by: ...}` and says so.
    ///
    /// **Known gap:** an `array<T>` of objects has no natural order, and
    /// [ADR 0013](../../../docs/adr/0013-comparable-interface.md) says what it
    /// should be — `Comparable::compareTo`. Calling an *instance* method from
    /// a helper is not reachable yet, so an object without a `comparator` is a
    /// throw naming the interface rather than a wrong answer.
    fn mwl_core_arr_sort(ctx, args: [5]) {
        let subject = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::sort expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let by = optional_callback(&args[1], "sort", "by")?;
        let descending = match args[2].as_int() {
            Some(0) => false,
            Some(1) => true,
            _ => {
                return Err(Fault::fatal(format!(
                    "Core\\Arr::sort expected a `Core\\Order` case for `order`, got tag {} \
                     value {}",
                    args[2].tag_byte(),
                    args[2].bits()
                )));
            }
        };
        let comparator = optional_callback(&args[3], "sort", "comparator")?;
        let preserve_keys = args[4].as_bool().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::sort expected {:?} for `preserveKeys`, got tag {}",
                Tag::Bool,
                args[4].tag_byte()
            ))
        })?;

        // Every entry, in insertion order. Both halves are *borrowed* from the
        // subject: the keys are released by their own `MwlStr` drops, and the
        // values belong to the array, which outlives this call.
        let mut keys: Vec<MwlStr> = Vec::new();
        let mut values: Vec<Value> = Vec::new();
        let mut from = 0usize;
        loop {
            #[expect(
                unsafe_code,
                reason = "a Tag::Array argument owns a reference to a live \
                          allocation, so it is live for the length of this \
                          call, and `from` only ever advances past a slot \
                          this same cursor reported"
            )]
            let (slot, key, value) = unsafe {
                let slot = mwl_runtime::mwl_array_next_slot(subject, from);
                let Ok(slot) = usize::try_from(slot) else {
                    break;
                };
                let key = MwlStr::from_raw(mwl_runtime::mwl_array_key_at(subject, slot));
                let mut value = Value::null();
                mwl_runtime::mwl_array_value_at(subject, slot, &raw mut value);
                (slot, key, value)
            };
            from = slot + 1;
            keys.push(key);
            values.push(value);
        }

        // Decorate. Dropped by the guard on every exit path below, including
        // a throw out of the extractor itself.
        let mut sort_keys = Extracted(Vec::new());
        if let Some(by) = by {
            for (index, value) in values.iter().enumerate() {
                let key_arg = Value::str(keys[index].clone());
                let extracted = mwl_runtime::call_closure(ctx, by, &[*value, key_arg]);
                #[expect(
                    unsafe_code,
                    reason = "this frame owns exactly the reference \
                              `keys[index].clone()` just produced"
                )]
                unsafe {
                    key_arg.release();
                }
                sort_keys.0.push(extracted?);
            }
        }

        // What the comparison actually reads: the extracted keys where `by`
        // was given, the entries themselves otherwise.
        let compared: &[Value] = if by.is_some() { &sort_keys.0 } else { &values };
        let mut permutation: Vec<usize> = (0..values.len()).collect();
        let mut compare = |left: usize, right: usize| -> Result<std::cmp::Ordering, Fault> {
            let ordering = match comparator {
                Some(comparator) => {
                    let verdict = mwl_runtime::call_closure(
                        ctx,
                        comparator,
                        &[compared[left], compared[right]],
                    )?;
                    let sign = comparator_sign(verdict);
                    #[expect(
                        unsafe_code,
                        reason = "the verdict is a fresh value this frame \
                                  owns; a comparator returning a heap value \
                                  would otherwise leak one reference per \
                                  comparison"
                    )]
                    unsafe {
                        verdict.release();
                    }
                    sign?
                }
                None => compare_values(&compared[left], &compared[right], r"Core\Arr::sort")?,
            };
            Ok(if descending { ordering.reverse() } else { ordering })
        };
        merge_sort(&mut permutation, &mut compare)?;

        let mut out = MwlArray::new();
        for index in permutation {
            #[expect(
                unsafe_code,
                reason = "the entry is owned by the subject array, which \
                          outlives this call, so the copy stored here needs a \
                          reference of its own"
            )]
            unsafe {
                values[index].retain();
            }
            if preserve_keys {
                out.set(keys[index].clone(), values[index]);
            } else {
                out.append(values[index]);
            }
        }
        Ok(Value::array(out))
    }
}

/// One optional callback option: the closure it names, or `None` for the
/// `Tag::Null` an omitting call site passes.
///
/// `mwl_stdlib::registry::Const::Null` owns why "not given" is spelled that
/// way rather than as a value of the option's own type.
fn optional_callback(value: &Value, member: &str, option: &str) -> Result<Option<Value>, Fault> {
    match value.tag() {
        Some(Tag::Null) => Ok(None),
        Some(Tag::Object) => Ok(Some(*value)),
        _ => Err(Fault::fatal(format!(
            "Core\\Arr::{member} expected a closure or nothing for `{option}`, got tag {}",
            value.tag_byte()
        ))),
    }
}

/// A comparator's verdict as an [`std::cmp::Ordering`] — negative, zero or
/// positive, exactly `usort`'s contract.
///
/// A `float` verdict is accepted for the same reason `int` is: the contract is
/// about the *sign*, and a comparator written as a subtraction of two floats
/// is the shape PHP code already has. A `NaN` has no sign, so it is a throw
/// rather than a silent `Equal`.
fn comparator_sign(verdict: Value) -> Result<std::cmp::Ordering, Fault> {
    if let Some(int) = verdict.as_int() {
        return Ok(int.cmp(&0));
    }
    if let Some(uint) = verdict.as_uint() {
        return Ok(uint.cmp(&0));
    }
    if let Some(float) = verdict.as_float() {
        return float.partial_cmp(&0.0).ok_or_else(|| {
            Fault::thrown("Core\\Arr::sort's comparator returned NaN, which has no ordering")
        });
    }
    Err(Fault::fatal(format!(
        "Core\\Arr::sort's comparator returned tag {}, not a number",
        verdict.tag_byte()
    )))
}

/// A stable, bottom-up merge sort over `permutation`, with a comparison that
/// may fail.
///
/// Bottom-up rather than recursive so the scratch buffer is allocated once,
/// and over an index permutation rather than the values so nothing is moved
/// twice. [`mwl_core_arr_sort`] owns why this exists at all instead of
/// `slice::sort_by`.
fn merge_sort<F>(permutation: &mut [usize], compare: &mut F) -> Result<(), Fault>
where
    F: FnMut(usize, usize) -> Result<std::cmp::Ordering, Fault>,
{
    let len = permutation.len();
    if len < 2 {
        return Ok(());
    }
    let mut buffer = vec![0usize; len];
    let mut width = 1;
    while width < len {
        let mut start = 0;
        while start < len {
            let middle = (start + width).min(len);
            let end = (start + 2 * width).min(len);
            merge(
                &permutation[start..middle],
                &permutation[middle..end],
                &mut buffer[start..end],
                compare,
            )?;
            start = end;
        }
        permutation.copy_from_slice(&buffer);
        width *= 2;
    }
    Ok(())
}

/// Merges two already-sorted runs into `out`, taking from `left` on a tie —
/// which is the whole of what makes the sort stable.
fn merge<F>(
    left: &[usize],
    right: &[usize],
    out: &mut [usize],
    compare: &mut F,
) -> Result<(), Fault>
where
    F: FnMut(usize, usize) -> Result<std::cmp::Ordering, Fault>,
{
    let (mut i, mut j, mut k) = (0usize, 0usize, 0usize);
    while i < left.len() && j < right.len() {
        if compare(left[i], right[j])? == std::cmp::Ordering::Greater {
            out[k] = right[j];
            j += 1;
        } else {
            out[k] = left[i];
            i += 1;
        }
        k += 1;
    }
    // Exactly one of the two runs still has entries, but which one is not
    // known here — so each remainder is copied into its own length rather than
    // into "the rest of `out`".
    let remaining = left.len() - i;
    out[k..k + remaining].copy_from_slice(&left[i..]);
    out[k + remaining..].copy_from_slice(&right[j..]);
    Ok(())
}

// ============================================================================
// Absence — the members that answer `?T`
// ============================================================================

/// The subject array of a member whose first parameter is one, as the safe
/// handle the walking members want.
///
/// Every member below decodes its subject the same way, so it is one function
/// rather than the same seven lines per helper — see [`borrowed`] for why the
/// handle it returns is never dropped.
fn subject(args: &[Value], member: &str) -> Result<std::mem::ManuallyDrop<MwlArray>, Fault> {
    let array = args[0].array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Arr::{member} expected {:?}, got tag {}",
            Tag::Array,
            args[0].tag_byte()
        ))
    })?;
    Ok(borrowed(array))
}

/// The value at `slot` as a fresh reference this frame owns — what a `?T`
/// answer has to be.
///
/// [`MwlArray::value_at`] *borrows*, and a returned value belongs to the
/// caller, so every one of these members retains before it answers. The
/// [`Value::null`] answer needs no counterpart: `Tag::Null` releases to
/// nothing.
fn owned_value_at(subject: &MwlArray, slot: usize) -> Value {
    let value = subject
        .value_at(slot)
        .expect("next_slot only names live entries");
    #[expect(
        unsafe_code,
        reason = "the entry is owned by the subject array, which outlives this \
                  call, so the value handed back needs a reference of its own"
    )]
    unsafe {
        value.retain();
    }
    value
}

/// The key at `slot` as a `?string` answer — already a fresh reference, since
/// [`MwlArray::key_at`] clones.
fn owned_key_at(subject: &MwlArray, slot: usize) -> Value {
    Value::str(
        subject
            .key_at(slot)
            .expect("next_slot only names live entries"),
    )
}

/// The first slot `predicate` answers truthily for, or `None`.
///
/// Shared by all four predicate members below — `find` and `findKey` read a
/// different half of the same entry, `any` asks only whether there was one,
/// and `all` asks the same question of the *negated* predicate. Every one of
/// them stops at the first match, which is what makes them one walk.
///
/// The callback receives `($value, $key)` and may declare fewer parameters,
/// the same rule and the same `mwl_runtime::call_closure` trimming
/// [`mwl_core_arr_filter`] documents; the retain/release around the key
/// argument and around the verdict are that member's too, and for the same
/// reasons.
fn find_slot(
    ctx: &mut mwl_runtime::Ctx,
    subject: &MwlArray,
    predicate: Value,
    negate: bool,
) -> Result<Option<usize>, Fault> {
    let mut from = 0usize;
    while let Some(slot) = subject.next_slot(from) {
        from = slot + 1;
        let value = subject
            .value_at(slot)
            .expect("next_slot only names live entries");
        let key = Value::str(
            subject
                .key_at(slot)
                .expect("next_slot only names live entries"),
        );
        let verdict = mwl_runtime::call_closure(ctx, predicate, &[value, key]);
        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the reference `key_at` cloned"
        )]
        unsafe {
            key.release();
        }
        let verdict = verdict?;
        let truthy = mwl_runtime::value_truthy(verdict);
        #[expect(
            unsafe_code,
            reason = "the verdict is a fresh value this frame owns; a \
                      predicate returning a heap value would otherwise leak \
                      one reference per entry"
        )]
        unsafe {
            verdict.release();
        }
        if truthy != negate {
            return Ok(Some(slot));
        }
    }
    Ok(None)
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::first(array<T> $a): ?T` — the value of the first entry in
    /// insertion order, replacing PHP's `reset`, `current` and
    /// `$a[array_key_first($a)]`.
    ///
    /// The first `Core` member to answer `?T`. The empty array is `null`, not
    /// a throw: ADR 0063 R5 makes `?T` the absence spelling and R4's throw is
    /// for a *failure*, which asking a possibly-empty array for its first
    /// entry is not. The spec's § 2 notes the one thing this costs — over an
    /// `array<?T>` the answer cannot tell "absent" from "present and null",
    /// which `isEmpty` answers directly.
    ///
    /// PHP's three spellings all move or read an internal array pointer;
    /// there is none here, and the spec's § 2 says why a cursor inside a
    /// copy-on-write *value* is incoherent.
    fn mwl_core_arr_first(_ctx, args: [1]) {
        let subject = subject(args, "first")?;
        Ok(match subject.next_slot(0) {
            Some(slot) => owned_value_at(&subject, slot),
            None => Value::null(),
        })
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::last(array<T> $a): ?T` — the value of the last entry in
    /// insertion order, replacing PHP's `end` and `$a[array_key_last($a)]`.
    ///
    /// [`mwl_core_arr_first`]'s own docs own the `null`-for-empty rule.
    ///
    /// The last entry cannot be named without walking, since the ordered hash
    /// has no backward cursor — the same property
    /// [`mwl_core_arr_without_last`] walks for. This one keeps a slot number
    /// rather than copying, so the walk allocates nothing.
    fn mwl_core_arr_last(_ctx, args: [1]) {
        let subject = subject(args, "last")?;
        let mut seen = None;
        let mut from = 0usize;
        while let Some(slot) = subject.next_slot(from) {
            seen = Some(slot);
            from = slot + 1;
        }
        Ok(match seen {
            Some(slot) => owned_value_at(&subject, slot),
            None => Value::null(),
        })
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::firstKey(array<T> $a): ?string` — the first entry's key,
    /// replacing PHP's `array_key_first` and `key`.
    ///
    /// `?string` and never `?int`: ADR 0007 § 5 stores every key as a string,
    /// and `Core\Arr::keys` already answers `array<string>` for the same
    /// reason — a member that guessed a key's "original" type would be the
    /// key-type-dependent behaviour ADR 0069 removes.
    fn mwl_core_arr_first_key(_ctx, args: [1]) {
        let subject = subject(args, "firstKey")?;
        Ok(match subject.next_slot(0) {
            Some(slot) => owned_key_at(&subject, slot),
            None => Value::null(),
        })
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::lastKey(array<T> $a): ?string` — the last entry's key,
    /// replacing PHP's `array_key_last`.
    ///
    /// [`mwl_core_arr_first_key`] owns why the answer is a `string`, and
    /// [`mwl_core_arr_last`] why finding it is a walk.
    fn mwl_core_arr_last_key(_ctx, args: [1]) {
        let subject = subject(args, "lastKey")?;
        let mut seen = None;
        let mut from = 0usize;
        while let Some(slot) = subject.next_slot(from) {
            seen = Some(slot);
            from = slot + 1;
        }
        Ok(match seen {
            Some(slot) => owned_key_at(&subject, slot),
            None => Value::null(),
        })
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::find(array<T> $a, callable $predicate): ?T` — the first
    /// value the predicate answers truthily for, replacing PHP's `array_find`.
    ///
    /// `null` when nothing matches, on [`mwl_core_arr_first`]'s terms. The
    /// walk stops at the first match, so the predicate is called once per
    /// entry *up to* it and never after — which is the property that makes
    /// this different from `filter` plus `first`, and the reason both members
    /// exist.
    fn mwl_core_arr_find(ctx, args: [2]) {
        let subject = subject(args, "find")?;
        Ok(match find_slot(ctx, &subject, args[1], false)? {
            Some(slot) => owned_value_at(&subject, slot),
            None => Value::null(),
        })
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::findKey(array<T> $a, callable $predicate): ?string` — the
    /// key of the first entry the predicate answers truthily for, replacing
    /// PHP's `array_find_key`.
    ///
    /// [`mwl_core_arr_find`] owns the walk; [`mwl_core_arr_first_key`] owns
    /// why the answer is a `string`.
    fn mwl_core_arr_find_key(ctx, args: [2]) {
        let subject = subject(args, "findKey")?;
        Ok(match find_slot(ctx, &subject, args[1], false)? {
            Some(slot) => owned_key_at(&subject, slot),
            None => Value::null(),
        })
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::any(array<T> $a, callable $predicate): bool` — whether any
    /// entry satisfies the predicate, replacing PHP's `array_any`.
    ///
    /// Short-circuits at the first match, and is `false` over an empty array.
    fn mwl_core_arr_any(ctx, args: [2]) {
        let subject = subject(args, "any")?;
        let found = find_slot(ctx, &subject, args[1], false)?;
        Ok(Value::bool(found.is_some()))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::all(array<T> $a, callable $predicate): bool` — whether every
    /// entry satisfies the predicate, replacing PHP's `array_all`.
    ///
    /// The same walk as [`mwl_core_arr_any`] against the *negated* predicate:
    /// "every entry matches" is "no entry fails", so it short-circuits at the
    /// first failure. `true` over an empty array, which is the vacuous answer
    /// PHP's own `array_all` gives.
    fn mwl_core_arr_all(ctx, args: [2]) {
        let subject = subject(args, "all")?;
        let failed = find_slot(ctx, &subject, args[1], true)?;
        Ok(Value::bool(failed.is_none()))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::contains(array<T> $haystack, T $needle): bool` — whether any
    /// entry is the needle, replacing PHP's `in_array`.
    ///
    /// **Always strict.** `in_array`'s default is a loose comparison, so
    /// `in_array("abc", [0])` is `true` in PHP versions before 8.0 and
    /// `in_array(0, ["a"])` still surprises people; the spec's § 2 table
    /// names this one "(always strict)" and there is no third argument to
    /// forget. What "strict" means is `mwl_runtime::value_identical`, whose
    /// own module docs own every row of it, including the two that differ
    /// from a naive bit comparison (`0.0` and `-0.0` are one value, `NaN` is
    /// identical to nothing) and the one that diverges from PHP's `===`:
    /// `int`, `uint`, `float` and `decimal` are **one numeric domain**, so
    /// `contains([1.0], 1)` is `true` where `in_array(1, [1.0], true)` is
    /// `false`. That is ADR 0090 § 3's numeric row, and taking it here is what
    /// keeps this member and the `==` operator one comparison rather than two.
    fn mwl_core_arr_contains(_ctx, args: [2]) {
        let subject = subject(args, "contains")?;
        Ok(Value::bool(slot_of(&subject, args[1]).is_some()))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::keyOf(array<T> $haystack, T $needle): ?string` — the key of
    /// the first entry that is the needle, replacing PHP's `array_search`.
    ///
    /// `?string` rather than `string|false`: ADR 0063 R5 makes `?T` the one
    /// absence spelling, which is the whole of what removes `array_search`'s
    /// `=== false` trap — a `0` key and a "not found" answer are the same
    /// value under `==` in PHP, and the reason its manual warns to compare
    /// strictly. [`mwl_core_arr_first_key`] owns why a key is a `string`, and
    /// [`mwl_core_arr_contains`] owns what "is the needle" means.
    ///
    /// The *first* match, in insertion order, exactly as `array_search`
    /// answers the first.
    fn mwl_core_arr_key_of(_ctx, args: [2]) {
        let subject = subject(args, "keyOf")?;
        Ok(match slot_of(&subject, args[1]) {
            Some(slot) => owned_key_at(&subject, slot),
            None => Value::null(),
        })
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::unique(array<T> $a, {by?: callable}): array<T>` — the
    /// entries whose value has not been seen before, replacing PHP's
    /// `array_unique`.
    ///
    /// **The first occurrence of each wins, and keeps its key**, which is
    /// `array_unique`'s own behaviour; the spec's § 2 table names no
    /// `preserveKeys` option here, for the reason [`mwl_core_arr_filter`]
    /// gives — a member that renumbered would change what a following
    /// `Core\Arr::keys` answers.
    ///
    /// **Compared by strict identity**, so `1` and `"1"` are two entries.
    /// That is the spec's § 2 *Combining* note in full: PHP's default is
    /// `SORT_STRING`, which casts every element to a string and therefore
    /// collapses `0`, `"0"`, `false` and `null` into one. Reproducing that
    /// would make this the one member whose answer depends on a value's
    /// *spelling* rather than its identity.
    ///
    /// `{by: fn}` names what to compare *by* — the callback receives
    /// `($value, $key)` like every other `Core\Arr` callback, is called
    /// exactly once per entry, and its result rather than the entry is what
    /// identity is asked about. The entry kept is still the entry, not the
    /// extracted key, which is what makes `unique($users, {by: fn($u) =>
    /// $u->email})` mean what it reads as.
    ///
    /// The seen set is a hash set over [`Identity`], not a linear scan:
    /// scanning would make this quadratic, and a quadratic `Core` member over
    /// request-shaped input is a denial of service rather than a slow path.
    /// The cost is one `HashSet` entry per *distinct* value.
    fn mwl_core_arr_unique(ctx, args: [2]) {
        let subject = subject(args, "unique")?;
        let by = optional_callback(&args[1], "unique", "by")?;

        // Freed on every exit path, including a throw out of the extractor —
        // see [`Extracted`]. Declared before the set so the values it owns
        // outlive every borrow of them; the set holds only `Value` bits,
        // which own nothing themselves.
        let mut extracted = Extracted(Vec::new());
        let mut seen: std::collections::HashSet<Identity> = std::collections::HashSet::new();
        let mut out = MwlArray::new();
        let mut from = 0usize;
        while let Some(slot) = subject.next_slot(from) {
            let value = subject
                .value_at(slot)
                .expect("next_slot only names live entries");
            let key = subject
                .key_at(slot)
                .expect("next_slot only names live entries");
            from = slot + 1;

            let compared = match by {
                None => value,
                Some(by) => {
                    // One reference for the duration of the call, released
                    // right after: `call_closure` takes its own.
                    let key_arg = Value::str(key.clone());
                    let named = mwl_runtime::call_closure(ctx, by, &[value, key_arg]);
                    #[expect(
                        unsafe_code,
                        reason = "this frame owns exactly the reference \
                                  `key.clone()` just produced"
                    )]
                    unsafe {
                        key_arg.release();
                    }
                    let named = named?;
                    extracted.0.push(named);
                    named
                }
            };

            if !seen.insert(Identity(compared)) {
                continue;
            }
            #[expect(
                unsafe_code,
                reason = "the entry is owned by the subject array, which \
                          outlives this call, so the copy stored here needs a \
                          reference of its own"
            )]
            unsafe {
                value.retain();
            }
            out.set(key, value);
        }
        Ok(Value::array(out))
    }
}

/// Calls `each` with every layer of a combination member's variadic tail, in
/// the order the call wrote them.
///
/// The tail arrives as **one** `array` argument holding the trailing
/// arguments under `"0"`, `"1"`, … — [`crate::registry::CoreTy::Variadic`]
/// owns why — so all four of ADR 0069's members are ordinary two-slot helpers
/// and this is the walk they share. A layer whose tag is not `Tag::Array` is
/// a fatal rather than a skip: the row declares `array<U>`, so meeting
/// anything else means the value did not come through the checker.
fn for_each_layer(
    tail: &Value,
    member: &str,
    mut each: impl FnMut(&MwlArray) -> Result<(), Fault>,
) -> Result<(), Fault> {
    for_each_trailing(tail, member, |value| {
        let layer = value.array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::{member} expected {:?} for a layer, got tag {}",
                Tag::Array,
                value.tag_byte()
            ))
        })?;
        each(&borrowed(layer))
    })
}

/// Calls `each` with every value of a variadic tail, in the order the call
/// wrote them.
///
/// The tail is **one** `array` argument holding the trailing arguments under
/// `"0"`, `"1"`, … — [`crate::registry::CoreTy::Variadic`] owns why — so a
/// member with one is an ordinary two-slot helper and this is the walk it
/// starts from. The values are *borrowed*: they belong to the tail array,
/// which the calling frame owns for the length of the call, so anything
/// stored out of one takes a reference of its own ([`append_borrowed`]).
/// [`for_each_layer`] is this with each value decoded as an array first.
fn for_each_trailing(
    tail: &Value,
    member: &str,
    mut each: impl FnMut(Value) -> Result<(), Fault>,
) -> Result<(), Fault> {
    let values = tail.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Arr::{member} expected {:?} for its trailing arguments, got tag {}",
            Tag::Array,
            tail.tag_byte()
        ))
    })?;
    let values = borrowed(values);
    let mut from = 0usize;
    while let Some(slot) = values.next_slot(from) {
        let value = values
            .value_at(slot)
            .expect("next_slot only names live entries");
        from = slot + 1;
        each(value)?;
    }
    Ok(())
}

/// Copies every entry of `subject` into `out`, key and all — [`copy_entry`]
/// over the whole array.
fn copy_all(subject: &MwlArray, out: &mut MwlArray) {
    let mut from = 0usize;
    while let Some(slot) = subject.next_slot(from) {
        copy_entry(subject, slot, out);
        from = slot + 1;
    }
}

/// ADR 0069 § 1's overlay walk: `layer`'s entries written over `out`, an
/// existing key **replacing in place** and a new key landing at the end.
///
/// The key order falls out of `MwlArray::set` rather than being arranged
/// here — replacing an indexed key leaves its slot where it was, and a new
/// one is pushed — which is what makes `overlay($b, $a)` and
/// `underlay($a, $b)` two operations rather than one with its arguments
/// flipped.
///
/// `deep` is `overlayDeep`'s one extra rule and the only difference between
/// the two members; [`merged`] holds it.
fn overlay_into(out: &mut MwlArray, layer: &MwlArray, deep: bool) {
    let mut from = 0usize;
    while let Some(slot) = layer.next_slot(from) {
        let key = layer
            .key_at(slot)
            .expect("next_slot only names live entries");
        let value = layer
            .value_at(slot)
            .expect("next_slot only names live entries");
        from = slot + 1;

        if deep && let Some(nested) = merged(out, key.as_bytes(), value) {
            // `Value::array` takes over the fresh allocation's only reference,
            // and `set` releases whatever it displaces — the array this one
            // was built from.
            out.set(key, Value::array(nested));
            continue;
        }
        #[expect(
            unsafe_code,
            reason = "the entry is owned by the layer, which outlives this \
                      call, so the copy stored here needs a reference of its own"
        )]
        unsafe {
            value.retain();
        }
        out.set(key, value);
    }
}

/// The recursive half of [`overlay_into`]: what `key` should hold once
/// `value` is overlaid onto whatever `out` already has there, or `None` where
/// ADR 0069 § 1's test says the right-hand value replaces the left wholesale.
///
/// The test is *both* sides holding an array and **neither** being a list.
/// A list is replaced rather than merged element-wise because element-wise is
/// the surprise in PHP's `array_replace_recursive` — overlaying `[9]` onto
/// `[1, 2, 3]` yielding `[9, 2, 3]` is never what a configuration merge
/// wanted — and [`is_list`] is the predicate so the rule is stated in terms
/// the language already has.
///
/// The result is a fresh array rather than a mutation of the existing one:
/// an MWL array is a copy-on-write *value* (ADR 0007 § 5), so the entry `out`
/// holds may be shared with the caller's own binding and writing through it
/// would be visible there.
fn merged(out: &MwlArray, key: &[u8], value: Value) -> Option<MwlArray> {
    let existing = borrowed(out.get(key)?.array_ptr()?);
    let incoming = borrowed(value.array_ptr()?);
    if is_list(&existing) || is_list(&incoming) {
        return None;
    }
    let mut nested = MwlArray::new();
    copy_all(&existing, &mut nested);
    overlay_into(&mut nested, &incoming, true);
    Some(nested)
}

/// ADR 0069 § 1's underlay walk: `layer`'s entries written *under* `out`, an
/// existing key ignored and a new key landing at the end.
fn underlay_into(out: &mut MwlArray, layer: &MwlArray) {
    let mut from = 0usize;
    while let Some(slot) = layer.next_slot(from) {
        let key = layer
            .key_at(slot)
            .expect("next_slot only names live entries");
        from = slot + 1;
        if out.has_key(key.as_bytes()) {
            continue;
        }
        copy_entry(layer, slot, out);
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::overlay(array<T> $base, array<U> ...$layers): array<T|U>` —
    /// [ADR 0069](../../../../docs/adr/0069-array-combination-is-key-type-independent.md)
    /// § 1's right-wins combination, replacing PHP's `array_replace` exactly
    /// and its `array_merge` over maps.
    ///
    /// **Every key is treated the same way**, which is the whole of that ADR:
    /// PHP's `array_merge` renumbers integer keys and keeps string ones, so
    /// the same call is a replace or an append depending on data the call site
    /// cannot see. Here an existing key replaces in place and a new one is
    /// appended, whatever the key looks like.
    ///
    /// [`overlay_into`] owns the walk and the key order; [`COMBINED`] owns
    /// why the return type is spelled `array<T|U>` rather than `array<mixed>`.
    fn mwl_core_arr_overlay(_ctx, args: [2]) {
        let base = subject(args, "overlay")?;
        let mut out = MwlArray::new();
        copy_all(&base, &mut out);
        for_each_layer(&args[1], "overlay", |layer| {
            overlay_into(&mut out, layer, false);
            Ok(())
        })?;
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::overlayDeep(array<T> $base, array<U> ...$layers): array<T|U>`
    /// — [`mwl_core_arr_overlay`] with ADR 0069 § 1's recursion rule,
    /// replacing PHP's `array_replace_recursive`.
    ///
    /// It recurses only where both sides of a key hold an array and neither
    /// is a list; [`merged`] holds why, and it is also why
    /// `array_merge_recursive` has no replacement at all — promoting two
    /// colliding scalars into a two-element array is a data-shape change
    /// rather than a merge.
    fn mwl_core_arr_overlay_deep(_ctx, args: [2]) {
        let base = subject(args, "overlayDeep")?;
        let mut out = MwlArray::new();
        copy_all(&base, &mut out);
        for_each_layer(&args[1], "overlayDeep", |layer| {
            overlay_into(&mut out, layer, true);
            Ok(())
        })?;
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::underlay(array<T> $base, array<U> ...$layers): array<T|U>` —
    /// ADR 0069 § 1's left-wins combination, exactly PHP's `$a + $b` and the
    /// member `E0467` names when a program writes that operator.
    ///
    /// Not `overlay` with its arguments flipped: `overlay($b, $a)` holds the
    /// same entries but in `$b`'s key order, and an MWL array is
    /// insertion-ordered, so the difference is observable in `foreach`, in
    /// `Core\Json::encode` and in every `Arr::first`. Two behaviours, two
    /// names — ADR 0063 R15.
    fn mwl_core_arr_underlay(_ctx, args: [2]) {
        let base = subject(args, "underlay")?;
        let mut out = MwlArray::new();
        copy_all(&base, &mut out);
        for_each_layer(&args[1], "underlay", |layer| {
            underlay_into(&mut out, layer);
            Ok(())
        })?;
        Ok(Value::array(out))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::appendAll(array<T> $a, array<U> ...$others): array<T|U>` —
    /// ADR 0069 § 1's key-discarding combination, replacing PHP's
    /// `array_merge` over lists and its `array_merge(...$arrays)` flatten
    /// idiom.
    ///
    /// **Always a list**, whatever the arguments were: every value of every
    /// argument in order, under fresh keys. That is the half of `array_merge`
    /// a call site actually meant when its arguments were lists, and naming
    /// it separately is what lets `overlay` be the other half without either
    /// one deciding by key type at run time.
    fn mwl_core_arr_append_all(_ctx, args: [2]) {
        let base = subject(args, "appendAll")?;
        let mut out = MwlArray::new();
        append_values(&base, &mut out);
        for_each_layer(&args[1], "appendAll", |layer| {
            append_values(layer, &mut out);
            Ok(())
        })?;
        Ok(Value::array(out))
    }
}

/// Which part of an entry a set member compares — spec § 2's `Core\SetOn`,
/// decoded from the `{on: ...}` option's integer case.
///
/// [`SET_ON`] is where the cases and their values are stated; this is the
/// same closed set read back on the runtime side, and the two cannot drift
/// because the default is written as a named case rather than as its number.
#[derive(Clone, Copy, PartialEq, Eq)]
enum On {
    Values,
    Keys,
    Both,
}

/// The `{on: ...}` option as an [`On`].
fn on_of(value: &Value, member: &str) -> Result<On, Fault> {
    match value.as_int() {
        Some(0) => Ok(On::Values),
        Some(1) => Ok(On::Keys),
        Some(2) => Ok(On::Both),
        _ => Err(Fault::fatal(format!(
            "Core\\Arr::{member} expected a `Core\\SetOn` case for `on`, got tag {} value {}",
            value.tag_byte(),
            value.bits()
        ))),
    }
}

/// What one entry contributes to a set member's comparison, under `on` and
/// `by`.
///
/// **`on` selects the part that is compared and `by` maps the part it
/// selected.** Under `Values` and `Both` that part is the value; under `Keys`
/// it is the key, so a `by` there replaces the key rather than being quietly
/// ignored — an option that could not affect the answer would be the worse of
/// the two designs. The callback receives `($value, $key)` in every case, as
/// every other `Core\Arr` callback does.
///
/// Every reference produced here is handed to `extracted`, which releases it
/// on the way out: a callback's answer belongs to this frame, and so does the
/// `Value` a key is wrapped in.
fn comparison_subject(
    ctx: &mut mwl_runtime::Ctx,
    on: On,
    by: Option<Value>,
    key: &MwlStr,
    value: Value,
    extracted: &mut Extracted,
) -> Result<Value, Fault> {
    let Some(by) = by else {
        if on != On::Keys {
            return Ok(value);
        }
        let held = Value::str(key.clone());
        extracted.0.push(held);
        return Ok(held);
    };

    // One reference for the duration of the call, released right after:
    // `call_closure` takes its own.
    let key_arg = Value::str(key.clone());
    let answer = mwl_runtime::call_closure(ctx, by, &[value, key_arg]);
    #[expect(
        unsafe_code,
        reason = "this frame owns exactly the reference `key.clone()` just produced"
    )]
    unsafe {
        key_arg.release();
    }
    let answer = answer?;
    extracted.0.push(answer);
    Ok(answer)
}

/// The shared body of [`mwl_core_arr_diff`] and [`mwl_core_arr_intersect`]:
/// the entries of `args[0]` whose presence in `args[1]` is `keep_when_present`.
///
/// The two members ask one question and keep opposite answers, so they are one
/// walk — the same pairing [`slot_of`] serves for `contains`/`keyOf`. Twelve
/// PHP functions collapse into it: `on` chooses what is compared, `by` maps it
/// and `comparator` replaces identity with a program.
///
/// **Linear where it can be, quadratic only where a comparator forces it.**
/// Without one, identity is hashable ([`Identity`]), so the other side goes
/// into a set built in one pass and each entry of the subject is one lookup. A
/// comparator is an arbitrary function with no hash to agree with, so that
/// path is the pairwise scan PHP's `array_udiff` also is. Under `On::Both` the
/// index is keyed by the other side's key bytes, which costs one `Vec<u8>` per
/// entry of it — spent, per AGENTS.md's priority ordering, to keep the common
/// `array_diff_assoc` shape off the quadratic path.
fn set_member(
    ctx: &mut mwl_runtime::Ctx,
    args: &[Value],
    member: &str,
    keep_when_present: bool,
) -> Result<Value, Fault> {
    let subject = subject(args, member)?;
    let other = args[1].array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Arr::{member} expected {:?} for the second array, got tag {}",
            Tag::Array,
            args[1].tag_byte()
        ))
    })?;
    let other = borrowed(other);
    let on = on_of(&args[2], member)?;
    let by = optional_callback(&args[3], member, "by")?;
    let comparator = optional_callback(&args[4], member, "comparator")?;

    // Freed on every exit path, including a throw out of a callback — see
    // [`Extracted`]. Declared before anything that borrows from it.
    let mut extracted = Extracted(Vec::new());

    // The other side's comparison subjects, computed once. `by` is called
    // once per entry rather than once per pair, which is the difference
    // between one pass and a quadratic one even on the comparator path.
    let mut theirs: Vec<(MwlStr, Value)> = Vec::new();
    let mut from = 0usize;
    while let Some(slot) = other.next_slot(from) {
        let key = other
            .key_at(slot)
            .expect("next_slot only names live entries");
        let value = other
            .value_at(slot)
            .expect("next_slot only names live entries");
        from = slot + 1;
        let compared = comparison_subject(ctx, on, by, &key, value, &mut extracted)?;
        theirs.push((key, compared));
    }

    let mut values: std::collections::HashSet<Identity> = std::collections::HashSet::new();
    let mut pairs: std::collections::HashMap<Vec<u8>, Identity> = std::collections::HashMap::new();
    if comparator.is_none() {
        for (key, compared) in &theirs {
            if on == On::Both {
                pairs.insert(key.as_bytes().to_vec(), Identity(*compared));
            } else {
                values.insert(Identity(*compared));
            }
        }
    }

    let mut out = MwlArray::new();
    let mut from = 0usize;
    while let Some(slot) = subject.next_slot(from) {
        let key = subject
            .key_at(slot)
            .expect("next_slot only names live entries");
        let value = subject
            .value_at(slot)
            .expect("next_slot only names live entries");
        from = slot + 1;
        let mine = comparison_subject(ctx, on, by, &key, value, &mut extracted)?;

        let present = match comparator {
            None if on == On::Both => pairs
                .get(key.as_bytes())
                .is_some_and(|theirs| *theirs == Identity(mine)),
            None => values.contains(&Identity(mine)),
            Some(comparator) => {
                let mut found = false;
                for (their_key, compared) in &theirs {
                    if on == On::Both && their_key.as_bytes() != key.as_bytes() {
                        continue;
                    }
                    let verdict = mwl_runtime::call_closure(ctx, comparator, &[mine, *compared])?;
                    let sign = comparator_sign(verdict);
                    #[expect(
                        unsafe_code,
                        reason = "the verdict is a fresh value this frame \
                                  owns; a comparator returning a heap value \
                                  would otherwise leak one reference per \
                                  comparison"
                    )]
                    unsafe {
                        verdict.release();
                    }
                    if sign? == std::cmp::Ordering::Equal {
                        found = true;
                        break;
                    }
                }
                found
            }
        };

        if present == keep_when_present {
            #[expect(
                unsafe_code,
                reason = "the entry is owned by the subject array, which \
                          outlives this call, so the copy stored here needs a \
                          reference of its own"
            )]
            unsafe {
                value.retain();
            }
            out.set(key, value);
        }
    }
    Ok(Value::array(out))
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::diff(array<T> $a, array<T> $b, {on?: SetOn, by?: callable, comparator?: callable}): array<T>`
    /// — the entries of `$a` that `$b` does not have, replacing PHP's
    /// `array_diff` and its five variants.
    ///
    /// **Strict identity, never a string cast.** PHP's `array_diff` compares
    /// `(string) $x === (string) $y`, so `1` and `"1"` are the same element
    /// and two arrays are the same element as each other. [ADR 0069](../../../../docs/adr/0069-array-combination-is-key-type-independent.md)
    /// § 3 calls that a bug source rather than a decision; this compares the
    /// way `contains` and `unique` already do, which is `mwl_runtime`'s
    /// `value_identical`.
    ///
    /// `$a`'s keys and order are kept, as PHP keeps them. [`set_member`] owns
    /// the walk, the `on`/`by`/`comparator` composition and the cost.
    fn mwl_core_arr_diff(ctx, args: [5]) {
        set_member(ctx, args, "diff", false)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::intersect(array<T> $a, array<T> $b, {on?: SetOn, by?: callable, comparator?: callable}): array<T>`
    /// — the entries of `$a` that `$b` also has, replacing PHP's
    /// `array_intersect` and its five variants.
    ///
    /// The same question [`mwl_core_arr_diff`] asks with the other answer
    /// kept, and the same strict-identity rule.
    fn mwl_core_arr_intersect(ctx, args: [5]) {
        set_member(ctx, args, "intersect", true)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::min(array<T> $a): ?T` — the smallest entry under the
    /// natural ordering, replacing PHP's `min` with an array argument.
    ///
    /// `null` over an empty array rather than PHP's `ValueError`, on
    /// [`mwl_core_arr_first`]'s terms: ADR 0063 R5 makes `?T` the absence
    /// spelling, and "what is the smallest of nothing" is a question with an
    /// answer, not a failure.
    ///
    /// The ordering is [`compare_values`] — the same total order
    /// [`mwl_core_arr_sort`] uses without a comparator, so
    /// `min($a) === first(sort($a))` holds by construction. It is therefore
    /// *not* PHP's `min`, which compares loosely: `min([0, "a"])` is `"a"` in
    /// PHP 8 and a throw here, because a `string` and an `int` have no order
    /// between them. The spec's § 2 has no comparator option on either
    /// member; a caller who wants one writes `first(sort($a, {by: ...}))`.
    ///
    /// PHP's variadic `min(1, 2, 3)` has no member at all: that is what `<`
    /// and a ternary are for (ADR 0063 R17), and the array form is the one
    /// that cannot be written in the language.
    fn mwl_core_arr_min(_ctx, args: [1]) {
        let subject = subject(args, "min")?;
        extremum(&subject, std::cmp::Ordering::Less, r"Core\Arr::min")
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::max(array<T> $a): ?T` — the largest entry under the natural
    /// ordering, replacing PHP's `max` with an array argument.
    ///
    /// [`mwl_core_arr_min`] owns the ordering, the empty case and the
    /// divergence from PHP's loose comparison.
    fn mwl_core_arr_max(_ctx, args: [1]) {
        let subject = subject(args, "max")?;
        extremum(&subject, std::cmp::Ordering::Greater, r"Core\Arr::max")
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::sum(array<int|float|decimal> $a): int|float|decimal` —
    /// replacing PHP's `array_sum`.
    ///
    /// [`Total`] owns the three-way accumulation and every throw it can
    /// produce; the empty array is `int` `0`, PHP's own answer and the
    /// identity of the operation.
    fn mwl_core_arr_sum(_ctx, args: [1]) {
        let subject = subject(args, "sum")?;
        fold_numbers(&subject, Total::Integer(0), "sum")
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::product(array<int|float|decimal> $a): int|float|decimal` —
    /// replacing PHP's `array_product`. [`mwl_core_arr_sum`]'s structure with
    /// the other operation, so the empty array is `int` `1`, that operation's
    /// identity and PHP's answer.
    fn mwl_core_arr_product(_ctx, args: [1]) {
        let subject = subject(args, "product")?;
        fold_numbers(&subject, Total::Integer(1), "product")
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Arr::average(array<int|float|decimal> $a): ?(float|decimal)` —
    /// the spec's replacement for `array_sum($a)/count($a)`, "with the empty
    /// case answered".
    ///
    /// **`null` over an empty array**, on [`mwl_core_arr_min`]'s terms: ADR
    /// 0063 R5 makes `?T` the absence spelling, and the average of nothing is
    /// a question with an answer rather than a division by zero.
    ///
    /// An exact subject stays exact: a `decimal` total is divided by the count
    /// as a `decimal`, which ADR 0054 § 3 rounds half to even at the widest
    /// scale the quotient admits. Every other total answers `float`, which is
    /// what makes this member's result type a union rather than one type.
    fn mwl_core_arr_average(_ctx, args: [1]) {
        let subject = subject(args, "average")?;
        let count = subject.count();
        if count == 0 {
            return Ok(Value::null());
        }
        let divisor = u64::try_from(count).map_err(|_| {
            Fault::fatal("Core\\Arr::average was given more entries than a `uint` counts")
        })?;
        match fold(&subject, Total::Integer(0), "average", Total::plus)? {
            Total::Exact(total) => total
                .checked_div(Decimal::from_u64(divisor))
                .map(Value::decimal)
                .ok_or_else(|| {
                    Fault::thrown(
                        "Core\\Arr::average has no `decimal` answer for this subject: the \
                         quotient is outside ADR 0054 § 1's range"
                            .to_owned(),
                    )
                }),
            other => Ok(Value::float(other.as_f64() / f64_of(divisor))),
        }
    }
}

/// What [`mwl_core_arr_sum`], [`mwl_core_arr_product`] and
/// [`mwl_core_arr_average`] accumulate into — spec § 2's
/// `int|float|decimal` as the three shapes an accumulator can be in.
///
/// The promotion rules are ADR 0007 § 4's and ADR 0054 § 3's, applied
/// entry by entry rather than to a pair of static types:
///
/// * `int` with `int` stays `int` and **throws** on overflow — no wrap and no
///   promotion to `float`, which is that ADR's row verbatim and a deliberate
///   divergence from `array_sum`, which silently becomes a `float`.
/// * a `float` anywhere makes the total a `float`, exactly as `int ⊕ float`
///   does.
/// * a `decimal` anywhere makes the total a `decimal`, since an `int` is exact
///   in 96 bits.
/// * a `float` and a `decimal` in **one** subject throws: ADR 0054 § 3 makes
///   that pair a compile error where the types are static, and there is no
///   representable common type here either. It is reachable only through an
///   `array<int|float|decimal>` holding both.
#[derive(Clone, Copy)]
enum Total {
    /// An `int`, or a `uint` small enough to be one.
    Integer(i64),
    /// A `float`.
    Real(f64),
    /// A `decimal` — ADR 0054's scalar, and the one arm that is exact.
    Exact(Decimal),
}

impl Total {
    /// One entry, decoded — `None` where the tag is not a number at all,
    /// which is a miscompile rather than a program error.
    fn of(value: Value) -> Option<Self> {
        if let Some(int) = value.as_int() {
            return Some(Self::Integer(int));
        }
        if let Some(uint) = value.as_uint() {
            return Some(i64::try_from(uint).map_or_else(
                // Past `int`'s largest, and still exact in 96 bits.
                |_| Self::Exact(Decimal::from_u64(uint)),
                Self::Integer,
            ));
        }
        if let Some(float) = value.as_float() {
            return Some(Self::Real(float));
        }
        value.as_decimal().map(Self::Exact)
    }

    /// This total as a `float`, for the two arms that answer one.
    fn as_f64(self) -> f64 {
        match self {
            Self::Integer(n) => f64_of_i64(n),
            Self::Real(n) => n,
            // Only reached through `average`'s own arm, which handles `Exact`
            // before it asks.
            Self::Exact(n) => n.to_f64(),
        }
    }

    /// `a + b`, promoting by this type's own docs.
    fn plus(self, other: Self) -> Option<Self> {
        Self::combine(
            self,
            other,
            i64::checked_add,
            |a, b| a + b,
            Decimal::checked_add,
        )
    }

    /// `a * b`, promoting the same way.
    fn times(self, other: Self) -> Option<Self> {
        Self::combine(
            self,
            other,
            i64::checked_mul,
            |a, b| a * b,
            Decimal::checked_mul,
        )
    }

    fn combine(
        left: Self,
        right: Self,
        integer: fn(i64, i64) -> Option<i64>,
        real: fn(f64, f64) -> f64,
        exact: fn(Decimal, Decimal) -> Option<Decimal>,
    ) -> Option<Self> {
        match (left, right) {
            (Self::Integer(a), Self::Integer(b)) => integer(a, b).map(Self::Integer),
            (Self::Exact(a), Self::Exact(b)) => exact(a, b).map(Self::Exact),
            (Self::Exact(a), Self::Integer(b)) => exact(a, Decimal::from_i64(b)).map(Self::Exact),
            (Self::Integer(a), Self::Exact(b)) => exact(Decimal::from_i64(a), b).map(Self::Exact),
            // The one pair with no representable common type.
            (Self::Exact(_), Self::Real(_)) | (Self::Real(_), Self::Exact(_)) => None,
            (a, b) => Some(Self::Real(real(a.as_f64(), b.as_f64()))),
        }
    }

    /// This total as the value a member answers with.
    fn into_value(self) -> Value {
        match self {
            Self::Integer(n) => Value::int(n),
            Self::Real(n) => Value::float(n),
            Self::Exact(n) => Value::decimal(n),
        }
    }
}

/// [`Total::add`] over every entry, as [`mwl_core_arr_sum`]'s answer.
fn fold_numbers(subject: &MwlArray, seed: Total, member: &str) -> Result<Value, Fault> {
    let operation = if member == "sum" {
        Total::plus
    } else {
        Total::times
    };
    Ok(fold(subject, seed, member, operation)?.into_value())
}

/// The fold itself, shared by all three members.
fn fold(
    subject: &MwlArray,
    seed: Total,
    member: &str,
    operation: fn(Total, Total) -> Option<Total>,
) -> Result<Total, Fault> {
    let mut total = seed;
    let mut from = 0usize;
    while let Some(slot) = subject.next_slot(from) {
        let value = subject
            .value_at(slot)
            .expect("next_slot only names live entries");
        from = slot + 1;
        let entry = Total::of(value).ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::{member} expected a number in every entry, got tag {}",
                value.tag_byte()
            ))
        })?;
        total = operation(total, entry).ok_or_else(|| {
            Fault::thrown(format!(
                "Core\\Arr::{member} has no answer for this subject: the running total either \
                 left its type's range or met a `float` and a `decimal` in one array, which \
                 have no common type (ADR 0007 § 4, ADR 0054 § 3)"
            ))
        })?;
    }
    Ok(total)
}

/// A count as the `f64` it divides by. Exact for every array this runtime can
/// hold, which is far below 2^53 entries.
fn f64_of(count: u64) -> f64 {
    f64_of_i64(i64::try_from(count).unwrap_or(i64::MAX))
}

/// An `i64` as an `f64` — the one narrowing cast this module makes, and the
/// only place a total can lose precision. It is what `int|float` means: past
/// 2^53 the quotient is a `float`, and a `float` is what the signature says.
#[expect(
    clippy::cast_precision_loss,
    reason = "an average is a `float` by declaration; the exact answer is what a `decimal` subject gets"
)]
fn f64_of_i64(value: i64) -> f64 {
    value as f64
}

/// One value, compared and hashed the way `mwl_runtime` defines identity —
/// what puts a `Core\Arr` set member's seen values in a `HashSet` instead of a
/// linear scan.
///
/// It borrows: a `Value` owns nothing on its own (`mwl_runtime::Value`'s
/// *Ownership* section), so whatever reference keeps the payload alive must
/// outlive the set. Every use below satisfies that by construction — the
/// subject array outlives the call, and a `by`-extracted value is held by an
/// [`Extracted`] guard declared first.
///
/// **`Eq` is not quite reflexive**, because identity is not: a `NaN` entry is
/// identical to nothing, itself included. The consequence is the intended one
/// and the only one — a set never matches a `NaN`, so every `NaN` entry
/// survives `unique` — and it is a logic-error-only contract in `std`, never
/// a soundness one.
#[derive(Clone, Copy)]
struct Identity(Value);

impl PartialEq for Identity {
    fn eq(&self, other: &Self) -> bool {
        mwl_runtime::value_identical(self.0, other.0)
    }
}

impl Eq for Identity {}

impl std::hash::Hash for Identity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        mwl_runtime::value_hash(self.0, state);
    }
}

/// The first slot holding `needle`, by strict identity, or `None`.
///
/// Shared by [`mwl_core_arr_contains`] and [`mwl_core_arr_key_of`], which ask
/// the same question and read a different half of the answer — the same
/// pairing [`find_slot`] serves for the predicate members. Linear, and
/// deliberately so: both members answer about *one* needle, so there is
/// nothing to amortize an index over.
fn slot_of(subject: &MwlArray, needle: Value) -> Option<usize> {
    let mut from = 0usize;
    while let Some(slot) = subject.next_slot(from) {
        let value = subject
            .value_at(slot)
            .expect("next_slot only names live entries");
        if mwl_runtime::value_identical(value, needle) {
            return Some(slot);
        }
        from = slot + 1;
    }
    None
}

/// The entry that compares `wanted` against every other, as a fresh reference,
/// or `null` over an empty array — [`mwl_core_arr_min`] and
/// [`mwl_core_arr_max`] in one walk.
///
/// The **first** extreme wins a tie, so a stable sort and this member name the
/// same entry.
fn extremum(subject: &MwlArray, wanted: std::cmp::Ordering, member: &str) -> Result<Value, Fault> {
    let mut best: Option<(usize, Value)> = None;
    let mut from = 0usize;
    while let Some(slot) = subject.next_slot(from) {
        let value = subject
            .value_at(slot)
            .expect("next_slot only names live entries");
        from = slot + 1;
        match best {
            None => best = Some((slot, value)),
            Some((_, incumbent)) => {
                if compare_values(&value, &incumbent, member)? == wanted {
                    best = Some((slot, value));
                }
            }
        }
    }
    Ok(match best {
        Some((slot, _)) => owned_value_at(subject, slot),
        None => Value::null(),
    })
}

#[cfg(test)]
mod tests {
    use mwl_runtime::{Ctx, MwlArray, MwlStr, OutputSink, Value, call};

    /// The member end to end through the ADR 0002 boundary compiled code will
    /// reach it at — `call` builds the same three pointers a JIT frame does.
    #[test]
    fn count_reports_the_number_of_live_entries() {
        let mut array = MwlArray::new();
        array.set(MwlStr::new(b"a"), Value::int(1));
        array.set(MwlStr::new(b"b"), Value::int(2));
        array.unset(b"a");

        let mut ctx = Ctx::new(OutputSink::Sink);
        let subject = Value::array(array);
        let result = call(super::mwl_core_arr_count, &mut ctx, &[subject])
            .expect("counting an array never fails");
        assert_eq!(result.as_uint(), Some(1));

        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built above, and the \
                      helper borrowed rather than consumed it"
        )]
        unsafe {
            subject.release();
        }
    }

    /// Both spellings of a key find the same entry, which is what `key_bytes`
    /// exists for: `mwl-ir` already normalizes an `int` subscript to its
    /// decimal string, and a helper argument arrives untouched, so this is the
    /// other half of the same rule.
    #[test]
    fn has_key_normalizes_an_int_key_the_way_a_subscript_does() {
        let mut array = MwlArray::new();
        array.set(MwlStr::new(b"5"), Value::int(1));
        array.set(MwlStr::new(b"name"), Value::int(2));
        let subject = Value::array(array);

        let asked = |key: Value| {
            let mut ctx = Ctx::new(OutputSink::Sink);
            call(super::mwl_core_arr_has_key, &mut ctx, &[subject, key])
                .expect("asking never fails")
                .as_bool()
                .expect("hasKey returns a bool")
        };
        assert!(asked(Value::int(5)));
        assert!(asked(Value::str(MwlStr::new(b"5"))));
        assert!(asked(Value::str(MwlStr::new(b"name"))));
        assert!(!asked(Value::int(6)));
        assert!(!asked(Value::str(MwlStr::new(b"nope"))));

        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built above, and the \
                      helper borrowed rather than consumed it"
        )]
        unsafe {
            subject.release();
        }
    }

    /// The borrowed handle `hasKey` reads through must not disturb the
    /// caller's reference count — a release there would be a double free, and
    /// a retain there would leak one reference per call.
    #[test]
    fn asking_for_a_key_leaves_the_subjects_refcount_alone() {
        let mut array = MwlArray::new();
        array.set(MwlStr::new(b"a"), Value::int(1));
        let before = array.refcount();
        let subject = Value::array(array);

        let mut ctx = Ctx::new(OutputSink::Sink);
        call(
            super::mwl_core_arr_has_key,
            &mut ctx,
            &[subject, Value::str(MwlStr::new(b"a"))],
        )
        .expect("asking never fails");

        // The handle takes over the one reference this test owns and drops at
        // the end of the statement, which is also this test's release.
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built above, and the \
                      helper borrowed rather than consumed it"
        )]
        let after =
            unsafe { MwlArray::from_raw(subject.array_ptr().expect("an array")) }.refcount();
        assert_eq!(after, before);
    }

    /// Every row verified against PHP 8.5's own `array_is_list`, including the
    /// two ADR 0007 § 5 makes interesting: a canonical integer *string* key is
    /// a list key (PHP normalizes it to an int), and a non-canonical one
    /// (`"01"`) is not.
    #[test]
    fn is_list_matches_phps_answer_for_every_key_shape() {
        let asked = |keys: &[&[u8]]| {
            let mut array = MwlArray::new();
            for key in keys {
                array.set(MwlStr::new(key), Value::int(1));
            }
            let subject = Value::array(array);
            let mut ctx = Ctx::new(OutputSink::Sink);
            let answer = call(super::mwl_core_arr_is_list, &mut ctx, &[subject])
                .expect("asking never fails")
                .as_bool()
                .expect("isList returns a bool");
            #[expect(
                unsafe_code,
                reason = "this test owns the one reference it built above, and \
                          the helper borrowed rather than consumed it"
            )]
            unsafe {
                subject.release();
            }
            answer
        };
        assert!(asked(&[]));
        assert!(asked(&[b"0", b"1", b"2"]));
        assert!(!asked(&[b"x", b"y"]));
        assert!(!asked(&[b"1", b"0"]));
        assert!(!asked(&[b"0", b"2"]));
        assert!(!asked(&[b"01"]));
        // A hole left by an `unset` renumbers nothing, so the array stops
        // being a list — the same answer PHP gives after `unset($a[0])`.
        let mut array = MwlArray::new();
        array.set(MwlStr::new(b"0"), Value::int(1));
        array.set(MwlStr::new(b"1"), Value::int(2));
        array.unset(b"0");
        let subject = Value::array(array);
        let mut ctx = Ctx::new(OutputSink::Sink);
        assert_eq!(
            call(super::mwl_core_arr_is_list, &mut ctx, &[subject])
                .expect("asking never fails")
                .as_bool(),
            Some(false)
        );
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built above, and the \
                      helper borrowed rather than consumed it"
        )]
        unsafe {
            subject.release();
        }
    }

    /// `values` renumbers from zero and keeps insertion order, and takes a
    /// reference of its own for every value it copies — a missing retain is a
    /// double free the moment either array is dropped.
    #[test]
    fn values_renumbers_from_zero_and_retains_what_it_copies() {
        let mut array = MwlArray::new();
        array.set(MwlStr::new(b"x"), Value::str(MwlStr::new(b"a")));
        array.set(MwlStr::new(b"y"), Value::str(MwlStr::new(b"b")));
        let subject = Value::array(array);

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(super::mwl_core_arr_values, &mut ctx, &[subject])
            .expect("taking values never fails");
        #[expect(
            unsafe_code,
            reason = "the helper returned one fresh reference, which the handle \
                      takes over and releases on drop"
        )]
        let out =
            unsafe { MwlArray::from_raw(result.array_ptr().expect("values returns an array")) };
        assert_eq!(out.keys(), vec![b"0".to_vec(), b"1".to_vec()]);
        assert_eq!(
            out.get(b"0")
                .and_then(|v| v.as_str_bytes().map(<[u8]>::to_vec)),
            Some(b"a".to_vec())
        );
        assert_eq!(
            out.get(b"1")
                .and_then(|v| v.as_str_bytes().map(<[u8]>::to_vec)),
            Some(b"b".to_vec())
        );
        drop(out);

        // The subject still holds its own values after the result is gone,
        // which is what the retain bought.
        #[expect(
            unsafe_code,
            reason = "this test still owns the one reference it built above"
        )]
        let still = unsafe { MwlArray::from_raw(subject.array_ptr().expect("an array")) };
        assert_eq!(
            still
                .get(b"x")
                .and_then(|v| v.as_str_bytes().map(<[u8]>::to_vec)),
            Some(b"a".to_vec())
        );
    }

    /// The entries of an array a helper returned, in cursor order, as
    /// `(key, value-as-string)` pairs — the shape the three key-shuffling
    /// members below are all asserted in, so a wrong *order* fails and not
    /// only a wrong set.
    fn entries_of(result: Value) -> Vec<(Vec<u8>, Vec<u8>)> {
        #[expect(
            unsafe_code,
            reason = "the helper returned one fresh reference, which the handle \
                      takes over and releases on drop"
        )]
        let array =
            unsafe { MwlArray::from_raw(result.array_ptr().expect("the member returns an array")) };
        let mut out = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let key = array.key_at(slot).expect("a live entry has a key");
            let value = array
                .value_at(slot)
                .and_then(|v| v.as_str_bytes().map(<[u8]>::to_vec))
                .expect("every entry here is a string");
            out.push((key.as_bytes().to_vec(), value));
            from = slot + 1;
        }
        out
    }

    /// `["x" => "a", "10" => "b", "y" => "c"]`, the mixed-looking-key subject
    /// the three members share.
    fn mixed_keys() -> Value {
        let mut array = MwlArray::new();
        array.set(MwlStr::new(b"x"), Value::str(MwlStr::new(b"a")));
        array.set(MwlStr::new(b"10"), Value::str(MwlStr::new(b"b")));
        array.set(MwlStr::new(b"y"), Value::str(MwlStr::new(b"c")));
        Value::array(array)
    }

    /// Verified against PHP 8.5's `array_keys`, except that the `"10"` key
    /// comes back as the string it is stored as rather than as an `int`.
    #[test]
    fn keys_yields_the_stored_spelling_of_every_key() {
        let subject = mixed_keys();
        let mut ctx = Ctx::new(OutputSink::Sink);
        let result =
            call(super::mwl_core_arr_keys, &mut ctx, &[subject]).expect("taking keys never fails");
        assert_eq!(
            entries_of(result),
            vec![
                (b"0".to_vec(), b"x".to_vec()),
                (b"1".to_vec(), b"10".to_vec()),
                (b"2".to_vec(), b"y".to_vec()),
            ]
        );

        // The subject still holds its own keys, which is what `key_at`'s fresh
        // reference bought.
        #[expect(
            unsafe_code,
            reason = "this test still owns the one reference it built above"
        )]
        let still = unsafe { MwlArray::from_raw(subject.array_ptr().expect("an array")) };
        assert_eq!(
            still.keys(),
            vec![b"x".to_vec(), b"10".to_vec(), b"y".to_vec()]
        );
    }

    /// The default discards every key rather than PHP's renumber-the-integers-
    /// keep-the-strings, which is ADR 0069 § 3's rule and the one place this
    /// member is not `array_reverse`.
    #[test]
    fn reverse_renumbers_by_default_and_keeps_every_key_on_request() {
        let mut ctx = Ctx::new(OutputSink::Sink);

        let renumbered = call(
            super::mwl_core_arr_reverse,
            &mut ctx,
            &[mixed_keys(), Value::bool(false)],
        )
        .expect("reversing never fails");
        assert_eq!(
            entries_of(renumbered),
            vec![
                (b"0".to_vec(), b"c".to_vec()),
                (b"1".to_vec(), b"b".to_vec()),
                (b"2".to_vec(), b"a".to_vec()),
            ]
        );

        let kept = call(
            super::mwl_core_arr_reverse,
            &mut ctx,
            &[mixed_keys(), Value::bool(true)],
        )
        .expect("reversing never fails");
        assert_eq!(
            entries_of(kept),
            vec![
                (b"y".to_vec(), b"c".to_vec()),
                (b"10".to_vec(), b"b".to_vec()),
                (b"x".to_vec(), b"a".to_vec()),
            ]
        );
    }

    /// Verified against PHP 8.5's `array_flip`, duplicate collapse included.
    #[test]
    fn flip_collapses_a_duplicate_value_in_its_first_position() {
        let mut array = MwlArray::new();
        array.set(MwlStr::new(b"a"), Value::str(MwlStr::new(b"p")));
        array.set(MwlStr::new(b"b"), Value::int(7));
        array.set(MwlStr::new(b"c"), Value::str(MwlStr::new(b"p")));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(super::mwl_core_arr_flip, &mut ctx, &[Value::array(array)])
            .expect("an int|string value never fails");
        assert_eq!(
            entries_of(result),
            vec![
                (b"p".to_vec(), b"c".to_vec()),
                (b"7".to_vec(), b"b".to_vec()),
            ]
        );
    }

    /// A value that is neither an `int` nor a `string` is the checker having
    /// let an `array<int|string>` position take something else — a contained
    /// `FATAL`, not a skipped entry the way PHP's warning-and-continue leaves.
    #[test]
    fn flipping_a_value_that_is_not_a_key_is_a_contained_fault() {
        let mut array = MwlArray::new();
        array.set(MwlStr::new(b"a"), Value::bool(true));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(super::mwl_core_arr_flip, &mut ctx, &[Value::array(array)])
            .expect_err("a bool is not a key");
        assert_eq!(status, mwl_runtime::FATAL);
    }

    /// The values `range` produces, in order — read back through the array's
    /// own cursor rather than by key, so a wrong *order* fails here and not
    /// only a wrong set.
    fn range_of(start: i64, end: i64, step: i64) -> Vec<i64> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::mwl_core_arr_range,
            &mut ctx,
            &[Value::int(start), Value::int(end), Value::int(step)],
        )
        .expect("a positive step never fails");
        #[expect(
            unsafe_code,
            reason = "the helper returned one fresh reference, which the \
                      handle takes over and releases on drop"
        )]
        let array =
            unsafe { MwlArray::from_raw(result.array_ptr().expect("range returns an array")) };
        let mut out = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            out.push(
                array
                    .value_at(slot)
                    .and_then(Value::as_int)
                    .expect("every entry is an int"),
            );
            from = slot + 1;
        }
        out
    }

    /// Every row verified against PHP 8.5's own `range`, which is what the
    /// spec's **Replaces** column promises this subsumes.
    #[test]
    fn range_matches_phps_ascending_descending_and_stepped_forms() {
        assert_eq!(range_of(1, 5, 1), vec![1, 2, 3, 4, 5]);
        assert_eq!(range_of(1, 10, 3), vec![1, 4, 7, 10]);
        // A step that overshoots stops at the last in-range value.
        assert_eq!(range_of(1, 10, 4), vec![1, 5, 9]);
        // The direction is the arguments', not the step's — there is no
        // negative step to write.
        assert_eq!(range_of(10, 1, 1), vec![10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(range_of(10, 1, 2), vec![10, 8, 6, 4, 2]);
        // A one-element range, both ways round.
        assert_eq!(range_of(5, 5, 1), vec![5]);
        assert_eq!(range_of(5, 5, 3), vec![5]);
        assert_eq!(range_of(-2, 2, 2), vec![-2, 0, 2]);
    }

    /// A step of zero or less is `THROWN`, not a hang and not a silent
    /// reversal — PHP raises `ValueError` for both.
    #[test]
    fn a_step_of_zero_or_less_throws() {
        for step in [0, -1] {
            let mut ctx = Ctx::new(OutputSink::Sink);
            let status = call(
                super::mwl_core_arr_range,
                &mut ctx,
                &[Value::int(1), Value::int(5), Value::int(step)],
            )
            .expect_err("a non-positive step is refused");
            assert_eq!(status, mwl_runtime::THROWN);
        }
    }

    /// A `chunk` size of zero is `THROWN` rather than an empty answer: no run
    /// length terminates, so answering `[]` would turn a caller's arithmetic
    /// bug into a loop that quietly does nothing. PHP raises `ValueError`.
    ///
    /// Here rather than in a `.mwlt` case because a conformance case cannot
    /// catch at file scope, and the value of this row is the *class* of
    /// failure, not the message.
    #[test]
    fn a_chunk_size_of_zero_throws() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::mwl_core_arr_chunk,
            &mut ctx,
            &[mixed_keys(), Value::uint(0), Value::bool(false)],
        )
        .expect_err("a zero run length is refused");
        assert_eq!(status, mwl_runtime::THROWN);
    }

    /// The cursor stops rather than wrapping when the next step would leave
    /// `int` — ADR 0007 § 4's rule applied to a loop this member owns.
    #[test]
    fn a_range_whose_next_step_would_overflow_stops() {
        assert_eq!(range_of(i64::MAX - 1, i64::MAX, 4), vec![i64::MAX - 1]);
        assert_eq!(range_of(i64::MIN + 1, i64::MIN, 4), vec![i64::MIN + 1]);
    }

    /// `sort`'s five ABI arguments, with the two callbacks absent — the shape
    /// an options bag with nothing written flattens into.
    ///
    /// Written out here rather than hidden behind a builder because the
    /// *order* is `SORT_OPTIONS`' own declared order, and a paste error that
    /// swapped `order` and `preserveKeys` would otherwise be invisible.
    fn sorted(entries: &[(&[u8], Value)], descending: bool, preserve_keys: bool) -> Vec<Vec<u8>> {
        let mut array = MwlArray::new();
        for (key, value) in entries {
            array.set(MwlStr::new(key), *value);
        }
        let subject = Value::array(array);
        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::mwl_core_arr_sort,
            &mut ctx,
            &[
                subject,
                Value::null(),
                Value::int(i64::from(descending)),
                Value::null(),
                Value::bool(preserve_keys),
            ],
        )
        .expect("sorting comparable values never fails");
        #[expect(
            unsafe_code,
            reason = "the helper returned one fresh reference, which the handle \
                      takes over and releases on drop, and this test still owns \
                      the subject it built"
        )]
        let out = unsafe {
            let out = MwlArray::from_raw(result.array_ptr().expect("sort returns an array"));
            subject.release();
            out
        };
        let mut rendered = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = out.next_slot(from) {
            let key = out.key_at(slot).expect("every slot has a key");
            let value = out.value_at(slot).expect("every slot has a value");
            let mut row = key.as_bytes().to_vec();
            row.push(b'=');
            row.extend_from_slice(&rendered_value(value));
            rendered.push(row);
            from = slot + 1;
        }
        rendered
    }

    /// One value as the bytes a test compares — a string's own, or an int's
    /// decimal spelling.
    fn rendered_value(value: Value) -> Vec<u8> {
        value.as_str_bytes().map_or_else(
            || {
                value
                    .as_int()
                    .expect("a test value is a string or an int")
                    .to_string()
                    .into_bytes()
            },
            <[u8]>::to_vec,
        )
    }

    /// Ascending with `preserveKeys` off is PHP's `sort`: renumbered from
    /// zero, insertion order gone. Verified against PHP 8.5's own
    /// `sort(["c" => 3, "a" => 1, "b" => 2])`.
    #[test]
    fn sorting_renumbers_by_default_and_keeps_keys_on_request() {
        let entries: [(&[u8], Value); 3] = [
            (b"c", Value::int(3)),
            (b"a", Value::int(1)),
            (b"b", Value::int(2)),
        ];
        assert_eq!(
            sorted(&entries, false, false),
            vec![b"0=1".to_vec(), b"1=2".to_vec(), b"2=3".to_vec()]
        );
        // PHP's `asort`.
        assert_eq!(
            sorted(&entries, false, true),
            vec![b"a=1".to_vec(), b"b=2".to_vec(), b"c=3".to_vec()]
        );
        // PHP's `arsort`.
        assert_eq!(
            sorted(&entries, true, true),
            vec![b"c=3".to_vec(), b"b=2".to_vec(), b"a=1".to_vec()]
        );
    }

    /// Two strings compare **bytewise**, never numerically — the one
    /// deliberate divergence from PHP's own `sort`, which answers
    /// `["9", "10"]` here because it reads two numeric strings as numbers.
    /// `mwl_core_arr_sort`'s own docs own the reasoning.
    #[test]
    fn two_strings_compare_bytewise_rather_than_numerically() {
        let entries: [(&[u8], Value); 2] = [
            (b"0", Value::str(MwlStr::new(b"9"))),
            (b"1", Value::str(MwlStr::new(b"10"))),
        ];
        assert_eq!(
            sorted(&entries, false, false),
            vec![b"0=10".to_vec(), b"1=9".to_vec()]
        );
    }

    /// The sort is stable: entries that compare equal come out in the order
    /// they went in, ascending **and** descending, because `Desc` reverses the
    /// comparison rather than the result. PHP 8's sorts are stable the same
    /// way — its own `rsort(["bb", "aa", "cc"])` compared by a constant leaves
    /// them untouched.
    #[test]
    fn equal_entries_keep_their_original_order_in_both_directions() {
        let entries: [(&[u8], Value); 4] = [
            (b"w", Value::int(7)),
            (b"x", Value::int(7)),
            (b"y", Value::int(7)),
            (b"z", Value::int(7)),
        ];
        for descending in [false, true] {
            assert_eq!(
                sorted(&entries, descending, true),
                vec![
                    b"w=7".to_vec(),
                    b"x=7".to_vec(),
                    b"y=7".to_vec(),
                    b"z=7".to_vec()
                ]
            );
        }
    }

    /// An empty array and a one-entry array both come back unchanged rather
    /// than reaching the merge at all — the two sizes a hand-written sort gets
    /// wrong first.
    #[test]
    fn an_empty_and_a_single_entry_array_sort_to_themselves() {
        assert_eq!(sorted(&[], false, true), Vec::<Vec<u8>>::new());
        assert_eq!(
            sorted(&[(b"k", Value::int(1))], true, true),
            vec![b"k=1".to_vec()]
        );
    }

    /// Every run length the bottom-up merge takes a different path through —
    /// an odd tail, a power of two, and one either side — against the same
    /// answer computed by Rust's own sort. This is the check a hand-written
    /// merge actually owes: the `copy_from_slice` that pairs two runs of
    /// unequal length is where it went wrong the first time.
    #[test]
    fn every_run_length_merges_to_the_same_answer_a_reference_sort_gives() {
        for len in 0..40i64 {
            // A shape with duplicates, a descending prefix and an ascending
            // tail, so no length is accidentally already sorted.
            let values: Vec<i64> = (0..len).map(|i| (len - i) % 7).collect();
            let entries: Vec<(Vec<u8>, Value)> = values
                .iter()
                .enumerate()
                .map(|(index, value)| (index.to_string().into_bytes(), Value::int(*value)))
                .collect();
            let borrowed: Vec<(&[u8], Value)> = entries
                .iter()
                .map(|(key, value)| (key.as_slice(), *value))
                .collect();

            let mut expected = values.clone();
            expected.sort_unstable();
            let expected: Vec<Vec<u8>> = expected
                .iter()
                .enumerate()
                .map(|(index, value)| format!("{index}={value}").into_bytes())
                .collect();
            assert_eq!(sorted(&borrowed, false, false), expected, "at length {len}");
        }
    }

    /// Two values with no natural order between them are `THROWN`, not a
    /// silent `Equal` — an array is the case that reaches this today, and an
    /// object is the one ADR 0013's `Comparable` is the eventual answer for.
    #[test]
    fn a_pair_with_no_natural_order_throws() {
        let mut array = MwlArray::new();
        array.set(MwlStr::new(b"0"), Value::int(1));
        array.set(MwlStr::new(b"1"), Value::array(MwlArray::new()));
        let subject = Value::array(array);
        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::mwl_core_arr_sort,
            &mut ctx,
            &[
                subject,
                Value::null(),
                Value::int(0),
                Value::null(),
                Value::bool(false),
            ],
        )
        .expect_err("an int and an array have no order");
        assert_eq!(status, mwl_runtime::THROWN);
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built above, and the \
                      helper borrowed rather than consumed it"
        )]
        unsafe {
            subject.release();
        }
    }

    /// An `order` argument that is not one of `Core\Order`'s two cases is a
    /// contained `FATAL`: the checker refuses an `int` there
    /// (`E_TYPE_MISMATCH`), so reaching this means the compiler let through a
    /// call it should not have.
    #[test]
    fn an_order_outside_the_enum_is_a_contained_fault() {
        let subject = Value::array(MwlArray::new());
        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::mwl_core_arr_sort,
            &mut ctx,
            &[
                subject,
                Value::null(),
                Value::int(7),
                Value::null(),
                Value::bool(false),
            ],
        )
        .expect_err("7 is not a Core\\Order case");
        assert_eq!(status, mwl_runtime::FATAL);
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built above, and the \
                      helper borrowed rather than consumed it"
        )]
        unsafe {
            subject.release();
        }
    }

    /// A wrong tag is a contained `FATAL`, not a panic that takes the process
    /// down — the check every helper's argument decoding owes.
    #[test]
    fn a_non_array_argument_is_a_contained_fault() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(super::mwl_core_arr_count, &mut ctx, &[Value::int(7)])
            .expect_err("an int is not an array");
        assert_eq!(status, mwl_runtime::FATAL);

        // `map` decodes its subject before it ever looks at the callback, so a
        // wrong subject is reported rather than reaching `call_closure` with a
        // value that is not a closure either.
        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::mwl_core_arr_map,
            &mut ctx,
            &[Value::int(7), Value::int(7)],
        )
        .expect_err("an int is not an array");
        assert_eq!(status, mwl_runtime::FATAL);
    }

    /// The deliberate divergence from `array_shift`/`array_pop`, which renumber
    /// the integer keys and keep the string ones: PHP answers `0,y` and `x,10`
    /// for this subject, and both members answer with the surviving keys
    /// untouched.
    #[test]
    fn dropping_an_end_entry_leaves_every_surviving_key_alone() {
        let mut ctx = Ctx::new(OutputSink::Sink);

        let tail = call(super::mwl_core_arr_without_first, &mut ctx, &[mixed_keys()])
            .expect("dropping the first entry never fails");
        assert_eq!(
            entries_of(tail),
            vec![
                (b"10".to_vec(), b"b".to_vec()),
                (b"y".to_vec(), b"c".to_vec()),
            ]
        );

        let head = call(super::mwl_core_arr_without_last, &mut ctx, &[mixed_keys()])
            .expect("dropping the last entry never fails");
        assert_eq!(
            entries_of(head),
            vec![
                (b"x".to_vec(), b"a".to_vec()),
                (b"10".to_vec(), b"b".to_vec()),
            ]
        );
    }

    /// Neither member is a special case at the ends: an empty array has nothing
    /// to drop and a one-entry array drops all it has, rather than PHP's
    /// `null`-and-a-warning for the element `array_shift` would have returned.
    #[test]
    fn dropping_from_an_empty_or_single_entry_array_yields_an_empty_one() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let mut single = MwlArray::new();
        single.set(MwlStr::new(b"only"), Value::str(MwlStr::new(b"a")));

        for member in [
            super::mwl_core_arr_without_first,
            super::mwl_core_arr_without_last,
        ] {
            for subject in [Value::array(MwlArray::new()), Value::array(single.clone())] {
                let result = call(member, &mut ctx, &[subject]).expect("dropping never fails");
                assert_eq!(entries_of(result), vec![]);
                #[expect(
                    unsafe_code,
                    reason = "this test owns the one reference it built above, \
                              and the helper borrowed rather than consumed it"
                )]
                unsafe {
                    subject.release();
                }
            }
        }
    }

    /// Verified against PHP 8.5's `array_combine`: pairing is by position, the
    /// keys array's own keys are discarded, and an `int` key arrives under its
    /// decimal spelling the way every other key position normalizes it.
    #[test]
    fn from_keys_and_values_pairs_by_position_and_normalizes_each_key() {
        let mut keys = MwlArray::new();
        keys.set(MwlStr::new(b"ignored"), Value::str(MwlStr::new(b"a")));
        keys.append(Value::int(10));
        let mut values = MwlArray::new();
        values.append(Value::str(MwlStr::new(b"first")));
        values.set(
            MwlStr::new(b"also ignored"),
            Value::str(MwlStr::new(b"second")),
        );

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::mwl_core_arr_from_keys_and_values,
            &mut ctx,
            &[Value::array(keys), Value::array(values)],
        )
        .expect("two arrays of the same length combine");
        assert_eq!(
            entries_of(result),
            vec![
                (b"a".to_vec(), b"first".to_vec()),
                (b"10".to_vec(), b"second".to_vec()),
            ]
        );
    }

    /// ADR 0063 R4: PHP 8 raises a `ValueError` here rather than pairing what
    /// it can, and silently dropping the excess would lose data.
    #[test]
    fn two_arrays_of_different_lengths_do_not_combine() {
        let mut keys = MwlArray::new();
        keys.append(Value::str(MwlStr::new(b"a")));
        let mut values = MwlArray::new();
        values.append(Value::int(1));
        values.append(Value::int(2));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::mwl_core_arr_from_keys_and_values,
            &mut ctx,
            &[Value::array(keys), Value::array(values)],
        )
        .expect_err("one key cannot carry two values");
        assert_eq!(status, mwl_runtime::THROWN);
    }

    /// The `(bucket, count)` pairs a `countBy` result holds, in cursor order —
    /// `entries_of`'s counterpart for the one member whose values are `uint`
    /// rather than strings.
    fn counts_of(result: Value) -> Vec<(Vec<u8>, u64)> {
        #[expect(
            unsafe_code,
            reason = "the helper returned one fresh reference, which the handle \
                      takes over and releases on drop"
        )]
        let array =
            unsafe { MwlArray::from_raw(result.array_ptr().expect("the member returns an array")) };
        let mut out = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let key = array.key_at(slot).expect("a live entry has a key");
            let count = array
                .value_at(slot)
                .and_then(|value| value.as_uint())
                .expect("every count is a uint");
            out.push((key.as_bytes().to_vec(), count));
            from = slot + 1;
        }
        out
    }

    /// Verified against PHP 8.5's `array_count_values`, first-occurrence order
    /// included. The `1`/`"1"` pair is the half PHP shares with every other key
    /// position and this member reaches through `key_bytes`.
    #[test]
    fn count_by_counts_each_bucket_in_first_occurrence_order() {
        let mut array = MwlArray::new();
        array.append(Value::str(MwlStr::new(b"red")));
        array.append(Value::int(1));
        array.append(Value::str(MwlStr::new(b"red")));
        array.append(Value::str(MwlStr::new(b"1")));
        array.append(Value::str(MwlStr::new(b"blue")));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::mwl_core_arr_count_by,
            &mut ctx,
            &[Value::array(array), Value::null()],
        )
        .expect("int and string values both name a bucket");
        assert_eq!(
            counts_of(result),
            vec![
                (b"red".to_vec(), 2),
                (b"1".to_vec(), 2),
                (b"blue".to_vec(), 1),
            ]
        );
    }

    /// A value that cannot name a bucket and has no `by` to name one for it is
    /// a throw, not PHP's warn-and-skip — `flip`'s treatment of the identical
    /// situation.
    #[test]
    fn counting_by_a_value_that_is_not_a_key_throws() {
        let mut array = MwlArray::new();
        array.append(Value::float(1.5));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::mwl_core_arr_count_by,
            &mut ctx,
            &[Value::array(array), Value::null()],
        )
        .expect_err("a float is not a key");
        assert_eq!(status, mwl_runtime::FATAL);
    }

    /// The deliberate divergence from `array_pad`, which renumbers the integer
    /// keys and keeps the string ones: PHP answers
    /// `{"0":"z","x":"a","1":"b","y":"c"}` for this subject padded on the left
    /// — the padding does not even end up contiguous — where this renumbers
    /// every key, on both sides alike.
    #[test]
    fn padding_renumbers_every_key_rather_than_only_the_integer_ones() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let pad = || Value::str(MwlStr::new(b"z"));

        let started = call(
            super::mwl_core_arr_pad_start,
            &mut ctx,
            &[mixed_keys(), Value::uint(4), pad()],
        )
        .expect("padding a three-entry array to four never fails");
        assert_eq!(
            entries_of(started),
            vec![
                (b"0".to_vec(), b"z".to_vec()),
                (b"1".to_vec(), b"a".to_vec()),
                (b"2".to_vec(), b"b".to_vec()),
                (b"3".to_vec(), b"c".to_vec()),
            ]
        );

        let ended = call(
            super::mwl_core_arr_pad_end,
            &mut ctx,
            &[mixed_keys(), Value::uint(4), pad()],
        )
        .expect("padding a three-entry array to four never fails");
        assert_eq!(
            entries_of(ended),
            vec![
                (b"0".to_vec(), b"a".to_vec()),
                (b"1".to_vec(), b"b".to_vec()),
                (b"2".to_vec(), b"c".to_vec()),
                (b"3".to_vec(), b"z".to_vec()),
            ]
        );
    }

    /// A subject already at or past the size adds nothing — PHP's own answer,
    /// except that the keys go here too, since this member has already said it
    /// does not keep them.
    #[test]
    fn a_subject_already_long_enough_gains_no_entries() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        for member in [super::mwl_core_arr_pad_start, super::mwl_core_arr_pad_end] {
            for size in [0u64, 3] {
                let result = call(
                    member,
                    &mut ctx,
                    &[
                        mixed_keys(),
                        Value::uint(size),
                        Value::str(MwlStr::new(b"z")),
                    ],
                )
                .expect("padding to a size already reached never fails");
                assert_eq!(
                    entries_of(result),
                    vec![
                        (b"0".to_vec(), b"a".to_vec()),
                        (b"1".to_vec(), b"b".to_vec()),
                        (b"2".to_vec(), b"c".to_vec()),
                    ]
                );
            }
        }
    }

    /// Verified against PHP 8.5's `array_fill(0, ...)`; the `$start_index`
    /// overload PHP also has is [`super::mwl_core_arr_fill_keys`]'s job here,
    /// and this member's own docs own why.
    #[test]
    fn fill_repeats_one_value_under_a_fresh_run_of_keys() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::mwl_core_arr_fill,
            &mut ctx,
            &[Value::uint(3), Value::str(MwlStr::new(b"v"))],
        )
        .expect("filling never fails");
        assert_eq!(
            entries_of(result),
            vec![
                (b"0".to_vec(), b"v".to_vec()),
                (b"1".to_vec(), b"v".to_vec()),
                (b"2".to_vec(), b"v".to_vec()),
            ]
        );

        let none = call(
            super::mwl_core_arr_fill,
            &mut ctx,
            &[Value::uint(0), Value::str(MwlStr::new(b"v"))],
        )
        .expect("a count of zero is an empty array, not a throw");
        assert_eq!(entries_of(none), vec![]);
    }

    /// Verified against PHP 8.5's `array_fill_keys`, duplicate collapse
    /// included: the keys array contributes its values, each normalized the
    /// way every other key position normalizes one.
    #[test]
    fn fill_keys_stores_one_value_under_each_distinct_key() {
        let mut keys = MwlArray::new();
        keys.set(MwlStr::new(b"ignored"), Value::str(MwlStr::new(b"a")));
        keys.append(Value::int(10));
        keys.append(Value::str(MwlStr::new(b"a")));
        keys.append(Value::str(MwlStr::new(b"10")));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::mwl_core_arr_fill_keys,
            &mut ctx,
            &[Value::array(keys), Value::str(MwlStr::new(b"v"))],
        )
        .expect("filling keys never fails");
        assert_eq!(
            entries_of(result),
            vec![
                (b"a".to_vec(), b"v".to_vec()),
                (b"10".to_vec(), b"v".to_vec()),
            ]
        );
    }

    /// A key that is neither an `int` nor a `string` is a throw rather than
    /// PHP's warn-and-skip — `flip`'s and `countBy`'s treatment of the
    /// identical situation.
    #[test]
    fn filling_under_a_key_that_is_not_a_key_throws() {
        let mut keys = MwlArray::new();
        keys.append(Value::float(1.5));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::mwl_core_arr_fill_keys,
            &mut ctx,
            &[Value::array(keys), Value::str(MwlStr::new(b"v"))],
        )
        .expect_err("a float is not a key");
        assert_eq!(status, mwl_runtime::FATAL);
    }

    /// The four ends of a map, over a subject whose insertion order is not its
    /// key order — so an implementation that read the hash rather than the
    /// insertion list would answer differently.
    #[test]
    fn the_four_end_members_read_insertion_order() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let asked = |ctx: &mut Ctx, helper: mwl_runtime::MwlFn| {
            rendered_value(call(helper, ctx, &[mixed_keys()]).expect("asking an end never fails"))
        };
        assert_eq!(asked(&mut ctx, super::mwl_core_arr_first), b"a");
        assert_eq!(asked(&mut ctx, super::mwl_core_arr_last), b"c");
        assert_eq!(asked(&mut ctx, super::mwl_core_arr_first_key), b"x");
        assert_eq!(asked(&mut ctx, super::mwl_core_arr_last_key), b"y");
    }

    /// An empty subject answers `null` at all four ends rather than throwing —
    /// ADR 0063 R5's absence spelling, not R4's failure.
    #[test]
    fn an_empty_array_has_no_ends() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        for helper in [
            super::mwl_core_arr_first as mwl_runtime::MwlFn,
            super::mwl_core_arr_last,
            super::mwl_core_arr_first_key,
            super::mwl_core_arr_last_key,
        ] {
            let answer = call(helper, &mut ctx, &[Value::array(MwlArray::new())])
                .expect("an empty array is not a failure");
            assert_eq!(answer.tag_byte(), mwl_runtime::Tag::Null as u8);
        }
    }

    /// Reading an end retains what it hands back: the answer outlives the
    /// subject, so the entry's own reference cannot be the one returned.
    #[test]
    fn an_end_value_is_a_reference_of_its_own() {
        let mut array = MwlArray::new();
        let held = MwlStr::new(b"only");
        let before = held.refcount();
        array.append(Value::str(held));
        let subject = Value::array(array);

        let mut ctx = Ctx::new(OutputSink::Sink);
        let answer =
            call(super::mwl_core_arr_first, &mut ctx, &[subject]).expect("a one-entry array");
        #[expect(
            unsafe_code,
            reason = "the answer is a fresh reference this test owns, and the \
                      subject is the one reference it built above; the handle \
                      below only reads a count, so it must not release one"
        )]
        unsafe {
            let handle =
                std::mem::ManuallyDrop::new(MwlStr::from_raw(answer.str_ptr().expect("a string")));
            assert_eq!(handle.refcount(), before + 1);
            answer.release();
            subject.release();
        }
    }

    /// `find`/`findKey` answer from the *first* match, and `any`/`all` answer
    /// an empty subject the two vacuous ways round. The predicate here is
    /// `Value::null()`, which `call_closure` rejects — so these go through the
    /// conformance suite instead, and what is checked here is the one shape a
    /// unit test can reach: a non-array subject.
    #[test]
    fn a_predicate_member_over_a_non_array_subject_is_a_contained_fault() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        for helper in [
            super::mwl_core_arr_find as mwl_runtime::MwlFn,
            super::mwl_core_arr_find_key,
            super::mwl_core_arr_any,
            super::mwl_core_arr_all,
        ] {
            let status = call(helper, &mut ctx, &[Value::int(1), Value::null()])
                .expect_err("an int is not an array");
            assert_eq!(status, mwl_runtime::FATAL);
        }
    }
}
