//! Draining `rule:iteration/foreach-subjects`'s iterable shapes from native code.
//!
//! `Core\Arr::from`'s parameter is *whatever `foreach` accepts* —
//! `nvs_stdlib::registry::CoreTy::Iterated`, which owns why an `array<T>` is
//! one of them — and this is the one place such an argument is read. Each of
//! them arrives in one 16-byte [`Value`]: an `array<T>` as
//! [`Tag::Array`](crate::Tag::Array), an `Iterable<T>` or an `Iterator<T>` as
//! [`Tag::Object`](crate::Tag::Object). Nothing else can reach here, because the
//! checker already refused it.
//!
//! # Why the cursor is driven by name
//!
//! `iterate`, `advance` and `current` are bodiless declarations
//! (`nvs_types::iter_lib`), so no compiled function exists for a *call site* to
//! name — the class the member is handed decides, and it is a class this crate
//! knows nothing about. [`crate::object`]'s descriptor carries exactly the
//! table that answers it, and [`crate::call_closure`] already reaches a
//! closure's `invoke` through it. This is that same lookup, over a cursor's
//! names rather than a closure's one, and it is the same dispatch a `foreach`
//! over the same value performs — [`crate::dispatch`] is where it lives, since
//! more than one member asks an object something by name.
//!
//! An object answering [`ITERATE`] is driven through the cursor that returns,
//! and one that does not is the cursor. A class implementing both interfaces
//! therefore iterates from a *fresh* cursor, which is what `Iterable<T>` means
//! and what `foreach` does with the same value.
//!
//! # What it costs, and what bounds it
//!
//! One `Vec<Value>` plus one owned reference per element, held for the length
//! of the call and handed to the caller — so a member that materialises a
//! sequence holds it twice at the moment it builds its result. Handing back a
//! cursor the caller drives itself would buy that back only for a member that
//! can consume one element at a time, and `Core\Arr::from`'s whole job is to
//! materialise. `limit` is the *caller's* bound on that, and the spec's
//! `{limit?: uint}` is that argument.
//!
//! **`[limits] memory` is what bounds a sequence that never ends**, and [`drain`]
//! asks for it in front of every push through [`crate::abi::affordable`].
//! `counting_alloc` charges the `Vec` and every element to the request as they
//! are taken, but a native loop passes no statement boundary and calls no
//! member, so nothing between two pushes would otherwise read the balance:
//! the request would hold the host's memory rather than its own ceiling, which
//! `rule:programs/memory-priority` does not permit whatever the caller passed
//! for `limit`.
//!
//! **The ask is priced at what the push takes, which is the `Vec`'s growth and
//! not the element**: nothing while there is room, and the whole block the
//! `Vec` grows into where it is full. A `Vec` doubles, so the push that finds
//! it full takes as much again as the loop has taken so far. An ask of one
//! element is affordable right up to that push, and the push then carries the
//! request past its whole budget, `rule:errors/on-limit`'s reserve included —
//! the breach is found by the entry poll of the next `advance` the drive calls,
//! with the `Vec` still held, and the program's `Core\Fatal::onLimit` handler
//! is entered with nothing to run on: its first member call is stopped, so it
//! says nothing. Refused in front of the growth, the drain gives back what it
//! took and [`crate::run_helper`]'s failure arm runs the handler on a budget
//! that still has its reserve — the same two lines an array append refused in
//! compiled code reaches.
//! `tests/conformance/core/arr-from-over-a-sequence-with-no-end-runs-the-on-limit-handler.nvst`
//! pins the handler and the mark, beside
//! `tests/conformance/core/arr-from-over-a-sequence-with-no-end-is-stopped-by-the-memory-ceiling.nvst`,
//! which pins the stop.
//!
//! [`for_each`] is the entry for the member that *can* consume one element at
//! a time, such as `Core\IO::writeStream` — a stream reaching disk
//! (`rule:core-classes/io-write-stream`) must not hold the file it is writing. It is the same drive
//! with the `Vec` taken out, and [`drain`] is written over it, so there is one
//! cursor loop rather than a second one that could disagree about when
//! `iterate()` is called or who owns an element.

use std::mem::ManuallyDrop;

use crate::abi::Fault;
use crate::array::{ArrayHeader, NvsArray};
use crate::ctx::Ctx;
use crate::dispatch::method_address;
use crate::value::Value;

/// `Iterable<T>`'s sole member (`rule:iteration/two-interfaces`) — a fresh cursor over the same
/// sequence. Must agree with `nvs_types::iter_lib`'s seeded spelling.
pub const ITERATE: &str = "iterate";

/// `Iterator<T>`'s "move to the next element, `false` once exhausted" member
/// (`rule:iteration/two-interfaces`).
pub const ADVANCE: &str = "advance";

/// `Iterator<T>`'s "the element [`ADVANCE`] just moved to" member
/// (`rule:iteration/two-interfaces`).
pub const CURRENT: &str = "current";

/// Reads `sequence` — an `array<T>`, an `Iterable<T>` or an `Iterator<T>` —
/// into at most `limit` elements, in order, each an **owned** reference the
/// caller must store or release.
///
/// Keys are not returned, for the reason the spec's § 2 gives `Core\Arr::from`
/// itself: a cursor has none (`rule:iteration/two-interfaces` gives `Iterator<T>` exactly
/// `advance` and `current`), so a member reading a sequence can only ever
/// answer with a list.
///
/// `what` names the member for a fault message and is never shown to a program
/// that is behaving.
///
/// # Errors
///
/// [`Fault::Pending`] when a driven `iterate`/`advance`/`current` throws,
/// carrying that call's own status so the exception reaches the request
/// unchanged. [`Fault::Fatal`] when `sequence` is neither an array nor an
/// object, or the object declares none of the members its interface owes —
/// both engine faults: the checker admits only `rule:iteration/foreach-subjects`'s shapes
/// here, and a class claiming one of the two interfaces owes its members by
/// `nvs_types::conformance`.
pub fn drain(
    ctx: &mut Ctx,
    sequence: Value,
    limit: Option<usize>,
    what: &str,
) -> Result<Vec<Value>, Fault> {
    let mut out = Vec::new();
    let mut collect = |value: Value| {
        // Asked in front of the push, for the reason `crate::abi::affordable`
        // gives: the balance already carries what this loop has taken, and this
        // is the only thing in the loop that reads it. A sequence with no end is
        // stopped here, at the ceiling, rather than where the host runs out.
        //
        // Priced at what the push takes. A full `Vec` takes the whole block it
        // grows into, on top of the one it holds until the move is done, and
        // the module doc owns why asking for one element in front of that push
        // leaves the limit handler nothing to run on. One with room takes
        // nothing, and the ask is made all the same: zero bytes is still a read
        // of the balance, which is what stops a drive whose elements are what
        // grew.
        let grown = (out.len() == out.capacity()).then(|| grown_capacity(out.capacity()));
        let ask = match grown {
            Some(capacity) => capacity.checked_mul(size_of::<Value>()),
            None => Some(0),
        };
        if let Err(fault) = crate::abi::affordable(ask, what) {
            #[expect(
                unsafe_code,
                reason = "`each` hands the sink an owned reference and releases \
                          nothing it has given away, so this element is this \
                          frame's on the call that refuses it"
            )]
            unsafe {
                value.release();
            }
            return Err(fault);
        }
        if let Some(capacity) = grown {
            // The growth that was just priced, taken here so that the push
            // below is never the one that chooses a size of its own.
            out.reserve_exact(capacity.saturating_sub(out.len()));
        }
        out.push(value);
        Ok(())
    };
    let result = each(ctx, sequence, limit, what, &mut collect);
    match result {
        Ok(()) => Ok(out),
        // A throw partway through leaves this frame owning every element
        // drained so far, and nothing else will ever see them.
        Err(fault) => {
            for value in out {
                #[expect(
                    unsafe_code,
                    reason = "each element is an owned reference this frame \
                              produced and has not handed anywhere"
                )]
                unsafe {
                    value.release();
                }
            }
            Err(fault)
        }
    }
}

/// How many elements [`drain`]'s `Vec` holds after growing from `capacity`.
///
/// Doubling, from the four a `Vec` of 16-byte elements starts at — the policy
/// `Vec::push` would have chosen, stated here because [`drain`] has to price
/// the growth before it happens and so cannot leave the choice to the push.
/// Saturating: a count that cannot double is one whose byte size
/// [`crate::abi::affordable`] refuses as unallocatable.
fn grown_capacity(capacity: usize) -> usize {
    capacity.saturating_mul(2).max(4)
}

/// Reads `sequence` — an `array<T>`, an `Iterable<T>` or an `Iterator<T>` —
/// one element at a time, handing each to `sink` and never holding two at
/// once.
///
/// [`drain`] without the `Vec`, for the member that consumes as it goes rather
/// than materialising: `Core\IO::writeStream` writes each chunk to the file
/// and keeps none of them, so what it holds is bounded by the largest single
/// element instead of by the length of the stream.
///
/// **`sink` is handed an owned reference and owns it from that moment**,
/// including on the call it fails — this function releases nothing it has
/// already given away. That is the same contract [`drain`] answers to, where
/// the sink is the `Vec` and the caller frees it.
///
/// There is no `limit` here for the reason there is one on [`drain`]: a sink
/// that wants to stop early stops by failing, and it is the sink — not this
/// function — that knows what bound it is enforcing.
///
/// # Errors
///
/// Whatever `sink` returns, or [`drain`]'s own two faults for the same two
/// reasons.
pub fn for_each(
    ctx: &mut Ctx,
    sequence: Value,
    what: &str,
    sink: &mut dyn FnMut(Value) -> Result<(), Fault>,
) -> Result<(), Fault> {
    each(ctx, sequence, None, what, sink)
}

/// The drive both entries share: `rule:iteration/foreach-subjects`'s shapes, split into the
/// two representations they arrive in.
fn each(
    ctx: &mut Ctx,
    sequence: Value,
    limit: Option<usize>,
    what: &str,
    sink: &mut dyn FnMut(Value) -> Result<(), Fault>,
) -> Result<(), Fault> {
    if let Some(array) = sequence.array_ptr() {
        return each_array(array, limit, sink);
    }
    if sequence.obj_ptr().is_some() {
        return each_cursor(ctx, sequence, limit, what, sink);
    }
    Err(Fault::fatal(format!(
        "internal error: {what} was handed tag {} where a sequence was expected",
        sequence.tag_byte()
    )))
}

/// The array half: every value in key order, each retained on the way out
/// because it belongs to the subject array rather than to this frame.
fn each_array(
    array: *mut ArrayHeader,
    limit: Option<usize>,
    sink: &mut dyn FnMut(Value) -> Result<(), Fault>,
) -> Result<(), Fault> {
    #[expect(
        unsafe_code,
        reason = "a Tag::Array argument owns a reference to a live allocation, \
                  so it is live for the length of this call"
    )]
    // `ManuallyDrop`, because `from_raw` hands back an *owning* handle and the
    // reference being read through here belongs to the caller's argument slot.
    let subject = ManuallyDrop::new(unsafe { NvsArray::from_raw(array) });

    let mut taken = 0usize;
    let mut from = 0usize;
    while limit.is_none_or(|limit| taken < limit) {
        let Some(slot) = subject.next_slot(from) else {
            break;
        };
        let value = subject
            .value_at(slot)
            .expect("next_slot only names live entries");
        #[expect(
            unsafe_code,
            reason = "the entry is owned by the subject array, which outlives \
                      this call, so the copy handed to the caller needs a \
                      reference of its own"
        )]
        unsafe {
            value.retain();
        }
        sink(value)?;
        taken += 1;
        from = slot + 1;
    }
    Ok(())
}

/// The object half: `iterate()` where the value reaches `Iterable<T>`, then
/// the `advance()`/`current()` pair until it says it is exhausted.
fn each_cursor(
    ctx: &mut Ctx,
    sequence: Value,
    limit: Option<usize>,
    what: &str,
    sink: &mut dyn FnMut(Value) -> Result<(), Fault>,
) -> Result<(), Fault> {
    let cursor = match method_address(sequence, ITERATE, what)? {
        Some(iterate) => call_member(ctx, sequence, iterate)?,
        // Not an `Iterable<T>`, so it is the cursor itself — retained so that
        // both branches leave this frame owning exactly one reference to it.
        None => {
            #[expect(
                unsafe_code,
                reason = "the caller owns a reference to this argument, so the \
                          allocation is live for the length of this call"
            )]
            unsafe {
                sequence.retain();
            }
            sequence
        }
    };

    let result = pump(ctx, cursor, limit, what, sink);
    #[expect(
        unsafe_code,
        reason = "this frame owns exactly the one reference the match above \
                  produced"
    )]
    unsafe {
        cursor.release();
    }
    result
}

/// Drives `cursor` into `sink`. Split out so [`each_cursor`] has one place to
/// release the cursor from, whichever step failed — what the sink has already
/// taken is the sink's, per [`for_each`]'s contract.
fn pump(
    ctx: &mut Ctx,
    cursor: Value,
    limit: Option<usize>,
    what: &str,
    sink: &mut dyn FnMut(Value) -> Result<(), Fault>,
) -> Result<(), Fault> {
    let advance = required_member(cursor, ADVANCE, what)?;
    let current = required_member(cursor, CURRENT, what)?;
    let mut taken = 0usize;
    while limit.is_none_or(|limit| taken < limit) {
        let more = call_member(ctx, cursor, advance)?;
        match more.as_bool() {
            Some(true) => {}
            Some(false) => break,
            None => {
                return Err(Fault::fatal(format!(
                    "internal error: {what} drove an `{ADVANCE}` answering tag {} rather than \
                     a bool",
                    more.tag_byte()
                )));
            }
        }
        sink(call_member(ctx, cursor, current)?)?;
        taken += 1;
    }
    Ok(())
}

/// [`method_address`] where the member is owed rather than optional — the two
/// `Iterator<T>` halves, which `nvs_types::conformance` already made a
/// compile error to omit.
fn required_member(receiver: Value, name: &str, what: &str) -> Result<*const u8, Fault> {
    method_address(receiver, name, what)?.ok_or_else(|| {
        Fault::fatal(format!(
            "internal error: {what} drove a cursor declaring no `{name}`"
        ))
    })
}

/// Calls a no-argument member on `receiver`, returning whatever it produced.
///
/// [`crate::dispatch::call_at`] with an empty argument list: every cursor
/// member `rule:iteration/two-interfaces` declares takes none, and the address is looked up once
/// and driven many times rather than re-resolved per element.
fn call_member(ctx: &mut Ctx, receiver: Value, target: *const u8) -> Result<Value, Fault> {
    crate::dispatch::call_at(ctx, receiver, target, &[])
}
