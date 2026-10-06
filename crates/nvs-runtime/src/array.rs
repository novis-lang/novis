//! Novis's array: `rule:types/arrays`'s insertion-ordered, string-keyed hash, refcounted and copy-on-write.
//!
//! This is what `nvs_ir::ty::Ty::Array` lowers to, and `Core\Arr`'s whole
//! contract rests on it.
//!
//! # Decision: the representation is opaque to compiled code
//!
//! [`crate::NvsStr`] and [`crate::NvsObj`] both publish byte offsets so
//! `nvs-codegen` can load a field or a payload byte inline. An array
//! publishes exactly one, [`ARRAY_REFCOUNT_OFFSET`], and nothing else:
//! every operation on it — a read, a write, an append, an iteration step — is
//! an out-of-line call to one of the primitives at the bottom of this file.
//!
//! That is what lets the container itself be ordinary safe Rust rather than a
//! hand-rolled open-addressed table over a flexible-array-member allocation.
//! An entry lookup already costs a hash and a probe, so the call is not on the
//! margin the way an object field load is; buying the whole table's
//! memory-safety for it spends a few instructions of latency on security, and
//! on a simpler module besides, which [AGENTS.md](/AGENTS.md)'s ordering
//! permits.
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
//! hasher (`FxHash`, the one `nvs-codegen` uses for its compiler-internal
//! tables) is trivially floodable. So this map keeps std's per-process-seeded
//! SipHash: security over latency, the one direction the ordering allows.
//! It also means this module adds no dependency at all.
//!
//! # Decision: a list-shaped array is packed, and hashes nothing
//!
//! An array whose keys are exactly `"0"`…`"n−1"` in insertion order stores a
//! `Vec<Value>` and nothing else — no index map, no key strings. The first
//! operation that breaks that invariant — a `"08"`, a gap, a non-numeric key,
//! an `unset` anywhere but the end — converts it to the hash form above, which
//! is PHP's own arrangement. [`key_at`](NvsArray::key_at) synthesizes the
//! decimal on demand, so `foreach ($a as $v)`, which never asks for a key,
//! never pays for one.
//!
//! **[ADR 0007 § 5](/docs/decisions/0007.md) is
//! unchanged by this.** Every key is still a `string`, `"08"` is still a
//! distinct key from `"8"` (it is what forces the degrade), insertion order is
//! still the iteration order, and `Core\Arr::keys` still answers
//! `array<string>`. This is representation, not semantics.
//!
//! The reason it is not an optimisation to schedule later, measured on this
//! tree against the PHP 8.5.9 oracle on the same machine — and PHP's figures
//! include VM opcode dispatch that compiled Novis does not pay, so the
//! comparison already flatters the interpreter: unpacked, `$a[] = $v` and
//! `$a[$i]` are both several times slower than the interpreter's, while
//! `$a['name']` matches it and `foreach ($a as $v)` beats it outright.
//!
//! Nearly all of an unpacked append is the key the packed form does not build:
//! rendering the index to a decimal `String`, allocating the [`NvsStr`] for it,
//! hashing and probing it, and behind those the index-map insert and its
//! growth. A `Vec<Value>` push and an index into one are a small fraction of
//! any of them. The assoc and iteration rows are healthy and this changes
//! neither.
//!
//! **The ABI is what shapes this.** [`nvs_array_get`] and [`nvs_array_set`]
//! take a `*const StrHeader`, so compiled code calling *those* must build a key
//! string first and a packed form would have to parse the decimal back out —
//! pointless. So [`nvs_array_get_index`] and [`nvs_array_set_index`] sit beside
//! them, taking the `i64` the subscript already was and answering from the
//! packed form with nothing rendered and nothing allocated; they degrade to a
//! synthesized key only where the shape is already `Hashed`, which is exactly
//! the case that would be building one anyway. They are a **compatible
//! addition** — the key-taking pair stands as it is and is still the only path
//! for a `string` subscript — which is why the pair belongs here while nothing
//! depends on the current set, rather than as a versioned break once
//! `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`
//! artifacts and M9's WIT signatures do.
//!
//! **What still calls the key-taking pair for an integer subscript is
//! `nvs-codegen`.** `nvs_ir::lower::Lowering::lower_array_key` normalizes an
//! `int`/`uint` subscript to its decimal string through `Helper::IntToString`
//! before `InstKind::ArrayGet`/`ArraySet` ever reaches codegen, so the key's
//! representation at the emit site is already `Ty::Str` and the allocation has
//! already happened. Routing `$a[$i]` here is therefore an `nvs-ir` change —
//! letting those two instructions carry a `Ty::Int`/`Ty::Uint` key and
//! dispatching on it in `nvs_codegen::emit` — not a codegen-local one.
//!
//! What it spends, as `rule:programs/memory-priority`
//! requires: **nothing — it saves.** A list drops two of its three allocations
//! and every key string. It also takes list data out of the SipHash path the
//! decision above exists to justify, which leaves that decision protecting the
//! case it was actually written for: attacker-controlled *names*.
//!
//! # Decision: an empty array is a per-thread singleton, not an allocation
//!
//! [`nvs_array_new`] hands every caller on a thread the *same* header, and the
//! thread-local holding it owns a reference it never gives up — so that count
//! never reaches zero, and `retain`, `release` and the whole teardown path are
//! unchanged. **No hot path needs a pointer comparison**, which is what makes
//! this cheaper here than the equivalent for the other refcounted container
//! ([`crate::string`]'s § *An immortal string*, where every release compares
//! against a sentinel). `[]` is a `Cell<usize>` bump and a return where it
//! would otherwise be a `Box` per evaluation — and userland produces empty
//! arrays constantly that are never written into: an early return, a `filter`
//! that matched nothing, a lookup that missed, a collector on a branch not
//! taken.
//!
//! What lets the sharing be invisible is what an array already is. Every
//! mutator goes through `make_unique`, which separates whenever the count is
//! not 1, and the singleton's count is never 1 while a caller holds it — so a
//! singleton cannot be written through, by the ordinary copy-on-write path
//! rather than by a special case. `nvs_array_eq` compares by content, so
//! sharing is unobservable to
//! `rule:expressions/one-equality-operator`'s
//! identity row. And an array is neither `Send` nor `Sync`, so per-thread is
//! per-owner and the count stays non-atomic.
//!
//! Deliberately *not* the immortal-header-in-the-data-section arrangement a
//! string literal gets, and the reason is the [`RefCell`]: a read takes
//! `borrow()`, which **writes** the borrow flag, so a header two threads could
//! reach would race on every `count()`. Thread-local is what keeps that flag
//! sound.
//!
//! **Only [`nvs_array_new`] hands the singleton out.** [`NvsArray::new`]
//! allocates, because its Rust-side callers — `make_unique` first among them —
//! take a handle they are about to write through.
//!
//! What it spends, as `rule:programs/memory-priority` requires: **nothing per request — it saves.**
//! One header per thread, permanently, against one per empty array that stays
//! empty. An empty array that *is* later written pays one `make_unique`
//! separation, allocating exactly the header the old path allocated eagerly,
//! plus a failed `== 1` branch — a wash plus a branch, not a regression. The
//! one non-obvious cost is the leak check: a per-thread block that is never
//! freed is *still reachable* rather than *definitely lost*, and
//! `tools/leak-check.sh`'s threshold is what says whether that matters.
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
//! `rule:types/arrays` gives arrays copy-on-write **value semantics**, so
//! `$b = $a; $b["k"] = 1;` must not be visible through `$a`. The separation
//! that guarantees it produces a *different allocation*, which the writer must
//! then be holding — and compiled code keeps a local in an SSA register, not
//! in a memory slot a callee could write back through.
//!
//! So every mutating primitive here takes the array by value and gives it
//! back: it consumes one reference to its `array` argument and returns one
//! reference to the array that now holds the change. When the refcount was
//! already `1` that is the same pointer and the same reference, mutated in
//! place with no copy at all — the fast path `rule:core-api/shape-rules`
//! R3's "nothing mutates" API shape rests on, measured by
//! `a_refcount_one_array_member_mutates_in_place` in `benches/abi-probe`.
//! When it was higher, the entry storage is copied, every key and value in it
//! retained, the caller's reference dropped, and the fresh copy returned.
//!
//! [`NvsArray`] expresses the same protocol safely: its mutators take
//! `&mut self` and re-point the handle, which is why nothing outside this
//! module writes a separation by hand.
//!
//! # Decision: the append is the one array write with a fault channel
//!
//! `$a[] = v` has an outcome no other write has: PHP 8.5 refuses it with
//! *"Cannot add element to the array as the next element is already
//! occupied"* once the append counter names a live key, which
//! [`Table::note_index`]'s saturation at `i64::MAX` is the only way to reach.
//! Novis matches that refusal rather than PHP's older silent overwrite, so
//! [`nvs_array_append`] needs somewhere to put a failure — and the
//! pointer-in, pointer-out shape every other primitive here has does not have
//! one.
//!
//! It therefore takes `rule:errors/propagation`'s
//! shape instead — `(ctx, array, value, out) -> status`, the array it yields
//! travelling through a caller-owned pointer-wide slot the way
//! `nvs_object_slot_set`'s result does — and it is the **only** array
//! primitive that does. `nvs_ir::ir::InstKind::ArrayAppend` carries an
//! `Inst::on_error` edge to match, and `nvs-codegen` gives `nvs_array_unset` a
//! signature of its own rather than borrowing this one.
//!
//! What the refusal costs is stated here because the section above promises
//! the opposite: **the occupancy test runs before the copy-on-write
//! separation** ([`NvsArray::try_append`]), so a refused append leaves the
//! caller's pointer live and still owning the reference it was handed, and
//! compiled code has nothing to re-point on the error path. One comparison on
//! every append pays for it — an unsaturated counter is strictly greater than
//! every integer key in use, so only a counter *at* `i64::MAX` ever reaches
//! the lookup behind it.
//!
//! [`NvsArray::append`] keeps the infallible signature every `Core` producer's
//! `out.append(…)` calls, because an array a call is building from index 0
//! cannot reach that state; it panics rather than overwriting where one
//! somehow does, which `nvs_helper!`'s `catch_unwind` contains to a single
//! request. A `debug_assert` there would leave a release build silently
//! overwriting a live entry — the one place an array could lose a value.
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
//! # Decision: the element type is checked at the write, not remembered
//!
//! `rule:types/arrays` gives an array header a pointer to an interned,
//! immutable descriptor of its element type exactly where something reads one
//! back, and [`ArrayHeader`] carries none: every array here is built by
//! compiled code the checker proved well-typed, and the two runtime checks that
//! exist ask the *target* type rather than the array. A write through `mixed`
//! is checked against the element type of the array being written to, and
//! `as array<U>` restamps at O(n) against `U` ([`crate::helpers`]'s
//! `nvs_to_array_of`) — neither has anything to ask an array what it holds.
//!
//! The one reader that would take a coarser answer takes it knowingly:
//! [`crate::object`]'s tag list records that [`crate::value::Tag::Array`]
//! answers for every `array<T>`, so a shape slot declared `array<Dog>` admits
//! an `array<Animal>`. Closing *that* is what puts the first real reader on the
//! descriptor, and it is a widening of [`ArrayHeader`] rather than a redesign:
//! **one pointer per header**, interned process-wide, so a process pays
//! O(distinct array types in the program) and an array pays a word.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::ptr::NonNull;

use crate::string::{NvsStr, StrHeader};
use crate::value::Value;

/// One key/value pair, owning one reference to each.
pub(crate) struct Entry {
    pub(crate) key: NvsStr,
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
/// which one it is holding: [ADR 0007 § 5](/docs/decisions/0007.md)
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
    index: HashMap<NvsStr, usize>,
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

/// One bucket of a key index, its control byte included — what a `HashMap`
/// holds per entry, and so the stride that prices the index's growth.
const INDEX_BUCKET: usize = size_of::<NvsStr>() + size_of::<usize>() + 1;

/// The slots a store's first allocation takes, however few are asked of it.
const FIRST_SLOTS: usize = 4;

/// What a store of `capacity` slots holding `len` of them, each `stride` bytes
/// wide, asks the allocator for when one more entry goes in: nothing at all
/// while there is room, and its own size again where the amortized growth
/// doubles it. An empty store asks for the floor its first allocation takes
/// rather than a double of nothing.
fn amortized(len: usize, capacity: usize, stride: usize) -> usize {
    if len < capacity {
        return 0;
    }
    capacity.max(FIRST_SLOTS).saturating_mul(stride)
}

/// The `i64` a canonical decimal key denotes, or `None` if the key is not one.
///
/// Canonical means what PHP means by it: an optional `-`, then either `0` or a
/// digit run with no leading zero, and the whole thing in range. `"08"` and
/// `"-0"` are therefore ordinary string keys that collide with nothing, which
/// is `rule:types/arrays`'s "which subscripts collide does not change".
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
            self.note_index(number);
        }
    }

    /// The same, for a key that arrived as an integer and was never rendered —
    /// [`Table::set_index`]'s half of [`Table::note_key`].
    fn note_index(&mut self, number: i64) {
        let after = number.saturating_add(1);
        self.next_index = Some(match self.next_index {
            None => after,
            Some(current) => current.max(after),
        });
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
    /// removal from the middle — is one call to this and then the ordinary
    /// hash-form path. Materializing the keys here is what makes it O(n): the
    /// bet the module docs state is that a list is written as a list.
    fn hashed_mut(&mut self) -> &mut Hashed {
        if let Shape::Packed(values) = &mut self.shape {
            let packed = std::mem::take(values);
            let mut hashed = Hashed::default();
            for (index, value) in packed.into_iter().enumerate() {
                let displaced = hashed.set(NvsStr::new(index.to_string().as_bytes()), value);
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
    /// `nvs_ir::ir::InstKind::FieldGet` reads a property.
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
    /// end — PHP's own behaviour, and what `rule:types/arrays`'s "iteration order is
    /// insertion order, always" means for a repeated write.
    ///
    /// The packed form holds for a write at an existing position or at exactly
    /// the next one; anything else degrades first. The `key` it was handed is
    /// dropped rather than stored in that case — the packed form has no use for
    /// it, which is the whole saving.
    fn set(&mut self, key: NvsStr, value: Value) -> Option<Value> {
        self.note_key(key.as_bytes());
        if let Shape::Packed(values) = &mut self.shape
            && let Some(slot) = packed_index(key.as_bytes()).filter(|slot| *slot <= values.len())
        {
            if slot == values.len() {
                values.push(value);
                return None;
            }
            return Some(std::mem::replace(&mut values[slot], value));
        }
        self.hashed_mut().set(key, value)
    }

    /// [`Table::get`] reached by the integer the subscript already was, with
    /// **no decimal rendered and nothing allocated** while the shape is packed
    /// — `$a[$i]`'s whole point.
    ///
    /// A negative index answers `None` from the packed form directly, because
    /// `"−1"` is a key the packed invariant forbids. Only the hash form has to
    /// synthesize the string, and there it is what the caller would be building
    /// anyway.
    fn get_index(&self, index: i64) -> Option<Value> {
        match &self.shape {
            Shape::Packed(values) => values.get(usize::try_from(index).ok()?).copied(),
            Shape::Hashed(hashed) => hashed.get(index.to_string().as_bytes()),
        }
    }

    /// [`Table::set`] reached the same way, allocating no key while the shape
    /// is packed and the write lands at an existing position or exactly the
    /// next one. The counter is advanced from the integer itself, so a write
    /// that *does* degrade still leaves `$a[]` appending where PHP's would.
    fn set_index(&mut self, index: i64, value: Value) -> Option<Value> {
        self.note_index(index);
        if let Shape::Packed(values) = &mut self.shape
            && let Some(slot) = usize::try_from(index)
                .ok()
                .filter(|slot| *slot <= values.len())
        {
            if slot == values.len() {
                values.push(value);
                return None;
            }
            return Some(std::mem::replace(&mut values[slot], value));
        }
        let key = NvsStr::new(index.to_string().as_bytes());
        self.hashed_mut().set(key, value)
    }

    /// Whether the key `$a[]` would append under is already live — PHP 8.5's
    /// *"Cannot add element to the array as the next element is already
    /// occupied"*, and the one state [`Table::append`] cannot serve.
    ///
    /// Only a saturated counter can name a live key: [`Table::note_index`]
    /// leaves every other counter strictly greater than the key that moved it.
    /// So the comparison short-circuits on every ordinary append, and the
    /// lookup behind it runs only for a program that has actually used
    /// `i64::MAX` as a key — which is what keeps the refusal off the hot path.
    fn next_is_occupied(&self) -> bool {
        self.next_index == Some(i64::MAX) && self.get_index(i64::MAX).is_some()
    }

    /// Appends under the next integer key, or hands `value` back untouched
    /// where [`Table::next_is_occupied`] says there is no next key to use.
    ///
    /// A packed array whose counter is its own length — every list that has
    /// not had an entry removed — pushes, and that is the path with no key
    /// rendering, no allocation and no hashing at all.
    fn append(&mut self, value: Value) -> Result<(), Value> {
        if self.next_is_occupied() {
            return Err(value);
        }
        let counter = self.next_index.unwrap_or(0);
        if let Shape::Packed(values) = &mut self.shape
            && usize::try_from(counter).is_ok_and(|next| next == values.len())
        {
            values.push(value);
            self.next_index = Some(counter.saturating_add(1));
            return Ok(());
        }
        let key = NvsStr::new(counter.to_string().as_bytes());
        self.note_key(key.as_bytes());
        let displaced = self.hashed_mut().set(key, value);
        debug_assert!(
            displaced.is_none(),
            "the append counter never names a live key"
        );
        Ok(())
    }

    /// Room for `additional` more entries in whichever form is held, or
    /// `false` — [`NvsArray::try_reserve`] owns what the answer promises.
    fn try_reserve(&mut self, additional: usize) -> bool {
        match &mut self.shape {
            Shape::Packed(values) => values.try_reserve(additional).is_ok(),
            Shape::Hashed(hashed) => hashed.try_reserve(additional),
        }
    }

    /// The bytes one more entry asks the allocator for — the ask
    /// [`crate::budget::affords`] is given in front of a write, and `0` while
    /// the storage it lands in already has room for it.
    ///
    /// Both stores an insert grows are counted, because the hash form grows
    /// the order and the index together. Each doubles, so one that is full
    /// moves the balance by what it already holds while one with room moves it
    /// by nothing: the difference between two sizes that `StrWriter::grow`
    /// asks against, rather than the whole store.
    ///
    /// A write landing on a key already present is priced as the insert it
    /// might be. Telling the two apart costs the lookup the write is about to
    /// make anyway, and the whole difference between the two answers is one
    /// growth.
    fn growth_cost(&self) -> usize {
        match &self.shape {
            Shape::Packed(values) => amortized(values.len(), values.capacity(), size_of::<Value>()),
            Shape::Hashed(hashed) => amortized(
                hashed.entries.len(),
                hashed.entries.capacity(),
                size_of::<Option<Entry>>(),
            )
            .saturating_add(amortized(
                hashed.index.len(),
                hashed.index.capacity(),
                INDEX_BUCKET,
            )),
        }
    }

    /// The bytes a copy-on-write separation of this table asks for: the live
    /// entries and no room past them, which is what [`Table::separate`]
    /// reserves.
    fn separation_cost(&self) -> usize {
        match &self.shape {
            Shape::Packed(values) => values.len().saturating_mul(size_of::<Value>()),
            Shape::Hashed(hashed) => hashed
                .live
                .saturating_mul(size_of::<Option<Entry>>() + INDEX_BUCKET),
        }
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

    /// One past the last slot, holes included — the range [`Table::next_slot`]
    /// walks.
    fn slot_end(&self) -> usize {
        match &self.shape {
            Shape::Packed(values) => values.len(),
            Shape::Hashed(hashed) => hashed.entries.len(),
        }
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
    fn key_at(&self, slot: usize) -> Option<NvsStr> {
        match &self.shape {
            Shape::Packed(values) => {
                (slot < values.len()).then(|| NvsStr::new(slot.to_string().as_bytes()))
            }
            Shape::Hashed(hashed) => hashed.at(slot).map(|entry| entry.key.clone()),
        }
    }

    /// The key at `slot` in whichever form the shape already holds it, so
    /// **nothing is rendered and nothing is allocated** while the array is
    /// packed — [`SlotKey`] says which callers that is for.
    fn slot_key(&self, slot: usize) -> Option<SlotKey> {
        match &self.shape {
            Shape::Packed(values) => (slot < values.len()).then(|| {
                SlotKey::Index(
                    i64::try_from(slot).expect("a packed array is bounded by isize::MAX"),
                )
            }),
            Shape::Hashed(hashed) => hashed.at(slot).map(|entry| SlotKey::Str(entry.key.clone())),
        }
    }

    /// The value at `slot`, borrowed rather than retained.
    fn value_at(&self, slot: usize) -> Option<Value> {
        match &self.shape {
            Shape::Packed(values) => values.get(slot).copied(),
            Shape::Hashed(hashed) => hashed.at(slot).map(|entry| entry.value),
        }
    }

    /// Puts `value` at the live `slot` in place of the value there, and gives
    /// back the value it replaced for the caller to release. The key, the
    /// order and the shape do not change.
    fn replace_at(&mut self, slot: usize, value: Value) -> Value {
        let stored = match &mut self.shape {
            Shape::Packed(values) => values.get_mut(slot),
            Shape::Hashed(hashed) => hashed
                .entries
                .get_mut(slot)
                .and_then(Option::as_mut)
                .map(|entry| &mut entry.value),
        };
        std::mem::replace(stored.expect("replace_at is only given a live slot"), value)
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
    /// ordinary string owning no Novis value, so dropping the storage frees the
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
    /// `nvs_ir::ir::InstKind::FieldGet` reads a property.
    fn get(&self, key: &[u8]) -> Option<Value> {
        let slot = *self.index.get(key)?;
        self.entries[slot].as_ref().map(|entry| entry.value)
    }

    /// Inserts or overwrites, taking over `key`'s and `value`'s references and
    /// handing back whatever it displaced for the caller to release. The
    /// append counter is [`Table`]'s, and is already up to date by here.
    fn set(&mut self, key: NvsStr, value: Value) -> Option<Value> {
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

    /// Room for `additional` more entries in both of the vectors an insertion
    /// grows — the order and the index — or `false` from whichever refused.
    fn try_reserve(&mut self, additional: usize) -> bool {
        self.entries.try_reserve(additional).is_ok() && self.index.try_reserve(additional).is_ok()
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

    /// Gives the entry at `from` the key `to`, at the same position, and
    /// answers whether it did. `to` must not be present already.
    fn rename(&mut self, from: &[u8], to: NvsStr) -> bool {
        let Some(slot) = self.index.remove(from) else {
            return false;
        };
        self.index.insert(to.clone(), slot);
        let entry = self.entries[slot]
            .as_mut()
            .expect("an indexed slot is always live");
        drop(std::mem::replace(&mut entry.key, to));
        true
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
    /// [AGENTS.md](/AGENTS.md)'s memory section asks for. Every
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

/// A key as the array's own shape holds it — [`NvsArray::slot_key`]'s answer.
///
/// [`NvsArray::key_at`] answers every key as an [`NvsStr`], which means a
/// packed list renders a decimal and allocates one per entry. A caller that
/// only means to *store* the entry under the same key, or to pass the key on
/// to a callback that may not want it, does not need that string built: the
/// `Index` arm carries the position itself, and [`NvsArray::set_index`] takes
/// it without rendering anything. `docs/perf/userland-gap.md` § D is the
/// measurement, and `Core\Arr`'s `map`/`filter`/`reduce` are the callers.
///
/// The `Str` arm carries a reference the receiver owns, exactly as `key_at`
/// does — dropping it releases.
#[derive(Debug, Clone)]
pub enum SlotKey {
    /// A packed array's key *is* its position; nothing was allocated to
    /// answer, and nothing has to be freed.
    Index(i64),
    /// A hashed array already holds its key as a string, so this is a
    /// reference to the one it holds and not a fresh rendering.
    Str(NvsStr),
}

impl SlotKey {
    /// The key as a string, rendering the decimal a packed array does not
    /// hold — the allocation § D exists to avoid making where nothing asks
    /// for one, paid here where something does.
    #[must_use]
    pub fn to_str(&self) -> NvsStr {
        match self {
            Self::Index(index) => NvsStr::new(index.to_string().as_bytes()),
            Self::Str(key) => key.clone(),
        }
    }
}

/// An owning handle to one reference of an Novis array.
///
/// Cloning retains, dropping releases, and a mutator separates first when the
/// handle is not the only owner — so Rust-side code (helpers, tests, and
/// `nvs-stdlib`'s `Core\Arr`) never writes the copy-on-write protocol by hand.
/// Compiled code instead calls the primitives at the bottom of this file,
/// which are the same operations with the ownership left implicit.
///
/// Neither `Send` nor `Sync`, by construction — see [`crate::string`]'s own
/// docs for the reasoning, which is identical here.
#[repr(transparent)]
pub struct NvsArray {
    ptr: NonNull<ArrayHeader>,
}

impl NvsArray {
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

    /// Converts to the hash form without changing a single answer — how the
    /// equivalence test produces the same content in the other shape, and the
    /// only caller of [`Table::hashed_mut`] that is not a broken invariant.
    #[cfg(test)]
    fn degrade(&mut self) {
        self.make_unique();
        self.header().table.borrow_mut().hashed_mut();
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

    /// Whether the running request can afford one write here — the copy this
    /// handle separates into where it shares its storage, and the growth that
    /// storage takes where it is full.
    ///
    /// `rule:errors/on-limit`'s memory ceiling asked in front of the write
    /// rather than behind it: [`crate::budget::add`] compares once the block is
    /// already held, which bounds a *loop* of writes and cannot bound one of
    /// them. A `false` has already recorded the breach and asked for the poll
    /// that reports it, so what the caller owes is an operation that allocates
    /// nothing and an answer that costs nothing — for [`nvs_array_set`] and
    /// [`nvs_array_set_index`], the array unchanged. A `Core` member that
    /// appends in a loop over its input asks it in front of every append, and
    /// stops with a fatal fault on the first `false`.
    ///
    /// A shared handle is priced at the copy **and** one growth of it, because
    /// [`Table::separate`] reserves room for the live entries exactly, so the
    /// write that follows grows what the separation just made. An unshared one
    /// is priced at the growth alone, which is the whole of what the common
    /// write allocates.
    ///
    /// The ask is made whatever it costs, zero included: a balance already
    /// past the ceiling is a request that is over, and a write it makes after
    /// that is one it is not entitled to however little it would take.
    #[must_use]
    pub fn affords_write(&self) -> bool {
        let table = self.header().table.borrow();
        let ask = if self.refcount() == 1 {
            table.growth_cost()
        } else {
            size_of::<ArrayHeader>().saturating_add(table.separation_cost().saturating_mul(2))
        };
        crate::budget::affords(ask)
    }

    /// Writes `value` at `key`, taking over both references and separating
    /// first if this handle is not the only owner.
    pub fn set(&mut self, key: NvsStr, value: Value) {
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

    /// The value stored at the integer key `index`, borrowed rather than
    /// retained, and reached with no key string built at all while the array
    /// is a list — [`nvs_array_get_index`].
    #[must_use]
    pub fn get_index(&self, index: i64) -> Option<Value> {
        self.header().table.borrow().get_index(index)
    }

    /// Writes `value` at the integer key `index`, taking over its reference,
    /// separating first if this handle is not the only owner, and building no
    /// key string while the array is a list — [`nvs_array_set_index`].
    pub fn set_index(&mut self, index: i64, value: Value) {
        self.make_unique();
        let displaced = self.header().table.borrow_mut().set_index(index, value);
        if let Some(old) = displaced {
            #[expect(
                unsafe_code,
                reason = "the table owned exactly one reference to the value \
                          this write displaced, and no longer holds it"
            )]
            unsafe {
                crate::release::release_value(old);
            }
        }
    }

    /// Appends `value` under the next integer key, taking over its reference
    /// and separating first if this handle is not the only owner, or hands the
    /// reference back where the next integer key is already live.
    ///
    /// The occupancy test runs **before** the copy-on-write separation, so a
    /// refusal leaves this handle's allocation — and therefore the raw pointer
    /// its caller holds — exactly as it was. That is what lets
    /// [`nvs_array_append`] report PHP's refusal without its caller having to
    /// re-point anything: the reference the call was given is still the one the
    /// caller's slot names. See this module's *the append is the one array
    /// write with a fault channel*.
    ///
    /// # Errors
    ///
    /// `value`, unappended and with its reference still owed to the caller,
    /// where [`Table::next_is_occupied`].
    pub fn try_append(&mut self, value: Value) -> Result<(), Value> {
        if self.header().table.borrow().next_is_occupied() {
            return Err(value);
        }
        self.make_unique();
        self.header().table.borrow_mut().append(value)
    }

    /// Appends `value` under the next integer key, for an array **this call is
    /// building** — every `Core` producer's `out.append(…)`.
    ///
    /// # Panics
    ///
    /// Where the next integer key is already live, which an array filled from
    /// index 0 by the caller cannot reach. Compiled `$a[] = …` runs over an
    /// array the *program* supplied and can, so it goes through
    /// [`Self::try_append`] and reports PHP's refusal instead.
    pub fn append(&mut self, value: Value) {
        assert!(
            self.try_append(value).is_ok(),
            "an array a producer built from index 0 cannot have its next integer key occupied"
        );
    }

    /// Room for `additional` more entries, answering `false` where the
    /// allocator refuses rather than aborting.
    ///
    /// The fallible seam for a producer whose entry count is a **count off a
    /// call site** — `Core\Arr::fill`'s is the whole of its first argument.
    /// [`crate::affordable`] refuses a size past `isize::MAX` and one past the
    /// running request's remaining budget, so what still reaches the allocator
    /// is an uncapped request's every count between those and what the machine
    /// can actually serve — and an allocator that refuses inside [`Vec::push`]
    /// is an abort: the process and every in-flight request with it, for a
    /// refusal the caller may well want to handle. This is
    /// [`NvsStr::try_build`](crate::NvsStr::try_build)'s bargain over the entry
    /// storage instead of over a payload.
    ///
    /// **What it makes infallible is the entry storage's growth and nothing
    /// else.** A packed array's append allocates nothing but that, so
    /// `additional` appends into a reserved list cannot abort; the hash form's
    /// append also renders and allocates a key string, which this does not
    /// cover, so a caller that needs the guarantee appends into a list.
    #[must_use]
    pub fn try_reserve(&mut self, additional: usize) -> bool {
        self.make_unique();
        self.header().table.borrow_mut().try_reserve(additional)
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

    /// Gives the entry at `from` the key `to` and keeps its position in the
    /// order, separating first if this handle is not the only owner. Answers
    /// `false` and changes nothing where `from` is absent or `to` is present.
    ///
    /// A list-shaped array converts to the hash form first, the same as any
    /// write that cannot keep its keys `0`…`n−1`.
    pub fn rename(&mut self, from: &[u8], to: NvsStr) -> bool {
        if !self.has_key(from) || self.has_key(to.as_bytes()) {
            return false;
        }
        self.make_unique();
        let mut table = self.header().table.borrow_mut();
        table.note_key(to.as_bytes());
        table.hashed_mut().rename(from, to)
    }

    /// One past the last slot, holes included. Once it passes eight, at least
    /// half of the slots below it are live — the compaction this module's docs
    /// describe — so a uniform draw over `0..slot_end()` that draws again on a
    /// hole takes two tries on average and never walks the array.
    #[must_use]
    pub fn slot_end(&self) -> usize {
        self.header().table.borrow().slot_end()
    }

    /// The position of the first live entry at or after `from` — the cursor
    /// `foreach` advances.
    #[must_use]
    pub fn next_slot(&self, from: usize) -> Option<usize> {
        self.header().table.borrow().next_slot(from)
    }

    /// The key at `slot`, as a fresh reference the caller owns.
    #[must_use]
    pub fn key_at(&self, slot: usize) -> Option<NvsStr> {
        self.header().table.borrow().key_at(slot)
    }

    /// The key at `slot` in whichever form the array already holds it,
    /// allocating nothing while the array is a list — [`SlotKey`].
    #[must_use]
    pub fn slot_key(&self, slot: usize) -> Option<SlotKey> {
        self.header().table.borrow().slot_key(slot)
    }

    /// The value at `slot`, borrowed rather than retained.
    #[must_use]
    pub fn value_at(&self, slot: usize) -> Option<Value> {
        self.header().table.borrow().value_at(slot)
    }

    /// A new array with the same keys in the same order, whose values are
    /// what `convert` gives for each value of this one. This is the copy
    /// `array<int> as array<float>` makes, and it costs one array of this
    /// one's size.
    ///
    /// `convert` is given each value borrowed. `Ok(Some(v))` stores `v`, whose
    /// reference the copy takes over, and `Ok(None)` keeps the value as it is.
    /// An `Err` stops the walk, frees the copy and is returned.
    ///
    /// `Ok(None)` from this method means the running request cannot afford
    /// the copy. [`crate::budget::affords`] has already recorded that, so the
    /// caller allocates nothing more.
    pub(crate) fn map_values<E>(
        &self,
        mut convert: impl FnMut(Value) -> Result<Option<Value>, E>,
    ) -> Result<Option<Self>, E> {
        let cost = {
            let table = self.header().table.borrow();
            size_of::<ArrayHeader>().saturating_add(table.separation_cost())
        };
        if !crate::budget::affords(cost) {
            return Ok(None);
        }
        let copy = Self::new();
        *copy.header().table.borrow_mut() = self.header().table.borrow().separate();
        let mut from = 0;
        while let Some(slot) = copy.next_slot(from) {
            let value = copy
                .value_at(slot)
                .expect("next_slot only names live entries");
            if let Some(converted) = convert(value)? {
                let old = copy.header().table.borrow_mut().replace_at(slot, converted);
                #[expect(
                    unsafe_code,
                    reason = "the copy owned exactly one reference to the value it \
                              replaced, and no longer holds it"
                )]
                unsafe {
                    crate::release::release_value(old);
                }
            }
            from = slot + 1;
        }
        Ok(Some(copy))
    }

    /// Every key in insertion order — `Core\Arr::keys`, and what a test reads
    /// to assert `rule:types/arrays`'s ordering.
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
            nvs_array_release(old.as_ptr());
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
    /// pointer to [`nvs_array_release`] or [`NvsArray::from_raw`].
    #[must_use]
    pub fn into_raw(self) -> *mut ArrayHeader {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }

    /// Reclaims a reference previously given up by [`NvsArray::into_raw`].
    ///
    /// # Safety
    ///
    /// `ptr` must be a pointer produced by [`NvsArray::into_raw`] (or by
    /// [`nvs_array_new`]) whose reference has not already been released, and
    /// it must not be reclaimed twice.
    ///
    /// # Panics
    ///
    /// If `ptr` is null, which no Novis array pointer ever is.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "reclaiming a reference is the caller's obligation to state"
    )]
    pub unsafe fn from_raw(ptr: *mut ArrayHeader) -> Self {
        Self {
            ptr: NonNull::new(ptr).expect("an Novis array pointer is never null"),
        }
    }

    /// How many owners hold the array `ptr` refers to.
    ///
    /// # Safety
    ///
    /// `ptr` must refer to a live Novis array allocation.
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

impl Default for NvsArray {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for NvsArray {
    fn clone(&self) -> Self {
        bump(self.ptr.as_ptr());
        Self { ptr: self.ptr }
    }
}

impl Drop for NvsArray {
    fn drop(&mut self) {
        #[expect(
            unsafe_code,
            reason = "this handle owns exactly the reference being dropped"
        )]
        unsafe {
            nvs_array_release(self.ptr.as_ptr());
        }
    }
}

impl fmt::Debug for NvsArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NvsArray")
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
            .expect("an Novis array's reference count cannot overflow a usize"),
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
/// `ptr` must refer to a live Novis array allocation whose reference the caller
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
/// Each entry's *key* is an ordinary string, which owns no Novis value, so
/// dropping the entry vector frees the keys directly.
///
/// # Safety
///
/// `ptr` must refer to an Novis array allocation whose reference count reached
/// zero in [`drop_one`], and must be dismantled exactly once.
#[expect(
    unsafe_code,
    reason = "reaching zero exactly once is the caller's obligation to state"
)]
pub(crate) unsafe fn dismantle(ptr: *mut ArrayHeader, work: &mut Vec<crate::release::Dying>) {
    #[expect(
        unsafe_code,
        reason = "the count reached zero, so nothing else can observe the \
                  allocation; it was produced by `Box::leak` in `NvsArray::new`"
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

/// Every value the array at `ptr` holds, in slot order, **borrowed rather than
/// retained** — [`crate::object::sweep`]'s reader for the edges an element
/// carries.
///
/// The values are copied out of the table rather than read under its borrow so
/// that the caller may walk into whatever they point at without holding a
/// `RefCell` guard across the walk. Nothing here changes a reference count, so
/// a returned `Value` is live only as long as the array is — which is the whole
/// of the sweep, since it runs with no user code in flight.
///
/// # Safety
///
/// `ptr` must refer to a live Novis array allocation, and the caller must not
/// release any value this hands back.
#[must_use]
#[expect(
    unsafe_code,
    reason = "the pointee's liveness is the caller's obligation to state"
)]
pub(crate) unsafe fn borrowed_values(ptr: *mut ArrayHeader) -> Vec<Value> {
    #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
    // `ManuallyDrop`, because `from_raw` reclaims a reference this borrow does
    // not own: dropping the handle would release the array under its holder.
    let handle = std::mem::ManuallyDrop::new(unsafe { NvsArray::from_raw(ptr) });
    let table = handle.header().table.borrow();
    let mut values = Vec::with_capacity(table.len());
    let mut slot = 0;
    while let Some(live) = table.next_slot(slot) {
        values.push(
            table
                .value_at(live)
                .expect("next_slot only names live entries"),
        );
        slot = live + 1;
    }
    values
}

// ---------------------------------------------------------------------------
// The primitives compiled code calls
// ---------------------------------------------------------------------------
//
// The same split [`crate::string`]'s own primitives are on, for the same
// reason: none of these can fail, so none of them wears `rule:errors/propagation`'s
// checked-return shape. Every one is `extern "C"` and never
// `extern "C-unwind"`.
//
// **A `Value` crosses this boundary through a pointer, never by value.** A
// 16-byte struct is classified differently by the SysV and Windows x64 ABIs —
// two integer registers on one, a hidden pointer on the other — and
// `nvs-codegen` would have to encode that difference to call these at all.
// Every site that needs one therefore passes the address of a 16-byte slot the
// caller owns, which is exactly what `rule:errors/propagation`'s own `(ctx, args, out)` helper
// shape already does, so codegen reuses `store_value`/`load_value` unchanged.
//
// Every mutator consumes one reference to its `array` argument and returns
// one — see this module's copy-on-write decision, which is the whole reason
// the signatures are shaped that way rather than returning nothing.

thread_local! {
    /// This thread's empty array — the module docs § *an empty array is a
    /// per-thread singleton*. Null until the thread's first [`nvs_array_new`],
    /// and the one reference this slot owns is never given up.
    ///
    /// `const`-initialized and holding no `Drop` type, per
    /// [`crate::alloc`]'s § *The one trap*: a lazily-initialized thread local
    /// allocates its own state and one with a destructor registers that
    /// destructor, both from inside the allocator this crate installs.
    static EMPTY: Cell<*mut ArrayHeader> = const { Cell::new(std::ptr::null_mut()) };
}

/// An empty array with one more reference than it had —
/// `nvs_ir::InstKind::ArrayNew`'s allocation half, which allocates only the
/// first time a thread asks.
///
/// The array is the thread's singleton, so this is a refcount bump rather than
/// a `Box`; the module docs § *an empty array is a per-thread singleton* own
/// why nothing can observe the sharing. It is also the **only** producer that
/// hands the singleton out — [`NvsArray::new`] still allocates, for the callers
/// that mean to write through the handle they get.
///
/// The one primitive here that is safe to call: it reads no pointer the caller
/// supplied, because it takes none.
#[expect(
    unsafe_code,
    reason = "exporting an unmangled symbol is what makes compiled code able \
              to call this at all"
)]
#[unsafe(no_mangle)]
pub extern "C" fn nvs_array_new() -> *mut ArrayHeader {
    EMPTY.with(|slot| {
        let mut ptr = slot.get();
        if ptr.is_null() {
            // The slot's own reference, which outlives every caller's.
            ptr = NvsArray::new().into_raw();
            slot.set(ptr);
        }
        bump(ptr);
        ptr
    })
}

/// Allocates this thread's empty-array singleton if nothing has yet, so a test
/// measuring an allocation balance can open its window after it.
///
/// One header per thread is neither a leak nor a frame's local, but a
/// [`crate::budget::live_bytes`] balance taken across the thread's *first* `[]`
/// cannot tell it from one — the module docs § *an empty array is a per-thread
/// singleton* own why it is there. Every later `[]` on that thread allocates
/// nothing at all, which is what makes one call at the top of a measured run
/// enough.
pub fn prime_empty_array() {
    #[expect(
        unsafe_code,
        reason = "this line owns exactly the reference it gives back"
    )]
    unsafe {
        nvs_array_release(nvs_array_new());
    }
}

/// Adds a reference — `nvs_ir::InstKind::Retain` for a `Ty::Array` operand.
///
/// # Safety
///
/// `ptr` must refer to a live Novis array allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw array pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_retain(ptr: *mut ArrayHeader) {
    if ptr.is_null() {
        return;
    }
    bump(ptr);
}

/// Drops a reference, freeing the array and everything it solely owns if it
/// was the last — `nvs_ir::InstKind::Release` for a `Ty::Array` operand.
///
/// # Safety
///
/// `ptr` must refer to a live Novis array allocation whose reference this caller
/// owns, and must not be released twice.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw array pointer whose ownership the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_release(ptr: *mut ArrayHeader) {
    if ptr.is_null() {
        return;
    }
    #[expect(unsafe_code, reason = "the caller guarantees it owns the reference")]
    unsafe {
        crate::release::release_value(Value::from_array_ptr(ptr));
    }
}

/// The entry at `key`, or `None` when the key is absent — the one lookup
/// every read in this module is built out of, and the only one that tells an
/// absent key from a stored `null`.
///
/// # Safety
///
/// `array` must refer to a live Novis array allocation.
#[expect(
    unsafe_code,
    reason = "the pointee's liveness is the caller's obligation to state"
)]
pub(crate) unsafe fn entry(array: *mut ArrayHeader, key: &[u8]) -> Option<Value> {
    #[expect(unsafe_code, reason = "the caller guarantees the array is live")]
    unsafe {
        (*array).table.borrow().get(key)
    }
}

/// [`entry`] for an integer key, answered straight out of a list-shaped
/// array's `Vec<Value>` — see [`nvs_array_get_index`] for why that path
/// exists.
///
/// # Safety
///
/// `array` must refer to a live Novis array allocation.
#[expect(
    unsafe_code,
    reason = "the pointee's liveness is the caller's obligation to state"
)]
pub(crate) unsafe fn entry_at_index(array: *mut ArrayHeader, index: i64) -> Option<Value> {
    #[expect(unsafe_code, reason = "the caller guarantees the array is live")]
    unsafe {
        (*array).table.borrow().get_index(index)
    }
}

/// Reads the entry at `key` **without** retaining what it holds, answering a
/// missing key with `null`.
///
/// This is the *vivifying* read — the one `nvs_runtime::helpers::
/// nvs_array_row_for_write` is built on, where an absent key means "build the
/// row PHP would have built". A read written in source goes through
/// `nvs_array_required_get` instead, which throws there
/// (`nvs_ir::InstKind::ArrayGet`). Neither the array nor the key is consumed,
/// and the value written to `out` is *borrowed*: the array keeps its
/// reference, so a caller that stores the result retains it itself.
///
/// # Safety
///
/// `array` must refer to a live Novis array allocation, `key` to a live Novis
/// string allocation, and `out` to a writable, aligned 16-byte slot.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw pointers whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_get(
    array: *mut ArrayHeader,
    key: *const StrHeader,
    out: *mut Value,
) {
    #[expect(unsafe_code, reason = "the caller guarantees every pointee is live")]
    unsafe {
        let bytes = NvsStr::bytes_of(key);
        out.write(entry(array, bytes).unwrap_or_default());
    }
}

/// Reads the entry at the integer key `index`, **without** retaining what it
/// holds and **without rendering a key** — the `int`/`uint` half of
/// `nvs_ir::InstKind::ArrayGet`.
///
/// Semantically identical to [`nvs_array_get`] called with `index`'s decimal
/// form: `$a[8]` is `$a["8"]` (`rule:types/arrays`), and a negative index names the
/// key `"-1"` exactly as its decimal form does. What differs is that a list-shaped
/// array answers straight out of its `Vec<Value>` — no decimal, no `NvsStr`,
/// no hash — which is the saving this module's packed decision exists for.
/// Neither the array nor the index is consumed, and the value written to `out`
/// is *borrowed*.
///
/// # Safety
///
/// `array` must refer to a live Novis array allocation and `out` to a writable,
/// aligned 16-byte slot.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw pointers whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_get_index(array: *mut ArrayHeader, index: i64, out: *mut Value) {
    #[expect(unsafe_code, reason = "the caller guarantees every pointee is live")]
    unsafe {
        out.write(entry_at_index(array, index).unwrap_or_default());
    }
}

/// Whether `key` is present — `Core\Arr::hasKey`'s entry point. Consumes
/// nothing.
///
/// # Safety
///
/// `array` must refer to a live Novis array allocation and `key` to a live Novis
/// string allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes two raw pointers whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_has_key(array: *mut ArrayHeader, key: *const StrHeader) -> bool {
    #[expect(unsafe_code, reason = "the caller guarantees both pointees are live")]
    unsafe {
        let bytes = NvsStr::bytes_of(key);
        (*array).table.borrow().get(bytes).is_some()
    }
}

/// Writes `value` at `key` — `nvs_ir::InstKind::ArraySet`.
///
/// Consumes one reference to `array`, one to `key` and one to `value`, and
/// returns the one reference to the array that now holds the entry: the same
/// pointer when `array` was uniquely owned, a separated copy otherwise.
///
/// Where [`NvsArray::affords_write`] refuses, `array` comes back exactly as it
/// arrived and the other two references are released — the degenerate answer a
/// primitive with no status channel has in place of one. The refusal is a
/// **complete no-op**, the separation included: a copy that was made but not
/// written, or a write into the original a `foreach` is still walking, is the
/// one way a refused write reaches [`nvs_array_value_at`]'s live-entry
/// expectation.
///
/// # Safety
///
/// `array` must refer to a live Novis array allocation whose reference the
/// caller owns, `key` to a live Novis string allocation whose reference the
/// caller owns, and `value` must own the reference it transfers.
#[expect(
    unsafe_code,
    reason = "compiled code passes raw pointers whose ownership the signature \
              cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_set(
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
        let mut handle = NvsArray::from_raw(array);
        if !handle.affords_write() {
            drop(NvsStr::from_raw(key));
            crate::release::release_value(value.read());
            return handle.into_raw();
        }
        handle.set(NvsStr::from_raw(key), value.read());
        handle.into_raw()
    }
}

/// Writes `value` at the integer key `index` — the `int`/`uint` half of
/// `nvs_ir::InstKind::ArraySet`, building no key string while the array is a
/// list.
///
/// Semantically identical to [`nvs_array_set`] called with `index`'s decimal
/// form, the append counter included: a write at `8` still makes the next
/// `$a[]` land at `9`. Consumes one reference to `array` and one to `value`,
/// and returns the one reference to the array that now holds the entry — see
/// [`nvs_array_set`]. The index owns nothing, so nothing about it is consumed.
///
/// A refused write answers the same way [`nvs_array_set`]'s does — the array
/// unchanged, `value`'s reference released, and neither a growth nor a
/// separation taken.
///
/// # Safety
///
/// `array` must refer to a live Novis array allocation whose reference the
/// caller owns, and `value` must own the reference it transfers.
#[expect(
    unsafe_code,
    reason = "compiled code passes raw pointers whose ownership the signature \
              cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_set_index(
    array: *mut ArrayHeader,
    index: i64,
    value: *const Value,
) -> *mut ArrayHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees it owns one reference to `array` and \
                  that `value` points at a readable 16-byte slot"
    )]
    unsafe {
        let mut handle = NvsArray::from_raw(array);
        if !handle.affords_write() {
            crate::release::release_value(value.read());
            return handle.into_raw();
        }
        handle.set_index(index, value.read());
        handle.into_raw()
    }
}

/// Appends `value` under the next integer key —
/// `nvs_ir::InstKind::ArrayAppend`, `$a[] = expr`.
///
/// Consumes one reference to `array` and one to `value`, and writes into `out`
/// the one reference to the array that now holds the entry — see
/// [`nvs_array_set`], whose protocol this shares apart from where the array
/// comes back.
///
/// Answers [`crate::OK`], or [`crate::THROWN`] where the next integer key is
/// already live: PHP 8.5's *"Cannot add element to the array as the next
/// element is already occupied"*, raised as
/// [`ThrownClass::Logic`](crate::ThrownClass::Logic) because that is spec
/// § 10's class for a program that asked for something its own state forbids.
/// **On that refusal `array` is written to `out` unchanged and `value`'s
/// reference is released**, so the pointer the caller already holds stays live
/// and stays owned and there is nothing for the error path to re-point — see
/// this module's *the append is the one array write with a fault channel* for
/// why the signature is this shape at all.
///
/// Or [`crate::FATAL`] where [`NvsArray::affords_write`] refuses the entry: the
/// same unchanged array and the same released reference, reported through the
/// status this signature already carries rather than as a degenerate value.
/// That is what [`nvs_array_set`] cannot do and why the two answer a ceiling
/// differently.
///
/// # Safety
///
/// `ctx` must refer to the live [`Ctx`](crate::Ctx) of the request this call
/// runs inside, `array` to a live Novis array allocation whose reference the
/// caller owns, `value` must own the reference it transfers, and `out` must
/// point at a writable pointer-wide slot.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw pointer and a value whose ownership \
              the signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_append(
    ctx: *mut crate::Ctx,
    array: *mut ArrayHeader,
    value: *const Value,
    out: *mut *mut ArrayHeader,
) -> i32 {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees it owns one reference to each argument, \
                  that `value` points at a readable 16-byte slot, and that \
                  `ctx` and `out` are live and writable"
    )]
    unsafe {
        let mut handle = NvsArray::from_raw(array);
        if !handle.affords_write() {
            crate::release::release_value(value.read());
            out.write(handle.into_raw());
            return crate::abi::report_refusal(&mut *ctx);
        }
        let outcome = handle.try_append(value.read());
        out.write(handle.into_raw());
        match outcome {
            Ok(()) => crate::OK,
            Err(refused) => {
                crate::release::release_value(refused);
                (*ctx).set_pending_as(
                    crate::ThrownClass::Logic,
                    "Cannot add element to the array as the next element is already occupied",
                );
                crate::THROWN
            }
        }
    }
}

/// Copies every entry of `subject` into `array` — `nvs_ir::InstKind::ArraySpread`,
/// the `[...$a]` array-literal element.
///
/// Consumes one reference to `array` and **borrows** `subject`, writing into
/// `out` the one reference to the array that now holds the entries. Every
/// value copied is **retained** before it is stored, because the entry is now
/// held by two arrays; the caller emits no retain of its own beside this.
///
/// **Which key survives is `rule:types/arrays`'s rule, not a representation question.** A key that reads as a
/// canonical decimal integer is *renumbered* — appended under this array's own
/// counter — and every other key is preserved, overwriting an entry already
/// there in place. That is PHP's own spread, expressed in the two writes Novis
/// already has, and [`Table::note_key`] is where the identical predicate
/// already decides where a later `$a[]` lands. [`SlotKey::Index`] is therefore
/// not the test: it says the subject is *packed*, which every list is and no
/// hashed array is, so a hashed `"5"` takes the same renumbering by way of
/// [`integer_key`].
///
/// Answers [`crate::OK`], or [`crate::THROWN`] with `array` written to `out`
/// holding whatever it had copied so far — the append it stopped at is an
/// ordinary [`nvs_array_append`] refusal, and this shares that whole fault
/// channel because it shares the write. See this module's *the append is the
/// one array write with a fault channel*.
///
/// Or [`crate::FATAL`] where [`NvsArray::affords_write`] refuses an entry, which
/// is asked per entry rather than for the spread as a whole: a subject large
/// enough to cross a ceiling crosses it partway through, and stopping there is
/// what bounds the copy instead of the loop around it. The entries already
/// copied stay in the array written to `out`, which the error path this status
/// branches to is the only reader of.
///
/// # Safety
///
/// `ctx` must refer to the live [`Ctx`](crate::Ctx) of the request this call
/// runs inside, `array` and `subject` to live Novis array allocations, the
/// caller owning `array`'s reference and holding `subject` live for the call,
/// and `out` must point at a writable pointer-wide slot.
#[expect(
    unsafe_code,
    reason = "compiled code passes raw pointers whose ownership the signature \
              cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_spread(
    ctx: *mut crate::Ctx,
    array: *mut ArrayHeader,
    subject: *mut ArrayHeader,
    out: *mut *mut ArrayHeader,
) -> i32 {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees one owned reference to `array`, a live \
                  `subject`, and that `ctx` and `out` are live and writable"
    )]
    unsafe {
        let mut handle = NvsArray::from_raw(array);
        // The subject's reference belongs to whoever lowered it — the
        // literal's element list borrows it, exactly as a call argument
        // borrows a receiver — so this handle must not run its own drop.
        let source = std::mem::ManuallyDrop::new(NvsArray::from_raw(subject));
        // Every read below takes its own scoped borrow of the subject's
        // table, so a write into `handle` never overlaps one. They are two
        // arrays in any case: the destination is the literal under
        // construction and nothing else can name it yet.
        let mut refused = None;
        let mut over_ceiling = false;
        let mut from = 0;
        while let Some(slot) = source.next_slot(from) {
            from = slot + 1;
            // Asked once per entry and ahead of the retain, so a spread of a
            // large subject is bounded entry by entry rather than as the one
            // operation a program wrote. What the destination already took is
            // left in it: the literal under construction is reached by nothing
            // but the error path this `FATAL` branches to.
            if !handle.affords_write() {
                over_ceiling = true;
                break;
            }
            let value = source.value_at(slot).expect("next_slot names a live entry");
            let key = source.slot_key(slot).expect("next_slot names a live entry");
            value.retain();
            let renumbered = match &key {
                SlotKey::Index(_) => true,
                SlotKey::Str(key) => integer_key(key.as_bytes()).is_some(),
            };
            if renumbered {
                if let Err(value) = handle.try_append(value) {
                    refused = Some(value);
                    break;
                }
            } else {
                let SlotKey::Str(key) = key else {
                    unreachable!("an index key is renumbered above");
                };
                handle.set(key, value);
            }
        }
        out.write(handle.into_raw());
        if over_ceiling {
            return crate::abi::report_refusal(&mut *ctx);
        }
        match refused {
            None => crate::OK,
            Some(value) => {
                crate::release::release_value(value);
                (*ctx).set_pending_as(
                    crate::ThrownClass::Logic,
                    "Cannot add element to the array as the next element is already occupied",
                );
                crate::THROWN
            }
        }
    }
}

/// Removes `key` if present — `unset($a[$k])`.
///
/// Consumes one reference to `array` and returns the one reference to the
/// array without the entry — see [`nvs_array_set`]. `key` is borrowed, not
/// consumed: nothing is stored, so nothing needs to be owned.
///
/// # Safety
///
/// `array` must refer to a live Novis array allocation whose reference the
/// caller owns, and `key` to a live Novis string allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes raw pointers whose ownership the signature \
              cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_unset(
    array: *mut ArrayHeader,
    key: *const StrHeader,
) -> *mut ArrayHeader {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees it owns `array`'s reference and that \
                  `key` is live"
    )]
    unsafe {
        let mut handle = NvsArray::from_raw(array);
        let bytes = NvsStr::bytes_of(key).to_vec();
        handle.unset(&bytes);
        handle.into_raw()
    }
}

/// How many entries the array holds — `Core\Arr::count`'s entry point.
/// Consumes nothing.
///
/// # Safety
///
/// `array` must refer to a live Novis array allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw array pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_count(array: *const ArrayHeader) -> i64 {
    #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
    let live = unsafe { (*array).table.borrow().len() };
    i64::try_from(live).expect("an array cannot hold more than i64::MAX entries")
}

/// The position of the first live entry at or after `from`, or `-1` when there
/// is none — one `foreach` step over `rule:types/arrays`'s insertion order. Consumes
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
/// `array` must refer to a live Novis array allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw array pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_next_slot(array: *const ArrayHeader, from: usize) -> i64 {
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
/// `array` must refer to a live Novis array allocation, and `slot` must be a
/// position [`nvs_array_next_slot`] returned and nothing has removed since.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw array pointer and a slot the \
              signature cannot bound"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_key_at(
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
/// `array` must refer to a live Novis array allocation, `slot` must be a
/// position [`nvs_array_next_slot`] returned and nothing has removed since,
/// and `out` must be a writable, aligned 16-byte slot.
#[expect(
    unsafe_code,
    reason = "compiled code passes raw pointers and a slot the signature \
              cannot bound"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_array_value_at(
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
    use crate::counting_alloc::{allocated_bytes, live_bytes};

    fn key(text: &str) -> NvsStr {
        NvsStr::new(text.as_bytes())
    }

    fn keys_of(array: &NvsArray) -> Vec<String> {
        array
            .keys()
            .into_iter()
            .map(|k| String::from_utf8(k).expect("test keys are ASCII"))
            .collect()
    }

    /// One named mutation, and whether the packed handle is still packed once
    /// it has been applied.
    type Mutation = (&'static str, fn(&mut NvsArray), bool);

    /// A list of `count` values, built the way every list is: by appending.
    fn list_of(count: i64) -> NvsArray {
        let mut array = NvsArray::new();
        for value in 0..count {
            array.append(Value::int(value * 10));
        }
        array
    }

    /// Everything one `foreach` cursor can see, in a form two arrays' can be
    /// compared with — `Value` carries no `PartialEq`, so the ints stand in.
    fn readout(array: &NvsArray) -> Vec<(String, Option<i64>)> {
        let mut seen = Vec::new();
        let mut cursor = 0;
        while let Some(slot) = array.next_slot(cursor) {
            let key = array.key_at(slot).expect("the cursor names a live entry");
            seen.push((
                String::from_utf8(key.as_bytes().to_vec()).expect("test keys are ASCII"),
                array.value_at(slot).and_then(Value::as_int),
            ));
            cursor = slot + 1;
        }
        seen
    }

    /// The same walk through the `extern "C"` trio compiled code actually
    /// calls, rather than through the safe handle they share a table with.
    fn raw_readout(array: &NvsArray) -> Vec<(String, Option<i64>)> {
        let ptr: *const ArrayHeader = array.header();
        let mut seen = Vec::new();
        let mut cursor = 0;
        loop {
            #[expect(
                unsafe_code,
                reason = "the handle in `array` keeps the allocation live for \
                          this whole walk, and every slot came from \
                          `nvs_array_next_slot` itself"
            )]
            unsafe {
                let slot = nvs_array_next_slot(ptr, cursor);
                let Ok(slot) = usize::try_from(slot) else {
                    return seen;
                };
                let key = NvsStr::from_raw(nvs_array_key_at(ptr, slot));
                let mut value = Value::default();
                nvs_array_value_at(ptr, slot, &raw mut value);
                seen.push((
                    String::from_utf8(key.as_bytes().to_vec()).expect("test keys are ASCII"),
                    value.as_int(),
                ));
                cursor = slot + 1;
            }
        }
    }

    /// Asserts that two arrays holding the same content answer every primitive
    /// alike, whatever shape each is in.
    fn agree(packed: &NvsArray, hashed: &NvsArray, step: &str) {
        assert_eq!(packed.count(), hashed.count(), "count, after {step}");
        assert_eq!(keys_of(packed), keys_of(hashed), "keys, after {step}");
        assert_eq!(readout(packed), readout(hashed), "cursor, after {step}");
        assert_eq!(
            raw_readout(packed),
            raw_readout(hashed),
            "the raw cursor, after {step}"
        );
        for probe in [
            b"0".as_slice(),
            b"1",
            b"2",
            b"5",
            b"6",
            b"08",
            b"-1",
            b"name",
            b"",
        ] {
            let named = String::from_utf8_lossy(probe).into_owned();
            assert_eq!(
                packed.get(probe).and_then(Value::as_int),
                hashed.get(probe).and_then(Value::as_int),
                "`{named}`, after {step}"
            );
            assert_eq!(
                packed.has_key(probe),
                hashed.has_key(probe),
                "`{named}` presence, after {step}"
            );
        }
        // The integer-subscript pair answers whatever the key-taking pair
        // answers for the same key, in both shapes — the whole of its contract.
        for index in [-1_i64, 0, 1, 2, 5, 6, 8] {
            let rendered = index.to_string();
            assert_eq!(
                packed.get_index(index).and_then(Value::as_int),
                packed.get(rendered.as_bytes()).and_then(Value::as_int),
                "index {index} against key `{rendered}`, after {step}"
            );
            assert_eq!(
                packed.get_index(index).and_then(Value::as_int),
                hashed.get_index(index).and_then(Value::as_int),
                "index {index}, after {step}"
            );
        }
    }

    /// A renamed entry keeps its place in the order, the old key stops
    /// answering, and a copy that shared the storage still has the old key.
    #[test]
    fn a_renamed_entry_keeps_its_position() {
        let mut array = NvsArray::new();
        for (name, value) in [("a", 1), ("b", 2), ("c", 3)] {
            array.set(key(name), Value::int(value));
        }
        let before = array.clone();
        assert!(array.rename(b"b", key("z")));
        assert_eq!(
            readout(&array),
            vec![
                ("a".to_owned(), Some(1)),
                ("z".to_owned(), Some(2)),
                ("c".to_owned(), Some(3)),
            ]
        );
        assert!(array.get(b"b").is_none());
        assert_eq!(keys_of(&before), vec!["a", "b", "c"]);
        assert!(!array.rename(b"missing", key("y")), "an absent key");
        assert!(!array.rename(b"a", key("c")), "a key that is taken");
        assert_eq!(keys_of(&array), vec!["a", "z", "c"]);
    }

    /// A list renamed at one position converts to the hash form and keeps
    /// every other key, so a later append still goes after the largest one.
    #[test]
    fn renaming_inside_a_list_keeps_the_append_counter() {
        let mut array = list_of(3);
        assert!(array.rename(b"1", key("7")));
        array.append(Value::int(99));
        assert_eq!(keys_of(&array), vec!["0", "7", "2", "8"]);
    }

    #[test]
    fn a_fresh_array_is_empty_and_solely_owned() {
        let array = NvsArray::new();
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
        let mut list = NvsArray::new();
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
        // one past the end, a negative, and `"08"`, which `rule:types/arrays` keeps
        // distinct from `"8"`.
        assert!(list.get(b"32").is_none());
        assert!(list.get(b"-1").is_none());
        assert!(list.get(b"08").is_none());
        assert!(list.get(b"x").is_none());
        assert!(!list.has_key(b"32"));
    }

    #[test]
    fn an_empty_array_allocates_nothing() {
        // The module docs § *an empty array is a per-thread singleton*. Bytes
        // *ever* allocated is the only reading that shows it: a `live_bytes`
        // balance reads zero for a header allocated and freed inside the loop
        // just as happily as for one that was never built.
        const RUN: usize = 16;

        // Neither the thread's first `[]` nor the vector holding the handles
        // belongs inside the window: the singleton is one allocation per
        // thread and this is the thread that makes it.
        let primed = nvs_array_new();
        let mut held: Vec<*mut ArrayHeader> = Vec::with_capacity(RUN);

        let before = allocated_bytes();
        for _ in 0..RUN {
            held.push(nvs_array_new());
        }
        assert_eq!(
            allocated_bytes() - before,
            0,
            "{RUN} empty arrays allocated something, and a header is the only \
             thing `nvs_array_new` could have built"
        );

        // Held all at once, so this is sharing rather than one cached header
        // the next call happens to reuse.
        assert!(
            held.iter().all(|ptr| *ptr == primed),
            "every empty array a thread produces is the same header"
        );
        for ptr in &held {
            #[expect(unsafe_code, reason = "this test owns each reference it drops")]
            unsafe {
                assert_eq!(nvs_array_count(*ptr), 0, "the singleton stays empty");
                nvs_array_release(*ptr);
            }
        }
        #[expect(unsafe_code, reason = "this test owns the reference it drops")]
        unsafe {
            nvs_array_release(primed);
        }

        // The slot's own reference is the one nothing gives up, so releasing
        // every handed-out one leaves the header alive rather than freed.
        #[expect(unsafe_code, reason = "the slot keeps this allocation live")]
        let remaining = unsafe { NvsArray::refcount_of(primed) };
        assert_eq!(remaining, 1, "the thread-local slot still holds one");
    }

    #[test]
    fn writing_into_an_empty_array_allocates() {
        // The control for `an_empty_array_allocates_nothing`: a measurement
        // that only ever reads zero passes just as well when it is broken. It
        // is also what makes the singleton sound — a handle from
        // `nvs_array_new` shares the thread's header, so its count is never 1
        // and `make_unique` separates before the first write lands.
        #[expect(unsafe_code, reason = "this test owns the reference it reclaims")]
        let mut written = unsafe { NvsArray::from_raw(nvs_array_new()) };
        #[expect(unsafe_code, reason = "this test owns the reference it reclaims")]
        let untouched = unsafe { NvsArray::from_raw(nvs_array_new()) };

        let before = allocated_bytes();
        written.append(Value::int(7));
        assert!(
            allocated_bytes() > before,
            "the first write into an empty array is what buys it a header of \
             its own"
        );

        assert_eq!(written.count(), 1);
        assert_eq!(
            untouched.count(),
            0,
            "the shared empty array was separated from, never written through"
        );

        // And a later `[]` on the same thread is still the empty array, not
        // the one the write left behind.
        #[expect(unsafe_code, reason = "this test owns the reference it reclaims")]
        let later = unsafe { NvsArray::from_raw(nvs_array_new()) };
        assert_eq!(later.count(), 0);
    }

    #[test]
    fn an_integer_subscript_allocates_no_key() {
        // The ABI claim, measured rather than asserted about. A `live_bytes`
        // delta cannot see this one: `nvs_array_set` builds an `NvsStr` key,
        // hands it to the packed arm, which has no use for it, and drops it
        // again before the call returns — so the *live* delta of the key-taking
        // path is zero too. Bytes ever allocated is the only reading that tells
        // a transient allocation from none at all.
        const RUN: i64 = 16;

        let mut list = list_of(RUN);
        assert!(list.is_packed(), "an appended-into array is a list");

        let before = allocated_bytes();
        for index in 0..RUN {
            assert_eq!(
                list.get_index(index).and_then(Value::as_int),
                Some(index * 10)
            );
            list.set_index(index, Value::int(index * 10));
        }
        assert_eq!(
            allocated_bytes() - before,
            0,
            "{RUN} integer subscripts over a list allocated something: only a \
             key can be what it rendered"
        );

        // The same accesses spelled the way compiled code still spells them,
        // for contrast — one key allocation per call, which is the cost the
        // pair above exists to remove.
        let before = allocated_bytes();
        for index in 0..RUN {
            let rendered = index.to_string();
            assert_eq!(
                list.get(rendered.as_bytes()).and_then(Value::as_int),
                Some(index * 10)
            );
            list.set(key(&rendered), Value::int(index * 10));
        }
        assert!(
            allocated_bytes() > before,
            "the key-taking pair is supposed to build a key"
        );

        // Still a list, and still answering exactly what the hash form would.
        assert!(list.is_packed(), "an in-range write keeps the packed form");
        let mut hashed = list_of(RUN);
        hashed.degrade();
        agree(&list, &hashed, "a run of integer subscripts");

        // A write one past the end extends the packed form; a write beyond
        // that, or at a negative index, is a gap and degrades — the same
        // invariant `Table::set` holds, reached without a key string.
        let mut extended = list_of(3);
        extended.set_index(3, Value::int(30));
        assert!(extended.is_packed(), "the next position extends a list");
        assert_eq!(extended.count(), 4);

        let mut gapped = list_of(3);
        gapped.set_index(5, Value::int(50));
        assert!(!gapped.is_packed(), "a gap degrades");
        assert_eq!(keys_of(&gapped), ["0", "1", "2", "5"]);
        gapped.append(Value::int(60));
        assert_eq!(
            keys_of(&gapped),
            ["0", "1", "2", "5", "6"],
            "an integer-subscript write advances PHP's append counter"
        );

        let mut negative = list_of(2);
        negative.set_index(-1, Value::int(-10));
        assert!(!negative.is_packed(), "a negative key degrades");
        assert_eq!(negative.get(b"-1").and_then(Value::as_int), Some(-10));
        assert_eq!(negative.get_index(-1).and_then(Value::as_int), Some(-10));
    }

    #[test]
    fn a_callback_that_does_not_want_a_key_synthesizes_none() {
        // `docs/perf/userland-gap.md` § D's guard, at the level where the
        // allocation either happens or does not: `Core\Arr::map` over a list
        // reads one key per entry and stores under it, and a one-parameter
        // callback never asks for the string. What that member walks is
        // exactly this pair, so measuring it here needs no compiled callable.
        const RUN: i64 = 16;

        let list = list_of(RUN);
        let walk = |out: &mut NvsArray| {
            let mut from = 0;
            while let Some(slot) = list.next_slot(from) {
                from = slot + 1;
                let value = list.value_at(slot).expect("a live entry");
                match list.slot_key(slot).expect("a live entry") {
                    SlotKey::Index(index) => out.set_index(index, value),
                    SlotKey::Str(key) => out.set(key, value),
                }
            }
        };

        // The first pass grows the result's own storage, which is the array
        // being built and not a key; the second writes at positions that
        // already exist, so a rendered decimal is the only thing left that
        // could allocate.
        let mut out = NvsArray::new();
        walk(&mut out);
        let before = allocated_bytes();
        walk(&mut out);
        assert_eq!(
            allocated_bytes() - before,
            0,
            "{RUN} key-preserving stores over a list allocated something: a \
             rendered decimal is the only thing they could have built"
        );
        assert_eq!(keys_of(&out), keys_of(&list));

        // And the string is there for the callback that does declare a second
        // parameter — the same walk, one `to_str` heavier.
        let before = allocated_bytes();
        assert_eq!(
            list.slot_key(0).expect("a live entry").to_str().as_bytes(),
            b"0"
        );
        assert!(
            allocated_bytes() > before,
            "asking for the key as a string is what renders it"
        );

        // A hashed array holds its keys already, so the same walk hands back a
        // reference to the one it holds rather than a second rendering.
        let mut hashed = list_of(RUN);
        hashed.degrade();
        let before = allocated_bytes();
        let key = hashed.slot_key(0).expect("a live entry");
        assert!(matches!(key, SlotKey::Str(_)));
        assert_eq!(key.to_str().as_bytes(), b"0");
        assert_eq!(
            allocated_bytes() - before,
            0,
            "a hashed key is retained, never rebuilt"
        );
    }

    #[test]
    fn a_non_sequential_key_degrades_the_packed_array() {
        // One block per way to break "the keys are exactly `0`…`n−1`, in
        // order". Each asserts both that the shape gave way and that the
        // answer is the one the hash form would have given all along.

        // A gap, and then an append that follows the counter over it.
        let mut gapped = list_of(3);
        gapped.set(key("5"), Value::int(50));
        assert!(!gapped.is_packed());
        assert_eq!(keys_of(&gapped), ["0", "1", "2", "5"]);
        gapped.append(Value::int(60));
        assert_eq!(keys_of(&gapped), ["0", "1", "2", "5", "6"]);
        assert_eq!(gapped.get(b"1").and_then(Value::as_int), Some(10));

        // A non-numeric key.
        let mut named = list_of(3);
        named.set(key("name"), Value::int(1));
        assert!(!named.is_packed());
        assert_eq!(keys_of(&named), ["0", "1", "2", "name"]);

        // A non-canonical decimal, which `rule:types/arrays` keeps distinct from
        // `"8"` — the degrade is what preserves that.
        let mut padded = list_of(3);
        padded.set(key("08"), Value::int(1));
        assert!(!padded.is_packed());
        assert_eq!(keys_of(&padded), ["0", "1", "2", "08"]);

        // A negative key, which the packed form has no position for.
        let mut negative = list_of(3);
        negative.set(key("-1"), Value::int(1));
        assert!(!negative.is_packed());
        assert_eq!(keys_of(&negative), ["0", "1", "2", "-1"]);

        // A removal from the middle leaves a hole, which is exactly what the
        // invariant forbids.
        let mut middle = list_of(3);
        middle.unset(b"1");
        assert!(!middle.is_packed());
        assert_eq!(keys_of(&middle), ["0", "2"]);

        // A removal from the end does not, and the counter survives it: `$a =
        // [1,2,3]; unset($a[2]); $a[] = 9;` writes key `3` in PHP 8.5, so the
        // append that follows is where this one gives way.
        let mut tail = list_of(3);
        tail.unset(b"2");
        assert!(tail.is_packed());
        assert_eq!(keys_of(&tail), ["0", "1"]);
        tail.append(Value::int(9));
        assert!(!tail.is_packed());
        assert_eq!(keys_of(&tail), ["0", "1", "3"]);

        // Unsetting a key that is not there touches nothing at all.
        let mut absent = list_of(3);
        absent.unset(b"9");
        absent.unset(b"name");
        assert!(absent.is_packed());
        assert_eq!(keys_of(&absent), ["0", "1", "2"]);

        // The two integer writes that hold the invariant: exactly the next
        // position, and any position already there.
        let mut extended = list_of(3);
        extended.set(key("3"), Value::int(30));
        extended.set(key("0"), Value::int(99));
        assert!(extended.is_packed());
        assert_eq!(keys_of(&extended), ["0", "1", "2", "3"]);
        assert_eq!(extended.get(b"0").and_then(Value::as_int), Some(99));
        extended.append(Value::int(40));
        assert!(extended.is_packed());
        assert_eq!(keys_of(&extended), ["0", "1", "2", "3", "4"]);
    }

    #[test]
    fn both_representations_answer_every_primitive_alike() {
        // The same content in both shapes, so any difference the assertions
        // find is a difference in the representation and nothing else.
        let packed = list_of(6);
        let mut hashed = list_of(6);
        hashed.degrade();
        assert!(packed.is_packed());
        assert!(!hashed.is_packed());
        agree(&packed, &hashed, "the build");

        // Then the same mutation applied to both, compared after each one.
        // The packed handle degrades of its own accord partway through, which
        // is the point: after that the two are the same shape and must still
        // agree with what the first half recorded.
        let mut packed = packed;
        for (step, mutate, still_packed) in mutations() {
            mutate(&mut packed);
            mutate(&mut hashed);
            assert_eq!(
                packed.is_packed(),
                still_packed,
                "the shape after {step} is not the one this test is comparing"
            );
            assert!(!hashed.is_packed(), "the hash form never packs itself");
            agree(&packed, &hashed, step);
        }

        // Freeing is the last primitive, and the counting allocator is the
        // only witness that says so.
        let before = live_bytes();
        {
            let mut packed = list_of(4);
            let mut hashed = list_of(4);
            hashed.degrade();
            packed.set(key("k"), Value::str(NvsStr::new(b"a stored string")));
            hashed.set(key("k"), Value::str(NvsStr::new(b"a stored string")));
            let alias = packed.clone();
            packed.append(Value::int(1));
            drop(alias);
        }
        assert_eq!(live_bytes(), before);
    }

    /// Every mutating primitive, in an order that holds the packed invariant
    /// through the writes a list allows and breaks it after — the third field
    /// is which, so the test pins where the shape gives way rather than only
    /// that the answers match.
    fn mutations() -> Vec<Mutation> {
        vec![
            ("an append", |array| array.append(Value::int(60)), true),
            (
                "an overwrite",
                |array| {
                    array.set(key("2"), Value::int(99));
                },
                true,
            ),
            (
                "a write at exactly the next position",
                |array| {
                    array.set(key("7"), Value::int(70));
                },
                true,
            ),
            ("a removal from the end", |array| array.unset(b"7"), true),
            (
                "a gap",
                |array| {
                    array.set(key("9"), Value::int(90));
                },
                false,
            ),
            (
                "a named key",
                |array| {
                    array.set(key("name"), Value::int(1));
                },
                false,
            ),
            (
                "a removal from the middle",
                |array| array.unset(b"1"),
                false,
            ),
            (
                "an append after all of it",
                |array| {
                    array.append(Value::int(100));
                },
                false,
            ),
            (
                "a removal of what is not there",
                |array| array.unset(b"zzz"),
                false,
            ),
        ]
    }

    #[test]
    fn iteration_order_is_insertion_order() {
        let mut array = NvsArray::new();
        array.set(key("alpha"), Value::int(1));
        array.set(key("beta"), Value::int(2));
        array.set(key("gamma"), Value::int(3));
        assert_eq!(keys_of(&array), ["alpha", "beta", "gamma"]);
    }

    #[test]
    fn overwriting_keeps_a_keys_position() {
        let mut array = NvsArray::new();
        array.set(key("alpha"), Value::int(1));
        array.set(key("beta"), Value::int(2));
        array.set(key("alpha"), Value::int(9));
        assert_eq!(keys_of(&array), ["alpha", "beta"]);
        assert_eq!(array.get(b"alpha").and_then(Value::as_int), Some(9));
        assert_eq!(array.count(), 2);
    }

    #[test]
    fn removing_and_reinserting_moves_a_key_to_the_end() {
        // `examples/arrays.nvs`'s frozen output depends on exactly this.
        let mut array = NvsArray::new();
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
        let mut array = NvsArray::new();
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
    fn an_append_onto_the_saturated_counter_is_refused_and_changes_nothing() {
        // Every assertion below is `php -r` output from the 8.5.9 oracle, and
        // `tests/conformance/array/` pins the same bound from Novis source.
        let mut array = NvsArray::new();
        array.set(key("9223372036854775806"), Value::int(1));
        // The last accepted side: the counter still has one key left to give.
        assert!(array.try_append(Value::int(2)).is_ok());
        assert_eq!(
            keys_of(&array),
            ["9223372036854775806", "9223372036854775807"]
        );

        // The first refused side. The value comes back unappended, so the
        // caller still owes its reference and nothing was overwritten.
        assert_eq!(
            array.try_append(Value::int(3)).unwrap_err().as_int(),
            Some(3)
        );
        assert_eq!(array.count(), 2);
        assert_eq!(
            array.get(b"9223372036854775807").and_then(Value::as_int),
            Some(2)
        );

        // `unset` frees the key without moving the counter, so the very next
        // append lands on it a second time.
        array.unset(b"9223372036854775807");
        assert!(array.try_append(Value::int(4)).is_ok());
        assert_eq!(
            keys_of(&array),
            ["9223372036854775806", "9223372036854775807"]
        );
        assert_eq!(
            array.get(b"9223372036854775807").and_then(Value::as_int),
            Some(4)
        );
    }

    #[test]
    fn a_negative_integer_key_advances_the_counter() {
        // Every assertion below is `php -r` output from the 8.5.9 oracle.
        let mut array = NvsArray::new();
        array.set(key("-5"), Value::int(1));
        array.append(Value::int(2));
        assert_eq!(keys_of(&array), ["-5", "-4"]);

        // A negative key may only *raise* the counter once something has set
        // it, so it is ignored here.
        let mut after_positive = NvsArray::new();
        after_positive.set(key("3"), Value::int(1));
        after_positive.set(key("-5"), Value::int(1));
        after_positive.append(Value::int(2));
        assert_eq!(keys_of(&after_positive), ["3", "-5", "4"]);

        // An append is itself what sets the counter on an empty array.
        let mut appended_first = NvsArray::new();
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

        let mut array = NvsArray::new();
        array.set(key("08"), Value::int(1));
        array.append(Value::int(2));
        assert_eq!(keys_of(&array), ["08", "0"]);
    }

    #[test]
    fn a_write_through_a_second_handle_separates() {
        let mut original = NvsArray::new();
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
        let mut array = NvsArray::new();
        let before = array.header() as *const ArrayHeader;
        for index in 0..64 {
            array.set(key(&index.to_string()), Value::int(index));
        }
        assert!(std::ptr::eq(before, array.header()));
        assert_eq!(array.count(), 64);
    }

    #[test]
    fn separation_shares_a_nested_arrays_storage_until_it_is_written() {
        let inner = NvsArray::new();
        let mut outer = NvsArray::new();
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
        let text = NvsStr::new(b"shared");
        let mut original = NvsArray::new();
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

    /// `slot_end` counts holes until compaction sweeps them, and at least half
    /// of the slots below it are live either way — the bound `Core\Random::pick`
    /// draws inside.
    #[test]
    fn slot_end_counts_holes_and_stays_under_twice_the_live_entries() {
        let mut array = NvsArray::new();
        for index in 0..20 {
            array.append(Value::int(index));
        }
        assert_eq!(array.slot_end(), 20);
        for index in 0..9 {
            array.unset(index.to_string().as_bytes());
        }
        assert_eq!((array.count(), array.slot_end()), (11, 20));
        assert!(array.value_at(0).is_none());
        array.unset(b"9");
        assert_eq!((array.count(), array.slot_end()), (10, 10));
    }

    #[test]
    fn compaction_preserves_order_and_the_append_counter() {
        let mut array = NvsArray::new();
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
        let mut array = NvsArray::new();
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
            let mut outer = NvsArray::new();
            for index in 0..16 {
                let mut inner = NvsArray::new();
                inner.set(key("name"), Value::str(NvsStr::new(b"a stored string")));
                inner.append(Value::str(NvsStr::new(b"another")));
                outer.set(key(&index.to_string()), Value::array(inner));
            }
            let alias = outer.clone();
            outer.set(key("written-after-aliasing"), Value::int(1));
            drop(alias);
        }
        assert_eq!(live_bytes(), before);
    }

    #[test]
    fn a_refused_append_neither_separates_nor_re_points() {
        // The order of `try_append`'s two steps is what compiled code rests
        // on, and it is reachable from source: `$b = $a; $a[] = 1;` over an
        // array already holding `i64::MAX`. `nvs_codegen`'s `emit_array_append`
        // loads the yielded pointer **only** on the continuation edge, so a
        // refusal that had already separated would leave the frame releasing a
        // reference the call consumed and the clone with nobody to release it.
        // Measured rather than asserted about: a refcount agreeing with itself
        // is not the memory coming back.
        let mut ctx = crate::Ctx::buffered();
        let before = live_bytes();
        {
            let mut full = NvsArray::new();
            full.set_index(i64::MAX, Value::int(1));
            let alias = full.clone();
            let given = full.into_raw();

            let value = Value::int(2);
            let mut out: *mut ArrayHeader = std::ptr::null_mut();
            #[expect(unsafe_code, reason = "this test owns every reference it hands over")]
            let status =
                unsafe { nvs_array_append(&raw mut ctx, given, &raw const value, &raw mut out) };

            assert_eq!(status, crate::THROWN, "the next integer key is occupied");
            assert_eq!(
                out, given,
                "the occupancy test runs before the separation, so a refusal \
                 leaves the caller's own pointer to hand back"
            );
            #[expect(unsafe_code, reason = "this test owns the reference it releases")]
            unsafe {
                nvs_array_release(out);
            }
            drop(alias);
        }
        drop(ctx.take_pending());
        assert_eq!(live_bytes(), before);
    }

    #[test]
    fn a_refused_spread_hands_back_the_separation_it_had_already_made() {
        // The spread is the one faulting write that *can* re-point, because it
        // writes before it refuses — so it writes the handle back through `out`
        // ahead of deciding on the fault, and the clone is never dropped. No
        // program reaches this state (a literal under construction is solely
        // owned unless it is still the empty singleton, which has no next index
        // to occupy), so the state is built here directly.
        let mut ctx = crate::Ctx::buffered();
        let before = live_bytes();
        {
            // Shared, so the first write separates; already at `i64::MAX`, so a
            // renumbered entry is refused.
            let mut dest = NvsArray::new();
            dest.set_index(i64::MAX, Value::int(1));
            let alias = dest.clone();
            let given = dest.into_raw();

            // A string key first — copied under its own name, and the write
            // that separates — then an integer key, which is renumbered and
            // refused.
            let mut subject = NvsArray::new();
            subject.set(key("name"), Value::int(2));
            subject.set_index(0, Value::int(3));
            let borrowed = subject.into_raw();

            let mut out: *mut ArrayHeader = std::ptr::null_mut();
            #[expect(unsafe_code, reason = "this test owns every reference it hands over")]
            let status = unsafe { nvs_array_spread(&raw mut ctx, given, borrowed, &raw mut out) };

            assert_eq!(status, crate::THROWN, "the next integer key is occupied");
            assert_ne!(
                out, given,
                "the copy the string key's write installed is what comes back"
            );
            #[expect(unsafe_code, reason = "this test owns every reference it releases")]
            unsafe {
                assert_eq!(NvsArray::refcount_of(out), 1, "the copy is solely owned");
                nvs_array_release(out);
                nvs_array_release(borrowed);
            }
            drop(alias);
        }
        drop(ctx.take_pending());
        assert_eq!(live_bytes(), before);
    }

    #[test]
    fn a_deeply_nested_array_releases_without_recursing() {
        // The depth that would overflow a recursive release. See this module's
        // "freeing is iterative" decision.
        let before = live_bytes();
        {
            let mut nest = NvsArray::new();
            for _ in 0..200_000 {
                let mut outer = NvsArray::new();
                outer.set(key("inner"), Value::array(nest));
                nest = outer;
            }
        }
        assert_eq!(live_bytes(), before);
    }
}
