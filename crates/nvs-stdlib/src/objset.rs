//! `Core\ObjectSet<T>` — docs/spec/01-core-library.md § 9's identity-keyed
//! set, which replaces `SplObjectStorage` used as a set.
//!
//! One slot, holding the store [`crate::identity_store`] describes: a member
//! is an entry under its own chain key, so `has` is a lookup, `count` is the
//! store's own count, and nothing here decides anything about identity that
//! `mwl_runtime::identity` has not already decided.

use mwl_runtime::{Fault, MwlStr, ObjHeader, Value};

use crate::identity_store as store;
use crate::registry::{CoreClass, CoreMethod, CoreTy};

/// The class's fully-qualified name, as [`CoreTy::Instance`] spells it.
pub(crate) const NAME: &str = r"Core\ObjectSet";

/// The linker symbol `new Core\ObjectSet<T>()` lowers to — see
/// [`crate::registry::CONSTRUCTORS`], which is the roster `mwl-ir` reads.
pub(crate) const NEW_SYMBOL: &str = "mwl_core_object_set_new";

/// The symbol behind `Iterable<T>::iterate()`, reached by name through this
/// class's method table rather than as a registered member — see
/// [`crate::cursor`] and [`crate::instance`]'s dispatch roster.
pub(crate) const ITERATE_SYMBOL: &str = "mwl_core_object_set_iterate";

/// `new Core\ObjectSet<T>()` — the constructor [`crate::registry::CONSTRUCTORS`]
/// registers, which takes nothing: a set's order is its insertion order and
/// its identity is `mwl_runtime::identity`'s, so there is nothing to give it.
pub(crate) const NEW: CoreMethod = CoreMethod {
    name: "constructor",
    params: &[],
    defaults: &[],
    return_ty: CoreTy::Instance(NAME),
    symbol: NEW_SYMBOL,
};

/// `Core\ObjectSet<T>` — docs/spec/01-core-library.md § 9's second row.
///
/// Every member is an instance member: a set is reached through a value, and
/// the only static entry point is the constructor, which is not a member at
/// all ([`crate::registry::CONSTRUCTORS`]).
///
/// **A `foreach` over a set yields its members**, in insertion order — the one
/// thing a set holds. [`crate::cursor`] owns the mechanism and what the
/// snapshot spends.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "add",
            params: &[CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "mwl_core_object_set_add",
        },
        CoreMethod {
            name: "has",
            params: &[CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_object_set_has",
        },
        CoreMethod {
            name: "remove",
            params: &[CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "mwl_core_object_set_remove",
        },
        CoreMethod {
            name: "count",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "mwl_core_object_set_count",
        },
        CoreMethod {
            name: "isEmpty",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_object_set_is_empty",
        },
        CoreMethod {
            name: "union",
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "mwl_core_object_set_union",
        },
        CoreMethod {
            name: "intersect",
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "mwl_core_object_set_intersect",
        },
        CoreMethod {
            name: "diff",
            params: &[CoreTy::Instance(NAME)],
            defaults: &[],
            return_ty: CoreTy::Instance(NAME),
            symbol: "mwl_core_object_set_diff",
        },
        CoreMethod {
            name: "clear",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "mwl_core_object_set_clear",
        },
    ],
    slots: &["entries"],
    constants: &[],
};

/// [`CLASS`]'s one slot, by index.
const ENTRIES: usize = 0;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        NEW_SYMBOL => (mwl_core_object_set_new as *const ()).cast(),
        "mwl_core_object_set_add" => (mwl_core_object_set_add as *const ()).cast(),
        "mwl_core_object_set_has" => (mwl_core_object_set_has as *const ()).cast(),
        "mwl_core_object_set_remove" => (mwl_core_object_set_remove as *const ()).cast(),
        "mwl_core_object_set_count" => (mwl_core_object_set_count as *const ()).cast(),
        "mwl_core_object_set_is_empty" => (mwl_core_object_set_is_empty as *const ()).cast(),
        "mwl_core_object_set_union" => (mwl_core_object_set_union as *const ()).cast(),
        "mwl_core_object_set_intersect" => (mwl_core_object_set_intersect as *const ()).cast(),
        "mwl_core_object_set_diff" => (mwl_core_object_set_diff as *const ()).cast(),
        "mwl_core_object_set_clear" => (mwl_core_object_set_clear as *const ()).cast(),
        ITERATE_SYMBOL => (mwl_core_object_set_iterate as *const ()).cast(),
        _ => return None,
    })
}

/// The receiver of one of this class's instance members.
///
/// # Errors
///
/// [`crate::instance::receiver`]'s, unchanged.
fn set_of(value: Value, member: &str) -> Result<*mut ObjHeader, Fault> {
    crate::instance::receiver(value, &CLASS, member)
}

/// Adds every value of `source` that `keep` answers `true` for to `out`.
///
/// The one shape behind `union`, `intersect` and `diff`: each is a walk of one
/// side deciding membership against the other, and the three differ only in
/// which walks and which verdict. Keys are recomputed rather than copied,
/// because a chain ordinal is a property of what is *already* in the store it
/// belongs to, and two stores do not agree about it.
fn collect_into(
    out: &mut mwl_runtime::MwlArray,
    source: &mwl_runtime::MwlArray,
    keep: impl Fn(Value) -> bool,
) {
    let mut from = 0usize;
    while let Some(slot) = source.next_slot(from) {
        from = slot + 1;
        let Some(value) = source.value_at(slot) else {
            continue;
        };
        if !keep(value) {
            continue;
        }
        let (key, present) = store::locate(out, value);
        if present {
            continue;
        }
        #[expect(
            unsafe_code,
            reason = "the entry is owned by the source store, which outlives \
                      this call, so the copy the result keeps needs a \
                      reference of its own"
        )]
        unsafe {
            value.retain();
        }
        out.set(MwlStr::new(&key), value);
    }
}

/// A fresh set whose store `fill` writes — the result every algebra member
/// hands back, built once rather than constructed and then edited.
fn built_from(fill: impl FnOnce(&mut mwl_runtime::MwlArray)) -> Value {
    let mut out = mwl_runtime::MwlArray::new();
    fill(&mut out);
    crate::instance::build(&CLASS, [Value::array(out)])
}

/// The two stores an algebra member reads, as `(this, other)`.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member if the argument is not a set, which
/// `mwl_types` already refused — the argument's declared type is
/// [`CoreTy::Instance`] of this very class.
fn both(
    args: &[Value],
    member: &str,
) -> Result<
    (
        std::mem::ManuallyDrop<mwl_runtime::MwlArray>,
        std::mem::ManuallyDrop<mwl_runtime::MwlArray>,
    ),
    Fault,
> {
    let mine = store::borrow(set_of(args[0], member)?, ENTRIES, &CLASS, member)?;
    let theirs = store::borrow(set_of(args[1], member)?, ENTRIES, &CLASS, member)?;
    Ok((mine, theirs))
}

mwl_runtime::mwl_helper! {
    /// `new Core\ObjectSet<T>()` — a fresh empty set.
    ///
    /// Reached as a symbol rather than as a registered `constructor` member:
    /// a `Core` class has no constructor a program could resolve, so `mwl-ir`
    /// lowers `new` on one straight to this helper and nothing below it
    /// learns that `Core` owns a class ([`crate::instance`]'s module docs).
    fn mwl_core_object_set_new(_ctx, _args: [0]) {
        let empty = mwl_runtime::MwlArray::new();
        Ok(crate::instance::build(&CLASS, [Value::array(empty)]))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectSet<T>::add(T $value): void` — adds `$value` if the set
    /// does not already hold something identical to it.
    ///
    /// Idempotent, and silent about which it did: a set that reported whether
    /// the value was new would be `add` answering two questions, and § 9's
    /// table gives it none. `has` is the spelling for the other one.
    fn mwl_core_object_set_add(_ctx, args: [2]) {
        let receiver = set_of(args[0], "add")?;
        let value = args[1];
        store::edit(receiver, ENTRIES, &CLASS, "add", |entries| {
            let (key, present) = store::locate(entries, value);
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
            entries.set(MwlStr::new(&key), value);
        })?;
        Ok(Value::null())
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectSet<T>::has(T $value): bool` — whether the set holds
    /// something identical to `$value`.
    fn mwl_core_object_set_has(_ctx, args: [2]) {
        let receiver = set_of(args[0], "has")?;
        let entries = store::borrow(receiver, ENTRIES, &CLASS, "has")?;
        let (_, present) = store::locate(&entries, args[1]);
        Ok(Value::bool(present))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectSet<T>::remove(T $value): void` — drops `$value` if the
    /// set holds it, and does nothing if it does not.
    ///
    /// Silent for the same reason `add` is: § 9's table gives it no answer,
    /// and "was it there" is `has`'s question.
    fn mwl_core_object_set_remove(_ctx, args: [2]) {
        let receiver = set_of(args[0], "remove")?;
        let value = args[1];
        store::edit(receiver, ENTRIES, &CLASS, "remove", |entries| {
            let (key, present) = store::locate(entries, value);
            if present {
                store::vacate(entries, &key);
            }
        })?;
        Ok(Value::null())
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectSet<T>::count(): uint` — how many distinct values the set
    /// holds.
    ///
    /// The store's own entry count, because the chain keys are one per member
    /// and never a tombstone — `crate::identity_store`'s docs.
    fn mwl_core_object_set_count(_ctx, args: [1]) {
        let receiver = set_of(args[0], "count")?;
        let entries = store::borrow(receiver, ENTRIES, &CLASS, "count")?;
        let count = u64::try_from(entries.count()).expect("an entry count fits in a `uint`");
        Ok(Value::uint(count))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectSet<T>::isEmpty(): bool` — whether the set holds nothing.
    ///
    /// A member of its own for the reason `Core\Arr::isEmpty` is one: § 9's
    /// table lists it, so the question is answered without the caller having
    /// to know that a count is `uint`.
    fn mwl_core_object_set_is_empty(_ctx, args: [1]) {
        let receiver = set_of(args[0], "isEmpty")?;
        let entries = store::borrow(receiver, ENTRIES, &CLASS, "isEmpty")?;
        Ok(Value::bool(entries.is_empty()))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectSet<T>::union(ObjectSet<T> $other): ObjectSet<T>` — a new
    /// set holding every value either side holds.
    ///
    /// A new set rather than a mutation of the receiver, which is what makes
    /// the three algebra members composable and is how the spec's `Core\Arr`
    /// counterparts already read. The receiver's values come first, so the
    /// result's iteration order is this side then the other's newcomers.
    fn mwl_core_object_set_union(_ctx, args: [2]) {
        let (mine, theirs) = both(args, "union")?;
        Ok(built_from(|out| {
            collect_into(out, &mine, |_| true);
            collect_into(out, &theirs, |_| true);
        }))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectSet<T>::intersect(ObjectSet<T> $other): ObjectSet<T>` — a
    /// new set holding the values both sides hold.
    fn mwl_core_object_set_intersect(_ctx, args: [2]) {
        let (mine, theirs) = both(args, "intersect")?;
        Ok(built_from(|out| {
            collect_into(out, &mine, |value| store::locate(&theirs, value).1);
        }))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectSet<T>::diff(ObjectSet<T> $other): ObjectSet<T>` — a new
    /// set holding the values this side holds and the other does not.
    ///
    /// Spelled `diff` rather than `difference` because `Core\Arr` spells it
    /// that way and one operation gets one name (§ 9's own note).
    fn mwl_core_object_set_diff(_ctx, args: [2]) {
        let (mine, theirs) = both(args, "diff")?;
        Ok(built_from(|out| {
            collect_into(out, &mine, |value| !store::locate(&theirs, value).1);
        }))
    }
}

mwl_runtime::mwl_helper! {
    /// `Iterable<T>::iterate(): Iterator<T>` — a cursor over a snapshot of the
    /// set's members.
    ///
    /// Not a registered member: it is reached by name through this class's
    /// method table, so its receiver is **transferred** rather than borrowed —
    /// [`crate::cursor`]'s module docs own both halves of that.
    fn mwl_core_object_set_iterate(_ctx, args: [1]) {
        let cursor = set_of(args[0], mwl_runtime::sequence::ITERATE).and_then(|receiver| {
            let entries =
                store::borrow(receiver, ENTRIES, &CLASS, mwl_runtime::sequence::ITERATE)?;
            Ok(crate::cursor::over(store::listed(&entries)))
        });
        crate::cursor::consume(args[0]);
        cursor
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\ObjectSet<T>::clear(): void` — drops every member.
    fn mwl_core_object_set_clear(_ctx, args: [1]) {
        let receiver = set_of(args[0], "clear")?;
        store::replace(receiver, ENTRIES);
        Ok(Value::null())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use mwl_runtime::{Ctx, MwlFn, call};

    /// Runs `member` with `set` as the receiver and `rest` past it.
    fn on(set: Value, member: MwlFn, rest: &[Value]) -> Value {
        let mut ctx = Ctx::buffered();
        let mut args = vec![set];
        args.extend_from_slice(rest);
        call(member, &mut ctx, &args).expect("a set member does not throw")
    }

    /// A fresh set, for a test to fill.
    fn empty() -> Value {
        let mut ctx = Ctx::buffered();
        call(mwl_core_object_set_new, &mut ctx, &[]).expect("a fresh set does not throw")
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
            entries.set(MwlStr::new(&head), Value::null());
        })
        .expect("a fresh set holds a store");

        on(set, mwl_core_object_set_add, &[Value::int(2)]);
        assert_eq!(on(set, mwl_core_object_set_count, &[]).as_uint(), Some(2));
        assert_eq!(
            on(set, mwl_core_object_set_has, &[Value::int(2)]).as_bool(),
            Some(true)
        );

        on(set, mwl_core_object_set_remove, &[Value::int(2)]);
        assert_eq!(
            on(set, mwl_core_object_set_has, &[Value::int(2)]).as_bool(),
            Some(false)
        );
        assert_eq!(on(set, mwl_core_object_set_count, &[]).as_uint(), Some(1));
        drop_set(set);
    }
}
