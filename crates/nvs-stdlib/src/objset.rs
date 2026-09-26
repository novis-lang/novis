//! `Core\ObjectSet<T>` — docs/spec/01-core-library.md § 9's identity-keyed
//! set, which replaces `SplObjectStorage` used as a set.
//!
//! One slot, holding the store [`crate::identity_store`] describes: a member
//! is an entry under its own chain key, so `has` is a lookup, `count` is the
//! store's own count, and nothing here decides anything about identity that
//! `nvs_runtime::identity` has not already decided.

use nvs_runtime::{Fault, NvsStr, ObjHeader, Value};

use crate::identity_store as store;
use crate::registry::{ClassDoc, CoreClass, CoreMethod, CoreTy, MethodDoc, ParamDoc};

/// The class's fully-qualified name, as [`CoreTy::Instance`] spells it.
pub(crate) const NAME: &str = r"Core\ObjectSet";

/// The linker symbol `new Core\ObjectSet<T>()` lowers to — see
/// [`crate::registry::CONSTRUCTORS`], which is the roster `nvs-ir` reads.
pub(crate) const NEW_SYMBOL: &str = "nvs_core_object_set_new";

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
    slots: &["entries"],
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
    short: "Builds an empty `Core\\ObjectSet<T>` — a set keyed by identity, which replaces \
            `SplObjectStorage` used as a set.",
    params: &[],
    ret: "A fresh set holding nothing, whose iteration order is insertion order.",
    errors: &[],
};

/// `Core\ObjectSet::add`'s reference card — `rule:core-api/reference-card`.
const ADD_DOC: MethodDoc = MethodDoc {
    short: "Adds `$value` unless the set already holds something identical to it.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to add, matched by identity.",
        shape: &[],
    }],
    ret: "Nothing; adding a value the set already holds changes nothing, and `has` is the \
          spelling for whether it was there.",
    errors: &[],
};

/// `Core\ObjectSet::has`'s reference card — `rule:core-api/reference-card`.
const HAS_DOC: MethodDoc = MethodDoc {
    short: "Whether the set holds something identical to `$value`.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to look for, matched by identity.",
        shape: &[],
    }],
    ret: "`true` when the value is a member, `false` otherwise.",
    errors: &[],
};

/// `Core\ObjectSet::remove`'s reference card — `rule:core-api/reference-card`.
const REMOVE_DOC: MethodDoc = MethodDoc {
    short: "Drops `$value` from the set.",
    params: &[ParamDoc {
        name: "value",
        desc: "The value to drop, matched by identity.",
        shape: &[],
    }],
    ret: "Nothing; a value the set does not hold is left alone rather than reported.",
    errors: &[],
};

/// `Core\ObjectSet::count`'s reference card — `rule:core-api/reference-card`.
const COUNT_DOC: MethodDoc = MethodDoc {
    short: "Counts the distinct values the set holds.",
    params: &[],
    ret: "The number of members; `0` for an empty set.",
    errors: &[],
};

/// `Core\ObjectSet::isEmpty`'s reference card — `rule:core-api/reference-card`.
const IS_EMPTY_DOC: MethodDoc = MethodDoc {
    short: "Whether the set holds nothing.",
    params: &[],
    ret: "`true` for a set with no members, `false` otherwise.",
    errors: &[],
};

/// `Core\ObjectSet::union`'s reference card — `rule:core-api/reference-card`.
const UNION_DOC: MethodDoc = MethodDoc {
    short: "Builds a new set holding every value this set or `$other` holds.",
    params: &[ParamDoc {
        name: "other",
        desc: "The set to combine with.",
        shape: &[],
    }],
    ret: "A fresh `Core\\ObjectSet<T>` — this set's members first, in their order, then \
          `$other`'s newcomers; neither operand is changed.",
    errors: &[],
};

/// `Core\ObjectSet::intersect`'s reference card — `rule:core-api/reference-card`.
const INTERSECT_DOC: MethodDoc = MethodDoc {
    short: "Builds a new set holding the values both this set and `$other` hold.",
    params: &[ParamDoc {
        name: "other",
        desc: "The set to intersect with.",
        shape: &[],
    }],
    ret: "A fresh `Core\\ObjectSet<T>` in this set's order, empty when the two share nothing; \
          neither operand is changed.",
    errors: &[],
};

/// `Core\ObjectSet::diff`'s reference card — `rule:core-api/reference-card`.
const DIFF_DOC: MethodDoc = MethodDoc {
    short: "Builds a new set holding the values this set holds and `$other` does not — spelled \
            `diff` as `Core\\Arr::diff` is, because one operation gets one name.",
    params: &[ParamDoc {
        name: "other",
        desc: "The set whose members are left out.",
        shape: &[],
    }],
    ret: "A fresh `Core\\ObjectSet<T>` in this set's order, empty when `$other` holds everything \
          this set does; neither operand is changed.",
    errors: &[],
};

/// `Core\ObjectSet::clear`'s reference card — `rule:core-api/reference-card`.
const CLEAR_DOC: MethodDoc = MethodDoc {
    short: "Drops every member, leaving the set empty.",
    params: &[],
    ret: "Nothing; the set itself is kept and can be filled again.",
    errors: &[],
};

/// [`CLASS`]'s one slot, by index.
const ENTRIES: usize = 0;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        NEW_SYMBOL => (nvs_core_object_set_new as *const ()).cast(),
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
    out: &mut nvs_runtime::NvsArray,
    source: &nvs_runtime::NvsArray,
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
        out.set(NvsStr::new(&key), value);
    }
}

/// A fresh set whose store `fill` writes — the result every algebra member
/// hands back, built once rather than constructed and then edited.
fn built_from(fill: impl FnOnce(&mut nvs_runtime::NvsArray)) -> Value {
    let mut out = nvs_runtime::NvsArray::new();
    fill(&mut out);
    crate::instance::build(&CLASS, [Value::array(out)])
}

/// The two stores an algebra member reads, as `(this, other)`.
///
/// # Errors
///
/// A [`Fault::fatal`] naming the member if the argument is not a set, which
/// `nvs_types` already refused — the argument's declared type is
/// [`CoreTy::Instance`] of this very class.
fn both(
    args: &[Value],
    member: &str,
) -> Result<
    (
        std::mem::ManuallyDrop<nvs_runtime::NvsArray>,
        std::mem::ManuallyDrop<nvs_runtime::NvsArray>,
    ),
    Fault,
> {
    let mine = store::borrow(set_of(args[0], member)?, ENTRIES, &CLASS, member)?;
    let theirs = store::borrow(set_of(args[1], member)?, ENTRIES, &CLASS, member)?;
    Ok((mine, theirs))
}

nvs_runtime::nvs_helper! {
    /// `new Core\ObjectSet<T>()` — a fresh empty set.
    ///
    /// Reached as a symbol rather than as a registered `constructor` member:
    /// a `Core` class has no constructor a program could resolve, so `nvs-ir`
    /// lowers `new` on one straight to this helper and nothing below it
    /// learns that `Core` owns a class ([`crate::instance`]'s module docs).
    fn nvs_core_object_set_new(_ctx, _args: [0]) {
        let empty = nvs_runtime::NvsArray::new();
        Ok(crate::instance::build(&CLASS, [Value::array(empty)]))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::add(T $value): void` — adds `$value` if the set
    /// does not already hold something identical to it.
    ///
    /// Idempotent, and silent about which it did: a set that reported whether
    /// the value was new would be `add` answering two questions, and § 9's
    /// table gives it none. `has` is the spelling for the other one.
    fn nvs_core_object_set_add(_ctx, args: [2]) {
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
            entries.set(NvsStr::new(&key), value);
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
        let (_, present) = store::locate(&entries, args[1]);
        Ok(Value::bool(present))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::remove(T $value): void` — drops `$value` if the
    /// set holds it, and does nothing if it does not.
    ///
    /// Silent for the same reason `add` is: § 9's table gives it no answer,
    /// and "was it there" is `has`'s question.
    fn nvs_core_object_set_remove(_ctx, args: [2]) {
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
    /// A member of its own for the reason `Core\Arr::isEmpty` is one: § 9's
    /// table lists it, so the question is answered without the caller having
    /// to know that a count is `uint`.
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
    /// the three algebra members composable and is how the spec's `Core\Arr`
    /// counterparts already read. The receiver's values come first, so the
    /// result's iteration order is this side then the other's newcomers.
    fn nvs_core_object_set_union(_ctx, args: [2]) {
        let (mine, theirs) = both(args, "union")?;
        Ok(built_from(|out| {
            collect_into(out, &mine, |_| true);
            collect_into(out, &theirs, |_| true);
        }))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::intersect(ObjectSet<T> $other): ObjectSet<T>` — a
    /// new set holding the values both sides hold.
    fn nvs_core_object_set_intersect(_ctx, args: [2]) {
        let (mine, theirs) = both(args, "intersect")?;
        Ok(built_from(|out| {
            collect_into(out, &mine, |value| store::locate(&theirs, value).1);
        }))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\ObjectSet<T>::diff(ObjectSet<T> $other): ObjectSet<T>` — a new
    /// set holding the values this side holds and the other does not.
    ///
    /// Spelled `diff` rather than `difference` because `Core\Arr` spells it
    /// that way and one operation gets one name (§ 9's own note).
    fn nvs_core_object_set_diff(_ctx, args: [2]) {
        let (mine, theirs) = both(args, "diff")?;
        Ok(built_from(|out| {
            collect_into(out, &mine, |value| !store::locate(&theirs, value).1);
        }))
    }
}

nvs_runtime::nvs_helper! {
    /// `Iterable<T>::iterate(): Iterator<T>` — a cursor over a snapshot of the
    /// set's members.
    ///
    /// Not a registered member: it is reached by name through this class's
    /// method table, so its receiver is **transferred** rather than borrowed —
    /// [`crate::cursor`]'s module docs own both halves of that.
    fn nvs_core_object_set_iterate(_ctx, args: [1]) {
        let cursor = set_of(args[0], nvs_runtime::sequence::ITERATE).and_then(|receiver| {
            let entries =
                store::borrow(receiver, ENTRIES, &CLASS, nvs_runtime::sequence::ITERATE)?;
            Ok(crate::cursor::over(store::listed(&entries)))
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

    use nvs_runtime::{Ctx, NvsFn, call};

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
    /// and keeps it usable, and a value added after `clear` is counted afresh.
    // covers: Core\ObjectSet::add, Core\ObjectSet::count, Core\ObjectSet::clear
    #[test]
    fn clear_empties_a_set_that_add_can_fill_again() {
        let set = empty();
        for n in [1, 2, 3, 2, 1] {
            on(set, nvs_core_object_set_add, &[Value::int(n)]);
        }
        assert_eq!(on(set, nvs_core_object_set_count, &[]).as_uint(), Some(3));

        on(set, nvs_core_object_set_clear, &[]);
        assert_eq!(on(set, nvs_core_object_set_count, &[]).as_uint(), Some(0));
        assert_eq!(
            on(set, nvs_core_object_set_has, &[Value::int(1)]).as_bool(),
            Some(false)
        );

        on(set, nvs_core_object_set_add, &[Value::int(1)]);
        on(set, nvs_core_object_set_add, &[Value::int(1)]);
        assert_eq!(on(set, nvs_core_object_set_count, &[]).as_uint(), Some(1));
        drop_set(set);
    }
}
