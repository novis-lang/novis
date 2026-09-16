//! The one worklist every refcounted value is freed through.
//!
//! # Decision: freeing a graph is iterative, and shared across every kind
//!
//! Releasing the last reference to an object frees the objects it solely owns,
//! which frees the arrays *they* solely own, which frees the objects in those,
//! to whatever depth the program built. Doing that recursively would make the
//! depth of a user's data structure — a linked list, a parse tree, `$a = [$a]`
//! in a loop — decide whether the process survives freeing it, and a stack
//! overflow aborts the process rather than failing one request. That is
//! [AGENTS.md](/AGENTS.md)'s priority 1, so the worklist's allocation
//! is not optional.
//!
//! Giving each kind its own worklist would not do: a chain that alternates
//! object, array, object, array is exactly the shape that would then recurse
//! once per level, at the boundary between the two. So there is one drain, and
//! both [`crate::object`] and [`crate::array`] hand it their children rather
//! than dropping them.
//!
//! The cost, stated as AGENTS.md requires: **one `Vec` allocation per
//! outermost release that actually frees a container**, and nothing at all for
//! a release that only decrements or that frees a string. The worklist starts
//! empty and is only ever touched once a count has already reached zero.

use crate::array;
use crate::object;
use crate::value::{Tag, Value};

/// An allocation whose reference count has just reached zero and which has not
/// been taken apart yet.
///
/// A string is deliberately not a variant: it owns no Novis value, so it can
/// never extend the graph, and [`step_field`] frees one outright.
pub(crate) enum Dying {
    Object(*mut object::ObjHeader),
    Array(*mut array::ArrayHeader),
}

/// Drops the one reference `value` owns, freeing it and everything it solely
/// owns.
///
/// The single entry point: [`Value::release`], `nvs_object_release` and
/// `nvs_array_release` all end up here, so there is exactly one place a graph
/// is dismantled.
///
/// # Safety
///
/// `value` must own the reference being dropped, and must not be released
/// twice.
#[expect(
    unsafe_code,
    reason = "owning the reference is the caller's obligation to state"
)]
pub(crate) unsafe fn release_value(value: Value) {
    // The overwhelmingly common case: a scalar, a string, or a container whose
    // count does not reach zero. None of them needs a worklist, so none of
    // them allocates one.
    #[expect(
        unsafe_code,
        reason = "the caller guarantees this value owns exactly the reference \
                  being dropped"
    )]
    let Some(first) = (unsafe { step_field(value) }) else {
        return;
    };

    let mut work = vec![first];
    while let Some(dying) = work.pop() {
        #[expect(
            unsafe_code,
            reason = "every entry reached zero in `step_field`, so nothing else \
                      can observe it, and each is dismantled exactly once \
                      because it is popped exactly once"
        )]
        unsafe {
            match dying {
                Dying::Object(ptr) => object::dismantle(ptr, &mut work),
                Dying::Array(ptr) => array::dismantle(ptr, &mut work),
            }
        }
    }
}

/// Drops one reference to a value a dying container held, reporting the
/// allocation to dismantle if that was the last one and there is anything
/// behind it.
///
/// # Safety
///
/// `value` must own the reference being dropped, and must not be released
/// twice.
#[expect(
    unsafe_code,
    reason = "owning the reference is the caller's obligation to state"
)]
pub(crate) unsafe fn step_field(value: Value) -> Option<Dying> {
    match value.tag()? {
        Tag::Object => {
            let ptr = value.obj_ptr()?;
            if ptr.is_null() {
                return None;
            }
            #[expect(
                unsafe_code,
                reason = "the caller guarantees this value owns one reference \
                          to a live allocation"
            )]
            let last = unsafe { object::drop_one(ptr) };
            last.then_some(Dying::Object(ptr))
        }
        Tag::Array => {
            let ptr = value.array_ptr()?;
            if ptr.is_null() {
                return None;
            }
            #[expect(
                unsafe_code,
                reason = "the caller guarantees this value owns one reference \
                          to a live allocation"
            )]
            let last = unsafe { array::drop_one(ptr) };
            last.then_some(Dying::Array(ptr))
        }
        // One arm for both: a `bytes` is a `string`'s allocation without the
        // UTF-8 promise, so there is nothing here for a second release path to
        // do differently (`Value::buffer_ptr`).
        Tag::Str | Tag::Bytes => {
            let ptr = value.buffer_ptr()?;
            #[expect(
                unsafe_code,
                reason = "the caller guarantees this value owns one reference \
                          to a live allocation; a string owns no Novis value, so \
                          freeing it can never extend the graph"
            )]
            unsafe {
                crate::string::nvs_str_release(ptr);
            }
            None
        }
        // Every remaining tag is a scalar whose payload is its own bits, plus
        // `Tag::Unset`, which is not refcounted either: nothing to drop.
        _ => None,
    }
}
