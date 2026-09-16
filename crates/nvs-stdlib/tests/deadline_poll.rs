//! A member whose runtime scales with its input stops when the deadline has
//! passed, and stops at the batch boundary the combinator owns.
//!
//! `rule:http-server/time-is-bounded-inside-a-helper`
//! 's first constraint puts the poll in `nvs_runtime::bounded_loop` rather
//! than in each helper that remembers to ask, and `Core\Arr::map` is the first
//! member to adopt it: a callback per entry, over an array a request supplied,
//! is the definition of the shape that ADR exists for. What is pinned here is
//! the part the combinator's own unit tests cannot see — that the member
//! actually goes through it, and that a fired poll is `FATAL` rather than
//! something a `catch` could swallow (`rule:concurrency/cancellation-runs-no-user-code`).
//!
//! Two counts, one from each side of the same bound: the entries visited before
//! the poll fires, and the entries visited when nothing set a deadline. A
//! member that grew its own walk again would still pass the second and fail the
//! first.
//!
//! `closure_of` is `tests/allocation_policy.rs`'s, and deliberately a second
//! copy rather than a shared module: the two test binaries are separate crates,
//! and the alternative is a `mod` that neither of them owns.

use std::sync::atomic::{AtomicUsize, Ordering};

use nvs_runtime::{DEADLINE_POLL_BATCH, Value};
use nvs_stdlib::arr::nvs_core_arr_map;

/// Releases a value this test frame owns.
#[expect(
    unsafe_code,
    reason = "the value is live and this frame holds the reference it is \
              handing back"
)]
fn release(value: Value) {
    unsafe {
        nvs_runtime::nvs_value_release(u64::from(value.tag_byte()), value.bits());
    }
}

/// A packed list of `count` integers.
fn list_of(count: usize) -> Value {
    let mut list = nvs_runtime::NvsArray::new();
    for index in 0..count {
        list.append(Value::int(
            i64::try_from(index).expect("a test-sized index"),
        ));
    }
    Value::array(list)
}

/// A closure value whose `invoke` is a plain Rust function.
///
/// `nvs_runtime::call_closure` reads exactly four things off a closure — its
/// class's `ClassTable::set_closure` bit, slot `CLOSURE_ARITY_SLOT`, slot
/// `CLOSURE_PARAM_TAGS_SLOT`, and the `CLOSURE_INVOKE` method's address in its
/// class — so a test in this crate can hand a `Core` member a `callable`
/// without a compiler in front of it. The
/// table is leaked because a descriptor's *address* is its identity and it must
/// outlive every instance made from it.
fn closure_of(arity: usize, invoke: nvs_runtime::NvsFn) -> Value {
    let mut table = nvs_runtime::ClassTable::new();
    let id = table.define("{closure}", &["arity", "params"], &[]);
    table.set_methods(
        id,
        vec![nvs_runtime::MethodRow {
            name: nvs_runtime::CLOSURE_INVOKE.to_owned(),
            code: invoke as *const u8,
            arity: 0,
            param_tags: 0,
            param_names: Vec::new(),
            public: true,
            native: false,
        }],
    );
    table.set_closure(id);
    let table: &'static nvs_runtime::ClassTable = Box::leak(Box::new(table));
    #[expect(
        unsafe_code,
        reason = "the table above is leaked, so the descriptor outlives every \
                  instance made from it — `NvsObj::new`'s whole obligation"
    )]
    let object = unsafe { nvs_runtime::NvsObj::new(table.desc(id)) };
    object.set_field(
        nvs_runtime::CLOSURE_ARITY_SLOT,
        Value::int(i64::try_from(arity).expect("a small arity")),
    );
    object.set_field(
        nvs_runtime::CLOSURE_PARAM_TAGS_SLOT,
        Value::int(i64::from(nvs_runtime::CLOSURE_PARAM_TAG_ANY)),
    );
    Value::object(object)
}

/// Counts its calls and sweeps the references `call_closure` retained for it —
/// the receiver and the one parameter — so an abandoned walk leaks nothing the
/// WSL valgrind leg would find later.
#[expect(
    unsafe_code,
    reason = "`call_closure` passes exactly the receiver and one parameter, \
              each retained for this callee to release, and the result pointer \
              is the address of a live `Value` — neither is expressible in the \
              signature compiled code calls through"
)]
unsafe fn counted(counter: &AtomicUsize, args: *const Value, out: *mut Value) -> i32 {
    counter.fetch_add(1, Ordering::Relaxed);
    for index in 0..2 {
        release(unsafe { *args.add(index) });
    }
    unsafe {
        *out = Value::bool(true);
    }
    nvs_runtime::OK
}

static UNDER_A_DEADLINE: AtomicUsize = AtomicUsize::new(0);

#[expect(unsafe_code, reason = "forwarding this callee's own contract")]
unsafe extern "C" fn counts_under_a_deadline(
    _ctx: *mut nvs_runtime::Ctx,
    args: *const Value,
    out: *mut Value,
) -> i32 {
    unsafe { counted(&UNDER_A_DEADLINE, args, out) }
}

static WITH_NO_DEADLINE: AtomicUsize = AtomicUsize::new(0);

#[expect(unsafe_code, reason = "forwarding this callee's own contract")]
unsafe extern "C" fn counts_with_no_deadline(
    _ctx: *mut nvs_runtime::Ctx,
    args: *const Value,
    out: *mut Value,
) -> i32 {
    unsafe { counted(&WITH_NO_DEADLINE, args, out) }
}

/// Four batches of entries, so the first boundary is nowhere near the end.
const ENTRIES: usize = DEADLINE_POLL_BATCH * 4;

#[test]
fn an_expired_deadline_stops_arr_map_at_the_first_batch_boundary() {
    let mut ctx = nvs_runtime::Ctx::buffered();
    ctx.expire_deadline();

    let subject = list_of(ENTRIES);
    let callback = closure_of(1, counts_under_a_deadline);
    let status = nvs_runtime::call(nvs_core_arr_map, &mut ctx, &[subject, callback])
        .expect_err("the deadline had already passed when the walk began");

    assert_eq!(
        status,
        nvs_runtime::FATAL,
        "a deadline is a cancellation, so `rule:concurrency/cancellation-runs-no-user-code` makes it uncatchable"
    );
    assert_eq!(
        UNDER_A_DEADLINE.load(Ordering::Relaxed),
        DEADLINE_POLL_BATCH - 1,
        "`Core\\Arr::map` must stop at the combinator's first batch boundary, \
         not per entry and not at the end of {ENTRIES}"
    );
    assert!(
        ctx.take_pending().is_some_and(
            |message| message.contains("Core\\Arr::map") && message.contains("deadline")
        ),
        "a fired poll names the member it interrupted"
    );

    release(subject);
    release(callback);
}

#[test]
fn a_walk_with_no_deadline_still_visits_every_entry() {
    let mut ctx = nvs_runtime::Ctx::buffered();

    let subject = list_of(ENTRIES);
    let callback = closure_of(1, counts_with_no_deadline);
    let mapped = nvs_runtime::call(nvs_core_arr_map, &mut ctx, &[subject, callback])
        .expect("nothing set a deadline on this context");

    assert_eq!(
        WITH_NO_DEADLINE.load(Ordering::Relaxed),
        ENTRIES,
        "a poll site must not swallow the entry it guards"
    );
    let array = mapped.array_ptr().expect("`map` answers an array");
    #[expect(
        unsafe_code,
        reason = "the allocation is live: this frame holds the only reference \
                  the member just answered with"
    )]
    let count = unsafe { nvs_runtime::nvs_array_count(array) };
    assert_eq!(
        usize::try_from(count).expect("a test-sized count"),
        ENTRIES,
        "every entry the callback answered is in the result"
    );

    release(mapped);
    release(subject);
    release(callback);
}
