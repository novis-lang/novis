//! MWL's array: [ADR 0007](../../../docs/adr/0007-explicit-type-system.md)
//! § 5's insertion-ordered, string-keyed hash, refcounted and copy-on-write.
//!
//! This is what `mwl_ir::ty::Ty::Array` lowers to, and the other half of M4's
//! Stage 1 gate — `Core\Arr`'s whole contract rests on it
//! (`docs/agent/loop-goal.md`).
//!
//! # Decision: the representation is opaque to compiled code
//!
//! [`crate::MwlStr`] and [`crate::MwlObj`] both publish byte offsets so
//! `mwl-codegen` can load a field or a payload byte inline. An array
//! publishes exactly one, [`ARRAY_REFCOUNT_OFFSET`], and nothing else:
//! every operation on it — a read, a write, an append, an iteration step — is
//! an out-of-line call to one of the primitives at the bottom of this file.
//!
//! That is what lets the container itself be ordinary safe Rust rather than a
//! hand-rolled open-addressed table over a flexible-array-member allocation.
//! An entry lookup already costs a hash and a probe, so the call is not on the
//! margin the way an object field load is; buying the whole table's
//! memory-safety for it is [AGENTS.md](../../../AGENTS.md)'s priority 1 and 4
//! bought with a few instructions of priority 3, which the ordering permits.
//! The cost, stated as AGENTS.md requires: **three allocations per array**
//! (the header, the entry vector, the index map) rather than one, and one call
//! per element access. A list-shaped array pays two of the three — see the
//! packed decision below.
//!
//! # Decision: the index map hashes with std's `RandomState`, not a fast hasher
//!
//! Array keys are the single most likely place attacker-controlled bytes
//! become hash inputs — a form field, a JSON object, a query string. PHP's own
//! hashtable produced a real remote-DoS CVE that way, and a fast non-keyed
//! hasher (`FxHash`, the one `mwl-codegen` uses for its compiler-internal
//! tables) is trivially floodable. So this map keeps std's per-process-seeded
//! SipHash: priority 1 over priority 3, the one direction the ordering allows.
//! It also means this module adds no dependency at all.
//!
//! # Decision: a list-shaped array is packed, and hashes nothing
//!
//! An array whose keys are exactly `"0"`…`"n−1"` in insertion order stores a
//! `Vec<Value>` and nothing else — no index map, no key strings. The first
//! operation that breaks that invariant — a `"08"`, a gap, a non-numeric key,
//! an `unset` anywhere but the end — converts it to the hash form above, which
//! is PHP's own arrangement. [`key_at`](MwlArray::key_at) synthesizes the
//! decimal on demand, so `foreach ($a as $v)`, which never asks for a key,
//! never pays for one.
//!
//! **[ADR 0007 § 5](../../../docs/adr/0007-explicit-type-system.md) is
//! unchanged by this.** Every key is still a `string`, `"08"` is still a
//! distinct key from `"8"` (it is what forces the degrade), insertion order is
//! still the iteration order, and `Core\Arr::keys` still answers
//! `array<string>`. This is representation, not semantics.
//!
//! The reason it is not an optimisation to schedule later, measured on this
//! tree against the PHP 8.5.9 oracle on the same machine — and PHP's figures
//! include VM opcode dispatch that compiled MWL does not pay, so the
//! comparison already flatters the interpreter:
//!
//! | | PHP 8.5.9 | this module, unpacked |
//! |---|---|---|
//! | `$a[] = $v` | 23.4 ns | **219.5 ns** |
//! | `$a[$i]` | 30.5 ns | **113.8 ns** |
//! | `$a['name']` | 24.4 ns | 24.1 ns |
//! | `foreach ($a as $v)` | 21.0 ns | 4.9 ns |
//!
//! The 219.5 ns is 37.2 rendering the index to a decimal `String`, 43.1
//! allocating the [`MwlStr`](crate::MwlStr) key, 23.1 hashing and probing, and
//! the rest index-map insert and growth. A `Vec<Value>` push is 1.9 ns and an
//! index 0.34 ns. The assoc and iteration rows are healthy and this changes
//! neither.
//!
//! **The ABI is the part that expires.** [`mwl_array_get`] and
//! [`mwl_array_set`] take a `*const StrHeader`, so compiled code must build a
//! key string before it calls, and a packed form would have to parse the
//! decimal back out — pointless. `mwl_array_get_index`/`mwl_array_set_index`
//! are a compatible addition while nothing depends on the current set, and a
//! versioned break once [ADR 0042](../../../docs/adr/0042-on-disk-artifact-cache-format.md)
//! artifacts and M9's WIT signatures do.
//!
//! What it spends, as [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md)
//! requires: **nothing — it saves.** A list drops two of its three allocations
//! and every key string. It also takes list data out of the SipHash path the
//! decision above exists to justify, which leaves that decision protecting the
//! case it was actually written for: attacker-controlled *names*.
//!
//! # Decision: deletion tombstones, with amortized compaction
//!
//! The entry vector is insertion order with a `None` where a key was
//! removed, exactly PHP's own arrangement. `unset()` is therefore O(1) and
//! leaves every surviving key's position — and so `foreach`'s order —
//! untouched, where an order-preserving vector removal would be O(n) and turn
//! the ordinary "unset in a loop" shape quadratic. The holes are swept when
//! they outnumber the live entries, so the vector stays O(live) amortized.
//!
//! # Decision: a mutation consumes one reference and returns one
//!
//! ADR 0007 § 5 gives arrays copy-on-write **value semantics**, so
//! `$b = $a; $b["k"] = 1;` must not be visible through `$a`. The separation
//! that guarantees it produces a *different allocation*, which the writer must
//! then be holding — and compiled code keeps a local in an SSA register, not
//! in a memory slot a callee could write back through.
//!
//! So every mutating primitive here takes the array by value and gives it
//! back: it consumes one reference to its `array` argument and returns one
//! reference to the array that now holds the change. When the refcount was
//! already `1` that is the same pointer and the same reference, mutated in
//! place with no copy at all — the fast path [ADR 0063](../../../docs/adr/0063-core-api-conventions.md)
//! R3's "nothing mutates" API shape rests on, measured by
//! `a_refcount_one_array_member_mutates_in_place` in `benches/abi-probe`.
//! When it was higher, the entry storage is copied, every key and value in it
//! retained, the caller's reference dropped, and the fresh copy returned.
//!
//! [`MwlArray`] expresses the same protocol safely: its mutators take
//! `&mut self` and re-point the handle, which is why nothing outside this
//! module writes a separation by hand.
//!
//! # Decision: freeing is iterative, and shared with objects
//!
//! An array can hold an object that holds an array, to any depth, so freeing
//! one is the same "the depth of a user's data structure must not decide
//! whether the process survives freeing it" problem [`crate::object`] already
//! solved for a linked list — and solving it separately per kind would only
//! move the recursion to the boundary between them. [`crate::release`] owns
//! the one worklist both kinds drain into.
//!
//! # Known gap: no interned element-type descriptor
//!
//! ADR 0007 § 5 gives an array header a pointer to an interned, immutable
//! descriptor of its element type, so a value arriving through `mixed`,
//! `json_decode` or an isolate boundary can be checked. Nothing constructs an
//! array from any of those routes yet — every array here is built by compiled
//! code the checker already proved well-typed — so the word is not carried,
//! and adding it is a widening of [`ArrayHeader`] rather than a redesign.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::ptr::NonNull;

use crate::string::{MwlStr, StrHeader};
use crate::value::Value;

/// One key/value pair, owning one reference to each.
pub(crate) struct Entry {
    pub(crate) key: MwlStr,
    pub(crate) value: Value,
}

/// One array's storage: PHP's append counter, and whichever of the two shapes
/// the keys used so far allow — see this module's packed decision.
pub(crate) struct Table {
    /// PHP's "highest integer key used so far, plus one", which `$a[]` appends
    /// under. Survives removals, exactly as PHP's does.
    ///
    /// It is carried by *both* shapes because it is the one thing a packed
    /// array cannot re-derive from its own length: `[1,2,3]` with its last
    /// entry unset holds the keys `"0"` and `"1"` and still appends at `3`.
    ///
    /// `None` is PHP's `ZEND_LONG_MIN` marker for "no integer key has ever
    /// been used", which is why `[-5 => 1]` then `$a[] = 2` appends at `-4`
    /// while `[3 => 1]` then `$a[] = 2` appends at `4`: the first integer key
    /// sets the counter outright, and every later one may only raise it. PHP
    /// 8.3 introduced that marker, and PHP 8.5 is the differential oracle.
    next_index: Option<i64>,
    /// How the entries themselves are held.
    shape: Shape,
}

/// The two representations of one array's entries.
///
/// Every operation on a [`Table`] either answers from the packed form directly
/// or converts to the hash form first, so nothing outside this module can tell
/// which one it is holding: [ADR 0007 § 5](../../../docs/adr/0007-explicit-type-system.md)
/// is a statement about keys, not about storage.
enum Shape {
    /// The packed form: the keys are exactly `"0"`…`"n−1"` in order, so they
    /// are not stored at all and there is no index map to hash into.
    /// [`Table::key_at`] renders one on demand.
    Packed(Vec<Value>),
    /// The general form, which every array that is not a list degrades into.
    Hashed(Hashed),
}

/// The ordered hash itself: insertion order in [`Hashed::entries`], key lookup
/// through [`Hashed::index`].
#[derive(Default)]
struct Hashed {
    /// Every entry ever inserted and not since removed, in insertion order,
    /// with a `None` where a removal left a hole — see this module's docs.
    entries: Vec<Option<Entry>>,
    /// Key to position in [`Hashed::entries`]. Holds its own reference to each
    /// key, which is the same allocation the entry holds: a key is stored
    /// twice as a pointer, never twice as bytes.
    index: HashMap<MwlStr, usize>,
    /// How many of [`Hashed::entries`] are `Some` — the array's `count()`.
    live: usize,
}

impl Default for Table {
    /// A fresh array is packed and empty, which is the shape every list is
    /// built by appending into.
    fn default() -> Self {
        Self {
            next_index: None,
            shape: Shape::Packed(Vec::new()),
        }
    }
}

/// The `i64` a canonical decimal key denotes, or `None` if the key is not one.
///
/// Canonical means what PHP means by it: an optional `-`, then either `0` or a
/// digit run with no leading zero, and the whole thing in range. `"08"` and
/// `"-0"` are therefore ordinary string keys that collide with nothing, which
/// is ADR 0007 § 5's "which subscripts collide does not change".
fn integer_key(bytes: &[u8]) -> Option<i64> {
    let (negative, digits) = match bytes.split_first() {
        Some((b'-', rest)) => (true, rest),
        _ => (false, bytes),
    };
    match digits {
        [] => return None,
        [b'0'] if !negative => {}
        [b'0', ..] => return None,
        _ => {}
    }
    if !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    text.parse::<i64>().ok()
}

/// The position a canonical decimal key names in the packed form, whatever the
/// array's length — `None` for every key that is not one, a negative one
/// included.
fn packed_index(key: &[u8]) -> Option<usize> {
    usize::try_from(integer_key(key)?).ok()
}

impl Table {
    /// Records that `key` was used, so a later `$a[]` append does not collide
    /// with it — PHP 8.3's rule, which counts a negative key too.
    fn note_key(&mut self, key: &[u8]) {
        if let Some(number) = integer_key(key) {
            let after = number.saturating_add(1);
            self.next_index = Some(match self.next_index {
                None => after,
                Some(current) => current.max(after),
            });
        }
    }

    /// How many entries the array holds — the array's `count()`.
    fn len(&self) -> usize {
        match &self.shape {
            Shape::Packed(values) => values.len(),
            Shape::Hashed(hashed) => hashed.live,
        }
    }

    /// Converts the packed form to the hash form and answers the hash, or
    /// answers the hash the table already held.
    ///
    /// This is the **only** place the packed invariant is given up, so every
    /// operation that cannot hold it — a gap, a non-numeric key, a `"08"`, a
    /// removal from the middle — is one call to this and then today's code
    /// path unchanged. Materializing the keys here is what makes it O(n): the
    /// bet the module docs state is that a list is written as a list.
    fn hashed_mut(&mut self) -> &mut Hashed {
        if let Shape::Packed(values) = &mut self.shape {
            let packed = std::mem::take(values);
            let mut hashed = Hashed::default();
            for (index, value) in packed.into_iter().enumerate() {
                let displaced = hashed.set(MwlStr::new(index.to_string().as_bytes()), value);
                debug_assert!(displaced.is_none(), "a packed key appears exactly once");
            }
            self.shape = Shape::Hashed(hashed);
        }
        let Shape::Hashed(hashed) = &mut self.shape else {
            unreachable!("the packed form was converted just above")
        };
        hashed
    }

    /// The value at `key`, borrowed: no reference is added, the same way
    /// `mwl_ir::ir::InstKind::FieldGet` reads a property.
    fn get(&self, key: &[u8]) -> Option<Value> {
        match &self.shape {
            Shape::Packed(values) => values.get(packed_index(key)?).copied(),
            Shape::Hashed(hashed) => hashed.get(key),
        }
    }

    /// Inserts or overwrites, taking over `key`'s and `value`'s references and
    /// handing back whatever it displaced for the caller to release.
    ///
    /// An overwrite keeps the existing entry's position and its existing key
    /// allocation, so `$a["k"] = 1; $a["k"] = 2;` does not move `"k"` to the
    /// end — PHP's own behaviour, and what ADR 0007 § 5's "iteration order is
    /// insertion order, always" means for a repeated write.
    ///
    /// The packed form holds for a write at an existing position or at exactly
    /// the next one; anything else degrades first. The `key` it was handed is
    /// dropped rather than stored in that case — the packed form has no use for
    /// it, which is the whole saving.
    fn set(&mut self, key: MwlStr, value: Value) -> Option<Value> {
        self.note_key(key.as_bytes());
        if let Shape::Packed(values) = &mut self.shape {
            if let Some(slot) = packed_index(key.as_bytes()).filter(|slot| *slot <= values.len()) {
                if slot == values.len() {
                    values.push(value);
                    return None;
                }
                return Some(std::mem::replace(&mut values[slot], value));
            }
        }
        self.hashed_mut().set(key, value)
    }

    /// Appends under the next integer key.
    ///
    /// A packed array whose counter is its own length — every list that has
    /// not had an entry removed — pushes, and that is the path with no key
    /// rendering, no allocation and no hashing at all.
    fn append(&mut self, value: Value) {
        let counter = self.next_index.unwrap_or(0);
        if let Shape::Packed(values) = &mut self.shape {
            if usize::try_from(counter).is_ok_and(|next| next == values.len()) {
                values.push(value);
                self.next_index = Some(counter.saturating_add(1));
                return;
            }
        }
        let key = MwlStr::new(counter.to_string().as_bytes());
        self.note_key(key.as_bytes());
        let displaced = self.hashed_mut().set(key, value);
        debug_assert!(
            displaced.is_none(),
            "the append counter never names a live key"
        );
    }

    /// Removes `key`, handing back the value for the caller to release.
    ///
    /// A packed array stays packed when the key removed is the last one, and
    /// degrades for any other position — a hole is exactly what the invariant
    /// forbids. The counter is untouched either way, so
    /// `$a = [1,2,3]; unset($a[2]); $a[] = 9;` appends at `3` in both shapes,
    /// which is what PHP 8.5 does.
    fn remove(&mut self, key: &[u8]) -> Option<Value> {
        if let Shape::Packed(values) = &mut self.shape {
            match packed_index(key).filter(|slot| *slot < values.len()) {
                None => return None,
                Some(slot) if slot + 1 == values.len() => return values.pop(),
                Some(_) => {}
            }
        }
        self.hashed_mut().remove(key)
    }

    /// The position of the first live entry at or after `from`, or `None` when
    /// there is none — one `foreach` step.
    fn next_slot(&self, from: usize) -> Option<usize> {
        match &self.shape {
            Shape::Packed(values) => (from < values.len()).then_some(from),
            Shape::Hashed(hashed) => hashed.next_slot(from),
        }
    }

    /// The key at `slot`, as a fresh reference the caller owns — synthesized
    /// from the position itself in the packed form, so `foreach ($a as $v)`,
    /// which never asks, never pays for one.
    fn key_at(&self, slot: usize) -> Option<MwlStr> {
        match &self.shape {
            Shape::Packed(values) => (slot < values.len())
                .then(|| MwlStr::new(slot.to_string().as_bytes())),
            Shape::Hashed(hashed) => hashed.at(slot).map(|entry| entry.key.clone()),
        }
    }

    /// The value at `slot`, borrowed rather than retained.
    fn value_at(&self, slot: usize) -> Option<Value> {
        match &self.shape {
            Shape::Packed(values) => values.get(slot).copied(),
            Shape::Hashed(hashed) => hashed.at(slot).map(|entry| entry.value),
        }
    }

    /// A separated copy: the same entries in the same shape and the same
    /// order, each key and value retained.
    fn separate(&self) -> Self {
        let shape = match &self.shape {
            Shape::Packed(values) => {
                for value in values {
                    #[expect(
                        unsafe_code,
                        reason = "every stored value is well-formed and kept alive by \
                                  the reference this table already owns, so adding one \
                                  more is exactly what the copy needs to own"
                    )]
                    unsafe {
                        value.retain();
                    }
                }
                Shape::Packed(values.clone())
            }
            Shape::Hashed(hashed) => Shape::Hashed(hashed.separate()),
        };
        Self {
            next_index: self.next_index,
            shape,
        }
    }

    /// Empties the table, handing back every value it held for the caller to
    /// release — [`dismantle`]'s half of freeing an array. Each key is an
    /// ordinary string owning no MWL value, so dropping the storage frees the
    /// keys directly.
    fn take_values(&mut self) -> Vec<Value> {
        match &mut self.shape {
            Shape::Packed(values) => std::mem::take(values),
            Shape::Hashed(hashed) => {
                hashed.index.clear();
                hashed.live = 0;
                std::mem::take(&mut hashed.entries)
                    .into_iter()
                    .flatten()
                    .map(|entry| entry.value)
                    .collect()
            }
        }
    }
}

impl Hashed {
    /// The value at `key`, borrowed: no reference is added, the same way
    /// `mwl_ir::ir::InstKind::FieldGet` reads a property.
    fn get(&self, key: &[u8]) -> Option<Value> {
        let slot = *self.index.get(key)?;
        self.entries[slot].as_ref().map(|entry| entry.value)
    }

    /// Inserts or overwrites, taking over `key`'s and `value`'s references and
    /// handing back whatever it displaced for the caller to release. The
    /// append counter is [`Table`]'s, and is already up to date by here.
    fn set(&mut self, key: MwlStr, value: Value) -> Option<Value> {
        if let Some(&slot) = self.index.get(key.as_bytes()) {
            let entry = self.entries[slot]
                .as_mut()
                .expect("an indexed slot is always live");
            return Some(std::mem::replace(&mut entry.value, value));
        }
        let slot = self.entries.len();
        self.index.insert(key.clone(), slot);
        self.entries.push(Some(Entry { key, value }));
        self.live += 1;
        None
    }

    /// Removes `key`, handing back the value for the caller to release.
    ///
    /// Leaves a hole rather than shifting, then sweeps when the holes
    /// outnumber the live entries — see this module's docs.
    fn remove(&mut self, key: &[u8]) -> Option<Value> {
        let slot = self.index.remove(key)?;
        let entry = self.entries[slot]
            .take()
            .expect("an indexed slot is always live");
        self.live -= 1;
        let value = entry.value;
        drop(entry.key);
        if self.entries.len() > 8 && self.entries.len() >= self.live * 2 {
            self.compact();
        }
        Some(value)
    }

    /// Drops every hole, then reindexes. O(entries), run only when at least
    /// half of them are holes, so removal stays O(1) amortized.
    fn compact(&mut self) {
        self.entries.retain(Option::is_some);
        self.index.clear();
        for (slot, entry) in self.entries.iter().enumerate() {
            let key = &entry.as_ref().expect("holes were just dropped").key;
            self.index.insert(key.clone(), slot);
        }
    }

    /// The position of the first live entry at or after `from`, or `None` when
    /// there is none — one `foreach` step.
    fn next_slot(&self, from: usize) -> Option<usize> {
        (from..self.entries.len()).find(|slot| self.entries[*slot].is_some())
    }

    /// The live entry at `slot`, if there is one.
    fn at(&self, slot: usize) -> Option<&Entry> {
        self.entries.get(slot)?.as_ref()
    }

    /// A separated copy: the same entries in the same order, each key and
    /// value retained.
    ///
    /// Shallow by construction, and correctly so — a nested array is itself
    /// copy-on-write, so retaining it shares its storage until whichever
    /// holder writes first separates it in turn.
    fn separate(&self) -> Self {
        let mut copy = Self {
            entries: Vec::with_capacity(self.live),
            index: HashMap::with_capacity(self.live),
            live: self.live,
        };
        for entry in self.entries.iter().flatten() {
            #[expect(
                unsafe_code,
                reason = "every stored value is well-formed and kept alive by \
                          the reference this table already owns, so adding one \
                          more is exactly what the copy needs to own"
            )]
            unsafe {
                entry.value.retain();
            }
            copy.index.insert(entry.key.clone(), copy.entries.len());
            copy.entries.push(Some(Entry {
                key: entry.key.clone(),
                value: entry.value,
            }));
        }
        copy
    }
}

/// The header every array pointer refers to.
///
/// `#[repr(C)]` with the reference count first so that the one field compiled
/// code may ever read inline sits at a fixed offset; everything behind it is
/// this module's business — see the module docs.
#[repr(C)]
pub struct ArrayHeader {
    /// How many owners hold this allocation. Reaching `0` frees it.
    refcount: Cell<usize>,
    /// The ordered hash. Behind a [`RefCell`] rather than reached through
    /// `&mut *ptr`, so the "only a uniquely owned array is mutated" rule is
    /// checked at runtime instead of remembered — the direction
    /// [AGENTS.md](../../../AGENTS.md)'s memory section asks for. Every
    /// mutator below drops the borrow before releasing anything, so no
    /// release can re-enter one.
    table: RefCell<Table>,
}

/// Byte offset of the reference count within [`ArrayHeader`].
pub const ARRAY_REFCOUNT_OFFSET: usize = std::mem::offset_of!(ArrayHeader, refcount);

impl fmt::Debug for ArrayHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArrayHeader")
            .field("refcount", &self.refcount.get())
            .field("count", &self.table.borrow().len())
            .finish()
    }
}

/// An owning handle to one reference of an MWL array.
///
/// Cloning retains, dropping releases, and a mutator separates first when the
/// handle is not the only owner — so Rust-side code (helpers, tests, and
/// `mwl-stdlib`'s `Core\Arr`) never writes the copy-on-write protocol by hand.
/// Compiled code instead calls the primitives at the bottom of this file,
/// which are the same operations with the ownership left implicit.
///
/// Neither `Send` nor `Sync`, by construction — see [`crate::string`]'s own
/// docs for the reasoning, which is identical here.
#[repr(transparent)]
pub struct MwlArray {
    ptr: NonNull<ArrayHeader>,
}

impl MwlArray {
    /// A fresh empty array with a reference count of one.
    #[must_use]
    pub fn new() -> Self {
        let boxed = Box::new(ArrayHeader {
            refcount: Cell::new(1),
            table: RefCell::new(Table::default()),
        });
        let ptr = NonNull::from(Box::leak(boxed));
        Self { ptr }
    }

    /// How many entries the array holds — `Core\Arr::count`.
    #[must_use]
    pub fn count(&self) -> usize {
        self.header().table.borrow().len()
    }

    /// Whether the array holds no entries — `Helper::ArrayTruthy`'s falsy
    /// case.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// How many owners currently hold this allocation.
    #[must_use]
    pub fn refcount(&self) -> usize {
        self.header().refcount.get()
    }

    /// Whether the array is in the packed form — the representation invariant
    /// this module's own tests assert, and the one thing about the storage
    /// nothing else is allowed to ask.
    #[cfg(test)]
    fn is_packed(&self) -> bool {
        matches!(self.header().table.borrow().shape, Shape::Packed(_))
    }

    /// The value stored at `key`, borrowed rather than retained.
    #[must_use]
    pub fn get(&self, key: &[u8]) -> Option<Value> {
        self.header().table.borrow().get(key)
    }

    /// Whether `key` is present — `Core\Arr::hasKey`.
    #[must_use]
    pub fn has_key(&self, key: &[u8]) -> bool {
        self.get(key).is_some()
    }

    /// Writes `value` at `key`, taking over both references and separating
    /// first if this handle is not the only owner.
    pub fn set(&mut self, key: MwlStr, value: Value) {
        self.make_unique();
        let displaced = self.header().table.borrow_mut().set(key, value);
        if let Some(old) = displaced {
            #[expect(
                unsafe_code,
                reason = "the table owned exactly one reference to the value                           this write displaced, and no longer holds it"
            )]
            unsafe {
                crate::release::release_value(old);
            }
        }
    }

    /// Appends `value` under the next integer key, taking over its reference
    /// and separating first if this handle is not the only owner.
    pub fn append(&mut self, value: Value) {
        self.make_unique();
        self.header().table.borrow_mut().append(value);
    }

    /// Removes `key` if present, separating first if this handle is not the
    /// only owner — `unset($a[$k])`.
    pub fn unset(&mut self, key: &[u8]) {
        if !self.has_key(key) {
            return;
        }
        self.make_unique();
        let removed = self.header().table.borrow_mut().remove(key);
        if let Some(value) = removed {
            #[expect(
                unsafe_code,
                reason = "the table owned exactly one reference to the value                           this removal took out, and no longer holds it"
            )]
            unsafe {
                crate::release::release_value(value);
            }
        }
    }

    /// The position of the first live entry at or after `from` — the cursor
    /// `foreach` advances.
    #[must_use]
    pub fn next_slot(&self, from: usize) -> Option<usize> {
        self.header().table.borrow().next_slot(from)
    }

    /// The key at `slot`, as a fresh reference the caller owns.
    #[must_use]
    pub fn key_at(&self, slot: usize) -> Option<MwlStr> {
        self.header().table.borrow().key_at(slot)
    }

    /// The value at `slot`, borrowed rather than retained.
    #[must_use]
    pub fn value_at(&self, slot: usize) -> Option<Value> {
        self.header().table.borrow().value_at(slot)
    }

    /// Every key in insertion order — `Core\Arr::keys`, and what a test reads
    /// to assert ADR 0007 § 5's ordering.
    #[must_use]
    pub fn keys(&self) -> Vec<Vec<u8>> {
        let table = self.header().table.borrow();
        let mut keys = Vec::with_capacity(table.len());
        let mut slot = 0;
        while let Some(live) = table.next_slot(slot) {
            keys.push(
                table
                    .key_at(live)
                    .expect("next_slot only names live entries")
                    .as_bytes()
                    .to_vec(),
            );
            slot = live + 1;
        }
        keys
    }

    /// Makes this handle the sole owner, copying the entry storage if it was
    /// not — the copy-on-write separation, and the one place it happens.
    ///
    /// A no-op at a reference count of one, which is the fast path every
    /// mutator above is written to take.
    fn make_unique(&mut self) {
        if self.refcount() == 1 {
            return;
        }
        let copy = Self::new();
        *copy.header().table.borrow_mut() = self.header().table.borrow().separate();
        let old = std::mem::replace(&mut self.ptr, copy.ptr);
        std::mem::forget(copy);
        #[expect(
            unsafe_code,
            reason = "this handle owned exactly the reference being given up, \
                      and has already been re-pointed at the copy"
        )]
        unsafe {
            mwl_array_release(old.as_ptr());
        }
    }

    fn header(&self) -> &ArrayHeader {
        #[expect(
            unsafe_code,
            reason = "`self.ptr` is live for `&self`'s borrow: this handle owns \
                      one of the references keeping it alive"
        )]
        unsafe {
            self.ptr.as_ref()
        }
    }

    /// Gives up ownership of this handle's reference, yielding the raw pointer
    /// compiled code holds.
    ///
    /// The caller now owns exactly one reference and must eventually pass the
    /// pointer to [`mwl_array_release`] or [`MwlArray::from_raw`].
    #[must_use]
    pub fn into_raw(self) -> *mut ArrayHeader {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }

    /// Reclaims a reference previously given up by [`MwlArray::into_raw`].
    ///
    /// # Safety
    ///
    /// `ptr` must be a pointer produced by [`MwlArray::into_raw`] (or by
    /// [`mwl_array_new`]) whose reference has not already been released, and
    /// it must not be reclaimed twice.
    ///
    /// # Panics
    ///
    /// If `ptr` is null, which no MWL array pointer ever is.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "reclaiming a reference is the caller's obligation to state"
    )]
    pub unsafe fn from_raw(ptr: *mut ArrayHeader) -> Self {
        Self {
            ptr: NonNull::new(ptr).expect("an MWL array pointer is never null"),
        }
    }

    /// How many owners hold the array `ptr` refers to.
    ///
    /// # Safety
    ///
    /// `ptr` must refer to a live MWL array allocation.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the pointee's liveness is the caller's obligation to state"
    )]
    pub unsafe fn refcount_of(ptr: *const ArrayHeader) -> usize {
        #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
        unsafe {
            (*ptr).refcount.get()
        }
    }
}

impl Default for MwlArray {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MwlArray {
    fn clone(&self) -> Self {
        bump(self.ptr.as_ptr());
        Self { ptr: self.ptr }
    }
}

impl Drop for MwlArray {
    fn drop(&mut self) {
        #[expect(
            unsafe_code,
            reason = "this handle owns exactly the reference being dropped"
        )]
        unsafe {
            mwl_array_release(self.ptr.as_ptr());
        }
    }
}

impl fmt::Debug for MwlArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MwlArray")
            .field("refcount", &self.refcount())
            .field("count", &self.count())
            .finish()
    }
}

/// Adds one reference to the array at `ptr`.
///
/// Not itself `unsafe` to *call* from this module because every caller here
/// already holds a live handle.
fn bump(ptr: *mut ArrayHeader) {
    #[expect(
        unsafe_code,
        reason = "every caller in this module holds a live reference to `ptr`"
    )]
    let header = unsafe { &*ptr };
    header.refcount.set(
        header
            .refcount
            .get()
            .checked_add(1)
            .expect("an MWL array's reference count cannot overflow a usize"),
    );
}

/// Drops one reference to the array at `ptr`, reporting whether that was the
/// last.
///
/// Does **not** free: [`crate::release`] owns that step, so a nested array's
/// or object's release never recurses.
///
/// # Safety
///
/// `ptr` must refer to a live MWL array allocation whose reference the caller
/// owns.
#[expect(
    unsafe_code,
    reason = "owning the reference is the caller's obligation to state"
)]
pub(crate) unsafe fn drop_one(ptr: *mut ArrayHeader) -> bool {
    #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
    let header = unsafe { &*ptr };
    let remaining = header.refcount.get() - 1;
    header.refcount.set(remaining);
    remaining == 0
}

/// Frees an array allocation whose count reached zero, handing every value it
/// held to `work` rather than releasing it here — see [`crate::release`].
///
/// Each entry's *key* is an ordinary string, which owns no MWL value, so
/// dropping the entry vector frees the keys directly.
///
/// # Safety
///
/// `ptr` must refer to an MWL array allocation whose reference count reached
/// zero in [`drop_one`], and must be dismantled exactly once.
#[expect(
    unsafe_code,
    reason = "reaching zero exactly once is the caller's obligation to state"
)]
pub(crate) unsafe fn dismantle(ptr: *mut ArrayHeader, work: &mut Vec<crate::release::Dying>) {
    #[expect(
        unsafe_code,
        reason = "the count reached zero, so nothing else can observe the \
                  allocation; it was produced by `Box::leak` in `MwlArray::new`"
    )]
    let boxed = unsafe { Box::from_raw(ptr) };
    let values = boxed.table.borrow_mut().take_values();
    for value in values {
        #[expect(
            unsafe_code,
            reason = "the array owned exactly one reference to each value it \
                      held, and it no longer exists"
        )]
        if let Some(dying) = unsafe { crate::release::step_field(value) } {
            work.push(dying);
        }
    }
}

// ---------------------------------------------------------------------------
// The primitives compiled code calls
// ---------------------------------------------------------------------------
//
// The same split [`crate::string`]'s own primitives are on, for the same
// reason: none of these can fail, so none of them wears ADR 0002's
// checked-return shape. Every one is `extern "C"` and never
// `extern "C-unwind"`.
//
// **A `Value` crosses this boundary through a pointer, never by value.** A
// 16-byte struct is classified differently by the SysV and Windows x64 ABIs —
// two integer registers on one, a hidden pointer on the other — and
// `mwl-codegen` would have to encode that difference to call these at all.
// Every site that needs one therefore passes the address of a 16-byte slot the
// caller owns, which is exactly what ADR 0002's own `(ctx, args, out)` helper
// shape already does, so codegen reuses `store_value`/`load_value` unchanged.
//
// Every mutator consumes one reference to its `array` argument and returns
// one — see this module's copy-on-write decision, which is the whole reason
// the signatures are shaped that way rather than returning nothing.

/// A fresh empty array with a reference count of one —
/// `mwl_ir::InstKind::ArrayNew`'s allocation half.
///
/// The one primitive here that is safe to call: it reads no pointer the caller
/// supplied, because it takes none.
#[expect(
    unsafe_code,
    reason = "exporting an unmangled symbol is what makes compiled code able \
              to call this at all"
)]
#[unsafe(no_mangle)]
pub extern "C" fn mwl_array_new() -> *mut ArrayHeader {
    MwlArray::new().into_raw()
}

/// Adds a reference — `mwl_ir::InstKind::Retain` for a `Ty::Array` operand.
///
/// # Safety
///
/// `ptr` must refer to a live MWL array allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw array pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_array_retain(ptr: *mut ArrayHeader) {
    if ptr.is_null() {
        return;
    }
    bump(ptr);
}

/// Drops a reference, freeing the array and everything it solely owns if it
/// was the last — `mwl_ir::InstKind::Release` for a `Ty::Array` operand.
///
/// # Safety
///
/// `ptr` must refer to a live MWL array allocation whose reference this caller
/// owns, and must not be released twice.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw array pointer whose ownership the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_array_release(ptr: *mut ArrayHeader) {
    if ptr.is_null() {
        return;
    }
    #[expect(unsafe_code, reason = "the caller guarantees it owns the reference")]
    unsafe {
        crate::release::release_value(Value::from_array_ptr(ptr));
    }
}

/// Reads the entry at `key` **without** retaining what it holds —
/// `mwl_ir::InstKind::ArrayGet`.
///
/// A missing key reads back `null`, which is the only thing this instruction
/// can do until `Core\Arr` and the checker settle what an absent key means
/// (that variant's own doc comment names the gap). Neither the array nor the
/// key is consumed, and the value written to `out` is *borrowed*: the array
/// keeps its reference, so a caller that stores the result retains it itself.
///
/// # Safety
///
/// `array` must refer to a live MWL array allocation, `key` to a live MWL
/// string allocation, and `out` to a writable, aligned 16-byte slot.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw pointers whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_array_get(
    array: *mut ArrayHeader,
    key: *const StrHeader,
    out: *mut Value,
) {
    #[expect(unsafe_code, reason = "the caller guarantees every pointee is live")]
    unsafe {
        let bytes = MwlStr::bytes_of(key);
        let value = (*array).table.borrow().get(bytes).unwrap_or_default();
        out.write(value);
    }
}

/// Whether `key` is present — `Core\Arr::hasKey`'s entry point. Consumes
/// nothing.
///
/// # Safety
///
/// `array` must refer to a live MWL array allocation and `key` to a live MWL
/// string allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw pointers whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_array_has_key(array: *mut ArrayHeader, key: *const StrHeader) -> bool {
    #[expect(unsafe_code, reason = "the caller guarantees both pointees are live")]
    unsafe {
        let bytes = MwlStr::bytes_of(key);
        (*array).table.borrow().get(bytes).is_some()
    }
}

/// Writes `value` at `key` — `mwl_ir::InstKind::ArraySet`.
///
/// Consumes one reference to `array`, one to `key` and one to `value`, and
/// returns the one reference to the array that now holds the entry: the same
/// pointer when `array` was uniquely owned, a separated copy otherwise.
///
/// # Safety
///
/// `array` must refer to a live MWL array allocation whose reference the
/// caller owns, `key` to a live MWL string allocation whose reference the
/// caller owns, and `value` must own the reference it transfers.
#[expect(
    unsafe_code,
    reason = "compiled code passes raw pointers whose ownership the signature \
              cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_array_set(
    array: *mut ArrayHeader,
    key: *mut StrHeader,
    value: *const Value,
) -> *mut ArrayHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees it owns one reference to each argument \
                  and that `value` points at a readable 16-byte slot"
    )]
    unsafe {
        let mut handle = MwlArray::from_raw(array);
        handle.set(MwlStr::from_raw(key), value.read());
        handle.into_raw()
    }
}

/// Appends `value` under the next integer key —
/// `mwl_ir::InstKind::ArrayAppend`, `$a[] = expr`.
///
/// Consumes one reference to `array` and one to `value`, and returns the one
/// reference to the array that now holds the entry — see
/// [`mwl_array_set`].
///
/// # Safety
///
/// `array` must refer to a live MWL array allocation whose reference the
/// caller owns, and `value` must own the reference it transfers.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw pointer and a value whose ownership \
              the signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_array_append(
    array: *mut ArrayHeader,
    value: *const Value,
) -> *mut ArrayHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees it owns one reference to each argument \
                  and that `value` points at a readable 16-byte slot"
    )]
    unsafe {
        let mut handle = MwlArray::from_raw(array);
        handle.append(value.read());
        handle.into_raw()
    }
}

/// Removes `key` if present — `unset($a[$k])`.
///
/// Consumes one reference to `array` and returns the one reference to the
/// array without the entry — see [`mwl_array_set`]. `key` is borrowed, not
/// consumed: nothing is stored, so nothing needs to be owned.
///
/// # Safety
///
/// `array` must refer to a live MWL array allocation whose reference the
/// caller owns, and `key` to a live MWL string allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes raw pointers whose ownership the signature \
              cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_array_unset(
    array: *mut ArrayHeader,
    key: *const StrHeader,
) -> *mut ArrayHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees it owns `array`'s reference and that \
                  `key` is live"
    )]
    unsafe {
        let mut handle = MwlArray::from_raw(array);
        let bytes = MwlStr::bytes_of(key).to_vec();
        handle.unset(&bytes);
        handle.into_raw()
    }
}

/// How many entries the array holds — `Core\Arr::count`'s entry point.
/// Consumes nothing.
///
/// # Safety
///
/// `array` must refer to a live MWL array allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw array pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_array_count(array: *const ArrayHeader) -> i64 {
    #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
    let live = unsafe { (*array).table.borrow().len() };
    i64::try_from(live).expect("an array cannot hold more than i64::MAX entries")
}

/// The position of the first live entry at or after `from`, or `-1` when there
/// is none — one `foreach` step over ADR 0007 § 5's insertion order. Consumes
/// nothing.
///
/// A cursor rather than a borrowed iterator because the loop body runs
/// compiled code between two steps: `foreach` holds its own reference to the
/// array for the loop's whole duration, so a write inside the body separates
/// and the cursor keeps walking the snapshot the loop started on — which is
/// exactly PHP's by-value `foreach`.
///
/// # Safety
///
/// `array` must refer to a live MWL array allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw array pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_array_next_slot(array: *const ArrayHeader, from: usize) -> i64 {
    #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
    let slot = unsafe { (*array).table.borrow().next_slot(from) };
    slot.map_or(-1, |slot| {
        i64::try_from(slot).expect("an array cannot hold more than i64::MAX entries")
    })
}

/// The key at `slot`, as a fresh reference the caller owns — `foreach`'s
/// `$k` binding. Consumes nothing.
///
/// # Safety
///
/// `array` must refer to a live MWL array allocation, and `slot` must be a
/// position [`mwl_array_next_slot`] returned and nothing has removed since.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw array pointer and a slot the \
              signature cannot bound"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_array_key_at(
    array: *const ArrayHeader,
    slot: usize,
) -> *mut StrHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the allocation is live and the slot is \
                  a live entry's"
    )]
    let key = unsafe {
        (*array)
            .table
            .borrow()
            .key_at(slot)
            .expect("a foreach cursor only names live entries")
    };
    key.into_raw()
}

/// Writes the value at `slot` into `out`, borrowed rather than retained —
/// `foreach`'s `$v` binding. Consumes nothing.
///
/// # Safety
///
/// `array` must refer to a live MWL array allocation, `slot` must be a
/// position [`mwl_array_next_slot`] returned and nothing has removed since,
/// and `out` must be a writable, aligned 16-byte slot.
#[expect(
    unsafe_code,
    reason = "compiled code passes raw pointers and a slot the signature \
              cannot bound"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_array_value_at(
    array: *const ArrayHeader,
    slot: usize,
    out: *mut Value,
) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the allocation is live, that the slot \
                  is a live entry's, and that `out` is a writable 16-byte slot"
    )]
    unsafe {
        let value = (*array)
            .table
            .borrow()
            .value_at(slot)
            .expect("a foreach cursor only names live entries");
        out.write(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counting_alloc::live_bytes;

    fn key(text: &str) -> MwlStr {
        MwlStr::new(text.as_bytes())
    }

    fn keys_of(array: &MwlArray) -> Vec<String> {
        array
            .keys()
            .into_iter()
            .map(|k| String::from_utf8(k).expect("test keys are ASCII"))
            .collect()
    }

    #[test]
    fn a_fresh_array_is_empty_and_solely_owned() {
        let array = MwlArray::new();
        assert_eq!(array.count(), 0);
        assert!(array.is_empty());
        assert_eq!(array.refcount(), 1);
    }

    #[test]
    fn a_list_shaped_array_holds_no_index_map() {
        // The invariant, measured rather than asserted about: a list of `RUN`
        // values costs the header and one `Vec<Value>`. An index map would add
        // its own allocation and a key string would add `RUN` more, so the
        // ceiling below is the whole claim — see this module's packed
        // decision.
        const RUN: usize = 32;

        let before = live_bytes();
        let mut list = MwlArray::new();
        for value in 0..RUN {
            list.append(Value::int(i64::try_from(value).expect("a small count")));
        }
        let held = live_bytes() - before;

        assert!(list.is_packed(), "an appended-into array is a list");
        let ceiling = isize::try_from(
            std::mem::size_of::<ArrayHeader>() + RUN * std::mem::size_of::<Value>(),
        )
        .expect("a small size");
        assert!(
            held <= ceiling,
            "a packed array of {RUN} holds {held} bytes, over the {ceiling} \
             its header and values cost: something is storing keys"
        );

        // Every key is still `string`-shaped and still there to be asked for,
        // which is what makes this representation and not semantics.
        assert_eq!(list.count(), RUN);
        assert_eq!(
            keys_of(&list),
            (0..RUN).map(|n| n.to_string()).collect::<Vec<_>>()
        );
        assert_eq!(list.get(b"0").and_then(Value::as_int), Some(0));
        assert_eq!(list.get(b"31").and_then(Value::as_int), Some(31));
        assert!(list.has_key(b"31"));

        // And the keys that are not there are the same ones a hash would miss:
        // one past the end, a negative, and `"08"`, which ADR 0007 § 5 keeps
        // distinct from `"8"`.
        assert!(list.get(b"32").is_none());
        assert!(list.get(b"-1").is_none());
        assert!(list.get(b"08").is_none());
        assert!(list.get(b"x").is_none());
        assert!(!list.has_key(b"32"));
    }

    #[test]
    fn iteration_order_is_insertion_order() {
        let mut array = MwlArray::new();
        array.set(key("alpha"), Value::int(1));
        array.set(key("beta"), Value::int(2));
        array.set(key("gamma"), Value::int(3));
        assert_eq!(keys_of(&array), ["alpha", "beta", "gamma"]);
    }

    #[test]
    fn overwriting_keeps_a_keys_position() {
        let mut array = MwlArray::new();
        array.set(key("alpha"), Value::int(1));
        array.set(key("beta"), Value::int(2));
        array.set(key("alpha"), Value::int(9));
        assert_eq!(keys_of(&array), ["alpha", "beta"]);
        assert_eq!(array.get(b"alpha").and_then(Value::as_int), Some(9));
        assert_eq!(array.count(), 2);
    }

    #[test]
    fn removing_and_reinserting_moves_a_key_to_the_end() {
        // `examples/arrays.mwl`'s frozen output depends on exactly this.
        let mut array = MwlArray::new();
        for (name, number) in [("alpha", 1), ("beta", 2), ("gamma", 3)] {
            array.set(key(name), Value::int(number));
        }
        array.unset(b"beta");
        assert_eq!(keys_of(&array), ["alpha", "gamma"]);
        array.set(key("beta"), Value::int(20));
        assert_eq!(keys_of(&array), ["alpha", "gamma", "beta"]);
    }

    #[test]
    fn appending_follows_the_highest_integer_key_used() {
        let mut array = MwlArray::new();
        array.append(Value::int(10));
        array.append(Value::int(11));
        assert_eq!(keys_of(&array), ["0", "1"]);
        array.set(key("7"), Value::int(12));
        array.append(Value::int(13));
        assert_eq!(keys_of(&array), ["0", "1", "7", "8"]);
        // The counter survives a removal, exactly as PHP's does.
        array.unset(b"8");
        array.append(Value::int(14));
        assert_eq!(keys_of(&array), ["0", "1", "7", "9"]);
    }

    #[test]
    fn a_negative_integer_key_advances_the_counter() {
        // Every assertion below is `php -r` output from the 8.5.9 oracle.
        let mut array = MwlArray::new();
        array.set(key("-5"), Value::int(1));
        array.append(Value::int(2));
        assert_eq!(keys_of(&array), ["-5", "-4"]);

        // A negative key may only *raise* the counter once something has set
        // it, so it is ignored here.
        let mut after_positive = MwlArray::new();
        after_positive.set(key("3"), Value::int(1));
        after_positive.set(key("-5"), Value::int(1));
        after_positive.append(Value::int(2));
        assert_eq!(keys_of(&after_positive), ["3", "-5", "4"]);

        // An append is itself what sets the counter on an empty array.
        let mut appended_first = MwlArray::new();
        appended_first.append(Value::int(1));
        appended_first.set(key("-9"), Value::int(1));
        appended_first.append(Value::int(2));
        assert_eq!(keys_of(&appended_first), ["0", "-9", "1"]);
    }

    #[test]
    fn a_non_canonical_numeric_key_stays_a_string_key() {
        assert_eq!(integer_key(b"8"), Some(8));
        assert_eq!(integer_key(b"0"), Some(0));
        assert_eq!(integer_key(b"-1"), Some(-1));
        assert_eq!(integer_key(b"08"), None);
        assert_eq!(integer_key(b"-0"), None);
        assert_eq!(integer_key(b""), None);
        assert_eq!(integer_key(b"1x"), None);
        assert_eq!(integer_key(b" 1"), None);
        assert_eq!(integer_key(b"9223372036854775808"), None);

        let mut array = MwlArray::new();
        array.set(key("08"), Value::int(1));
        array.append(Value::int(2));
        assert_eq!(keys_of(&array), ["08", "0"]);
    }

    #[test]
    fn a_write_through_a_second_handle_separates() {
        let mut original = MwlArray::new();
        original.set(key("alpha"), Value::int(1));
        let mut copy = original.clone();
        assert_eq!(original.refcount(), 2);

        copy.set(key("alpha"), Value::int(99));

        assert_eq!(original.get(b"alpha").and_then(Value::as_int), Some(1));
        assert_eq!(copy.get(b"alpha").and_then(Value::as_int), Some(99));
        assert_eq!(original.refcount(), 1);
        assert_eq!(copy.refcount(), 1);
    }

    #[test]
    fn a_solely_owned_write_never_separates() {
        let mut array = MwlArray::new();
        let before = array.header() as *const ArrayHeader;
        for index in 0..64 {
            array.set(key(&index.to_string()), Value::int(index));
        }
        assert!(std::ptr::eq(before, array.header()));
        assert_eq!(array.count(), 64);
    }

    #[test]
    fn separation_shares_a_nested_arrays_storage_until_it_is_written() {
        let inner = MwlArray::new();
        let mut outer = MwlArray::new();
        outer.set(key("cells"), Value::array(inner.clone()));
        assert_eq!(inner.refcount(), 2);

        let mut copy = outer.clone();
        copy.set(key("other"), Value::int(1));
        // Both arrays now name the same inner one: separation is shallow, and
        // the inner array is copy-on-write in its own right.
        assert_eq!(inner.refcount(), 3);
    }

    #[test]
    fn separation_retains_every_stored_string() {
        let text = MwlStr::new(b"shared");
        let mut original = MwlArray::new();
        original.set(key("k"), Value::str(text.clone()));
        assert_eq!(text.refcount(), 2);

        let mut copy = original.clone();
        copy.set(key("other"), Value::int(1));
        assert_eq!(text.refcount(), 3);

        drop(copy);
        assert_eq!(text.refcount(), 2);
        drop(original);
        assert_eq!(text.refcount(), 1);
    }

    #[test]
    fn compaction_preserves_order_and_the_append_counter() {
        let mut array = MwlArray::new();
        for index in 0..32 {
            array.append(Value::int(index));
        }
        for index in 0..30 {
            array.unset(index.to_string().as_bytes());
        }
        assert_eq!(keys_of(&array), ["30", "31"]);
        array.append(Value::int(99));
        assert_eq!(keys_of(&array), ["30", "31", "32"]);
        assert_eq!(array.get(b"31").and_then(Value::as_int), Some(31));
    }

    #[test]
    fn a_cursor_walks_every_live_entry_in_order() {
        let mut array = MwlArray::new();
        for name in ["a", "b", "c", "d"] {
            array.set(key(name), Value::int(1));
        }
        array.unset(b"b");

        let mut seen = Vec::new();
        let mut cursor = 0;
        while let Some(slot) = array.next_slot(cursor) {
            let found = array.key_at(slot).expect("the cursor names a live entry");
            seen.push(String::from_utf8(found.as_bytes().to_vec()).expect("ASCII"));
            cursor = slot + 1;
        }
        assert_eq!(seen, ["a", "c", "d"]);
    }

    #[test]
    fn an_acyclic_array_graph_releases_every_allocation() {
        // The allocator is measured rather than the refcount: a refcount
        // reaching zero says the bookkeeping agreed with itself, not that the
        // memory came back. `crate::object` guards objects the same way.
        let before = live_bytes();
        {
            let mut outer = MwlArray::new();
            for index in 0..16 {
                let mut inner = MwlArray::new();
                inner.set(key("name"), Value::str(MwlStr::new(b"a stored string")));
                inner.append(Value::str(MwlStr::new(b"another")));
                outer.set(key(&index.to_string()), Value::array(inner));
            }
            let alias = outer.clone();
            outer.set(key("written-after-aliasing"), Value::int(1));
            drop(alias);
        }
        assert_eq!(live_bytes(), before);
    }

    #[test]
    fn a_deeply_nested_array_releases_without_recursing() {
        // The depth that would overflow a recursive release. See this module's
        // "freeing is iterative" decision.
        let before = live_bytes();
        {
            let mut nest = MwlArray::new();
            for _ in 0..200_000 {
                let mut outer = MwlArray::new();
                outer.set(key("inner"), Value::array(nest));
                nest = outer;
            }
        }
        assert_eq!(live_bytes(), before);
    }
}
