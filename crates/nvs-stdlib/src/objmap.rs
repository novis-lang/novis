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

use nvs_runtime::{Fault, NvsArray, NvsStr, ObjHeader, Value};

use crate::identity_store as store;
use crate::registry::{CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc};

/// The class's fully-qualified name, as [`CoreTy::Instance`] spells it.
pub(crate) const NAME: &str = r"Core\ObjectMap";

/// The linker symbol `new Core\ObjectMap<K, V>()` lowers to — see
/// [`crate::registry::CONSTRUCTORS`], which is the roster `nvs-ir` reads.
pub(crate) const NEW_SYMBOL: &str = "nvs_core_object_map_new";

/// The symbol behind `Iterable<K>::iterate()`, reached by name through this
/// class's method table rather than as a registered member — see
/// [`crate::cursor`] and [`crate::instance`]'s dispatch roster.
pub(crate) const ITERATE_SYMBOL: &str = "nvs_core_object_map_iterate";

/// `new Core\ObjectMap<K, V>()` — the constructor
/// [`crate::registry::CONSTRUCTORS`] registers, which takes nothing: a map's
/// order is its insertion order and its keying is `nvs_runtime::identity`'s,
/// so there is nothing to give it.
pub(crate) const NEW: CoreMethod = CoreMethod {
    name: "constructor",
    names: &[],
    params: &[],
    defaults: &[],
    return_ty: CoreTy::Instance(NAME),
    symbol: NEW_SYMBOL,
    doc: Some(&CONSTRUCTOR_DOC),
};

/// `Core\ObjectMap<K, V>` — docs/spec/01-core-library.md § 9's first row.
///
/// **`get` returns `?V`, not a throwing read**: § 9 says so outright, because
/// these types have no subscript (`rule:iteration/two-interfaces`
/// rejects `ArrayAccess`) and so cannot offer the `$a[$k]` / `$a[$k] ?? $d`
/// pair that `array<T>` does, while `rule:core-api/shape-rules` R5 bans a `getOrNull` twin.
///
/// **A `foreach` over a map yields its keys**, which is what
/// `SplObjectStorage` yields and the only choice that loses nothing: a key
/// hands `get` back its value, while a value hands nothing back the key it was
/// stored under — and `rule:iteration/two-interfaces` gives a cursor no key binding to carry the
/// other half in. `values()` is the spelling for the other direction.
/// [`crate::cursor`] owns the mechanism and what the snapshot spends.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: None,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "set",
            names: &["key", "value"],
            params: &[CoreTy::Var("K"), CoreTy::Var("V")],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_object_map_set",
            doc: Some(&SET_DOC),
        },
        CoreMethod {
            name: "get",
            names: &["key"],
            params: &[CoreTy::Var("K")],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("V")),
            symbol: "nvs_core_object_map_get",
            doc: Some(&GET_DOC),
        },
        CoreMethod {
            name: "has",
            names: &["key"],
            params: &[CoreTy::Var("K")],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_object_map_has",
            doc: Some(&HAS_DOC),
        },
        CoreMethod {
            name: "remove",
            names: &["key"],
            params: &[CoreTy::Var("K")],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_object_map_remove",
            doc: Some(&REMOVE_DOC),
        },
        CoreMethod {
            name: "count",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_object_map_count",
            doc: Some(&COUNT_DOC),
        },
        CoreMethod {
            name: "isEmpty",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_object_map_is_empty",
            doc: Some(&IS_EMPTY_DOC),
        },
        CoreMethod {
            name: "keys",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("K")),
            symbol: "nvs_core_object_map_keys",
            doc: Some(&KEYS_DOC),
        },
        CoreMethod {
            name: "values",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("V")),
            symbol: "nvs_core_object_map_values",
            doc: Some(&VALUES_DOC),
        },
        CoreMethod {
            name: "clear",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_object_map_clear",
            doc: Some(&CLEAR_DOC),
        },
    ],
    slots: &["keys", "values"],
    constants: &[],
};

/// `new Core\ObjectMap`'s reference card — `rule:core-api/reference-card`.
const CONSTRUCTOR_DOC: MethodDoc = MethodDoc {
    short: "Builds an empty `Core\\ObjectMap<K, V>` — a map keyed by identity, which replaces \
            `SplObjectStorage` used as a map and every `spl_object_id` side table.",
    params: &[],
    ret: "A fresh map holding nothing, whose iteration order is insertion order.",
    errors: &[],
};

/// `Core\ObjectMap::set`'s reference card — `rule:core-api/reference-card`.
const SET_DOC: MethodDoc = MethodDoc {
    short: "Associates `$value` with `$key`, replacing whatever `$key` held; a key is matched by \
            identity, never by equality.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "The key, matched by identity against the keys the map holds.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The value to store under `$key`.",
            shape: &[],
        },
    ],
    ret: "Nothing; a key set again keeps its place in the insertion order.",
    errors: &[],
};

/// `Core\ObjectMap::get`'s reference card — `rule:core-api/reference-card`.
const GET_DOC: MethodDoc = MethodDoc {
    short: "Answers what `$key` holds — `?V` rather than a throwing read, because these types \
            have no subscript and so no `$a[$k] ?? $d` to offer instead.",
    params: &[ParamDoc {
        name: "key",
        desc: "The key to look up, matched by identity.",
        shape: &[],
    }],
    ret: "The value stored under `$key`, or `null` where the map holds no key identical to it — \
          which a stored `null` is indistinguishable from, so `has` is the question that tells \
          the two apart.",
    errors: &[],
};

/// `Core\ObjectMap::has`'s reference card — `rule:core-api/reference-card`.
const HAS_DOC: MethodDoc = MethodDoc {
    short: "Whether the map holds a key identical to `$key`.",
    params: &[ParamDoc {
        name: "key",
        desc: "The key to look for, matched by identity.",
        shape: &[],
    }],
    ret: "`true` when the key is present — even where it holds `null` — and `false` otherwise.",
    errors: &[],
};

/// `Core\ObjectMap::remove`'s reference card — `rule:core-api/reference-card`.
const REMOVE_DOC: MethodDoc = MethodDoc {
    short: "Drops `$key` and the value it held.",
    params: &[ParamDoc {
        name: "key",
        desc: "The key to drop, matched by identity.",
        shape: &[],
    }],
    ret: "Nothing; a key the map does not hold is left alone rather than reported.",
    errors: &[],
};

/// `Core\ObjectMap::count`'s reference card — `rule:core-api/reference-card`.
const COUNT_DOC: MethodDoc = MethodDoc {
    short: "Counts the pairs the map holds.",
    params: &[],
    ret: "The number of keys; `0` for an empty map.",
    errors: &[],
};

/// `Core\ObjectMap::isEmpty`'s reference card — `rule:core-api/reference-card`.
const IS_EMPTY_DOC: MethodDoc = MethodDoc {
    short: "Whether the map holds no pairs.",
    params: &[],
    ret: "`true` for a map with no keys, `false` otherwise.",
    errors: &[],
};

/// `Core\ObjectMap::keys`'s reference card — `rule:core-api/reference-card`.
const KEYS_DOC: MethodDoc = MethodDoc {
    short: "Every key, as a list in insertion order — the same order a `foreach` over the map \
            yields.",
    params: &[],
    ret: "A fresh `array<K>` list, empty for an empty map, whose positions pair with `values()`.",
    errors: &[],
};

/// `Core\ObjectMap::values`'s reference card — `rule:core-api/reference-card`.
const VALUES_DOC: MethodDoc = MethodDoc {
    short: "Every value, as a list in the order `keys()` answers its keys.",
    params: &[],
    ret: "A fresh `array<V>` list, empty for an empty map, whose positions pair with `keys()`.",
    errors: &[],
};

/// `Core\ObjectMap::clear`'s reference card — `rule:core-api/reference-card`.
const CLEAR_DOC: MethodDoc = MethodDoc {
    short: "Drops every pair, leaving the map empty.",
    params: &[],
    ret: "Nothing; the map itself is kept and can be filled again.",
    errors: &[],
};

/// [`CLASS`]'s slots, by index — the two stores this module's docs describe.
const KEYS: usize = 0;
/// See [`KEYS`].
const VALUES: usize = 1;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        NEW_SYMBOL => (nvs_core_object_map_new as *const ()).cast(),
        "nvs_core_object_map_set" => (nvs_core_object_map_set as *const ()).cast(),
        "nvs_core_object_map_get" => (nvs_core_object_map_get as *const ()).cast(),
        "nvs_core_object_map_has" => (nvs_core_object_map_has as *const ()).cast(),
        "nvs_core_object_map_remove" => (nvs_core_object_map_remove as *const ()).cast(),
        "nvs_core_object_map_count" => (nvs_core_object_map_count as *const ()).cast(),
        "nvs_core_object_map_is_empty" => (nvs_core_object_map_is_empty as *const ()).cast(),
        "nvs_core_object_map_keys" => (nvs_core_object_map_keys as *const ()).cast(),
        "nvs_core_object_map_values" => (nvs_core_object_map_values as *const ()).cast(),
        "nvs_core_object_map_clear" => (nvs_core_object_map_clear as *const ()).cast(),
        ITERATE_SYMBOL => (nvs_core_object_map_iterate as *const ()).cast(),
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

nvs_runtime::nvs_helper! {
    /// `new Core\ObjectMap<K, V>()` — a fresh empty map.
    ///
    /// Reached as a symbol rather than as a registered `constructor` member,
    /// for the reason [`crate::instance`]'s module docs give.
    fn nvs_core_object_map_new(_ctx, _args: [0]) {
        Ok(crate::instance::build(
            &CLASS,
            [
                Value::array(NvsArray::new()),
                Value::array(NvsArray::new()),
            ],
        ))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::set(K $key, V $value): void` — associates
    /// `$value` with `$key`, replacing whatever `$key` held.
    ///
    /// The key is written only where it is new: an insertion-ordered hash
    /// keeps a rewritten entry where it was, so re-setting an existing key
    /// would release and re-store an identical value for nothing, and would
    /// be the one place the two stores could drift apart.
    fn nvs_core_object_map_set(_ctx, args: [3]) {
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
                keys.set(NvsStr::new(&chain), key);
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
            // replace rather than leak — `NvsArray::set`'s own contract.
            values.set(NvsStr::new(&chain), value);
        })?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::get(K $key): ?V` — what `$key` holds, or `null`
    /// where the map holds no key identical to it.
    ///
    /// `?V` rather than a throwing read: § 9 says so, and [`CLASS`]'s own doc
    /// comment holds the reasoning. A map that genuinely holds `null` under a
    /// key is therefore indistinguishable from one that holds nothing — the
    /// same ambiguity `$a[$k] ?? $d` has over an array, and `has` is the
    /// spelling that resolves it.
    fn nvs_core_object_map_get(_ctx, args: [2]) {
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
                      the value handed back to Novis code needs one of its own"
        )]
        unsafe {
            held.retain();
        }
        Ok(held)
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::has(K $key): bool` — whether the map holds a
    /// key identical to `$key`.
    fn nvs_core_object_map_has(_ctx, args: [2]) {
        let receiver = map_of(args[0], "has")?;
        let (_, present) = at(receiver, args[1], "has")?;
        Ok(Value::bool(present))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::remove(K $key): void` — drops `$key` and what
    /// it held, and does nothing where the map holds no such key.
    ///
    /// [`store::vacate`] on each store in turn, and the two agree on which
    /// entry moves into the hole because their key sets are identical — this
    /// module's alignment invariant.
    fn nvs_core_object_map_remove(_ctx, args: [2]) {
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

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::count(): uint` — how many pairs the map holds.
    fn nvs_core_object_map_count(_ctx, args: [1]) {
        let receiver = map_of(args[0], "count")?;
        let keys = store::borrow(receiver, KEYS, &CLASS, "count")?;
        let count = u64::try_from(keys.count()).expect("an entry count fits in a `uint`");
        Ok(Value::uint(count))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::isEmpty(): bool` — whether the map holds no
    /// pairs.
    fn nvs_core_object_map_is_empty(_ctx, args: [1]) {
        let receiver = map_of(args[0], "isEmpty")?;
        let keys = store::borrow(receiver, KEYS, &CLASS, "isEmpty")?;
        Ok(Value::bool(keys.is_empty()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::keys(): array<K>` — every key, as a list.
    ///
    /// A list rather than a map keyed by anything: an `array<T>` keys on
    /// `string` (`rule:types/arrays`), which is the whole reason this class
    /// exists, so there is no key here to preserve. Position pairs with
    /// [`nvs_core_object_map_values`]'s.
    fn nvs_core_object_map_keys(_ctx, args: [1]) {
        let receiver = map_of(args[0], "keys")?;
        let keys = store::borrow(receiver, KEYS, &CLASS, "keys")?;
        Ok(Value::array(store::listed(&keys)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::values(): array<V>` — every value, as a list,
    /// in the order [`nvs_core_object_map_keys`] answers its keys.
    fn nvs_core_object_map_values(_ctx, args: [1]) {
        let receiver = map_of(args[0], "values")?;
        let values = store::borrow(receiver, VALUES, &CLASS, "values")?;
        Ok(Value::array(store::listed(&values)))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<K>::iterate(): Iterator<K>` — a cursor over a snapshot of the
    /// map's keys.
    ///
    /// Not a registered member: it is reached by name through this class's
    /// method table, so its receiver is **transferred** rather than borrowed —
    /// [`crate::cursor`]'s module docs own both halves of that.
    fn nvs_core_object_map_iterate(_ctx, args: [1]) {
        let cursor = map_of(args[0], nvs_runtime::sequence::ITERATE).and_then(|receiver| {
            let keys = store::borrow(receiver, KEYS, &CLASS, nvs_runtime::sequence::ITERATE)?;
            Ok(crate::cursor::over(store::listed(&keys)))
        });
        crate::cursor::consume(args[0]);
        cursor
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::clear(): void` — drops every pair.
    fn nvs_core_object_map_clear(_ctx, args: [1]) {
        let receiver = map_of(args[0], "clear")?;
        store::replace(receiver, KEYS);
        store::replace(receiver, VALUES);
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use nvs_runtime::{Ctx, NvsFn, call};

    /// Runs `member` with `map` as the receiver and `rest` past it.
    fn on(map: Value, member: NvsFn, rest: &[Value]) -> Value {
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
        let map = call(nvs_core_object_map_new, &mut ctx, &[]).expect("a fresh map does not throw");

        for n in 1..=3i64 {
            on(
                map,
                nvs_core_object_map_set,
                &[Value::int(n), Value::int(n * 10)],
            );
        }
        on(
            map,
            nvs_core_object_map_set,
            &[Value::int(2), Value::int(99)],
        );
        assert_eq!(on(map, nvs_core_object_map_count, &[]).as_uint(), Some(3));
        assert_eq!(ints(on(map, nvs_core_object_map_keys, &[])), vec![1, 2, 3]);
        assert_eq!(
            ints(on(map, nvs_core_object_map_values, &[])),
            vec![10, 99, 30]
        );

        on(map, nvs_core_object_map_remove, &[Value::int(1)]);
        assert_eq!(on(map, nvs_core_object_map_count, &[]).as_uint(), Some(2));
        let keys = ints(on(map, nvs_core_object_map_keys, &[]));
        let values = ints(on(map, nvs_core_object_map_values, &[]));
        for (key, value) in keys.iter().zip(&values) {
            let expected = if *key == 2 { 99 } else { key * 10 };
            assert_eq!(*value, expected, "key {key} lost its value");
        }
        assert_eq!(
            on(map, nvs_core_object_map_get, &[Value::int(1)]).as_int(),
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
