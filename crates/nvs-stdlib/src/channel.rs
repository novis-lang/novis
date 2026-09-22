//! `Core\Task\Channel<T>` — the language surface over goal `concurrency`'s item 11: a
//! bounded queue between two tasks whose `send` **suspends** at the bound
//! instead of growing.
//!
//! `rule:concurrency/one-scheduler`
//! scopes the roster and explicitly not this type's spelling, so what is
//! decided here is decided here. `crates/nvs-host/src/channel.rs`'s module doc
//! is the one home for *why* a bound rather than growth — an unbounded queue
//! between a fast producer and a slow consumer is O(messages produced), which
//! is `rule:programs/memory-priority`'s growth
//! with traffic rather than with concurrency — and this module does not restate
//! it.
//!
//! # Decision: the queue is Novis values in the instance's own slots
//!
//! Not a handle into a table the host keeps. `nvs-host` already has a channel,
//! and wrapping *that* would mean an integer in a slot naming a `Sender`/
//! `Receiver` pair the host owns — and a `Core` instance has no native drop
//! ([`crate::instance`]), so nothing would ever tell the host that the last
//! reference to a channel had gone. The table's footprint would then be
//! O(channels created) for the life of the worker, which
//! [AGENTS.md](/AGENTS.md)'s memory rule calls a leak rather than a
//! trade-off.
//!
//! Keeping the queue in ordinary slots costs the duplication of about thirty
//! lines of "wait when full, wait when empty" and buys the whole lifetime
//! question: releasing the object releases the array, which releases every
//! value still queued, charged to the request that made it and bounded by the
//! `capacity` written at construction. **What it spends:** one instance, one
//! array, and `capacity` values at most.
//!
//! The two channels are not one implementation for a second reason: they are
//! not the same shape. `nvs_host::channel` is a generic Rust queue with two
//! handles and a disconnection derived from dropping them; this is one object,
//! held by both ends, whose end-of-stream is the explicit `close()` a program
//! writes.
//!
//! # Decision: the object is its own iterator, and the wait is in `advance`
//!
//! `foreach ($chan as int $v)` may not take a snapshot — the whole point is
//! that the values are not there yet. So `iterate()` answers with the receiver
//! itself rather than with a [`crate::cursor`], and the class carries
//! [`nvs_runtime::sequence`]'s `advance`/`current` rows of its own: `advance`
//! takes the next value out, **suspending while the queue is empty and the
//! channel is open**, and answers `false` the moment it is empty and closed.
//! That is what ends a consumer's loop, and a consumer waiting on a channel
//! nobody will send to again is the deadlock the `close()` exists to prevent.
//!
//! # Decision: a wake goes to every waiter on the channel, not to one
//!
//! A parked task hands out a [`Waker`] before it suspends, and the registry
//! below holds those handles — they are Rust closures, so they cannot live in
//! a slot the way the queue does. Every state change drains **all** of a
//! channel's handles rather than picking the one that can now make progress.
//!
//! `nvs_host::channel` wakes exactly one and says why: one `send` frees one
//! slot, so waking the rest spends a resume to be told to park again. That
//! argument is about a queue with many producers; this type's surface is one
//! object shared by a producer and a consumer, where the waiter list is
//! almost always length one and a wrong pick is a hang rather than a slow path.
//! Waking all of them is correct under both, because every waiter re-checks the
//! state it woke for — [`nvs_runtime::host::Waker`] is a hint by contract.
//!
//! **What it spends:** one entry per *parked* task, removed by the task itself
//! when its park ends however it ends, cancellation included. O(in-flight),
//! never O(channels) and never O(values sent).

use std::cell::{Cell, RefCell};

use nvs_runtime::host::{Waker, Woken};
use nvs_runtime::{Ctx, Fault, NvsArray, NvsStr, ObjHeader, Value};

use crate::identity_store as store;
use crate::registry::{CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc};

/// The class's fully-qualified name, as [`CoreTy::Instance`] spells it.
pub(crate) const NAME: &str = r"Core\Task\Channel";

/// The linker symbol `new Core\Task\Channel<T>(…)` lowers to — see
/// [`crate::registry::CONSTRUCTORS`], which is the roster `nvs-ir` reads.
pub(crate) const NEW_SYMBOL: &str = "nvs_core_channel_new";

/// The symbol behind `Iterable<T>::iterate()`, reached by name through this
/// class's method table rather than as a registered member — see
/// [`crate::instance`]'s dispatch roster.
pub(crate) const ITERATE_SYMBOL: &str = "nvs_core_channel_iterate";
/// The symbol behind `Iterator<T>::advance()`, which is where a consumer waits.
pub(crate) const ADVANCE_SYMBOL: &str = "nvs_core_channel_advance";
/// The symbol behind `Iterator<T>::current()`.
pub(crate) const CURRENT_SYMBOL: &str = "nvs_core_channel_current";

/// `new Core\Task\Channel<T>(uint $capacity)` — the constructor
/// [`crate::registry::CONSTRUCTORS`] registers.
///
/// A positional required parameter rather than an options bag: `rule:core-api/shape-rules` R2
/// puts a bag last for a member with several settings, and a channel has
/// exactly one thing to say about itself. It is required rather than defaulted
/// because there is no capacity a program should acquire by not thinking about
/// one — the bound *is* the backpressure.
pub(crate) const NEW: CoreMethod = CoreMethod {
    name: "constructor",
    names: &["capacity"],
    params: &[CoreTy::Uint],
    defaults: &[],
    return_ty: CoreTy::Instance(NAME),
    symbol: NEW_SYMBOL,
    doc: Some(&CONSTRUCTOR_DOC),
};

/// `Core\Task\Channel<T>` — two members, because everything else a program
/// wants of one is spelled `foreach`.
///
/// No `count`, no `isFull`, no `isEmpty`: each of those is a question whose
/// answer is stale before the caller reads it, since the only way to observe it
/// is to have suspended in between. `examples/channel.nvs` is written around
/// that deliberately.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "send",
            names: &["value"],
            params: &[CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_channel_send",
            doc: Some(&SEND_DOC),
        },
        CoreMethod {
            name: "close",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_channel_close",
            doc: Some(&CLOSE_DOC),
        },
    ],
    slots: &["items", "capacity", "closed", "next", "current"],
    constants: &[],
};

/// `new Core\Task\Channel`'s reference card — `rule:core-api/reference-card`.
const CONSTRUCTOR_DOC: MethodDoc = MethodDoc {
    short: "Builds a `Core\\Task\\Channel<T>` — a bounded queue between two tasks, whose `send` \
            suspends at the bound instead of growing.",
    params: &[ParamDoc {
        name: "capacity",
        desc: "How many values the channel holds before a `send` suspends; at least `1`, since \
               a `0` is refused as a fatal error rather than raised to one.",
        shape: &[],
    }],
    ret: "A fresh open, empty channel, which is its own iterator: a `foreach` over it takes each \
          value out, suspending while the channel is empty and open, and ends once it is empty \
          and closed.",
    errors: &[],
};

/// `Core\Task\Channel::send`'s reference card — `rule:core-api/reference-card`.
const SEND_DOC: MethodDoc = MethodDoc {
    short: "Queues `$value` for the consuming task, suspending the calling task for as long as \
            the channel is full.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to hand across.",
        shape: &[],
    }],
    ret: "Nothing, once the value is queued; a `send` on a closed channel is a fatal error, \
          which no `catch` sees.",
    errors: &[],
};

/// `Core\Task\Channel::close`'s reference card — `rule:core-api/reference-card`.
const CLOSE_DOC: MethodDoc = MethodDoc {
    short: "Ends the stream, so that a consumer's `foreach` finishes once the queued values are \
            drained.",
    params: &[],
    ret: "Nothing; values sent before the close are still delivered, and closing twice changes \
          nothing.",
    errors: &[],
};

/// [`CLASS`]'s slots, by index: the queue itself, oldest entry first …
const ITEMS: usize = 0;
/// … the bound written at construction …
const CAPACITY: usize = 1;
/// … whether `close()` has run …
const CLOSED: usize = 2;
/// … the integer key the next `send` writes under, monotonic so that a key
/// freed at the front is never handed out again while the entry behind it is
/// still live …
const NEXT: usize = 3;
/// … and the value the last `advance()` took out, which `current()` answers.
const CURRENT: usize = 4;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::address_of`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        NEW_SYMBOL => (nvs_core_channel_new as *const ()).cast(),
        "nvs_core_channel_send" => (nvs_core_channel_send as *const ()).cast(),
        "nvs_core_channel_close" => (nvs_core_channel_close as *const ()).cast(),
        ITERATE_SYMBOL => (nvs_core_channel_iterate as *const ()).cast(),
        ADVANCE_SYMBOL => (nvs_core_channel_advance as *const ()).cast(),
        CURRENT_SYMBOL => (nvs_core_channel_current as *const ()).cast(),
        _ => return None,
    })
}

// ============================================================================
// The waiter registry
// ============================================================================

/// One parked task, waiting on the channel at `channel`.
struct Parked {
    /// The receiver's address, which is a channel's identity for as long as a
    /// task is parked on it — the object cannot be freed while a waiter holds
    /// a reference to it through its own frame.
    channel: usize,
    /// This registration's own number, so a task removes *its* entry and not
    /// whichever one happens to be at the same index.
    ticket: u64,
    /// The handle that makes the parked task runnable again.
    waker: Waker,
}

thread_local! {
    /// Every task parked on a channel on this core. A `Vec` rather than a map
    /// keyed by channel: the list is the length of the *parked* set, which is
    /// one or two in every shape this type is written for, and a linear scan
    /// over that beats a hash of a pointer.
    static WAITERS: RefCell<Vec<Parked>> = const { RefCell::new(Vec::new()) };
    /// The next ticket, never reused within a core's life.
    static TICKETS: Cell<u64> = const { Cell::new(0) };
}

/// Registers a wake for the calling task against `channel`, answering the
/// ticket that takes it back off again — or `None` when there is no task
/// beneath the call and so nothing that could ever be woken.
fn register(channel: *mut ObjHeader) -> Option<u64> {
    let waker = nvs_runtime::host::with_current(|host| host.waker())??;
    let ticket = TICKETS.with(|next| {
        let ticket = next.get();
        next.set(ticket + 1);
        ticket
    });
    WAITERS.with_borrow_mut(|waiters| {
        waiters.push(Parked {
            channel: channel as usize,
            ticket,
            waker,
        });
    });
    Some(ticket)
}

/// Takes `ticket`'s registration off the list if it is still there.
///
/// Already absent is the ordinary case: whoever woke the task drained it. What
/// this closes is the other two endings — a cancellation, and a wake that
/// raced with the state changing back — so that the list never holds a handle
/// for a task that is not parked.
fn deregister(ticket: u64) {
    WAITERS.with_borrow_mut(|waiters| {
        if let Some(at) = waiters.iter().position(|parked| parked.ticket == ticket) {
            waiters.remove(at);
        }
    });
}

/// Wakes every task parked on `channel`.
///
/// The handles are taken out of the list before any of them fires: a wake runs
/// no task itself — it queues a task id on the scheduler's tree — but taking
/// them first is what keeps this module free of a borrow held across anything
/// it does not own.
fn notify(channel: *mut ObjHeader) {
    let channel = channel as usize;
    let woken: Vec<Waker> = WAITERS.with_borrow_mut(|waiters| {
        let mut woken = Vec::new();
        let mut at = 0;
        while at < waiters.len() {
            if waiters[at].channel == channel {
                woken.push(waiters.remove(at).waker);
            } else {
                at += 1;
            }
        }
        woken
    });
    for wake in woken {
        wake();
    }
}

/// Gives the core back until something changes about `receiver`, and answers
/// whether the caller may look again.
///
/// # Errors
///
/// [`Ctx::cancel`]'s status when the task was cancelled while it waited — ADR
/// 0072 § 5 reached through the return status rather than through an unwind,
/// because the task is standing on this `extern "C"` frame — and a
/// [`Fault::fatal`] naming `member` when there is no task to park at all,
/// which is a program asking for backpressure on a thread that cannot deliver
/// it.
fn wait(ctx: &mut Ctx, receiver: *mut ObjHeader, member: &str) -> Result<(), Fault> {
    let Some(ticket) = register(receiver) else {
        return Err(Fault::fatal(format!(
            "{NAME}::{member} would wait and there is no scheduler on this thread to wait on"
        )));
    };
    // No deadline: a receiver waits for a sender and for nothing else, and ADR
    // 0074 § 5's "no spelling for an unbounded wait" is about a *network* wait,
    // not about one end of a channel this program owns both halves of.
    let woken = nvs_runtime::host::with_current(|host| host.park(None));
    deregister(ticket);
    match woken {
        Some(Woken::Cancelled) => Err(ctx.cancel()),
        _ => Ok(()),
    }
}

// ============================================================================
// The state, in the slots
// ============================================================================

/// The receiver of one of this class's members.
///
/// # Errors
///
/// [`crate::instance::receiver`]'s, unchanged.
fn channel_of(value: Value, member: &str) -> Result<*mut ObjHeader, Fault> {
    crate::instance::receiver(value, &CLASS, member)
}

/// An integer slot, which only a bug in this crate can find holding anything
/// else.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member and the slot.
fn int_slot(receiver: *mut ObjHeader, index: usize, member: &str) -> Result<i64, Fault> {
    let held = crate::instance::slot(receiver, index);
    held.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected an int in its `{}` slot, got tag {}",
            CLASS.slots[index],
            held.tag_byte()
        ))
    })
}

/// Whether `close()` has run.
fn is_closed(receiver: *mut ObjHeader) -> bool {
    crate::instance::slot(receiver, CLOSED)
        .as_bool()
        .unwrap_or(false)
}

/// How many values the channel is holding right now.
///
/// # Errors
///
/// [`store::borrow`]'s, unchanged.
fn queued(receiver: *mut ObjHeader, member: &str) -> Result<usize, Fault> {
    Ok(store::borrow(receiver, ITEMS, &CLASS, member)?.count())
}

/// The queue key of the `nth` value ever sent, rendered the way
/// [`crate::heap`] renders a tree position: decimal, so the array's own
/// insertion order and the key order agree.
fn key(nth: i64) -> Vec<u8> {
    nth.to_string().into_bytes()
}

// ============================================================================
// The members
// ============================================================================

nvs_runtime::nvs_helper! {
    /// `new Core\Task\Channel<T>(uint $capacity)` — a fresh empty channel that
    /// holds `$capacity` values before a `send` suspends.
    ///
    /// A `capacity` of zero is refused rather than raised to one. `nvs-host`'s
    /// own channel raises it, and says why: a zero-capacity queue is a
    /// rendezvous, a different primitive, and not something to acquire by
    /// passing a computed 0. At *this* surface the same argument ends
    /// differently — a program wrote the number, so the honest answer is to
    /// name it rather than to substitute one.
    fn nvs_core_channel_new(_ctx, args: [1]) {
        let capacity = args[0].as_uint().ok_or_else(|| {
            Fault::fatal(format!(
                "{NAME}'s constructor expected a uint for `capacity`, got tag {}",
                args[0].tag_byte()
            ))
        })?;
        if capacity == 0 {
            return Err(Fault::fatal(format!(
                "{NAME}'s constructor expected a capacity of at least 1, got {capacity}"
            )));
        }
        // Held as an `int` rather than as the `uint` it arrived as: the slot is
        // only ever compared against a queue length, and one representation for
        // that comparison beats two.
        let capacity = i64::try_from(capacity).unwrap_or(i64::MAX);
        Ok(crate::instance::build(
            &CLASS,
            [
                Value::array(NvsArray::new()),
                Value::int(capacity),
                Value::bool(false),
                Value::int(0),
                Value::null(),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Task\Channel<T>::send(T $value): void` — queues `$value`,
    /// suspending the calling task for as long as the channel is full.
    ///
    /// The loop is the contract: a task resumed here re-reads the queue rather
    /// than assuming the wake meant room, because
    /// [`nvs_runtime::host::Waker`] is a hint and every waiter on this channel
    /// is woken together.
    fn nvs_core_channel_send(ctx, args: [2]) {
        let receiver = channel_of(args[0], "send")?;
        let value = args[1];
        loop {
            if is_closed(receiver) {
                return Err(Fault::fatal(format!(
                    "{NAME}::send was called on a closed channel"
                )));
            }
            let capacity = int_slot(receiver, CAPACITY, "send")?;
            if i64::try_from(queued(receiver, "send")?).unwrap_or(i64::MAX) < capacity {
                let nth = int_slot(receiver, NEXT, "send")?;
                store::edit(receiver, ITEMS, &CLASS, "send", |items| {
                    #[expect(
                        unsafe_code,
                        reason = "the argument is borrowed from the caller's \
                                  frame, so the copy the queue keeps needs a \
                                  reference of its own"
                    )]
                    unsafe {
                        value.retain();
                    }
                    items.set(NvsStr::new(&key(nth)), value);
                })?;
                crate::instance::set_slot(receiver, NEXT, Value::int(nth.wrapping_add(1)));
                // After the store, never before: a woken consumer runs on this
                // core and must find the value already there.
                notify(receiver);
                return Ok(Value::null());
            }
            wait(ctx, receiver, "send")?;
        }
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Task\Channel<T>::close(): void` — ends the stream, so that a
    /// consumer's `foreach` finishes once the queued values are drained.
    ///
    /// Idempotent, and it does not discard what is already queued: a value
    /// sent before the close is delivered after it. Closing twice is not an
    /// error, because the second call asks for a state the channel is already
    /// in and there is nothing for a diagnostic to help with.
    fn nvs_core_channel_close(_ctx, args: [1]) {
        let receiver = channel_of(args[0], "close")?;
        crate::instance::set_slot(receiver, CLOSED, Value::bool(true));
        // Every consumer parked on an empty channel can now finish, and every
        // producer parked on a full one now has a fatal to report.
        notify(receiver);
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<T>::iterate(): Iterator<T>` — the channel itself.
    ///
    /// Not a registered member: it is reached by name through this class's
    /// method table, so its receiver is **transferred** rather than borrowed
    /// ([`crate::cursor`]'s module doc owns that convention). This one hands
    /// the very reference it was given straight back as its answer, which is
    /// why — alone among the three — it neither retains nor releases: a
    /// snapshot would defeat the whole type.
    fn nvs_core_channel_iterate(_ctx, args: [1]) {
        channel_of(args[0], nvs_runtime::sequence::ITERATE)?;
        Ok(args[0])
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<T>::advance(): bool` — takes the next value out, suspending
    /// while the channel is empty and open, and answering `false` once it is
    /// empty and closed.
    ///
    /// This is where a consumer waits, and it is the only place a `foreach`
    /// over a channel can: `current()` answers what this already took.
    fn nvs_core_channel_advance(ctx, args: [1]) {
        let stepped = advance(ctx, args[0]);
        crate::cursor::consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<T>::current(): T` — the value the last `advance()` took out.
    fn nvs_core_channel_current(_ctx, args: [1]) {
        let read = current(args[0]);
        crate::cursor::consume(args[0]);
        read
    }
}

/// [`nvs_core_channel_advance`]'s body, so that the receiver reference a
/// virtual call transferred is released on both edges.
///
/// # Errors
///
/// [`wait`]'s, and a [`Fault::fatal`] for slots this crate wrote wrong.
fn advance(ctx: &mut Ctx, value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::ADVANCE;
    let receiver = channel_of(value, member)?;
    loop {
        let taken = store::edit(receiver, ITEMS, &CLASS, member, |items| {
            let slot = items.next_slot(0)?;
            let held = items.value_at(slot)?;
            let held_key = items.key_at(slot)?;
            let key = held_key.as_bytes().to_vec();
            #[expect(
                unsafe_code,
                reason = "the retain and the `unset`'s release are one \
                          transfer: this frame ends up owning exactly the \
                          reference the queue gave up"
            )]
            unsafe {
                held.retain();
            }
            items.unset(&key);
            Some(held)
        })?;
        if let Some(held) = taken {
            crate::instance::set_slot(receiver, CURRENT, held);
            // A producer parked on a full channel now has room.
            notify(receiver);
            return Ok(Value::bool(true));
        }
        if is_closed(receiver) {
            return Ok(Value::bool(false));
        }
        wait(ctx, receiver, member)?;
    }
}

/// [`nvs_core_channel_current`]'s body, retained because the slot keeps its
/// own reference until the next `advance()`.
///
/// # Errors
///
/// A [`Fault::fatal`] for a receiver that is not a channel.
fn current(value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::CURRENT;
    let receiver = channel_of(value, member)?;
    let held = crate::instance::slot(receiver, CURRENT);
    #[expect(
        unsafe_code,
        reason = "the slot keeps its reference until the next `advance`, so \
                  the value handed back needs one of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

#[cfg(test)]
mod tests {
    use super::*;

    use nvs_runtime::call;

    /// A fresh channel of `capacity`, as the constructor builds one.
    fn channel(capacity: u64) -> Value {
        let mut ctx = Ctx::buffered();
        call(nvs_core_channel_new, &mut ctx, &[Value::uint(capacity)])
            .expect("a channel of a positive capacity is built")
    }

    /// Runs `member` with `chan` as the receiver and `rest` past it.
    fn on(chan: Value, member: nvs_runtime::NvsFn, rest: &[Value]) -> Value {
        let mut ctx = Ctx::buffered();
        let mut args = vec![chan];
        args.extend_from_slice(rest);
        call(member, &mut ctx, &args).expect("a channel member over ints does not throw")
    }

    /// A receiver reference the virtual-call convention would have
    /// transferred, so that a direct call to one of the three protocol members
    /// balances the release it owes.
    fn transferred(chan: Value) -> Value {
        #[expect(
            unsafe_code,
            reason = "this test frame owns the channel and is standing in for \
                      the dispatch that would have retained it"
        )]
        unsafe {
            chan.retain();
        }
        chan
    }

    #[test]
    fn a_capacity_of_zero_is_named_rather_than_raised_to_one() {
        let mut ctx = Ctx::buffered();
        let refused = call(nvs_core_channel_new, &mut ctx, &[Value::uint(0)]);
        assert!(refused.is_err(), "a zero capacity is refused");
    }

    // covers: Core\Task\Channel::send
    #[test]
    fn values_come_back_in_the_order_they_were_sent() {
        let chan = channel(4);
        for n in 1..=3 {
            on(chan, nvs_core_channel_send, &[Value::int(n)]);
        }
        on(chan, nvs_core_channel_close, &[]);
        let mut seen = Vec::new();
        while on(transferred(chan), nvs_core_channel_advance, &[])
            .as_bool()
            .expect("`advance` answers a bool")
        {
            seen.push(
                on(transferred(chan), nvs_core_channel_current, &[])
                    .as_int()
                    .expect("the channel was sent ints"),
            );
        }
        assert_eq!(seen, vec![1, 2, 3]);
    }

    // covers: Core\Task\Channel::close
    #[test]
    fn a_closed_and_drained_channel_ends_the_walk_rather_than_waiting() {
        let chan = channel(2);
        on(chan, nvs_core_channel_close, &[]);
        assert_eq!(
            on(transferred(chan), nvs_core_channel_advance, &[]).as_bool(),
            Some(false),
            "an empty closed channel has nothing to wait for"
        );
    }

    // covers: Core\Task\Channel::send
    #[test]
    fn a_send_with_no_scheduler_under_it_is_named_rather_than_blocking_the_core() {
        let chan = channel(1);
        on(chan, nvs_core_channel_send, &[Value::int(1)]);
        let mut ctx = Ctx::buffered();
        let refused = call(nvs_core_channel_send, &mut ctx, &[chan, Value::int(2)]);
        assert!(
            refused.is_err(),
            "a full channel with no task beneath it reports rather than parks"
        );
    }
}
