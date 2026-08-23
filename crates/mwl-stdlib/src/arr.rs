//! `Core\Arr` — [docs/spec/01-core-library.md](../../../../docs/spec/01-core-library.md)
//! § 2, over `mwl_runtime`'s insertion-ordered, copy-on-write `array<T>`.
//!
//! Every member here is pure (ADR 0063 R3) and borrows its subject rather
//! than consuming it — see [`crate`]'s own docs for why that falls out of
//! being a helper rather than being a rule this module states.

use mwl_runtime::{Fault, MwlArray, MwlStr, Tag, Value};

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
