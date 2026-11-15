//! The one worklist every refcounted value is freed through.
//!
//! # Decision: freeing a graph is iterative, and shared across every kind
//!
//! Releasing the last reference to an object frees the objects it solely owns,
//! which frees the arrays *they* solely own, which frees the objects in those,
//! to whatever depth the program built. Doing that recursively would make the
//! depth of a user's data structure — a linked list, a parse tree, `$a = [$a]`
//! in a loop — decide whether the process survives freeing it, and a stack
//! overflow aborts the process rather than failing one request. That is a
//! security failure, so the worklist's allocation is not optional.
//!
//! Giving each kind its own worklist would not do: a chain that alternates
//! object, array, object, array is exactly the shape that would then recurse
//! once per level, at the boundary between the two. So there is one drain, and
//! both [`crate::object`] and [`crate::array`] hand it their children rather
//! than dropping them.
//!
//! The cost, stated as AGENTS.md requires: **one `Vec` allocation per
//! outermost release that has more than [`INLINE`] containers waiting to be
//! taken apart at once**, and nothing at all for any other release. The
//! [`Worklist`] keeps its first [`INLINE`] entries in an array on the stack,
//! and only an entry past those goes to the heap. A linked list of any length
//! keeps one entry waiting at a time, so freeing one never allocates. An array
//! of many objects spills once, when it hands over its ninth. **What it
//! spends:** about 160 bytes of stack per outermost release that reaches zero,
//! and no heap.
//!
//! The array is plain safe Rust: an `Option` per entry and a count. No entry
//! is read past the count, because each is moved out with `take`.

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

/// How many waiting entries [`Worklist`] keeps on the stack.
pub(crate) const INLINE: usize = 8;

/// The allocations still to take apart, last in, first out.
///
/// The first [`INLINE`] entries are kept in an array, and the rest in a `Vec`
/// that allocates on its first push. The module doc states the cost.
pub(crate) struct Worklist {
    inline: [Option<Dying>; INLINE],
    /// How many leading entries of `inline` are `Some`.
    held: usize,
    spill: Vec<Dying>,
}

impl Worklist {
    fn new() -> Self {
        Self {
            inline: [const { None }; INLINE],
            held: 0,
            spill: Vec::new(),
        }
    }

    /// Adds `dying` to the list. It goes into the array while the array has
    /// room and nothing has spilled, and onto the `Vec` otherwise.
    pub(crate) fn push(&mut self, dying: Dying) {
        if self.spill.is_empty()
            && let Some(slot) = self.inline.get_mut(self.held)
        {
            *slot = Some(dying);
            self.held = self.held.saturating_add(1);
            return;
        }
        self.spill.push(dying);
    }

    /// The entry pushed last, or `None` when the list is empty. The `Vec` is
    /// emptied first, because what it has was pushed after the array filled.
    fn pop(&mut self) -> Option<Dying> {
        if let Some(dying) = self.spill.pop() {
            return Some(dying);
        }
        let last = self.held.checked_sub(1)?;
        self.held = last;
        self.inline.get_mut(last)?.take()
    }
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
#[deny(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::unreachable,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
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

    // The first one is dismantled out of a local rather than out of the
    // worklist, and the worklist keeps its first entries on the stack, so a
    // small graph is freed with no allocation of its own. The module doc
    // states when it spills to the heap.
    let mut work = Worklist::new();
    let mut dying = first;
    loop {
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
        let Some(next) = work.pop() else {
            return;
        };
        dying = next;
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
#[deny(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::unreachable,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
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
