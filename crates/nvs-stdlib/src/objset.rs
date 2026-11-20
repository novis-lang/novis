//! `Core\ObjectSet<T>`, the identity-keyed set, which replaces `SplObjectStorage` used as a set.
//!
//! One store slot, holding the store [`crate::identity_store`] describes: a
//! member is an entry under its own key, so `has` is a lookup, `count` is the
//! store's own count, and nothing here decides anything about identity that
//! `nvs_runtime::identity` has not already decided. The key is the chain key,
//! the member itself for a set of exactly `string` or `int`, or the member's
//! identity key for a set of a type whose every value is an object; the
//! `keying` slot says which.

use nvs_runtime::{Fault, NvsArray, ObjHeader, Value};

use crate::identity_store as store;
use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc};

/// The class's fully-qualified name, as [`CoreTy::Instance`] spells it.
pub(crate) const NAME: &str = r"Core\ObjectSet";

/// The linker symbol `new Core\ObjectSet<T>()` lowers to — see
/// [`crate::registry::CONSTRUCTORS`], which is the roster `nvs-ir` reads.
pub(crate) const NEW_SYMBOL: &str = "nvs_core_object_set_new";

/// The constructor of a set of exactly `string` ([`store::Keying::Str`]),
/// which [`crate::registry::constructor_symbol`] names for that keying.
pub(crate) const NEW_STRING_SYMBOL: &str = "nvs_core_object_set_new_string";

/// The constructor of a set of exactly `int` ([`store::Keying::Int`]).
pub(crate) const NEW_INT_SYMBOL: &str = "nvs_core_object_set_new_int";

/// The constructor of a set of a type whose every value is an object
/// ([`store::Keying::Object`]).
pub(crate) const NEW_OBJECT_SYMBOL: &str = "nvs_core_object_set_new_object";

/// The symbol behind `Iterable<T>::iterate()`, reached by name through this
/// class's method table rather than as a registered member — see
/// [`crate::cursor`] and [`crate::instance`]'s dispatch roster.
pub(crate) const ITERATE_SYMBOL: &str = "nvs_core_object_set_iterate";

/// `new Core\ObjectSet<T>()` — the constructor [`crate::registry::CONSTRUCTORS`]
/// registers, which takes nothing: a set's order is its insertion order and
/// its identity is `nvs_runtime::identity`'s, so there is nothing to give it.
pub(crate) const NEW: CoreMethod = CoreMethod {
    name: "constructor",
    names: &[],
    params: &[],
    defaults: &[],
    return_ty: CoreTy::Instance(NAME),
    symbol: NEW_SYMBOL,
    doc: Some(&CONSTRUCTOR_DOC),
};

/// `Core\ObjectSet<T>`, the identity-keyed set.
///
/// Every member is an instance member: a set is reached through a value, and
/// the only static entry point is the constructor, which is not a member at
/// all ([`crate::registry::CONSTRUCTORS`]).
///
/// **A `foreach` over a set yields its members**, in insertion order — the one
/// thing a set holds. [`crate::cursor`] owns the mechanism and what a loop
/// spends.
// rank: 300
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    doc: Some(&CARD),
    methods: &[],
    instance: &[
        CoreMethod {
            name: "add",
            names: &["value"],
            params: &[CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_object_set_add",
            doc: Some(&ADD_DOC),
        },
        CoreMethod {
            name: "has",
            names: &["value"],
            params: &[CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_object_set_has",
            doc: Some(&HAS_DOC),
        },
        CoreMethod {
            name: "remove",
            names: &["value"],
            params: &[CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_object_set_remove",
            doc: Some(&REMOVE_DOC),
        },
        CoreMethod {
            name: "count",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "nvs_core_object_set_count",
            doc: Some(&COUNT_DOC),
        },
        CoreMethod {
            name: "isEmpty",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "nvs_core_object_set_is_empty",
            doc: Some(&IS_EMPTY_DOC),
        },
        CoreMethod {
            name: "union",
            names: &["other"],
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_object_set_union",
            doc: Some(&UNION_DOC),
        },
        CoreMethod {
            name: "intersect",
            names: &["other"],
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_object_set_intersect",
            doc: Some(&INTERSECT_DOC),
        },
        CoreMethod {
            name: "diff",
            names: &["other"],
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "nvs_core_object_set_diff",
            doc: Some(&DIFF_DOC),
        },
        CoreMethod {
            name: "clear",
            names: &[],
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_object_set_clear",
            doc: Some(&CLEAR_DOC),
        },
    ],
    slots: &["entries", "keying"],
    constants: &[],
};

/// `Core\ObjectSet`'s class card — `rule:core-api/reference-card`.
const CARD: ClassDoc = ClassDoc {
    short: "A set of objects or other values of any type, each one at most once. A value is \
            matched by identity: the same object, not an equal one. Build one with `new \
            Core\\ObjectSet<T>()`. A `foreach` over the set gives its values in insertion order.",
};

/// `new Core\ObjectSet`'s reference card — `rule:core-api/reference-card`.
const CONSTRUCTOR_DOC: MethodDoc = MethodDoc {
    short: "Builds an empty `Core\\ObjectSet<T>`. Two entries are the same only when they are the \
            same object.",
    params: &[],
    ret: "A new empty set. A `foreach` over it gives its values in insertion order.",
    errors: &[],
};

/// `Core\ObjectSet::add`'s reference card — `rule:core-api/reference-card`.
const ADD_DOC: MethodDoc = MethodDoc {
    short: "Adds `$value` to the set, unless the set already contains that same value.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to add, matched by identity.",
        shape: &[],
    }],
    ret: "Nothing. If the set already contains the value, the set does not change. Use `has` \
          to check whether a value is in the set.",
    errors: &[],
};

/// `Core\ObjectSet::has`'s reference card — `rule:core-api/reference-card`.
const HAS_DOC: MethodDoc = MethodDoc {
    short: "Checks whether the set contains `$value`, matched by identity.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to look for, matched by identity.",
        shape: &[],
    }],
    ret: "`true` when the value is in the set, `false` otherwise.",
    errors: &[],
};

/// `Core\ObjectSet::remove`'s reference card — `rule:core-api/reference-card`.
const REMOVE_DOC: MethodDoc = MethodDoc {
    short: "Removes `$value` from the set.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to remove, matched by identity.",
        shape: &[],
    }],
    ret: "Nothing. If the set does not contain the value, the set does not change.",
    errors: &[],
};

/// `Core\ObjectSet::count`'s reference card — `rule:core-api/reference-card`.
const COUNT_DOC: MethodDoc = MethodDoc {
    short: "Counts the values in the set.",
    params: &[],
    ret: "The number of values in the set. An empty set gives `0`.",
    errors: &[],
};

/// `Core\ObjectSet::isEmpty`'s reference card — `rule:core-api/reference-card`.
const IS_EMPTY_DOC: MethodDoc = MethodDoc {
    short: "Checks whether the set is empty.",
    params: &[],
    ret: "`true` for a set with no values, `false` otherwise.",
    errors: &[],
};

/// `Core\ObjectSet::union`'s reference card — `rule:core-api/reference-card`.
const UNION_DOC: MethodDoc = MethodDoc {
    short: "Builds a new set with every value that is in this set, in `$other`, or in both.",
    params: &[ParamDoc {
        name: "other",
        desc: "The set to combine with.",
        shape: &[],
    }],
    ret: "A new `Core\\ObjectSet<T>`. The values of this set come first, in their order. Then \
          the values of `$other` that are not in this set follow. Neither set is changed.",
    errors: &[],
};

/// `Core\ObjectSet::intersect`'s reference card — `rule:core-api/reference-card`.
const INTERSECT_DOC: MethodDoc = MethodDoc {
    short: "Builds a new set with only the values that are in this set and also in `$other`.",
    params: &[ParamDoc {
        name: "other",
        desc: "The set to intersect with.",
        shape: &[],
    }],
    ret: "A new `Core\\ObjectSet<T>`, in the order of this set. It is empty when the two sets \
          have no value in common. Neither set is changed.",
    errors: &[],
};

/// `Core\ObjectSet::diff`'s reference card — `rule:core-api/reference-card`.
const DIFF_DOC: MethodDoc = MethodDoc {
    short: "Builds a new set with the values that are in this set and not in `$other`. The name \
            is `diff`, the same as `Core\\Arr::diff`.",
    params: &[ParamDoc {
        name: "other",
        desc: "The set whose values are left out.",
        shape: &[],
    }],
    ret: "A new `Core\\ObjectSet<T>`, in the order of this set. It is empty when `$other` has \
          every value of this set. Neither set is changed.",
    errors: &[],
};

/// `Core\ObjectSet::clear`'s reference card — `rule:core-api/reference-card`.
const CLEAR_DOC: MethodDoc = MethodDoc {
    short: "Removes every value from the set. The set is then empty.",
    params: &[],
    ret: "Nothing. You can add values to the same set again.",
    errors: &[],
};

/// [`CLASS`]'s store slot, by index.
const ENTRIES: usize = 0;
/// The set's [`store::Keying`], written once by its constructor.
const KEYING: usize = 1;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<nvs_runtime::Entry> {
    Some(nvs_runtime::Entry::borrowed(match symbol {
        NEW_SYMBOL => (nvs_core_object_set_new as *const ()).cast(),
        NEW_STRING_SYMBOL => (nvs_core_object_set_new_string as *const ()).cast(),
        NEW_INT_SYMBOL => (nvs_core_object_set_new_int as *const ()).cast(),
        NEW_OBJECT_SYMBOL => (nvs_core_object_set_new_object as *const ()).cast(),
        "nvs_core_object_set_add" => (nvs_core_object_set_add as *const ()).cast(),
        "nvs_core_object_set_has" => (nvs_core_object_set_has as *const ()).cast(),
        "nvs_core_object_set_remove" => (nvs_core_object_set_remove as *const ()).cast(),
        "nvs_core_object_set_count" => (nvs_core_object_set_count as *const ()).cast(),
        "nvs_core_object_set_is_empty" => (nvs_core_object_set_is_empty as *const ()).cast(),
        "nvs_core_object_set_union" => (nvs_core_object_set_union as *const ()).cast(),
        "nvs_core_object_set_intersect" => (nvs_core_object_set_intersect as *const ()).cast(),
        "nvs_core_object_set_diff" => (nvs_core_object_set_diff as *const ()).cast(),
        "nvs_core_object_set_clear" => (nvs_core_object_set_clear as *const ()).cast(),
        ITERATE_SYMBOL => (nvs_core_object_set_iterate as *const ()).cast(),
        _ => return None,
    }))
}

/// The receiver of one of this class's instance members.
///
/// # Errors
///
/// [`crate::instance::receiver`]'s, unchanged.
fn set_of(value: Value, member: &str) -> Result<*mut ObjHeader, Fault> {
    crate::instance::receiver(value, &CLASS, member)
}

/// One side of an algebra member: its store and its keying.
type Side = (std::mem::ManuallyDrop<nvs_runtime::NvsArray>, store::Keying);

/// The result an algebra member is building: its store and its keying, which
/// starts as the receiver's.
struct Building {
    store: nvs_runtime::NvsArray,
    keying: store::Keying,
}

impl Building {
    /// Adds `member` if the result does not hold it yet. A member whose tag is
    /// not the result's keying turns the result into a chain set first.
    fn add(&mut self, member: Value) {
        if !self.keying.admits(member) {
            self.store = store::rechain_members(&self.store);
            self.keying = store::Keying::Chain;
        }
        let (place, present) = store::place(&self.store, self.keying, member);
        if present {
            return;
        }
        #[expect(
            unsafe_code,
            reason = "the entry is owned by the source store, which outlives \
                      this call, so the copy the result keeps needs a \
                      reference of its own"
        )]
        unsafe {
            member.retain();
        }
        store::put(&mut self.store, &place, false, member);
    }
}

/// Adds every member of `source` that `keep` answers `true` for to `out`.
///
/// The one shape behind `union`, `intersect` and `diff`: each is a walk of one
/// side deciding membership against the other, and the three differ only in
/// which walks and which verdict. Keys are recomputed rather than copied,
/// because a chain ordinal is a property of what is *already* in the store it
/// belongs to, and two stores do not agree about it. A member is the value
/// under its key in every keying, so the walk reads the values.
fn collect_into(out: &mut Building, source: &nvs_runtime::NvsArray, keep: impl Fn(Value) -> bool) {
    let mut from = 0usize;
    while let Some(slot) = source.next_slot(from) {
        from = slot + 1;
        let Some(value) = source.value_at(slot) else {
            continue;
        };
        if keep(value) {
            out.add(value);
        }
    }
}

/// A fresh set built from the receiver's keying, whose store `fill` writes —
/// the result every algebra member hands back.
fn built_from(keying: store::Keying, fill: impl FnOnce(&mut Building)) -> Value {
    let mut out = Building {
        store: nvs_runtime::NvsArray::new(),
        keying,
    };
    fill(&mut out);
    crate::instance::build(&CLASS, [Value::array(out.store), out.keying.value()])
}

/// The two sides an algebra member reads, as `(this, other)`, each with its
/// own keying.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member if the argument is not a set, which
/// `nvs_types` already refused — the argument's declared type is
/// [`CoreTy::Instance`] of this very class.
fn both(args: &[Value], member: &str) -> Result<(Side, Side), Fault> {
    let side = |value: Value| -> Result<Side, Fault> {
        let receiver = set_of(value, member)?;
        Ok((
            store::borrow(receiver, ENTRIES, &CLASS, member)?,
            keying_of(receiver),
        ))
    };
    Ok((side(args[0])?, side(args[1])?))
}

/// `receiver`'s keying, from its `keying` slot.
fn keying_of(receiver: *mut ObjHeader) -> store::Keying {
    store::Keying::of(crate::instance::slot(receiver, KEYING))
}

/// A fresh empty set keyed by `keying`.
fn built(keying: store::Keying) -> Value {
    crate::instance::build(
        &CLASS,
        [Value::array(nvs_runtime::NvsArray::new()), keying.value()],
    )
}

nvs_runtime::nvs_helper! {
    /// `new Core\ObjectSet<T>()` — a fresh empty set.
    ///
    /// Reached as a symbol rather than as a registered `constructor` member:
    /// a `Core` class has no constructor a program could resolve, so `nvs-ir`
    /// lowers `new` on one straight to this helper and nothing below it
    /// learns that `Core` owns a class ([`crate::instance`]'s module docs).
    fn nvs_core_object_set_new(_ctx, _args: [0]) {
        Ok(built(store::Keying::Chain))
    }
}

nvs_runtime::nvs_helper! {
    /// `new Core\ObjectSet<string>()` — a fresh empty set keyed by each member
    /// itself.
    fn nvs_core_object_set_new_string(_ctx, _args: [0]) {
        Ok(built(store::Keying::Str))
    }
}

nvs_runtime::nvs_helper! {
    /// `new Core\ObjectSet<int>()` — a fresh empty set keyed by each member
    /// itself.
    fn nvs_core_object_set_new_int(_ctx, _args: [0]) {
        Ok(built(store::Keying::Int))
    }
}

nvs_runtime::nvs_helper! {
    /// `new Core\ObjectSet<T>()` for a `T` whose every value is an object — a
    /// fresh empty set keyed by each member's identity.
    fn nvs_core_object_set_new_object(_ctx, _args: [0]) {
        Ok(built(store::Keying::Object))
    }
}

/// Rebuilds an identity-keyed set's store under its members' own identities,
/// after a graph copy made new objects of them. A chain or direct set is left
/// as it is. The class's descriptor calls this
/// ([`nvs_runtime::ClassTable::set_rekey`]).
pub(crate) fn rekey(set: Value) {
    let Some(receiver) = set.obj_ptr() else {
        return;
    };
    if keying_of(receiver) != store::Keying::Object {
        return;
    }
    let Ok(entries) = store::borrow(receiver, ENTRIES, &CLASS, "rekey") else {
        return;
    };
    let rebuilt = store::rebuild_members(&entries, store::Keying::Object);
    crate::instance::set_slot(receiver, ENTRIES, Value::array(rebuilt));
}

/// The set's [`nvs_runtime::KeysPrivate`] hook: a chain or identity key is
/// the store's own and is never shown, and a `string` or `int` member is its
/// own key and is shown.
pub(crate) fn keys_private(set: Value) -> bool {
    set.obj_ptr()
        .is_none_or(|receiver| keying_of(receiver).keeps_keys())
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::add(T $value): void` — adds `$value` if the set
    /// does not already hold something identical to it.
    ///
    /// Idempotent, and silent about which it did: a set that reported whether
    /// the value was new would be `add` answering two questions. `has` is the
    /// spelling for the other one.
    fn nvs_core_object_set_add(_ctx, args: [2]) {
        let receiver = set_of(args[0], "add")?;
        let value = args[1];
        let mut keying = keying_of(receiver);
        if !keying.admits(value) {
            // A member of another tag: the set becomes a chain set, as
            // `crate::identity_store`'s docs say.
            let rebuilt = {
                let entries = store::borrow(receiver, ENTRIES, &CLASS, "add")?;
                store::rechain_members(&entries)
            };
            crate::instance::set_slot(receiver, ENTRIES, Value::array(rebuilt));
            keying = store::Keying::Chain;
            crate::instance::set_slot(receiver, KEYING, keying.value());
        }
        store::edit(receiver, ENTRIES, &CLASS, "add", |entries| {
            let (place, present) = store::place(entries, keying, value);
            if present {
                return;
            }
            #[expect(
                unsafe_code,
                reason = "the argument is borrowed from the caller's frame, so \
                          the copy the store keeps needs a reference of its own"
            )]
            unsafe {
                value.retain();
            }
            store::put(entries, &place, false, value);
        })?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::has(T $value): bool` — whether the set holds
    /// something identical to `$value`.
    fn nvs_core_object_set_has(_ctx, args: [2]) {
        let receiver = set_of(args[0], "has")?;
        let entries = store::borrow(receiver, ENTRIES, &CLASS, "has")?;
        Ok(Value::bool(store::contains(&entries, keying_of(receiver), args[1])))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::remove(T $value): void` — drops `$value` if the
    /// set holds it, and does nothing if it does not.
    ///
    /// Silent for the same reason `add` is: "was it there" is `has`'s question.
    fn nvs_core_object_set_remove(_ctx, args: [2]) {
        let receiver = set_of(args[0], "remove")?;
        let value = args[1];
        let keying = keying_of(receiver);
        if !keying.admits(value) {
            return Ok(Value::null());
        }
        store::edit(receiver, ENTRIES, &CLASS, "remove", |entries| {
            let (place, present) = store::place(entries, keying, value);
            if present {
                store::remove(entries, &place);
            }
        })?;
        Ok(Value::null())
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::count(): uint` — how many distinct values the set
    /// holds.
    ///
    /// The store's own entry count, because the chain keys are one per member
    /// and never a tombstone — `crate::identity_store`'s docs.
    fn nvs_core_object_set_count(_ctx, args: [1]) {
        let receiver = set_of(args[0], "count")?;
        let entries = store::borrow(receiver, ENTRIES, &CLASS, "count")?;
        let count = u64::try_from(entries.count()).expect("an entry count fits in a `uint`");
        Ok(Value::uint(count))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::isEmpty(): bool` — whether the set holds nothing.
    ///
    /// A member of its own for the reason `Core\Arr::isEmpty` is one: the
    /// question is answered without the caller having to know that a count is
    /// `uint`.
    fn nvs_core_object_set_is_empty(_ctx, args: [1]) {
        let receiver = set_of(args[0], "isEmpty")?;
        let entries = store::borrow(receiver, ENTRIES, &CLASS, "isEmpty")?;
        Ok(Value::bool(entries.is_empty()))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::union(ObjectSet<T> $other): ObjectSet<T>` — a new
    /// set holding every value either side holds.
    ///
    /// A new set rather than a mutation of the receiver, which is what makes
    /// the three algebra members composable and is how the `Core\Arr`
    /// counterparts already read. The receiver's values come first, so the
    /// result's iteration order is this side then the other's newcomers.
    fn nvs_core_object_set_union(_ctx, args: [2]) {
        let ((mine, keying), (theirs, _)) = both(args, "union")?;
        Ok(built_from(keying, |out| {
            collect_into(out, &mine, |_| true);
            collect_into(out, &theirs, |_| true);
        }))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::intersect(ObjectSet<T> $other): ObjectSet<T>` — a
    /// new set holding the values both sides hold.
    fn nvs_core_object_set_intersect(_ctx, args: [2]) {
        let ((mine, keying), (theirs, their_keying)) = both(args, "intersect")?;
        Ok(built_from(keying, |out| {
            collect_into(out, &mine, |value| store::contains(&theirs, their_keying, value));
        }))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::diff(ObjectSet<T> $other): ObjectSet<T>` — a new
    /// set holding the values this side holds and the other does not.
    ///
    /// Spelled `diff` rather than `difference` because `Core\Arr` spells it
    /// that way and one operation gets one name.
    fn nvs_core_object_set_diff(_ctx, args: [2]) {
        let ((mine, keying), (theirs, their_keying)) = both(args, "diff")?;
        Ok(built_from(keying, |out| {
            collect_into(out, &mine, |value| !store::contains(&theirs, their_keying, value));
        }))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<T>::iterate(): Iterator<T>` — a cursor over the set's own
    /// store, which yields each member. It takes one reference on the store
    /// and copies nothing; a write in the loop separates the set
    /// ([`crate::cursor`]).
    ///
    /// Not a registered member: it is reached by name through this class's
    /// method table, so its receiver is **transferred** rather than borrowed —
    /// [`crate::cursor`]'s module docs own both halves of that.
    fn nvs_core_object_set_iterate(_ctx, args: [1]) {
        let cursor = set_of(args[0], nvs_runtime::sequence::ITERATE).and_then(|receiver| {
            let entries =
                store::borrow(receiver, ENTRIES, &CLASS, nvs_runtime::sequence::ITERATE)?;
            Ok(crate::cursor::over(NvsArray::clone(&entries)))
        });
        crate::cursor::consume(args[0]);
        cursor
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::clear(): void` — drops every member.
    fn nvs_core_object_set_clear(_ctx, args: [1]) {
        let receiver = set_of(args[0], "clear")?;
        store::replace(receiver, ENTRIES);
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use nvs_runtime::{Ctx, NvsFn, NvsStr, call};

    /// Runs `member` with `set` as the receiver and `rest` past it.
    fn on(set: Value, member: NvsFn, rest: &[Value]) -> Value {
        let mut ctx = Ctx::buffered();
        let mut args = vec![set];
        args.extend_from_slice(rest);
        call(member, &mut ctx, &args).expect("a set member does not throw")
    }

    /// A fresh set, for a test to fill.
    fn empty() -> Value {
        let mut ctx = Ctx::buffered();
        call(nvs_core_object_set_new, &mut ctx, &[]).expect("a fresh set does not throw")
    }

    /// Releases a set this frame owns the only reference to.
    fn drop_set(set: Value) {
        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference the constructor \
                      produced, and releasing it is what the compiled caller \
                      would do"
        )]
        unsafe {
            set.release();
        }
    }

    /// A value whose chain head is already taken by something else gets the
    /// next ordinal, and `remove` still finds it — the property the chain
    /// exists for, pinned by planting the collision rather than by hoping to
    /// find one.
    // covers: Core\ObjectSet::add, Core\ObjectSet::has, Core\ObjectSet::remove, Core\ObjectSet::count
    #[test]
    fn a_hash_collision_takes_the_next_ordinal_in_the_chain() {
        let set = empty();
        let receiver = set.obj_ptr().expect("a set is an object");
        let head = store::locate(
            &store::borrow(receiver, ENTRIES, &CLASS, "test").expect("a fresh set holds a store"),
            Value::int(2),
        )
        .0;
        store::edit(receiver, ENTRIES, &CLASS, "test", |entries| {
            entries.set(NvsStr::new(&head), Value::null());
        })
        .expect("a fresh set holds a store");

        on(set, nvs_core_object_set_add, &[Value::int(2)]);
        assert_eq!(on(set, nvs_core_object_set_count, &[]).as_uint(), Some(2));
        assert_eq!(
            on(set, nvs_core_object_set_has, &[Value::int(2)]).as_bool(),
            Some(true)
        );

        on(set, nvs_core_object_set_remove, &[Value::int(2)]);
        assert_eq!(
            on(set, nvs_core_object_set_has, &[Value::int(2)]).as_bool(),
            Some(false)
        );
        assert_eq!(on(set, nvs_core_object_set_count, &[]).as_uint(), Some(1));
        drop_set(set);
    }

    /// `add` of a value the set holds changes nothing, `clear` empties the set
    /// and keeps it usable, a value added after `clear` is counted afresh, and
    /// `isEmpty` agrees with a count of `0` at each step.
    // covers: Core\ObjectSet::add, Core\ObjectSet::count, Core\ObjectSet::isEmpty, Core\ObjectSet::clear
    #[test]
    fn clear_empties_a_set_that_add_can_fill_again() {
        let set = empty();
        assert_eq!(
            on(set, nvs_core_object_set_is_empty, &[]).as_bool(),
            Some(true)
        );
        for n in [1, 2, 3, 2, 1] {
            on(set, nvs_core_object_set_add, &[Value::int(n)]);
        }
        assert_eq!(on(set, nvs_core_object_set_count, &[]).as_uint(), Some(3));
        assert_eq!(
            on(set, nvs_core_object_set_is_empty, &[]).as_bool(),
            Some(false)
        );

        on(set, nvs_core_object_set_clear, &[]);
        assert_eq!(on(set, nvs_core_object_set_count, &[]).as_uint(), Some(0));
        assert_eq!(
            on(set, nvs_core_object_set_is_empty, &[]).as_bool(),
            Some(true)
        );
        assert_eq!(
            on(set, nvs_core_object_set_has, &[Value::int(1)]).as_bool(),
            Some(false)
        );

        on(set, nvs_core_object_set_add, &[Value::int(1)]);
        on(set, nvs_core_object_set_add, &[Value::int(1)]);
        assert_eq!(on(set, nvs_core_object_set_count, &[]).as_uint(), Some(1));
        drop_set(set);
    }

    /// `union`, `intersect` and `diff` of `{1, 2, 3}` and `{2, 3, 4}` each
    /// build a new set with the members the walk decides, a shared member is
    /// counted once, and neither operand changes.
    // covers: Core\ObjectSet::union, Core\ObjectSet::intersect, Core\ObjectSet::diff
    #[test]
    fn the_algebra_builds_new_sets_and_leaves_both_operands_alone() {
        let left = empty();
        let right = empty();
        for n in [1, 2, 3] {
            on(left, nvs_core_object_set_add, &[Value::int(n)]);
        }
        for n in [2, 3, 4] {
            on(right, nvs_core_object_set_add, &[Value::int(n)]);
        }

        let members = |set: Value| -> Vec<bool> {
            (1..=4)
                .map(|n| {
                    on(set, nvs_core_object_set_has, &[Value::int(n)])
                        .as_bool()
                        .expect("`has` answers a bool")
                })
                .collect()
        };
        for (member, count, held) in [
            (
                nvs_core_object_set_union as NvsFn,
                4,
                [true, true, true, true],
            ),
            (nvs_core_object_set_intersect, 2, [false, true, true, false]),
            (nvs_core_object_set_diff, 1, [true, false, false, false]),
        ] {
            let result = on(left, member, &[right]);
            assert_eq!(
                on(result, nvs_core_object_set_count, &[]).as_uint(),
                Some(count)
            );
            assert_eq!(members(result), held);
            drop_set(result);
        }

        let back = on(right, nvs_core_object_set_diff, &[left]);
        assert_eq!(members(back), [false, false, false, true]);
        drop_set(back);

        assert_eq!(members(left), [true, true, true, false]);
        assert_eq!(members(right), [false, true, true, true]);
        drop_set(left);
        drop_set(right);
    }
}
