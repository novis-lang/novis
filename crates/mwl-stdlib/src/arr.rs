//! `Core\Arr` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 2, over `mwl_runtime`'s insertion-ordered, copy-on-write `array<T>`.
//!
//! Every member here is pure (ADR 0063 R3) and borrows its subject rather
//! than consuming it — see [`crate`]'s own docs for why that falls out of
//! being a helper rather than being a rule this module states.

use mwl_runtime::{Fault, MwlArray, MwlStr, Tag, Value};

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
};

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

/// `int|string` — ADR 0007 § 5's two array-key types, which the spec's § 2
/// writes at every member taking or producing a key.
const ARRAY_KEY: &[CoreTy] = &[CoreTy::Int, CoreTy::Str];

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
        "mwl_core_arr_values" => (mwl_core_arr_values as *const ()).cast(),
        "mwl_core_arr_sort" => (mwl_core_arr_sort as *const ()).cast(),
        "mwl_core_arr_range" => (mwl_core_arr_range as *const ()).cast(),
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
/// Deliberately never dropped: a helper's arguments are *borrowed* (see
/// [`crate`]'s own docs), so the reference this handle wraps belongs to the
/// caller and releasing it here would be a double free. Wrapping in
/// `ManuallyDrop` rather than retaining first keeps the borrow free — there is
/// no refcount traffic at all.
fn borrowed(array: *mut mwl_runtime::ArrayHeader) -> std::mem::ManuallyDrop<MwlArray> {
    #[expect(
        unsafe_code,
        reason = "a Tag::Array argument owns a reference to a live allocation, \
                  so it is live for the length of this call, and the handle is \
                  never dropped"
    )]
    std::mem::ManuallyDrop::new(unsafe { MwlArray::from_raw(array) })
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
        use std::fmt::Write as _;

        let array = args[0].array_ptr().ok_or_else(|| {
            Fault::fatal(format!(
                "Core\\Arr::isList expected {:?}, got tag {}",
                Tag::Array,
                args[0].tag_byte()
            ))
        })?;
        let subject = borrowed(array);

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
                return Ok(Value::bool(false));
            }
            from = slot + 1;
            index += 1;
        }
        Ok(Value::bool(true))
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

/// One entry's `by`-extracted sort key, owned by the frame that extracted it.
///
/// `mwl_runtime::call_closure` hands back one fresh reference per call, so the
/// extracted keys are this frame's to free — unlike the entries themselves,
/// which belong to the subject array. Held in a guard rather than released at
/// the end of [`mwl_core_arr_sort`] because a `by` closure, a comparator or a
/// wrong tag can all leave part-way through, and a `Drop` is the only release
/// every one of those paths runs.
struct SortKeys(Vec<Value>);

impl Drop for SortKeys {
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
        let by = optional_callback(&args[1], "by")?;
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
        let comparator = optional_callback(&args[3], "comparator")?;
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
        let mut sort_keys = SortKeys(Vec::new());
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
                None => compare_values(&compared[left], &compared[right])?,
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
fn optional_callback(value: &Value, option: &str) -> Result<Option<Value>, Fault> {
    match value.tag() {
        Some(Tag::Null) => Ok(None),
        Some(Tag::Object) => Ok(Some(*value)),
        _ => Err(Fault::fatal(format!(
            "Core\\Arr::sort expected a closure or nothing for `{option}`, got tag {}",
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

/// The natural ordering of two values, or a throw for a pair that has none.
///
/// One row per representation, and nothing crosses between rows except the
/// numeric ones:
///
/// * `null` — one value, so always equal.
/// * `bool` — `false` before `true`.
/// * `int`/`uint` — exactly, through `i128`, so no large `uint` is rounded.
/// * `float` against anything numeric — `f64::total_cmp`, which is a real
///   total order (unlike `partial_cmp`, which a `NaN` makes intransitive and
///   therefore unusable by any sort at all). Its two visible consequences are
///   that `-0.0` sorts before `0.0` and that `NaN` sorts at one end rather
///   than throwing.
/// * `string`/`bytes` — **bytewise**, never numerically. See
///   [`mwl_core_arr_sort`], which owns that divergence from PHP.
///
/// Anything else — an object, an array, or two different rows above — is
/// `THROWN`, naming both tags. An object is the one worth calling out: ADR
/// 0013 makes `Comparable` the answer, and reaching an instance method from a
/// helper is the thing that is not built yet.
fn compare_values(left: &Value, right: &Value) -> Result<std::cmp::Ordering, Fault> {
    use std::cmp::Ordering;

    if let (Some(Tag::Null), Some(Tag::Null)) = (left.tag(), right.tag()) {
        return Ok(Ordering::Equal);
    }
    if let (Some(a), Some(b)) = (left.as_bool(), right.as_bool()) {
        return Ok(a.cmp(&b));
    }
    if let (Some(a), Some(b)) = (left.as_str_bytes(), right.as_str_bytes()) {
        return Ok(a.cmp(b));
    }
    if let (Some(a), Some(b)) = (numeric(left), numeric(right)) {
        return Ok(match (a, b) {
            (Numeric::Integer(a), Numeric::Integer(b)) => a.cmp(&b),
            (Numeric::Integer(a), Numeric::Real(b)) => real(a).total_cmp(&b),
            (Numeric::Real(a), Numeric::Integer(b)) => a.total_cmp(&real(b)),
            (Numeric::Real(a), Numeric::Real(b)) => a.total_cmp(&b),
        });
    }
    Err(Fault::thrown(format!(
        "Core\\Arr::sort has no natural order for tag {} against tag {}; pass \
         `{{comparator: ...}}`, or implement `Comparable` and compare by that",
        left.tag_byte(),
        right.tag_byte()
    )))
}

/// One value's numeric content, or `None` for a value that has none.
#[derive(Clone, Copy)]
enum Numeric {
    /// An `int` or a `uint`, widened so the two compare exactly.
    Integer(i128),
    /// A `float`.
    Real(f64),
}

fn numeric(value: &Value) -> Option<Numeric> {
    if let Some(int) = value.as_int() {
        return Some(Numeric::Integer(i128::from(int)));
    }
    if let Some(uint) = value.as_uint() {
        return Some(Numeric::Integer(i128::from(uint)));
    }
    value.as_float().map(Numeric::Real)
}

/// An exact integer as the `f64` it is compared against.
#[expect(
    clippy::cast_precision_loss,
    reason = "an integer past 2^53 loses low bits on the way to `f64`, which \
              is the same rounding ADR 0007 § 4's int-to-float widening \
              already allows; the alternative is a mixed int/float array \
              having no order at all"
)]
fn real(value: i128) -> f64 {
    value as f64
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
}
