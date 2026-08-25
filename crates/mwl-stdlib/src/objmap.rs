//! `Core\ObjectMap<K, V>` — docs/spec/01-core-library.md § 9's identity-keyed
//! map, which replaces `SplObjectStorage` used as a map and every
//! `spl_object_id` side table.
//!
//! # Decision: two stores under one set of chain keys
//!
//! [`crate::identity_store`] keys an entry by the identity hash of the value
//! it holds, which answers a *set*'s question directly. A map has two values
//! per entry, and only the key is hashed — so this class holds two stores,
//! `keys` and `values`, and writes both under the key [`identity_store::locate`]
//! derives from the `keys` store alone.
//!
//! The two stay aligned because every mutation touches them the same way: an
//! insert appends to both at the same position, a re-`set` overwrites in
//! place (an insertion-ordered hash keeps a rewritten key where it was), and a
//! removal calls [`identity_store::vacate`] on each, which moves the same
//! chain tail into the same hole because the two stores have identical key
//! sets. That alignment is what makes `keys()` and `values()` pair up
//! positionally, which is the one property a caller can observe about the
//! order.
//!
//! The rejected alternative was one store whose values are two-element
//! arrays: it halves the entry count and costs a nested allocation per pair
//! plus a take-out-mutate-put-back on every write, which is exactly the shape
//! [`crate::identity_store`]'s docs reject for the store itself.
//!
//! **What it spends:** two array allocations per map, and two entries per
//! pair — see [`crate::identity_store`] for the per-entry figure.

use mwl_runtime::{Fault, MwlArray, MwlStr, ObjHeader, Value};

use crate::identity_store as store;
use crate::registry::{CoreClass, CoreMethod, CoreTy};

/// The class's fully-qualified name, as [`CoreTy::Instance`] spells it.
pub(crate) const NAME: &str = r"Core\ObjectMap";

/// The linker symbol `new Core\ObjectMap<K, V>()` lowers to — see
/// [`crate::registry::CONSTRUCTORS`], which is the roster `mwl-ir` reads.
pub(crate) const NEW_SYMBOL: &str = "mwl_core_object_map_new";

/// `Core\ObjectMap<K, V>` — docs/spec/01-core-library.md § 9's first row.
///
/// **`get` returns `?V`, not a throwing read**: § 9 says so outright, because
/// these types have no subscript ([ADR 0053](../../../../docs/adr/0053-iteration-and-generators.md)
/// rejects `ArrayAccess`) and so cannot offer the `$a[$k]` / `$a[$k] ?? $d`
/// pair that `array<T>` does, while ADR 0063 R5 bans a `getOrNull` twin.
/// `Iterable` is the section's one remaining row and is not here yet.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "set",
            params: &[CoreTy::Var("K"), CoreTy::Var("V")],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "mwl_core_object_map_set",
        },
        CoreMethod {
            name: "get",
            params: &[CoreTy::Var("K")],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("V")),
            symbol: "mwl_core_object_map_get",
        },
        CoreMethod {
            name: "has",
            params: &[CoreTy::Var("K")],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_object_map_has",
        },
        CoreMethod {
            name: "remove",
            params: &[CoreTy::Var("K")],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "mwl_core_object_map_remove",
        },
        CoreMethod {
            name: "count",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "mwl_core_object_map_count",
        },
        CoreMethod {
            name: "isEmpty",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_object_map_is_empty",
        },
        CoreMethod {
            name: "keys",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("K")),
            symbol: "mwl_core_object_map_keys",
        },
        CoreMethod {
            name: "values",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("V")),
            symbol: "mwl_core_object_map_values",
        },
        CoreMethod {
            name: "clear",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "mwl_core_object_map_clear",
        },
    ],
    slots: &["keys", "values"],
    constants: &[],
};

/// [`CLASS`]'s slots, by index — the two stores this module's docs describe.
const KEYS: usize = 0;
/// See [`KEYS`].
const VALUES: usize = 1;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        NEW_SYMBOL => (mwl_core_object_map_new as *const ()).cast(),
        "mwl_core_object_map_set" => (mwl_core_object_map_set as *const ()).cast(),
        "mwl_core_object_map_get" => (mwl_core_object_map_get as *const ()).cast(),
        "mwl_core_object_map_has" => (mwl_core_object_map_has as *const ()).cast(),
        "mwl_core_object_map_remove" => (mwl_core_object_map_remove as *const ()).cast(),
        "mwl_core_object_map_count" => (mwl_core_object_map_count as *const ()).cast(),
        "mwl_core_object_map_is_empty" => (mwl_core_object_map_is_empty as *const ()).cast(),
        "mwl_core_object_map_keys" => (mwl_core_object_map_keys as *const ()).cast(),
        "mwl_core_object_map_values" => (mwl_core_object_map_values as *const ()).cast(),
        "mwl_core_object_map_clear" => (mwl_core_object_map_clear as *const ()).cast(),
        _ => return None,
    })
}

/// The receiver of one of this class's instance members.
///
/// # Errors
///
/// [`crate::instance::receiver`]'s, unchanged.
fn map_of(value: Value, member: &str) -> Result<*mut ObjHeader, Fault> {
    crate::instance::receiver(value, &CLASS, member)
}

/// Where `key` lives in `receiver`'s key store, as `(chain key, present)` —
/// [`store::locate`] over the one slot that decides it.
///
/// # Errors
///
/// [`store::borrow`]'s, unchanged.
fn at(receiver: *mut ObjHeader, key: Value, member: &str) -> Result<(Vec<u8>, bool), Fault> {
    let keys = store::borrow(receiver, KEYS, &CLASS, member)?;
    Ok(store::locate(&keys, key))
}

/// Every value a store holds, in the store's own order, as a fresh MWL list.
///
/// Each entry is retained: the store outlives the call, so the list needs a
/// reference of its own — the rule [`crate::arr`] applies everywhere it copies
/// an entry out of a borrowed subject.
fn listed(store: &MwlArray) -> MwlArray {
    let mut out = MwlArray::new();
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

mwl_runtime::mwl_helper! {
    /// `new Core\ObjectMap<K, V>()` — a fresh empty map.
    ///
    /// Reached as a symbol rather than as a registered `constructor` member,
    /// for the reason [`crate::instance`]'s module docs give.
    fn mwl_core_object_map_new(_ctx, _args: [0]) {
        Ok(crate::instance::build(
            &CLASS,
            [
                Value::array(MwlArray::new()),
                Value::array(MwlArray::new()),
            ],
        ))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectMap<K, V>::set(K $key, V $value): void` — associates
    /// `$value` with `$key`, replacing whatever `$key` held.
    ///
    /// The key is written only where it is new: an insertion-ordered hash
    /// keeps a rewritten entry where it was, so re-setting an existing key
    /// would release and re-store an identical value for nothing, and would
    /// be the one place the two stores could drift apart.
    fn mwl_core_object_map_set(_ctx, args: [3]) {
        let receiver = map_of(args[0], "set")?;
        let (key, value) = (args[1], args[2]);
        let (chain, present) = at(receiver, key, "set")?;
        if !present {
            store::edit(receiver, KEYS, &CLASS, "set", |keys| {
                #[expect(
                    unsafe_code,
                    reason = "the argument is borrowed from the caller's frame, \
                              so the copy the store keeps needs a reference of \
                              its own"
                )]
                unsafe {
                    key.retain();
                }
                keys.set(MwlStr::new(&chain), key);
            })?;
        }
        store::edit(receiver, VALUES, &CLASS, "set", |values| {
            #[expect(
                unsafe_code,
                reason = "the argument is borrowed from the caller's frame, so \
                          the copy the store keeps needs a reference of its own"
            )]
            unsafe {
                value.retain();
            }
            // Releases what this key held, which is what makes a re-`set`
            // replace rather than leak — `MwlArray::set`'s own contract.
            values.set(MwlStr::new(&chain), value);
        })?;
        Ok(Value::null())
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectMap<K, V>::get(K $key): ?V` — what `$key` holds, or `null`
    /// where the map holds no key identical to it.
    ///
    /// `?V` rather than a throwing read: § 9 says so, and [`CLASS`]'s own doc
    /// comment holds the reasoning. A map that genuinely holds `null` under a
    /// key is therefore indistinguishable from one that holds nothing — the
    /// same ambiguity `$a[$k] ?? $d` has over an array, and `has` is the
    /// spelling that resolves it.
    fn mwl_core_object_map_get(_ctx, args: [2]) {
        let receiver = map_of(args[0], "get")?;
        let (chain, present) = at(receiver, args[1], "get")?;
        if !present {
            return Ok(Value::null());
        }
        let values = store::borrow(receiver, VALUES, &CLASS, "get")?;
        let held = values.get(&chain).unwrap_or_else(Value::null);
        #[expect(
            unsafe_code,
            reason = "the store owns the reference and outlives this call, so \
                      the value handed back to MWL code needs one of its own"
        )]
        unsafe {
            held.retain();
        }
        Ok(held)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectMap<K, V>::has(K $key): bool` — whether the map holds a
    /// key identical to `$key`.
    fn mwl_core_object_map_has(_ctx, args: [2]) {
        let receiver = map_of(args[0], "has")?;
        let (_, present) = at(receiver, args[1], "has")?;
        Ok(Value::bool(present))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectMap<K, V>::remove(K $key): void` — drops `$key` and what
    /// it held, and does nothing where the map holds no such key.
    ///
    /// [`store::vacate`] on each store in turn, and the two agree on which
    /// entry moves into the hole because their key sets are identical — this
    /// module's alignment invariant.
    fn mwl_core_object_map_remove(_ctx, args: [2]) {
        let receiver = map_of(args[0], "remove")?;
        let (chain, present) = at(receiver, args[1], "remove")?;
        if !present {
            return Ok(Value::null());
        }
        for slot in [KEYS, VALUES] {
            store::edit(receiver, slot, &CLASS, "remove", |held| {
                store::vacate(held, &chain);
            })?;
        }
        Ok(Value::null())
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectMap<K, V>::count(): uint` — how many pairs the map holds.
    fn mwl_core_object_map_count(_ctx, args: [1]) {
        let receiver = map_of(args[0], "count")?;
        let keys = store::borrow(receiver, KEYS, &CLASS, "count")?;
        let count = u64::try_from(keys.count()).expect("an entry count fits in a `uint`");
        Ok(Value::uint(count))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectMap<K, V>::isEmpty(): bool` — whether the map holds no
    /// pairs.
    fn mwl_core_object_map_is_empty(_ctx, args: [1]) {
        let receiver = map_of(args[0], "isEmpty")?;
        let keys = store::borrow(receiver, KEYS, &CLASS, "isEmpty")?;
        Ok(Value::bool(keys.is_empty()))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectMap<K, V>::keys(): array<K>` — every key, as a list.
    ///
    /// A list rather than a map keyed by anything: an `array<T>` keys on
    /// `int|string` (ADR 0063 § 4), which is the whole reason this class
    /// exists, so there is no key here to preserve. Position pairs with
    /// [`mwl_core_object_map_values`]'s.
    fn mwl_core_object_map_keys(_ctx, args: [1]) {
        let receiver = map_of(args[0], "keys")?;
        let keys = store::borrow(receiver, KEYS, &CLASS, "keys")?;
        Ok(Value::array(listed(&keys)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectMap<K, V>::values(): array<V>` — every value, as a list,
    /// in the order [`mwl_core_object_map_keys`] answers its keys.
    fn mwl_core_object_map_values(_ctx, args: [1]) {
        let receiver = map_of(args[0], "values")?;
        let values = store::borrow(receiver, VALUES, &CLASS, "values")?;
        Ok(Value::array(listed(&values)))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectMap<K, V>::clear(): void` — drops every pair.
    fn mwl_core_object_map_clear(_ctx, args: [1]) {
        let receiver = map_of(args[0], "clear")?;
        store::replace(receiver, KEYS);
        store::replace(receiver, VALUES);
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use mwl_runtime::{Ctx, MwlFn, call};

    /// Runs `member` with `map` as the receiver and `rest` past it.
    fn on(map: Value, member: MwlFn, rest: &[Value]) -> Value {
        let mut ctx = Ctx::buffered();
        let mut args = vec![map];
        args.extend_from_slice(rest);
        call(member, &mut ctx, &args).expect("a map member does not throw")
    }

    /// The list `keys`/`values` answered, as `int`s.
    fn ints(list: Value) -> Vec<i64> {
        let ptr = list.array_ptr().expect("a list is an array");
        let held = crate::arr::borrowed(ptr);
        let mut out = Vec::new();
        let mut from = 0usize;
        while let Some(slot) = held.next_slot(from) {
            out.push(held.value_at(slot).and_then(Value::as_int).unwrap_or(-1));
            from = slot + 1;
        }
        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference the member produced"
        )]
        unsafe {
            list.release();
        }
        out
    }

    /// `keys` and `values` stay paired across an overwrite and a removal —
    /// the alignment invariant this module's docs make load-bearing, which no
    /// single member can check on its own.
    #[test]
    fn the_two_stores_stay_paired_through_set_and_remove() {
        let mut ctx = Ctx::buffered();
        let map = call(mwl_core_object_map_new, &mut ctx, &[]).expect("a fresh map does not throw");

        for n in 1..=3i64 {
            on(
                map,
                mwl_core_object_map_set,
                &[Value::int(n), Value::int(n * 10)],
            );
        }
        on(
            map,
            mwl_core_object_map_set,
            &[Value::int(2), Value::int(99)],
        );
        assert_eq!(on(map, mwl_core_object_map_count, &[]).as_uint(), Some(3));
        assert_eq!(ints(on(map, mwl_core_object_map_keys, &[])), vec![1, 2, 3]);
        assert_eq!(
            ints(on(map, mwl_core_object_map_values, &[])),
            vec![10, 99, 30]
        );

        on(map, mwl_core_object_map_remove, &[Value::int(1)]);
        assert_eq!(on(map, mwl_core_object_map_count, &[]).as_uint(), Some(2));
        let keys = ints(on(map, mwl_core_object_map_keys, &[]));
        let values = ints(on(map, mwl_core_object_map_values, &[]));
        for (key, value) in keys.iter().zip(&values) {
            let expected = if *key == 2 { 99 } else { key * 10 };
            assert_eq!(*value, expected, "key {key} lost its value");
        }
        assert_eq!(
            on(map, mwl_core_object_map_get, &[Value::int(1)]).as_int(),
            None
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference the constructor produced"
        )]
        unsafe {
            map.release();
        }
    }
}
