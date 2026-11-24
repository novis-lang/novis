//! `Core\ObjectMap<K, V>`, the map that finds a key with `==`, which replaces `SplObjectStorage` used as a
//! map and every `spl_object_id` side table.
//!
//! # Decision: two stores when the key bytes give no key back, one otherwise
//!
//! A map keyed by the chain or by an object's identity holds two stores,
//! `keys` and `values`, because neither store key gives the key back. A map
//! keyed by exactly `string` or `int` holds one store, the values under the
//! key itself. [`crate::identity_store`] chooses the keying, and
//! [`identity_store::Keying::keeps_keys`] says which of the two layouts a map
//! has.
//!
//! [`crate::identity_store`] keys a chain entry by the identity hash of the
//! value it holds, which answers a *set*'s question directly. A map has two
//! values per entry, and only the key is hashed, so a chain map writes both
//! stores under the key [`identity_store::locate`] derives from the `keys`
//! store alone.
//!
//! The two stay aligned because every mutation touches them the same way: an
//! insert appends to both at the same position, a re-`set` overwrites in
//! place (an insertion-ordered hash keeps a rewritten key where it was), and a
//! removal calls [`identity_store::vacate`] on each, which renames the same
//! chain tail to the same removed key because the two stores have identical
//! key sets. That alignment is what makes `keys()` and `values()` pair up
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
//!
//! A map keyed by exactly `string` or `int` has no chain, so it holds one
//! store: the values, under the key itself. Its `keys` slot keeps the empty
//! array, which is shared and allocates nothing, and `keys()` reads each key
//! back from the value store. [`crate::identity_store`]'s docs § *A `string`
//! or `int` key is stored as it is* own that choice.
//!
//! A map keyed by a type whose every value is an object, and none a value
//! class, keeps both stores,
//! under each key's identity key in place of the chain key, so the alignment
//! above holds for it too. `get` and `has` read the value store alone.
//! [`crate::identity_store`]'s docs § *An object key is its identity* own it.
//!
//! # Why the three collections are mutable
//!
//! `ObjectMap`, `ObjectSet` and `Heap` are the only mutable `Core` types: a
//! persistent structure would give up the O(log n) that is their reason to
//! exist. A `foreach` with a value binding alone over a map yields its *keys*,
//! because a key gives `get` its value while a value gives nothing back. A
//! `foreach` with a key binding yields each key with its value, through
//! `iterateKeyed()` and [`crate::cursor::KEYED_CLASS`]
//! (`rule:iteration/keyed-interfaces`). Over a set it yields its members, and
//! over a heap its elements in `pop` order. Each walk sees the collection as it was
//! when the walk began, so it is re-iterable and a mutation in the body does
//! not disturb it. A map's or set's walk shares its store, and a write in the
//! body separates the collection once ([`crate::cursor`]).
//! `Heap` is `Iterable` for that reason alone: without it, a heap's contents are
//! reachable only by emptying it.

use nvs_runtime::revoke::{self, MembershipKind};
use nvs_runtime::{Fault, NvsArray, NvsStr, ObjHeader, Value};

use crate::identity_store as store;
use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc, TextOrigin};

/// The class's fully-qualified name, as [`CoreTy::Instance`] spells it.
pub(crate) const NAME: &str = r"Core\ObjectMap";

/// The linker symbol `new Core\ObjectMap<K, V>()` lowers to — see
/// [`crate::registry::CONSTRUCTORS`], which is the roster `nvs-ir` reads.
pub(crate) const NEW_SYMBOL: &str = "nvs_core_object_map_new";

/// The constructor of a map keyed by exactly `string`
/// ([`store::Keying::Str`]), which [`crate::registry::constructor_symbol`]
/// names for that keying.
pub(crate) const NEW_STRING_SYMBOL: &str = "nvs_core_object_map_new_string";

/// The constructor of a map keyed by exactly `int` ([`store::Keying::Int`]).
pub(crate) const NEW_INT_SYMBOL: &str = "nvs_core_object_map_new_int";

/// The constructor of a map keyed by a type whose every value is an object
/// ([`store::Keying::Object`]).
pub(crate) const NEW_OBJECT_SYMBOL: &str = "nvs_core_object_map_new_object";

/// The symbol behind `Iterable<K>::iterate()`, reached by name through this
/// class's method table rather than as a registered member — see
/// [`crate::cursor`] and [`crate::instance`]'s dispatch roster.
pub(crate) const ITERATE_SYMBOL: &str = "nvs_core_object_map_iterate";

/// The symbol behind `KeyedIterable<K, V>::iterateKeyed()`, reached by name as
/// [`ITERATE_SYMBOL`] is.
pub(crate) const ITERATE_KEYED_SYMBOL: &str = "nvs_core_object_map_iterate_keyed";

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
    text: TextOrigin::Unmarked,
    symbol: NEW_SYMBOL,
    doc: Some(&CONSTRUCTOR_DOC),
};

/// `Core\ObjectMap<K, V>`, the identity-keyed map.
///
/// **`get` returns `?V`, not a throwing read**, because
/// these types have no subscript (`rule:iteration/two-interfaces`
/// rejects `ArrayAccess`) and so cannot offer the `$a[$k]` / `$a[$k] ?? $d`
/// pair that `array<T>` does, while `rule:core-api/shape-rules` R5 bans a `getOrNull` twin.
///
/// **A `foreach` over a map yields its keys**, which is what
/// `SplObjectStorage` yields and the only choice that loses nothing: a key
/// hands `get` back its value, while a value hands nothing back the key it was
/// stored under — and `rule:iteration/two-interfaces` gives a cursor no key binding to carry the
/// other half in. `values()` is the spelling for the other direction.
/// [`crate::cursor`] owns the mechanism and what a loop spends.
// rank: 290
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "set",
            names: &["key", "value"],
            params: &[CoreTy::Var("K"), CoreTy::Var("V")],
            defaults: &[],
            return_ty: CoreTy::Void,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_object_map_set",
            doc: Some(&SET_DOC),
        },
        CoreMethod {
            name: "get",
            names: &["key"],
            params: &[CoreTy::Var("K")],
            defaults: &[],
            return_ty: CoreTy::Nullable(&CoreTy::Var("V")),
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_object_map_get",
            doc: Some(&GET_DOC),
        },
        CoreMethod {
            name: "has",
            names: &["key"],
            params: &[CoreTy::Var("K")],
            defaults: &[],
            return_ty: CoreTy::Bool,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_object_map_has",
            doc: Some(&HAS_DOC),
        },
        CoreMethod {
            name: "remove",
            names: &["key"],
            params: &[CoreTy::Var("K")],
            defaults: &[],
            return_ty: CoreTy::Void,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_object_map_remove",
            doc: Some(&REMOVE_DOC),
        },
        CoreMethod {
            name: "count",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_object_map_count",
            doc: Some(&COUNT_DOC),
        },
        CoreMethod {
            name: "isEmpty",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_object_map_is_empty",
            doc: Some(&IS_EMPTY_DOC),
        },
        CoreMethod {
            name: "keys",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("K")),
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_object_map_keys",
            doc: Some(&KEYS_DOC),
        },
        CoreMethod {
            name: "values",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Array(&CoreTy::Var("V")),
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_object_map_values",
            doc: Some(&VALUES_DOC),
        },
        CoreMethod {
            name: "clear",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            text: TextOrigin::Unmarked,
            symbol: "nvs_core_object_map_clear",
            doc: Some(&CLEAR_DOC),
        },
    ],
    slots: &["keys", "values", "keying"],
    constants: &[],
};

/// `Core\ObjectMap`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "A map whose keys are objects or other values of any type. A key is found with \
            `==`. For most objects, only the same object matches. A `Core\\Time` value, such \
            as a `Date`, matches an equal value. Build one with `new Core\\ObjectMap<K, V>()`. A `foreach` over the map gives its keys in insertion order.",
};

/// `new Core\ObjectMap`'s reference card — `rule:core-api/reference-card`.
const CONSTRUCTOR_DOC: MethodDoc = MethodDoc {
    short: "Creates an empty `Core\\ObjectMap<K, V>`. Two keys are the same when they are \
            `==`.",
    params: &[],
    ret: "A new, empty map. A loop over it gives the keys in the order they were added.",
    errors: &[],
};

/// `Core\ObjectMap::set`'s reference card — `rule:core-api/reference-card`.
const SET_DOC: MethodDoc = MethodDoc {
    short: "Stores `$value` under `$key`. If `$key` already has a value, the new value replaces \
            it. A key matches a key that is `==` to it.",
    params: &[
        ParamDoc {
            name: "key",
            desc: "The key. It matches a key in the map that is `==` to it.",
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The value to store under `$key`.",
            shape: &[],
        },
    ],
    ret: "Nothing. A key that is set again keeps its first place in the order of the keys.",
    errors: &[],
};

/// `Core\ObjectMap::get`'s reference card — `rule:core-api/reference-card`.
const GET_DOC: MethodDoc = MethodDoc {
    short: "Returns the value stored under `$key`, as a `?V`. It does not throw for a missing \
            key. This map has no `$map[$key]` syntax, so `get` is the way to read a value.",
    params: &[ParamDoc {
        name: "key",
        desc: "The key to look up. It matches a key that is `==` to it.",
        shape: &[],
    }],
    ret: "The value stored under `$key`, or `null` if the map has no such key. A stored `null` \
          also returns `null`. Use `has` to tell the two cases apart.",
    errors: &[],
};

/// `Core\ObjectMap::has`'s reference card — `rule:core-api/reference-card`.
const HAS_DOC: MethodDoc = MethodDoc {
    short: "Checks whether the map has a key that is `==` to `$key`.",
    params: &[ParamDoc {
        name: "key",
        desc: "The key to look for. It matches a key that is `==` to it.",
        shape: &[],
    }],
    ret: "`true` if the key is in the map, also when its value is `null`. Otherwise `false`.",
    errors: &[],
};

/// `Core\ObjectMap::remove`'s reference card — `rule:core-api/reference-card`.
const REMOVE_DOC: MethodDoc = MethodDoc {
    short: "Deletes `$key` and its value from the map.",
    params: &[ParamDoc {
        name: "key",
        desc: "The key to delete. It matches a key that is `==` to it.",
        shape: &[],
    }],
    ret: "Nothing. If the map does not have the key, the call does nothing and does not throw.",
    errors: &[],
};

/// `Core\ObjectMap::count`'s reference card — `rule:core-api/reference-card`.
const COUNT_DOC: MethodDoc = MethodDoc {
    short: "Counts the keys in the map.",
    params: &[],
    ret: "The number of keys; `0` for an empty map.",
    errors: &[],
};

/// `Core\ObjectMap::isEmpty`'s reference card — `rule:core-api/reference-card`.
const IS_EMPTY_DOC: MethodDoc = MethodDoc {
    short: "Checks whether the map is empty.",
    params: &[],
    ret: "`true` for a map with no keys, `false` otherwise.",
    errors: &[],
};

/// `Core\ObjectMap::keys`'s reference card — `rule:core-api/reference-card`.
const KEYS_DOC: MethodDoc = MethodDoc {
    short: "Returns every key as a list, in the order the keys were added. A `foreach` over the \
            map uses the same order.",
    params: &[],
    ret: "A new `array<K>` list. It is empty for an empty map. Each position matches the same \
          position in `values()`.",
    errors: &[],
};

/// `Core\ObjectMap::values`'s reference card — `rule:core-api/reference-card`.
const VALUES_DOC: MethodDoc = MethodDoc {
    short: "Returns every value as a list, in the same order as `keys()` returns the keys.",
    params: &[],
    ret: "A new `array<V>` list. It is empty for an empty map. Each position matches the same \
          position in `keys()`.",
    errors: &[],
};

/// `Core\ObjectMap::clear`'s reference card — `rule:core-api/reference-card`.
const CLEAR_DOC: MethodDoc = MethodDoc {
    short: "Deletes every key and value. The map is then empty.",
    params: &[],
    ret: "Nothing. You can add new keys to the same map afterwards.",
    errors: &[],
};

/// [`CLASS`]'s slots, by index — the two stores this module's docs describe.
/// A map keyed by `string` or `int` keeps the empty array in [`KEYS`].
const KEYS: usize = 0;
/// See [`KEYS`].
pub(crate) const VALUES: usize = 1;
/// The map's [`store::Keying`], written once by its constructor.
const KEYING: usize = 2;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    Some(nvs_runtime::Entry::borrowed(match symbol {
        NEW_SYMBOL => (nvs_core_object_map_new as *const ()).cast(),
        NEW_STRING_SYMBOL => (nvs_core_object_map_new_string as *const ()).cast(),
        NEW_INT_SYMBOL => (nvs_core_object_map_new_int as *const ()).cast(),
        NEW_OBJECT_SYMBOL => (nvs_core_object_map_new_object as *const ()).cast(),
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
        ITERATE_KEYED_SYMBOL => (nvs_core_object_map_iterate_keyed as *const ()).cast(),
        _ => return None,
    }))
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
fn at(
    receiver: *mut ObjHeader,
    key: Value,
    member: &str,
) -> Result<(store::ChainKey, bool), Fault> {
    let keys = store::borrow(receiver, KEYS, &CLASS, member)?;
    Ok(store::locate(&keys, key))
}

/// `receiver`'s keying, from its `keying` slot.
fn keying_of(receiver: *mut ObjHeader) -> store::Keying {
    store::Keying::of(crate::instance::slot(receiver, KEYING))
}

/// The keying a write of `key` runs under. A direct map given a key of
/// another tag becomes a chain map first, so the write lands where the chain
/// puts it — [`crate::identity_store`]'s docs § *A key whose tag is not the
/// keying's*.
///
/// # Errors
///
/// [`store::borrow`]'s, unchanged.
fn keying_for_write(
    receiver: *mut ObjHeader,
    key: Value,
    member: &str,
) -> Result<store::Keying, Fault> {
    let keying = keying_of(receiver);
    if keying.admits(key) {
        return Ok(keying);
    }
    let (keys, values) = {
        let values = store::borrow(receiver, VALUES, &CLASS, member)?;
        if keying == store::Keying::Object {
            let keys = store::borrow(receiver, KEYS, &CLASS, member)?;
            store::rebuild_pairs(&keys, &values, store::Keying::Chain)
        } else {
            store::rechain_pairs(&values, keying)
        }
    };
    crate::instance::set_slot(receiver, KEYS, Value::array(keys));
    crate::instance::set_slot(receiver, VALUES, Value::array(values));
    crate::instance::set_slot(receiver, KEYING, store::Keying::Chain.value());
    Ok(store::Keying::Chain)
}

/// Every key of `receiver`, as a fresh list in insertion order: the key store's
/// values for a chain or identity-keyed map, and the value store's own keys for
/// a `string` or `int` one.
///
/// # Errors
///
/// [`store::borrow`]'s, unchanged.
fn keys_of(receiver: *mut ObjHeader, member: &str) -> Result<NvsArray, Fault> {
    let keying = keying_of(receiver);
    if keying.keeps_keys() {
        let keys = store::borrow(receiver, KEYS, &CLASS, member)?;
        return Ok(store::listed(&keys));
    }
    let values = store::borrow(receiver, VALUES, &CLASS, member)?;
    Ok(store::keys_listed(&values, keying))
}

/// A fresh empty map keyed by `keying`.
fn built(keying: store::Keying) -> Value {
    crate::instance::build(
        &CLASS,
        [
            Value::array(NvsArray::new()),
            Value::array(NvsArray::new()),
            keying.value(),
        ],
    )
}

nvs_runtime::nvs_helper! {
    /// `new Core\ObjectMap<K, V>()` — a fresh empty map.
    ///
    /// Reached as a symbol rather than as a registered `constructor` member,
    /// for the reason [`crate::instance`]'s module docs give.
    fn nvs_core_object_map_new(_ctx, _args: [0]) {
        Ok(built(store::Keying::Chain))
    }
}

nvs_runtime::nvs_helper! {
    /// `new Core\ObjectMap<string, V>()` — a fresh empty map keyed by the
    /// string itself.
    fn nvs_core_object_map_new_string(_ctx, _args: [0]) {
        Ok(built(store::Keying::Str))
    }
}

nvs_runtime::nvs_helper! {
    /// `new Core\ObjectMap<int, V>()` — a fresh empty map keyed by the `int`
    /// itself.
    fn nvs_core_object_map_new_int(_ctx, _args: [0]) {
        Ok(built(store::Keying::Int))
    }
}

nvs_runtime::nvs_helper! {
    /// `new Core\ObjectMap<K, V>()` for a `K` whose every value is an object —
    /// a fresh empty map keyed by each key's identity.
    fn nvs_core_object_map_new_object(_ctx, _args: [0]) {
        Ok(built(store::Keying::Object))
    }
}

/// Rebuilds an identity-keyed map's two stores under its keys' own
/// identities, after a graph copy made new objects of them. A chain or direct
/// map is left as it is. The class's descriptor calls this
/// ([`nvs_runtime::ClassTable::set_rekey`]).
pub(crate) fn rekey(map: Value) {
    let Some(receiver) = map.obj_ptr() else {
        return;
    };
    if keying_of(receiver) != store::Keying::Object {
        return;
    }
    let (Ok(keys), Ok(values)) = (
        store::borrow(receiver, KEYS, &CLASS, "rekey"),
        store::borrow(receiver, VALUES, &CLASS, "rekey"),
    ) else {
        return;
    };
    let (keys, values) = store::rebuild_pairs(&keys, &values, store::Keying::Object);
    crate::instance::set_slot(receiver, KEYS, Value::array(keys));
    crate::instance::set_slot(receiver, VALUES, Value::array(values));
}

/// The map's [`nvs_runtime::KeysPrivate`] hook: a chain or identity key is
/// the store's own and is never shown, and a `string` or `int` key is the
/// program's own key and is shown.
pub(crate) fn keys_private(map: Value) -> bool {
    map.obj_ptr()
        .is_none_or(|receiver| keying_of(receiver).keeps_keys())
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::set(K $key, V $value): void` — associates
    /// `$value` with `$key`, replacing whatever `$key` held.
    ///
    /// Where a revocable class exists, a revocable key or value gets a
    /// membership of this map, and a replaced value loses its own.
    fn nvs_core_object_map_set(_ctx, args: [3]) {
        let receiver = map_of(args[0], "set")?;
        let (key, value) = (args[1], args[2]);
        if !revoke::in_use() {
            write_pair(receiver, key, value)?;
            return Ok(Value::null());
        }
        // `rule:core-api/collections-drop-revoked-objects`: a map never holds a
        // revoked object, so a pair with one is dropped at once.
        if revoke::is_revoked_value(key) || revoke::is_revoked_value(value) {
            return Ok(Value::null());
        }
        // The value this write replaces is kept alive across the write, so
        // its membership can be removed once the write has landed.
        let old = held_under(receiver, key, "set")?;
        if let Some(old) = old {
            #[expect(unsafe_code, reason = "borrowed from the store, which holds it")]
            unsafe {
                old.retain();
            }
        }
        let written = write_pair(receiver, key, value).and_then(|()| {
            // A write the memory ceiling refused leaves the map as it was, so
            // it links and unlinks nothing.
            Ok(held_under(receiver, key, "set")?.is_some_and(|held| same(held, value)))
        });
        if let Ok(true) = written {
            if let Some(old) = old {
                revoke::unlink(old, receiver, MembershipKind::MapValue, key);
            } else {
                revoke::link(key, receiver, MembershipKind::MapKey, Value::null());
            }
            revoke::link(value, receiver, MembershipKind::MapValue, key);
        }
        if let Some(old) = old {
            #[expect(unsafe_code, reason = "gives up the reference taken above")]
            unsafe {
                old.release();
            }
        }
        written?;
        Ok(Value::null())
    }
}

/// The value `receiver` holds under `key`, borrowed from its store, or `None`
/// when it holds no key identical to `key`.
///
/// # Errors
///
/// [`store::borrow`]'s, unchanged.
fn held_under(receiver: *mut ObjHeader, key: Value, member: &str) -> Result<Option<Value>, Fault> {
    let keying = keying_of(receiver);
    if keying == store::Keying::Chain {
        let (chain, present) = at(receiver, key, member)?;
        if !present {
            return Ok(None);
        }
        let values = store::borrow(receiver, VALUES, &CLASS, member)?;
        return Ok(values.get(&chain));
    }
    let values = store::borrow(receiver, VALUES, &CLASS, member)?;
    Ok(store::find(&values, keying, key))
}

/// Calls `visit` with each key and value `receiver` holds, both borrowed. A
/// map whose stores are gone, as in a map being freed by the cycle collector,
/// visits nothing.
fn each_pair(receiver: *mut ObjHeader, member: &str, mut visit: impl FnMut(Value, Value)) {
    let keying = keying_of(receiver);
    let Ok(values) = store::borrow(receiver, VALUES, &CLASS, member) else {
        return;
    };
    let keys = if keying.keeps_keys() {
        let Ok(keys) = store::borrow(receiver, KEYS, &CLASS, member) else {
            return;
        };
        Some(keys)
    } else {
        None
    };
    let mut from = 0usize;
    while let Some(slot) = values.next_slot(from) {
        from = slot + 1;
        let Some(value) = values.value_at(slot) else {
            continue;
        };
        if let Some(keys) = &keys {
            let key = values
                .key_at(slot)
                .and_then(|bytes| keys.get(bytes.as_bytes()))
                .unwrap_or_else(Value::null);
            visit(key, value);
        } else if let Some(key) = store::key_value(&values, slot, keying) {
            visit(key, value);
            #[expect(unsafe_code, reason = "`key_value` returned a key this frame owns")]
            unsafe {
                key.release();
            }
        }
    }
}

/// Whether `a` and `b` are the same value: the same object, or equal bits for
/// anything else. It is identity and not `==`, even for a `Core\Time` value,
/// because both callers ask whether the stored value is the object a write or
/// a revoke holds, whose membership is linked to that one object. Finding a
/// key reads `==` through [`store::locate`].
fn same(a: Value, b: Value) -> bool {
    match (a.obj_ptr(), b.obj_ptr()) {
        (Some(a), Some(b)) => std::ptr::eq(a, b),
        _ => a.bits() == b.bits(),
    }
}

/// Takes `key`'s entry out of `receiver`, and the memberships of its key and
/// its value with it. What `remove` and a revoke run.
///
/// With `only`, the entry is taken out only when the value under `key` is
/// that object: a revoked value removes its own entry and never another.
///
/// # Errors
///
/// [`store::borrow`]'s and [`store::edit`]'s, unchanged.
fn remove_entry(
    receiver: *mut ObjHeader,
    key: Value,
    only: Option<Value>,
    member: &str,
) -> Result<(), Fault> {
    // Only a program with a revocable class looks the value up first, and
    // `only` comes from a revoke, so it is always such a program.
    if revoke::in_use() {
        let held = held_under(receiver, key, member)?;
        if let Some(only) = only
            && !held.is_some_and(|held| same(held, only))
        {
            return Ok(());
        }
        if let Some(value) = held {
            revoke::unlink(value, receiver, MembershipKind::MapValue, key);
            revoke::unlink(key, receiver, MembershipKind::MapKey, Value::null());
        }
    }
    remove_pair(receiver, key, member)
}

/// The map's [`revoke::CollectionHooks`]: a revoked key or value takes its
/// entry out with `remove`'s own code.
pub(crate) const HOOKS: revoke::CollectionHooks = revoke::CollectionHooks {
    drop_entry: |map, object, kind, key| {
        let Some(receiver) = map.obj_ptr() else {
            return;
        };
        let (key, only) = if kind == MembershipKind::MapKey {
            (object, None)
        } else {
            (key, Some(object))
        };
        let _ = remove_entry(receiver, key, only, "revoke");
    },
    unlink_all: |map| {
        let Some(receiver) = map.obj_ptr() else {
            return;
        };
        each_pair(receiver, "release", |key, value| {
            revoke::unlink(key, receiver, MembershipKind::MapKey, Value::null());
            revoke::unlink(value, receiver, MembershipKind::MapValue, key);
        });
    },
    link_all: |map| {
        let Some(receiver) = map.obj_ptr() else {
            return;
        };
        each_pair(receiver, "copy", |key, value| {
            revoke::link(key, receiver, MembershipKind::MapKey, Value::null());
            revoke::link(value, receiver, MembershipKind::MapValue, key);
        });
    },
};

/// Writes `value` under `key`, what `set` does once a revoked object is ruled
/// out.
///
/// The key is written only where it is new: an insertion-ordered hash keeps a
/// rewritten entry where it was, so re-setting an existing key would release
/// and re-store an identical value for nothing, and would be the one place the
/// two stores could drift apart.
///
/// # Errors
///
/// [`store::borrow`]'s and [`store::edit`]'s, unchanged.
fn write_pair(receiver: *mut ObjHeader, key: Value, value: Value) -> Result<(), Fault> {
    {
        let keying = keying_for_write(receiver, key, "set")?;
        if keying == store::Keying::Object {
            let present = {
                let values = store::borrow(receiver, VALUES, &CLASS, "set")?;
                store::place(&values, keying, key).1
            };
            for (slot, held) in [(KEYS, key), (VALUES, value)] {
                if slot == KEYS && present {
                    continue;
                }
                store::edit(receiver, slot, &CLASS, "set", |stored| {
                    let (place, present) = store::place(stored, keying, key);
                    #[expect(
                        unsafe_code,
                        reason = "the argument is borrowed from the caller's frame, \
                                  so the copy the store keeps needs a reference of \
                                  its own"
                    )]
                    unsafe {
                        held.retain();
                    }
                    store::put(stored, &place, present, held);
                })?;
            }
            return Ok(());
        }
        if !keying.keeps_keys() {
            store::edit(receiver, VALUES, &CLASS, "set", |values| {
                let (place, present) = store::place(values, keying, key);
                #[expect(
                    unsafe_code,
                    reason = "the argument is borrowed from the caller's frame, so \
                              the copy the store keeps needs a reference of its own"
                )]
                unsafe {
                    value.retain();
                }
                store::put(values, &place, present, value);
            })?;
            return Ok(());
        }
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
            // replace rather than leak. An existing entry is overwritten
            // through the chain key on the stack, so a re-`set` allocates
            // nothing; the two stores share a key set, so `present` means the
            // value store has the key too.
            if present {
                let unplaced = values.overwrite(&chain, value);
                debug_assert!(unplaced.is_ok(), "the two stores share one key set");
                if let Err(value) = unplaced {
                    values.set(NvsStr::new(&chain), value);
                }
            } else {
                values.set(NvsStr::new(&chain), value);
            }
        })?;
        Ok(())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::get(K $key): ?V` — what `$key` holds, or `null`
    /// where the map holds no key identical to it.
    ///
    /// `?V` rather than a throwing read: [`CLASS`]'s own doc
    /// comment holds the reasoning. A map that genuinely holds `null` under a
    /// key is therefore indistinguishable from one that holds nothing — the
    /// same ambiguity `$a[$k] ?? $d` has over an array, and `has` is the
    /// spelling that resolves it.
    fn nvs_core_object_map_get(_ctx, args: [2]) {
        let receiver = map_of(args[0], "get")?;
        let keying = keying_of(receiver);
        let held = if keying == store::Keying::Chain {
            let (chain, present) = at(receiver, args[1], "get")?;
            if !present {
                return Ok(Value::null());
            }
            let values = store::borrow(receiver, VALUES, &CLASS, "get")?;
            values.get(&chain).unwrap_or_else(Value::null)
        } else {
            let values = store::borrow(receiver, VALUES, &CLASS, "get")?;
            store::find(&values, keying, args[1]).unwrap_or_else(Value::null)
        };
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
        let keying = keying_of(receiver);
        if keying == store::Keying::Chain {
            let (_, present) = at(receiver, args[1], "has")?;
            return Ok(Value::bool(present));
        }
        let values = store::borrow(receiver, VALUES, &CLASS, "has")?;
        Ok(Value::bool(store::contains(&values, keying, args[1])))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::remove(K $key): void` — drops `$key` and what
    /// it held, and does nothing where the map holds no such key.
    ///
    /// [`store::vacate`] on each store in turn, and the two agree on which
    /// entry takes the removed key because their key sets are identical —
    /// this module's alignment invariant.
    fn nvs_core_object_map_remove(_ctx, args: [2]) {
        let receiver = map_of(args[0], "remove")?;
        remove_entry(receiver, args[1], None, "remove")?;
        Ok(Value::null())
    }
}

/// Takes `key`'s entry out of both stores, and does nothing where the map holds
/// no such key.
///
/// # Errors
///
/// [`store::borrow`]'s and [`store::edit`]'s, unchanged.
fn remove_pair(receiver: *mut ObjHeader, key: Value, member: &str) -> Result<(), Fault> {
    {
        let keying = keying_of(receiver);
        if keying != store::Keying::Chain {
            let present = {
                let values = store::borrow(receiver, VALUES, &CLASS, member)?;
                store::contains(&values, keying, key)
            };
            if !present {
                return Ok(());
            }
            // An identity-keyed map removes the key from its key store too.
            let slots: &[usize] = if keying.keeps_keys() {
                &[KEYS, VALUES]
            } else {
                &[VALUES]
            };
            for slot in slots {
                store::edit(receiver, *slot, &CLASS, member, |stored| {
                    let (place, present) = store::place(stored, keying, key);
                    if present {
                        store::remove(stored, &place);
                    }
                })?;
            }
            return Ok(());
        }
        let (chain, present) = at(receiver, key, member)?;
        if !present {
            return Ok(());
        }
        for slot in [KEYS, VALUES] {
            store::edit(receiver, slot, &CLASS, member, |held| {
                store::vacate(held, &chain);
            })?;
        }
        Ok(())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::count(): uint` — how many pairs the map holds.
    fn nvs_core_object_map_count(_ctx, args: [1]) {
        let receiver = map_of(args[0], "count")?;
        let values = store::borrow(receiver, VALUES, &CLASS, "count")?;
        let count = u64::try_from(values.count()).expect("an entry count fits in a `uint`");
        Ok(Value::uint(count))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::isEmpty(): bool` — whether the map holds no
    /// pairs.
    fn nvs_core_object_map_is_empty(_ctx, args: [1]) {
        let receiver = map_of(args[0], "isEmpty")?;
        let values = store::borrow(receiver, VALUES, &CLASS, "isEmpty")?;
        Ok(Value::bool(values.is_empty()))
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
        Ok(Value::array(keys_of(receiver, "keys")?))
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
    /// `Iterable<K>::iterate(): Iterator<K>` — a cursor over the map's own
    /// store, which yields each key: the key store's values for a chain or
    /// identity-keyed map, and the value store's keys, as a `string` or an
    /// `int`, for a direct one. It takes one reference on the store and copies
    /// nothing; a write in the loop separates the map ([`crate::cursor`]).
    ///
    /// Not a registered member: it is reached by name through this class's
    /// method table, so its receiver is **transferred** rather than borrowed —
    /// [`crate::cursor`]'s module docs own both halves of that.
    fn nvs_core_object_map_iterate(_ctx, args: [1]) {
        let cursor = map_of(args[0], nvs_runtime::sequence::ITERATE).and_then(|receiver| {
            let (slot, yields) = match keying_of(receiver) {
                store::Keying::Chain | store::Keying::Object => {
                    (KEYS, crate::cursor::Yield::Value)
                }
                store::Keying::Str => (VALUES, crate::cursor::Yield::StrKey),
                store::Keying::Int => (VALUES, crate::cursor::Yield::IntKey),
            };
            let walked = store::borrow(receiver, slot, &CLASS, nvs_runtime::sequence::ITERATE)?;
            // A loop over the key store skips an entry whose value was revoked
            // during the loop, so it holds the value store too.
            let pairs = if slot == KEYS {
                let values =
                    store::borrow(receiver, VALUES, &CLASS, nvs_runtime::sequence::ITERATE)?;
                Some(NvsArray::clone(&values))
            } else {
                None
            };
            Ok(crate::cursor::paired(NvsArray::clone(&walked), yields, pairs))
        });
        crate::cursor::consume(args[0]);
        cursor
    }
}

nvs_runtime::nvs_helper! {
    /// `KeyedIterable<K, V>::iterateKeyed(): KeyedIterator<K, V>` — a keyed
    /// cursor over the map's own stores, which gives each key with its value.
    /// It takes one reference on each store it reads and copies nothing, so a
    /// write in the loop separates the map ([`crate::cursor`]).
    ///
    /// Reached by name, as [`nvs_core_object_map_iterate`] is, so its receiver
    /// is transferred.
    fn nvs_core_object_map_iterate_keyed(_ctx, args: [1]) {
        let member = nvs_runtime::sequence::ITERATE_KEYED;
        let cursor = map_of(args[0], member).and_then(|receiver| {
            let values = store::borrow(receiver, VALUES, &CLASS, member)?;
            let (keys, keys_from) = match keying_of(receiver) {
                store::Keying::Chain | store::Keying::Object => {
                    let keys = store::borrow(receiver, KEYS, &CLASS, member)?;
                    (NvsArray::clone(&keys), crate::cursor::Yield::Value)
                }
                store::Keying::Str => (NvsArray::new(), crate::cursor::Yield::StrKey),
                store::Keying::Int => (NvsArray::new(), crate::cursor::Yield::IntKey),
            };
            Ok(crate::cursor::keyed(keys, NvsArray::clone(&values), keys_from))
        });
        crate::cursor::consume(args[0]);
        cursor
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectMap<K, V>::clear(): void` — drops every pair.
    fn nvs_core_object_map_clear(_ctx, args: [1]) {
        let receiver = map_of(args[0], "clear")?;
        if revoke::in_use() {
            (HOOKS.unlink_all)(args[0]);
        }
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
    // covers: Core\ObjectMap::set, Core\ObjectMap::get, Core\ObjectMap::remove, Core\ObjectMap::count, Core\ObjectMap::keys, Core\ObjectMap::values
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

    /// `has` and `isEmpty` follow the keys and never the values: a key
    /// holding `null` is present and makes the map non-empty, and only
    /// `remove` or `clear` takes it out — a map that read a `null` value as
    /// an absent key would answer `false` and `true` on the second block.
    // covers: Core\ObjectMap::has, Core\ObjectMap::isEmpty
    #[test]
    fn has_and_is_empty_follow_the_keys_and_not_the_values() {
        let mut ctx = Ctx::buffered();
        let map = call(nvs_core_object_map_new, &mut ctx, &[]).expect("a fresh map does not throw");
        let has = |key: i64| on(map, nvs_core_object_map_has, &[Value::int(key)]).as_bool();
        let empty = || on(map, nvs_core_object_map_is_empty, &[]).as_bool();

        assert_eq!((has(1), empty()), (Some(false), Some(true)));

        on(
            map,
            nvs_core_object_map_set,
            &[Value::int(1), Value::null()],
        );
        assert_eq!(
            (has(1), has(2), empty()),
            (Some(true), Some(false), Some(false))
        );
        assert_eq!(
            on(map, nvs_core_object_map_get, &[Value::int(1)]).tag(),
            Some(nvs_runtime::Tag::Null)
        );

        on(map, nvs_core_object_map_remove, &[Value::int(2)]);
        assert_eq!(
            (has(1), empty()),
            (Some(true), Some(false)),
            "an absent key removes nothing"
        );
        on(map, nvs_core_object_map_remove, &[Value::int(1)]);
        assert_eq!((has(1), empty()), (Some(false), Some(true)));

        on(
            map,
            nvs_core_object_map_set,
            &[Value::int(2), Value::int(20)],
        );
        on(map, nvs_core_object_map_clear, &[]);
        assert_eq!((has(2), empty()), (Some(false), Some(true)));

        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference the constructor produced"
        )]
        unsafe {
            map.release();
        }
    }

    /// `clear` releases the reference the map held on every key and every
    /// value, and leaves a map that takes the next write like a fresh one —
    /// a clear that swapped the stores without releasing the old ones would
    /// leave both counts at two.
    // covers: Core\ObjectMap::clear
    #[test]
    fn clear_releases_every_key_and_value_and_the_map_takes_a_write_after() {
        let mut ctx = Ctx::buffered();
        let map = call(nvs_core_object_map_new, &mut ctx, &[]).expect("a fresh map does not throw");
        let key = Value::array(NvsArray::new());
        let value = Value::array(NvsArray::new());
        let counts = || {
            #[expect(
                unsafe_code,
                reason = "this frame owns one reference to each array, so both are live"
            )]
            unsafe {
                (
                    NvsArray::refcount_of(key.array_ptr().expect("an array")),
                    NvsArray::refcount_of(value.array_ptr().expect("an array")),
                )
            }
        };

        on(map, nvs_core_object_map_set, &[key, value]);
        assert_eq!(counts(), (2, 2), "the map keeps a reference to each");

        on(map, nvs_core_object_map_clear, &[]);
        assert_eq!(counts(), (1, 1), "clear gives both references back");
        assert_eq!(on(map, nvs_core_object_map_count, &[]).as_uint(), Some(0));
        on(map, nvs_core_object_map_clear, &[]);
        assert_eq!(counts(), (1, 1), "a second clear releases nothing twice");

        on(
            map,
            nvs_core_object_map_set,
            &[Value::int(7), Value::int(70)],
        );
        assert_eq!(on(map, nvs_core_object_map_count, &[]).as_uint(), Some(1));
        assert_eq!(ints(on(map, nvs_core_object_map_values, &[])), vec![70]);

        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference each of the three was built with"
        )]
        unsafe {
            map.release();
            key.release();
            value.release();
        }
    }

    /// A live graph copy of an object-keyed map and set, while the originals
    /// stay alive, makes new key objects, so the copies are re-keyed under
    /// them. The copies find the copied keys and not the originals; without
    /// the re-key they would find only the originals.
    // covers: Core\ObjectMap::has, Core\ObjectMap::get, Core\ObjectSet::has
    #[test]
    fn a_graph_copy_of_an_object_keyed_map_and_set_finds_the_copied_keys() {
        let mut ctx = Ctx::buffered();
        let map = call(nvs_core_object_map_new_object, &mut ctx, &[])
            .expect("a fresh map does not throw");
        let set = call(crate::objset::nvs_core_object_set_new_object, &mut ctx, &[])
            .expect("a fresh set does not throw");
        // Two objects to key by: any instance is one, so two empty sets.
        let first = call(crate::objset::nvs_core_object_set_new, &mut ctx, &[])
            .expect("a fresh set does not throw");
        let second = call(crate::objset::nvs_core_object_set_new, &mut ctx, &[])
            .expect("a fresh set does not throw");
        on(map, nvs_core_object_map_set, &[first, Value::int(1)]);
        on(map, nvs_core_object_map_set, &[second, Value::int(2)]);
        on(set, crate::objset::nvs_core_object_set_add, &[first]);
        on(set, crate::objset::nvs_core_object_set_add, &[second]);

        // One graph, so the copy shares each copied key between the copied
        // collections and the list.
        let mut graph = NvsArray::new();
        for value in [map, set, first, second] {
            #[expect(
                unsafe_code,
                reason = "the list keeps a reference of its own, and this frame keeps its own"
            )]
            unsafe {
                value.retain();
            }
            graph.append(value);
        }
        let graph = Value::array(graph);
        let into = nvs_runtime::Receiving::over(None, Some(crate::instance::core_class_desc));
        let copied = nvs_runtime::copy_graph_to(graph, &into).expect("the graph crosses");
        let held = crate::arr::borrowed(copied.array_ptr().expect("the copy is a list"));
        let at = |slot: usize| held.value_at(slot).expect("the copy has four entries");
        let (map_copy, set_copy, first_copy, second_copy) = (at(0), at(1), at(2), at(3));
        assert_ne!(
            first_copy.bits(),
            first.bits(),
            "a key still held here is copied, not adopted"
        );

        let has = |map: Value, key: Value| on(map, nvs_core_object_map_has, &[key]).as_bool();
        assert_eq!(has(map_copy, first_copy), Some(true));
        assert_eq!(has(map_copy, second_copy), Some(true));
        assert_eq!(has(map_copy, first), Some(false));
        assert_eq!(has(map_copy, second), Some(false));
        assert_eq!(
            on(map_copy, nvs_core_object_map_get, &[second_copy]).as_int(),
            Some(2)
        );
        assert_eq!(has(map, first), Some(true), "the original keeps its keys");

        let member = |set: Value, key: Value| {
            on(set, crate::objset::nvs_core_object_set_has, &[key]).as_bool()
        };
        assert_eq!(member(set_copy, first_copy), Some(true));
        assert_eq!(member(set_copy, second_copy), Some(true));
        assert_eq!(member(set_copy, first), Some(false));
        assert_eq!(
            member(set, second),
            Some(true),
            "the original keeps its members"
        );

        #[expect(
            unsafe_code,
            reason = "this frame owns one reference to each of the five"
        )]
        unsafe {
            copied.release();
            map.release();
            set.release();
            first.release();
            second.release();
        }
    }
}
