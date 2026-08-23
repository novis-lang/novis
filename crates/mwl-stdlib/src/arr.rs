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
    }
}
