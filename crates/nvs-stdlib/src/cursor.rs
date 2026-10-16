//! The cursor `docs/spec/01-core-library.md` § 9's collections hand a
//! `foreach` — `rule:iteration/two-interfaces`'s `Iterator<T>` half, over a snapshot list.
//!
//! # Decision: a `Core` collection answers a `foreach` through its method table
//!
//! Not through a native drive of its own. `foreach` over an object is one
//! `iterate()` and then an `advance()`/`current()` pair per element, each an
//! `nvs_ir::ir::InstKind::CallVirtual` that looks the name up in the
//! receiver's own [`nvs_runtime::ClassDesc`] — so a `Core` class joins that
//! protocol by *having* the three names in its descriptor's method table, and
//! nothing in `nvs-ir`, `nvs-codegen` or `nvs_runtime::sequence` learns that
//! `Core` owns the receiver. The rejected alternative was a fourth
//! `nvs_types::ForeachDrive` and a lowering that calls a helper by symbol: it
//! buys one indirect call per element and costs a second iteration protocol,
//! reachable only from `foreach` and not from `Core\Arr::from`, which drives
//! the same three names by name already.
//!
//! [`crate::instance`]'s dispatch roster is the one place a `Core` class's
//! method table is written, and it is what closes [`crate::heap`]'s standing
//! gap about a `Core` receiver carrying no members.
//!
//! # Decision: the cursor walks a snapshot, not the live collection
//!
//! `iterate()` copies the elements it will yield into a fresh list and the
//! cursor walks that. § 9's three classes are the spec's only *mutable* `Core`
//! types, so the live alternative has to answer what a `remove` mid-loop does
//! — PHP's own answer for a rehashed `SplObjectStorage` is undefined — and for
//! [`crate::heap`] it cannot be answered at all: pop order is not the order the
//! entries array is in, so a live cursor would have to consume the heap it is
//! iterating.
//!
//! **What it spends:** one list allocation plus one reference per element, per
//! `foreach`, held for the length of the loop and released with the cursor.
//! That is O(in-flight) and charged to the request that wrote the loop, which
//! is the trade [AGENTS.md](/AGENTS.md)'s ordering asks for: the
//! loop sees the collection as it was when the loop began, whatever the body
//! does to it.
//!
//! # Why this class has no registry row
//!
//! It is a runtime artifact, not surface. `iterate()`'s declared return type is
//! the seeded `Iterator<T>` (`nvs_types::iter_lib`), so no signature ever names
//! this class, and a row in [`crate::registry::CLASSES`] would only make
//! `Core\Cursor` a type a program can write and nothing can produce.
//! [`crate::instance`]'s internal roster is what gives it a descriptor —
//! the same shape [`crate::registry::CoreClass`] already is, minus the row.
//!
//! # A receiver is transferred here, not borrowed
//!
//! Every other `Core` member borrows its arguments (`nvs_ir`'s
//! `ArgOwnership::Borrowed`). These three do not: they are reached as *methods*
//! through a class descriptor, and a virtual call transfers the receiver's
//! reference to the callee exactly as it does for a compiled Novis method — which
//! is why each body below releases argument slot 0 on both edges, and why
//! [`nvs_runtime::dispatch::call_method`] retains before it calls.

use nvs_runtime::sequence::{ADVANCE, CURRENT};
use nvs_runtime::{Fault, NvsArray, ObjHeader, Value};

use crate::identity_store as store;
use crate::registry::CoreClass;

/// The class's fully-qualified name — see this module's docs for why no
/// program can write it.
pub(crate) const NAME: &str = r"Core\Cursor";

/// [`CLASS`]'s slots, by index: the snapshot being walked …
const ITEMS: usize = 0;
/// … and the slot of it the cursor is on, `-1` before the first
/// [`ADVANCE`].
const INDEX: usize = 1;

/// The cursor's own layout — a [`CoreClass`] like any other, and deliberately
/// not one of [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
    methods: &[],
    instance: &[],
    slots: &["items", "index"],
    constants: &[],
};

/// The symbol behind [`ADVANCE`], as [`crate::instance`]'s dispatch roster
/// spells it.
pub(crate) const ADVANCE_SYMBOL: &str = "nvs_core_cursor_advance";
/// The symbol behind [`CURRENT`].
pub(crate) const CURRENT_SYMBOL: &str = "nvs_core_cursor_current";

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        ADVANCE_SYMBOL => (nvs_core_cursor_advance as *const ()).cast(),
        CURRENT_SYMBOL => (nvs_core_cursor_current as *const ()).cast(),
        _ => return None,
    })
}

/// A fresh cursor over `items`, taking over its reference — what every
/// `iterate()` in this crate returns.
pub(crate) fn over(items: NvsArray) -> Value {
    crate::instance::build(&CLASS, [Value::array(items), Value::int(-1)])
}

/// Releases the receiver reference a virtual call transferred — see this
/// module's docs for why exactly these members owe one.
pub(crate) fn consume(receiver: Value) {
    #[expect(
        unsafe_code,
        reason = "a virtual call transfers argument slot 0's reference to the \
                  callee, so this frame owns exactly the one it is releasing"
    )]
    unsafe {
        receiver.release();
    }
}

/// The cursor's own state: its snapshot and the slot it is on.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member for a receiver that is not a cursor or
/// whose slots hold the wrong shape — only a bug in this crate can produce
/// either.
fn state(
    value: Value,
    member: &str,
) -> Result<(*mut ObjHeader, std::mem::ManuallyDrop<NvsArray>, i64), Fault> {
    let receiver = crate::instance::receiver(value, &CLASS, member)?;
    let items = store::borrow(receiver, ITEMS, &CLASS, member)?;
    let held = crate::instance::slot(receiver, INDEX);
    let index = held.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{member} expected an int in its `index` slot, got tag {}",
            held.tag_byte()
        ))
    })?;
    Ok((receiver, items, index))
}

/// [`ADVANCE`]'s body: moves to the next occupied slot, answering whether
/// there was one.
fn stepped(value: Value) -> Result<Value, Fault> {
    let (receiver, items, index) = state(value, ADVANCE)?;
    let from = usize::try_from(index + 1).unwrap_or(0);
    let Some(slot) = items.next_slot(from) else {
        return Ok(Value::bool(false));
    };
    let slot = i64::try_from(slot)
        .map_err(|_| Fault::fatal(format!("{NAME}::{ADVANCE} walked past slot {slot}")))?;
    crate::instance::set_slot(receiver, INDEX, Value::int(slot));
    Ok(Value::bool(true))
}

/// [`CURRENT`]'s body: the element [`ADVANCE`] last moved to, retained,
/// because the snapshot outlives this call.
fn read(value: Value) -> Result<Value, Fault> {
    let (_, items, index) = state(value, CURRENT)?;
    let slot = usize::try_from(index).map_err(|_| {
        Fault::fatal(format!(
            "{NAME}::{CURRENT} was asked for an element before the first `{ADVANCE}`"
        ))
    })?;
    let held = items.value_at(slot).ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{CURRENT} found no element at slot {slot} of a snapshot holding {}",
            items.count()
        ))
    })?;
    #[expect(
        unsafe_code,
        reason = "the element belongs to the snapshot, which outlives this \
                  call, so the value handed back needs a reference of its own"
    )]
    unsafe {
        held.retain();
    }
    Ok(held)
}

nvs_runtime::nvs_helper! {
    /// `Iterator<T>::advance(): bool` for a `Core` collection's cursor.
    fn nvs_core_cursor_advance(_ctx, args: [1]) {
        let stepped = stepped(args[0]);
        consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterator<T>::current(): T` for a `Core` collection's cursor.
    fn nvs_core_cursor_current(_ctx, args: [1]) {
        let read = read(args[0]);
        consume(args[0]);
        read
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_runtime::{Ctx, call};

    /// One cursor over a two-element snapshot, driven exactly as a `foreach`
    /// drives it — a retain before each call, because the callee consumes.
    #[test]
    fn a_cursor_walks_its_snapshot_once() {
        let mut items = NvsArray::new();
        items.append(Value::int(7));
        items.append(Value::int(9));
        let cursor = over(items);

        let mut ctx = Ctx::buffered();
        let mut seen = Vec::new();
        loop {
            #[expect(
                unsafe_code,
                reason = "each call consumes a reference, so the driver holds \
                          one of its own and retains per call — exactly what \
                          `nvs_ir::lower::control`'s cursor loop emits"
            )]
            unsafe {
                cursor.retain();
            }
            let more = call(nvs_core_cursor_advance, &mut ctx, &[cursor]).expect("advance");
            if more.as_bool() != Some(true) {
                break;
            }
            #[expect(unsafe_code, reason = "as above")]
            unsafe {
                cursor.retain();
            }
            let value = call(nvs_core_cursor_current, &mut ctx, &[cursor]).expect("current");
            seen.push(value.as_int().expect("an int element"));
        }
        assert_eq!(seen, vec![7, 9]);
        #[expect(
            unsafe_code,
            reason = "the loop's own reference, released the way the after-block \
                      of a `foreach` releases it"
        )]
        unsafe {
            cursor.release();
        }
    }
}
