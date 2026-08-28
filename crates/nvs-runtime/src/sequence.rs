//! Draining [ADR 0053](../../../docs/adr/0053-iteration-and-generators.md)
//! § 3's three iterable shapes from native code.
//!
//! `Core\Arr::from` is the first member whose parameter is *whatever `foreach`
//! accepts* — `nvs_stdlib::registry::CoreTy::Iterated`, which owns why an
//! `array<T>` is one of the three — and this is the one place such an argument
//! is read. All three arrive in one 16-byte [`Value`]: an `array<T>` as
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
//! closure's `invoke` through it. This is the same lookup with three names
//! instead of one, and it is the same dispatch a `foreach` over the same value
//! performs — [`crate::dispatch`] is where it lives, since more than one
//! member asks an object something by name.
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
//! materialise. `limit` is what bounds this against a generator that never
//! ends: the spec's `{limit?: uint}` is that argument, and a caller passing
//! `None` is trusting its own.

use std::mem::ManuallyDrop;

use crate::abi::Fault;
use crate::array::{ArrayHeader, NvsArray};
use crate::ctx::Ctx;
use crate::dispatch::method_address;
use crate::value::Value;

/// `Iterable<T>`'s sole member (ADR 0053 § 1) — a fresh cursor over the same
/// sequence. Must agree with `nvs_types::iter_lib`'s seeded spelling.
pub const ITERATE: &str = "iterate";

/// `Iterator<T>`'s "move to the next element, `false` once exhausted" member
/// (ADR 0053 § 1).
pub const ADVANCE: &str = "advance";

/// `Iterator<T>`'s "the element [`ADVANCE`] just moved to" member
/// (ADR 0053 § 1).
pub const CURRENT: &str = "current";

/// Reads `sequence` — an `array<T>`, an `Iterable<T>` or an `Iterator<T>` —
/// into at most `limit` elements, in order, each an **owned** reference the
/// caller must store or release.
///
/// Keys are not returned, for the reason the spec's § 2 gives `Core\Arr::from`
/// itself: a cursor has none (ADR 0053 § 1 gives `Iterator<T>` exactly
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
/// both engine faults: the checker admits only ADR 0053 § 3's three shapes
/// here, and a class claiming one of the two interfaces owes its members by
/// `nvs_types::conformance`.
pub fn drain(
    ctx: &mut Ctx,
    sequence: Value,
    limit: Option<usize>,
    what: &str,
) -> Result<Vec<Value>, Fault> {
    if let Some(array) = sequence.array_ptr() {
        return Ok(drain_array(array, limit));
    }
    if sequence.obj_ptr().is_some() {
        return drain_cursor(ctx, sequence, limit, what);
    }
    Err(Fault::fatal(format!(
        "internal error: {what} was handed tag {} where a sequence was expected",
        sequence.tag_byte()
    )))
}

/// The array half: every value in key order, each retained on the way out
/// because it belongs to the subject array rather than to this frame.
fn drain_array(array: *mut ArrayHeader, limit: Option<usize>) -> Vec<Value> {
    #[expect(
        unsafe_code,
        reason = "a Tag::Array argument owns a reference to a live allocation, \
                  so it is live for the length of this call"
    )]
    // `ManuallyDrop`, because `from_raw` hands back an *owning* handle and the
    // reference being read through here belongs to the caller's argument slot.
    let subject = ManuallyDrop::new(unsafe { NvsArray::from_raw(array) });

    let mut out = Vec::new();
    let mut from = 0usize;
    while limit.is_none_or(|limit| out.len() < limit) {
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
        out.push(value);
        from = slot + 1;
    }
    out
}

/// The object half: `iterate()` where the value reaches `Iterable<T>`, then
/// the `advance()`/`current()` pair until it says it is exhausted.
fn drain_cursor(
    ctx: &mut Ctx,
    sequence: Value,
    limit: Option<usize>,
    what: &str,
) -> Result<Vec<Value>, Fault> {
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

    let mut out = Vec::new();
    let result = pump(ctx, cursor, limit, what, &mut out);
    #[expect(
        unsafe_code,
        reason = "this frame owns exactly the one reference the match above \
                  produced"
    )]
    unsafe {
        cursor.release();
    }
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

/// Drives `cursor` into `out`. Split out so [`drain_cursor`] has one place to
/// release the cursor and the partial result from, whichever step failed.
fn pump(
    ctx: &mut Ctx,
    cursor: Value,
    limit: Option<usize>,
    what: &str,
    out: &mut Vec<Value>,
) -> Result<(), Fault> {
    let advance = required_member(cursor, ADVANCE, what)?;
    let current = required_member(cursor, CURRENT, what)?;
    while limit.is_none_or(|limit| out.len() < limit) {
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
        out.push(call_member(ctx, cursor, current)?);
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
/// member ADR 0053 § 1 declares takes none, and the address is looked up once
/// and driven many times rather than re-resolved per element.
fn call_member(ctx: &mut Ctx, receiver: Value, target: *const u8) -> Result<Value, Fault> {
    crate::dispatch::call_at(ctx, receiver, target, &[])
}
