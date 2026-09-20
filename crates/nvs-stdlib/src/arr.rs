//! `Core\Arr` — [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
//! § 2, over `nvs_runtime`'s insertion-ordered, copy-on-write `array<T>`.
//!
//! Every member here is pure (`rule:core-api/shape-rules` R3) and borrows its subject rather
//! than consuming it — see [`crate`]'s own docs for why that falls out of
//! being a helper rather than being a rule this module states.
//!
//! # `array<T|U>` is written literally, not flattened to `array<mixed>`
//!
//! `rule:types/array-combination` writes all four combination members as `array<T|U>`, and the registry
//! can state exactly that: a union is legal in either direction
//! ([`crate::registry::CoreTy::Union`]), so the return type is
//! [`COMBINED`] rather than the `array<mixed>` that would be the safe
//! fallback. It is never worse and usually better — `overlay` over two
//! `array<string>`s answers `array<string>`, which a call site can index and
//! read without a cast, where `array<mixed>` would force one at every use.
//!
//! What that costs is a *shape* rule the spec's signature already implies and
//! this is the one place it is written down: `T` binds from the base and `U`
//! from the **first** trailing layer (`nvs_types::generics` binds
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
//! those three interact. The rule, spec § 2's *Ordering* paragraph: **`on`
//! selects the part of an entry that is compared — the value, the key, or
//! both — and `by` and `comparator` apply to the part it selected.** So a
//! `by` under `SetOn::Keys` replaces the *key*, not the
//! value; it is never an option that could not change the answer.
//! [`comparison_subject`] and [`set_member`] hold the mechanics and the cost.
//!
//! # A callback that does not want a key is never handed one
//!
//! [`nvs_core_arr_map`], [`nvs_core_arr_filter`] and [`nvs_core_arr_reduce`]
//! each call back into Novis code per entry, and § 2's "every callback receives
//! `($value, $key)` and may declare fewer parameters" means the key is
//! frequently built and then dropped by the trimming in
//! `nvs_runtime::call_closure`. On a list that is a rendered decimal and an
//! `NvsStr` allocation per entry, for nothing. So each of them reads
//! `nvs_runtime::closure_arity` **once before the loop** and builds the key
//! only where the callback declared a parameter to receive it.
//!
//! Rendering it *where it is wanted* is a rule and not a convenience: `rule:types/arrays` makes the `$key` a callback receives a `string` over every array shape,
//! so a packed list's `SlotKey::Index` becomes a decimal here even though
//! [`store_at`] just below would take the position as it stands. Handing the
//! position on unrendered is what that section refuses, since it would make a
//! callback's `$key` type depend on how the subject is stored. It is visible
//! now that `nvs_runtime::call_closure` checks each argument against the
//! parameter tags the closure declared: a callback writing `int $k` throws
//! `LogicError` at the call, on a list exactly as on a map, rather than
//! reading a string's payload as an integer.
//!
//! Preserving a key is not the same as rendering one, which is the other half:
//! [`store_at`] writes an entry back under `nvs_runtime::SlotKey`, the key in
//! whichever form the subject's own shape already holds it, so a `map` over a
//! list allocates no keys at all and a `filter` allocates them only where it
//! left a gap. `docs/perf/userland-gap.md` § D is the measurement, and
//! `nvs-runtime`'s `a_callback_that_does_not_want_a_key_synthesizes_none` is
//! the guard.
//!
//! [`nvs_core_arr_sort`] takes the same rule one step further, because it is
//! the one member that may not need the keys *at all*: with `preserveKeys`
//! false and no `by` closure declaring a second parameter, nothing downstream
//! can observe a key, so its walk collects none rather than collecting and
//! discarding them.
//!
//! # Known gaps
//!
//! 1. **A callback that reshapes the subject reshapes what the walk is
//!    reading.** Every member here borrows its subject ([`crate`]'s *Arguments
//!    are borrowed, never consumed*), so a subject reached through a static or
//!    instance property has no second reference while the member holds it: a
//!    predicate appending to that property appends under the cursor, and
//!    [`nvs_core_arr_filter`] over it walks the entries it is adding until the
//!    request's memory limit. PHP's `array_filter` walks its own copy and
//!    ends, and Novis's own `foreach` holds a reference for the loop's length
//!    so the body's write copies first. The fix is one retained reference for
//!    the length of the walk in every member that calls back into Novis code,
//!    which prices a refcount pair onto every one of those calls and is a
//!    decision about the helper convention rather than about one member.
//!    `tests/hostile/core/Arr/filter/01-predicates-written-to-break-the-walk.nvs`
//!    step 4 is the attack, and it passes: the ending is the memory ceiling
//!    rather than a crash.
//!    — owner: M11
//!
//! 2. **A walk builds its callback's key argument whether the callback takes
//!    one or not, and over a list that is an allocation per entry.**
//!    [`find_slot`] and its siblings pass `&[value, key]` to
//!    [`nvs_runtime::call_closure`] unconditionally, so a predicate declaring
//!    only `($value)` still pays for the key beside it. Over a string-keyed
//!    array the key is the stored [`NvsStr`] and cloning it is a refcount
//!    pair; over a list it is the position rendered to text, which allocates.
//!    `benches/members/core/Arr/find.nvs` counts 10 allocations per operation
//!    over its four-entry list, and the same program over a four-entry
//!    string-keyed array counts 4. The fix is reading the closure's declared
//!    arity off `CLOSURE_ARITY_SLOT` before the call, which is a decision
//!    about what every callback member passes rather than about one member.
//!    — owner: M12
//!
//! 3. **A key argument is copied onto the heap before it is looked up.**
//!    [`key_bytes`] answers a `Vec<u8>`, so `hasKey`, `get`, `set` and every
//!    other member reading a key allocates one buffer per call even when the
//!    argument is already a [`NvsStr`] whose bytes could be borrowed for the
//!    length of the lookup. `benches/members/core/Arr/hasKey.nvs` counts one
//!    allocation per operation, and the member neither walks nor calls back.
//!    The fix is a borrowing return type, which every caller's ownership has
//!    to be read against, and each of their ledger rows re-measured.
//!    — owner: M12
//!
//! 4. **A member that builds an array grows its storage as it appends, with
//!    the entry count already in hand.** [`NvsArray`] offers `new` and no way
//!    to reserve, so [`nvs_core_arr_keys`] over a three-entry subject counts
//!    four allocations for three keys that are clones of the stored ones, and
//!    the members that build a larger result — `filter` at 16, `mapKeys` at
//!    25, `groupBy` at 41 — carry the same growth inside their own figures.
//!    The fix is a capacity constructor on the runtime's array, which every
//!    building member is then read against one at a time.
//!    — owner: M12

use nvs_runtime::{Ctx, Decimal, Fault, NvsArray, NvsStr, SlotKey, Tag, ThrownClass, Value};

use crate::ordering::compare_values;
use crate::registry::{
    CaseDoc, Const, CoreClass, CoreEnum, CoreMethod, CoreOption, CoreTy, EnumDoc, ErrorDoc,
    MethodDoc, ParamDoc, Qual,
};

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
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_arr_count",
            doc: Some(&COUNT_DOC),
        },
        CoreMethod {
            name: "filter",
            names: &["a", "predicate"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::CallableSig(&[CoreTy::Var("T"), CoreTy::Str], &CoreTy::Bool),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_filter",
            doc: Some(&FILTER_DOC),
        },
        CoreMethod {
            name: "map",
            names: &["a", "fn"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::CallableSig(&[CoreTy::Var("T"), CoreTy::Str], &CoreTy::Var("U")),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("U")),
            symbol: "nvs_core_arr_map",
            doc: Some(&MAP_DOC),
        },
        CoreMethod {
            name: "mapKeys",
            names: &["a", "fn"],
            // The callback answers a *key*, and `rule:types/arrays` normalises
            // an integer one to its decimal at the subscript rather than
            // refusing it, so the return type is the same `int|string` an
            // `indexBy` option accepts — [`ARRAY_KEY_CONTAGIOUS`].
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::CallableSig(
                    &[CoreTy::Var("T"), CoreTy::Str],
                    &CoreTy::Union(ARRAY_KEY_CONTAGIOUS),
                ),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_map_keys",
            doc: Some(&MAP_KEYS_DOC),
        },
        CoreMethod {
            name: "groupBy",
            names: &["a", "key"],
            // The bucket name is a key, so it takes [`ARRAY_KEY_CONTAGIOUS`]
            // for the reason `mapKeys` does.
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::CallableSig(
                    &[CoreTy::Var("T"), CoreTy::Str],
                    &CoreTy::Union(ARRAY_KEY_CONTAGIOUS),
                ),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Array(&CoreTy::Var("T"))),
            symbol: "nvs_core_arr_group_by",
            doc: Some(&GROUP_BY_DOC),
        },
        CoreMethod {
            name: "reduce",
            names: &["a", "fn", "initial"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::CallableSig(
                    &[CoreTy::Var("U"), CoreTy::Var("T"), CoreTy::Str],
                    &CoreTy::Var("U"),
                ),
                CoreTy::Var("U"),
            ],
            defaults: &[],
            return_ty: CoreTy::Var("U"),
            symbol: "nvs_core_arr_reduce",
            doc: Some(&REDUCE_DOC),
        },
        CoreMethod {
            name: "find",
            names: &["a", "predicate"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::CallableSig(&[CoreTy::Var("T"), CoreTy::Str], &CoreTy::Bool),
            ],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_find",
            doc: Some(&FIND_DOC),
        },
        CoreMethod {
            name: "findKey",
            names: &["a", "predicate"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::CallableSig(&[CoreTy::Var("T"), CoreTy::Str], &CoreTy::Bool),
            ],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_arr_find_key",
            doc: Some(&FIND_KEY_DOC),
        },
        CoreMethod {
            name: "any",
            names: &["a", "predicate"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::CallableSig(&[CoreTy::Var("T"), CoreTy::Str], &CoreTy::Bool),
            ],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_arr_any",
            doc: Some(&ANY_DOC),
        },
        CoreMethod {
            name: "all",
            names: &["a", "predicate"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::CallableSig(&[CoreTy::Var("T"), CoreTy::Str], &CoreTy::Bool),
            ],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_arr_all",
            doc: Some(&ALL_DOC),
        },
        CoreMethod {
            name: "isEmpty",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_arr_is_empty",
            doc: Some(&IS_EMPTY_DOC),
        },
        CoreMethod {
            name: "hasKey",
            names: &["a", "key"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Union(ARRAY_KEY_NEUTRAL),
            ],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_arr_has_key",
            doc: Some(&HAS_KEY_DOC),
        },
        CoreMethod {
            name: "contains",
            names: &["haystack", "needle"],
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_arr_contains",
            doc: Some(&CONTAINS_DOC),
        },
        CoreMethod {
            name: "keyOf",
            names: &["haystack", "needle"],
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_arr_key_of",
            doc: Some(&KEY_OF_DOC),
        },
        CoreMethod {
            name: "isList",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_arr_is_list",
            doc: Some(&IS_LIST_DOC),
        },
        CoreMethod {
            name: "keys",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_arr_keys",
            doc: Some(&KEYS_DOC),
        },
        CoreMethod {
            name: "values",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_values",
            doc: Some(&VALUES_DOC),
        },
        CoreMethod {
            name: "first",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_first",
            doc: Some(&FIRST_DOC),
        },
        CoreMethod {
            name: "last",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_last",
            doc: Some(&LAST_DOC),
        },
        CoreMethod {
            name: "firstKey",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_arr_first_key",
            doc: Some(&FIRST_KEY_DOC),
        },
        CoreMethod {
            name: "lastKey",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Str),
            symbol: "nvs_core_arr_last_key",
            doc: Some(&LAST_KEY_DOC),
        },
        CoreMethod {
            name: "slice",
            names: &["a", "offset", "length"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Int,
                CoreTy::Nullable(&CoreTy::Int),
                CoreTy::Options(PRESERVE_KEYS),
            ],
            defaults: &[Const::Null],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_slice",
            doc: Some(&SLICE_DOC),
        },
        CoreMethod {
            name: "replaceRange",
            names: &["a", "offset", "length", "replacement"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Int,
                CoreTy::Nullable(&CoreTy::Int),
                CoreTy::Array(&CoreTy::Var("T")),
            ],
            defaults: &[Const::EmptyArray],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_replace_range",
            doc: Some(&REPLACE_RANGE_DOC),
        },
        CoreMethod {
            name: "chunk",
            names: &["a", "size"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Uint,
                CoreTy::Options(PRESERVE_KEYS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Array(&CoreTy::Var("T"))),
            symbol: "nvs_core_arr_chunk",
            doc: Some(&CHUNK_DOC),
        },
        CoreMethod {
            name: "append",
            names: &["a", "values"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Variadic(&CoreTy::Var("T")),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_append",
            doc: Some(&APPEND_DOC),
        },
        CoreMethod {
            name: "prepend",
            names: &["a", "values"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Variadic(&CoreTy::Var("T")),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_prepend",
            doc: Some(&PREPEND_DOC),
        },
        CoreMethod {
            name: "withoutFirst",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_without_first",
            doc: Some(&WITHOUT_FIRST_DOC),
        },
        CoreMethod {
            name: "withoutLast",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_without_last",
            doc: Some(&WITHOUT_LAST_DOC),
        },
        CoreMethod {
            name: "padStart",
            names: &["a", "size", "value"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Uint,
                CoreTy::Var("T"),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_pad_start",
            doc: Some(&PAD_START_DOC),
        },
        CoreMethod {
            name: "padEnd",
            names: &["a", "size", "value"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Uint,
                CoreTy::Var("T"),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_pad_end",
            doc: Some(&PAD_END_DOC),
        },
        CoreMethod {
            name: "reverse",
            names: &["a"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Options(PRESERVE_KEYS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_reverse",
            doc: Some(&REVERSE_DOC),
        },
        CoreMethod {
            name: "flip",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Union(ARRAY_KEY))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Str),
            symbol: "nvs_core_arr_flip",
            doc: Some(&FLIP_DOC),
        },
        CoreMethod {
            name: "flatten",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Array(&CoreTy::Var("T")))],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_flatten",
            doc: Some(&FLATTEN_DOC),
        },
        CoreMethod {
            name: "flattenDeep",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Mixed),
            symbol: "nvs_core_arr_flatten_deep",
            doc: Some(&FLATTEN_DEEP_DOC),
        },
        CoreMethod {
            name: "column",
            names: &["a", "column"],
            params: &[
                CoreTy::Array(&CoreTy::Array(&CoreTy::Var("T"))),
                CoreTy::Union(ARRAY_KEY_CONTAGIOUS),
                CoreTy::Options(COLUMN_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_column",
            doc: Some(&COLUMN_DOC),
        },
        CoreMethod {
            name: "sort",
            names: &["a"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Options(SORT_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_sort",
            doc: Some(&SORT_DOC),
        },
        CoreMethod {
            name: "sortByKey",
            names: &["a"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Options(SORT_BY_KEY_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_sort_by_key",
            doc: Some(&SORT_BY_KEY_DOC),
        },
        CoreMethod {
            name: "fill",
            names: &["count", "value"],
            params: &[CoreTy::Uint, CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_fill",
            doc: Some(&FILL_DOC),
        },
        CoreMethod {
            name: "fillKeys",
            names: &["keys", "value"],
            params: &[CoreTy::Array(&CoreTy::Union(ARRAY_KEY)), CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_fill_keys",
            doc: Some(&FILL_KEYS_DOC),
        },
        CoreMethod {
            name: "range",
            names: &["start", "end"],
            params: &[CoreTy::Int, CoreTy::Int, CoreTy::Options(RANGE_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Int),
            symbol: "nvs_core_arr_range",
            doc: Some(&RANGE_DOC),
        },
        CoreMethod {
            name: "fromKeysAndValues",
            names: &["keys", "values"],
            params: &[
                CoreTy::Array(&CoreTy::Union(ARRAY_KEY)),
                CoreTy::Array(&CoreTy::Var("T")),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_from_keys_and_values",
            doc: Some(&FROM_KEYS_AND_VALUES_DOC),
        },
        CoreMethod {
            name: "from",
            names: &["items"],
            params: &[
                CoreTy::Iterated(&CoreTy::Var("T")),
                CoreTy::Options(FROM_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_from",
            doc: Some(&FROM_DOC),
        },
        CoreMethod {
            name: "overlay",
            names: &["base", "layers"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Variadic(&CoreTy::Array(&CoreTy::Var("U"))),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Union(COMBINED)),
            symbol: "nvs_core_arr_overlay",
            doc: Some(&OVERLAY_DOC),
        },
        CoreMethod {
            name: "overlayDeep",
            names: &["base", "layers"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Variadic(&CoreTy::Array(&CoreTy::Var("U"))),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Union(COMBINED)),
            symbol: "nvs_core_arr_overlay_deep",
            doc: Some(&OVERLAY_DEEP_DOC),
        },
        CoreMethod {
            name: "underlay",
            names: &["base", "layers"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Variadic(&CoreTy::Array(&CoreTy::Var("U"))),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Union(COMBINED)),
            symbol: "nvs_core_arr_underlay",
            doc: Some(&UNDERLAY_DOC),
        },
        CoreMethod {
            name: "appendAll",
            names: &["a", "others"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Variadic(&CoreTy::Array(&CoreTy::Var("U"))),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Union(COMBINED)),
            symbol: "nvs_core_arr_append_all",
            doc: Some(&APPEND_ALL_DOC),
        },
        CoreMethod {
            name: "diff",
            names: &["a", "b"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Options(SET_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_diff",
            doc: Some(&DIFF_DOC),
        },
        CoreMethod {
            name: "intersect",
            names: &["a", "b"],
            params: &[
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Array(&CoreTy::Var("T")),
                CoreTy::Options(SET_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_intersect",
            doc: Some(&INTERSECT_DOC),
        },
        CoreMethod {
            name: "countBy",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Options(BY_OPTION)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Uint),
            symbol: "nvs_core_arr_count_by",
            doc: Some(&COUNT_BY_DOC),
        },
        CoreMethod {
            name: "unique",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T")), CoreTy::Options(BY_OPTION)],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_unique",
            doc: Some(&UNIQUE_DOC),
        },
        CoreMethod {
            name: "min",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_min",
            doc: Some(&MIN_DOC),
        },
        CoreMethod {
            name: "max",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Var("T"))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("T")),
            symbol: "nvs_core_arr_max",
            doc: Some(&MAX_DOC),
        },
        CoreMethod {
            name: "sum",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Union(crate::math::NUMBER))],
            defaults: &[],
            return_ty: CoreTy::Union(crate::math::NUMBER),
            symbol: "nvs_core_arr_sum",
            doc: Some(&SUM_DOC),
        },
        CoreMethod {
            name: "product",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Union(crate::math::NUMBER))],
            defaults: &[],
            return_ty: CoreTy::Union(crate::math::NUMBER),
            symbol: "nvs_core_arr_product",
            doc: Some(&PRODUCT_DOC),
        },
        CoreMethod {
            name: "average",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Union(crate::math::NUMBER))],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Union(QUOTIENT)),
            symbol: "nvs_core_arr_average",
            doc: Some(&AVERAGE_DOC),
        },
        // Last rather than in the spec's order because the spec has no row for
        // it yet — `docs/agent/loop-goal.md` § *Standing decisions* owes §§ 6
        // and 15 that entry, and this comment goes when it lands.
        CoreMethod {
            name: "shapeAs",
            names: &["a"],
            params: &[CoreTy::Array(&CoreTy::Mixed)],
            defaults: &[],
            return_ty: CoreTy::Written("T"),
            symbol: "nvs_core_arr_shape_as",
            doc: Some(&SHAPE_AS_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// `Core\Arr::count`'s reference card — `rule:core-api/reference-card`.
const COUNT_DOC: MethodDoc = MethodDoc {
    short: "Counts the entries in `$a`, as `count` and `sizeof` do.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to count.",
        shape: &[],
    }],
    ret: "The number of entries; `0` for the empty array.",
    errors: &[],
};

/// `Core\Arr::filter`'s reference card — `rule:core-api/reference-card`.
const FILTER_DOC: MethodDoc = MethodDoc {
    short: "Keeps the entries `$predicate` answers truthily for, as `array_filter` does — with \
            both of its flags folded in, since the callback receives `($value, $key)` and may \
            declare fewer parameters.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to filter.",
            shape: &[],
        },
        ParamDoc {
            name: "predicate",
            desc: "Called as `($value, $key)` per entry; a truthy answer keeps the entry, on \
                   the same truthiness table `if` reads.",
            shape: &[],
        },
    ],
    ret: "The kept entries under their own keys, never renumbered; an empty array when nothing \
          passes.",
    errors: &[],
};

/// `Core\Arr::map`'s reference card — `rule:core-api/reference-card`.
const MAP_DOC: MethodDoc = MethodDoc {
    short: "Replaces every value with what `$fn` answers for it, as `array_map` does over one \
            array; keys are preserved.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to map.",
            shape: &[],
        },
        ParamDoc {
            name: "fn",
            desc: "Called as `($value, $key)` per entry, and may declare fewer parameters; its \
                   return type is the result's `U`.",
            shape: &[],
        },
    ],
    ret: "One entry per entry of `$a`, under the same key, holding the callback's answer.",
    errors: &[],
};

/// `Core\Arr::mapKeys`'s reference card — `rule:core-api/reference-card`.
const MAP_KEYS_DOC: MethodDoc = MethodDoc {
    short: "Stores every value under the key `$fn` answers for it — the `keyBy` idiom, and \
            `array_combine(array_map(…), …)` as one member.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to re-key.",
            shape: &[],
        },
        ParamDoc {
            name: "fn",
            desc: "Called as `($value, $key)` per entry, and may declare fewer parameters; it \
                   answers the entry's new key, an `int` or a `string`.",
            shape: &[],
        },
    ],
    ret: "The values under their new keys, in first-occurrence order; two entries given one key \
          collapse to the last, and `1` and `\"1\"` are one key.",
    errors: &[],
};

/// `Core\Arr::groupBy`'s reference card — `rule:core-api/reference-card`.
const GROUP_BY_DOC: MethodDoc = MethodDoc {
    short: "Partitions the entries into buckets named by what `$key` answers for each — the \
            group-by loop PHP has no function for.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to partition.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "Called as `($value, $key)` per entry, and may declare fewer parameters; it \
                   answers the bucket's name, an `int` or a `string`.",
            shape: &[],
        },
    ],
    ret: "One inner array per distinct answer, in first-occurrence order, each keeping its entries \
          under their own keys; `1` and `\"1\"` name one bucket.",
    errors: &[],
};

/// `Core\Arr::reduce`'s reference card — `rule:core-api/reference-card`.
const REDUCE_DOC: MethodDoc = MethodDoc {
    short: "Folds the entries into one value from `$initial`, as `array_reduce` does; the \
            callback receives `($carry, $value, $key)` and may declare fewer parameters.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to fold.",
            shape: &[],
        },
        ParamDoc {
            name: "fn",
            desc: "Called as `($carry, $value, $key)` per entry, answering the next carry.",
            shape: &[],
        },
        ParamDoc {
            name: "initial",
            desc: "The seed, whose type is the fold's `U` and therefore the result's.",
            shape: &[],
        },
    ],
    ret: "The last carry; `$initial` itself over an empty array, with no call made.",
    errors: &[],
};

/// `Core\Arr::find`'s reference card — `rule:core-api/reference-card`.
const FIND_DOC: MethodDoc = MethodDoc {
    short: "The first value `$predicate` answers truthily for, as `array_find` does; the walk \
            stops at the match.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to search.",
            shape: &[],
        },
        ParamDoc {
            name: "predicate",
            desc: "Called as `($value, $key)` per entry, and may declare fewer parameters.",
            shape: &[],
        },
    ],
    ret: "The matching value, or `null` when nothing matches — which a stored `null` in an \
          `array<?T>` cannot be told from.",
    errors: &[],
};

/// `Core\Arr::findKey`'s reference card — `rule:core-api/reference-card`.
const FIND_KEY_DOC: MethodDoc = MethodDoc {
    short: "The key of the first entry `$predicate` answers truthily for, as `array_find_key` \
            does; the walk stops at the match.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to search.",
            shape: &[],
        },
        ParamDoc {
            name: "predicate",
            desc: "Called as `($value, $key)` per entry, and may declare fewer parameters.",
            shape: &[],
        },
    ],
    ret: "The key as a `string`, whatever it was written as; `null` when nothing matches.",
    errors: &[],
};

/// `Core\Arr::any`'s reference card — `rule:core-api/reference-card`.
const ANY_DOC: MethodDoc = MethodDoc {
    short: "Whether at least one entry satisfies `$predicate`, as `array_any` does; the walk \
            stops at the first match.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to test.",
            shape: &[],
        },
        ParamDoc {
            name: "predicate",
            desc: "Called as `($value, $key)` per entry, and may declare fewer parameters.",
            shape: &[],
        },
    ],
    ret: "`true` at the first truthy answer; `false` over the empty array.",
    errors: &[],
};

/// `Core\Arr::all`'s reference card — `rule:core-api/reference-card`.
const ALL_DOC: MethodDoc = MethodDoc {
    short: "Whether every entry satisfies `$predicate`, as `array_all` does; the walk stops at \
            the first failure.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to test.",
            shape: &[],
        },
        ParamDoc {
            name: "predicate",
            desc: "Called as `($value, $key)` per entry, and may declare fewer parameters.",
            shape: &[],
        },
    ],
    ret: "`false` at the first falsy answer; `true` over the empty array.",
    errors: &[],
};

/// `Core\Arr::isEmpty`'s reference card — `rule:core-api/reference-card`.
const IS_EMPTY_DOC: MethodDoc = MethodDoc {
    short: "Whether `$a` holds no entries — `empty($a)` and `count($a) === 0` as one member.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to test.",
        shape: &[],
    }],
    ret: "`true` for the empty array, `false` otherwise.",
    errors: &[],
};

/// `Core\Arr::hasKey`'s reference card — `rule:core-api/reference-card`.
const HAS_KEY_DOC: MethodDoc = MethodDoc {
    short: "Whether `$a` holds an entry under `$key`, as `array_key_exists` does — and as \
            `isset($a[$k])` does, since a stored `null` still counts.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to look in.",
            shape: &[],
        },
        ParamDoc {
            name: "key",
            desc: "The key, an `int` or a `string`; `1` and `\"1\"` name one entry.",
            shape: &[],
        },
    ],
    ret: "`true` when the key is present, whatever value it holds.",
    errors: &[],
};

/// `Core\Arr::contains`'s reference card — `rule:core-api/reference-card`.
const CONTAINS_DOC: MethodDoc = MethodDoc {
    short: "Whether some entry is `$needle` under strict identity, as `in_array` with `strict: \
            true` does — there is no loose form.",
    params: &[
        ParamDoc {
            name: "haystack",
            desc: "The array to search.",
            shape: &[],
        },
        ParamDoc {
            name: "needle",
            desc: "The value to look for, compared by identity; `int`, `uint`, `float` and \
                   `decimal` are one numeric domain, so `1` finds `1.0`.",
            shape: &[],
        },
    ],
    ret: "`true` at the first identical entry; `false` over the empty array.",
    errors: &[],
};

/// `Core\Arr::keyOf`'s reference card — `rule:core-api/reference-card`.
const KEY_OF_DOC: MethodDoc = MethodDoc {
    short: "The key of the first entry that is `$needle` under strict identity, as `array_search` \
            with `strict: true` does.",
    params: &[
        ParamDoc {
            name: "haystack",
            desc: "The array to search.",
            shape: &[],
        },
        ParamDoc {
            name: "needle",
            desc: "The value to look for, compared as `contains` compares.",
            shape: &[],
        },
    ],
    ret: "The key as a `string`, or `null` when no entry is identical — never `false`.",
    errors: &[],
};

/// `Core\Arr::isList`'s reference card — `rule:core-api/reference-card`.
const IS_LIST_DOC: MethodDoc = MethodDoc {
    short: "Whether the keys are exactly `\"0\", \"1\", …` in that order, as `array_is_list` \
            does.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to test.",
        shape: &[],
    }],
    ret: "`true` for a list and for the empty array; `false` once any key is out of sequence or \
          is not its index's exact decimal spelling.",
    errors: &[],
};

/// `Core\Arr::keys`'s reference card — `rule:core-api/reference-card`.
const KEYS_DOC: MethodDoc = MethodDoc {
    short: "Every key in insertion order, as `array_keys` does without its search argument.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array whose keys are wanted.",
        shape: &[],
    }],
    ret: "A list of the keys as `string`s — `\"10\"` stays `\"10\"`, never an `int`; the empty \
          array for an empty subject.",
    errors: &[],
};

/// `Core\Arr::values`'s reference card — `rule:core-api/reference-card`.
const VALUES_DOC: MethodDoc = MethodDoc {
    short: "Every value in insertion order under fresh keys, as `array_values` does.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array whose values are wanted.",
        shape: &[],
    }],
    ret: "A list renumbered from `\"0\"`.",
    errors: &[],
};

/// `Core\Arr::first`'s reference card — `rule:core-api/reference-card`.
const FIRST_DOC: MethodDoc = MethodDoc {
    short: "The first entry's value, as `reset`, `current` and `$a[array_key_first($a)]` answer \
            it — with no internal pointer to move.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to read.",
        shape: &[],
    }],
    ret: "The value, or `null` for the empty array — which a stored `null` in an `array<?T>` \
          cannot be told from.",
    errors: &[],
};

/// `Core\Arr::last`'s reference card — `rule:core-api/reference-card`.
const LAST_DOC: MethodDoc = MethodDoc {
    short: "The last entry's value, as `end` and `$a[array_key_last($a)]` answer it.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to read.",
        shape: &[],
    }],
    ret: "The value, or `null` for the empty array — which a stored `null` in an `array<?T>` \
          cannot be told from.",
    errors: &[],
};

/// `Core\Arr::firstKey`'s reference card — `rule:core-api/reference-card`.
const FIRST_KEY_DOC: MethodDoc = MethodDoc {
    short: "The first entry's key, as `array_key_first` and `key` answer it.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to read.",
        shape: &[],
    }],
    ret: "The key as a `string`, or `null` for the empty array.",
    errors: &[],
};

/// `Core\Arr::lastKey`'s reference card — `rule:core-api/reference-card`.
const LAST_KEY_DOC: MethodDoc = MethodDoc {
    short: "The last entry's key, as `array_key_last` answers it.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to read.",
        shape: &[],
    }],
    ret: "The key as a `string`, or `null` for the empty array.",
    errors: &[],
};

/// `Core\Arr::slice`'s reference card — `rule:core-api/reference-card`.
const SLICE_DOC: MethodDoc = MethodDoc {
    short: "The entries in one window of positions, as `array_slice` does; a position is \
            ordinal, whatever the keys are.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to take from.",
            shape: &[],
        },
        ParamDoc {
            name: "offset",
            desc: "Where the window opens; a negative offset counts back from the last entry, \
                   and one reaching past the first clamps to it.",
            shape: &[],
        },
        ParamDoc {
            name: "length",
            desc: "How many entries the window holds; a negative length stops that many short \
                   of the end, and `null` runs to the end.",
            shape: &[],
        },
        ParamDoc {
            name: "preserveKeys",
            desc: "Keep every entry's own key; the default discards every key, string keys \
                   included, and renumbers from `\"0\"`.",
            shape: &[],
        },
    ],
    ret: "The window's entries; the empty array when the window closes before it opens.",
    errors: &[],
};

/// `Core\Arr::replaceRange`'s reference card — `rule:core-api/reference-card`.
const REPLACE_RANGE_DOC: MethodDoc = MethodDoc {
    short: "The array with one window of positions replaced by `$replacement`'s values — what \
            `array_splice` returns, never what it does by reference.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to edit.",
            shape: &[],
        },
        ParamDoc {
            name: "offset",
            desc: "Where the window opens, on `slice`'s sign rule; an offset at the end \
                   appends.",
            shape: &[],
        },
        ParamDoc {
            name: "length",
            desc: "How many entries the window holds, on `slice`'s sign rule; `0` inserts at \
                   the position and `null` reaches the end.",
            shape: &[],
        },
        ParamDoc {
            name: "replacement",
            desc: "The values to put in the window's place; omitted, the window is simply \
                   removed.",
            shape: &[],
        },
    ],
    ret: "A list renumbered from `\"0\"`, kept entries and replacement alike.",
    errors: &[],
};

/// `Core\Arr::chunk`'s reference card — `rule:core-api/reference-card`.
const CHUNK_DOC: MethodDoc = MethodDoc {
    short: "The entries in consecutive runs of `$size`, as `array_chunk` does; the last run is \
            short when the count does not divide.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to split.",
            shape: &[],
        },
        ParamDoc {
            name: "size",
            desc: "How many entries each run holds; at least `1`.",
            shape: &[],
        },
        ParamDoc {
            name: "preserveKeys",
            desc: "Keep each entry's own key inside its run; the default renumbers every run \
                   from `\"0\"`.",
            shape: &[],
        },
    ],
    ret: "A list of runs; no runs at all for the empty array.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$size` is `0`.",
    }],
};

/// `Core\Arr::append`'s reference card — `rule:core-api/reference-card`.
const APPEND_DOC: MethodDoc = MethodDoc {
    short: "The array with every trailing value added after its last entry, as `array_push` does \
            — `$a[] = $v` in expression position.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to extend.",
            shape: &[],
        },
        ParamDoc {
            name: "values",
            desc: "The values to add, in written order.",
            shape: &[],
        },
    ],
    ret: "The subject's entries under their own keys, then each value under the next free \
          integer key — one past the largest present; the subject itself when no values are \
          given.",
    errors: &[],
};

/// `Core\Arr::prepend`'s reference card — `rule:core-api/reference-card`.
const PREPEND_DOC: MethodDoc = MethodDoc {
    short: "The trailing values, then the array's values, as `array_unshift` does.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to put them in front of.",
            shape: &[],
        },
        ParamDoc {
            name: "values",
            desc: "The values to put in front, in written order.",
            shape: &[],
        },
    ],
    ret: "A list renumbered from `\"0\"` — every key is discarded, since a value put in front \
          has no key of its own to take.",
    errors: &[],
};

/// `Core\Arr::withoutFirst`'s reference card — `rule:core-api/reference-card`.
const WITHOUT_FIRST_DOC: MethodDoc = MethodDoc {
    short: "Every entry but the first — what `array_shift` leaves behind, with `first` answering \
            the element it removes.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to shorten.",
        shape: &[],
    }],
    ret: "The remaining entries under their own keys, never renumbered; the empty array for a \
          subject of one entry or none.",
    errors: &[],
};

/// `Core\Arr::withoutLast`'s reference card — `rule:core-api/reference-card`.
const WITHOUT_LAST_DOC: MethodDoc = MethodDoc {
    short: "Every entry but the last — what `array_pop` leaves behind, with `last` answering the \
            element it removes.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to shorten.",
        shape: &[],
    }],
    ret: "The remaining entries under their own keys, never renumbered; the empty array for a \
          subject of one entry or none.",
    errors: &[],
};

/// `Core\Arr::padStart`'s reference card — `rule:core-api/reference-card`.
const PAD_START_DOC: MethodDoc = MethodDoc {
    short: "Copies of `$value` in front until the array holds `$size` entries, as `array_pad` \
            with a negative length does.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to pad.",
            shape: &[],
        },
        ParamDoc {
            name: "size",
            desc: "The length to reach; a subject already that long is not padded.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The value every added entry holds.",
            shape: &[],
        },
    ],
    ret: "A list renumbered from `\"0\"` — every key is discarded, padded or not.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The padded result is larger than any array this process could hold.",
    }],
};

/// `Core\Arr::padEnd`'s reference card — `rule:core-api/reference-card`.
const PAD_END_DOC: MethodDoc = MethodDoc {
    short: "Copies of `$value` after the last entry until the array holds `$size` entries, as \
            `array_pad` with a positive length does.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to pad.",
            shape: &[],
        },
        ParamDoc {
            name: "size",
            desc: "The length to reach; a subject already that long is not padded.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The value every added entry holds.",
            shape: &[],
        },
    ],
    ret: "A list renumbered from `\"0\"` — every key is discarded, padded or not.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The padded result is larger than any array this process could hold.",
    }],
};

/// `Core\Arr::reverse`'s reference card — `rule:core-api/reference-card`.
const REVERSE_DOC: MethodDoc = MethodDoc {
    short: "The entries in the opposite order, as `array_reverse` does.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to reverse.",
            shape: &[],
        },
        ParamDoc {
            name: "preserveKeys",
            desc: "Keep every entry's own key; the default discards every key, string keys \
                   included, and renumbers from `\"0\"`.",
            shape: &[],
        },
    ],
    ret: "The same entries, last first.",
    errors: &[],
};

/// `Core\Arr::flip`'s reference card — `rule:core-api/reference-card`.
const FLIP_DOC: MethodDoc = MethodDoc {
    short: "Every value made a key and every key made its value, as `array_flip` does.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to flip; every value is an `int` or a `string`, and `1` and `\"1\"` \
               flip to one key.",
        shape: &[],
    }],
    ret: "The old keys as `string` values under the old values as keys, in first-occurrence \
          order; a repeated value keeps its first position and takes its last key.",
    errors: &[],
};

/// `Core\Arr::flatten`'s reference card — `rule:core-api/reference-card`.
const FLATTEN_DOC: MethodDoc = MethodDoc {
    short: "One level of nesting removed — every inner array's values in order, as a hand-written \
            walk does.",
    params: &[ParamDoc {
        name: "a",
        desc: "An array of arrays.",
        shape: &[],
    }],
    ret: "A list renumbered from `\"0\"`; the inner keys are discarded, since two inner arrays \
          can share one.",
    errors: &[],
};

/// `Core\Arr::flattenDeep`'s reference card — `rule:core-api/reference-card`.
const FLATTEN_DEEP_DOC: MethodDoc = MethodDoc {
    short: "Every level of nesting removed, as a recursive walk or `iterator_to_array` over a \
            `RecursiveIteratorIterator` does; there is no depth count, since `flatten` is the \
            other real case.",
    params: &[ParamDoc {
        name: "a",
        desc: "An array nested to any depth.",
        shape: &[],
    }],
    ret: "A list of every non-array leaf in walk order, renumbered from `\"0\"`; `mixed` \
          because the depth is the caller's data, not the type's.",
    errors: &[],
};

/// `Core\Arr::column`'s reference card — `rule:core-api/reference-card`.
const COLUMN_DOC: MethodDoc = MethodDoc {
    short: "One named cell out of every row, as `array_column` does.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The rows, each an array.",
            shape: &[],
        },
        ParamDoc {
            name: "column",
            desc: "The key of the cell to take; `0` and `\"0\"` name one cell, as `$row[0]` \
                   and `$row[\"0\"]` do.",
            shape: &[],
        },
        ParamDoc {
            name: "indexBy",
            desc: "The key of the cell to key the result by; the default gives the result fresh \
                   integer keys.",
            shape: &[],
        },
    ],
    ret: "The cells in row order: a row without the column is skipped, a row without the \
          `indexBy` cell lands under the next free integer key, and two rows with one `indexBy` \
          value collapse to the last.",
    errors: &[],
};

/// `Core\Arr::sort`'s reference card — `rule:core-api/reference-card`.
const SORT_DOC: MethodDoc = MethodDoc {
    short: "The entries in order, stably — `sort`, `rsort`, `asort`, `arsort`, `usort`, \
            `uasort`, `natsort`, `natcasesort` and `array_multisort` as one member and four \
            options.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to sort.",
            shape: &[],
        },
        ParamDoc {
            name: "by",
            desc: "Called once per entry as `($value, $key)`, and may declare fewer parameters; \
                   what it answers is compared in the entry's place.",
            shape: &[],
        },
        ParamDoc {
            name: "order",
            desc: "`Order::Asc` (the default) or `Order::Desc`, which reverses the comparison \
                   rather than the result, so equal entries keep their relative order either \
                   way.",
            shape: &[],
        },
        ParamDoc {
            name: "comparator",
            desc: "Called as `($a, $b)` on two values — or on two `by` answers — and answering a \
                   negative, zero or positive number, as `usort`'s callback does; it replaces \
                   the natural order.",
            shape: &[],
        },
        ParamDoc {
            name: "preserveKeys",
            desc: "Keep each entry's own key, as `asort` does; the default renumbers from \
                   `\"0\"`, as `sort` does.",
            shape: &[],
        },
    ],
    ret: "The sorted entries. The natural order is between two numbers, two strings bytewise, \
          two bools or two nulls — `\"10\"` sorts before `\"9\"` — and nothing else.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "Two entries have no natural order between them — an `int` against a `string`, \
               or any object — and no `comparator` was given; or the comparator answered \
               `NaN`.",
    }],
};

/// `Core\Arr::sortByKey`'s reference card — `rule:core-api/reference-card`.
const SORT_BY_KEY_DOC: MethodDoc = MethodDoc {
    short: "The entries in key order, stably, every key kept — `ksort`, `krsort` and `uksort` as \
            one member.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to sort.",
            shape: &[],
        },
        ParamDoc {
            name: "order",
            desc: "`Order::Asc` (the default) or `Order::Desc`, which reverses the comparison \
                   rather than the result, so equal keys keep their insertion order either way.",
            shape: &[],
        },
        ParamDoc {
            name: "comparator",
            desc: "Called as `($a, $b)` on two keys, both `string`s, and answering a negative, \
                   zero or positive number, as `uksort`'s callback does; it replaces the \
                   natural order.",
            shape: &[],
        },
    ],
    ret: "The same entries under the same keys, ordered by key; the natural order is bytewise, \
          so `\"10\"` precedes `\"9\"`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The comparator answered `NaN`.",
    }],
};

/// `Core\Arr::fill`'s reference card — `rule:core-api/reference-card`.
const FILL_DOC: MethodDoc = MethodDoc {
    short: "`$count` copies of `$value` under `\"0\", \"1\", …`, as `array_fill` does from index \
            zero; there is no start index, since `fillKeys` takes the keys a caller wants.",
    params: &[
        ParamDoc {
            name: "count",
            desc: "How many entries; `0` yields the empty array.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The value every entry holds.",
            shape: &[],
        },
    ],
    ret: "A list of `$count` entries.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`$count` names a result larger than any array this process could hold.",
    }],
};

/// `Core\Arr::fillKeys`'s reference card — `rule:core-api/reference-card`.
const FILL_KEYS_DOC: MethodDoc = MethodDoc {
    short: "`$value` stored under every key in `$keys`, as `array_fill_keys` does.",
    params: &[
        ParamDoc {
            name: "keys",
            desc: "The keys to store under, as the array's *values* — each an `int` or a \
                   `string`; its own keys are ignored.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The value every entry holds.",
            shape: &[],
        },
    ],
    ret: "One entry per distinct key, in first-occurrence order; `1` and `\"1\"` are one key.",
    errors: &[],
};

/// `Core\Arr::range`'s reference card — `rule:core-api/reference-card`.
const RANGE_DOC: MethodDoc = MethodDoc {
    short: "The integers from `$start` to `$end` inclusive, as `range` does; the direction is \
            the bounds', so `$start > $end` counts down.",
    params: &[
        ParamDoc {
            name: "start",
            desc: "The first value.",
            shape: &[],
        },
        ParamDoc {
            name: "end",
            desc: "The last value that may appear; a step that overshoots stops at the last \
                   in-range value.",
            shape: &[],
        },
        ParamDoc {
            name: "step",
            desc: "The distance between neighbours, `1` by default; always positive, since the \
                   direction comes from the bounds.",
            shape: &[],
        },
    ],
    ret: "A list from `$start` towards `$end`; one entry when the two are equal.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "`step` is `0` or negative.",
    }],
};

/// `Core\Arr::fromKeysAndValues`'s reference card — `rule:core-api/reference-card`.
const FROM_KEYS_AND_VALUES_DOC: MethodDoc = MethodDoc {
    short: "`$values`' entries stored under `$keys`' entries, paired in order, as \
            `array_combine` does.",
    params: &[
        ParamDoc {
            name: "keys",
            desc: "The keys, as the array's *values* — each an `int` or a `string`; its own \
                   keys are ignored.",
            shape: &[],
        },
        ParamDoc {
            name: "values",
            desc: "The values, in the same order; its own keys are ignored.",
            shape: &[],
        },
    ],
    ret: "One entry per pair, in `$keys`' order; a repeated key keeps its first position and \
          takes its last value, and `1` and `\"1\"` are one key.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The two arrays hold different numbers of entries.",
    }],
};

/// `Core\Arr::from`'s reference card — `rule:core-api/reference-card`.
const FROM_DOC: MethodDoc = MethodDoc {
    short: "A sequence materialised as an array, as `iterator_to_array` does with \
            `preserve_keys: false` — an `Iterable<T>`, an `Iterator<T>` or an array, whatever \
            `foreach` would take.",
    params: &[
        ParamDoc {
            name: "items",
            desc: "The sequence to drain, once.",
            shape: &[],
        },
        ParamDoc {
            name: "limit",
            desc: "Stop the drive after this many elements — the only guard against an unbounded \
                   generator; the default takes everything, and `0` takes nothing.",
            shape: &[],
        },
    ],
    ret: "A list renumbered from `\"0\"` — a cursor has no keys, and an array's are discarded.",
    errors: &[],
};

/// `Core\Arr::overlay`'s reference card — `rule:core-api/reference-card`.
const OVERLAY_DOC: MethodDoc = MethodDoc {
    short: "`$base` with every layer's entries written over it, the right-hand value winning, as \
            `array_replace` does — and as `array_merge` over maps does, with every key treated \
            alike.",
    params: &[
        ParamDoc {
            name: "base",
            desc: "The array underneath.",
            shape: &[],
        },
        ParamDoc {
            name: "layers",
            desc: "The arrays laid over it, in order; a later layer wins over an earlier one.",
            shape: &[],
        },
    ],
    ret: "Every key of every argument: an existing key keeps its position and takes the last \
          value written under it, a new one is appended.",
    errors: &[],
};

/// `Core\Arr::overlayDeep`'s reference card — `rule:core-api/reference-card`.
const OVERLAY_DEEP_DOC: MethodDoc = MethodDoc {
    short: "`overlay` recursing wherever both sides of a key hold an array and neither is a \
            list, as `array_replace_recursive` does; a list is replaced wholesale.",
    params: &[
        ParamDoc {
            name: "base",
            desc: "The array underneath.",
            shape: &[],
        },
        ParamDoc {
            name: "layers",
            desc: "The arrays laid over it, in order; a later layer wins over an earlier one.",
            shape: &[],
        },
    ],
    ret: "Every key of every argument, as `overlay` answers it, with two maps under one key \
          combined the same way rather than the right one winning.",
    errors: &[],
};

/// `Core\Arr::underlay`'s reference card — `rule:core-api/reference-card`.
const UNDERLAY_DOC: MethodDoc = MethodDoc {
    short: "`$base` with every layer's entries written underneath it, the left-hand value \
            winning — PHP's `array + array`, which does not compile here.",
    params: &[
        ParamDoc {
            name: "base",
            desc: "The array on top.",
            shape: &[],
        },
        ParamDoc {
            name: "layers",
            desc: "The arrays laid under it, in order; an earlier one wins over a later one.",
            shape: &[],
        },
    ],
    ret: "`$base`'s entries in their own positions, then each layer's keys not yet present, \
          appended in order.",
    errors: &[],
};

/// `Core\Arr::appendAll`'s reference card — `rule:core-api/reference-card`.
const APPEND_ALL_DOC: MethodDoc = MethodDoc {
    short: "Every value of `$a`, then of each of `$others`, under fresh keys — `array_merge` \
            over lists, and the `array_merge(...$arrays)` idiom.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The first array.",
            shape: &[],
        },
        ParamDoc {
            name: "others",
            desc: "The arrays whose values follow, in order.",
            shape: &[],
        },
    ],
    ret: "A list renumbered from `\"0\"`; every key is discarded.",
    errors: &[],
};

/// `Core\Arr::diff`'s reference card — `rule:core-api/reference-card`.
const DIFF_DOC: MethodDoc = MethodDoc {
    short: "The entries of `$a` that `$b` does not have, under strict identity — `array_diff`, \
            `array_diff_key`, `array_diff_assoc` and their three `u` variants as one member.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array whose entries are kept.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The array whose entries exclude.",
            shape: &[],
        },
        ParamDoc {
            name: "on",
            desc: "Which part of an entry is compared: `SetOn::Values` (the default), \
                   `SetOn::Keys` or `SetOn::Both`.",
            shape: &[],
        },
        ParamDoc {
            name: "by",
            desc: "Called once per entry as `($value, $key)`, and may declare fewer parameters; \
                   what it answers stands in for the part `on` selected — the key, under \
                   `SetOn::Keys`.",
            shape: &[],
        },
        ParamDoc {
            name: "comparator",
            desc: "Called as `($a, $b)` on two selected parts and answering a negative, zero or \
                   positive number, as `array_udiff`'s callback does; it replaces identity, and \
                   makes the walk pairwise.",
            shape: &[],
        },
    ],
    ret: "The kept entries of `$a` under their own keys, in `$a`'s order.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The comparator answered `NaN`.",
    }],
};

/// `Core\Arr::intersect`'s reference card — `rule:core-api/reference-card`.
const INTERSECT_DOC: MethodDoc = MethodDoc {
    short: "The entries of `$a` that `$b` also has, under strict identity — `array_intersect` \
            and its five variants as one member.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array whose entries are kept.",
            shape: &[],
        },
        ParamDoc {
            name: "b",
            desc: "The array an entry must also be in.",
            shape: &[],
        },
        ParamDoc {
            name: "on",
            desc: "Which part of an entry is compared: `SetOn::Values` (the default), \
                   `SetOn::Keys` or `SetOn::Both`.",
            shape: &[],
        },
        ParamDoc {
            name: "by",
            desc: "Called once per entry as `($value, $key)`, and may declare fewer parameters; \
                   what it answers stands in for the part `on` selected — the key, under \
                   `SetOn::Keys`.",
            shape: &[],
        },
        ParamDoc {
            name: "comparator",
            desc: "Called as `($a, $b)` on two selected parts and answering a negative, zero or \
                   positive number, as `array_uintersect`'s callback does; it replaces \
                   identity, and makes the walk pairwise.",
            shape: &[],
        },
    ],
    ret: "The kept entries of `$a` under their own keys, in `$a`'s order.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The comparator answered `NaN`.",
    }],
};

/// `Core\Arr::countBy`'s reference card — `rule:core-api/reference-card`.
const COUNT_BY_DOC: MethodDoc = MethodDoc {
    short: "How many entries fall under each distinct value, as `array_count_values` does — or \
            under each answer of `by`, the group-and-count loop PHP has no function for.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to count over.",
            shape: &[],
        },
        ParamDoc {
            name: "by",
            desc: "Called once per entry as `($value, $key)`, and may declare fewer parameters; \
                   it answers the bucket's name, an `int` or a `string`. Without it the value \
                   itself is the bucket.",
            shape: &[],
        },
    ],
    ret: "One `uint` per distinct bucket, keyed by it in first-occurrence order; `1` and `\"1\"` \
          are one bucket.",
    errors: &[],
};

/// `Core\Arr::unique`'s reference card — `rule:core-api/reference-card`.
const UNIQUE_DOC: MethodDoc = MethodDoc {
    short: "The entries whose value has not been seen before, as `array_unique` does — by strict \
            identity, so `1` and `\"1\"` are two entries, never `SORT_STRING`'s cast.",
    params: &[
        ParamDoc {
            name: "a",
            desc: "The array to deduplicate.",
            shape: &[],
        },
        ParamDoc {
            name: "by",
            desc: "Called once per entry as `($value, $key)`, and may declare fewer parameters; \
                   identity is asked of what it answers, while the entry kept is still the \
                   entry.",
            shape: &[],
        },
    ],
    ret: "The first occurrence of each value, under its own key and in its own position.",
    errors: &[],
};

/// `Core\Arr::min`'s reference card — `rule:core-api/reference-card`.
const MIN_DOC: MethodDoc = MethodDoc {
    short: "The smallest entry under the natural order, as `min` with an array argument does — \
            the order `sort` uses without a comparator, never PHP's loose comparison.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to search.",
        shape: &[],
    }],
    ret: "The smallest value, the first of equals; `null` for the empty array. Two strings \
          compare bytewise, so `min([\"1e2\", \"50\"])` is `\"1e2\"`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "Two entries have no natural order between them — an `int` against a `string`, \
               or any object.",
    }],
};

/// `Core\Arr::max`'s reference card — `rule:core-api/reference-card`.
const MAX_DOC: MethodDoc = MethodDoc {
    short: "The largest entry under the natural order, as `max` with an array argument does — \
            the order `sort` uses without a comparator, never PHP's loose comparison.",
    params: &[ParamDoc {
        name: "a",
        desc: "The array to search.",
        shape: &[],
    }],
    ret: "The largest value, the first of equals; `null` for the empty array. Two strings \
          compare bytewise, so `max([\"1e2\", \"50\"])` is `\"50\"`.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "Two entries have no natural order between them — an `int` against a `string`, \
               or any object.",
    }],
};

/// `Core\Arr::sum`'s reference card — `rule:core-api/reference-card`.
const SUM_DOC: MethodDoc = MethodDoc {
    short: "The entries added up, as `array_sum` does, promoting entry by entry: `int` with `int` \
            stays `int`, a `float` anywhere makes the total `float`, a `decimal` anywhere makes \
            it `decimal`.",
    params: &[ParamDoc {
        name: "a",
        desc: "The numbers to add.",
        shape: &[],
    }],
    ret: "The total, of the widest type met; `0` as an `int` for the empty array.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The running `int` or `decimal` total leaves its type's range — there is no \
               silent promotion to `float` — or a `float` and a `decimal` meet in one array, \
               which have no common type.",
    }],
};

/// `Core\Arr::product`'s reference card — `rule:core-api/reference-card`.
const PRODUCT_DOC: MethodDoc = MethodDoc {
    short: "The entries multiplied together, as `array_product` does, promoting entry by entry \
            exactly as `sum` does.",
    params: &[ParamDoc {
        name: "a",
        desc: "The numbers to multiply.",
        shape: &[],
    }],
    ret: "The product, of the widest type met; `1` as an `int` for the empty array.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The running `int` or `decimal` product leaves its type's range — there is no \
               silent promotion to `float` — or a `float` and a `decimal` meet in one array, \
               which have no common type.",
    }],
};

/// `Core\Arr::average`'s reference card — `rule:core-api/reference-card`.
const AVERAGE_DOC: MethodDoc = MethodDoc {
    short: "The entries' mean — `array_sum($a) / count($a)` with the empty case answered.",
    params: &[ParamDoc {
        name: "a",
        desc: "The numbers to average.",
        shape: &[],
    }],
    ret: "A `decimal` when the total is one — exact, rounded half to even at the widest scale \
          the quotient admits — and a `float` otherwise; `null` for the empty array.",
    errors: &[ErrorDoc {
        error: "RuntimeError",
        desc: "The running total leaves its type's range or meets a `float` and a `decimal` in \
               one array, as `sum` throws; or a `decimal` quotient falls outside what a \
               `decimal` can hold.",
    }],
};

/// `Core\Arr::shapeAs`'s reference card — `rule:core-api/reference-card`.
const SHAPE_AS_DOC: MethodDoc = MethodDoc {
    short: "Reads `$a` as the type `T` written at the call site — an inline shape, `{name: T}`, \
            or a class carrying `#[Json\\Derive]` — converting each named field with `as` and \
            leaving every key the type does not name behind; write `array<T>` to read a list of \
            them instead.",
    params: &[ParamDoc {
        name: "a",
        desc: "The loose values to read: a form, a query string, a decoded document, or an \
               array a program built.",
        shape: &[],
    }],
    ret: "A new `T` whose every field is the declared type, or — for an `array<T>` — one per \
          entry, in the array's own order.",
    errors: &[
        ErrorDoc {
            error: "ParseError",
            desc: "A field `T` requires is absent, or holds a value `as` refuses for its \
                   declared type. Every field that failed is one issue on the error, at its own \
                   dotted path, so a form shows the whole list rather than the first item of it.",
        },
        ErrorDoc {
            error: "LogicError",
            desc: "`T` is a class that carries no `#[Json\\Derive]`, so there is no field list \
                   to read `$a` against.",
        },
    ],
};

/// `float|decimal` — what dividing spec § 2's `int|float|decimal` by a count
/// can land on, and the whole of what `average` answers under its `?`.
///
/// `int` is not on it, deliberately: an average is a quotient, and `rule:types/arithmetic` already makes `int / int` yield `int|float` rather than an `int`. A
/// `decimal` subject stays exact (its quotient is a `decimal`), which is the
/// reason this is a union rather than plain `float`.
const QUOTIENT: &[CoreTy] = &[CoreTy::Float, CoreTy::Decimal];

/// `Core\Order` — the enum [`nvs_core_arr_sort`]'s `{order: ...}` option takes.
///
/// Declared here, beside its only consumer, for the same reason [`CLASS`] is.
/// [`crate::registry`]'s `ENUMS` lists it; the value of each case is written
/// out rather than auto-incremented, and `Asc` is `0` so it is also what an
/// omitted `{order: ...}` ends up meaning.
pub const ORDER: CoreEnum = CoreEnum {
    name: r"Core\Order",
    cases: &[("Asc", 0), ("Desc", 1)],
    doc: Some(&ORDER_DOC),
};

/// [`ORDER`]'s reference card — `rule:core-api/reference-card`, on the first enum documented.
const ORDER_DOC: EnumDoc = EnumDoc {
    short: "The direction `Core\\Arr::sort` and `sortByKey` put elements in — spec § 2's \
            `{order: …}` option, which is `Asc` when omitted.",
    cases: &[
        CaseDoc {
            name: "Asc",
            desc: "Smallest first — what an omitted `{order: …}` means.",
        },
        CaseDoc {
            name: "Desc",
            desc: "Largest first — `rsort`, `arsort` and `krsort` as one option rather than \
                   three names.",
        },
    ],
};

/// `Core\SetOn` — the enum [`nvs_core_arr_diff`] and [`nvs_core_arr_intersect`]
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
    doc: Some(&SET_ON_DOC),
};

/// [`SET_ON`]'s reference card — `rule:core-api/reference-card`.
const SET_ON_DOC: EnumDoc = EnumDoc {
    short: "Which part of an entry `Core\\Arr::diff` and `intersect` compare — spec § 2's \
            `{on: …}` option, which is `Values` when omitted; `by` and `comparator` apply to \
            the part it selects.",
    cases: &[
        CaseDoc {
            name: "Values",
            desc: "The value alone — `array_diff`, `array_intersect` and their `u` forms.",
        },
        CaseDoc {
            name: "Keys",
            desc: "The key alone — `array_diff_key`, `array_intersect_key` and their `u` \
                   forms; a `by` here maps the key.",
        },
        CaseDoc {
            name: "Both",
            desc: "The key and the value together — `array_diff_assoc`, \
                   `array_intersect_assoc` and their `u` forms.",
        },
    ],
};

/// `int|string` — `rule:types/arrays`'s two array-key types, which the spec's § 2
/// writes at every member taking or producing a key.
///
/// **Element positions only**, which is why its `string` arm stays the
/// unclassified [`CoreTy::Str`]: `rule:security/unclassified-parameter-refuses-tainted` classifies a *parameter*, and
/// `every_member_parameter_carries_a_qualifier_classification` walks a row's
/// `params` and its options bag but not through a [`CoreTy::Array`]. A key
/// written at a parameter position takes [`ARRAY_KEY_NEUTRAL`] or
/// [`ARRAY_KEY_CONTAGIOUS`] instead — one union per classification, because the
/// mark belongs to the member and a shared const can only hold one answer.
const ARRAY_KEY: &[CoreTy] = &[CoreTy::Int, CoreTy::Str];

/// [`ARRAY_KEY`] at a parameter position whose member answers a `bool`.
///
/// [`Qual::Neutral`] by the first bullet of [`Qual`]'s own rule, which is where
/// that rule is written: `hasKey`'s answer carries no byte of either argument.
const ARRAY_KEY_NEUTRAL: &[CoreTy] = &[CoreTy::Int, CoreTy::Text(Qual::Neutral)];

/// [`ARRAY_KEY`] at a parameter position whose member answers part of its
/// subject.
///
/// [`Qual::Contagious`] by that rule's second bullet: `column`'s key names the
/// cell to lift out of each row and its `indexBy` names the cell to key the
/// result by, so neither one's own bytes reach the answer — and that is
/// precisely the case the bullet calls laundering by influence and refuses to
/// let a member do silently.
const ARRAY_KEY_CONTAGIOUS: &[CoreTy] = &[CoreTy::Int, CoreTy::Text(Qual::Contagious)];

/// `T|U` — the element type `rule:types/array-combination`'s four combination members answer, and
/// the spelling this module's own docs record as the one the registry can
/// state.
///
/// A union is legal in either direction ([`CoreTy::Union`]), so the spec's
/// `array<T|U>` is written literally rather than flattened to `array<mixed>`.
/// `T` binds from the base and `U` from the *first* trailing layer, and
/// `nvs_types::generics` substitutes both before a single argument is checked
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
/// `rule:types/preserve-keys` removes.
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

/// `{indexBy?: int|string}` — [`nvs_core_arr_column`]'s only option, and the
/// first union option whose members are not literals.
///
/// [`CoreTy::Union`]'s docs own the rule this widened: what an option's
/// default has to be is a value **outside** the declared type, so that "not
/// given" cannot be confused with something a call site wrote. `int|string`
/// admits no `null`, so [`Const::Null`] is that sentinel here exactly as it
/// already is for `{by?: callable}` — the closed-set-of-literals shape
/// `Core\Validate::isIp`'s `{version?: 4|6}` needed was one instance of that
/// rule rather than the rule itself.
const COLUMN_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "indexBy",
    ty: CoreTy::Union(ARRAY_KEY_CONTAGIOUS),
    default: Const::Null,
}];

/// `Core\Arr::from`'s `{limit?: uint}` — the spec's § 2 prose calls it "the
/// only guard against materialising an unbounded generator", so an omitted
/// limit is [`Const::Null`] and means *no* limit rather than zero, and a
/// written `{limit: 0}` is an empty result the caller asked for.
const FROM_OPTIONS: &[CoreOption] = &[CoreOption {
    name: "limit",
    ty: CoreTy::Uint,
    default: Const::Null,
}];

/// `Core\Arr::sort`'s
/// `{by?: callable, order?: Order, comparator?: callable, preserveKeys?: bool}`
/// — the eleven PHP sort functions plus `array_multisort` in one bag, which is
/// what the spec's § 2 *Ordering* note means by "descending is
/// `{order: Order::Desc}`, key-preservation is an option rather than a letter
/// in the name."
///
/// [`nvs_core_arr_sort`]'s own docs own what each option does and which
/// combinations are refused. Two things about the *declaration* belong here:
/// `by` and `comparator` are the first options whose default is
/// [`Const::Null`] (there is no "no callback" callable), and `order` is the
/// first use of [`CoreTy::Enum`].
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

/// `Core\Arr::sortByKey`'s `{order?: Order, comparator?: callable}` —
/// [`SORT_OPTIONS`] without the two options a key sort has no use for.
///
/// `by` is absent because the thing compared is already the key, and
/// `preserveKeys` is absent because a sort *by* key that renumbered would
/// have thrown away what it just sorted on. Both are the spec's own row
/// (§ 2 *Ordering*), not a narrowing this module chose; [`nvs_core_arr_sort_by_key`]'s
/// docs own why the row is shaped that way.
const SORT_BY_KEY_OPTIONS: &[CoreOption] = &[
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
];

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain.
///
/// [`crate::symbols`] chains one of these per domain, so a new class adds an
/// arm here rather than to a single workspace-wide match.
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_arr_count" => (nvs_core_arr_count as *const ()).cast(),
        "nvs_core_arr_filter" => (nvs_core_arr_filter as *const ()).cast(),
        "nvs_core_arr_map" => (nvs_core_arr_map as *const ()).cast(),
        "nvs_core_arr_map_keys" => (nvs_core_arr_map_keys as *const ()).cast(),
        "nvs_core_arr_group_by" => (nvs_core_arr_group_by as *const ()).cast(),
        "nvs_core_arr_reduce" => (nvs_core_arr_reduce as *const ()).cast(),
        "nvs_core_arr_is_empty" => (nvs_core_arr_is_empty as *const ()).cast(),
        "nvs_core_arr_has_key" => (nvs_core_arr_has_key as *const ()).cast(),
        "nvs_core_arr_is_list" => (nvs_core_arr_is_list as *const ()).cast(),
        "nvs_core_arr_keys" => (nvs_core_arr_keys as *const ()).cast(),
        "nvs_core_arr_values" => (nvs_core_arr_values as *const ()).cast(),
        "nvs_core_arr_reverse" => (nvs_core_arr_reverse as *const ()).cast(),
        "nvs_core_arr_flip" => (nvs_core_arr_flip as *const ()).cast(),
        "nvs_core_arr_sort" => (nvs_core_arr_sort as *const ()).cast(),
        "nvs_core_arr_sort_by_key" => (nvs_core_arr_sort_by_key as *const ()).cast(),
        "nvs_core_arr_range" => (nvs_core_arr_range as *const ()).cast(),
        "nvs_core_arr_slice" => (nvs_core_arr_slice as *const ()).cast(),
        "nvs_core_arr_replace_range" => (nvs_core_arr_replace_range as *const ()).cast(),
        "nvs_core_arr_flatten" => (nvs_core_arr_flatten as *const ()).cast(),
        "nvs_core_arr_flatten_deep" => (nvs_core_arr_flatten_deep as *const ()).cast(),
        "nvs_core_arr_column" => (nvs_core_arr_column as *const ()).cast(),
        "nvs_core_arr_chunk" => (nvs_core_arr_chunk as *const ()).cast(),
        "nvs_core_arr_append" => (nvs_core_arr_append as *const ()).cast(),
        "nvs_core_arr_prepend" => (nvs_core_arr_prepend as *const ()).cast(),
        "nvs_core_arr_without_first" => (nvs_core_arr_without_first as *const ()).cast(),
        "nvs_core_arr_without_last" => (nvs_core_arr_without_last as *const ()).cast(),
        "nvs_core_arr_from_keys_and_values" => {
            (nvs_core_arr_from_keys_and_values as *const ()).cast()
        }
        "nvs_core_arr_from" => (nvs_core_arr_from as *const ()).cast(),
        "nvs_core_arr_pad_start" => (nvs_core_arr_pad_start as *const ()).cast(),
        "nvs_core_arr_pad_end" => (nvs_core_arr_pad_end as *const ()).cast(),
        "nvs_core_arr_fill" => (nvs_core_arr_fill as *const ()).cast(),
        "nvs_core_arr_fill_keys" => (nvs_core_arr_fill_keys as *const ()).cast(),
        "nvs_core_arr_overlay" => (nvs_core_arr_overlay as *const ()).cast(),
        "nvs_core_arr_overlay_deep" => (nvs_core_arr_overlay_deep as *const ()).cast(),
        "nvs_core_arr_underlay" => (nvs_core_arr_underlay as *const ()).cast(),
        "nvs_core_arr_append_all" => (nvs_core_arr_append_all as *const ()).cast(),
        "nvs_core_arr_diff" => (nvs_core_arr_diff as *const ()).cast(),
        "nvs_core_arr_intersect" => (nvs_core_arr_intersect as *const ()).cast(),
        "nvs_core_arr_count_by" => (nvs_core_arr_count_by as *const ()).cast(),
        "nvs_core_arr_first" => (nvs_core_arr_first as *const ()).cast(),
        "nvs_core_arr_last" => (nvs_core_arr_last as *const ()).cast(),
        "nvs_core_arr_first_key" => (nvs_core_arr_first_key as *const ()).cast(),
        "nvs_core_arr_last_key" => (nvs_core_arr_last_key as *const ()).cast(),
        "nvs_core_arr_find" => (nvs_core_arr_find as *const ()).cast(),
        "nvs_core_arr_find_key" => (nvs_core_arr_find_key as *const ()).cast(),
        "nvs_core_arr_any" => (nvs_core_arr_any as *const ()).cast(),
        "nvs_core_arr_all" => (nvs_core_arr_all as *const ()).cast(),
        "nvs_core_arr_contains" => (nvs_core_arr_contains as *const ()).cast(),
        "nvs_core_arr_key_of" => (nvs_core_arr_key_of as *const ()).cast(),
        "nvs_core_arr_unique" => (nvs_core_arr_unique as *const ()).cast(),
        "nvs_core_arr_min" => (nvs_core_arr_min as *const ()).cast(),
        "nvs_core_arr_max" => (nvs_core_arr_max as *const ()).cast(),
        "nvs_core_arr_sum" => (nvs_core_arr_sum as *const ()).cast(),
        "nvs_core_arr_product" => (nvs_core_arr_product as *const ()).cast(),
        "nvs_core_arr_average" => (nvs_core_arr_average as *const ()).cast(),
        "nvs_core_arr_shape_as" => (nvs_core_arr_shape_as as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::count(array<T> $a): uint` — how many entries the array
    /// holds, replacing PHP's `count`/`sizeof`.
    ///
    /// `uint` rather than `int` because a count cannot be negative and ADR
    /// 0007 § 4 gives Novis a type that says so; the conversion below cannot
    /// fail, since `nvs_array_count` counts live entries of an allocation
    /// that fits in memory.
    fn nvs_core_arr_count(_ctx, args: [1]) {
        // Unreachable from source: parameter 0 is `array<T>` in `CLASS`
        // above, so a non-container argument is `E0401: expected
        // array<mixed>, found mixed` at the checker and no program reaches
        // this guard. It stays because it is what makes `array_ptr`'s answer
        // safe to unwrap. Every other guard on an `array` parameter in this
        // module is the same judgement and cites this one.
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
        let count = unsafe { nvs_runtime::nvs_array_count(array) };
        Ok(Value::uint(count.cast_unsigned()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::isEmpty(array<T> $a): bool` — whether the array holds no
    /// entries, replacing PHP's `empty($a)` and the `count($a) === 0` idiom.
    ///
    /// A member of its own rather than left to `count(…) === 0` because ADR
    /// 0063 R20's "no operation reachable two ways" is about *spellings the
    /// library offers*, and the spec's § 2 table lists this one: the question
    /// "is it empty" is answered without the caller having to know that a
    /// count is `uint` and therefore needs `0` written as one.
    fn nvs_core_arr_is_empty(_ctx, args: [1]) {
        // Unreachable from source: an `array<T>` parameter, refused at the
        // checker — [`nvs_core_arr_count`]'s guard states the judgement.
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
        let count = unsafe { nvs_runtime::nvs_array_count(array) };
        Ok(Value::bool(count == 0))
    }
}

nvs_runtime::nvs_helper! {
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
    /// `nvs_runtime::call_closure` trims them to what the closure wants, and
    /// is also where the retain/release around the call lives.
    ///
    /// Truthiness is `rule:expressions/truthy-positions`'s
    /// table through `nvs_runtime::value_truthy`, so a predicate returning
    /// `0`, `""` or an empty array behaves here exactly as it would in a
    /// condition.
    ///
    /// The first `Core` member to call back into Novis code, and therefore the
    /// first that can fail partway through. Nothing special is needed for
    /// that: `NvsArray` and `NvsStr` both release on drop, so the partial
    /// result and the entry's own key are freed by the early return itself.
    fn nvs_core_arr_filter(ctx, args: [2]) {
        let base = subject(args, "filter")?;
        // Read once for the whole walk rather than per entry: a predicate
        // declaring one parameter is never handed a key, so none is rendered
        // for it — `docs/perf/userland-gap.md` § D.
        let wants_key = nvs_runtime::closure_arity(args[1])? >= 2;

        let mut kept = NvsArray::new();
        let mut from = 0usize;
        while let Some(slot) = base.next_slot(from) {
            from = slot + 1;
            let value = base
                .value_at(slot)
                .expect("next_slot only names live entries");
            // Read *before* the call, since the predicate may reshape the
            // subject under the cursor. On a list this is the position itself
            // and allocates nothing — see [`store_at`].
            let key = base
                .slot_key(slot)
                .expect("next_slot only names live entries");

            let verdict = if wants_key {
                // One reference for the duration of the call, released right
                // after: `call_closure` takes its own, and `key` itself is
                // still owed to either `kept` or its own drop below.
                let key_arg = Value::str(key.to_str());
                let verdict = nvs_runtime::call_closure(ctx, args[1], &[value, key_arg]);
                #[expect(
                    unsafe_code,
                    reason = "this frame owns exactly the reference \
                              `key.to_str()` just produced"
                )]
                unsafe {
                    key_arg.release();
                }
                verdict
            } else {
                nvs_runtime::call_closure(ctx, args[1], &[value])
            };
            let verdict = verdict?;
            let truthy = nvs_runtime::value_truthy(verdict);
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
                store_at(&mut kept, key, value);
            }
        }
        Ok(Value::array(kept))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::map(array<T> $a, callable $fn): array<U>` — every entry
    /// replaced by what the callback answers for it, replacing PHP's
    /// `array_map`.
    ///
    /// **Keys are preserved**, which is PHP's own single-array behaviour and
    /// the one `Core\Arr::filter` already keeps: `array_map` renumbers only in
    /// its multi-array form, which `rule:core-api/shape-rules` R20 leaves no room for anyway.
    /// Re-keying is `mapKeys`, its own member in the spec's § 2 table.
    ///
    /// The callback receives `($value, $key)` and may declare fewer parameters,
    /// the same rule and the same `nvs_runtime::call_closure` trimming
    /// [`nvs_core_arr_filter`] documents.
    ///
    /// **The `U` in the signature is real.** `map`'s result type is the
    /// callback's own return type, bound at the call site from the `fn`
    /// literal's recorded return — `nvs_types::generics` owns that rule and the
    /// one argument shape that still leaves it `mixed`. Nothing here depends on
    /// it: the helper stores whatever `Value` the callback produced.
    ///
    /// The mapped value is *owned* by this frame — `call_closure` returns one
    /// fresh reference — so it is stored without a retain and never released.
    /// That is the difference from `filter`, which stores a value belonging to
    /// the subject array and therefore has to retain one first.
    fn nvs_core_arr_map(ctx, args: [2]) {
        let base = subject(args, "map")?;
        // Read once for the whole walk — [`nvs_core_arr_filter`]'s comment.
        let wants_key = nvs_runtime::closure_arity(args[1])? >= 2;

        let mut out = NvsArray::new();
        // The live slots, as an iterator, so the walk goes through
        // `nvs_runtime::bounded_loop` and the deadline poll arrives with the
        // shape rather than being remembered here — `rule:http-server/time-is-bounded-inside-a-helper`'s first
        // constraint. An entry boundary is a point where abandoning is
        // consistent: `out` is a named local holding whole entries, so
        // propagating a fired poll releases the partial result by dropping it,
        // exactly as the throw path below already does.
        let mut from = 0usize;
        let slots = std::iter::from_fn(|| {
            let slot = base.next_slot(from)?;
            from = slot + 1;
            Some(slot)
        });
        nvs_runtime::bounded_loop(ctx, "Core\\Arr::map", slots, |ctx, slot| {
            let value = base
                .value_at(slot)
                .expect("next_slot only names live entries");
            // Read before the call, and on a list this is the position itself:
            // preserving a key is not the same as rendering one — [`store_at`].
            let key = base
                .slot_key(slot)
                .expect("next_slot only names live entries");

            let mapped = if wants_key {
                // One reference for the duration of the call, released right
                // after — `call_closure` takes its own. `key` itself is still
                // owed to [`store_at`] below.
                let key_arg = Value::str(key.to_str());
                let mapped = nvs_runtime::call_closure(ctx, args[1], &[value, key_arg]);
                #[expect(
                    unsafe_code,
                    reason = "this frame owns exactly the reference \
                              `key.to_str()` just produced"
                )]
                unsafe {
                    key_arg.release();
                }
                mapped
            } else {
                nvs_runtime::call_closure(ctx, args[1], &[value])
            };
            // Unwrapped into a local of its own *before* `key` is moved: a
            // throw partway through then frees the partial result and this
            // entry's key by dropping two named locals, which `NvsArray` and
            // `SlotKey`'s own `NvsStr` both do by releasing.
            let mapped = mapped?;
            store_at(&mut out, key, mapped);
            Ok(())
        })?;
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::mapKeys(array<T> $a, callable $fn): array<T>` — every entry
    /// under the key its callback names, replacing the
    /// `array_combine(array_map(…), …)` dance and the userland `keyBy` idiom.
    ///
    /// [`nvs_core_arr_map`]'s mirror: that member replaces the value and keeps
    /// the key, this one replaces the key and keeps the value, and the spec's
    /// § 2 prose calls this the only member that changes a key. The callback
    /// receives `($value, $key)` and may declare fewer parameters, the rule
    /// [`nvs_core_arr_filter`] documents.
    ///
    /// The new key goes through [`key_bytes`], so a callback answering `1` and
    /// one answering `"1"` name one entry — the normalization every key
    /// position in this class shares. A callback answering anything else is a
    /// throw, which is `flip`'s treatment of the same situation.
    ///
    /// **Duplicate new keys collapse, the last entry winning.** That is
    /// `NvsArray::set`'s own rule rather than this member's opinion, and it is
    /// what `flip` already does; a member that refused instead would make
    /// `keyBy`-style re-keying unusable over data whose new key is not unique.
    ///
    /// The value belongs to the subject array, so the copy stored takes a
    /// reference of its own — [`nvs_core_arr_filter`]'s obligation, not
    /// `map`'s, because the callback's own answer is a *key* here and is
    /// released as soon as its bytes are read.
    fn nvs_core_arr_map_keys(ctx, args: [2]) {
        let base = subject(args, "mapKeys")?;
        let mut out = NvsArray::new();
        let mut from = 0usize;
        while let Some(slot) = base.next_slot(from) {
            from = slot + 1;
            let value = base
                .value_at(slot)
                .expect("next_slot only names live entries");
            let key = base
                .key_at(slot)
                .expect("next_slot only names live entries");

            // One reference for the duration of the call, released right
            // after — `call_closure` takes its own, and the old key is not
            // owed to anything else here.
            let key_arg = Value::str(key);
            let named = nvs_runtime::call_closure(ctx, args[1], &[value, key_arg]);
            #[expect(
                unsafe_code,
                reason = "this frame owns exactly the reference `key_at` just \
                          handed back"
            )]
            unsafe {
                key_arg.release();
            }
            let named = named?;
            let fresh = key_bytes(&named, "mapKeys");
            #[expect(
                unsafe_code,
                reason = "the new key is a fresh value this frame owns; a \
                          callback returning a string would otherwise leak one \
                          reference per entry"
            )]
            unsafe {
                named.release();
            }
            // Unwrapped *before* the retain below: a throw there would
            // otherwise leave one reference owed to a result this frame is
            // about to drop.
            let fresh = fresh?;
            #[expect(
                unsafe_code,
                reason = "the value is owned by the subject array, which \
                          outlives this call, so the copy stored here needs a \
                          reference of its own"
            )]
            unsafe {
                value.retain();
            }
            out.set(NvsStr::new(&fresh), value);
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::groupBy(array<T> $a, callable $key): array<array<T>>` — the
    /// subject partitioned into buckets its callback names, replacing the
    /// `$out[$fn($v)][] = $v` loop PHP has no function for at all.
    ///
    /// [`nvs_core_arr_count_by`] with the entries kept instead of counted, and
    /// it shares that member's two rules: the callback receives
    /// `($value, $key)`, and a bucket is named through [`key_bytes`] so `1`
    /// and `"1"` are one bucket. Buckets come out in **first-occurrence**
    /// order, which is `countBy`'s order and the only one that does not
    /// impose a sort nobody asked for.
    ///
    /// **A bucket keeps each entry's own key.** Every other member that puts
    /// entries side by side renumbers — `flatten`, `appendAll`, `values` — but
    /// each does so because two sources can hold the same key and there is no
    /// rule that keeps both
    /// (`rule:types/preserve-keys`). A partition has no such collision: each entry lands in exactly
    /// one bucket under the key it already had, so preserving it loses
    /// nothing and answers "which entries grouped here" as well as "what". A
    /// caller wanting lists writes `Core\Arr::values` over the buckets, which
    /// is the member for exactly that and is not recoverable the other way
    /// round. `map` and `filter` preserve keys for the same reason and the
    /// spec's § 2 prose says so.
    ///
    /// Buckets are built in a `Vec` beside the result rather than mutated
    /// through it: `NvsArray::set` would hand back a *borrowed* bucket to
    /// mutate, and every write to it would have to reason about whether the
    /// result still holds the only reference. The `Vec` costs one pointer and
    /// one key copy per distinct bucket, for the length of the call, and its
    /// `Drop` is what frees a partial answer when a callback throws part-way
    /// through — the reason [`Extracted`] exists one member down.
    fn nvs_core_arr_group_by(ctx, args: [2]) {
        let base = subject(args, "groupBy")?;
        let mut buckets: Vec<(Vec<u8>, NvsArray)> = Vec::new();
        let mut index_of: std::collections::HashMap<Vec<u8>, usize> =
            std::collections::HashMap::new();
        let mut from = 0usize;
        while let Some(slot) = base.next_slot(from) {
            from = slot + 1;
            let value = base
                .value_at(slot)
                .expect("next_slot only names live entries");
            let key = base
                .key_at(slot)
                .expect("next_slot only names live entries");

            // One reference for the duration of the call, released right
            // after: `call_closure` takes its own, and `key` itself is still
            // owed to the bucket below.
            let key_arg = Value::str(key.clone());
            let named = nvs_runtime::call_closure(ctx, args[1], &[value, key_arg]);
            #[expect(
                unsafe_code,
                reason = "this frame owns exactly the reference `key.clone()` \
                          just produced"
            )]
            unsafe {
                key_arg.release();
            }
            let named = named?;
            let bucket = key_bytes(&named, "groupBy");
            #[expect(
                unsafe_code,
                reason = "the bucket name is a fresh value this frame owns; a \
                          callback returning a string would otherwise leak one \
                          reference per entry"
            )]
            unsafe {
                named.release();
            }
            let bucket = bucket?;

            let next = buckets.len();
            let position = *index_of.entry(bucket.clone()).or_insert(next);
            if position == next {
                buckets.push((bucket, NvsArray::new()));
            }
            #[expect(
                unsafe_code,
                reason = "the value is owned by the subject array, which \
                          outlives this call, so the copy stored here needs a \
                          reference of its own"
            )]
            unsafe {
                value.retain();
            }
            buckets[position].1.set(key, value);
        }

        let mut out = NvsArray::new();
        for (name, bucket) in buckets {
            out.set(NvsStr::new(&name), Value::array(bucket));
        }
        Ok(Value::array(out))
    }
}

/// The `{preserveKeys?: bool}` option's value — [`PRESERVE_KEYS`] read back on
/// the runtime side.
///
/// The bag is flattened into one ordinary argument by
/// `nvs_ir::lower::lower_call_args`, so the slot is always present and a wrong
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
/// `rule:types/arrays` makes every stored key a `string`, and `nvs-ir` already
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

/// A borrowed `NvsArray` handle over an argument's pointer, for the members
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
pub(crate) fn borrowed(array: *mut nvs_runtime::ArrayHeader) -> std::mem::ManuallyDrop<NvsArray> {
    #[expect(
        unsafe_code,
        reason = "a Tag::Array argument owns a reference to a live allocation, \
                  so it is live for the length of this call, and the handle is \
                  never dropped"
    )]
    std::mem::ManuallyDrop::new(unsafe { NvsArray::from_raw(array) })
}

/// Stores `value` under the key a subject entry already had, taking over its
/// reference — the key-preserving half of [`nvs_core_arr_map`] and
/// [`nvs_core_arr_filter`].
///
/// A [`SlotKey::Index`] goes through [`NvsArray::set_index`], which writes at
/// an existing position or the next one with no decimal rendered at all, so
/// mapping a list allocates no keys — and a `filter` that skips an entry still
/// preserves the keys it kept, because the gap is exactly what degrades the
/// result to the hash form and renders them there. `docs/perf/userland-gap.md`
/// § D is the measurement.
pub(crate) fn store_at(out: &mut NvsArray, key: SlotKey, value: Value) {
    match key {
        SlotKey::Index(index) => out.set_index(index, value),
        SlotKey::Str(key) => out.set(key, value),
    }
}

/// Appends `times` copies of a borrowed `value` to a result being built.
///
/// The value belongs to the calling frame's argument, which outlives the call,
/// so every copy stored takes a reference of its own — the same rule
/// [`copy_entry`] applies to an entry, over a value that is repeated rather
/// than walked. `times` is a `u64` because it comes from a `uint` argument and
/// the loop is what turns it into allocations.
///
/// **Two checks, and they answer different questions** — `Core\Str`'s
/// `built_fallibly` and `Core\Bytes`' `reserved` run the same pair for the same
/// reason. `nvs_runtime::affordable` is the policy seam every count-shaped
/// argument passes through and refuses only a size past `isize::MAX`; every
/// count below it that the machine cannot serve used to reach an infallible
/// `Vec::push` inside [`NvsArray`] and abort, which takes the process and every
/// in-flight request with it for a refusal a caller may well want to handle.
/// [`NvsArray::try_reserve`] asks the allocator instead, and because `out` is a
/// list here — all three callers append into a fresh array — the reservation is
/// the whole of what the loop below allocates.
///
/// `member` is the qualified name, because both sentences carry it and a
/// padding member reaching here must not report `Core\Arr::fill`.
fn append_copies(out: &mut NvsArray, value: Value, times: u64, member: &str) -> Result<(), Fault> {
    let entries = usize::try_from(times)
        .ok()
        .and_then(|count| count.checked_mul(std::mem::size_of::<Value>()));
    // The byte size `affordable` accepted, back over the size of one entry —
    // exactly, because that is how it was formed.
    let count = nvs_runtime::affordable(entries, member)? / std::mem::size_of::<Value>();
    if !out.try_reserve(count) {
        return Err(Fault::thrown(format!(
            "{member}: the result is larger than any array this process could hold"
        )));
    }
    for _ in 0..times {
        append_borrowed(out, value);
    }
    Ok(())
}

/// Appends one borrowed value to a result being built, under the next
/// integer key.
///
/// The value belongs to an argument of the calling frame, which outlives the
/// call, so the copy stored takes a reference of its own — [`copy_entry`]'s
/// rule without the key half. Shared by [`append_copies`], which is this
/// repeated, and by the two members whose variadic tail is a run of loose
/// values rather than of arrays.
fn append_borrowed(out: &mut NvsArray, value: Value) {
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

/// [`append_borrowed`] for a result that may already hold `i64::MAX` as a key,
/// refusing there instead of panicking.
///
/// [`NvsArray::append`] asserts that the next integer key is free, which holds
/// for every result built from index 0 by the call itself and **not** for
/// [`nvs_core_arr_append`]'s, which starts as a copy of the subject and
/// therefore inherits whatever keys the program put there. The refusal is the
/// one `$a[] = $v` already reports from `nvs_runtime`'s `nvs_array_append`,
/// message included, so the statement and the member answer the same program
/// the same way.
fn append_borrowed_or_refuse(out: &mut NvsArray, value: Value) -> Result<(), Fault> {
    #[expect(
        unsafe_code,
        reason = "the value is owned by the caller's argument, which \
                  outlives this call, so the copy stored here needs a \
                  reference of its own — and a refusal hands that \
                  reference straight back, so it is released again"
    )]
    unsafe {
        value.retain();
    }
    out.try_append(value).map_err(|refused| {
        #[expect(
            unsafe_code,
            reason = "the refusal handed back the reference retained above, \
                      which nothing else holds"
        )]
        unsafe {
            refused.release();
        }
        Fault::thrown_as(
            ThrownClass::Logic,
            "Cannot add element to the array as the next element is already occupied",
        )
    })
}

/// Appends every value of a borrowed subject to a result being built, under
/// fresh `0, 1, …` keys — [`nvs_core_arr_values`]'s walk, shared by the two
/// padding members because each one wraps it in padding on a different side,
/// and by [`nvs_core_arr_append_all`], which is that walk over every argument
/// in turn and nothing else.
fn append_values(subject: &NvsArray, out: &mut NvsArray) {
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
/// because [`NvsArray::slot_key`] already hands back either a position or a
/// fresh reference to the one the subject holds. That asymmetry is the single
/// thing a key-and-value copy has to get right, so it lives here once rather
/// than in each member that walks entries through unchanged.
///
/// **The key travels as a [`SlotKey`] and is never rendered.** A packed
/// subject's key *is* its position, so rendering the decimal here would
/// allocate a string per entry that [`store_at`]'s `set_index` has no use for
/// — and the result lands at the same key either way, because that is what
/// `set_index` writes at. The `$key` a **callback** receives is the other
/// question and takes the other answer, which the section above states.
fn copy_entry(subject: &NvsArray, slot: usize, out: &mut NvsArray) {
    let key = subject
        .slot_key(slot)
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
    store_at(out, key, value);
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::hasKey(array<T> $a, int|string $key): bool` — replacing
    /// PHP's `array_key_exists` **and** `isset($a[$k])`, which differ in PHP
    /// only over a stored `null` and therefore cannot both survive `rule:core-api/shape-rules`
    /// R20.
    ///
    /// The first member with a **union** parameter. It needs no IR
    /// representation for one: the helper's argument slot is a tagged
    /// `nvs_runtime::Value` written from the argument's own type, so
    /// [`key_bytes`] decodes by tag. `nvs_ir::lower::ArgSig::helper` owns why
    /// that is a property of the helper convention rather than of this member.
    fn nvs_core_arr_has_key(_ctx, args: [2]) {
        // Unreachable from source: an `array<T>` parameter, refused at the
        // checker — [`nvs_core_arr_count`]'s guard states the judgement.
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

nvs_runtime::nvs_helper! {
    /// `Core\Arr::isList(array<T> $a): bool` — whether the keys are exactly
    /// `0, 1, …, n-1` in that order, replacing PHP's `array_is_list`. An empty
    /// array is a list, as it is in PHP.
    ///
    /// `rule:types/arrays` stores every key as a `string`, so "is this an integer key"
    /// is a question about the *bytes*: the key must be the index's decimal
    /// spelling exactly, which rules out `"01"` and `"+1"` the way PHP's
    /// canonical-integer-key normalization already would. Those bytes are
    /// compared against a spelling rendered on the stack, so the member is O(n)
    /// and allocates nothing at all — `benches/members/core/Arr/isList.nvs`
    /// declares the zero and is what would catch it coming back.
    fn nvs_core_arr_is_list(_ctx, args: [1]) {
        // Unreachable from source: an `array<T>` parameter, refused at the
        // checker — [`nvs_core_arr_count`]'s guard states the judgement.
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

/// `rule:types/arrays`'s `"0" … "n−1"` test, over a borrowed array.
///
/// Lifted out of [`nvs_core_arr_is_list`] rather than left inline because
/// `rule:types/array-combination` states `overlayDeep`'s recursion rule in terms of it — two sides of a
/// key merge only where both hold an array and **neither is a list** — so the
/// member and the rule now read the same predicate rather than two spellings
/// of it.
fn is_list(subject: &NvsArray) -> bool {
    let mut from = 0usize;
    let mut index = 0usize;
    while let Some(slot) = subject.next_slot(from) {
        // `slot_key` rather than `key_at`: a packed array *is* its positions,
        // and asking for them as strings rendered one `NvsStr` per entry for a
        // comparison that never needed the bytes.
        let carries_its_index = match subject
            .slot_key(slot)
            .expect("next_slot only names live entries")
        {
            SlotKey::Index(position) => usize::try_from(position).is_ok_and(|at| at == index),
            SlotKey::Str(key) => is_index_spelling(key.as_bytes(), index),
        };
        if !carries_its_index {
            return false;
        }
        from = slot + 1;
        index += 1;
    }
    true
}

/// Whether `key` is `index`'s decimal spelling exactly, rendered on the stack.
///
/// The comparison [`is_list`] makes per entry, written this way so the walk
/// holds no buffer: twenty digits is every `usize`, and a key of any other
/// length loses on the slice comparison before a digit is read. Length is what
/// rules out `"01"`, and the digits are what rule out `"+1"` and `" 1"`.
fn is_index_spelling(key: &[u8], index: usize) -> bool {
    let mut digits = [0u8; 20];
    let mut written = digits.len();
    let mut rest = index;
    loop {
        written -= 1;
        digits[written] = b'0' + u8::try_from(rest % 10).expect("a decimal digit is below ten");
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    key == &digits[written..]
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::keys(array<T> $a): array<string>` — every key in insertion
    /// order under fresh `0, 1, …` keys of its own, replacing PHP's
    /// `array_keys`.
    ///
    /// **`array<string>`, never `array<int|string>`.** `rule:types/arrays` stores
    /// every key as a `string`, so a key that *looks* like an integer is one
    /// only in its spelling — `Core\Arr::keys(["10" => "x"])` yields `["10"]`,
    /// and PHP's `array_keys` yielding an `int` there is the key-type-dependent
    /// behaviour `rule:types/array-combination` removes. A caller who wants the number writes
    /// `$k as int`, which is the same thing a `foreach` key binding already
    /// does.
    ///
    /// PHP's `array_keys($a, $search)` search form is not reproduced: that is
    /// `Core\Arr::keyOf` for one and a `filter` for many, and folding two
    /// unrelated questions into one name is what `rule:core-api/shape-rules` R20 refuses.
    fn nvs_core_arr_keys(_ctx, args: [1]) {
        // Unreachable from source: an `array<T>` parameter, refused at the
        // checker — [`nvs_core_arr_count`]'s guard states the judgement.
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::keys expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let subject = borrowed(array);

        let mut out = NvsArray::new();
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
/// array of `count` entries, under `rule:core-api/shape-rules` R8's sign rule.
///
/// Deliberately `crate::str::window`'s rule, one unit up: an array counts in
/// *entries* where a string counts in characters, and nothing else differs.
///
/// * A **negative offset** counts back from the last entry, and one reaching
///   past the first clamps to it.
/// * A **negative length** stops that many entries short of the end.
/// * A **null length** runs to the end. That is the type saying what a
///   sentinel would otherwise have to (`rule:core-api/shape-rules` R5), and [`Const::Null`] is
///   what a call site materializes for a `slice` that omits it.
/// * The end never precedes the start, so a window that closes before it
///   opens is empty rather than reversed.
///
/// The positions are *ordinal*, not keys: entry 0 is whichever entry the walk
/// meets first. An ordered hash has no other reading, and it is what makes
/// `slice` answer the same entries whatever the keys happen to be — the
/// key-type independence
/// `rule:types/array-combination`
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
fn carry_entry(subject: &NvsArray, slot: usize, out: &mut NvsArray, preserve: bool) {
    if preserve {
        copy_entry(subject, slot, out);
    } else {
        let value = subject
            .value_at(slot)
            .expect("next_slot only names live entries");
        append_borrowed(out, value);
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::slice(array<T> $a, int $offset, ?int $length = null, {preserveKeys?: bool}): array<T>`
    /// — the entries in one window, replacing PHP's `array_slice`.
    ///
    /// [`window`] owns what each sign means, and [`PRESERVE_KEYS`] owns what
    /// the option's `false` default does: every key discarded and the result
    /// renumbered from `"0"`, rather than `array_slice`'s
    /// renumber-the-integers-keep-the-strings. That is the divergence to know
    /// about, and it is `rule:types/preserve-keys`'s rule rather than this member's opinion.
    ///
    /// An `args: [4]` helper for a three-parameter signature: the option bag
    /// flattens into one ordinary argument, as [`nvs_core_arr_range`]'s docs
    /// spell out.
    ///
    /// The walk stops at the window's end rather than running to the last
    /// entry — a slice of the first two of ten touches two entries, which is
    /// what makes this the member a paging loop calls.
    fn nvs_core_arr_slice(_ctx, args: [4]) {
        let base = subject(args, "slice")?;
        let preserve = preserve_keys(&args[3], "slice")?;
        let (start, end) = window(base.count(), &args[1], &args[2], "slice")?;

        let mut out = NvsArray::new();
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

nvs_runtime::nvs_helper! {
    /// `Core\Arr::replaceRange(array<T> $a, int $offset, ?int $length, array<T> $replacement = []): array<T>`
    /// — one window's entries substituted, replacing PHP's `array_splice` in
    /// its returning form (`rule:core-api/shape-rules` R3 makes every member pure, so the
    /// by-reference half is not reproduced).
    ///
    /// This is [`nvs_core_arr_slice`]'s window with the entries *replaced*
    /// rather than returned, and it reads its two positional arguments through
    /// the same [`window`] — so `replaceRange($a, $o, $n)` removes exactly what
    /// `slice($a, $o, $n)` answers, for every sign of every argument.
    /// `crate::str`'s pair of the same two names already holds that property
    /// one unit down, in characters rather than entries.
    ///
    /// **The result renumbers**, subject and replacement alike: a list from
    /// `"0"`, never a mix of kept and fresh keys. That is `slice`'s own
    /// default answer and
    /// `rule:types/preserve-keys`'s rule rather than this member's opinion — `array_splice`'s
    /// "renumber the integers, keep the strings" is exactly the key-type
    /// dependence that rule refuses. There is no `{preserveKeys?: bool}`
    /// option, and not only because the spec row declares none: the entries
    /// either side of the window and the ones spliced into it cannot all keep
    /// a key without colliding, so the option would have no honest meaning.
    ///
    /// An empty window is an **insertion** at that position — how a `$length`
    /// of `0`, or a negative one reaching back past the offset, reads — and an
    /// offset at the end appends. An omitted `$replacement` makes the member a
    /// removal; `registry::Const::EmptyArray` is the constant that call site
    /// materializes, and its docs own what the omission costs.
    fn nvs_core_arr_replace_range(_ctx, args: [4]) {
        let base = subject(args, "replaceRange")?;
        let replacement = array_at(&args[3], "replaceRange", "the replacement")?;
        let (start, end) = window(base.count(), &args[1], &args[2], "replaceRange")?;

        // Three walks, one per part, exactly as `crate::str`'s member is three
        // pushes: the head, the replacement, then the tail. The window itself
        // is walked only to step over it.
        let mut out = NvsArray::new();
        let mut from = 0usize;
        let mut position = 0usize;
        while position < start {
            let Some(slot) = base.next_slot(from) else {
                break;
            };
            from = slot + 1;
            carry_entry(&base, slot, &mut out, false);
            position += 1;
        }
        append_values(&replacement, &mut out);
        while position < end {
            let Some(slot) = base.next_slot(from) else {
                break;
            };
            from = slot + 1;
            position += 1;
        }
        while let Some(slot) = base.next_slot(from) {
            from = slot + 1;
            carry_entry(&base, slot, &mut out, false);
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::chunk(array<T> $a, uint $size, {preserveKeys?: bool}): array<array<T>>`
    /// — the entries in runs of `$size`, replacing PHP's `array_chunk`.
    ///
    /// The last run is short when the count does not divide, and an empty
    /// subject yields no runs at all rather than one empty one. Both are PHP's
    /// answers.
    ///
    /// **A `$size` of zero throws**, for [`nvs_core_arr_range`]'s reason and
    /// with the class that member's docs say the tree still owes: there is no
    /// run length that would terminate, so silently answering an empty array
    /// would turn a caller's arithmetic bug into a loop that does nothing.
    /// `$size` is a `uint`, so a negative one is a diagnostic rather than a
    /// throw — PHP's `ValueError` covers both.
    ///
    /// [`PRESERVE_KEYS`] governs the *inner* arrays' keys; the outer one is
    /// always a list, since a run has no key of its own to keep. That is the
    /// same question [`nvs_core_arr_slice`] answers and it is answered the
    /// same way, which is why the two share [`carry_entry`].
    fn nvs_core_arr_chunk(_ctx, args: [3]) {
        let base = subject(args, "chunk")?;
        // Unreachable from source: parameter 1 is `CoreTy::Uint` in `CLASS`
        // above, so anything else is `E0401: expected 'uint', found …` at the
        // checker — probed with a `mixed` binding and with the literal `-1`,
        // which is the near miss worth checking here because a size a caller
        // computed wrong is how a negative one would arrive, and it is
        // `expected 'uint', found 'int'` rather than the `size == 0` throw
        // below. `nvs_core_arr_count` states the general form.
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

        let mut out = NvsArray::new();
        let mut run = NvsArray::new();
        let mut held = 0u64;
        let mut from = 0usize;
        while let Some(slot) = base.next_slot(from) {
            from = slot + 1;
            carry_entry(&base, slot, &mut run, preserve);
            held += 1;
            if held == size {
                out.append(Value::array(std::mem::replace(&mut run, NvsArray::new())));
                held = 0;
            }
        }
        if held > 0 {
            out.append(Value::array(run));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::append(array<T> $a, T ...$values): array<T>` — the subject
    /// with every trailing value after its last entry, replacing PHP's
    /// `array_push` and giving `$a[] = $v` a form that is an expression.
    ///
    /// **Every key of the subject is kept**, and each added value lands under
    /// the next free integer key — which is what `$a[] = $v` does, and is the
    /// whole reason this member can claim to replace it. The two part in
    /// exactly one place, and it is R3's purity rather than a different rule
    /// about keys: `$a[] = $v` mutates one array and inherits its counter,
    /// which survives an `unset` exactly as PHP's does, while this member
    /// returns a *fresh* array whose counter is derived from the entries
    /// copied into it — one past the largest integer key actually present. So
    /// after `unset($a["9"])` the statement lands on 10 and the member on 6.
    /// `tests/differential/core/arr-append-derives-the-next-free-key-where-php-remembers-it.nvst`
    /// pins both halves against PHP, including `array_pop` *lowering* the
    /// counter to the key it removed, which nothing here reproduces. That is
    /// not a
    /// key-type-dependent rule of the kind
    /// `rule:types/preserve-keys` removes: the key chosen is one counter's next value whatever the
    /// existing keys look like, so it never has to ask what type they were.
    /// It is also why this member takes no `preserveKeys` option — appending
    /// invents no key that could collide with one already there, so there is
    /// nothing to choose. [`nvs_core_arr_prepend`] is the side where there is.
    ///
    /// A call with no trailing values at all is the subject, entry for entry.
    ///
    /// **The one refusal is the one `$a[] = $v` has**, and for the same
    /// reason: a subject already holding `i64::MAX` as a key has no next
    /// integer key to derive, so a call adding a value to it throws the
    /// `LogicError` that statement throws, with that statement's message.
    /// [`append_borrowed_or_refuse`] is where the two meet; the subject is
    /// untouched either way, since this member builds a result.
    fn nvs_core_arr_append(_ctx, args: [2]) {
        let base = subject(args, "append")?;
        let mut out = NvsArray::new();
        copy_all(&base, &mut out);
        for_each_trailing(&args[1], "append", |value| {
            append_borrowed_or_refuse(&mut out, value)
        })?;
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::prepend(array<T> $a, T ...$values): array<T>` — the trailing
    /// values in written order, then the subject's, replacing PHP's
    /// `array_unshift`.
    ///
    /// **The result is always a list**, whatever the subject's keys were, for
    /// [`nvs_core_arr_pad_start`]'s reason and no other: an entry put in
    /// *front* of an existing one has no non-arbitrary key. `"0"` is the only
    /// candidate and the subject may already hold it, at which point the
    /// alternatives are to overwrite an entry the call never mentioned or to
    /// pick `"1"` and store it ahead of `"0"` — an order that contradicts the
    /// keys. PHP escapes that by renumbering the integer keys and keeping the
    /// string ones, which is the key-type-dependent behaviour `rule:types/preserve-keys`
    /// removes; renumbering *every* key is the same answer applied uniformly,
    /// and it is what spec § 2 means by `{preserveKeys: false}` wherever the
    /// option appears. The option is not declared here because, as in the
    /// padding members, keeping a key is not a choice that can be offered.
    ///
    /// So the two members are **not** mirror images, and the asymmetry is
    /// inherited rather than invented: appending has a free key to use and
    /// [`nvs_core_arr_append`] therefore keeps the subject's, while prepending
    /// does not. A call with no trailing values is `Core\Arr::values` of the
    /// subject.
    fn nvs_core_arr_prepend(_ctx, args: [2]) {
        let base = subject(args, "prepend")?;
        let mut out = NvsArray::new();
        for_each_trailing(&args[1], "prepend", |value| {
            append_borrowed(&mut out, value);
            Ok(())
        })?;
        append_values(&base, &mut out);
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::withoutFirst(array<T> $a): array<T>` — every entry but the
    /// first, replacing what is left of the subject after PHP's `array_shift`.
    ///
    /// PHP splits that question across a statement that answers two at once:
    /// `array_shift($a)` hands back the element *and* mutates `$a`. `rule:core-api/shape-rules` R3
    /// makes nothing mutate, so the spec's § 2 table splits it in two —
    /// `Core\Arr::first` gets the element, this gets the remainder — and an
    /// empty array yields an empty array rather than a `null` and a warning.
    ///
    /// **Every surviving key is kept.** The member declares no `preserveKeys`
    /// option, and the spec's § 2 rule that the option's `false` default
    /// discards every key applies where it *appears*; here there is nothing to
    /// choose, so this behaves like [`nvs_core_arr_filter`], which also drops
    /// entries without renumbering the ones that remain. That is deliberately
    /// not `array_shift`'s behaviour, which renumbers integer keys and keeps
    /// string ones — the key-type-dependent rule
    /// `rule:types/preserve-keys` removes. A caller who wants `0, 1, …` writes `Core\Arr::values` and
    /// says so; a caller who wanted to keep a map's keys has no way to get them
    /// back once a member has thrown them away, so keeping is the direction
    /// that loses nothing.
    fn nvs_core_arr_without_first(_ctx, args: [1]) {
        // Unreachable from source: an `array<T>` parameter, refused at the
        // checker — [`nvs_core_arr_count`]'s guard states the judgement.
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
            None => return Ok(Value::array(NvsArray::new())),
        };
        let mut out = NvsArray::new();
        while let Some(slot) = subject.next_slot(from) {
            copy_entry(&subject, slot, &mut out);
            from = slot + 1;
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::withoutLast(array<T> $a): array<T>` — every entry but the
    /// last, replacing what is left of the subject after PHP's `array_pop`.
    ///
    /// [`nvs_core_arr_without_first`]'s own docs own the split from PHP's
    /// mutate-and-return pair and the rule that every surviving key is kept.
    ///
    /// The last entry cannot be named without walking, since the ordered hash
    /// has no backward cursor — the same property
    /// [`nvs_core_arr_reverse`] collects slots for. This one needs no `Vec`:
    /// it copies each entry only once it has seen that another follows.
    fn nvs_core_arr_without_last(_ctx, args: [1]) {
        // Unreachable from source: an `array<T>` parameter, refused at the
        // checker — [`nvs_core_arr_count`]'s guard states the judgement.
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::withoutLast expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let subject = borrowed(array);

        let mut out = NvsArray::new();
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
) -> Result<(std::mem::ManuallyDrop<NvsArray>, u64, Value), Fault> {
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

nvs_runtime::nvs_helper! {
    /// `Core\Arr::padStart(array<T> $a, uint $size, T $value): array<T>` —
    /// enough copies of `$value` in front to reach `$size` entries, replacing
    /// PHP's `array_pad` with a *negative* length.
    ///
    /// **The result is always a list**, whatever the subject's keys were, and
    /// that is the one place these two members diverge from
    /// [`nvs_core_arr_without_first`]'s rule that every surviving key is kept.
    /// The difference is that padding *adds* entries, and there is no
    /// non-arbitrary key for an added one beside an existing map's keys —
    /// prepending to `["a" => 1]` would have to invent `"0"`, which the
    /// subject may already hold. PHP resolves that by renumbering the integer
    /// keys and keeping the string ones, which is exactly the
    /// key-type-dependent rule
    /// `rule:types/preserve-keys` removes; renumbering *every* key is the same answer applied
    /// uniformly, and it is what spec § 2 says `{preserveKeys: false}` means
    /// wherever the option appears. Neither member declares the option,
    /// because keeping a key here is not a choice that can be offered.
    ///
    /// A subject already at or past `$size` is returned as a list of its own
    /// values, unpadded — PHP returns it unchanged, and the difference is only
    /// the keys, which this member has already said it does not keep. `$size`
    /// is a `uint`, so PHP's "negative length pads on the left" overload is a
    /// member name here rather than a sign.
    fn nvs_core_arr_pad_start(_ctx, args: [3]) {
        let (subject, missing, value) = padding(args, "padStart")?;
        let mut out = NvsArray::new();
        append_copies(&mut out, value, missing, "Core\\Arr::padStart")?;
        append_values(&subject, &mut out);
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::padEnd(array<T> $a, uint $size, T $value): array<T>` —
    /// enough copies of `$value` after the last entry to reach `$size`,
    /// replacing PHP's `array_pad` with a positive length.
    ///
    /// [`nvs_core_arr_pad_start`]'s own docs own why the result is always a
    /// list and why neither member takes a `preserveKeys` option. The two are
    /// the same walk with the padding on the other side.
    fn nvs_core_arr_pad_end(_ctx, args: [3]) {
        let (subject, missing, value) = padding(args, "padEnd")?;
        let mut out = NvsArray::new();
        append_values(&subject, &mut out);
        append_copies(&mut out, value, missing, "Core\\Arr::padEnd")?;
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
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
    /// `rule:programs/memory-priority`'s ordering buys without discussion.
    fn nvs_core_arr_reverse(_ctx, args: [2]) {
        // Unreachable from source: an `array<T>` parameter, refused at the
        // checker — [`nvs_core_arr_count`]'s guard states the judgement.
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

        let mut out = NvsArray::new();
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

nvs_runtime::nvs_helper! {
    /// `Core\Arr::flip(array<int|string> $a): array<string>` — each value
    /// becomes a key and each key becomes a value, replacing PHP's
    /// `array_flip`.
    ///
    /// Duplicate values collapse, **the last occurrence winning**, which is
    /// PHP's rule and the spec's § 2 *Structure* note. The result is keyed in
    /// first-occurrence order all the same, because a re-`set` of an existing
    /// key overwrites in place rather than moving the entry to the end —
    /// `rule:types/arrays`'s insertion order is a property of the *key*, not of the
    /// most recent write.
    ///
    /// The values become keys through [`key_bytes`], the same normalization
    /// `hasKey` uses, so an `int` value and its decimal spelling flip to the
    /// same key. A value of any other type is the checker having let an
    /// `array<int|string>` position take something else, and is reported as
    /// such rather than skipped the way PHP's warning-and-continue does.
    fn nvs_core_arr_flip(_ctx, args: [1]) {
        // Unreachable from source: an `array<int|string>` parameter, refused
        // at the checker — [`nvs_core_arr_count`]'s guard states the
        // judgement. The *element* rule is a different question and the guard
        // below this one answers it.
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::flip expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let subject = borrowed(array);

        let mut out = NvsArray::new();
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
            out.set(NvsStr::new(&key_bytes(&value, "flip")?), Value::str(key));
            from = slot + 1;
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::flatten(array<array<T>> $a): array<T>` — one level of
    /// nesting removed, replacing a hand-written walk.
    ///
    /// **A list, always**: every inner array's keys are discarded and the
    /// result renumbers from `"0"`. Two inner arrays can hold the same key,
    /// so there is no key rule that keeps both, and
    /// `rule:types/preserve-keys` refuses the one PHP would reach for — keep the strings, renumber
    /// the integers. `appendAll` is this member's variadic sibling and answers
    /// the same shape for the same reason.
    ///
    /// The declared parameter is `array<array<T>>` rather than the spec's
    /// original `array<T>`, and that is the whole of the pair's type story:
    /// unwrapping *one* level is exactly what a nested element type can state,
    /// so this member keeps `T` all the way to its answer. It is
    /// [`nvs_core_arr_flatten_deep`] that cannot, and its own docs say why.
    fn nvs_core_arr_flatten(_ctx, args: [1]) {
        let base = subject(args, "flatten")?;
        let mut out = NvsArray::new();
        let mut from = 0usize;
        while let Some(slot) = base.next_slot(from) {
            from = slot + 1;
            let value = base
                .value_at(slot)
                .expect("next_slot only names live entries");
            let inner = array_at(&value, "flatten", "every element")?;
            append_values(&inner, &mut out);
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::flattenDeep(array<mixed> $a): array<mixed>` — every level of
    /// nesting removed, replacing a hand-written recursive walk and
    /// `iterator_to_array` over a `RecursiveIteratorIterator`.
    ///
    /// A list, for [`nvs_core_arr_flatten`]'s reason. It takes no depth count,
    /// because every real call means one level or all of them, which is what
    /// makes this and `flatten` two members rather than one with an argument —
    /// the same pairing `overlay`/`overlayDeep` already is.
    ///
    /// **`mixed` on both sides, and that is the honest type rather than a
    /// weak one.** An unbounded depth has no element type to state: an
    /// `array<array<T>>` parameter — [`nvs_core_arr_flatten`]'s, which is
    /// exact — binds `T` to `array<U>` when the argument is three deep, and
    /// the return would then claim one more level of nesting than the answer
    /// has. That is unsound, not merely imprecise, so this member erases
    /// instead (`rule:types/grammar`'s one unchecked position). A caller that knows the depth is two
    /// uses `flatten` and keeps its `T`; the spec's rows say both.
    ///
    /// An explicit stack rather than recursion: the nesting depth is the
    /// caller's data, and a deeply nested argument must not be able to reach
    /// the host stack's limit. An array cannot contain itself — it is a
    /// copy-on-write value, so `$a[] = $a` stores a copy — which is what makes
    /// the walk terminate with no visited set.
    fn nvs_core_arr_flatten_deep(_ctx, args: [1]) {
        let base = subject(args, "flattenDeep")?;
        let mut out = NvsArray::new();
        let mut stack = vec![(base, 0usize)];
        while let Some((array, from)) = stack.last_mut() {
            let Some(slot) = array.next_slot(*from) else {
                stack.pop();
                continue;
            };
            *from = slot + 1;
            let value = array
                .value_at(slot)
                .expect("next_slot only names live entries");
            match value.array_ptr() {
                Some(nested) => stack.push((borrowed(nested), 0)),
                None => append_borrowed(&mut out, value),
            }
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::column(array<array<T>> $a, int|string $column, {indexBy?: int|string}): array<T>`
    /// — one named cell out of every row, replacing PHP's `array_column`.
    ///
    /// The parameter is [`nvs_core_arr_flatten`]'s `array<array<T>>`, for that
    /// member's reason: unwrapping exactly one level is what a nested element
    /// type can state, so a column's values keep `T` all the way to the
    /// answer. Both key arguments go through [`key_bytes`], since either may
    /// be an `int` — `column($rows, 0)` and `column($rows, "0")` name one
    /// column, exactly as `$row[0]` and `$row["0"]` do.
    ///
    /// **Three questions the spec's row does not answer**, the first two
    /// settled PHP's way because a program porting `array_column` meets them
    /// on its first call and a different answer would be a silent behaviour
    /// change rather than a diagnosed one:
    ///
    /// - A row that **does not have** the named column is skipped — not a
    ///   fault, and not a hole in the answer. `array_column`'s subject is a
    ///   list of result rows, where a missing column means that row is not
    ///   one of the rows asked about; a fault would make the member unusable
    ///   over a ragged list, and a `null` filler would force a `?T` return
    ///   that every non-ragged call — which is nearly all of them — would
    ///   then have to unwrap.
    /// - A row **missing the `indexBy` key** while having the column still
    ///   contributes its value, under the next integer key — the next *free*
    ///   one, since it is `NvsArray::append`'s counter and a row keyed `"5"`
    ///   moves it past 5. So a partially keyed subject loses nothing, unless
    ///   the integer appended under is itself a later row's `indexBy` cell,
    ///   where the collapse rule below applies like any other duplicate.
    /// - An `indexBy` cell that is **not an `int|string`** is a fault, which
    ///   is where PHP renumbers instead. The bullet above already spends the
    ///   "append it" answer on an *absent* key, so reusing it here would make
    ///   a mistyped `indexBy` indistinguishable from a ragged subject — and
    ///   the declared option type says the caller means a key.
    ///
    /// `{indexBy}` collapses duplicates, the last row winning: that is
    /// `NvsArray::set`'s rule and `flip`'s, not this member's opinion.
    fn nvs_core_arr_column(_ctx, args: [3]) {
        let base = subject(args, "column")?;
        let column = key_bytes(&args[1], "column")?;
        let index_by = match args[2].tag() {
            Some(Tag::Null) => None,
            _ => Some(key_bytes(&args[2], "column")?),
        };

        let mut out = NvsArray::new();
        let mut from = 0usize;
        while let Some(slot) = base.next_slot(from) {
            from = slot + 1;
            let value = base
                .value_at(slot)
                .expect("next_slot only names live entries");
            let row = array_at(&value, "column", "every row")?;
            let Some(cell) = row.get(&column) else {
                continue;
            };
            match index_by.as_ref().and_then(|key| row.get(key)) {
                Some(key) => {
                    let key = NvsStr::new(&key_bytes(&key, "column")?);
                    #[expect(
                        unsafe_code,
                        reason = "the cell is owned by the subject's row, which \
                                  outlives this call, so the copy stored here \
                                  needs a reference of its own"
                    )]
                    unsafe {
                        cell.retain();
                    }
                    out.set(key, cell);
                }
                None => append_borrowed(&mut out, cell),
            }
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::values(array<T> $a): array<T>` — the values in insertion
    /// order under fresh `0, 1, …` keys, replacing PHP's `array_values`.
    ///
    /// Each value belongs to the subject array, which outlives this call, so
    /// the copy stored here takes a reference of its own — the opposite of
    /// [`nvs_core_arr_map`], whose values are the callback's and are already
    /// owned. `NvsArray::append` is what assigns the new keys, so this member
    /// states no key rule of its own.
    fn nvs_core_arr_values(_ctx, args: [1]) {
        // Unreachable from source: an `array<T>` parameter, refused at the
        // checker — [`nvs_core_arr_count`]'s guard states the judgement.
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::values expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let subject = borrowed(array);

        let mut out = NvsArray::new();
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

nvs_runtime::nvs_helper! {
    /// `Core\Arr::fill(uint $count, T $value): array<T>` — `$count` copies of
    /// one value under `0, 1, …`, replacing PHP's `array_fill`.
    ///
    /// **PHP's `$start_index` is dropped.** `array_fill(5, 3, 'v')` produces
    /// keys `5, 6, 7`, which is a list that does not start at zero — a shape
    /// [`nvs_core_arr_is_list`] answers `false` for, and one every caller then
    /// has to reason about. The two real uses of the parameter are covered
    /// without it: `0` is this member, and any other keys are
    /// [`nvs_core_arr_fill_keys`] over the keys the caller actually wants.
    ///
    /// `$count` of zero yields an empty array rather than throwing, which is
    /// PHP's own answer and the one that lets a computed count through
    /// unguarded.
    fn nvs_core_arr_fill(_ctx, args: [2]) {
        // Unreachable from source for `nvs_core_arr_chunk`'s `size` reason and
        // on the same two probes: parameter 0 is `CoreTy::Uint` in `CLASS`
        // above, so both a `mixed` binding and the literal `-1` are `E0401`
        // before any of this runs. The zero the doc comment above admits is a
        // `uint` and reaches the body; nothing else does.
        let count = args[0].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::fill expected {:?} for `count`, got tag {}",
                Tag::Uint,
                args[0].tag_byte()
            ))
        })?;
        let mut out = NvsArray::new();
        append_copies(&mut out, args[1], count, "Core\\Arr::fill")?;
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::fillKeys(array<int|string> $keys, T $value): array<T>` — one
    /// value stored under every key named, replacing PHP's `array_fill_keys`.
    ///
    /// The `$keys` array contributes its *values* and nothing else, exactly as
    /// [`nvs_core_arr_from_keys_and_values`] takes its keys, and each goes
    /// through [`key_bytes`] — so `1` and `"1"` name one entry, and a duplicate
    /// key collapses in its first occurrence's position. Every entry holds the
    /// same value, so which of two duplicates "wins" is not observable; what is
    /// observable is the count, and it counts distinct keys the way PHP's does.
    fn nvs_core_arr_fill_keys(_ctx, args: [2]) {
        // Unreachable from source: parameter 0 is
        // `CoreTy::Array(CoreTy::Union(ARRAY_KEY))` in `CLASS` above, so a
        // non-container argument is `E0401: expected 'array<string|int>',
        // found 'mixed'` at the checker. This is `nvs_core_arr_count`'s
        // judgement, which states it in full.
        let keys = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::fillKeys expected {:?} for the keys, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let keys = borrowed(keys);

        let mut out = NvsArray::new();
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
            out.set(NvsStr::new(&bytes), args[1]);
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::range(int $start, int $end, {step?: int}): array<int>` — the
    /// integers from `$start` to `$end` inclusive, replacing PHP's `range`.
    ///
    /// The first member with an `rule:core-api/shape-rules` R2 options bag, and therefore the
    /// first whose arity says something the spec's signature does not:
    /// `{step?: int}` is flattened into one ordinary argument by
    /// `nvs_ir::lower::lower_call_args`, so this is an `args: [3]` helper and
    /// `args[2]` is always present — the call site materialized the default
    /// where it was not written. `nvs_stdlib::registry`'s own docs own why.
    ///
    /// Three behaviours, all PHP 8.5's and all verified against it:
    ///
    /// * **The direction comes from the arguments, not the step.** `$start >
    ///   $end` counts down; there is no negative step.
    /// * **A `step` of zero or less throws**, rather than looping forever or
    ///   silently reversing. PHP raises `ValueError`; this raises the tree's
    ///   root, because a `Core` helper cannot yet name the class it throws
    ///   (`nvs_runtime::Ctx::set_runtime_error_class`) — spec § 10's
    ///   `LogicError` is the class this owes once it can.
    /// * **A step that overshoots stops at the last in-range value**, so
    ///   `range(1, 10, {step: 3})` is `1, 4, 7, 10` and
    ///   `range(1, 10, {step: 4})` is `1, 5, 9`.
    ///
    /// The cursor advances by `checked_add`/`checked_sub` rather than by
    /// multiplying an index: a range whose next step would leave `int` stops
    /// instead of wrapping, which is `rule:types/arithmetic`'s rule applied to a loop
    /// this member owns rather than to arithmetic a program wrote.
    ///
    /// **The length is the bounds', so `[limits] memory` is what bounds it**,
    /// and the loop asks [`nvs_runtime::affordable`] per entry for the reason
    /// `nvs_runtime::sequence::drain` does: a native loop passes no statement
    /// boundary and calls no member, so nothing between two appends would
    /// otherwise read the balance and a wide enough span would hold the host's
    /// memory rather than the request's ceiling — which
    /// `rule:programs/memory-priority` does not permit. The refusal is the
    /// `FATAL` `rule:errors/on-limit` gives every resource limit.
    fn nvs_core_arr_range(_ctx, args: [3]) {
        let start = integer(&args[0], "range", "the start")?;
        let end = integer(&args[1], "range", "the end")?;
        let step = integer(&args[2], "range", "the `step` option")?;
        if step <= 0 {
            return Err(Fault::thrown(format!(
                "Core\\Arr::range(): the `step` option must be greater than 0, got {step}"
            )));
        }

        let mut out = NvsArray::new();
        let mut cursor = start;
        loop {
            // Asked in front of the append, so the entry that would cross the
            // ceiling is never stored: the balance already carries every entry
            // this loop has appended, and this is the only thing in it that
            // reads that balance.
            nvs_runtime::affordable(Some(size_of::<Value>()), r"Core\Arr::range")?;
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

nvs_runtime::nvs_helper! {
    /// `Core\Arr::from(Iterable<T>|Iterator<T>|array<T> $items, {limit?: uint}):
    /// array<T>` — a sequence materialised, replacing `iterator_to_array` and
    /// the materialising half of `iterator_count`.
    ///
    /// **Always a list**, whatever it drained: a cursor has no keys at all
    /// (`rule:iteration/two-interfaces` gives `Iterator<T>` exactly `advance` and `current`), so
    /// answering with the argument's keys where it happens to have some would
    /// make the result's shape depend on which of the three shapes was passed.
    /// An array argument's keys are therefore discarded exactly as
    /// [`nvs_core_arr_values`] discards them, which is also PHP's
    /// `iterator_to_array($it, false)` and the only reading that composes.
    ///
    /// **`{limit: n}` stops after `n` elements** and an omitted limit does not
    /// stop at all — the spec's § 2 prose calls this the only guard against
    /// materialising an unbounded generator, so it is the *drive* that stops
    /// rather than the result being truncated afterwards: a generator asked
    /// for three elements runs three segments of its body and no more.
    ///
    /// The three shapes are decoded by tag and the cursor is driven by name in
    /// `nvs_runtime::sequence`, whose module doc owns both halves and what
    /// they cost. Every drained element arrives owned, so each is stored
    /// without a retain — the same convention [`nvs_core_arr_map`] follows for
    /// a value its callback produced.
    fn nvs_core_arr_from(ctx, args: [2]) {
        // An omitted `{limit}` is `Const::Null`, which is *no* limit; a
        // written one is a `uint` that cannot exceed what a `Vec` can hold on
        // any target this runs on.
        let limit = args[1]
            .as_uint()
            .map(|limit| usize::try_from(limit).unwrap_or(usize::MAX));
        let drained = nvs_runtime::sequence::drain(ctx, args[0], limit, r"Core\Arr::from")?;

        let mut out = NvsArray::new();
        for value in drained {
            out.append(value);
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::fromKeysAndValues(array<int|string> $keys, array<T> $values): array<T>`
    /// — one array's values used as the keys of another's, replacing PHP's
    /// `array_combine`.
    ///
    /// **Two arrays of different lengths throw**, which is the spec's § 2
    /// *Structure* note and `rule:core-api/shape-rules` R4: PHP 8 raises a `ValueError` here, and
    /// the shorter-wins alternative would silently drop data. The two are
    /// walked by their own cursors, so it is each array's *entry count* that
    /// has to match — neither side's keys are looked at, and the `$keys`
    /// array's own keys are discarded exactly as [`nvs_core_arr_values`]
    /// discards them.
    ///
    /// Each key goes through [`key_bytes`], the normalization `hasKey` and
    /// `flip` share, so an `int` key and its decimal spelling name the same
    /// entry. Duplicate keys collapse with the **last** value winning, in the
    /// **first** occurrence's position — `flip`'s rule, and for the same reason:
    /// re-`set`ting an existing key overwrites in place.
    fn nvs_core_arr_from_keys_and_values(_ctx, args: [2]) {
        let keys = array_at(&args[0], "fromKeysAndValues", "the keys")?;
        let values = array_at(&args[1], "fromKeysAndValues", "the values")?;
        if keys.count() != values.count() {
            return Err(Fault::thrown(format!(
                "Core\\Arr::fromKeysAndValues(): the two arrays must be the same length, \
                 got {} against {}",
                keys.count(),
                values.count()
            )));
        }

        let mut out = NvsArray::new();
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
            out.set(NvsStr::new(&bytes), value);
            key_from = key_slot + 1;
            value_from = value_slot + 1;
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::countBy(array<T> $a, {by?: callable}): array<uint>` — how
    /// many entries fall under each distinct value, replacing PHP's
    /// `array_count_values` and the userland group-and-count loop.
    ///
    /// Without `by` the value itself is the bucket, which is
    /// `array_count_values`; with it, the callback names the bucket and
    /// receives `($value, $key)` like every other `Core\Arr` callback, which is
    /// the half PHP has no function for at all. The result is keyed in
    /// **first-occurrence** order and its values are `uint`, for the reason
    /// [`nvs_core_arr_count`] returns one: a count cannot be negative.
    ///
    /// A bucket is named through [`key_bytes`], so `1` and `"1"` count as one
    /// bucket — the same normalization every other key position uses. A value
    /// that is neither an `int` nor a `string` and has no `by` to name it is a
    /// throw rather than PHP's warn-and-skip, which is `flip`'s treatment of
    /// the identical situation.
    fn nvs_core_arr_count_by(ctx, args: [2]) {
        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::countBy expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let by = optional_callback(&args[1], "countBy", "by")?;
        let subject = borrowed(array);

        let mut out = NvsArray::new();
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
                    let named = nvs_runtime::call_closure(ctx, callback, &[value, key_arg]);
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
            out.set(NvsStr::new(&bucket), Value::uint(seen + 1));
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::reduce(array<T> $a, callable $fn, U $initial): U` — the
    /// spec's § 2 *Iteration and aggregation* fold, replacing `array_reduce`.
    ///
    /// # The callback takes the carry first
    ///
    /// `($carry, $value, $key)`, and may declare fewer parameters (R9). That
    /// extends § 2's "every callback receives `($value, $key)`" with a leading
    /// carry rather than departing from it: the pair is still there, in the
    /// same order, and a fold has nowhere else to put the accumulator that
    /// every language — `array_reduce` included — writes first.
    ///
    /// # `U` is bound by `$initial`, not by the callback
    ///
    /// [`CoreTy::Var`] infers a variable by unifying the declared
    /// parameters against the call's arguments, and `$initial` is a real value
    /// at a real argument position, so it answers `U` on its own. That is why
    /// the callback's own written return type is `U` and not a second
    /// variable: `map` reads what it produces off the callback because nothing
    /// else in that signature knows it, while here a second site for one
    /// variable would leave "first binding wins" to settle by accident which
    /// of the two is the answer.
    ///
    /// The consequence a caller sees is that the fold's type is the type of
    /// the seed. `reduce($ints, $fn, 0)` is an `int` whatever the callback
    /// returns, and a fold that means to build a string starts from `""`.
    ///
    /// # An empty array is `$initial`
    ///
    /// Returned unchanged, with no call made — the identity every fold needs,
    /// and the reason this member has no "empty" case to throw on the way
    /// [`nvs_core_arr_sum`] must reason about one.
    fn nvs_core_arr_reduce(ctx, args: [3]) {
        let base = subject(args, "reduce")?;

        // The carry is the one value this frame owns across the whole walk.
        // `$initial` belongs to the caller, so it takes a reference of its own
        // here and the guard below owns exactly one from then on: each round
        // swaps in the callback's fresh answer and releases what it replaced.
        #[expect(
            unsafe_code,
            reason = "`$initial` belongs to the caller, and what this frame \
                      hands back is a reference of its own"
        )]
        unsafe {
            args[2].retain();
        }
        let mut carry = Extracted(vec![args[2]]);

        // Read once for the whole fold: the key sits at the third position, so
        // a callback declaring `($carry, $value)` — the shape a sum is written
        // in — never sees one and none is built. § D, and
        // [`nvs_core_arr_filter`] is the same reading two positions earlier.
        let wants_key = nvs_runtime::closure_arity(args[1])? >= 3;

        let mut from = 0usize;
        while let Some(slot) = base.next_slot(from) {
            from = slot + 1;
            let value = base
                .value_at(slot)
                .expect("next_slot only names live entries");

            let next = if wants_key {
                // One reference for the duration of the call, released right
                // after — `call_closure` takes its own. Nothing else in a fold
                // wants the key, so this is the only place it is built.
                let key_arg = Value::str(
                    base.slot_key(slot)
                        .expect("next_slot only names live entries")
                        .to_str(),
                );
                let next = nvs_runtime::call_closure(ctx, args[1], &[carry.0[0], value, key_arg]);
                #[expect(
                    unsafe_code,
                    reason = "this frame owns exactly the reference \
                              `slot_key(..).to_str()` just produced"
                )]
                unsafe {
                    key_arg.release();
                }
                next
            } else {
                nvs_runtime::call_closure(ctx, args[1], &[carry.0[0], value])
            };
            // Unwrapped after the key is released and while the guard still
            // holds the current carry, so a throwing callback frees both.
            let next = next?;
            let previous = std::mem::replace(&mut carry.0[0], next);
            #[expect(
                unsafe_code,
                reason = "the reference this frame owned until the line above"
            )]
            unsafe {
                previous.release();
            }
        }

        // Taken *out* of the guard, so its `Drop` frees nothing: the reference
        // it was holding is the one the caller receives.
        Ok(carry
            .0
            .pop()
            .expect("the carry is pushed once and only replaced in place"))
    }
}

/// What a callback produced, owned by the frame that called it —
/// [`nvs_core_arr_sort`]'s sort keys, [`nvs_core_arr_unique`]'s identity keys
/// and [`nvs_core_arr_reduce`]'s carry, which are one obligation under three
/// names.
///
/// `nvs_runtime::call_closure` hands back one fresh reference per call, so the
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

nvs_runtime::nvs_helper! {
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
    /// # The sort is stable, and there are two of them
    ///
    /// Stable, like every PHP 8 sort. Without a `comparator`, a subject whose
    /// compared values all sit in one row of [`compare_values`]'s table — all
    /// `int`/`uint`, all `float`, all `string`, all `bool` or all `null` — can
    /// never throw, and [`crate::sort`] hands it to `brainsort` as a key sort;
    /// that module owns the key mapping, the proof that the permutation is
    /// the same one, and what the call spends. Everything else runs the
    /// hand-written bottom-up [`merge_sort`] over an index permutation rather
    /// than `slice::sort_by`, for one reason: a comparison there can **fail**
    /// — the callback can throw, and two values of incomparable types are a
    /// throw of this member's own. Rust's sorts take an infallible comparator,
    /// so the alternatives were swallowing the fault until the sort finished
    /// (leaving a comparator that is no longer a total order, which those
    /// sorts are documented to be allowed to panic on) or this. It costs one
    /// `Vec<usize>` of scratch space, which
    /// `rule:programs/memory-priority`'s ordering buys without discussion.
    ///
    /// # Natural ordering, and where it diverges from PHP
    ///
    /// [`compare_values`] owns the table. The one deliberate divergence:
    /// **two `string`s always compare bytewise**, never numerically. PHP
    /// compares `"10"` and `"9"` as numbers, which is the same
    /// changes-type-by-itself behaviour
    /// `rule:types/declaration` rejects
    /// everywhere else; a caller who wants a numeric order over numeric
    /// strings writes `{by: ...}` and says so.
    ///
    /// An `array<T>` of objects sorts under
    /// `rule:classes/comparable`'s `compareTo`,
    /// which [`compare_values`] reaches through the class's own descriptor —
    /// a class that does not implement `Comparable` is the throw naming the
    /// interface, exactly as `<` on two of its instances is a compile error.
    /// Objects are never the key sort's row, so they always take
    /// [`merge_sort`], whose comparison is the fallible one they need.
    fn nvs_core_arr_sort(ctx, args: [5]) {
        let base = subject(args, "sort")?;
        let by = optional_callback(&args[1], "sort", "by")?;
        let descending = match args[2].as_int() {
            Some(0) => false,
            Some(1) => true,
            _ => {
                // Unreachable from source: `order` is
                // `CoreTy::Enum(r"Core\Order")` in `SORT_OPTIONS`, so a value
                // that is no case is `E0401: expected 'Core\Order', found …`
                // at the checker — probed with a `mixed` binding and with the
                // bare `int` literal `1`, which is the near miss worth
                // checking because the discriminant compiled code writes for a
                // case is exactly an integer. `Core\Str::normalize`'s
                // `normal_form_of` is the same judgement, stated in full.
                return Err(Fault::fatal(format!(
                    "Core\\Arr::sort expected a `Core\\Order` case for `order`, got tag {} \
                     value {}",
                    args[2].tag_byte(),
                    args[2].bits()
                )));
            }
        };
        let comparator = optional_callback(&args[3], "sort", "comparator")?;
        // Unreachable from source: `preserveKeys` is `CoreTy::Bool` in
        // `SORT_OPTIONS`, so `E0401: expected 'bool', found 'mixed'` refuses
        // the call before the option list compiled code writes is built.
        let preserve_keys = args[4].as_bool().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::sort expected {:?} for `preserveKeys`, got tag {}",
                Tag::Bool,
                args[4].tag_byte()
            ))
        })?;

        // Whether anything at all is going to look at a key: the result keeps
        // them, or a `by` closure declared a parameter to receive one. A sort
        // that renumbers and extracts by value alone — `sort($list)`, the
        // common call — reads none, so this walk collects none.
        // `docs/perf/userland-gap.md` § D's second paragraph.
        let by_wants_key = match by {
            Some(by) => nvs_runtime::closure_arity(by)? >= 2,
            None => false,
        };
        let needs_keys = preserve_keys || by_wants_key;

        // Every entry, in insertion order. Both halves are *borrowed* from the
        // subject: a `SlotKey::Str` is released by its own `NvsStr` drop, and
        // the values belong to the array, which outlives this call.
        let mut keys: Vec<SlotKey> = Vec::new();
        let mut values: Vec<Value> = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = base.next_slot(from) {
            from = slot + 1;
            if needs_keys {
                keys.push(
                    base.slot_key(slot)
                        .expect("next_slot only names live entries"),
                );
            }
            values.push(
                base.value_at(slot)
                    .expect("next_slot only names live entries"),
            );
        }

        // Decorate. Dropped by the guard on every exit path below, including
        // a throw out of the extractor itself.
        let mut sort_keys = Extracted(Vec::new());
        if let Some(by) = by {
            for (index, value) in values.iter().enumerate() {
                let extracted = if by_wants_key {
                    let key_arg = Value::str(keys[index].to_str());
                    let extracted = nvs_runtime::call_closure(ctx, by, &[*value, key_arg]);
                    #[expect(
                        unsafe_code,
                        reason = "this frame owns exactly the reference \
                                  `keys[index].to_str()` just produced"
                    )]
                    unsafe {
                        key_arg.release();
                    }
                    extracted
                } else {
                    nvs_runtime::call_closure(ctx, by, &[*value])
                };
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
                    let verdict = nvs_runtime::call_closure(
                        ctx,
                        comparator,
                        &[compared[left], compared[right]],
                    )?;
                    let sign = comparator_sign(verdict, "sort");
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
                None => {
                    compare_values(ctx, &compared[left], &compared[right], r"Core\Arr::sort")?
                }
            };
            Ok(if descending { ordering.reverse() } else { ordering })
        };
        // The key sort where it applies, the merge sort otherwise — and a
        // comparator is always the merge sort, because it can throw.
        let sorted = comparator.is_none()
            && crate::sort::natural(&mut permutation, compared, descending);
        if !sorted {
            merge_sort(&mut permutation, &mut compare)?;
        }

        let mut out = NvsArray::new();
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
                store_at(&mut out, keys[index].clone(), values[index]);
            } else {
                out.append(values[index]);
            }
        }
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::sortByKey(array<T> $a, {order?: Order, comparator?: callable}): array<T>`
    /// — the spec's § 2 *Ordering* second member, replacing `ksort`, `krsort`
    /// and `uksort`.
    ///
    /// A second entry point into [`nvs_core_arr_sort`]'s machinery rather than
    /// a second sort: the same two stable sorts over an index permutation —
    /// [`crate::sort`]'s key sort over the keys' bytes when no comparator was
    /// given, the [`merge_sort`] under one — with the *keys* compared instead
    /// of a `by` closure's answers. What differs is only what the comparison
    /// reads.
    ///
    /// # Why the bag is two options and not four
    ///
    /// **There is no `preserveKeys`.** A sort by key that renumbered would
    /// have discarded the very thing it ordered on, so this member preserves
    /// keys unconditionally — PHP's `ksort` family has no `k`-less spelling
    /// either. **There is no `by`**, because the thing compared is already the
    /// key; a caller wanting to order by something derived from the key writes
    /// `comparator`, which receives the two keys.
    ///
    /// # The natural order is a byte compare
    ///
    /// `rule:types/arrays` makes
    /// every stored key a `string` — `arr-keys-are-always-strings.nvst` pins
    /// it — so there is no mixed-type case for [`compare_values`] to
    /// arbitrate and the default ordering is `[u8]`'s. That means `"10"`
    /// sorts before `"9"`, which is `ksort`'s `SORT_STRING` behaviour rather
    /// than its default `SORT_REGULAR`: comparing two strings numerically is
    /// the changes-type-by-itself reading [`nvs_core_arr_sort`] rejects for
    /// values, and a key sort is the same question. A caller wanting numeric
    /// order over numeric keys writes a `comparator` and says so.
    ///
    /// `order` reverses the comparison rather than the result, so equal keys
    /// keep their insertion order under either — the same rule, and the same
    /// stability, as the value sort.
    fn nvs_core_arr_sort_by_key(ctx, args: [3]) {
        let base = subject(args, "sortByKey")?;
        let descending = match args[1].as_int() {
            Some(0) => false,
            Some(1) => true,
            _ => {
                // Unreachable from source for the reason `nvs_core_arr_sort`'s
                // own `order` arm states, on the same two probes: this
                // option is `CoreTy::Enum(r"Core\Order")` in
                // `SORT_BY_KEY_OPTIONS`, and both a `mixed` binding and the
                // bare `int` literal `1` are `E0401` at the checker.
                return Err(Fault::fatal(format!(
                    "Core\\Arr::sortByKey expected a `Core\\Order` case for `order`, got tag {} \
                     value {}",
                    args[1].tag_byte(),
                    args[1].bits()
                )));
            }
        };
        let comparator = optional_callback(&args[2], "sortByKey", "comparator")?;

        // Every entry, in insertion order. Each key is a reference of this
        // frame's own, released by its `NvsStr` drop; each value is borrowed
        // from the subject, which outlives the call.
        let mut keys: Vec<NvsStr> = Vec::new();
        let mut values: Vec<Value> = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = base.next_slot(from) {
            from = slot + 1;
            keys.push(
                base.key_at(slot)
                    .expect("next_slot only names live entries"),
            );
            values.push(
                base.value_at(slot)
                    .expect("next_slot only names live entries"),
            );
        }

        let mut permutation: Vec<usize> = (0..keys.len()).collect();
        let mut compare = |left: usize, right: usize| -> Result<std::cmp::Ordering, Fault> {
            let ordering = match comparator {
                Some(comparator) => {
                    // One reference each for the duration of the call: the
                    // keys belong to `keys`, and a comparator called
                    // `n log n` times would otherwise leak that many.
                    let left_arg = Value::str(keys[left].clone());
                    let right_arg = Value::str(keys[right].clone());
                    let verdict =
                        nvs_runtime::call_closure(ctx, comparator, &[left_arg, right_arg]);
                    #[expect(
                        unsafe_code,
                        reason = "this frame owns exactly the two references \
                                  the clones above just produced"
                    )]
                    unsafe {
                        left_arg.release();
                        right_arg.release();
                    }
                    let verdict = verdict?;
                    let sign = comparator_sign(verdict, "sortByKey");
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
                None => keys[left].as_bytes().cmp(keys[right].as_bytes()),
            };
            Ok(if descending { ordering.reverse() } else { ordering })
        };
        if comparator.is_some() {
            merge_sort(&mut permutation, &mut compare)?;
        } else {
            crate::sort::by_bytes(&mut permutation, &keys, descending);
        }

        let mut out = NvsArray::new();
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
            out.set(keys[index].clone(), values[index]);
        }
        Ok(Value::array(out))
    }
}

/// One optional callback option: the closure it names, or `None` for the
/// `Tag::Null` an omitting call site passes.
///
/// `nvs_stdlib::registry::Const::Null` owns why "not given" is spelled that
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

/// A comparator's verdict as an [`std::cmp::Ordering`], for a member of this
/// class — [`crate::ordering::comparator_sign`] qualified with the class name,
/// which every call site here would otherwise spell out.
fn comparator_sign(verdict: Value, member: &str) -> Result<std::cmp::Ordering, Fault> {
    crate::ordering::comparator_sign(verdict, &format!("Core\\Arr::{member}"))
}

/// A stable, bottom-up merge sort over `permutation`, with a comparison that
/// may fail.
///
/// Bottom-up rather than recursive so the scratch buffer is allocated once,
/// and over an index permutation rather than the values so nothing is moved
/// twice. [`nvs_core_arr_sort`] owns why this exists at all instead of
/// `slice::sort_by`.
pub(crate) fn merge_sort<F>(permutation: &mut [usize], compare: &mut F) -> Result<(), Fault>
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
fn subject(args: &[Value], member: &str) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
    let array = args[0].array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Arr::{member} expected {:?}, got tag {}",
            Tag::Array,
            args[0].tag_byte()
        ))
    })?;
    Ok(borrowed(array))
}

/// [`subject`] for an `array` parameter that is **not** the first — named by
/// its position, since the message can no longer say "the subject" and mean
/// something.
///
/// Three members take a second array today (`fromKeysAndValues`'s two,
/// `replaceRange`'s replacement), which is what makes this one function rather
/// than the same seven lines each.
fn array_at(
    value: &Value,
    member: &str,
    position: &str,
) -> Result<std::mem::ManuallyDrop<NvsArray>, Fault> {
    let array = value.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "Core\\Arr::{member} expected {:?} for {position}, got tag {}",
            Tag::Array,
            value.tag_byte()
        ))
    })?;
    Ok(borrowed(array))
}

/// The value at `slot` as a fresh reference this frame owns — what a `?T`
/// answer has to be.
///
/// [`NvsArray::value_at`] *borrows*, and a returned value belongs to the
/// caller, so every one of these members retains before it answers. The
/// [`Value::null`] answer needs no counterpart: `Tag::Null` releases to
/// nothing.
fn owned_value_at(subject: &NvsArray, slot: usize) -> Value {
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
/// [`NvsArray::key_at`] clones.
fn owned_key_at(subject: &NvsArray, slot: usize) -> Value {
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
/// the same rule and the same `nvs_runtime::call_closure` trimming
/// [`nvs_core_arr_filter`] documents; the retain/release around the key
/// argument and around the verdict are that member's too, and for the same
/// reasons.
fn find_slot(
    ctx: &mut nvs_runtime::Ctx,
    subject: &NvsArray,
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
        let verdict = nvs_runtime::call_closure(ctx, predicate, &[value, key]);
        #[expect(
            unsafe_code,
            reason = "this frame owns exactly the reference `key_at` cloned"
        )]
        unsafe {
            key.release();
        }
        let verdict = verdict?;
        let truthy = nvs_runtime::value_truthy(verdict);
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

nvs_runtime::nvs_helper! {
    /// `Core\Arr::first(array<T> $a): ?T` — the value of the first entry in
    /// insertion order, replacing PHP's `reset`, `current` and
    /// `$a[array_key_first($a)]`.
    ///
    /// The first `Core` member to answer `?T`. The empty array is `null`, not
    /// a throw: `rule:core-api/shape-rules` R5 makes `?T` the absence spelling and R4's throw is
    /// for a *failure*, which asking a possibly-empty array for its first
    /// entry is not. The spec's § 2 notes the one thing this costs — over an
    /// `array<?T>` the answer cannot tell "absent" from "present and null",
    /// which `isEmpty` answers directly.
    ///
    /// PHP's three spellings all move or read an internal array pointer;
    /// there is none here, and the spec's § 2 says why a cursor inside a
    /// copy-on-write *value* is incoherent.
    fn nvs_core_arr_first(_ctx, args: [1]) {
        let subject = subject(args, "first")?;
        Ok(match subject.next_slot(0) {
            Some(slot) => owned_value_at(&subject, slot),
            None => Value::null(),
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::last(array<T> $a): ?T` — the value of the last entry in
    /// insertion order, replacing PHP's `end` and `$a[array_key_last($a)]`.
    ///
    /// [`nvs_core_arr_first`]'s own docs own the `null`-for-empty rule.
    ///
    /// The last entry cannot be named without walking, since the ordered hash
    /// has no backward cursor — the same property
    /// [`nvs_core_arr_without_last`] walks for. This one keeps a slot number
    /// rather than copying, so the walk allocates nothing.
    fn nvs_core_arr_last(_ctx, args: [1]) {
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

nvs_runtime::nvs_helper! {
    /// `Core\Arr::firstKey(array<T> $a): ?string` — the first entry's key,
    /// replacing PHP's `array_key_first` and `key`.
    ///
    /// `?string` and never `?int`: `rule:types/arrays` stores every key as a string,
    /// and `Core\Arr::keys` already answers `array<string>` for the same
    /// reason — a member that guessed a key's "original" type would be the
    /// key-type-dependent behaviour `rule:types/array-combination` removes.
    fn nvs_core_arr_first_key(_ctx, args: [1]) {
        let subject = subject(args, "firstKey")?;
        Ok(match subject.next_slot(0) {
            Some(slot) => owned_key_at(&subject, slot),
            None => Value::null(),
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::lastKey(array<T> $a): ?string` — the last entry's key,
    /// replacing PHP's `array_key_last`.
    ///
    /// [`nvs_core_arr_first_key`] owns why the answer is a `string`, and
    /// [`nvs_core_arr_last`] why finding it is a walk.
    fn nvs_core_arr_last_key(_ctx, args: [1]) {
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

nvs_runtime::nvs_helper! {
    /// `Core\Arr::find(array<T> $a, callable $predicate): ?T` — the first
    /// value the predicate answers truthily for, replacing PHP's `array_find`.
    ///
    /// `null` when nothing matches, on [`nvs_core_arr_first`]'s terms. The
    /// walk stops at the first match, so the predicate is called once per
    /// entry *up to* it and never after — which is the property that makes
    /// this different from `filter` plus `first`, and the reason both members
    /// exist.
    fn nvs_core_arr_find(ctx, args: [2]) {
        let subject = subject(args, "find")?;
        Ok(match find_slot(ctx, &subject, args[1], false)? {
            Some(slot) => owned_value_at(&subject, slot),
            None => Value::null(),
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::findKey(array<T> $a, callable $predicate): ?string` — the
    /// key of the first entry the predicate answers truthily for, replacing
    /// PHP's `array_find_key`.
    ///
    /// [`nvs_core_arr_find`] owns the walk; [`nvs_core_arr_first_key`] owns
    /// why the answer is a `string`.
    fn nvs_core_arr_find_key(ctx, args: [2]) {
        let subject = subject(args, "findKey")?;
        Ok(match find_slot(ctx, &subject, args[1], false)? {
            Some(slot) => owned_key_at(&subject, slot),
            None => Value::null(),
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::any(array<T> $a, callable $predicate): bool` — whether any
    /// entry satisfies the predicate, replacing PHP's `array_any`.
    ///
    /// Short-circuits at the first match, and is `false` over an empty array.
    fn nvs_core_arr_any(ctx, args: [2]) {
        let subject = subject(args, "any")?;
        let found = find_slot(ctx, &subject, args[1], false)?;
        Ok(Value::bool(found.is_some()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::all(array<T> $a, callable $predicate): bool` — whether every
    /// entry satisfies the predicate, replacing PHP's `array_all`.
    ///
    /// The same walk as [`nvs_core_arr_any`] against the *negated* predicate:
    /// "every entry matches" is "no entry fails", so it short-circuits at the
    /// first failure. `true` over an empty array, which is the vacuous answer
    /// PHP's own `array_all` gives.
    fn nvs_core_arr_all(ctx, args: [2]) {
        let subject = subject(args, "all")?;
        let failed = find_slot(ctx, &subject, args[1], true)?;
        Ok(Value::bool(failed.is_none()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::contains(array<T> $haystack, T $needle): bool` — whether any
    /// entry is the needle, replacing PHP's `in_array`.
    ///
    /// **Always strict.** `in_array`'s default is a loose comparison, so
    /// `in_array("abc", [0])` is `true` in PHP versions before 8.0 and
    /// `in_array(0, ["a"])` still surprises people; the spec's § 2 table
    /// names this one "(always strict)" and there is no third argument to
    /// forget. What "strict" means is `nvs_runtime::value_identical`, whose
    /// own module docs own every row of it, including the two that differ
    /// from a naive bit comparison (`0.0` and `-0.0` are one value, `NaN` is
    /// identical to nothing) and the one that diverges from PHP's `===`:
    /// `int`, `uint`, `float` and `decimal` are **one numeric domain**, so
    /// `contains([1.0], 1)` is `true` where `in_array(1, [1.0], true)` is
    /// `false`. That is `rule:expressions/equality-semantics`'s numeric row, and taking it here is what
    /// keeps this member and the `==` operator one comparison rather than two.
    fn nvs_core_arr_contains(_ctx, args: [2]) {
        let subject = subject(args, "contains")?;
        Ok(Value::bool(slot_of(&subject, args[1]).is_some()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::keyOf(array<T> $haystack, T $needle): ?string` — the key of
    /// the first entry that is the needle, replacing PHP's `array_search`.
    ///
    /// `?string` rather than `string|false`: `rule:core-api/shape-rules` R5 makes `?T` the one
    /// absence spelling, which is the whole of what removes `array_search`'s
    /// `=== false` trap — a `0` key and a "not found" answer are the same
    /// value under `==` in PHP, and the reason its manual warns to compare
    /// strictly. [`nvs_core_arr_first_key`] owns why a key is a `string`, and
    /// [`nvs_core_arr_contains`] owns what "is the needle" means.
    ///
    /// The *first* match, in insertion order, exactly as `array_search`
    /// answers the first.
    fn nvs_core_arr_key_of(_ctx, args: [2]) {
        let subject = subject(args, "keyOf")?;
        Ok(match slot_of(&subject, args[1]) {
            Some(slot) => owned_key_at(&subject, slot),
            None => Value::null(),
        })
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::unique(array<T> $a, {by?: callable}): array<T>` — the
    /// entries whose value has not been seen before, replacing PHP's
    /// `array_unique`.
    ///
    /// **The first occurrence of each wins, and keeps its key**, which is
    /// `array_unique`'s own behaviour; the spec's § 2 table names no
    /// `preserveKeys` option here, for the reason [`nvs_core_arr_filter`]
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
    fn nvs_core_arr_unique(ctx, args: [2]) {
        let subject = subject(args, "unique")?;
        let by = optional_callback(&args[1], "unique", "by")?;

        // Freed on every exit path, including a throw out of the extractor —
        // see [`Extracted`]. Declared before the set so the values it owns
        // outlive every borrow of them; the set holds only `Value` bits,
        // which own nothing themselves.
        let mut extracted = Extracted(Vec::new());
        let mut seen: std::collections::HashSet<Identity> = std::collections::HashSet::new();
        let mut out = NvsArray::new();
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
                    let named = nvs_runtime::call_closure(ctx, by, &[value, key_arg]);
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
/// owns why — so all four of `rule:types/array-combination`'s members are ordinary two-slot helpers
/// and this is the walk they share. A layer whose tag is not `Tag::Array` is
/// a fatal rather than a skip: the row declares `array<U>`, so meeting
/// anything else means the value did not come through the checker.
fn for_each_layer(
    tail: &Value,
    member: &str,
    mut each: impl FnMut(&NvsArray) -> Result<(), Fault>,
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
fn copy_all(subject: &NvsArray, out: &mut NvsArray) {
    let mut from = 0usize;
    while let Some(slot) = subject.next_slot(from) {
        copy_entry(subject, slot, out);
        from = slot + 1;
    }
}

/// `rule:types/array-combination`'s overlay walk: `layer`'s entries written over `out`, an
/// existing key **replacing in place** and a new key landing at the end.
///
/// The key order falls out of `NvsArray::set` rather than being arranged
/// here — replacing an indexed key leaves its slot where it was, and a new
/// one is pushed — which is what makes `overlay($b, $a)` and
/// `underlay($a, $b)` two operations rather than one with its arguments
/// flipped.
///
/// `deep` is `overlayDeep`'s one extra rule and the only difference between
/// the two members; [`merged`] holds it.
fn overlay_into(out: &mut NvsArray, layer: &NvsArray, deep: bool) {
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
/// `rule:types/array-combination`'s test says the right-hand value replaces the left wholesale.
///
/// The test is *both* sides holding an array and **neither** being a list.
/// A list is replaced rather than merged element-wise because element-wise is
/// the surprise in PHP's `array_replace_recursive` — overlaying `[9]` onto
/// `[1, 2, 3]` yielding `[9, 2, 3]` is never what a configuration merge
/// wanted — and [`is_list`] is the predicate so the rule is stated in terms
/// the language already has.
///
/// The result is a fresh array rather than a mutation of the existing one:
/// an Novis array is a copy-on-write *value* (`rule:types/arrays`), so the entry `out`
/// holds may be shared with the caller's own binding and writing through it
/// would be visible there.
fn merged(out: &NvsArray, key: &[u8], value: Value) -> Option<NvsArray> {
    let existing = borrowed(out.get(key)?.array_ptr()?);
    let incoming = borrowed(value.array_ptr()?);
    if is_list(&existing) || is_list(&incoming) {
        return None;
    }
    let mut nested = NvsArray::new();
    copy_all(&existing, &mut nested);
    overlay_into(&mut nested, &incoming, true);
    Some(nested)
}

/// `rule:types/array-combination`'s underlay walk: `layer`'s entries written *under* `out`, an
/// existing key ignored and a new key landing at the end.
fn underlay_into(out: &mut NvsArray, layer: &NvsArray) {
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

nvs_runtime::nvs_helper! {
    /// `Core\Arr::overlay(array<T> $base, array<U> ...$layers): array<T|U>` —
    /// `rule:types/array-combination`'s right-wins combination, replacing PHP's `array_replace` exactly
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
    fn nvs_core_arr_overlay(_ctx, args: [2]) {
        let base = subject(args, "overlay")?;
        let mut out = NvsArray::new();
        copy_all(&base, &mut out);
        for_each_layer(&args[1], "overlay", |layer| {
            overlay_into(&mut out, layer, false);
            Ok(())
        })?;
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::overlayDeep(array<T> $base, array<U> ...$layers): array<T|U>`
    /// — [`nvs_core_arr_overlay`] with `rule:types/array-combination`'s recursion rule,
    /// replacing PHP's `array_replace_recursive`.
    ///
    /// It recurses only where both sides of a key hold an array and neither
    /// is a list; [`merged`] holds why, and it is also why
    /// `array_merge_recursive` has no replacement at all — promoting two
    /// colliding scalars into a two-element array is a data-shape change
    /// rather than a merge.
    fn nvs_core_arr_overlay_deep(_ctx, args: [2]) {
        let base = subject(args, "overlayDeep")?;
        let mut out = NvsArray::new();
        copy_all(&base, &mut out);
        for_each_layer(&args[1], "overlayDeep", |layer| {
            overlay_into(&mut out, layer, true);
            Ok(())
        })?;
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::underlay(array<T> $base, array<U> ...$layers): array<T|U>` —
    /// `rule:types/array-combination`'s left-wins combination, exactly PHP's `$a + $b` and the
    /// member `E0467` names when a program writes that operator.
    ///
    /// Not `overlay` with its arguments flipped: `overlay($b, $a)` holds the
    /// same entries but in `$b`'s key order, and an Novis array is
    /// insertion-ordered, so the difference is observable in `foreach`, in
    /// `Core\Json::encode` and in every `Arr::first`. Two behaviours, two
    /// names — `rule:core-api/shape-rules` R15.
    fn nvs_core_arr_underlay(_ctx, args: [2]) {
        let base = subject(args, "underlay")?;
        let mut out = NvsArray::new();
        copy_all(&base, &mut out);
        for_each_layer(&args[1], "underlay", |layer| {
            underlay_into(&mut out, layer);
            Ok(())
        })?;
        Ok(Value::array(out))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::appendAll(array<T> $a, array<U> ...$others): array<T|U>` —
    /// `rule:types/array-combination`'s key-discarding combination, replacing PHP's
    /// `array_merge` over lists and its `array_merge(...$arrays)` flatten
    /// idiom.
    ///
    /// **Always a list**, whatever the arguments were: every value of every
    /// argument in order, under fresh keys. That is the half of `array_merge`
    /// a call site actually meant when its arguments were lists, and naming
    /// it separately is what lets `overlay` be the other half without either
    /// one deciding by key type at run time.
    fn nvs_core_arr_append_all(_ctx, args: [2]) {
        let base = subject(args, "appendAll")?;
        let mut out = NvsArray::new();
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
    ctx: &mut nvs_runtime::Ctx,
    on: On,
    by: Option<Value>,
    key: &NvsStr,
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
    let answer = nvs_runtime::call_closure(ctx, by, &[value, key_arg]);
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

/// The shared body of [`nvs_core_arr_diff`] and [`nvs_core_arr_intersect`]:
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
    ctx: &mut nvs_runtime::Ctx,
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
    let mut theirs: Vec<(NvsStr, Value)> = Vec::new();
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

    let mut out = NvsArray::new();
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
                    let verdict = nvs_runtime::call_closure(ctx, comparator, &[mine, *compared])?;
                    let sign = comparator_sign(verdict, member);
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

nvs_runtime::nvs_helper! {
    /// `Core\Arr::diff(array<T> $a, array<T> $b, {on?: SetOn, by?: callable, comparator?: callable}): array<T>`
    /// — the entries of `$a` that `$b` does not have, replacing PHP's
    /// `array_diff` and its five variants.
    ///
    /// **Strict identity, never a string cast.** PHP's `array_diff` compares
    /// `(string) $x === (string) $y`, so `1` and `"1"` are the same element
    /// and two arrays are the same element as each other. `rule:types/preserve-keys` calls that a bug source rather than a decision; this compares the
    /// way `contains` and `unique` already do, which is `nvs_runtime`'s
    /// `value_identical`.
    ///
    /// `$a`'s keys and order are kept, as PHP keeps them. [`set_member`] owns
    /// the walk, the `on`/`by`/`comparator` composition and the cost.
    fn nvs_core_arr_diff(ctx, args: [5]) {
        set_member(ctx, args, "diff", false)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::intersect(array<T> $a, array<T> $b, {on?: SetOn, by?: callable, comparator?: callable}): array<T>`
    /// — the entries of `$a` that `$b` also has, replacing PHP's
    /// `array_intersect` and its five variants.
    ///
    /// The same question [`nvs_core_arr_diff`] asks with the other answer
    /// kept, and the same strict-identity rule.
    fn nvs_core_arr_intersect(ctx, args: [5]) {
        set_member(ctx, args, "intersect", true)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::min(array<T> $a): ?T` — the smallest entry under the
    /// natural ordering, replacing PHP's `min` with an array argument.
    ///
    /// `null` over an empty array rather than PHP's `ValueError`, on
    /// [`nvs_core_arr_first`]'s terms: `rule:core-api/shape-rules` R5 makes `?T` the absence
    /// spelling, and "what is the smallest of nothing" is a question with an
    /// answer, not a failure.
    ///
    /// The ordering is [`compare_values`] — the same total order
    /// [`nvs_core_arr_sort`] uses without a comparator, so
    /// `min($a) === first(sort($a))` holds by construction. It is therefore
    /// *not* PHP's `min`, which compares loosely: `min([0, "a"])` is `0` in
    /// PHP 8 — the `int` cast to `"0"` and compared as a string — and a throw
    /// here, because a `string` and an `int` have no order between them.
    /// `min(["1e2", "50"])` is the other side of the same rule: PHP reads two
    /// numeral strings as numbers and answers `"50"`, and this member compares
    /// them bytewise, exactly as [`nvs_core_arr_sort`] does. The spec's § 2 has
    /// no comparator option on either member; a caller who wants one writes
    /// `first(sort($a, {by: ...}))`. Both halves are pinned in
    /// `tests/differential/core/arr-min-and-max-*`.
    ///
    /// PHP's variadic `min(1, 2, 3)` has no member at all: that is what `<`
    /// and a ternary are for (`rule:core-api/shape-rules` R17), and the array form is the one
    /// that cannot be written in the language.
    fn nvs_core_arr_min(ctx, args: [1]) {
        let subject = subject(args, "min")?;
        extremum(ctx, &subject, std::cmp::Ordering::Less, r"Core\Arr::min")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::max(array<T> $a): ?T` — the largest entry under the natural
    /// ordering, replacing PHP's `max` with an array argument.
    ///
    /// [`nvs_core_arr_min`] owns the ordering, the empty case and the
    /// divergence from PHP's loose comparison.
    fn nvs_core_arr_max(ctx, args: [1]) {
        let subject = subject(args, "max")?;
        extremum(ctx, &subject, std::cmp::Ordering::Greater, r"Core\Arr::max")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::sum(array<int|float|decimal> $a): int|float|decimal` —
    /// replacing PHP's `array_sum`.
    ///
    /// [`Total`] owns the three-way accumulation and every throw it can
    /// produce; the empty array is `int` `0`, PHP's own answer and the
    /// identity of the operation.
    fn nvs_core_arr_sum(_ctx, args: [1]) {
        let subject = subject(args, "sum")?;
        fold_numbers(&subject, Total::Integer(0), "sum")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::product(array<int|float|decimal> $a): int|float|decimal` —
    /// replacing PHP's `array_product`. [`nvs_core_arr_sum`]'s structure with
    /// the other operation, so the empty array is `int` `1`, that operation's
    /// identity and PHP's answer.
    fn nvs_core_arr_product(_ctx, args: [1]) {
        let subject = subject(args, "product")?;
        fold_numbers(&subject, Total::Integer(1), "product")
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Arr::average(array<int|float|decimal> $a): ?(float|decimal)` —
    /// the spec's replacement for `array_sum($a)/count($a)`, "with the empty
    /// case answered".
    ///
    /// **`null` over an empty array**, on [`nvs_core_arr_min`]'s terms: ADR
    /// 0063 R5 makes `?T` the absence spelling, and the average of nothing is
    /// a question with an answer rather than a division by zero.
    ///
    /// An exact subject stays exact: a `decimal` total is divided by the count
    /// as a `decimal`, which `rule:types/arithmetic` rounds half to even at the widest
    /// scale the quotient admits. Every other total answers `float`, which is
    /// what makes this member's result type a union rather than one type.
    fn nvs_core_arr_average(_ctx, args: [1]) {
        let subject = subject(args, "average")?;
        let count = subject.count();
        if count == 0 {
            return Ok(Value::null());
        }
        // Unreachable from source, and for a reason no diagnostic states:
        // `count` is a `usize` and `usize` is no wider than `u64` on any target
        // `deny.toml` builds for, so this conversion is total and the `Err` arm
        // is the price of not writing `as` rather than a boundary a long enough
        // array reaches. `Core\Str::length` states it in full.
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
                         quotient is outside `rule:types/decimal`'s range"
                            .to_owned(),
                    )
                }),
            other => Ok(Value::float(other.as_f64() / f64_of(divisor))),
        }
    }
}

/// What [`nvs_core_arr_sum`], [`nvs_core_arr_product`] and
/// [`nvs_core_arr_average`] accumulate into — spec § 2's
/// `int|float|decimal` as the three shapes an accumulator can be in.
///
/// The promotion rules are `rule:types/arithmetic`'s and `rule:types/arithmetic`'s, applied
/// entry by entry rather than to a pair of static types:
///
/// * `int` with `int` stays `int` and **throws** on overflow — no wrap and no
///   promotion to `float`, which is that ADR's row verbatim and a deliberate
///   divergence from `array_sum`, which silently becomes a `float`.
/// * a `float` anywhere makes the total a `float`, exactly as `int ⊕ float`
///   does.
/// * a `decimal` anywhere makes the total a `decimal`, since an `int` is exact
///   in 96 bits.
/// * a `float` and a `decimal` in **one** subject throws: `rule:types/arithmetic` makes
///   that pair a compile error where the types are static, and there is no
///   representable common type here either. It is reachable only through an
///   `array<int|float|decimal>` holding both.
#[derive(Clone, Copy)]
enum Total {
    /// An `int`, or a `uint` small enough to be one.
    Integer(i64),
    /// A `float`.
    Real(f64),
    /// A `decimal` — `rule:types/decimal`'s scalar, and the one arm that is exact.
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

/// [`Total::add`] over every entry, as [`nvs_core_arr_sum`]'s answer.
fn fold_numbers(subject: &NvsArray, seed: Total, member: &str) -> Result<Value, Fault> {
    let operation = if member == "sum" {
        Total::plus
    } else {
        Total::times
    };
    Ok(fold(subject, seed, member, operation)?.into_value())
}

/// The fold itself, shared by all three members.
fn fold(
    subject: &NvsArray,
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
                 have no common type (`rule:types/arithmetic`, `rule:types/arithmetic`)"
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

/// One value, compared and hashed the way `nvs_runtime` defines identity —
/// what puts a `Core\Arr` set member's seen values in a `HashSet` instead of a
/// linear scan.
///
/// It borrows: a `Value` owns nothing on its own (`nvs_runtime::Value`'s
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
        nvs_runtime::value_identical(self.0, other.0)
    }
}

impl Eq for Identity {}

impl std::hash::Hash for Identity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        nvs_runtime::value_hash(self.0, state);
    }
}

/// The first slot holding `needle`, by strict identity, or `None`.
///
/// Shared by [`nvs_core_arr_contains`] and [`nvs_core_arr_key_of`], which ask
/// the same question and read a different half of the answer — the same
/// pairing [`find_slot`] serves for the predicate members. Linear, and
/// deliberately so: both members answer about *one* needle, so there is
/// nothing to amortize an index over.
fn slot_of(subject: &NvsArray, needle: Value) -> Option<usize> {
    let mut from = 0usize;
    while let Some(slot) = subject.next_slot(from) {
        let value = subject
            .value_at(slot)
            .expect("next_slot only names live entries");
        if nvs_runtime::value_identical(value, needle) {
            return Some(slot);
        }
        from = slot + 1;
    }
    None
}

/// The entry that compares `wanted` against every other, as a fresh reference,
/// or `null` over an empty array — [`nvs_core_arr_min`] and
/// [`nvs_core_arr_max`] in one walk.
///
/// The **first** extreme wins a tie, so a stable sort and this member name the
/// same entry.
fn extremum(
    ctx: &mut Ctx,
    subject: &NvsArray,
    wanted: std::cmp::Ordering,
    member: &str,
) -> Result<Value, Fault> {
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
                if compare_values(ctx, &value, &incumbent, member)? == wanted {
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

nvs_runtime::nvs_helper! {
    /// `Core\Arr::shapeAs<T>(array<mixed> $a): T` — the array read as the type
    /// the call site wrote, which is the member the two request wrappers are
    /// one call over.
    ///
    /// **The walk is `crate::json`'s and nothing here is a second one.** A
    /// shape's field list, its presence column, its dotted paths and the one
    /// `ParseError` carrying every field that failed are all
    /// `rule:core-classes/derive-reports-every-field`'s, which
    /// `Core\Json::decodeAs` already reaches through the same
    /// [`crate::json::hydrate`]. Two field-error conventions in one runtime is
    /// how a program learns to catch two things, so this member converts and
    /// reports by exactly the rules that door does. What differs is one bit —
    /// `crate::json::Reading::Values`, because an array's entries carry no
    /// wire types and `"42"` in an `int` field is
    /// `rule:types/conversion`'s `string → int` row rather than a document
    /// disagreeing with itself. Nothing new enters that table.
    ///
    /// **Arguments 0 to 2 are what the call site wrote as its type argument**,
    /// not values — [`crate::registry::WRITTEN_CLASS_MEMBERS`] puts this member
    /// on the roster whose helper is handed a `nvs_runtime::ClassDesc`, the
    /// `array<...>` flag and an inline shape's wire contract ahead of its
    /// declared parameters, so the arity here is three more than the row's.
    ///
    /// **What it spends:** the object it hands back, freed with the request
    /// that asked for one, and nothing held between calls. The subject is the
    /// caller's and is neither copied nor kept.
    fn nvs_core_arr_shape_as(ctx, args: [4]) {
        // Unreachable from source, on `Core\Request::jsonAs`'s reasoning: the
        // three leading slots are constants `nvs_ir::lower` writes out of the
        // type argument, and a call naming none is `E0442` before any of this
        // runs.
        let class = args[0].as_class_desc().ok_or_else(|| Fault::fatal(
            "internal error: `Core\\Arr::shapeAs` was called with no class in argument 0",
        ))?;
        // Unreachable from source for the same reason and refused by the same
        // `E0442`: slot 1 is the `ConstBool` the lowering emits beside the
        // descriptor, so a call that has one has the other.
        let list = args[1].as_bool().ok_or_else(|| Fault::fatal(
            "internal error: `Core\\Arr::shapeAs` was called with no list flag in argument 1",
        ))?;
        // The zero word where the call site wrote a class, whose own contract
        // is on the descriptor above.
        let shape = args[2].as_shape_codec();
        #[expect(
            unsafe_code,
            reason = "the descriptor came out of a `ClassDescConst` the compiled \
                      unit owns, so it outlives this call and every object made \
                      from it"
        )]
        unsafe {
            crate::json::check_codec(class, shape, "Core\\Arr::shapeAs")?;
        }
        // The subject is the caller's array and a helper's arguments are
        // borrowed, while `hydrate` releases what it is handed — it was
        // written for a document its own caller had just built. So this owes
        // the reference it is about to give away.
        #[expect(
            unsafe_code,
            reason = "the caller holds this array for the length of the call, so \
                      a second reference to it is sound, and the callee releases it"
        )]
        unsafe {
            args[3].retain();
        }
        #[expect(
            unsafe_code,
            reason = "the same descriptor, still owned by the compiled unit"
        )]
        unsafe {
            crate::json::hydrate(
                ctx,
                class,
                shape,
                args[3],
                list,
                crate::json::Reading::Values,
                "Core\\Arr::shapeAs",
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use nvs_runtime::{
        ClassDesc, ClassTable, CodecField, CodecTy, Ctx, Fault, NvsArray, NvsObj, NvsStr,
        OutputSink, ShapeCodec, Tag, ThrownClass, Value, call,
    };

    /// The two halves of an inline shape's contract: the class whose slots the
    /// fields land in, and the [`ShapeCodec`] carrying their wire types.
    ///
    /// Nothing in this crate builds either, because nothing here compiles a
    /// call site. [`crate::request`]'s `reading_class` is the nearest fixture
    /// and it builds a *class*, whose contract rides on its own descriptor; a
    /// shape's cannot, because `{n: int}` and `{n: string}` are one class, so
    /// the field types live beside it and
    /// [`ClassTable::define_shape_codec`] is what hands back their address.
    ///
    /// `fields` is `(name, wire type, required, nullable)` — the last two are
    /// `rule:core-api/required-optional-and-nullable`'s two independent
    /// columns — and must be in **sorted field-name order**, which is the
    /// order a shape class lays its slots out in and therefore the order that
    /// makes a field's index both its slot and its `param`.
    ///
    /// The table is leaked for `reading_class`'s reason: a descriptor's
    /// *address* is its identity and it must outlive every instance made from
    /// it. The contract's address survives the move into the leaked box
    /// because [`ClassTable::define_shape_codec`] boxes each one, so the move
    /// carries only the vector's header.
    fn shape_of(fields: &[(&str, CodecTy, bool, bool)]) -> (*const ClassDesc, *const ShapeCodec) {
        let names: Vec<&str> = fields.iter().map(|(name, ..)| *name).collect();
        let mut table = ClassTable::new();
        // `nvs_runtime::ClassDesc::is_shape` reads exactly this prefix, and `$`
        // cannot start an Novis identifier, so no declared class collides.
        let id = table.define(format!("$shape{{{}}}", names.join(",")), &names, &[]);
        let codec: Vec<CodecField> = fields
            .iter()
            .enumerate()
            .map(|(slot, (key, ty, required, nullable))| CodecField {
                key: (*key).to_owned(),
                slot,
                param: slot,
                ty: *ty,
                element: None,
                class: None,
                cases: None,
                shape: None,
                nullable: *nullable,
                required: *required,
                // A shape declares no constructor, so an absent optional key is
                // the never-written marker rather than a default.
                default: None,
            })
            .collect();
        // One entry per field, null throughout: none of these names a class or
        // a nested contract, which is all the two vectors are read for.
        let classes = vec![std::ptr::null(); codec.len()];
        let shapes = vec![std::ptr::null(); codec.len()];
        let shape = table.define_shape_codec(codec, classes, shapes);
        // No `set_methods`, unlike the class fixture: a shape declares no
        // constructor and `crate::json`'s `build_shape` writes its slots
        // directly rather than reaching `nvs_runtime::construct`.
        let table: &'static ClassTable = Box::leak(Box::new(table));
        (table.desc(id), shape)
    }

    /// An array of the `(key, value)` pairs given, in order.
    ///
    /// The values are transferred: the array owns each one, so releasing the
    /// array is the whole of what a test using this owes.
    fn array_of<'a>(entries: impl IntoIterator<Item = (&'a str, Value)>) -> Value {
        let mut array = NvsArray::new();
        for (key, value) in entries {
            array.set(NvsStr::new(key.as_bytes()), value);
        }
        Value::array(array)
    }

    /// `Core\Arr::shapeAs<T>($a)` at the ABI a compiled call site gives it:
    /// [`crate::registry::WRITTEN_CLASS_MEMBERS`] writes the descriptor, the
    /// `array<...>` flag and the wire contract ahead of the declared
    /// parameter, so the helper's arity is three more than the row's.
    fn shape_as(
        ctx: &mut Ctx,
        contract: (*const ClassDesc, *const ShapeCodec),
        subject: Value,
    ) -> Result<Value, i32> {
        call(
            super::nvs_core_arr_shape_as,
            ctx,
            &[
                Value::class_desc(contract.0),
                Value::bool(false),
                Value::shape_codec(contract.1),
                subject,
            ],
        )
    }

    /// The `slot`th field of an instance, borrowed rather than taken: the
    /// caller still owns the reference it passed in.
    #[expect(
        unsafe_code,
        reason = "the value is an instance this frame holds a reference to, so \
                  the handle borrowed from it cannot outlive the allocation"
    )]
    fn field_at(instance: Value, slot: usize) -> Value {
        let object = std::mem::ManuallyDrop::new(unsafe {
            NvsObj::from_raw(
                instance
                    .obj_ptr()
                    .expect("`shapeAs()` answers an instance of the shape it was given"),
            )
        });
        object.field(slot)
    }

    /// A string field read back as text, for an assertion that wants the
    /// bytes rather than the handle.
    fn text_at(instance: Value, slot: usize) -> String {
        let field = field_at(instance, slot);
        let bytes = field
            .as_str_bytes()
            .expect("the field is declared `string`, so the walk produced one");
        String::from_utf8(bytes.to_vec()).expect("a test's own literals are UTF-8")
    }

    /// Releases a reference this test owns.
    #[expect(
        unsafe_code,
        reason = "each caller built or was handed the one reference it drops here"
    )]
    fn dropped(value: Value) {
        unsafe {
            value.release();
        }
    }

    /// Every issue on a `ParseError` as `(path, message)`, in the order the
    /// walk recorded them.
    ///
    /// Read off the [`Fault`] rather than through
    /// `nvs_runtime::Ctx::pending_slot`, which answers `None` until an
    /// exception class table is installed — a fixture this question does not
    /// otherwise need, since [`crate::json::hydrate`] is the walk the member
    /// is one ABI above.
    #[expect(
        unsafe_code,
        reason = "the fault owns the issue list for as long as the caller holds \
                  it, so a borrow of the array behind it cannot outlive the \
                  allocation"
    )]
    fn issues_of(fault: &Fault) -> Vec<(String, String)> {
        let Fault::ThrownWithSlots(class, _, slots) = fault else {
            panic!("a failed field is a `ParseError` carrying its issues");
        };
        assert!(
            matches!(class, ThrownClass::Parse),
            "a field that did not match is a `ParseError`"
        );
        let list = slots
            .first()
            .map(|(_, value)| *value)
            .expect("the issue list is the one slot a `ParseError` fills");
        let array = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(list.array_ptr().expect("`issues` is an `array<Issue>`"))
        });
        (0..array.count())
            .map(|at| {
                let issue = array
                    .get_index(i64::try_from(at).expect("a test's issue list is short"))
                    .expect("every position of a list is filled");
                // `crate::issue::FIELDS` is sorted field-name order, so slot 0
                // is `message` and slot 1 is `path`.
                (text_at(issue, 1), text_at(issue, 0))
            })
            .collect()
    }

    /// The member end to end through the `rule:errors/propagation` boundary compiled code will
    /// reach it at — `call` builds the same three pointers a JIT frame does.
    #[test]
    fn count_reports_the_number_of_live_entries() {
        let mut array = NvsArray::new();
        array.set(NvsStr::new(b"a"), Value::int(1));
        array.set(NvsStr::new(b"b"), Value::int(2));
        array.unset(b"a");

        let mut ctx = Ctx::new(OutputSink::Sink);
        let subject = Value::array(array);
        let result = call(super::nvs_core_arr_count, &mut ctx, &[subject])
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
    /// exists for: `nvs-ir` already normalizes an `int` subscript to its
    /// decimal string, and a helper argument arrives untouched, so this is the
    /// other half of the same rule.
    #[test]
    fn has_key_normalizes_an_int_key_the_way_a_subscript_does() {
        let mut array = NvsArray::new();
        array.set(NvsStr::new(b"5"), Value::int(1));
        array.set(NvsStr::new(b"name"), Value::int(2));
        let subject = Value::array(array);

        let asked = |key: Value| {
            let mut ctx = Ctx::new(OutputSink::Sink);
            call(super::nvs_core_arr_has_key, &mut ctx, &[subject, key])
                .expect("asking never fails")
                .as_bool()
                .expect("hasKey returns a bool")
        };
        assert!(asked(Value::int(5)));
        assert!(asked(Value::str(NvsStr::new(b"5"))));
        assert!(asked(Value::str(NvsStr::new(b"name"))));
        assert!(!asked(Value::int(6)));
        assert!(!asked(Value::str(NvsStr::new(b"nope"))));

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
        let mut array = NvsArray::new();
        array.set(NvsStr::new(b"a"), Value::int(1));
        let before = array.refcount();
        let subject = Value::array(array);

        let mut ctx = Ctx::new(OutputSink::Sink);
        call(
            super::nvs_core_arr_has_key,
            &mut ctx,
            &[subject, Value::str(NvsStr::new(b"a"))],
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
            unsafe { NvsArray::from_raw(subject.array_ptr().expect("an array")) }.refcount();
        assert_eq!(after, before);
    }

    /// Every row verified against PHP 8.5's own `array_is_list`, including the
    /// two `rule:types/arrays` makes interesting: a canonical integer *string* key is
    /// a list key (PHP normalizes it to an int), and a non-canonical one
    /// (`"01"`) is not.
    // covers: Core\Arr::isList
    #[test]
    fn is_list_matches_phps_answer_for_every_key_shape() {
        let asked = |keys: &[&[u8]]| {
            let mut array = NvsArray::new();
            for key in keys {
                array.set(NvsStr::new(key), Value::int(1));
            }
            let subject = Value::array(array);
            let mut ctx = Ctx::new(OutputSink::Sink);
            let answer = call(super::nvs_core_arr_is_list, &mut ctx, &[subject])
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
        let mut array = NvsArray::new();
        array.set(NvsStr::new(b"0"), Value::int(1));
        array.set(NvsStr::new(b"1"), Value::int(2));
        array.unset(b"0");
        let subject = Value::array(array);
        let mut ctx = Ctx::new(OutputSink::Sink);
        assert_eq!(
            call(super::nvs_core_arr_is_list, &mut ctx, &[subject])
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
    // covers: Core\Arr::values
    #[test]
    fn values_renumbers_from_zero_and_retains_what_it_copies() {
        let mut array = NvsArray::new();
        array.set(NvsStr::new(b"x"), Value::str(NvsStr::new(b"a")));
        array.set(NvsStr::new(b"y"), Value::str(NvsStr::new(b"b")));
        let subject = Value::array(array);

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(super::nvs_core_arr_values, &mut ctx, &[subject])
            .expect("taking values never fails");
        #[expect(
            unsafe_code,
            reason = "the helper returned one fresh reference, which the handle \
                      takes over and releases on drop"
        )]
        let out =
            unsafe { NvsArray::from_raw(result.array_ptr().expect("values returns an array")) };
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
        let still = unsafe { NvsArray::from_raw(subject.array_ptr().expect("an array")) };
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
            unsafe { NvsArray::from_raw(result.array_ptr().expect("the member returns an array")) };
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
        let mut array = NvsArray::new();
        array.set(NvsStr::new(b"x"), Value::str(NvsStr::new(b"a")));
        array.set(NvsStr::new(b"10"), Value::str(NvsStr::new(b"b")));
        array.set(NvsStr::new(b"y"), Value::str(NvsStr::new(b"c")));
        Value::array(array)
    }

    /// Verified against PHP 8.5's `array_keys`, except that the `"10"` key
    /// comes back as the string it is stored as rather than as an `int`.
    // covers: Core\Arr::keys
    #[test]
    fn keys_yields_the_stored_spelling_of_every_key() {
        let subject = mixed_keys();
        let mut ctx = Ctx::new(OutputSink::Sink);
        let result =
            call(super::nvs_core_arr_keys, &mut ctx, &[subject]).expect("taking keys never fails");
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
        let still = unsafe { NvsArray::from_raw(subject.array_ptr().expect("an array")) };
        assert_eq!(
            still.keys(),
            vec![b"x".to_vec(), b"10".to_vec(), b"y".to_vec()]
        );
    }

    /// The default discards every key rather than PHP's renumber-the-integers-
    /// keep-the-strings, which is `rule:types/preserve-keys`'s rule and the one place this
    /// member is not `array_reverse`.
    // covers: Core\Arr::reverse
    #[test]
    fn reverse_renumbers_by_default_and_keeps_every_key_on_request() {
        let mut ctx = Ctx::new(OutputSink::Sink);

        let renumbered = call(
            super::nvs_core_arr_reverse,
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
            super::nvs_core_arr_reverse,
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
    // covers: Core\Arr::flip
    #[test]
    fn flip_collapses_a_duplicate_value_in_its_first_position() {
        let mut array = NvsArray::new();
        array.set(NvsStr::new(b"a"), Value::str(NvsStr::new(b"p")));
        array.set(NvsStr::new(b"b"), Value::int(7));
        array.set(NvsStr::new(b"c"), Value::str(NvsStr::new(b"p")));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(super::nvs_core_arr_flip, &mut ctx, &[Value::array(array)])
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
        let mut array = NvsArray::new();
        array.set(NvsStr::new(b"a"), Value::bool(true));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(super::nvs_core_arr_flip, &mut ctx, &[Value::array(array)])
            .expect_err("a bool is not a key");
        assert_eq!(status, nvs_runtime::FATAL);
    }

    /// One level of nesting, and every key an inner array carried is spent on a
    /// position in the answer — `rule:types/preserve-keys` again, and why the
    /// answer is a list whatever the subject's keys were.
    // covers: Core\Arr::flatten
    #[test]
    fn flatten_takes_one_level_and_numbers_what_it_pulled_up() {
        let mut first = NvsArray::new();
        first.set(NvsStr::new(b"x"), Value::str(NvsStr::new(b"a")));
        first.set(NvsStr::new(b"y"), Value::str(NvsStr::new(b"b")));
        let mut second = NvsArray::new();
        second.set(NvsStr::new(b"10"), Value::str(NvsStr::new(b"c")));

        let mut outer = NvsArray::new();
        outer.set(NvsStr::new(b"left"), Value::array(first));
        outer.set(NvsStr::new(b"right"), Value::array(second));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::nvs_core_arr_flatten,
            &mut ctx,
            &[Value::array(outer)],
        )
        .expect("every element is an array");
        assert_eq!(
            entries_of(result),
            vec![
                (b"0".to_vec(), b"a".to_vec()),
                (b"1".to_vec(), b"b".to_vec()),
                (b"2".to_vec(), b"c".to_vec()),
            ]
        );
    }

    /// Depth is the whole difference from [`super::nvs_core_arr_flatten`]: the
    /// walk descends until it meets a value that is not an array, and a scalar
    /// standing beside an array at the same level keeps its place in the order.
    // covers: Core\Arr::flattenDeep
    #[test]
    fn flatten_deep_reaches_every_level_in_the_order_it_met_them() {
        let mut deepest = NvsArray::new();
        deepest.set(NvsStr::new(b"0"), Value::str(NvsStr::new(b"c")));

        let mut middle = NvsArray::new();
        middle.set(NvsStr::new(b"0"), Value::str(NvsStr::new(b"b")));
        middle.set(NvsStr::new(b"1"), Value::array(deepest));

        let mut outer = NvsArray::new();
        outer.set(NvsStr::new(b"0"), Value::str(NvsStr::new(b"a")));
        outer.set(NvsStr::new(b"1"), Value::array(middle));
        outer.set(NvsStr::new(b"2"), Value::str(NvsStr::new(b"d")));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::nvs_core_arr_flatten_deep,
            &mut ctx,
            &[Value::array(outer)],
        )
        .expect("a mixed array never fails");
        assert_eq!(
            entries_of(result),
            vec![
                (b"0".to_vec(), b"a".to_vec()),
                (b"1".to_vec(), b"b".to_vec()),
                (b"2".to_vec(), b"c".to_vec()),
                (b"3".to_vec(), b"d".to_vec()),
            ]
        );
    }

    /// Two rows whose cells are texts long enough to be allocations of their
    /// own, which is what
    /// [`column_stores_the_subject_cell_with_a_reference_of_its_own`] reads a
    /// reference count off — a one-character cell is stored inside the value
    /// and has none.
    fn rows_holding_long_cells() -> Value {
        let mut rows = NvsArray::new();
        for (position, id, name) in [
            (
                &b"0"[..],
                &b"r1"[..],
                &b"a text long enough to be its own allocation: a"[..],
            ),
            (
                &b"1"[..],
                &b"r2"[..],
                &b"a text long enough to be its own allocation: b"[..],
            ),
        ] {
            let mut row = NvsArray::new();
            row.set(NvsStr::new(b"id"), Value::str(NvsStr::new(id)));
            row.set(NvsStr::new(b"name"), Value::str(NvsStr::new(name)));
            rows.set(NvsStr::new(position), Value::array(row));
        }
        Value::array(rows)
    }

    /// Every entry of an array a helper returned, as `(key, how many references
    /// hold its text)`.
    fn references_of(result: Value) -> Vec<(Vec<u8>, usize)> {
        #[expect(
            unsafe_code,
            reason = "the handle takes over the one reference the member \
                      answered with, and releases it on drop"
        )]
        let array =
            unsafe { NvsArray::from_raw(result.array_ptr().expect("the member returns an array")) };
        let mut entries = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            let key = array.key_at(slot).expect("a live entry has a key");
            let value = array.value_at(slot).expect("a live entry has a value");
            #[expect(
                unsafe_code,
                reason = "the subject holds the text for the length of the \
                          caller's test, and the handle above holds the result"
            )]
            let references =
                unsafe { NvsStr::refcount_of(value.str_ptr().expect("every cell here is a text")) };
            entries.push((key.as_bytes().to_vec(), references));
            from = slot + 1;
        }
        entries
    }

    /// Every cell the result holds is the subject's own, held with a reference
    /// of its own, on both of the member's paths.
    ///
    /// **Only the Rust side can see the reference.** A cell is stored twice and
    /// owned once, so a member that stored it without retaining prints the
    /// right text from Novis for as long as the subject is alive and frees it
    /// twice at the end, and one that retained twice leaks a text per row while
    /// printing exactly the same thing. The two paths reach that reference from
    /// different places — the default appends a borrowed cell, `{indexBy}`
    /// retains one and sets it under a key it copied — so both are asked here.
    // covers: Core\Arr::column
    #[test]
    fn column_stores_the_subject_cell_with_a_reference_of_its_own() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let subject = rows_holding_long_cells();
        let cell = Value::str(NvsStr::new(b"name"));
        let by_id = Value::str(NvsStr::new(b"id"));

        let appended = call(
            super::nvs_core_arr_column,
            &mut ctx,
            &[subject, cell, Value::null()],
        )
        .expect("every row is an array");
        assert_eq!(
            references_of(appended),
            vec![(b"0".to_vec(), 2), (b"1".to_vec(), 2)]
        );

        let keyed = call(
            super::nvs_core_arr_column,
            &mut ctx,
            &[subject, cell, by_id],
        )
        .expect("every row is an array");
        assert_eq!(
            references_of(keyed),
            vec![(b"r1".to_vec(), 2), (b"r2".to_vec(), 2)]
        );

        dropped(subject);
        dropped(cell);
        dropped(by_id);
    }

    /// `["b" => …, "a" => …, "c" => …]` over texts long enough to be
    /// allocations of their own, so a reference count can be read off each of
    /// them, and under keys no byte compare leaves where it found them.
    fn out_of_order_holding_long_values() -> Value {
        let mut array = NvsArray::new();
        for (key, value) in [
            (
                &b"b"[..],
                &b"a text long enough to be its own allocation: b"[..],
            ),
            (
                &b"a"[..],
                &b"a text long enough to be its own allocation: a"[..],
            ),
            (
                &b"c"[..],
                &b"a text long enough to be its own allocation: c"[..],
            ),
        ] {
            array.set(NvsStr::new(key), Value::str(NvsStr::new(value)));
        }
        Value::array(array)
    }

    /// A two-parameter comparator answering `NaN`, the one verdict the sort
    /// refuses, so that a walk fails part-way through.
    ///
    /// The sweep of the three references is not optional, for [`below_ten`]'s
    /// reason.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes the receiver and two arguments, each \
                  retained for this callee to release, and `abi::call` passes \
                  the address of a live `Value` for the result — neither is \
                  expressible in the signature compiled code calls through"
    )]
    unsafe extern "C" fn compares_with_a_nan(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        for index in 0..3 {
            dropped(unsafe { *args.add(index) });
        }
        unsafe {
            *out = Value::float(f64::NAN);
        }
        nvs_runtime::OK
    }

    /// The result holds each of the subject's values with a reference of its
    /// own, and a comparator that fails leaves the subject holding them alone.
    ///
    /// **Only the Rust side can see either half.** The member collects a
    /// reference per key and borrows a value per entry before it compares
    /// anything, so a walk that fails half-way has both to give back; from
    /// Novis a leak there prints nothing at all, and a missing retain prints
    /// the right order until the subject is freed. The order asserted after the
    /// failure is the subject's own insertion order, which is the second thing
    /// a failed sort owes: it answered nothing, so it moved nothing.
    // covers: Core\Arr::sortByKey
    #[test]
    fn sort_by_key_takes_a_reference_per_value_and_gives_them_back_when_it_fails() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let subject = out_of_order_holding_long_values();

        let sorted = call(
            super::nvs_core_arr_sort_by_key,
            &mut ctx,
            &[subject, Value::int(0), Value::null()],
        )
        .expect("a byte compare never fails");
        assert_eq!(
            references_of(sorted),
            vec![(b"a".to_vec(), 2), (b"b".to_vec(), 2), (b"c".to_vec(), 2)]
        );

        let comparator = closure_of(2, compares_with_a_nan);
        call(
            super::nvs_core_arr_sort_by_key,
            &mut ctx,
            &[subject, Value::int(0), comparator],
        )
        .expect_err("NaN has no ordering");

        #[expect(
            unsafe_code,
            reason = "the reader below releases the reference it is handed, and \
                      this test still owns the subject afterwards"
        )]
        unsafe {
            subject.retain();
        }
        assert_eq!(
            references_of(subject),
            vec![(b"b".to_vec(), 1), (b"a".to_vec(), 1), (b"c".to_vec(), 1)]
        );

        dropped(subject);
        dropped(comparator);
    }

    /// The values `range` produces, in order — read back through the array's
    /// own cursor rather than by key, so a wrong *order* fails here and not
    /// only a wrong set.
    fn range_of(start: i64, end: i64, step: i64) -> Vec<i64> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::nvs_core_arr_range,
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
            unsafe { NvsArray::from_raw(result.array_ptr().expect("range returns an array")) };
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
    // covers: Core\Arr::range
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
    // covers: Core\Arr::range
    #[test]
    fn a_step_of_zero_or_less_throws() {
        for step in [0, -1] {
            let mut ctx = Ctx::new(OutputSink::Sink);
            let status = call(
                super::nvs_core_arr_range,
                &mut ctx,
                &[Value::int(1), Value::int(5), Value::int(step)],
            )
            .expect_err("a non-positive step is refused");
            assert_eq!(status, nvs_runtime::THROWN);
        }
    }

    /// A `chunk` size of zero is `THROWN` rather than an empty answer: no run
    /// length terminates, so answering `[]` would turn a caller's arithmetic
    /// bug into a loop that quietly does nothing. PHP raises `ValueError`.
    ///
    /// Here rather than in a `.nvst` case because a conformance case cannot
    /// catch at file scope, and the value of this row is the *class* of
    /// failure, not the message.
    // covers: Core\Arr::chunk
    #[test]
    fn a_chunk_size_of_zero_throws() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::nvs_core_arr_chunk,
            &mut ctx,
            &[mixed_keys(), Value::uint(0), Value::bool(false)],
        )
        .expect_err("a zero run length is refused");
        assert_eq!(status, nvs_runtime::THROWN);
    }

    /// A window is a copy: it renumbers from zero, keeps the subject's own
    /// keys when it is asked to, and takes a reference of its own for every
    /// entry it copies — a missing retain is a double free the moment either
    /// array is dropped.
    ///
    /// Here rather than in a `.nvst` case because the retain is the half no
    /// program can observe; the positions themselves are pinned from Novis.
    // covers: Core\Arr::slice
    #[test]
    fn a_slice_is_a_copy_that_renumbers_unless_it_is_asked_to_keep_the_keys() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let subject = mixed_keys();
        let window = |ctx: &mut Ctx, preserve: bool| {
            entries_of(
                call(
                    super::nvs_core_arr_slice,
                    ctx,
                    &[subject, Value::int(1), Value::int(2), Value::bool(preserve)],
                )
                .expect("a window over three entries never fails"),
            )
        };

        assert_eq!(
            window(&mut ctx, false),
            vec![
                (b"0".to_vec(), b"b".to_vec()),
                (b"1".to_vec(), b"c".to_vec())
            ]
        );
        assert_eq!(
            window(&mut ctx, true),
            vec![
                (b"10".to_vec(), b"b".to_vec()),
                (b"y".to_vec(), b"c".to_vec())
            ]
        );

        // Both windows are released by now, and the subject still holds every
        // value it lent them — which is what the retain bought.
        #[expect(
            unsafe_code,
            reason = "this test still owns the one reference it built above"
        )]
        let still = unsafe { NvsArray::from_raw(subject.array_ptr().expect("an array")) };
        assert_eq!(
            still
                .get(b"y")
                .and_then(|v| v.as_str_bytes().map(<[u8]>::to_vec)),
            Some(b"c".to_vec())
        );
    }

    /// The kept head, the replacement and the kept tail are three walks into
    /// one fresh list: every entry is renumbered, string keys included, and
    /// both subjects keep everything they lent it. The member copies rather
    /// than moves, so neither argument is left short a reference.
    // covers: Core\Arr::replaceRange
    #[test]
    fn a_replaced_range_renumbers_both_sides_and_copies_what_it_carries() {
        let mut replacement = NvsArray::new();
        replacement.set(NvsStr::new(b"k"), Value::str(NvsStr::new(b"n")));
        let handed = Value::array(replacement);
        let subject = mixed_keys();

        let mut ctx = Ctx::new(OutputSink::Sink);
        let out = entries_of(
            call(
                super::nvs_core_arr_replace_range,
                &mut ctx,
                &[subject, Value::int(1), Value::int(1), handed],
            )
            .expect("a window inside the array never fails"),
        );
        assert_eq!(
            out,
            vec![
                (b"0".to_vec(), b"a".to_vec()),
                (b"1".to_vec(), b"n".to_vec()),
                (b"2".to_vec(), b"c".to_vec())
            ]
        );

        // The result is released by now, and both arguments still hold every
        // value they lent it — which is what the retain bought.
        #[expect(
            unsafe_code,
            reason = "this test still owns the two references it built above, \
                      and the helper borrowed rather than consumed them"
        )]
        let (still, lent) = unsafe {
            (
                NvsArray::from_raw(subject.array_ptr().expect("an array")),
                NvsArray::from_raw(handed.array_ptr().expect("an array")),
            )
        };
        assert_eq!(
            still
                .get(b"10")
                .and_then(|v| v.as_str_bytes().map(<[u8]>::to_vec)),
            Some(b"b".to_vec())
        );
        assert_eq!(
            lent.get(b"k")
                .and_then(|v| v.as_str_bytes().map(<[u8]>::to_vec)),
            Some(b"n".to_vec())
        );
    }

    /// The cursor stops rather than wrapping when the next step would leave
    /// `int` — `rule:types/arithmetic`'s rule applied to a loop this member owns.
    // covers: Core\Arr::range
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
        let mut array = NvsArray::new();
        for (key, value) in entries {
            array.set(NvsStr::new(key), *value);
        }
        let subject = Value::array(array);
        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::nvs_core_arr_sort,
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
            let out = NvsArray::from_raw(result.array_ptr().expect("sort returns an array"));
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
    // covers: Core\Arr::sort
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
    /// `nvs_core_arr_sort`'s own docs own the reasoning.
    // covers: Core\Arr::sort
    #[test]
    fn two_strings_compare_bytewise_rather_than_numerically() {
        let entries: [(&[u8], Value); 2] = [
            (b"0", Value::str(NvsStr::new(b"9"))),
            (b"1", Value::str(NvsStr::new(b"10"))),
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
    // covers: Core\Arr::sort
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
    // covers: Core\Arr::sort
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
    // covers: Core\Arr::sort
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
    /// object is the one `rule:classes/comparable`'s `Comparable` is the eventual answer for.
    // covers: Core\Arr::sort
    #[test]
    fn a_pair_with_no_natural_order_throws() {
        let mut array = NvsArray::new();
        array.set(NvsStr::new(b"0"), Value::int(1));
        array.set(NvsStr::new(b"1"), Value::array(NvsArray::new()));
        let subject = Value::array(array);
        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::nvs_core_arr_sort,
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
        assert_eq!(status, nvs_runtime::THROWN);
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
    // covers: Core\Arr::sort
    #[test]
    fn an_order_outside_the_enum_is_a_contained_fault() {
        let subject = Value::array(NvsArray::new());
        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::nvs_core_arr_sort,
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
        assert_eq!(status, nvs_runtime::FATAL);
        #[expect(
            unsafe_code,
            reason = "this test owns the one reference it built above, and the \
                      helper borrowed rather than consumed it"
        )]
        unsafe {
            subject.release();
        }
    }

    /// The array half of `rule:iteration/foreach-subjects`'s three shapes: the argument's keys
    /// discarded, its order kept, and `{limit}` honoured. The two object
    /// shapes need compiled code to drive, so they are pinned by
    /// `tests/conformance/core/arr-from-drains-a-sequence.nvst` instead.
    #[test]
    fn from_materialises_an_array_as_a_list() {
        let mut entries = NvsArray::new();
        entries.set(NvsStr::new(b"named"), Value::str(NvsStr::new(b"a")));
        entries.append(Value::str(NvsStr::new(b"b")));
        let subject = Value::array(entries);

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::nvs_core_arr_from,
            &mut ctx,
            &[subject, Value::null()],
        )
        .expect("an array is one of the three shapes");
        assert_eq!(
            entries_of(result),
            vec![
                (b"0".to_vec(), b"a".to_vec()),
                (b"1".to_vec(), b"b".to_vec()),
            ]
        );

        let mut ctx = Ctx::new(OutputSink::Sink);
        let limited = call(
            super::nvs_core_arr_from,
            &mut ctx,
            &[subject, Value::uint(1)],
        )
        .expect("a limit stops the drain");
        assert_eq!(entries_of(limited), vec![(b"0".to_vec(), b"a".to_vec())]);

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
        let status = call(super::nvs_core_arr_count, &mut ctx, &[Value::int(7)])
            .expect_err("an int is not an array");
        assert_eq!(status, nvs_runtime::FATAL);

        // `map` decodes its subject before it ever looks at the callback, so a
        // wrong subject is reported rather than reaching `call_closure` with a
        // value that is not a closure either.
        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::nvs_core_arr_map,
            &mut ctx,
            &[Value::int(7), Value::int(7)],
        )
        .expect_err("an int is not an array");
        assert_eq!(status, nvs_runtime::FATAL);
    }

    /// The deliberate divergence from `array_shift`/`array_pop`, which renumber
    /// the integer keys and keep the string ones: PHP answers `0,y` and `x,10`
    /// for this subject, and both members answer with the surviving keys
    /// untouched.
    #[test]
    fn dropping_an_end_entry_leaves_every_surviving_key_alone() {
        let mut ctx = Ctx::new(OutputSink::Sink);

        let tail = call(super::nvs_core_arr_without_first, &mut ctx, &[mixed_keys()])
            .expect("dropping the first entry never fails");
        assert_eq!(
            entries_of(tail),
            vec![
                (b"10".to_vec(), b"b".to_vec()),
                (b"y".to_vec(), b"c".to_vec()),
            ]
        );

        let head = call(super::nvs_core_arr_without_last, &mut ctx, &[mixed_keys()])
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
        let mut single = NvsArray::new();
        single.set(NvsStr::new(b"only"), Value::str(NvsStr::new(b"a")));

        for member in [
            super::nvs_core_arr_without_first,
            super::nvs_core_arr_without_last,
        ] {
            for subject in [Value::array(NvsArray::new()), Value::array(single.clone())] {
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
    // covers: Core\Arr::fromKeysAndValues
    #[test]
    fn from_keys_and_values_pairs_by_position_and_normalizes_each_key() {
        let mut keys = NvsArray::new();
        keys.set(NvsStr::new(b"ignored"), Value::str(NvsStr::new(b"a")));
        keys.append(Value::int(10));
        let mut values = NvsArray::new();
        values.append(Value::str(NvsStr::new(b"first")));
        values.set(
            NvsStr::new(b"also ignored"),
            Value::str(NvsStr::new(b"second")),
        );

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::nvs_core_arr_from_keys_and_values,
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

    /// `rule:core-api/shape-rules` R4: PHP 8 raises a `ValueError` here rather than pairing what
    /// it can, and silently dropping the excess would lose data.
    // covers: Core\Arr::fromKeysAndValues
    #[test]
    fn two_arrays_of_different_lengths_do_not_combine() {
        let mut keys = NvsArray::new();
        keys.append(Value::str(NvsStr::new(b"a")));
        let mut values = NvsArray::new();
        values.append(Value::int(1));
        values.append(Value::int(2));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::nvs_core_arr_from_keys_and_values,
            &mut ctx,
            &[Value::array(keys), Value::array(values)],
        )
        .expect_err("one key cannot carry two values");
        assert_eq!(status, nvs_runtime::THROWN);
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
            unsafe { NvsArray::from_raw(result.array_ptr().expect("the member returns an array")) };
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
        let mut array = NvsArray::new();
        array.append(Value::str(NvsStr::new(b"red")));
        array.append(Value::int(1));
        array.append(Value::str(NvsStr::new(b"red")));
        array.append(Value::str(NvsStr::new(b"1")));
        array.append(Value::str(NvsStr::new(b"blue")));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::nvs_core_arr_count_by,
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
        let mut array = NvsArray::new();
        array.append(Value::float(1.5));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::nvs_core_arr_count_by,
            &mut ctx,
            &[Value::array(array), Value::null()],
        )
        .expect_err("a float is not a key");
        assert_eq!(status, nvs_runtime::FATAL);
    }

    /// The deliberate divergence from `array_pad`, which renumbers the integer
    /// keys and keeps the string ones: PHP answers
    /// `{"0":"z","x":"a","1":"b","y":"c"}` for this subject padded on the left
    /// — the padding does not even end up contiguous — where this renumbers
    /// every key, on both sides alike.
    #[test]
    fn padding_renumbers_every_key_rather_than_only_the_integer_ones() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let pad = || Value::str(NvsStr::new(b"z"));

        let started = call(
            super::nvs_core_arr_pad_start,
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
            super::nvs_core_arr_pad_end,
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
        for member in [super::nvs_core_arr_pad_start, super::nvs_core_arr_pad_end] {
            for size in [0u64, 3] {
                let result = call(
                    member,
                    &mut ctx,
                    &[
                        mixed_keys(),
                        Value::uint(size),
                        Value::str(NvsStr::new(b"z")),
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
    /// overload PHP also has is [`super::nvs_core_arr_fill_keys`]'s job here,
    /// and this member's own docs own why.
    // covers: Core\Arr::fill
    #[test]
    fn fill_repeats_one_value_under_a_fresh_run_of_keys() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::nvs_core_arr_fill,
            &mut ctx,
            &[Value::uint(3), Value::str(NvsStr::new(b"v"))],
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
            super::nvs_core_arr_fill,
            &mut ctx,
            &[Value::uint(0), Value::str(NvsStr::new(b"v"))],
        )
        .expect("a count of zero is an empty array, not a throw");
        assert_eq!(entries_of(none), vec![]);
    }

    /// Verified against PHP 8.5's `array_fill_keys`, duplicate collapse
    /// included: the keys array contributes its values, each normalized the
    /// way every other key position normalizes one.
    // covers: Core\Arr::fillKeys
    #[test]
    fn fill_keys_stores_one_value_under_each_distinct_key() {
        let mut keys = NvsArray::new();
        keys.set(NvsStr::new(b"ignored"), Value::str(NvsStr::new(b"a")));
        keys.append(Value::int(10));
        keys.append(Value::str(NvsStr::new(b"a")));
        keys.append(Value::str(NvsStr::new(b"10")));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let result = call(
            super::nvs_core_arr_fill_keys,
            &mut ctx,
            &[Value::array(keys), Value::str(NvsStr::new(b"v"))],
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
    // covers: Core\Arr::fillKeys
    #[test]
    fn filling_under_a_key_that_is_not_a_key_throws() {
        let mut keys = NvsArray::new();
        keys.append(Value::float(1.5));

        let mut ctx = Ctx::new(OutputSink::Sink);
        let status = call(
            super::nvs_core_arr_fill_keys,
            &mut ctx,
            &[Value::array(keys), Value::str(NvsStr::new(b"v"))],
        )
        .expect_err("a float is not a key");
        assert_eq!(status, nvs_runtime::FATAL);
    }

    /// The four ends of a map, over a subject whose insertion order is not its
    /// key order — so an implementation that read the hash rather than the
    /// insertion list would answer differently.
    // covers: Core\Arr::first, Core\Arr::last, Core\Arr::firstKey, Core\Arr::lastKey
    #[test]
    fn the_four_end_members_read_insertion_order() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let asked = |ctx: &mut Ctx, helper: nvs_runtime::NvsFn| {
            rendered_value(call(helper, ctx, &[mixed_keys()]).expect("asking an end never fails"))
        };
        assert_eq!(asked(&mut ctx, super::nvs_core_arr_first), b"a");
        assert_eq!(asked(&mut ctx, super::nvs_core_arr_last), b"c");
        assert_eq!(asked(&mut ctx, super::nvs_core_arr_first_key), b"x");
        assert_eq!(asked(&mut ctx, super::nvs_core_arr_last_key), b"y");
    }

    /// An empty subject answers `null` at all four ends rather than throwing —
    /// `rule:core-api/shape-rules` R5's absence spelling, not R4's failure.
    // covers: Core\Arr::first, Core\Arr::last, Core\Arr::firstKey, Core\Arr::lastKey
    #[test]
    fn an_empty_array_has_no_ends() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        for helper in [
            super::nvs_core_arr_first as nvs_runtime::NvsFn,
            super::nvs_core_arr_last,
            super::nvs_core_arr_first_key,
            super::nvs_core_arr_last_key,
        ] {
            let answer = call(helper, &mut ctx, &[Value::array(NvsArray::new())])
                .expect("an empty array is not a failure");
            assert_eq!(answer.tag_byte(), nvs_runtime::Tag::Null as u8);
        }
    }

    /// Reading an end retains what it hands back: the answer outlives the
    /// subject, so the entry's own reference cannot be the one returned.
    #[test]
    fn an_end_value_is_a_reference_of_its_own() {
        let mut array = NvsArray::new();
        let held = NvsStr::new(b"only");
        let before = held.refcount();
        array.append(Value::str(held));
        let subject = Value::array(array);

        let mut ctx = Ctx::new(OutputSink::Sink);
        let answer =
            call(super::nvs_core_arr_first, &mut ctx, &[subject]).expect("a one-entry array");
        #[expect(
            unsafe_code,
            reason = "the answer is a fresh reference this test owns, and the \
                      subject is the one reference it built above; the handle \
                      below only reads a count, so it must not release one"
        )]
        unsafe {
            let handle =
                std::mem::ManuallyDrop::new(NvsStr::from_raw(answer.str_ptr().expect("a string")));
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
            super::nvs_core_arr_find as nvs_runtime::NvsFn,
            super::nvs_core_arr_find_key,
            super::nvs_core_arr_any,
            super::nvs_core_arr_all,
        ] {
            let status = call(helper, &mut ctx, &[Value::int(1), Value::null()])
                .expect_err("an int is not an array");
            assert_eq!(status, nvs_runtime::FATAL);
        }
    }

    /// The whole of what a shape asks for, over three of the wire types a form
    /// or a program hands one: every field the shape names is on the instance,
    /// at the slot its sorted position gives it.
    #[test]
    fn an_array_hydrates_into_every_field_the_shape_names() {
        let contract = shape_of(&[
            ("flag", CodecTy::Bool, true, false),
            ("n", CodecTy::Int, true, false),
            ("name", CodecTy::Str, true, false),
        ]);
        let subject = array_of([
            ("n", Value::int(7)),
            ("name", Value::str(NvsStr::new(b"ada"))),
            ("flag", Value::bool(true)),
        ]);

        let mut ctx = Ctx::new(OutputSink::Sink);
        let built = shape_as(&mut ctx, contract, subject).expect("every field matched");
        assert_eq!(field_at(built, 0).as_bool(), Some(true));
        assert_eq!(field_at(built, 1).as_int(), Some(7));
        assert_eq!(text_at(built, 2), "ada");

        dropped(built);
        dropped(subject);
    }

    /// `rule:types/conversion`'s `string` → number rows, asserted on both
    /// sides: the whole-string reading is what `as` does, so `"42"` converts
    /// and `"42abc"` does not, and a negative reaches `int` where `uint`
    /// refuses it. Nothing new enters that table for this member — the rows
    /// are `nvs_runtime::to_int` and `nvs_runtime::to_uint`, which are the two
    /// `$mixed as int` and `$mixed as uint` already go through.
    #[test]
    fn a_numeric_string_reaches_an_int_field_by_the_same_rule_as_would() {
        let signed = shape_of(&[("n", CodecTy::Int, true, false)]);
        let unsigned = shape_of(&[("n", CodecTy::Uint, true, false)]);
        let mut ctx = Ctx::new(OutputSink::Sink);

        let asked =
            |ctx: &mut Ctx, contract: (*const ClassDesc, *const ShapeCodec), text: &[u8]| {
                let subject = array_of([("n", Value::str(NvsStr::new(text)))]);
                let answer = shape_as(ctx, contract, subject).map(|built| {
                    let field = field_at(built, 0);
                    let number = field.as_int().or_else(|| {
                        field
                            .as_uint()
                            .map(|value| i64::try_from(value).expect("a test's literals are small"))
                    });
                    dropped(built);
                    number
                });
                dropped(subject);
                answer
            };

        assert_eq!(asked(&mut ctx, signed, b"42"), Ok(Some(42)));
        assert_eq!(asked(&mut ctx, signed, b"-7"), Ok(Some(-7)));
        assert_eq!(asked(&mut ctx, unsigned, b"42"), Ok(Some(42)));
        // The *whole* string, so there is no prefix parse and no empty one.
        assert!(asked(&mut ctx, signed, b"42abc").is_err());
        assert!(asked(&mut ctx, signed, b"").is_err());
        // `string` → `uint` is exact, so the sign is where that row stops.
        assert!(asked(&mut ctx, unsigned, b"-7").is_err());
    }

    /// A value no row converts is one failed field, and the issue names the
    /// field rather than only the call — which is what makes the report usable
    /// by a form. `bool` is the type no conversion lands on at all, so a `"1"`
    /// there is the clearest case of the table being closed.
    #[test]
    fn a_value_as_refuses_is_a_failure_naming_its_field() {
        let contract = shape_of(&[("flag", CodecTy::Bool, true, false)]);
        let subject = array_of([("flag", Value::str(NvsStr::new(b"1")))]);

        let mut ctx = Ctx::new(OutputSink::Sink);
        let failure = hydrated_failure(&mut ctx, contract, subject, crate::json::Reading::Values);
        let issues = issues_of(&failure);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].0, "flag");
        dropped_fault(failure);
    }

    /// `rule:core-api/required-optional-and-nullable`'s first column, asserted
    /// in both directions at once: presence is the field's `required` bit and
    /// nothing else, so an absent required key fails while an absent optional
    /// one leaves its slot unwritten — `rule:types/shape-type`'s "the shape
    /// proves its type and not its presence".
    #[test]
    fn an_absent_required_key_is_a_failure_and_an_absent_optional_one_is_not() {
        let contract = shape_of(&[
            ("a", CodecTy::Int, true, false),
            ("b", CodecTy::Int, false, false),
        ]);
        let mut ctx = Ctx::new(OutputSink::Sink);

        let only_required = array_of([("a", Value::int(1))]);
        let built = shape_as(&mut ctx, contract, only_required)
            .expect("an absent optional key is not a failure");
        assert_eq!(field_at(built, 0).as_int(), Some(1));
        assert_eq!(
            field_at(built, 1).tag(),
            Some(Tag::Unset),
            "an absent optional key is never written, rather than written null"
        );
        dropped(built);
        dropped(only_required);

        let only_optional = array_of([("b", Value::int(2))]);
        assert!(
            shape_as(&mut ctx, contract, only_optional).is_err(),
            "an absent required key is a failure whatever its type admits"
        );
        dropped(only_optional);
    }

    /// `rule:types/shape-type`'s width subtyping over loose values: a source
    /// with more keys than the shape names still satisfies it, the extra key
    /// is not copied onto the instance, and the subject is left as the caller
    /// wrote it.
    #[expect(
        unsafe_code,
        reason = "this test owns the array it built and the descriptor it \
                  leaked, so both are live for the length of the assertions"
    )]
    #[test]
    fn a_key_the_shape_does_not_name_is_left_behind() {
        let contract = shape_of(&[("n", CodecTy::Int, true, false)]);
        let subject = array_of([
            ("n", Value::int(1)),
            ("extra", Value::str(NvsStr::new(b"kept"))),
            ("csrf", Value::int(9)),
        ]);

        let mut ctx = Ctx::new(OutputSink::Sink);
        let built = shape_as(&mut ctx, contract, subject).expect("extra keys are ignored");
        assert_eq!(field_at(built, 0).as_int(), Some(1));

        let source = std::mem::ManuallyDrop::new(unsafe {
            NvsArray::from_raw(subject.array_ptr().expect("an array"))
        });
        assert_eq!(source.count(), 3, "the subject is the caller's, untouched");
        assert_eq!(
            unsafe { &*contract.0 }.field_count(),
            1,
            "the instance carries the shape's slots and no others"
        );

        dropped(built);
        dropped(subject);
    }

    /// `rule:core-classes/derive-reports-every-field` through this door: three
    /// bad fields arrive as **one** error carrying three issues, each at its
    /// own path, rather than as the first of them. Counted rather than read
    /// off a line, because a walk that stopped at its first failure would
    /// still print a plausible issue.
    #[test]
    fn every_failure_of_one_call_arrives_in_one_error_with_its_paths() {
        let contract = shape_of(&[
            ("a", CodecTy::Int, true, false),
            ("b", CodecTy::Int, true, false),
            ("c", CodecTy::Int, true, false),
        ]);
        // `a` does not convert, `b` is absent and `c` is the empty string:
        // three different ways to fail, one report.
        let subject = array_of([
            ("a", Value::str(NvsStr::new(b"x"))),
            ("c", Value::str(NvsStr::new(b""))),
        ]);

        let mut ctx = Ctx::new(OutputSink::Sink);
        let failure = hydrated_failure(&mut ctx, contract, subject, crate::json::Reading::Values);
        let issues = issues_of(&failure);
        assert_eq!(issues.len(), 3);
        let paths: Vec<&str> = issues.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(paths, ["a", "b", "c"], "in the shape's own field order");
        dropped_fault(failure);
    }

    /// The agreement stage 3 of `docs/agent/loop-goal.md` asks for: an inline
    /// shape reached through `Core\Json::decodeAs` and through
    /// `Core\Arr::shapeAs` is **one** walk, so the two answer the same field
    /// for the same input and report a bad one the same way. Asserted as an
    /// agreement rather than as each member's own answer, because a second
    /// walk grown under either door would still look right on its own line.
    #[test]
    fn decode_as_over_an_inline_shape_uses_the_one_hydration_walk() {
        let contract = shape_of(&[("n", CodecTy::Int, true, false)]);
        let mut ctx = Ctx::new(OutputSink::Sink);

        let decoded =
            decode_as(&mut ctx, contract, b"{\"n\": 5}").expect("the document matches the shape");
        let subject = array_of([("n", Value::int(5))]);
        let hydrated = shape_as(&mut ctx, contract, subject).expect("the array matches the shape");
        assert_eq!(
            field_at(decoded, 0).as_int(),
            field_at(hydrated, 0).as_int()
        );
        dropped(decoded);
        dropped(hydrated);
        dropped(subject);

        // And one bad field is one report either way, at the same path: a
        // `true` is no more an `int` in an array than it is in a document, and
        // the two readings differ only in which conversion rows they reach.
        let reported = |ctx: &mut Ctx, reading| {
            let refused = array_of([("n", Value::bool(true))]);
            let failure = hydrated_failure(ctx, contract, refused, reading);
            let issues = issues_of(&failure);
            dropped_fault(failure);
            issues
        };
        let over_values = reported(&mut ctx, crate::json::Reading::Values);
        assert_eq!(over_values, reported(&mut ctx, crate::json::Reading::Wire));
        assert_eq!(over_values.len(), 1);
        assert_eq!(over_values[0].0, "n");

        // The `Wire` half end to end, so the agreement above is over the walk
        // `Core\Json::decodeAs` actually reaches rather than one beside it.
        assert!(decode_as(&mut ctx, contract, b"{\"n\": true}").is_err());
    }

    /// `Core\Json::decodeAs<T>($json)` at the ABI [`shape_as`] describes, one
    /// declared parameter along: the text, then the depth.
    fn decode_as(
        ctx: &mut Ctx,
        contract: (*const ClassDesc, *const ShapeCodec),
        json: &[u8],
    ) -> Result<Value, i32> {
        call(
            crate::json::nvs_core_json_decode_as,
            ctx,
            &[
                Value::class_desc(contract.0),
                Value::bool(false),
                Value::shape_codec(contract.1),
                Value::str(NvsStr::new(json)),
                Value::uint(crate::json::DEFAULT_MAX_DEPTH),
            ],
        )
    }

    /// The walk `shapeAs` is one ABI above, called directly so a failure
    /// arrives as the [`Fault`] carrying its issues.
    ///
    /// Reading the same list back off the *thrown object* would need an
    /// exception class table installed first — `nvs_runtime::Ctx::pending_slot`
    /// answers `None` until then — and that is a fixture none of these
    /// questions otherwise wants.
    ///
    /// `subject` is transferred, because the walk releases what it is handed.
    #[expect(
        unsafe_code,
        reason = "the descriptor is one this module leaked, so it outlives every \
                  instance made from it"
    )]
    fn hydrated(
        ctx: &mut Ctx,
        contract: (*const ClassDesc, *const ShapeCodec),
        subject: Value,
        reading: crate::json::Reading,
    ) -> Result<Value, Fault> {
        unsafe {
            crate::json::hydrate(
                ctx,
                contract.0,
                Some(contract.1),
                subject,
                false,
                reading,
                "Core\\Arr::shapeAs",
            )
        }
    }

    /// [`hydrated`] where the call is expected to fail.
    fn hydrated_failure(
        ctx: &mut Ctx,
        contract: (*const ClassDesc, *const ShapeCodec),
        subject: Value,
        reading: crate::json::Reading,
    ) -> Fault {
        match hydrated(ctx, contract, subject, reading) {
            Ok(built) => {
                dropped(built);
                panic!("the field does not convert, so the walk owed a failure");
            }
            Err(failure) => failure,
        }
    }

    /// Releases what a `ParseError` carries — the issue list is a reference the
    /// fault owns, and a test that read it still owes the drop.
    fn dropped_fault(fault: Fault) {
        if let Fault::ThrownWithSlots(_, _, slots) = fault {
            for (_, value) in slots {
                dropped(value);
            }
        }
    }

    /// A packed list of the `values` given, in order.
    ///
    /// The array owns every entry, so releasing the array is the whole of what
    /// a test using this owes.
    fn list_of(values: &[i64]) -> Value {
        let mut list = NvsArray::new();
        for value in values {
            list.append(Value::int(*value));
        }
        Value::array(list)
    }

    /// How many entries [`below_ten`] has been shown since a walk reset it.
    ///
    /// One counter for every predicate test in this file, so
    /// [`PREDICATE_LOCK`] is what keeps two of them from interleaving: the
    /// test binary runs its cases on several threads.
    static PREDICATE_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    /// Held for the whole of one walk, so that the count read afterwards is
    /// that walk's own.
    static PREDICATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// A one-parameter predicate answering whether its entry is below ten, and
    /// counting the entry it was shown.
    ///
    /// The sweep of the two references is not optional: `call_closure` retains
    /// the receiver and each argument for this callee to release, which is
    /// what a compiled closure body does on its way out.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes the receiver and one argument, each \
                  retained for this callee to release, and `abi::call` passes \
                  the address of a live `Value` for the result — neither is \
                  expressible in the signature compiled code calls through"
    )]
    unsafe extern "C" fn below_ten(_ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32 {
        PREDICATE_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let entry = unsafe { *args.add(1) };
        let answer = entry.as_int().expect("this file's own entries are ints") < 10;
        for index in 0..2 {
            dropped(unsafe { *args.add(index) });
        }
        unsafe {
            *out = Value::bool(answer);
        }
        nvs_runtime::OK
    }

    /// A closure value declaring `arity` parameters, whose body is `invoke`.
    ///
    /// `nvs_runtime::call_closure` reads the arity and the parameter tags off
    /// the *object*'s own two slots rather than off the method row, so a table
    /// carrying one `CLOSURE_INVOKE` row plus those two fields is a whole
    /// closure. Every parameter is tagged `CLOSURE_PARAM_TAG_ANY`, which is
    /// what a `mixed` one gets: a Rust callback declares no type for the tag
    /// check to hold it to.
    ///
    /// The table is leaked for [`shape_of`]'s reason: a descriptor's *address*
    /// is its identity and it must outlive every instance made from it.
    #[expect(
        unsafe_code,
        reason = "the table is leaked, so the descriptor outlives every \
                  instance made from it — `NvsObj::new`'s whole obligation"
    )]
    fn closure_of(arity: usize, invoke: nvs_runtime::NvsFn) -> Value {
        let mut table = ClassTable::new();
        let id = table.define("{closure}", &["arity", "params"], &[]);
        table.set_methods(
            id,
            vec![nvs_runtime::MethodRow {
                name: nvs_runtime::CLOSURE_INVOKE.to_owned(),
                code: invoke as *const u8,
                arity: 0,
                param_tags: 0,
                param_names: Vec::new(),
                param_types: Vec::new(),
                public: true,
                protected: false,
                native: false,
            }],
        );
        table.set_closure(id);
        let table: &'static ClassTable = Box::leak(Box::new(table));
        let object = unsafe { NvsObj::new(table.desc(id)) };
        object.set_field(
            nvs_runtime::CLOSURE_ARITY_SLOT,
            Value::int(i64::try_from(arity).expect("a small arity")),
        );
        let mut tags: u64 = 0;
        for parameter in 0..arity {
            tags |= u64::from(nvs_runtime::CLOSURE_PARAM_TAG_ANY) << (parameter * 4);
        }
        object.set_field(
            nvs_runtime::CLOSURE_PARAM_TAGS_SLOT,
            Value::int(i64::from_ne_bytes(tags.to_ne_bytes())),
        );
        Value::object(object)
    }

    /// What `member` answered over `entries` against [`below_ten`], and how
    /// many entries that predicate was shown.
    fn walked(member: nvs_runtime::NvsFn, entries: &[i64]) -> (Option<bool>, usize) {
        let guard = PREDICATE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut ctx = Ctx::buffered();
        let subject = list_of(entries);
        let predicate = closure_of(1, below_ten);
        PREDICATE_CALLS.store(0, std::sync::atomic::Ordering::Relaxed);
        let answer = call(member, &mut ctx, &[subject, predicate]).expect("the member answered");
        let seen = PREDICATE_CALLS.load(std::sync::atomic::Ordering::Relaxed);
        let verdict = answer.as_bool();
        drop(guard);
        dropped(answer);
        dropped(subject);
        dropped(predicate);
        (verdict, seen)
    }

    /// The walk stops at the first entry its predicate rejects, and answers
    /// `true` over an empty array without calling the predicate at all.
    ///
    /// **Counted rather than read off the verdict.** A member that walked
    /// every entry would answer `false` on the first row too, so the number of
    /// entries the predicate was shown is the only thing telling the two
    /// apart. The empty row is the other half: `true` there is an answer no
    /// entry can have produced.
    // covers: Core\Arr::all
    #[test]
    fn all_stops_at_the_first_entry_its_predicate_rejects() {
        // 40 is the third entry and the first at or above ten.
        assert_eq!(
            walked(super::nvs_core_arr_all, &[3, 4, 40, 5]),
            (Some(false), 3)
        );
        assert_eq!(
            walked(super::nvs_core_arr_all, &[3, 4, 5, 6]),
            (Some(true), 4)
        );
        assert_eq!(walked(super::nvs_core_arr_all, &[]), (Some(true), 0));
    }

    /// The other half of the same walk: it stops at the first entry its
    /// predicate accepts, and answers `false` over an empty array without
    /// calling the predicate at all.
    ///
    /// Asserted beside `all`'s row rather than on its own, because the two
    /// members are one walk under a flag: a `negate` read the wrong way round
    /// leaves each of them plausible alone and the pair disagreeing.
    // covers: Core\Arr::any
    #[test]
    fn any_stops_at_the_first_entry_its_predicate_accepts() {
        // 3 is the third entry and the first below ten.
        assert_eq!(
            walked(super::nvs_core_arr_any, &[40, 50, 3, 60]),
            (Some(true), 3)
        );
        assert_eq!(
            walked(super::nvs_core_arr_any, &[40, 50, 60]),
            (Some(false), 3)
        );
        assert_eq!(walked(super::nvs_core_arr_any, &[]), (Some(false), 0));
    }

    /// What `member` answered over `entries` against [`below_ten`], and how
    /// many entries that predicate was shown.
    ///
    /// The answer comes back as it was given, because the two members reading
    /// this walk answer in different types, and the caller owes its release.
    fn first_match(member: nvs_runtime::NvsFn, entries: &[i64]) -> (Value, usize) {
        let guard = PREDICATE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut ctx = Ctx::buffered();
        let subject = list_of(entries);
        let predicate = closure_of(1, below_ten);
        PREDICATE_CALLS.store(0, std::sync::atomic::Ordering::Relaxed);
        let answer = call(member, &mut ctx, &[subject, predicate]).expect("the member answered");
        let seen = PREDICATE_CALLS.load(std::sync::atomic::Ordering::Relaxed);
        drop(guard);
        dropped(subject);
        dropped(predicate);
        (answer, seen)
    }

    /// The walk stops at the first entry its predicate accepts and answers
    /// that entry, and answers `null` over an array holding no match.
    ///
    /// **The count is asserted beside the value.** A member that walked every
    /// entry and kept the first acceptance would answer 3 here too, so the
    /// number of entries the predicate was shown is the only thing telling the
    /// two apart — and it is the property `filter` plus `first` does not have,
    /// which is why both members exist.
    // covers: Core\Arr::find
    #[test]
    fn find_answers_the_first_accepted_entry_and_stops_there() {
        // 3 is the third entry and the first below ten.
        let (answer, seen) = first_match(super::nvs_core_arr_find, &[40, 50, 3, 4]);
        assert_eq!((answer.as_int(), seen), (Some(3), 3));
        dropped(answer);

        let (answer, seen) = first_match(super::nvs_core_arr_find, &[40, 50, 60]);
        assert_eq!((answer.tag(), seen), (Some(Tag::Null), 3));
        dropped(answer);

        let (answer, seen) = first_match(super::nvs_core_arr_find, &[]);
        assert_eq!((answer.tag(), seen), (Some(Tag::Null), 0));
        dropped(answer);
    }

    /// The other half of the same walk: the key of the first accepted entry,
    /// as a `string` even over a list, and `null` when no entry is accepted.
    ///
    /// Asserted beside `find`'s row rather than on its own, because the two
    /// members share one walk and differ only in what they read off the slot
    /// it stopped at — a member reading the value where the key belongs is
    /// plausible on its own line and fails here.
    // covers: Core\Arr::findKey
    #[test]
    fn find_key_answers_the_key_of_the_first_accepted_entry() {
        // 3 is the third entry and the first below ten, so its key is "2".
        let (answer, seen) = first_match(super::nvs_core_arr_find_key, &[40, 50, 3, 4]);
        assert_eq!((answer.as_str_bytes(), seen), (Some(b"2".as_slice()), 3));
        dropped(answer);

        let (answer, seen) = first_match(super::nvs_core_arr_find_key, &[40, 50, 60]);
        assert_eq!((answer.tag(), seen), (Some(Tag::Null), 3));
        dropped(answer);
    }

    /// What `member` answered over `arguments`, as a `bool`.
    ///
    /// The subject stays the caller's, because a case asking one array two
    /// questions builds it once.
    fn asked(member: nvs_runtime::NvsFn, arguments: &[Value]) -> Option<bool> {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let answer = call(member, &mut ctx, arguments).expect("the member answered");
        let verdict = answer.as_bool();
        dropped(answer);
        verdict
    }

    /// The empty array is the only array this answers `true` for, and an entry
    /// holding `null` is still an entry.
    ///
    /// **Asserted beside [`super::nvs_core_arr_first`]'s answer over the same
    /// array**, which is the pair the member exists for: both see a `null`
    /// there, and only this one says whether an entry is present at all. A
    /// member reading the first value instead of the count is plausible on the
    /// empty row alone and fails here.
    // covers: Core\Arr::isEmpty
    #[test]
    fn is_empty_is_true_only_for_an_array_holding_no_entry() {
        let empty = Value::array(NvsArray::new());
        assert_eq!(asked(super::nvs_core_arr_is_empty, &[empty]), Some(true));

        let mut holding_null = NvsArray::new();
        holding_null.append(Value::null());
        let holding_null = Value::array(holding_null);
        assert_eq!(
            asked(super::nvs_core_arr_is_empty, &[holding_null]),
            Some(false)
        );

        let mut ctx = Ctx::new(OutputSink::Sink);
        let first = call(super::nvs_core_arr_first, &mut ctx, &[holding_null])
            .expect("an empty array is not a failure");
        assert_eq!(
            first.tag(),
            Some(Tag::Null),
            "the value is null, and the entry is still there"
        );
        dropped(first);

        dropped(holding_null);
        dropped(empty);
    }

    /// An `int` and the text of that `int` name one entry, and a key holding
    /// `null` is present.
    ///
    /// **Asked of `10` and `"10"` together**, because a member comparing the
    /// tag before the bytes answers one of them correctly and the other not,
    /// and either row alone looks right. `crate::arr`'s `key_bytes` owns the
    /// decoding this rests on.
    // covers: Core\Arr::hasKey
    #[test]
    fn has_key_reads_one_entry_under_an_int_key_and_its_text() {
        let subject = mixed_keys();
        let ten = Value::str(NvsStr::new(b"10"));
        let absent = Value::str(NvsStr::new(b"z"));
        assert_eq!(
            asked(super::nvs_core_arr_has_key, &[subject, Value::int(10)]),
            Some(true)
        );
        assert_eq!(
            asked(super::nvs_core_arr_has_key, &[subject, ten]),
            Some(true)
        );
        assert_eq!(
            asked(super::nvs_core_arr_has_key, &[subject, absent]),
            Some(false)
        );
        dropped(absent);
        dropped(ten);
        dropped(subject);

        let mut holding_null = NvsArray::new();
        holding_null.set(NvsStr::new(b"note"), Value::null());
        let holding_null = Value::array(holding_null);
        let note = Value::str(NvsStr::new(b"note"));
        assert_eq!(
            asked(super::nvs_core_arr_has_key, &[holding_null, note]),
            Some(true),
            "a stored null is an entry the key names"
        );
        dropped(note);
        dropped(holding_null);
    }

    /// The search stops at the first identical entry, and the empty array holds
    /// no entry to stop at.
    ///
    /// **`1` is asked of an array holding `1.0`**, which is the row separating
    /// this member from `in_array($n, $a, true)`: `int`, `uint`, `float` and
    /// `decimal` are one numeric domain here, so a member comparing the tag
    /// before the number answers `false` there and looks right on every other
    /// row. `"3"` is asked of a list holding `3` for the other side of the same
    /// comparison, since a member that converted instead would answer `true`.
    // covers: Core\Arr::contains
    #[test]
    fn contains_is_true_at_the_first_identical_entry_across_the_numeric_domain() {
        let list = list_of(&[40, 50, 3]);
        let three = Value::str(NvsStr::new(b"3"));
        assert_eq!(
            asked(super::nvs_core_arr_contains, &[list, Value::int(3)]),
            Some(true)
        );
        assert_eq!(
            asked(super::nvs_core_arr_contains, &[list, Value::int(4)]),
            Some(false)
        );
        assert_eq!(
            asked(super::nvs_core_arr_contains, &[list, three]),
            Some(false),
            "a text is not the number it spells"
        );
        dropped(three);
        dropped(list);

        let mut fractional = NvsArray::new();
        fractional.append(Value::float(1.0));
        let fractional = Value::array(fractional);
        assert_eq!(
            asked(super::nvs_core_arr_contains, &[fractional, Value::int(1)]),
            Some(true),
            "one numeric domain: the int 1 is the entry 1.0"
        );
        dropped(fractional);

        let empty = Value::array(NvsArray::new());
        assert_eq!(
            asked(super::nvs_core_arr_contains, &[empty, Value::int(1)]),
            Some(false)
        );
        dropped(empty);
    }

    /// The other half of that walk: the key of the first identical entry, as a
    /// `string` even over a list, and `null` when no entry is identical.
    ///
    /// **Asked of an array holding the needle twice**, because the first of the
    /// two is the answer and a member reading the last one passes every
    /// [`super::nvs_core_arr_contains`] row above. The key comes back in the
    /// spelling it is stored under, which [`mixed_keys`]'s `"10"` is what shows.
    // covers: Core\Arr::keyOf
    #[test]
    fn key_of_answers_the_key_of_the_first_identical_entry() {
        let list = list_of(&[40, 3, 3]);
        let mut ctx = Ctx::new(OutputSink::Sink);
        let answer = call(super::nvs_core_arr_key_of, &mut ctx, &[list, Value::int(3)])
            .expect("the member answered");
        assert_eq!(
            answer.as_str_bytes(),
            Some(b"1".as_slice()),
            "the first of the two, as a string even over a list"
        );
        dropped(answer);

        let answer = call(super::nvs_core_arr_key_of, &mut ctx, &[list, Value::int(9)])
            .expect("the member answered");
        assert_eq!(answer.tag(), Some(Tag::Null));
        dropped(answer);
        dropped(list);

        let subject = mixed_keys();
        let needle = Value::str(NvsStr::new(b"b"));
        let answer = call(super::nvs_core_arr_key_of, &mut ctx, &[subject, needle])
            .expect("the member answered");
        assert_eq!(
            answer.as_str_bytes(),
            Some(b"10".as_slice()),
            "the stored spelling of the key, never an int"
        );
        dropped(answer);
        dropped(needle);
        dropped(subject);
    }

    /// The trailing values of a variadic call, at the ABI the member reads
    /// them at: one array holding them in written order, under `"0"`, `"1"`,
    /// … — which is the tail a compiled call site packs.
    fn trailing(values: &[&str]) -> Value {
        let mut tail = NvsArray::new();
        for value in values {
            tail.append(Value::str(NvsStr::new(value.as_bytes())));
        }
        Value::array(tail)
    }

    /// Every key of the subject survives, and each added value lands one past
    /// the largest integer key present — `"11"` and `"12"` here, because
    /// `"10"` is the largest and `"x"` and `"y"` are not integer keys at all.
    ///
    /// **Asserted over the keys, not the values.** A member that renumbered
    /// its whole result would hold the same values in the same order and
    /// differ only in what they are stored under, so reading the keys back is
    /// the only thing telling the two apart. The empty tail is the other half:
    /// it adds nothing, so it may change nothing.
    // covers: Core\Arr::append
    #[test]
    fn append_keeps_every_key_and_lands_one_past_the_largest_integer_key() {
        let mut ctx = Ctx::new(OutputSink::Sink);

        let subject = mixed_keys();
        let tail = trailing(&["d", "e"]);
        let result = call(super::nvs_core_arr_append, &mut ctx, &[subject, tail])
            .expect("a free integer key never fails");
        assert_eq!(
            entries_of(result),
            vec![
                (b"x".to_vec(), b"a".to_vec()),
                (b"10".to_vec(), b"b".to_vec()),
                (b"y".to_vec(), b"c".to_vec()),
                (b"11".to_vec(), b"d".to_vec()),
                (b"12".to_vec(), b"e".to_vec()),
            ]
        );
        dropped(tail);

        let empty = trailing(&[]);
        let same = call(super::nvs_core_arr_append, &mut ctx, &[subject, empty])
            .expect("adding nothing never fails");
        assert_eq!(
            entries_of(same),
            vec![
                (b"x".to_vec(), b"a".to_vec()),
                (b"10".to_vec(), b"b".to_vec()),
                (b"y".to_vec(), b"c".to_vec()),
            ]
        );
        dropped(empty);
        dropped(subject);
    }

    /// A subject already holding the largest integer key has no next one to
    /// derive, so the member throws where `$a[] = $v` throws.
    ///
    /// The result this member builds is a copy of the subject, so it inherits
    /// that key and reaches an append `nvs_runtime::NvsArray::append` asserts
    /// against. Asserted here rather than only in the `.nvst` case because
    /// what it guards is a panic: a status is a refusal a request survives,
    /// and a panic takes the process and every other request with it.
    // covers: Core\Arr::append
    #[test]
    fn append_refuses_where_the_next_integer_key_is_taken() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let subject = array_of([("9223372036854775807", Value::str(NvsStr::new(b"a")))]);
        let tail = trailing(&["b"]);

        let status = call(super::nvs_core_arr_append, &mut ctx, &[subject, tail])
            .expect_err("the next integer key is taken");
        assert_eq!(status, nvs_runtime::THROWN);

        // The subject is untouched, which is what building a result buys.
        let empty = trailing(&[]);
        let kept = call(super::nvs_core_arr_append, &mut ctx, &[subject, empty])
            .expect("adding nothing never fails");
        assert_eq!(
            entries_of(kept),
            vec![(b"9223372036854775807".to_vec(), b"a".to_vec())]
        );
        dropped(empty);
        dropped(tail);
        dropped(subject);
    }

    /// The values put in front keep the order they were written in, and the
    /// result is a list: every key of the subject is renumbered from `"0"`.
    ///
    /// Asserted beside [`append_keeps_every_key_and_lands_one_past_the_largest_integer_key`]
    /// rather than on its own, because the pair is the asymmetry: a value
    /// added at the end has a free key to take and one put in front does not,
    /// so a member that mirrored the other here would keep `"x"` and `"10"`
    /// and still look right on its own row.
    // covers: Core\Arr::prepend
    #[test]
    fn prepend_writes_its_values_in_order_and_renumbers_the_whole_result() {
        let mut ctx = Ctx::new(OutputSink::Sink);

        let subject = mixed_keys();
        let tail = trailing(&["d", "e"]);
        let result = call(super::nvs_core_arr_prepend, &mut ctx, &[subject, tail])
            .expect("prepending never fails");
        assert_eq!(
            entries_of(result),
            vec![
                (b"0".to_vec(), b"d".to_vec()),
                (b"1".to_vec(), b"e".to_vec()),
                (b"2".to_vec(), b"a".to_vec()),
                (b"3".to_vec(), b"b".to_vec()),
                (b"4".to_vec(), b"c".to_vec()),
            ]
        );
        dropped(tail);
        dropped(subject);

        // The key `Core\Arr::append` has no successor for is renumbered like
        // any other, so this side has nothing to refuse.
        let full = array_of([("9223372036854775807", Value::str(NvsStr::new(b"a")))]);
        let one = trailing(&["b"]);
        let renumbered = call(super::nvs_core_arr_prepend, &mut ctx, &[full, one])
            .expect("prepending never fails");
        assert_eq!(
            entries_of(renumbered),
            vec![
                (b"0".to_vec(), b"b".to_vec()),
                (b"1".to_vec(), b"a".to_vec()),
            ]
        );
        dropped(one);
        dropped(full);
    }

    /// Every entry that survives keeps the key it was stored under, and a
    /// subject of one entry or none answers the empty array.
    ///
    /// **Asserted over the keys, not the values.** A member that renumbered
    /// its result would hold the same values in the same order and differ
    /// only in what they are stored under, so reading the keys back is the
    /// only thing telling the two apart. The short subjects are the other
    /// half: PHP's `array_shift` answers `null` and warns where there is
    /// nothing to remove, and this answers an array a caller can walk.
    // covers: Core\Arr::withoutFirst
    #[test]
    fn without_first_keeps_the_key_of_every_entry_that_survives() {
        let mut ctx = Ctx::new(OutputSink::Sink);

        let subject = mixed_keys();
        let result = call(super::nvs_core_arr_without_first, &mut ctx, &[subject])
            .expect("dropping the first entry never fails");
        assert_eq!(
            entries_of(result),
            vec![
                (b"10".to_vec(), b"b".to_vec()),
                (b"y".to_vec(), b"c".to_vec()),
            ]
        );
        dropped(subject);

        let one = list_of(&[7]);
        let emptied = call(super::nvs_core_arr_without_first, &mut ctx, &[one])
            .expect("dropping the first entry never fails");
        assert!(entries_of(emptied).is_empty());
        dropped(one);

        let none = list_of(&[]);
        let still_empty = call(super::nvs_core_arr_without_first, &mut ctx, &[none])
            .expect("dropping the first entry never fails");
        assert!(entries_of(still_empty).is_empty());
        dropped(none);
    }

    /// The other end of the same shortening: every entry but the last
    /// survives under the key it was stored under, and a subject of one
    /// entry or none answers the empty array.
    ///
    /// Asserted beside [`without_first_keeps_the_key_of_every_entry_that_survives`]
    /// rather than on its own, because the two are one walk taken from
    /// opposite ends: a member that dropped the entry at the wrong end still
    /// answers two entries under two of the subject's own keys, so only the
    /// pair says which end went.
    // covers: Core\Arr::withoutLast
    #[test]
    fn without_last_keeps_the_key_of_every_entry_that_survives() {
        let mut ctx = Ctx::new(OutputSink::Sink);

        let subject = mixed_keys();
        let result = call(super::nvs_core_arr_without_last, &mut ctx, &[subject])
            .expect("dropping the last entry never fails");
        assert_eq!(
            entries_of(result),
            vec![
                (b"x".to_vec(), b"a".to_vec()),
                (b"10".to_vec(), b"b".to_vec()),
            ]
        );
        dropped(subject);

        let one = list_of(&[7]);
        let emptied = call(super::nvs_core_arr_without_last, &mut ctx, &[one])
            .expect("dropping the last entry never fails");
        assert!(entries_of(emptied).is_empty());
        dropped(one);

        let none = list_of(&[]);
        let still_empty = call(super::nvs_core_arr_without_last, &mut ctx, &[none])
            .expect("dropping the last entry never fails");
        assert!(entries_of(still_empty).is_empty());
        dropped(none);
    }

    /// The padding lands in front of the subject's own values, and the whole
    /// result is renumbered from `"0"` whatever the subject's keys were.
    ///
    /// **The unpadded row is the half that pins the renumbering.** A subject
    /// already at the wanted length has nothing added, so the keys are the
    /// only thing that moved — a member that returned the subject as it
    /// stands would answer the same three values in the same order and still
    /// be wrong, since [`nvs_core_arr_pad_start`]'s own docs make the list
    /// the result's shape rather than a side effect of padding.
    // covers: Core\Arr::padStart
    #[test]
    fn pad_start_writes_its_copies_in_front_and_renumbers_the_whole_result() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let subject = mixed_keys();
        let filler = Value::str(NvsStr::new(b"-"));

        let result = call(
            super::nvs_core_arr_pad_start,
            &mut ctx,
            &[subject, Value::uint(5), filler],
        )
        .expect("a length this short is always affordable");
        assert_eq!(
            entries_of(result),
            vec![
                (b"0".to_vec(), b"-".to_vec()),
                (b"1".to_vec(), b"-".to_vec()),
                (b"2".to_vec(), b"a".to_vec()),
                (b"3".to_vec(), b"b".to_vec()),
                (b"4".to_vec(), b"c".to_vec()),
            ]
        );

        let unpadded = call(
            super::nvs_core_arr_pad_start,
            &mut ctx,
            &[subject, Value::uint(2), filler],
        )
        .expect("a length this short is always affordable");
        assert_eq!(
            entries_of(unpadded),
            vec![
                (b"0".to_vec(), b"a".to_vec()),
                (b"1".to_vec(), b"b".to_vec()),
                (b"2".to_vec(), b"c".to_vec()),
            ]
        );

        dropped(filler);
        dropped(subject);
    }

    /// The other side of the same padding: the copies land after the
    /// subject's own values, and the whole result is renumbered from `"0"`.
    ///
    /// Asserted beside [`pad_start_writes_its_copies_in_front_and_renumbers_the_whole_result`]
    /// rather than on its own, because the two are one walk with the padding
    /// on the other side: a member that padded the wrong side still answers
    /// five entries under `"0"` to `"4"`, so only the pair says which side
    /// the copies went to.
    // covers: Core\Arr::padEnd
    #[test]
    fn pad_end_writes_its_copies_after_the_last_value_and_renumbers_the_whole_result() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let subject = mixed_keys();
        let filler = Value::str(NvsStr::new(b"-"));

        let result = call(
            super::nvs_core_arr_pad_end,
            &mut ctx,
            &[subject, Value::uint(5), filler],
        )
        .expect("a length this short is always affordable");
        assert_eq!(
            entries_of(result),
            vec![
                (b"0".to_vec(), b"a".to_vec()),
                (b"1".to_vec(), b"b".to_vec()),
                (b"2".to_vec(), b"c".to_vec()),
                (b"3".to_vec(), b"-".to_vec()),
                (b"4".to_vec(), b"-".to_vec()),
            ]
        );

        let unpadded = call(
            super::nvs_core_arr_pad_end,
            &mut ctx,
            &[subject, Value::uint(2), filler],
        )
        .expect("a length this short is always affordable");
        assert_eq!(
            entries_of(unpadded),
            vec![
                (b"0".to_vec(), b"a".to_vec()),
                (b"1".to_vec(), b"b".to_vec()),
                (b"2".to_vec(), b"c".to_vec()),
            ]
        );

        dropped(filler);
        dropped(subject);
    }

    /// What [`nvs_core_arr_count`] reports for `subject`, beside the number of
    /// entries a cursor walk finds in it.
    ///
    /// Consumes the one reference the caller built: the handle below takes it
    /// over and releases it on drop.
    fn counted_and_walked(subject: Value) -> (u64, u64) {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let counted = call(super::nvs_core_arr_count, &mut ctx, &[subject])
            .expect("counting an array never fails")
            .as_uint()
            .expect("the member returns a uint");
        #[expect(
            unsafe_code,
            reason = "the handle takes over the one reference the caller built \
                      above, and releases it on drop"
        )]
        let array = unsafe { NvsArray::from_raw(subject.array_ptr().expect("an array")) };
        let mut walked = 0u64;
        let mut from = 0usize;
        while let Some(slot) = array.next_slot(from) {
            walked += 1;
            from = slot + 1;
        }
        (counted, walked)
    }

    /// A one-parameter callback answering a fresh forty-character text for every
    /// entry it is shown.
    ///
    /// Long enough that the answer is a heap allocation of its own, which is
    /// what [`map_answers_are_stored_with_the_one_reference_they_arrived_with`]
    /// reads a reference count off.
    ///
    /// The sweep of the two references is not optional, for
    /// [`below_ten`]'s reason.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes the receiver and one argument, each \
                  retained for this callee to release, and `abi::call` passes \
                  the address of a live `Value` for the result — neither is \
                  expressible in the signature compiled code calls through"
    )]
    unsafe extern "C" fn labels_every_entry(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        for index in 0..2 {
            dropped(unsafe { *args.add(index) });
        }
        unsafe {
            *out = Value::str(NvsStr::new(b"a label long enough to be its own text"));
        }
        nvs_runtime::OK
    }

    /// Every entry of a mapped result holds exactly one reference to the value
    /// the callback answered with, under the subject's own key.
    ///
    /// **Only the Rust side can see this.** A retain on the way into the result
    /// would leave every mapped value one reference over, which is a leak per
    /// entry and prints nothing wrong from Novis: the values are right, the keys
    /// are right, and the program ends. [`nvs_core_arr_map`]'s docs state the
    /// rule this reads — the callback's answer is already owned by the frame
    /// storing it, which is what makes `map` different from
    /// [`nvs_core_arr_filter`].
    // covers: Core\Arr::map
    #[test]
    fn map_answers_are_stored_with_the_one_reference_they_arrived_with() {
        let mut ctx = Ctx::buffered();
        let subject = mixed_keys();
        let mapper = closure_of(1, labels_every_entry);

        let result = call(super::nvs_core_arr_map, &mut ctx, &[subject, mapper])
            .expect("this callback never fails");
        #[expect(
            unsafe_code,
            reason = "the handle takes over the one reference the member \
                      answered with, and releases it on drop"
        )]
        let mapped =
            unsafe { NvsArray::from_raw(result.array_ptr().expect("map answers an array")) };
        let mut entries = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = mapped.next_slot(from) {
            let key = mapped.key_at(slot).expect("a live entry has a key");
            let value = mapped.value_at(slot).expect("a live entry has a value");
            #[expect(
                unsafe_code,
                reason = "the entry owns the text for as long as this array \
                          holds it, and the handle above holds the array"
            )]
            let references = unsafe {
                NvsStr::refcount_of(value.str_ptr().expect("the callback answers a text"))
            };
            entries.push((key.as_bytes().to_vec(), references));
            from = slot + 1;
        }
        assert_eq!(
            entries,
            vec![(b"x".to_vec(), 1), (b"10".to_vec(), 1), (b"y".to_vec(), 1)]
        );

        dropped(subject);
        dropped(mapper);
    }

    /// `["x" => …, "10" => …, "y" => …]` over values long enough to be heap
    /// allocations of their own, which is what
    /// [`map_keys_stores_the_subject_value_with_a_reference_of_its_own`] reads a
    /// reference count off — [`mixed_keys`]'s one-character values are too
    /// short to have one.
    fn mixed_keys_holding_long_values() -> Value {
        let mut array = NvsArray::new();
        for (key, value) in [
            (
                &b"x"[..],
                &b"a text long enough to be its own allocation: a"[..],
            ),
            (
                &b"10"[..],
                &b"a text long enough to be its own allocation: b"[..],
            ),
            (
                &b"y"[..],
                &b"a text long enough to be its own allocation: c"[..],
            ),
        ] {
            array.set(NvsStr::new(key), Value::str(NvsStr::new(value)));
        }
        Value::array(array)
    }

    /// A two-parameter callback naming each entry after the key it was shown,
    /// so the three new keys are distinct and none of them collapses.
    ///
    /// The answer is a fresh text this frame owns, which is the reference
    /// [`nvs_core_arr_map_keys`] releases once it has read the bytes.
    ///
    /// The sweep of the three references is not optional, for [`below_ten`]'s
    /// reason.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes the receiver and two arguments, each \
                  retained for this callee to release, and `abi::call` passes \
                  the address of a live `Value` for the result — neither is \
                  expressible in the signature compiled code calls through"
    )]
    unsafe extern "C" fn names_the_entry_after_its_key(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        let mut named = b"under ".to_vec();
        named.extend_from_slice(
            unsafe { *args.add(2) }
                .as_str_bytes()
                .expect("the key arrives as a text"),
        );
        for index in 0..3 {
            dropped(unsafe { *args.add(index) });
        }
        unsafe {
            *out = Value::str(NvsStr::new(&named));
        }
        nvs_runtime::OK
    }

    /// Every value of the result is the subject's own, held with a reference of
    /// its own, under the key the callback named.
    ///
    /// **Only the Rust side can see the reference.** The value is stored twice
    /// and owned once, so a member that stored it without retaining prints the
    /// right text from Novis for as long as the subject is alive and frees it
    /// twice at the end, and one that retained twice leaks a text per entry
    /// while printing exactly the same thing. This is the obligation
    /// [`nvs_core_arr_map_keys`] carries and [`nvs_core_arr_map`] does not: the
    /// callback's answer is a *key* here, so the value it is stored against was
    /// never this frame's to give away.
    // covers: Core\Arr::mapKeys
    #[test]
    fn map_keys_stores_the_subject_value_with_a_reference_of_its_own() {
        let mut ctx = Ctx::buffered();
        let subject = mixed_keys_holding_long_values();
        let namer = closure_of(2, names_the_entry_after_its_key);

        let result = call(super::nvs_core_arr_map_keys, &mut ctx, &[subject, namer])
            .expect("this callback never fails");
        #[expect(
            unsafe_code,
            reason = "the handle takes over the one reference the member \
                      answered with, and releases it on drop"
        )]
        let rekeyed =
            unsafe { NvsArray::from_raw(result.array_ptr().expect("mapKeys answers an array")) };
        let mut entries = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = rekeyed.next_slot(from) {
            let key = rekeyed.key_at(slot).expect("a live entry has a key");
            let value = rekeyed.value_at(slot).expect("a live entry has a value");
            #[expect(
                unsafe_code,
                reason = "the subject holds the text for the length of this \
                          test, and the handle above holds the result"
            )]
            let references = unsafe {
                NvsStr::refcount_of(value.str_ptr().expect("every value here is a text"))
            };
            entries.push((key.as_bytes().to_vec(), references));
            from = slot + 1;
        }
        assert_eq!(
            entries,
            vec![
                (b"under x".to_vec(), 2),
                (b"under 10".to_vec(), 2),
                (b"under y".to_vec(), 2)
            ]
        );

        dropped(subject);
        dropped(namer);
    }

    /// A one-parameter callback naming a group after the last character of the
    /// entry's own text, and counting the entry it was shown.
    ///
    /// The count is what
    /// [`group_by_shows_each_entry_to_its_callback_once_and_keeps_the_keys`]
    /// reads, so that walk holds [`PREDICATE_LOCK`].
    ///
    /// The sweep of the two references is not optional, for [`below_ten`]'s
    /// reason.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes the receiver and one argument, each \
                  retained for this callee to release, and `abi::call` passes \
                  the address of a live `Value` for the result — neither is \
                  expressible in the signature compiled code calls through"
    )]
    unsafe extern "C" fn groups_by_the_last_character(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        PREDICATE_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let entry = unsafe { *args.add(1) };
        let name = [*entry
            .as_str_bytes()
            .and_then(<[u8]>::last)
            .expect("this test's own entries are texts")];
        for index in 0..2 {
            dropped(unsafe { *args.add(index) });
        }
        unsafe {
            *out = Value::str(NvsStr::new(&name));
        }
        nvs_runtime::OK
    }

    /// The callback is shown every entry exactly once, the groups come out in
    /// first-occurrence order, and each entry sits in its group under the key it
    /// already had.
    ///
    /// **The count is the half only the Rust side can see.** A member that sized
    /// the groups in one pass and filled them in a second would answer exactly
    /// these groups, in exactly this order, and call a program's function twice
    /// per entry — which is a doubled cost for any caller whose callback is
    /// slow, and nothing a program can observe unless the callback counts
    /// itself. [`nvs_core_arr_group_by`]'s docs state the single pass this
    /// reads, and the keys ride along because they are what tells a group
    /// *which* entries landed in it.
    // covers: Core\Arr::groupBy
    #[test]
    fn group_by_shows_each_entry_to_its_callback_once_and_keeps_the_keys() {
        let guard = PREDICATE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut ctx = Ctx::buffered();
        let mut array = NvsArray::new();
        for (key, value) in [
            (&b"x"[..], &b"one: a"[..]),
            (&b"10"[..], &b"two: b"[..]),
            (&b"y"[..], &b"three: a"[..]),
        ] {
            array.set(NvsStr::new(key), Value::str(NvsStr::new(value)));
        }
        let subject = Value::array(array);
        let namer = closure_of(1, groups_by_the_last_character);

        PREDICATE_CALLS.store(0, std::sync::atomic::Ordering::Relaxed);
        let result = call(super::nvs_core_arr_group_by, &mut ctx, &[subject, namer])
            .expect("this callback never fails");
        let seen = PREDICATE_CALLS.load(std::sync::atomic::Ordering::Relaxed);
        drop(guard);

        #[expect(
            unsafe_code,
            reason = "the handle takes over the one reference the member \
                      answered with, and releases it on drop"
        )]
        let grouped =
            unsafe { NvsArray::from_raw(result.array_ptr().expect("groupBy answers an array")) };
        let mut groups = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = grouped.next_slot(from) {
            let name = grouped.key_at(slot).expect("a live entry has a key");
            let held = grouped.value_at(slot).expect("a live entry has a value");
            #[expect(
                unsafe_code,
                reason = "the retain pairs with the release this handle makes on \
                          drop, so walking a group leaves the result holding it"
            )]
            let group = unsafe {
                held.retain();
                NvsArray::from_raw(held.array_ptr().expect("every group is an array"))
            };
            let mut keys = Vec::new();
            let mut inner = 0usize;
            while let Some(entry) = group.next_slot(inner) {
                keys.push(
                    group
                        .key_at(entry)
                        .expect("a live entry has a key")
                        .as_bytes()
                        .to_vec(),
                );
                inner = entry + 1;
            }
            groups.push((name.as_bytes().to_vec(), keys));
            from = slot + 1;
        }

        assert_eq!(seen, 3);
        assert_eq!(
            groups,
            vec![
                (b"a".to_vec(), vec![b"x".to_vec(), b"y".to_vec()]),
                (b"b".to_vec(), vec![b"10".to_vec()])
            ]
        );

        dropped(subject);
        dropped(namer);
    }

    /// A two-parameter fold answering a fresh forty-character text for every
    /// entry, and never the carry it was shown.
    ///
    /// Long enough that each answer is a heap allocation of its own, which is
    /// what [`reduce_holds_one_reference_to_the_carry_for_the_whole_fold`] reads
    /// a reference count off: an answer the member stored without releasing what
    /// it replaced is a text leaked per entry.
    ///
    /// The sweep of the three references is not optional, for [`below_ten`]'s
    /// reason.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes the receiver and two arguments, each \
                  retained for this callee to release, and `abi::call` passes \
                  the address of a live `Value` for the result — neither is \
                  expressible in the signature compiled code calls through"
    )]
    unsafe extern "C" fn drops_the_carry_for_a_text_of_its_own(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        for index in 0..3 {
            dropped(unsafe { *args.add(index) });
        }
        unsafe {
            *out = Value::str(NvsStr::new(b"a carry long enough to be its own text"));
        }
        nvs_runtime::OK
    }

    /// The seed comes back out of the fold with the caller's own reference and
    /// no other, and the answer carries exactly one.
    ///
    /// **Only the Rust side can see either number.** The member takes a
    /// reference to `$initial` and swaps a fresh answer in for it on the first
    /// entry, so a member that released the seed and one that did not print the
    /// same text from Novis for as long as the caller holds it — and the second
    /// leaks one text per fold, plus one per entry for every carry after it.
    /// [`Extracted`] is the guard that makes both releases run on the throwing
    /// path too, and [`nvs_core_arr_reduce`]'s docs state the rule this reads.
    // covers: Core\Arr::reduce
    #[test]
    fn reduce_holds_one_reference_to_the_carry_for_the_whole_fold() {
        let mut ctx = Ctx::buffered();
        let subject = mixed_keys();
        let folder = closure_of(2, drops_the_carry_for_a_text_of_its_own);
        let seed = Value::str(NvsStr::new(b"a seed long enough to be its own text"));

        let answer = call(
            super::nvs_core_arr_reduce,
            &mut ctx,
            &[subject, folder, seed],
        )
        .expect("this callback never fails");
        #[expect(
            unsafe_code,
            reason = "this frame still owns the seed it built, and the array \
                      below is the one the member answered with"
        )]
        let (held, answered) = unsafe {
            (
                NvsStr::refcount_of(seed.str_ptr().expect("the seed is a text")),
                NvsStr::refcount_of(answer.str_ptr().expect("the callback answers a text")),
            )
        };
        assert_eq!((held, answered), (1, 1));
        assert_eq!(
            answer.as_str_bytes(),
            Some(&b"a carry long enough to be its own text"[..])
        );

        dropped(answer);
        dropped(seed);
        dropped(subject);
        dropped(folder);
    }

    /// A one-parameter predicate keeping every entry whose value is not `"b"`,
    /// and answering with an `int` where a program could only write a `bool`.
    ///
    /// The verdict is what this fixture is for: [`nvs_core_arr_filter`] reads it
    /// through `nvs_runtime::value_truthy`, so `7` keeps an entry and `0` drops
    /// one, and a member reading the slot's own tag as a `bool` finds neither.
    ///
    /// The sweep of the two references is not optional, for
    /// [`below_ten`]'s reason.
    #[expect(
        unsafe_code,
        reason = "`call_closure` passes the receiver and one argument, each \
                  retained for this callee to release, and `abi::call` passes \
                  the address of a live `Value` for the result — neither is \
                  expressible in the signature compiled code calls through"
    )]
    unsafe extern "C" fn keeps_what_is_not_b(
        _ctx: *mut Ctx,
        args: *const Value,
        out: *mut Value,
    ) -> i32 {
        let entry = unsafe { *args.add(1) };
        let keep = entry.as_str_bytes() != Some(&b"b"[..]);
        for index in 0..2 {
            dropped(unsafe { *args.add(index) });
        }
        unsafe {
            *out = Value::int(if keep { 7 } else { 0 });
        }
        nvs_runtime::OK
    }

    /// The result carries the subject's own keys, and a verdict that is not a
    /// `bool` decides an entry the way a condition would.
    ///
    /// **Both halves need the Rust side.** The registry row declares
    /// `callable(T, string): bool`, so no program can hand this member an `int`
    /// verdict, and nothing reachable from Novis tells a member reading
    /// `nvs_runtime::value_truthy` from one reading the slot's tag. The keys
    /// ride along because `"10"` is the key a renumbering would leave looking
    /// plausible: a member that rebuilt the result as a list would answer the
    /// same two values in the same order under `"0"` and `"1"`.
    // covers: Core\Arr::filter
    #[test]
    fn filter_keeps_the_subject_keys_and_reads_a_verdict_that_is_not_a_bool() {
        let mut ctx = Ctx::buffered();
        let subject = mixed_keys();
        let predicate = closure_of(1, keeps_what_is_not_b);

        let result = call(super::nvs_core_arr_filter, &mut ctx, &[subject, predicate])
            .expect("this predicate never fails");
        assert_eq!(
            entries_of(result),
            vec![
                (b"x".to_vec(), b"a".to_vec()),
                (b"y".to_vec(), b"c".to_vec())
            ]
        );

        dropped(subject);
        dropped(predicate);
    }

    /// The count is the number of live entries, and it agrees with the cursor a
    /// `foreach` walks over every shape of key this module builds.
    ///
    /// **Counted against that walk rather than read off a literal alone.** A
    /// member reporting the highest key plus one is right on a packed list and
    /// wrong on the subject whose two keys are a million apart, and one
    /// reporting how many `set` calls were made is wrong where the second call
    /// named a key the first had already written. Both still report a plausible
    /// number on their own line, so the walk is what tells them apart.
    // covers: Core\Arr::count
    #[test]
    fn count_agrees_with_the_walk_over_gaps_overwrites_and_the_empty_array() {
        assert_eq!(counted_and_walked(Value::array(NvsArray::new())), (0, 0));
        assert_eq!(counted_and_walked(mixed_keys()), (3, 3));

        let mut far = NvsArray::new();
        far.set(NvsStr::new(b"0"), Value::int(1));
        far.set(NvsStr::new(b"1000000"), Value::int(2));
        assert_eq!(counted_and_walked(Value::array(far)), (2, 2));

        let mut overwritten = NvsArray::new();
        overwritten.set(NvsStr::new(b"x"), Value::int(1));
        overwritten.set(NvsStr::new(b"x"), Value::int(2));
        assert_eq!(counted_and_walked(Value::array(overwritten)), (1, 1));
    }
}
