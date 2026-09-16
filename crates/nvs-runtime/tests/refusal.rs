//! What a refused allocation leaves behind — `rule:errors/on-limit`'s memory
//! ceiling asked *in front of* an allocation rather than after it, so that one
//! operation is bounded and not merely a loop of them.
//!
//! The cases here ask the **verdict**, not the arithmetic. A request that was
//! refused holds fewer bytes than its ceiling, because the bytes it asked for
//! were never handed over, so every reader that goes on counting calls it
//! comfortably inside one. What has to be true instead is that the poll
//! following a refusal reports the breach, and that the refusal belongs to the
//! request it stopped and to no other.
//!
//! The tier-1 handler's side of it — that a refused request can still allocate
//! the report `rule:errors/on-limit` promises, inside the slice reserved for it
//! — is asserted end to end by
//! `tests/conformance/error/one-operation-past-the-ceiling-is-refused-before-it-allocates.nvst`,
//! where there is a registered closure to run. `Ctx::run_limit_handler` runs
//! nothing without one, so a case here could only assert the accessors it is
//! built from.

use nvs_runtime::{
    ArrayHeader, Ctx, FATAL, IMMORTAL_REFCOUNT, NvsArray, NvsStr, OutputSink, SafepointFlags,
    Value, affordable, budget, nvs_array_append, nvs_array_next_slot, nvs_array_release,
    nvs_array_set, nvs_array_set_index, nvs_array_value_at, nvs_str_append, nvs_str_concat,
    nvs_str_concat_n, nvs_str_release, prime_empty_array,
};

/// A request under `bytes` of ceiling, holding nothing of its own yet.
///
/// `Ctx::set_memory_limit` rather than a configuration, because nothing here
/// asks which directive the number came from — `tests/allocator_ceiling.rs`
/// holds the cases that do, and both arrive at the same armed threshold.
fn ctx_under(bytes: usize) -> Ctx {
    let mut ctx = Ctx::new(OutputSink::Buffer(Vec::new()));
    ctx.set_memory_limit(bytes);
    ctx
}

/// An ask larger than the whole remaining budget is refused in front of the
/// allocation, and the poll that follows reports it.
///
/// Both sides of the bound, in that order: an ask that fits is answered without
/// recording anything, which is what says the refusal below is the ask's and
/// not the ceiling's mere presence. The reading afterwards is the whole point
/// of the case — the request is *under* its ceiling and over its limit at the
/// same time, which is a breach only the verdict can carry.
#[test]
fn a_single_operation_past_the_ceiling_never_allocates_what_it_asked_for() {
    let ctx = ctx_under(16 << 20);
    assert!(
        budget::affords(1 << 10),
        "an ask a request has room for twenty times over was refused",
    );
    assert!(
        !ctx.over_memory_limit(),
        "a request that has been refused nothing is already over its ceiling, so nothing below is a refusal",
    );
    assert!(
        !ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT),
        "an ask that fits raised the memory flag",
    );

    let held = budget::live_bytes();
    let ask = 256 << 20;
    assert!(
        !budget::affords(ask),
        "an ask of sixteen times the whole ceiling was afforded, which is the single operation this stage exists to bound",
    );
    assert_eq!(
        budget::live_bytes(),
        held,
        "a refused ask moved the balance, so something was allocated on the way to refusing it",
    );
    assert!(
        ctx.memory_peak() < ask,
        "the mark holds the ask, which is what a request that allocated first and noticed afterwards leaves behind",
    );

    // The request is inside its ceiling by every measure that counts bytes, and
    // over it all the same. A reader that asked the counter alone would let the
    // program carry on with whatever degenerate value the refusal handed back.
    assert!(
        ctx.memory_used() < ctx.memory_limit(),
        "the refused request holds more than its ceiling, so the assertions below would hold for a runtime that allocated the ask and counted it",
    );
    assert!(
        ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT),
        "a refusal asked for no poll, so compiled code that allocates nothing else never reaches the reader below",
    );
    assert!(
        ctx.over_memory_limit(),
        "a refused request reads as inside its ceiling",
    );
    assert!(
        matches!(ctx.memory_breach(), Some(nvs_runtime::Fault::Fatal(_))),
        "the poll a refusal wakes does not report it, so `rule:errors/on-limit`'s `FATAL` never arrives",
    );
}

/// The seam every count-shaped `Core` argument passes through refuses an ask
/// the running request cannot afford — and refuses it at the tier a resource
/// limit gets, not the one an unallocatable size gets.
///
/// Both tiers, because the distinction is the whole of what `affordable`
/// carries. A size no process could hold is a `Thrown` a program may catch:
/// nothing about the request is wrong, and the caller may well have a fallback.
/// A size *this* request cannot hold is `rule:errors/on-limit`'s `Fatal`,
/// because the refusal is recorded by the time it is worded — a `catch` would
/// hand the program a way to carry on inside a request that is already over.
///
/// The ask that fits comes first, and records nothing, which is what says the
/// refusal below belongs to the size rather than to the ceiling's presence.
#[test]
fn an_ask_past_the_ceiling_is_refused_by_the_shared_size_check() {
    let ctx = ctx_under(16 << 20);
    assert_eq!(
        affordable(Some(1 << 10), "Core\\Arr::fill").ok(),
        Some(1 << 10),
        "an ask the request has room for thousands of times over was refused",
    );

    assert!(
        matches!(
            affordable(Some(usize::MAX), "Core\\Arr::fill"),
            Err(nvs_runtime::Fault::Thrown(..))
        ),
        "a size no process could allocate was reported as the request's own limit",
    );
    assert!(
        !ctx.over_memory_limit(),
        "one of the two asks above recorded a refusal, so nothing below is this seam's doing",
    );

    assert!(
        matches!(
            affordable(Some(256 << 20), "Core\\Str::repeat"),
            Err(nvs_runtime::Fault::Fatal(_))
        ),
        "an ask of sixteen times the whole ceiling was served, or was worded as a throw the program carries on from",
    );
    assert!(
        ctx.over_memory_limit(),
        "the refusal was not recorded against the request that made it",
    );
    assert!(
        matches!(ctx.memory_breach(), Some(nvs_runtime::Fault::Fatal(_))),
        "the poll this refusal wakes does not report it, so a member's caller never sees the ceiling",
    );
}

/// A refusal dies with the request it stopped.
///
/// The verdict is a thread-local, because the allocators that hit one hold no
/// `Ctx` — so without the displacement `Ctx::memory_refused_saved` names, the
/// next request a worker picked up would be stopped by a refusal it never
/// asked for, and a served core would answer one `FATAL` per request for ever.
#[test]
fn a_refusal_dies_with_the_request_it_stopped() {
    {
        let stopped = ctx_under(16 << 20);
        assert!(
            !budget::affords(256 << 20),
            "the ask was afforded, so nothing below is a refusal being confined",
        );
        assert!(
            stopped.over_memory_limit(),
            "the refusal was not recorded against the request that made it",
        );
    }
    let next = ctx_under(16 << 20);
    assert!(
        !next.over_memory_limit(),
        "a request inherited the refusal that stopped the one before it on this thread",
    );
    assert!(
        next.memory_breach().is_none(),
        "a request born clear was reported over a ceiling it has not been refused against",
    );
    assert!(
        budget::affords(1 << 10),
        "an inherited refusal is answering for asks this request has room for",
    );
}

/// Every way a string is allocated answers a value where the request is refused
/// one, instead of reaching `handle_alloc_error`.
///
/// The assertion is partly that the case **finishes**: an abort takes the whole
/// test binary with it, so a path that still aborted would fail this as a crash
/// rather than as a comparison. The three paths are the constructor every
/// `Core` member reaches, the fallible one a member that words its own refusal
/// reaches, and the writer's growth — the only one of the three that arrives
/// with a buffer already in hand, and so the only one whose degenerate answer
/// is a prefix rather than nothing.
///
/// What is left aborting in `string.rs` is the allocator's own refusal, a null
/// for a block no ceiling explains. That is `NvsStr::new`'s known gap, waiting
/// on the per-request arena, and no program can drive it.
#[test]
fn no_string_allocation_path_can_abort() {
    // Allocated before the ceiling is armed, so the balance it moves is part of
    // what the threshold is measured from rather than an ask against it.
    let bulk = vec![b'x'; 8 << 20];
    let ctx = ctx_under(1 << 20);

    let mut ran = false;
    let refused = NvsStr::build(256 << 20, |_| ran = true);
    assert!(
        !ran,
        "a refused `build` ran its writer, which has no buffer to write into",
    );
    assert!(
        refused.as_bytes().is_empty(),
        "a refused `build` answered bytes it was never given the room to hold",
    );
    assert_eq!(
        refused.refcount(),
        IMMORTAL_REFCOUNT,
        "the degenerate return is an allocation, so whoever the refusal handed it to will free it",
    );

    assert!(
        NvsStr::try_build(256 << 20, |out| out.push(b"")).is_none(),
        "the fallible constructor served an ask past the whole ceiling",
    );

    let truncated = NvsStr::build(5, |out| {
        out.push(b"novis");
        out.push(&bulk);
    });
    assert_eq!(
        truncated.as_bytes(),
        b"novis",
        "a refused growth wrote into room the request had just been refused",
    );

    assert!(
        ctx.over_memory_limit(),
        "none of the three refusals above was recorded against the request that made them",
    );
}

/// Both concatenation entry points answer one static empty string, and it
/// survives being released.
///
/// `nvs_ir::InstKind::Concat` lowers to an `extern "C"` call with nowhere to put
/// a status, so the refusal has to be a *value* — and compiled code owns that
/// value exactly as it owns an allocated one, releasing it when the temporary
/// dies. A degenerate return that could be freed would therefore be a
/// double-free on the first program that concatenated twice.
#[test]
fn a_concat_past_the_ceiling_returns_the_immortal_empty_string() {
    let payload = vec![b'x'; 8 << 20];
    let bulk = NvsStr::new(&payload).into_raw();
    let tail = NvsStr::new(b"!").into_raw();
    let ctx = ctx_under(1 << 20);

    #[expect(
        unsafe_code,
        reason = "the two operands are live for this whole case, which is what \
                  both primitives' safety contracts ask for; the results are \
                  read through the same module's own accessor"
    )]
    unsafe {
        // Two operands, which reach the allocation through `NvsStr::build`, and
        // three, which reach it through an allocation of their own.
        let joined = nvs_str_concat(bulk, tail);
        let pieces = [bulk.cast_const(), tail.cast_const(), bulk.cast_const()];
        let joined_n = nvs_str_concat_n(pieces.as_ptr(), pieces.len());

        assert!(
            NvsStr::bytes_of(joined).is_empty(),
            "a refused two-operand concatenation answered bytes it never allocated",
        );
        assert!(
            NvsStr::bytes_of(joined_n).is_empty(),
            "a refused n-operand concatenation answered bytes it never allocated",
        );
        assert_eq!(
            joined, joined_n,
            "the two entry points answered two different degenerate values, so one of them allocated",
        );

        // More releases than there were references, which is what a program
        // concatenating inside a loop hands a value it was never told is static.
        nvs_str_release(joined);
        nvs_str_release(joined_n);
        assert!(
            NvsStr::bytes_of(joined).is_empty(),
            "releasing the degenerate return freed a static, so the next reader is reading freed memory",
        );

        nvs_str_release(bulk);
        nvs_str_release(tail);
    }

    assert!(
        ctx.over_memory_limit(),
        "the concatenations were served rather than refused, so nothing above is about a refusal",
    );
}

/// A refused append answers its target, unmoved and unwritten.
///
/// `nvs_ir::ir::InstKind::StrAppend` consumes one reference to the target and
/// yields one to the result, so the target *is* the only degenerate return that
/// balances — anything else either leaks the accumulation or hands the caller a
/// reference it must not release. The bytes matter as much as the pointer: a
/// refusal that grew nothing but wrote into the target it answered would pass a
/// pointer comparison and corrupt a string two owners can see.
#[test]
fn an_append_past_the_ceiling_returns_its_target_unchanged() {
    let payload = vec![b'x'; 4 << 20];
    let added = vec![b'y'; 4 << 20];
    let target = NvsStr::new(&payload).into_raw();
    let suffix = NvsStr::new(&added).into_raw();
    let ctx = ctx_under(1 << 20);

    #[expect(
        unsafe_code,
        reason = "both pointees are live for this whole case and the reference \
                  the primitive consumes is the one released below, which is \
                  its safety contract exactly"
    )]
    unsafe {
        let answered = nvs_str_append(target, suffix);
        assert_eq!(
            answered, target,
            "a refused append moved the accumulation into the copy it had just been refused the room for",
        );
        assert_eq!(
            NvsStr::bytes_of(answered).len(),
            4 << 20,
            "a refused append wrote into the target it answered unchanged",
        );
        assert!(
            NvsStr::bytes_of(answered).iter().all(|byte| *byte == b'x'),
            "a refused append wrote the suffix into room the target already had",
        );

        nvs_str_release(answered);
        nvs_str_release(suffix);
    }

    assert!(
        ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT),
        "the append was served rather than refused, so nothing above is about a refusal",
    );
}

/// A refusal is a complete no-op, so every reference an operation was handed
/// balances exactly as it would have if the operation had been served.
///
/// Asserted as a **balance** rather than as a reference count, because the two
/// failures this has to catch are invisible to a count on the operand: an
/// operation that allocated before refusing leaks what it allocated, and one
/// that separated its target and then threw the copy away leaks the copy while
/// the original's count still reads right.
#[test]
fn a_refused_operation_balances_every_reference_it_was_handed() {
    // A request made and ended before the reading is taken: the first `Ctx` on
    // a thread fills in state that outlives it, and a one-time cost inside the
    // window would read here as the leak this case exists to catch.
    drop(ctx_under(1 << 20));
    let before = budget::live_bytes();
    {
        let payload = vec![b'x'; 4 << 20];
        let target = NvsStr::new(&payload).into_raw();
        let suffix = NvsStr::new(b"!").into_raw();
        let ctx = ctx_under(1 << 20);

        #[expect(
            unsafe_code,
            reason = "each pointee is live until the release that ends its one \
                      reference, and every primitive here is handed exactly the \
                      ownership its contract names"
        )]
        unsafe {
            let appended = nvs_str_append(target, suffix);
            let joined = nvs_str_concat(target, suffix);
            assert!(
                ctx.over_memory_limit(),
                "both operations were served, so the balance below says nothing about a refusal",
            );

            // What a caller holds after each: one reference to the append's
            // result, none to a concatenation's operands, and one to the value
            // it answered — including when that value is the static.
            nvs_str_release(joined);
            nvs_str_release(appended);
            nvs_str_release(suffix);
        }
    }
    assert_eq!(
        budget::live_bytes(),
        before,
        "a refused operation left bytes behind, which is a leak charged to whichever request runs next",
    );
}

/// A refused write hands back the array it was given, with nothing written into
/// it and — the half only a pointer comparison catches — nothing separated
/// either.
///
/// The array here is shared, which is the state a write is expensive in and the
/// one the no-op has to hold in. A copy made and then not written into is the
/// failure this is written around: the pointer the caller is handed looks
/// served, the counts on both handles still read right, and every other owner
/// goes on reading an original the program believes it has just written to.
///
/// Both primitives, because the string-keyed one owes one thing more: the key's
/// reference, which a served write takes over and a refused one has to release
/// rather than keep.
#[test]
fn an_array_set_past_the_ceiling_neither_writes_nor_separates() {
    // The two per-thread costs the playbook names, paid before any balance
    // below is read: the empty array's singleton, and a thread's first `Ctx`.
    prime_empty_array();
    drop(ctx_under(1 << 20));

    let mut array = NvsArray::new();
    for entry in 0..16 {
        array.append(Value::int(entry));
    }
    let shared = array.clone();
    let raw = array.into_raw();
    // Allocated while there is still a budget to allocate it out of: the case
    // is about the write's refusal, not the key's.
    let key = NvsStr::new(b"k").into_raw();

    let ctx = ctx_under(64);
    let held = budget::live_bytes();

    #[expect(
        unsafe_code,
        reason = "`raw` is live for the whole case, holding the one reference \
                  each primitive consumes and hands back, `key` owns the one \
                  reference it transfers, and each `Value` is an `int` owning \
                  nothing"
    )]
    unsafe {
        let answered = nvs_array_set_index(raw, 3, &Value::int(99));
        assert_eq!(
            answered, raw,
            "a refused write separated, so it allocated the copy it had just been refused the room for",
        );
        assert_eq!(
            budget::live_bytes(),
            held,
            "a refused write moved the balance, so something was allocated on the way to refusing it",
        );

        let answered = nvs_array_set(answered, key, &Value::int(99));
        assert_eq!(
            answered, raw,
            "a refused string-keyed write separated, the half of the no-op only the pointer says",
        );
        assert!(
            budget::live_bytes() < held,
            "a refused write kept the key it was handed, which is a leak charged to whichever request runs next",
        );
        drop(NvsArray::from_raw(answered));
    }

    assert_eq!(
        shared.get_index(3).and_then(Value::as_int),
        Some(3),
        "a refused write landed in the array it answered unchanged",
    );
    assert_eq!(
        shared.count(),
        16,
        "a refused write changed what the array holds",
    );
    assert!(
        ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT),
        "the write was served rather than refused, so nothing above is about a refusal",
    );
}

/// A primitive that carries a status says so instead of answering a degenerate
/// value: the append hands back its array unchanged, releases the reference it
/// was given, and reports the ceiling as the `FATAL` no `catch` can see.
///
/// `rule:errors/on-limit`'s tier, in other words, rather than the "no room" a
/// fallible producer would word as a throw. The array coming back untouched is
/// the other half: `$a[] = …`'s error path releases whatever `out` names, so an
/// append that separated and then refused would leave the caller's own pointer
/// dangling.
///
/// The balance is the one thing not asserted here. Reporting the breach builds
/// the message the status carries, which is an allocation the request makes
/// *because* it was refused; that the write itself takes none is what the cases
/// above pin, on the primitives that answer no status at all.
#[test]
fn an_array_append_past_the_ceiling_refuses_through_the_status_it_already_has() {
    prime_empty_array();
    drop(ctx_under(1 << 20));

    let mut array = NvsArray::new();
    for entry in 0..16 {
        array.append(Value::int(entry));
    }
    let shared = array.clone();
    let raw = array.into_raw();

    let mut ctx = ctx_under(64);
    let mut answered: *mut ArrayHeader = std::ptr::null_mut();

    #[expect(
        unsafe_code,
        reason = "`raw` holds the one reference the append consumes and writes \
                  back, `ctx` is the live context of the request making the \
                  call, and the `Value` is an `int` owning nothing"
    )]
    unsafe {
        let status = nvs_array_append(&raw mut ctx, raw, &Value::int(99), &mut answered);
        assert_eq!(
            status, FATAL,
            "a refused append answered something a program can carry on from, which is the tier `rule:errors/on-limit` puts a limit above",
        );
        assert_eq!(
            answered, raw,
            "a refused append separated, so the reference its caller holds is not the one it was handed back",
        );
        drop(NvsArray::from_raw(answered));
    }

    assert_eq!(
        shared.count(),
        16,
        "a refused append landed in the array it answered unchanged",
    );
    assert!(
        ctx.pending()
            .is_some_and(|message| message.contains("memory limit")),
        "a refused append recorded a member's own error in place of the ceiling that stopped it",
    );
}

/// A `foreach` whose body writes walks the snapshot it started on: the loop
/// holds its own reference, so the write separates and leaves the cursor on the
/// original. A refused write has to leave that true from the other side — it
/// separates nothing, so the cursor's snapshot and the program's array stay one
/// allocation, and it writes nothing into that allocation, so every slot the
/// cursor has still to reach names a live entry.
///
/// The failure this is written around is not a wrong answer but a panic:
/// `nvs_array_value_at` expects the slot it is handed to be live, and a refusal
/// that wrote into the shared original would be a contained `FATAL` where
/// `rule:errors/on-limit` says the request is stopped by its ceiling.
#[test]
fn a_refused_write_leaves_a_live_foreach_cursor_on_its_own_snapshot() {
    prime_empty_array();
    drop(ctx_under(1 << 20));

    let mut array = NvsArray::new();
    for entry in 0..8 {
        array.append(Value::int(entry));
    }
    // What `foreach` retains for the walk, and what the program's own variable
    // holds — the same allocation until a served write separates them.
    let cursor = array.clone().into_raw();
    let raw = array.into_raw();
    let ctx = ctx_under(64);
    let mut walked = Vec::new();

    #[expect(
        unsafe_code,
        reason = "both handles are live until the releases that end them, the \
                  slot passed to `nvs_array_value_at` is the one \
                  `nvs_array_next_slot` just named, and the `Value` is an \
                  `int` owning nothing"
    )]
    unsafe {
        let mut from = 0;
        loop {
            let slot = nvs_array_next_slot(cursor, from);
            let Ok(slot) = usize::try_from(slot) else {
                break;
            };
            let mut seen = Value::null();
            nvs_array_value_at(cursor, slot, &mut seen);
            walked.push(seen.as_int().expect("the case stored ints"));

            // The body's own `$a[0] = 99`, refused.
            let answered = nvs_array_set_index(raw, 0, &Value::int(99));
            assert_eq!(
                answered, cursor,
                "a refused write separated under a live cursor, which leaves the walk on an allocation nothing else holds",
            );
            from = slot + 1;
        }
        nvs_array_release(cursor);
        nvs_array_release(raw);
    }

    assert_eq!(
        walked,
        (0..8).collect::<Vec<i64>>(),
        "the walk over a refused write's array missed an entry or answered a written one",
    );
    assert!(
        ctx.safepoint_flags().contains(SafepointFlags::MEMORY_LIMIT),
        "the writes were served rather than refused, so nothing above is about a refusal",
    );
}
