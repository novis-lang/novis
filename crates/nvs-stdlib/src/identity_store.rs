//! The identity-keyed store behind `docs/spec/01-core-library.md` § 9's
//! `Core\ObjectMap` and `Core\ObjectSet`: an ordinary Novis array, used as the
//! hash table it already is.
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
/// `clear` of every § 9 collection.
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
