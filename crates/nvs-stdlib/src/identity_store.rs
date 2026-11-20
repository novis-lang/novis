//! The identity-keyed store behind `Core\ObjectMap` and `Core\ObjectSet`: an
//! ordinary Novis array, used as the hash table it already is.
//!
//! # Decision: the store is an `NvsArray` keyed by the identity hash
//!
//! A `Core` instance's slots hold values Novis can already hold
//! ([`crate::instance`]), so a collection's state cannot be a native
//! `HashMap` — an instance has no destructor to free one with, and a side
//! table keyed by the object's address would grow with every collection ever
//! constructed, which is the leak [AGENTS.md](/AGENTS.md)'s memory
//! rule names outright. So the slot holds an [`NvsArray`], whose keys are
//! byte strings, and this module chooses those keys.
//!
//! **A member is stored under `"<16 hex digits of its identity hash>#<n>"`,**
//! where `n` is the smallest ordinal at which the chain of equal-hash keys
//! holds nothing identical to it. [`nvs_runtime::value_hash`] is the hash and
//! [`nvs_runtime::value_identical`] the comparison, so a collection agrees
//! with `==` and with `Core\Arr::contains` by construction rather than by a
//! second set of divergence decisions (`nvs_runtime::identity` owns both).
//! The ordinal is what makes a collision *correct* rather than merely
//! unlikely: two distinct values may hash alike, and the chain gives each its
//! own key.
//!
//! **The chain is dense** — an ordinal is never skipped — which is what lets
//! [`locate`] stop at the first absent key. A removal therefore has to restore
//! that by renaming the chain's last entry to the removed key rather than
//! leaving a tombstone; [`vacate`] is the one place that happens. The rename
//! keeps the entry where it is in the store's order
//! ([`NvsArray::rename`]), so a removal never moves another member.
//!
//! Flat rather than a bucket array per hash because a nested array would have
//! to be taken out of its parent, mutated and put back on every write; the
//! ordinal buys the same O(1) average lookup with one level of allocation and
//! no refcount dance. It also makes `count` free: the store's own entry count
//! *is* the collection's, so there is no second slot to keep in step.
//!
//! **What it spends:** one array allocation per store, plus one entry — a
//! 16-byte [`Value`] and a 17- to 20-byte key — per distinct member, charged
//! to the request that built the collection and released with it. A map holds
//! two stores under one set of keys, so a pair costs two entries.
//!
//! The hash is seeded per core from [`RandomState`], so the key an
//! attacker-chosen member lands on is not predictable across processes — the
//! collision-flooding concern `nvs_runtime::identity`'s `hash_numeric` already
//! reasons about, at the one place a program chooses what goes into the table.
//!
//! # Decision: a `string` or `int` key is stored as it is
//!
//! A collection is built with one [`Keying`], which the checker picks from its
//! first type argument and `new` writes into the instance's `keying` slot. A
//! key type of exactly `string` or exactly `int` needs no chain: every value of
//! it is a key the array already compares the way `==` does. So the store is
//! keyed by the string's own bytes, or by the `int` as an array subscript is
//! (`rule:types/arrays`), and `get`, `has`, `set` and `remove` are one lookup
//! with no identity hash. Every other key type keeps the chain, which is
//! correct for all of them. A `tainted string` or `secret string` key stays on
//! the chain too.
//!
//! A direct map has one store: the values, under the key itself. `keys()` and a
//! `foreach` read each key back from it, as an `int` for an `int` map. A
//! direct set has one store, each member under its own key.
//!
//! The hash of a direct key is the array's own SipHash, seeded per process,
//! so the flooding concern above is met the same way.
//!
//! **What it spends:** one more slot per collection, an `int`. A direct map
//! drops its key store, so a pair costs one entry, not two, and a `string`
//! key shares the caller's string instead of a 17- to 20-byte chain key.
//!
//! # Decision: an object key is its identity
//!
//! A key type whose every value is an object (the checker's
//! `is_always_an_object`: a class, an interface, `object`, a shape, a
//! callable) is [`Keying::Object`]. An object is identical only to itself
//! (`rule:expressions/equality-semantics`), never moves, and its identity is
//! its address, so the store key is [`IdentityKey`]: `@` and the pointer's
//! sixteen hex digits, written on the stack. The `@` keeps it from reading as
//! an `int` subscript. `get`, `has`, `set` and `remove` are one lookup with no
//! identity hash and no chain walk.
//!
//! **The collection's reference keeps the key unique.** The store holds the
//! object while its entry exists, so no other object can take that address
//! until the entry is removed, and a removal deletes the key. The key never
//! leaves the store: `keys()`, `values()`, a `foreach` and every error read the
//! stored values, and the record walk shows a store slot by its values alone
//! (`nvs_runtime::ClassDesc::keyed_stores`). No address reaches a program.
//!
//! **A map keeps two stores,** keys and values, under the identity key, because
//! the key bytes give no object back. `get` and `has` read the value store
//! alone. A set keeps one store, each member under its own key.
//!
//! **A graph copy re-keys.** A live copy across a boundary may make new objects
//! of what a collection holds, which leaves the copied stores keyed by the old
//! addresses. The class's descriptor names a hook the copy calls once the
//! collection is complete (`nvs_runtime::ClassDesc::keyed_stores`), and it
//! rebuilds the stores under the new objects' keys ([`rebuild_members`],
//! [`rebuild_pairs`]). Without it a new object at a freed old address would be
//! found. The bytes carrier refuses both classes, so it needs nothing.
//!
//! **What it spends:** a map's pair costs two entries, as the chain's does,
//! with a 17-byte key in place of a 17- to 20-byte one. A live copy of an
//! identity-keyed collection rebuilds its stores once: one array and one
//! entry per member more, for the length of the copy.
//!
//! # Decision: a key whose tag is not the keying's
//!
//! An `as` or `is` checks a collection's class and never its type arguments,
//! so a map built `<string, int>` can be passed a `5` through an erased view.
//! Each keyed operation compares the key's tag with its keying first, one byte
//! compare ([`Keying::admits`]). A read of such a key answers absent, because
//! no stored key has that tag. A write rebuilds the store under chain keys
//! ([`rechain_members`], [`rechain_pairs`]), writes the chain into the
//! `keying` slot, and goes on as a chain collection. So every result is the one
//! the chain gives. Declined: throwing on the mismatch, which changes what a
//! working program does.

use std::cell::OnceCell;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::Write as _;
use std::mem::ManuallyDrop;

use nvs_runtime::{Fault, NvsArray, NvsStr, ObjHeader, Tag, Value};

use crate::registry::CoreClass;

thread_local! {
    /// This core's hash seed, drawn once. Per core rather than shared because
    /// the runtime is thread-per-core and a collection never crosses that
    /// boundary, and drawn at all for the reason this module's docs give.
    static SEED: OnceCell<RandomState> = const { OnceCell::new() };
}

/// `value`'s identity hash under this core's seed.
fn identity_hash(value: Value) -> u64 {
    SEED.with(|seed| {
        let mut hasher = seed.get_or_init(RandomState::new).build_hasher();
        nvs_runtime::value_hash(value, &mut hasher);
        hasher.finish()
    })
}

/// A chain key, written on the stack. Sixteen hex digits, `#` and a `usize`
/// ordinal are at most 37 bytes, so looking a key up allocates nothing — a
/// `get`, `has` or `set` over a present key costs no heap traffic at all.
pub(crate) struct ChainKey {
    bytes: [u8; 40],
    len: usize,
}

impl std::ops::Deref for ChainKey {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

/// The key at ordinal `n` of `hash`'s chain — see this module's docs.
fn chain_key(hash: u64, n: usize) -> ChainKey {
    let mut bytes = [0u8; 40];
    let mut rest = &mut bytes[..];
    write!(rest, "{hash:016x}#{n}").expect("a chain key fits in 40 bytes");
    let len = 40 - rest.len();
    ChainKey { bytes, len }
}

/// Where `value` lives in `store`, as `(key, present)`.
///
/// `present` is whether the key already holds a value identical to `value`;
/// where it is `false` the key is the first free one in the chain, which is
/// exactly where a write goes. Walking to the first absent key is what the
/// chain's density buys.
pub(crate) fn locate(store: &NvsArray, value: Value) -> (ChainKey, bool) {
    let hash = identity_hash(value);
    let mut n = 0usize;
    loop {
        let key = chain_key(hash, n);
        match store.get(&key) {
            None => return (key, false),
            Some(found) if nvs_runtime::value_identical(found, value) => return (key, true),
            Some(_) => n += 1,
        }
    }
}

/// Removes `key` from `store`, then renames the chain's last entry to the
/// removed key so the chain stays dense — see this module's docs for why it
/// must. The renamed entry keeps its own position, so the collection's
/// insertion order does not change.
///
/// `key` must be one [`locate`] answered `true` for; a key that is not there
/// leaves the store untouched.
pub(crate) fn vacate(store: &mut NvsArray, key: &[u8]) {
    let Some((hash, n)) = split_key(key) else {
        return;
    };
    if !store.has_key(key) {
        return;
    }
    let mut last = n;
    while store.get(&chain_key(hash, last + 1)).is_some() {
        last += 1;
    }
    store.unset(key);
    if last != n {
        let renamed = store.rename(&chain_key(hash, last), NvsStr::new(key));
        debug_assert!(renamed, "the chain's last key is live and the hole is free");
    }
}

/// A chain key back as `(hash, ordinal)`, or `None` if it is not one this
/// module wrote.
fn split_key(key: &[u8]) -> Option<(u64, usize)> {
    let text = std::str::from_utf8(key).ok()?;
    let (hash, ordinal) = text.split_once('#')?;
    Some((
        u64::from_str_radix(hash, 16).ok()?,
        ordinal.parse::<usize>().ok()?,
    ))
}

/// Slot `index` of `receiver` as a store, borrowed — the read half.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member if the slot does not hold an array,
/// which only a bug in this crate can produce: a store slot is written by its
/// class's constructor and by [`replace`] and by nothing else.
pub(crate) fn borrow(
    receiver: *mut ObjHeader,
    index: usize,
    class: &CoreClass,
    member: &str,
) -> Result<ManuallyDrop<NvsArray>, Fault> {
    let held = crate::instance::slot(receiver, index);
    let ptr = held.array_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{}::{member} expected {:?} in its `{}` slot, got tag {}",
            class.name,
            Tag::Array,
            class.slots[index],
            held.tag_byte()
        ))
    })?;
    Ok(crate::arr::borrowed(ptr))
}

/// Runs `edit` over slot `index`'s store, writing the result back.
///
/// The common case is that the object holds the store's only reference, and
/// then the edit happens in place and the slot's pointer never moves. A
/// `clone`d collection (`rule:classes/two-copy-depths`)
/// is the other case: two objects share one store, so [`NvsArray::set`]
/// separates a copy, and the slot has to take over that copy or the write
/// would land on an allocation this object no longer reads. The retain before
/// the owning handle is what makes the separation leave the *other* holder the
/// original rather than dropping it.
///
/// # Errors
///
/// [`borrow`]'s, unchanged.
pub(crate) fn edit<R>(
    receiver: *mut ObjHeader,
    index: usize,
    class: &CoreClass,
    member: &str,
    edit: impl FnOnce(&mut NvsArray) -> R,
) -> Result<R, Fault> {
    let mut shared = borrow(receiver, index, class, member)?;
    if shared.refcount() == 1 {
        return Ok(edit(&mut shared));
    }
    let held = crate::instance::slot(receiver, index);
    #[expect(
        unsafe_code,
        reason = "the slot owns a reference to a live allocation, and this \
                  retain is the one the owning handle below gives up"
    )]
    unsafe {
        held.retain();
    }
    let ptr = held.array_ptr().expect("`borrow` just read an array here");
    #[expect(
        unsafe_code,
        reason = "the retain above is exactly the reference this handle owns"
    )]
    let mut owned = unsafe { NvsArray::from_raw(ptr) };
    let out = edit(&mut owned);
    crate::instance::set_slot(receiver, index, Value::array(owned));
    Ok(out)
}

/// Replaces slot `index` with an empty store, releasing what it held — the
/// `clear` of `Core\ObjectMap` and `Core\ObjectSet`.
///
/// A fresh store rather than an entry-by-entry removal: the slot write
/// releases the old array, which releases every value it held, so the refcount
/// traffic is identical and there is no chain to keep dense while it happens.
pub(crate) fn replace(receiver: *mut ObjHeader, index: usize) {
    crate::instance::set_slot(receiver, index, Value::array(NvsArray::new()));
}

/// Every value a store holds, in the store's own order, as a fresh Novis list.
///
/// Each entry is retained: the store outlives the call, so the list needs a
/// reference of its own — the rule [`crate::arr`] applies everywhere it copies
/// an entry out of a borrowed subject. This is what a `keys()`/`values()` row
/// answers with and what a `foreach` walks ([`crate::cursor`]), which is why
/// it lives here rather than on either collection.
pub(crate) fn listed(store: &NvsArray) -> NvsArray {
    let mut out = NvsArray::new();
    let mut from = 0usize;
    while let Some(slot) = store.next_slot(from) {
        if let Some(value) = store.value_at(slot) {
            #[expect(
                unsafe_code,
                reason = "the entry is owned by the store, which outlives this \
                          call, so the copy stored here needs a reference of \
                          its own"
            )]
            unsafe {
                value.retain();
            }
            out.append(value);
        }
        from = slot + 1;
    }
    out
}

/// How a collection keys its store: chosen once, when `new` builds it, and
/// kept in the instance's `keying` slot.
///
/// The checker picks it from the first type argument
/// (`nvs_types::expr::calls`), and `nvs-ir` lowers `new` to the constructor
/// symbol of that keying ([`crate::registry::constructor_symbol`]). A key
/// whose tag is not the keying's turns the collection into a [`Self::Chain`]
/// one — this module's docs § *Decision: a key whose tag is not the keying's*.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keying {
    /// Every key type: the store is keyed by the chain key this module's docs
    /// describe.
    Chain,
    /// Exactly `string`: the store is keyed by the string's own bytes.
    Str,
    /// Exactly `int`: the store is keyed by the `int`, normalised as an array
    /// subscript is (`rule:types/arrays`).
    Int,
    /// A type whose every value is an object: the store is keyed by the
    /// object's identity, [`IdentityKey`].
    Object,
}

impl Keying {
    /// The value the `keying` slot holds for this keying.
    pub(crate) const fn value(self) -> Value {
        Value::int(match self {
            Self::Chain => 0,
            Self::Str => 1,
            Self::Int => 2,
            Self::Object => 3,
        })
    }

    /// Whether a map in this keying holds its keys in a store of their own:
    /// the chain and the identity key, whose key bytes give no key back.
    pub(crate) const fn keeps_keys(self) -> bool {
        matches!(self, Self::Chain | Self::Object)
    }

    /// The keying a `keying` slot's value names. Anything else is the chain,
    /// which is correct for every key.
    pub(crate) fn of(value: Value) -> Self {
        match value.as_int() {
            Some(1) => Self::Str,
            Some(2) => Self::Int,
            Some(3) => Self::Object,
            _ => Self::Chain,
        }
    }

    /// Whether `key` can be stored under this keying: the chain takes every
    /// value, and a direct keying only a value of its own tag.
    ///
    /// The raw tag byte, so an enum case backed by an `int` is not an `int`
    /// key here.
    pub(crate) fn admits(self, key: Value) -> bool {
        match self {
            Self::Chain => true,
            Self::Str => key.tag_byte() == Tag::Str as u8,
            Self::Int => key.tag_byte() == Tag::Int as u8,
            Self::Object => key.tag_byte() == Tag::Object as u8,
        }
    }
}

/// An object's identity key, written on the stack: `@` and the sixteen hex
/// digits of its pointer. The `@` keeps the array from reading it as an `int`
/// subscript. The key is unique while the entry exists, because the store
/// holds a reference to the object, and it never leaves the store.
pub(crate) struct IdentityKey([u8; 17]);

impl std::ops::Deref for IdentityKey {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

/// `object`'s identity key — see [`IdentityKey`].
fn identity_key(object: Value) -> IdentityKey {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut bytes = [b'@'; 17];
    let bits = object.bits();
    for (at, byte) in bytes[1..].iter_mut().enumerate() {
        let shift = 60 - 4 * at;
        *byte = HEX[usize::try_from((bits >> shift) & 0xf).unwrap_or_default()];
    }
    IdentityKey(bytes)
}

/// Where a key lives in a store under its collection's keying.
pub(crate) enum Place {
    /// A chain key, written on the stack.
    Chain(ChainKey),
    /// A `string` key, borrowed from the caller.
    Str(Value),
    /// An `int` key.
    Int(i64),
    /// An object's identity key, written on the stack.
    Object(IdentityKey),
}

impl Place {
    /// The key as the bytes the store holds it under. An `int` is written in
    /// decimal on the stack, as an array subscript is.
    fn with_bytes<R>(&self, read: impl FnOnce(&[u8]) -> R) -> R {
        match self {
            Self::Chain(key) => read(key),
            Self::Object(key) => read(key),
            Self::Str(key) => read(key.as_str_bytes().unwrap_or_default()),
            Self::Int(index) => {
                let mut bytes = [0u8; 24];
                let mut rest = &mut bytes[..];
                write!(rest, "{index}").expect("an `i64` fits in 24 bytes");
                let len = 24 - rest.len();
                read(&bytes[..len])
            }
        }
    }
}

/// Where `key` lives in `store` under `keying`, as `(place, present)`.
///
/// `key` must be one `keying` [admits](Keying::admits). The chain walks its
/// chain as [`locate`] does, and a direct keying is one lookup.
pub(crate) fn place(store: &NvsArray, keying: Keying, key: Value) -> (Place, bool) {
    debug_assert!(keying.admits(key), "the caller checked the key's tag");
    match keying {
        Keying::Chain => {
            let (chain, present) = locate(store, key);
            (Place::Chain(chain), present)
        }
        Keying::Str => {
            let present = key.as_str_bytes().is_some_and(|bytes| store.has_key(bytes));
            (Place::Str(key), present)
        }
        Keying::Int => {
            let index = key.as_int().unwrap_or_default();
            (Place::Int(index), store.get_index(index).is_some())
        }
        Keying::Object => {
            let identity = identity_key(key);
            let present = store.has_key(&identity);
            (Place::Object(identity), present)
        }
    }
}

/// The value a direct keying stores under `key`, borrowed, or `None` when the
/// key is absent or its tag is not the keying's. A chain store answers `None`:
/// its key comes from [`locate`].
pub(crate) fn find(store: &NvsArray, keying: Keying, key: Value) -> Option<Value> {
    if !keying.admits(key) {
        return None;
    }
    match keying {
        Keying::Chain => None,
        Keying::Str => store.get(key.as_str_bytes()?),
        Keying::Int => store.get_index(key.as_int()?),
        Keying::Object => store.get(&identity_key(key)),
    }
}

/// Whether `store` holds `key` under `keying`. A key of another tag is never
/// there, because no stored key has that tag.
pub(crate) fn contains(store: &NvsArray, keying: Keying, key: Value) -> bool {
    keying.admits(key) && place(store, keying, key).1
}

/// Writes `value` at `place`, taking over its reference. `present` is what
/// [`place`] answered, so an existing entry is overwritten where it is and
/// builds no key.
///
/// A `string` key is stored as the caller's own string, with a reference of
/// its own, so a new key copies no bytes.
pub(crate) fn put(store: &mut NvsArray, place: &Place, present: bool, value: Value) {
    let value = if present {
        match place.with_bytes(|bytes| store.overwrite(bytes, value)) {
            Ok(()) => return,
            Err(value) => value,
        }
    } else {
        value
    };
    match place {
        Place::Chain(chain) => store.set(NvsStr::new(chain), value),
        Place::Object(identity) => store.set(NvsStr::new(identity), value),
        Place::Str(key) => {
            let Some(ptr) = key.str_ptr() else { return };
            #[expect(
                unsafe_code,
                reason = "the key is borrowed from the caller's frame, so the \
                          copy the store keeps needs a reference of its own, \
                          and that reference is the one the handle below owns"
            )]
            let shared = unsafe {
                key.retain();
                NvsStr::from_raw(ptr)
            };
            store.set(shared, value);
        }
        Place::Int(index) => store.set_index(*index, value),
    }
}

/// Removes the entry at `place`, which [`place`] answered present for. A
/// chain store keeps its chain dense, as [`vacate`] does.
pub(crate) fn remove(store: &mut NvsArray, place: &Place) {
    match place {
        Place::Chain(chain) => vacate(store, chain),
        Place::Str(_) | Place::Int(_) | Place::Object(_) => {
            place.with_bytes(|bytes| store.unset(bytes));
        }
    }
}

/// The key at `slot` of a direct store, as a value the caller owns: a
/// `string` for [`Keying::Str`] and an `int` for [`Keying::Int`].
fn key_value(store: &NvsArray, slot: usize, keying: Keying) -> Option<Value> {
    match keying {
        Keying::Chain | Keying::Object => None,
        #[expect(
            clippy::disallowed_methods,
            reason = "a `string` store's every key was written from a `string` \
                      value, which is UTF-8 already"
        )]
        Keying::Str => store.key_at(slot).map(Value::str),
        Keying::Int => match store.slot_key(slot)? {
            nvs_runtime::SlotKey::Index(index) => Some(Value::int(index)),
            nvs_runtime::SlotKey::Str(key) => std::str::from_utf8(key.as_bytes())
                .ok()?
                .parse::<i64>()
                .ok()
                .map(Value::int),
        },
    }
}

/// Every key of a direct store, in the store's order, as a fresh Novis list:
/// what `keys()` and a `foreach` over a `string` or `int` map read.
pub(crate) fn keys_listed(store: &NvsArray, keying: Keying) -> NvsArray {
    let mut out = NvsArray::new();
    let mut from = 0usize;
    while let Some(slot) = store.next_slot(from) {
        if let Some(key) = key_value(store, slot, keying) {
            out.append(key);
        }
        from = slot + 1;
    }
    out
}

/// A set's direct store rebuilt under chain keys, in the same order. Each
/// member is the value under its own key, so the members are read from the
/// values.
pub(crate) fn rechain_members(store: &NvsArray) -> NvsArray {
    rebuild_members(store, Keying::Chain)
}

/// A set's store rebuilt under `keying`, which is the chain or the identity
/// key, in the same order. Each member is the value under its own key, so the
/// members are read from the values. `keying` admits every member: the chain
/// admits all, and an identity-keyed store holds only objects.
pub(crate) fn rebuild_members(store: &NvsArray, keying: Keying) -> NvsArray {
    let mut out = NvsArray::new();
    let mut from = 0usize;
    while let Some(slot) = store.next_slot(from) {
        from = slot + 1;
        let Some(member) = store.value_at(slot) else {
            continue;
        };
        let (place, present) = place(&out, keying, member);
        if present {
            continue;
        }
        #[expect(
            unsafe_code,
            reason = "the member is owned by the old store, so the copy the new \
                      store keeps needs a reference of its own"
        )]
        unsafe {
            member.retain();
        }
        put(&mut out, &place, false, member);
    }
    out
}

/// A map's two stores, `(keys, values)`, rebuilt under `keying`, which is the
/// chain or the identity key, in the same order. Each key is read from the key
/// store and its value from the value store under the same store key.
pub(crate) fn rebuild_pairs(
    keys: &NvsArray,
    values: &NvsArray,
    keying: Keying,
) -> (NvsArray, NvsArray) {
    let mut out_keys = NvsArray::new();
    let mut out_values = NvsArray::new();
    let mut from = 0usize;
    while let Some(slot) = keys.next_slot(from) {
        from = slot + 1;
        let Some(key) = keys.value_at(slot) else {
            continue;
        };
        let value = match keys.slot_key(slot) {
            Some(nvs_runtime::SlotKey::Str(name)) => values.get(name.as_bytes()),
            Some(nvs_runtime::SlotKey::Index(index)) => values.get_index(index),
            None => None,
        };
        let Some(value) = value else { continue };
        let (place, present) = place(&out_keys, keying, key);
        if present {
            continue;
        }
        #[expect(
            unsafe_code,
            reason = "the key and the value are owned by the old stores, so the \
                      copies the new stores keep need references of their own"
        )]
        unsafe {
            key.retain();
            value.retain();
        }
        put(&mut out_keys, &place, false, key);
        put(&mut out_values, &place, false, value);
    }
    (out_keys, out_values)
}

/// A map's direct store rebuilt as the chain's two stores, `(keys, values)`,
/// in the same order.
pub(crate) fn rechain_pairs(store: &NvsArray, keying: Keying) -> (NvsArray, NvsArray) {
    let mut keys = NvsArray::new();
    let mut values = NvsArray::new();
    let mut from = 0usize;
    while let Some(slot) = store.next_slot(from) {
        from = slot + 1;
        let (Some(key), Some(value)) = (key_value(store, slot, keying), store.value_at(slot))
        else {
            continue;
        };
        let (chain, _) = locate(&keys, key);
        #[expect(
            unsafe_code,
            reason = "the value is owned by the old store, so the copy the new \
                      store keeps needs a reference of its own"
        )]
        unsafe {
            value.retain();
        }
        keys.set(NvsStr::new(&chain), key);
        values.set(NvsStr::new(&chain), value);
    }
    (keys, values)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A chain key survives the round trip [`vacate`] depends on.
    #[test]
    fn a_chain_key_reads_back_as_its_hash_and_ordinal() {
        assert_eq!(
            split_key(&chain_key(0xdead_beef, 3)),
            Some((0xdead_beef, 3))
        );
        assert_eq!(split_key(b"not-a-chain-key"), None);
    }

    /// A removal from the middle of a chain gives the last entry the hole's
    /// key, so [`locate`]'s stop-at-the-first-gap walk still finds what is
    /// left — the invariant this module's docs make load-bearing — and every
    /// survivor, the renamed one included, stays in insertion order.
    #[test]
    fn vacating_the_middle_of_a_chain_keeps_it_dense() {
        let mut store = NvsArray::new();
        for n in 0..3usize {
            store.set(
                NvsStr::new(&chain_key(7, n)),
                Value::int(i64::try_from(n).expect("a small index is an `int`")),
            );
            store.set(NvsStr::new(b"other"), Value::int(9));
        }
        store.set(NvsStr::new(&chain_key(8, 0)), Value::int(10));
        vacate(&mut store, &chain_key(7, 1));
        assert_eq!(store.count(), 4);
        assert_eq!(store.get(&chain_key(7, 0)).and_then(Value::as_int), Some(0));
        assert_eq!(store.get(&chain_key(7, 1)).and_then(Value::as_int), Some(2));
        assert!(store.get(&chain_key(7, 2)).is_none());
        let order: Vec<Option<i64>> = store
            .keys()
            .iter()
            .map(|key| store.get(key).and_then(Value::as_int))
            .collect();
        assert_eq!(order, vec![Some(0), Some(9), Some(2), Some(10)]);
    }
}
