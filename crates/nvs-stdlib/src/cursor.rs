//! The cursor `Core\ObjectMap`, `Core\ObjectSet` and `Core\Heap` hand a
//! `foreach` —`rule:iteration/two-interfaces`'s `Iterator<T>` half, over an
//! array it holds one reference on: a map's or set's own store, or a heap's
//! sorted copy.
//!
//! Beside it, [`KEYED_CLASS`] is the keyed cursor `Core\ObjectMap::iterateKeyed()`
//! hands a `foreach` with a key binding, `rule:iteration/keyed-interfaces`'s
//! `KeyedIterator<K, V>` half. It shares the map's stores the same way, and
//! everything below holds for it too.
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
//! # Decision: the cursor shares the collection's store, and a write separates it
//!
//! A loop sees the collection as it was when the loop began, whatever the body
//! does to it. Those three classes are *mutable* `Core` types, so a cursor over
//! the live collection would have to answer what a `remove` mid-loop does, and
//! PHP's own answer for a rehashed `SplObjectStorage` is undefined.
//!
//! A map's or set's `iterate()` takes one reference on the store it walks and
//! copies nothing: the key store of a chain or identity-keyed map, and the one
//! store of every other map and set. The cursor walks that array by slot
//! ([`NvsArray::next_slot`]). Its `yields` slot ([`Yield`]) says what each
//! slot gives: the value, the key as a `string`, or the key as an `int`, which
//! is how a `string` or `int` map, whose keys are the store's own keys, yields
//! them. An `int` key is read through [`NvsArray::slot_key`], with no
//! allocation while the store is a list.
//!
//! A write in the body separates the collection once. Every store write goes
//! through an [`NvsArray`] mutator, which copies a shared array before it
//! writes ([`crate::identity_store::edit`]), and `clear` and a re-key put a
//! fresh array in the slot. Either way the cursor keeps the array it began
//! with, unchanged.
//!
//! `Core\Heap` keeps its copy. Its storage order is not its pop order, so its
//! cursor walks the sorted list [`crate::heap`]'s `sorted` builds, which
//! nothing else holds.
//!
//! **What it spends:** a map's or set's loop costs one reference on its store,
//! held for the length of the loop and released with the cursor. A body that
//! writes copies the store once, which is what every loop start cost before.
//! A heap's loop costs one sorted list and one reference per element. All of it
//! is O(in-flight) and charged to the request that wrote the loop.
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

/// [`CLASS`]'s slots, by index: the array being walked, which the cursor holds
/// one reference on …
const ITEMS: usize = 0;
/// … the slot of it the cursor is on, `-1` before the first [`ADVANCE`] …
const INDEX: usize = 1;
/// … and what each slot yields, a [`Yield`].
const YIELDS: usize = 2;

/// What a cursor yields at each slot of the array it walks. The `yields` slot
/// holds it as an `int`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Yield {
    /// The value at the slot: a list's element, a set's member, or a chain or
    /// identity-keyed map's key from its key store.
    Value,
    /// The key at the slot, as a `string`: a `string` map's key.
    StrKey,
    /// The key at the slot, as an `int`: an `int` map's key.
    IntKey,
}

impl Yield {
    /// The value the `yields` slot holds for this mode.
    const fn value(self) -> Value {
        Value::int(match self {
            Self::Value => 0,
            Self::StrKey => 1,
            Self::IntKey => 2,
        })
    }

    /// The mode a `yields` slot's value names. Only this module writes it.
    fn of(value: Value) -> Self {
        match value.as_int() {
            Some(1) => Self::StrKey,
            Some(2) => Self::IntKey,
            _ => Self::Value,
        }
    }
}

/// The cursor's own layout — a [`CoreClass`] like any other, and deliberately
/// not one of [`crate::registry::CLASSES`].
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
    methods: &[],
    instance: &[],
    slots: &["items", "index", "yields"],
    constants: &[],
};

/// The symbol behind [`ADVANCE`], as [`crate::instance`]'s dispatch roster
/// spells it.
pub(crate) const ADVANCE_SYMBOL: &str = "nvs_core_cursor_advance";
/// The symbol behind [`CURRENT`].
pub(crate) const CURRENT_SYMBOL: &str = "nvs_core_cursor_current";

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    Some(nvs_runtime::Entry::borrowed(match symbol {
        ADVANCE_SYMBOL => (nvs_core_cursor_advance as *const ()).cast(),
        CURRENT_SYMBOL => (nvs_core_cursor_current as *const ()).cast(),
        _ => keyed_address(symbol)?,
    }))
}

/// A fresh cursor over the values of `items`, taking over its reference: what
/// every `iterate()` in this crate returns but a `string` or `int` map's.
pub(crate) fn over(items: NvsArray) -> Value {
    yielding(items, Yield::Value)
}

/// A fresh cursor over `items` that yields what `yields` names at each slot,
/// taking over its reference. A map or set passes a clone of its own store,
/// which is one reference and no copy.
pub(crate) fn yielding(items: NvsArray, yields: Yield) -> Value {
    crate::instance::build(
        &CLASS,
        [Value::array(items), Value::int(-1), yields.value()],
    )
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

/// The cursor's own state: the array it walks and the slot it is on.
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

/// [`CURRENT`]'s body: what the slot [`ADVANCE`] last moved to yields, as a
/// value the caller owns. A value is retained, because the array outlives this
/// call. A key is read as the [`Yield`] says, with no allocation for an `int`
/// key while the array is a list.
fn read(value: Value) -> Result<Value, Fault> {
    let (receiver, items, index) = state(value, CURRENT)?;
    let slot = usize::try_from(index).map_err(|_| {
        Fault::fatal(format!(
            "{NAME}::{CURRENT} was asked for an element before the first `{ADVANCE}`"
        ))
    })?;
    let yields = Yield::of(crate::instance::slot(receiver, YIELDS));
    let found = match yields {
        Yield::Value => items.value_at(slot).inspect(|held| {
            #[expect(
                unsafe_code,
                reason = "the element belongs to the walked array, which outlives \
                          this call, so the value handed back needs a reference of \
                          its own"
            )]
            unsafe {
                held.retain();
            }
        }),
        Yield::StrKey => store::key_value(&items, slot, store::Keying::Str),
        Yield::IntKey => store::key_value(&items, slot, store::Keying::Int),
    };
    found.ok_or_else(|| {
        Fault::fatal(format!(
            "{NAME}::{CURRENT} found no element at slot {slot} of an array holding {}",
            items.count()
        ))
    })
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

/// The keyed cursor's fully-qualified name. It is internal for [`NAME`]'s
/// reason: `iterateKeyed()` declares the seeded `KeyedIterator<K, V>`, so no
/// signature names this class.
pub(crate) const KEYED_NAME: &str = r"Core\KeyedCursor";

/// [`KEYED_CLASS`]'s slots, by index: the map's key store, which a chain or
/// identity-keyed map reads its keys from …
const KEYED_KEYS: usize = 0;
/// … the map's value store, which the cursor steps through …
const KEYED_VALUES: usize = 1;
/// … the slot of the key store the cursor is on, `-1` before the first
/// [`ADVANCE`] …
const KEYED_KEY_INDEX: usize = 2;
/// … the slot of the value store the cursor is on, `-1` before the first
/// [`ADVANCE`] …
const KEYED_INDEX: usize = 3;
/// … and where each key comes from, a [`Yield`]: [`Yield::Value`] reads it from
/// the key store, and [`Yield::StrKey`] or [`Yield::IntKey`] from the value
/// store's own key.
const KEYED_KEYS_FROM: usize = 4;

/// The keyed cursor `Core\ObjectMap::iterateKeyed()` returns:
/// `rule:iteration/keyed-interfaces`'s `KeyedIterator<K, V>` half, over the
/// map's own stores.
///
/// It holds one reference on each store and copies nothing, so a loop sees the
/// map as it was when the loop began, as [`CLASS`] does. A chain or
/// identity-keyed map pairs its two stores by order (`crate::objmap`'s docs own
/// that alignment), so the cursor keeps one slot per store and steps both. A
/// `string` or `int` map has one store, and the key is that store's own key.
///
/// **What it spends:** one more reference than a value-only loop over a chain
/// or identity-keyed map, held for the length of the loop, and nothing per
/// entry beyond the key and the value it returns.
pub(crate) const KEYED_CLASS: CoreClass = CoreClass {
    name: KEYED_NAME,
    doc: None,
    methods: &[],
    instance: &[],
    slots: &["keys", "values", "keyIndex", "index", "keysFrom"],
    constants: &[],
};

/// The symbol behind the keyed cursor's [`ADVANCE`].
pub(crate) const KEYED_ADVANCE_SYMBOL: &str = "nvs_core_keyed_cursor_advance";
/// The symbol behind the keyed cursor's `key`.
pub(crate) const KEYED_KEY_SYMBOL: &str = "nvs_core_keyed_cursor_key";
/// The symbol behind the keyed cursor's [`CURRENT`].
pub(crate) const KEYED_CURRENT_SYMBOL: &str = "nvs_core_keyed_cursor_current";

/// The address of one of this module's keyed symbols, or `None`. [`address`]
/// asks it after its own rows.
fn keyed_address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        KEYED_ADVANCE_SYMBOL => (nvs_core_keyed_cursor_advance as *const ()).cast(),
        KEYED_KEY_SYMBOL => (nvs_core_keyed_cursor_key as *const ()).cast(),
        KEYED_CURRENT_SYMBOL => (nvs_core_keyed_cursor_current as *const ()).cast(),
        _ => return None,
    })
}

/// A fresh keyed cursor over a map's `values` store, taking over the reference
/// on each store. `keys_from` says where each key comes from: [`Yield::Value`]
/// reads it from `keys`, and [`Yield::StrKey`] or [`Yield::IntKey`] from the
/// `values` store's own key, in which case `keys` is the empty array.
pub(crate) fn keyed(keys: NvsArray, values: NvsArray, keys_from: Yield) -> Value {
    crate::instance::build(
        &KEYED_CLASS,
        [
            Value::array(keys),
            Value::array(values),
            Value::int(-1),
            Value::int(-1),
            keys_from.value(),
        ],
    )
}

/// The `int` a keyed cursor's index slot has.
fn keyed_index(receiver: *mut ObjHeader, index: usize, member: &str) -> Result<i64, Fault> {
    let held = crate::instance::slot(receiver, index);
    held.as_int().ok_or_else(|| {
        Fault::fatal(format!(
            "{KEYED_NAME}::{member} expected an int in slot {index}, got tag {}",
            held.tag_byte()
        ))
    })
}

/// The slot `index` names, once [`ADVANCE`] has moved there.
fn keyed_slot(receiver: *mut ObjHeader, index: usize, member: &str) -> Result<usize, Fault> {
    usize::try_from(keyed_index(receiver, index, member)?).map_err(|_| {
        Fault::fatal(format!(
            "{KEYED_NAME}::{member} was asked for an entry before the first `{ADVANCE}`"
        ))
    })
}

/// The keyed cursor's [`ADVANCE`]: moves the value slot to the next entry, and
/// the key slot with it when the keys have a store of their own.
fn keyed_stepped(value: Value) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(value, &KEYED_CLASS, ADVANCE)?;
    let values = store::borrow(receiver, KEYED_VALUES, &KEYED_CLASS, ADVANCE)?;
    let from = usize::try_from(keyed_index(receiver, KEYED_INDEX, ADVANCE)? + 1).unwrap_or(0);
    let Some(slot) = values.next_slot(from) else {
        return Ok(Value::bool(false));
    };
    let as_int = |slot: usize| {
        i64::try_from(slot)
            .map_err(|_| Fault::fatal(format!("{KEYED_NAME}::{ADVANCE} walked past slot {slot}")))
    };
    crate::instance::set_slot(receiver, KEYED_INDEX, Value::int(as_int(slot)?));
    if Yield::of(crate::instance::slot(receiver, KEYED_KEYS_FROM)) == Yield::Value {
        let keys = store::borrow(receiver, KEYED_KEYS, &KEYED_CLASS, ADVANCE)?;
        let from =
            usize::try_from(keyed_index(receiver, KEYED_KEY_INDEX, ADVANCE)? + 1).unwrap_or(0);
        let key_slot = keys.next_slot(from).ok_or_else(|| {
            Fault::fatal(format!(
                "{KEYED_NAME}::{ADVANCE} found a value at slot {slot} and no key to pair with it"
            ))
        })?;
        crate::instance::set_slot(receiver, KEYED_KEY_INDEX, Value::int(as_int(key_slot)?));
    }
    Ok(Value::bool(true))
}

/// Retains a value that belongs to a store the cursor holds, so the caller owns
/// what it is given.
fn owned(held: Value) -> Value {
    #[expect(
        unsafe_code,
        reason = "the value belongs to a store that outlives this call, so the \
                  value handed back needs a reference of its own"
    )]
    unsafe {
        held.retain();
    }
    held
}

/// The keyed cursor's `key`: the key of the entry [`ADVANCE`] last moved to,
/// as a value the caller owns.
fn keyed_key(value: Value) -> Result<Value, Fault> {
    let member = nvs_runtime::sequence::KEY;
    let receiver = crate::instance::receiver(value, &KEYED_CLASS, member)?;
    let found = match Yield::of(crate::instance::slot(receiver, KEYED_KEYS_FROM)) {
        Yield::Value => {
            let slot = keyed_slot(receiver, KEYED_KEY_INDEX, member)?;
            let keys = store::borrow(receiver, KEYED_KEYS, &KEYED_CLASS, member)?;
            keys.value_at(slot).map(owned)
        }
        from @ (Yield::StrKey | Yield::IntKey) => {
            let slot = keyed_slot(receiver, KEYED_INDEX, member)?;
            let values = store::borrow(receiver, KEYED_VALUES, &KEYED_CLASS, member)?;
            let keying = if from == Yield::StrKey {
                store::Keying::Str
            } else {
                store::Keying::Int
            };
            store::key_value(&values, slot, keying)
        }
    };
    found.ok_or_else(|| Fault::fatal(format!("{KEYED_NAME}::{member} found no key")))
}

/// The keyed cursor's [`CURRENT`]: the value of the entry [`ADVANCE`] last
/// moved to, as a value the caller owns.
fn keyed_value(value: Value) -> Result<Value, Fault> {
    let receiver = crate::instance::receiver(value, &KEYED_CLASS, CURRENT)?;
    let slot = keyed_slot(receiver, KEYED_INDEX, CURRENT)?;
    let values = store::borrow(receiver, KEYED_VALUES, &KEYED_CLASS, CURRENT)?;
    values
        .value_at(slot)
        .map(owned)
        .ok_or_else(|| Fault::fatal(format!("{KEYED_NAME}::{CURRENT} found no value at slot {slot}")))
}

nvs_runtime::nvs_helper! {
    /// `KeyedIterator<K, V>::advance(): bool` for a `Core\ObjectMap`'s keyed cursor.
    fn nvs_core_keyed_cursor_advance(_ctx, args: [1]) {
        let stepped = keyed_stepped(args[0]);
        consume(args[0]);
        stepped
    }
}

nvs_runtime::nvs_helper! {
    /// `KeyedIterator<K, V>::key(): K` for a `Core\ObjectMap`'s keyed cursor.
    fn nvs_core_keyed_cursor_key(_ctx, args: [1]) {
        let key = keyed_key(args[0]);
        consume(args[0]);
        key
    }
}

nvs_runtime::nvs_helper! {
    /// `KeyedIterator<K, V>::current(): V` for a `Core\ObjectMap`'s keyed cursor.
    fn nvs_core_keyed_cursor_current(_ctx, args: [1]) {
        let read = keyed_value(args[0]);
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

    /// The array a slot of `object` holds, by its address.
    fn array_in(object: Value, index: usize) -> *mut u8 {
        let receiver = object.obj_ptr().expect("an instance");
        crate::instance::slot(receiver, index)
            .array_ptr()
            .expect("an array slot")
            .cast()
    }

    /// Runs `member` over `receiver`, which it borrows, and `rest`.
    fn on(receiver: Value, member: nvs_runtime::NvsFn, rest: &[Value]) -> Value {
        let mut ctx = Ctx::buffered();
        let mut args = vec![receiver];
        args.extend_from_slice(rest);
        call(member, &mut ctx, &args).expect("the member does not throw")
    }

    /// Calls an `iterate()` the way a `foreach` does: it consumes the
    /// receiver, so the caller's own reference is retained first.
    fn iterate(receiver: Value, member: nvs_runtime::NvsFn) -> Value {
        #[expect(
            unsafe_code,
            reason = "`iterate()` consumes one reference, and this frame keeps its own"
        )]
        unsafe {
            receiver.retain();
        }
        on(receiver, member, &[])
    }

    /// Every element `cursor` yields, rendered, and the cursor released.
    fn drained(cursor: Value) -> Vec<String> {
        let mut ctx = Ctx::buffered();
        let mut seen = Vec::new();
        loop {
            #[expect(unsafe_code, reason = "each call consumes a reference")]
            unsafe {
                cursor.retain();
            }
            let more = call(nvs_core_cursor_advance, &mut ctx, &[cursor]).expect("advance");
            if more.as_bool() != Some(true) {
                break;
            }
            #[expect(unsafe_code, reason = "each call consumes a reference")]
            unsafe {
                cursor.retain();
            }
            let value = call(nvs_core_cursor_current, &mut ctx, &[cursor]).expect("current");
            seen.push(
                value
                    .as_text()
                    .map_or_else(|| format!("{:?}", value.as_int()), ToOwned::to_owned),
            );
            #[expect(
                unsafe_code,
                reason = "`current` handed back a reference this frame owns"
            )]
            unsafe {
                value.release();
            }
        }
        #[expect(unsafe_code, reason = "the loop's own reference")]
        unsafe {
            cursor.release();
        }
        seen
    }

    /// A `string` map's cursor holds the map's own value store, so a loop
    /// starts with no copy. A `set` during the loop gives the map another
    /// store, and the cursor still walks the first one, unchanged.
    #[test]
    fn a_cursor_shares_its_collections_store_until_a_write_separates_it() {
        let mut ctx = Ctx::buffered();
        let map = call(crate::objmap::nvs_core_object_map_new_string, &mut ctx, &[])
            .expect("a fresh map does not throw");
        let keys = [Value::text("a"), Value::text("b"), Value::text("c")];
        on(
            map,
            crate::objmap::nvs_core_object_map_set,
            &[keys[0], Value::int(1)],
        );
        on(
            map,
            crate::objmap::nvs_core_object_map_set,
            &[keys[1], Value::int(2)],
        );
        let cursor = iterate(map, crate::objmap::nvs_core_object_map_iterate);
        let store = array_in(map, crate::objmap::VALUES);
        assert_eq!(
            array_in(cursor, ITEMS),
            store,
            "the cursor and the map hold the same array"
        );

        on(
            map,
            crate::objmap::nvs_core_object_map_set,
            &[keys[2], Value::int(3)],
        );
        assert_ne!(
            array_in(map, crate::objmap::VALUES),
            store,
            "the write gave the map an array of its own"
        );
        assert_eq!(array_in(cursor, ITEMS), store, "the cursor keeps the first");
        assert_eq!(drained(cursor), vec!["a", "b"]);
        assert_eq!(
            drained(iterate(map, crate::objmap::nvs_core_object_map_iterate)),
            vec!["a", "b", "c"],
            "the next loop sees the write"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the constructor's reference and each key's"
        )]
        unsafe {
            map.release();
            for key in keys {
                key.release();
            }
        }
    }

    /// Every `(key, value)` pair a keyed cursor gives, rendered, and the cursor
    /// released. Driven as a keyed `foreach` drives it: `advance`, `key`, then
    /// `current`.
    fn drained_pairs(cursor: Value) -> Vec<String> {
        let render = |value: Value| {
            value
                .as_text()
                .map_or_else(|| format!("{:?}", value.as_int()), ToOwned::to_owned)
        };
        let mut ctx = Ctx::buffered();
        let mut seen = Vec::new();
        loop {
            #[expect(unsafe_code, reason = "each call consumes a reference")]
            unsafe {
                cursor.retain();
            }
            let more = call(nvs_core_keyed_cursor_advance, &mut ctx, &[cursor]).expect("advance");
            if more.as_bool() != Some(true) {
                break;
            }
            let mut pair = Vec::new();
            for member in [nvs_core_keyed_cursor_key, nvs_core_keyed_cursor_current] {
                #[expect(unsafe_code, reason = "each call consumes a reference")]
                unsafe {
                    cursor.retain();
                }
                let value = call(member, &mut ctx, &[cursor]).expect("key or current");
                pair.push(render(value));
                #[expect(unsafe_code, reason = "the member gave back a reference this frame owns")]
                unsafe {
                    value.release();
                }
            }
            seen.push(pair.join("="));
        }
        #[expect(unsafe_code, reason = "the loop's own reference")]
        unsafe {
            cursor.release();
        }
        seen
    }

    /// Releases the values this frame owns.
    fn release_all(values: &[Value]) {
        for value in values {
            #[expect(unsafe_code, reason = "this frame owns one reference on each")]
            unsafe {
                value.release();
            }
        }
    }

    /// A keyed cursor gives each key with its own value, for a map with a key
    /// store and for a map whose keys are its store's keys. A removed entry
    /// leaves a hole, and the pairs after it still match.
    #[test]
    fn a_keyed_cursor_pairs_each_key_with_its_value() {
        let mut ctx = Ctx::buffered();
        let keys = [Value::text("a"), Value::text("b"), Value::text("c")];
        for constructor in [
            crate::objmap::nvs_core_object_map_new as nvs_runtime::NvsFn,
            crate::objmap::nvs_core_object_map_new_string,
        ] {
            let map = call(constructor, &mut ctx, &[]).expect("a fresh map does not throw");
            for (key, n) in keys.iter().zip([1, 2, 3]) {
                on(
                    map,
                    crate::objmap::nvs_core_object_map_set,
                    &[*key, Value::int(n)],
                );
            }
            on(map, crate::objmap::nvs_core_object_map_remove, &[keys[0]]);
            on(
                map,
                crate::objmap::nvs_core_object_map_set,
                &[keys[0], Value::int(4)],
            );
            let cursor = iterate(map, crate::objmap::nvs_core_object_map_iterate_keyed);
            assert_eq!(drained_pairs(cursor), vec!["b=Some(2)", "c=Some(3)", "a=Some(4)"]);
            release_all(&[map]);
        }

        let map = call(crate::objmap::nvs_core_object_map_new_int, &mut ctx, &[])
            .expect("a fresh map does not throw");
        on(
            map,
            crate::objmap::nvs_core_object_map_set,
            &[Value::int(10), keys[1]],
        );
        let cursor = iterate(map, crate::objmap::nvs_core_object_map_iterate_keyed);
        assert_eq!(drained_pairs(cursor), vec!["Some(10)=b"]);
        release_all(&[map]);
        release_all(&keys);
    }

    /// A keyed cursor holds the map's own value store, so a loop starts with no
    /// copy. A write during the loop gives the map another store, and the
    /// cursor still walks the first one, with the values it had.
    #[test]
    fn a_keyed_cursor_shares_the_maps_store() {
        let mut ctx = Ctx::buffered();
        let map = call(crate::objmap::nvs_core_object_map_new_string, &mut ctx, &[])
            .expect("a fresh map does not throw");
        let keys = [Value::text("a"), Value::text("b")];
        on(
            map,
            crate::objmap::nvs_core_object_map_set,
            &[keys[0], Value::int(1)],
        );
        let cursor = iterate(map, crate::objmap::nvs_core_object_map_iterate_keyed);
        let store = array_in(map, crate::objmap::VALUES);
        assert_eq!(
            array_in(cursor, KEYED_VALUES),
            store,
            "the cursor and the map hold the same array"
        );

        on(
            map,
            crate::objmap::nvs_core_object_map_set,
            &[keys[0], Value::int(9)],
        );
        on(
            map,
            crate::objmap::nvs_core_object_map_set,
            &[keys[1], Value::int(2)],
        );
        assert_ne!(array_in(map, crate::objmap::VALUES), store);
        assert_eq!(array_in(cursor, KEYED_VALUES), store, "the cursor keeps the first");
        assert_eq!(drained_pairs(cursor), vec!["a=Some(1)"]);
        assert_eq!(
            drained_pairs(iterate(map, crate::objmap::nvs_core_object_map_iterate_keyed)),
            vec!["a=Some(9)", "b=Some(2)"],
            "the next loop sees both writes"
        );
        release_all(&[map]);
        release_all(&keys);
    }

    /// A heap's cursor walks a sorted list that is not the heap's store and
    /// that nothing else holds.
    #[test]
    fn a_heap_cursor_walks_a_sorted_copy_of_its_own() {
        let mut ctx = Ctx::buffered();
        let heap = call(crate::heap::nvs_core_heap_new, &mut ctx, &[Value::null()])
            .expect("a fresh heap does not throw");
        for n in [3, 1, 2] {
            on(heap, crate::heap::nvs_core_heap_push, &[Value::int(n)]);
        }
        let cursor = iterate(heap, crate::heap::nvs_core_heap_iterate);
        let walked = array_in(cursor, ITEMS);
        assert_ne!(walked, array_in(heap, crate::heap::ENTRIES));
        let receiver = cursor.obj_ptr().expect("an instance");
        let items = store::borrow(receiver, ITEMS, &CLASS, "test").expect("an array");
        assert_eq!(items.refcount(), 1, "nothing else holds the sorted list");
        assert_eq!(drained(cursor), vec!["Some(1)", "Some(2)", "Some(3)"]);

        #[expect(unsafe_code, reason = "this frame owns the constructor's reference")]
        unsafe {
            heap.release();
        }
    }
}
