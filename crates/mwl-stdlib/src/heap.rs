//! `Core\Heap<T>` — docs/spec/01-core-library.md § 9's priority queue, which
//! replaces `SplPriorityQueue`, `SplMinHeap` and `SplMaxHeap`.
//!
//! # Decision: one array, kept as a binary heap
//!
//! A `Core` instance's slots hold only values MWL already holds
//! ([`crate::instance`]), so the heap is an `array<T>` in slot `entries`,
//! keyed `"0"`, `"1"`, … and read as the usual implicit tree: the children of
//! `i` are `2i+1` and `2i+2`. Nothing else is stored — the count is the
//! array's own, and there is no separate size to keep in step.
//!
//! `peek` answers the **smallest** element under the ordering in force, which
//! is `SplMinHeap`'s reading rather than `SplPriorityQueue`'s. Ascending is
//! what `Core\Arr::sort` already means by no argument at all, and a max-heap
//! is one comparator away — `fn ($a, $b) => $b->compareTo($a)` — while the
//! opposite default would leave a min-heap needing exactly the same wrapper.
//!
//! **`peek` and `pop` throw on an empty heap.** § 9's row gives the class an
//! `isEmpty`, which is the question, and [ADR 0063](../../../../docs/adr/0063-core-api-conventions.md)
//! R5 bans the `peekOrNull` twin that a `?T` return would otherwise invite.
//!
//! # Decision: three orderings, tried in one fixed order
//!
//! 1. the `comparator` given at construction, if there is one;
//! 2. otherwise [ADR 0013](../../../../docs/adr/0013-comparable-interface.md)'s
//!    `Comparable::compareTo`, reached through the receiving object's own class
//!    descriptor ([`mwl_runtime::dispatch`]);
//! 3. otherwise the natural order [`crate::ordering::compare_values`] owns,
//!    which is every scalar and throws for anything else.
//!
//! A comparator wins over `Comparable` because it is the more specific of the
//! two and is written at the call site that wanted it — the same precedence
//! `Core\Arr::sort`'s `{comparator: …}` has over its own natural order.
//!
//! **Known gap:** a `Core`-owned instance — a `Core\Time\Instant`, say —
//! declares `compareTo` in the registry but carries no compiled method table
//! on its descriptor, so step 2 does not find it and such a heap needs an
//! explicit comparator. Closing that means giving a `Core` class's descriptor
//! its members, which is `crate::instance`'s question rather than this one's.
//!
//! # What it spends, and what a comparator may not do
//!
//! One array per heap and one entry per element — no wrapper object per
//! element, and no allocation at all on the `push`/`pop` path beyond what the
//! array's own growth costs. Each sift step is O(log n) comparisons and one
//! swap, and holds nothing borrowed from the store across a comparison: the
//! two values being compared are retained for the duration of the call and
//! released after it, and the store is re-read afterwards. That is what makes
//! a comparator that reaches back into *this* heap memory-safe. It is still
//! not **meaningful** — the ordering it observes is a heap mid-sift — so a
//! comparator that mutates the heap it is ordering is unsupported, exactly as
//! `usort`'s is in PHP.

use std::cmp::Ordering;

use mwl_runtime::{Ctx, Fault, MwlArray, MwlStr, ObjHeader, Tag, Value};

use crate::identity_store as store;
use crate::ordering::{comparator_sign, compare_values};
use crate::registry::{Const, CoreClass, CoreMethod, CoreTy};

/// The class's fully-qualified name, as [`CoreTy::Instance`] spells it.
pub(crate) const NAME: &str = r"Core\Heap";

/// The linker symbol `new Core\Heap<T>(...)` lowers to — see
/// [`crate::registry::CONSTRUCTORS`], which is the roster `mwl-ir` reads.
pub(crate) const NEW_SYMBOL: &str = "mwl_core_heap_new";

/// [ADR 0013](../../../../docs/adr/0013-comparable-interface.md)'s one member,
/// which a class opts into by implementing the interface. Must agree with
/// `mwl_types::iter_lib`'s seeded spelling, exactly as
/// [`mwl_runtime::sequence`]'s three names do.
const COMPARE_TO: &str = "compareTo";

/// `new Core\Heap<T>({comparator})` — the constructor
/// [`crate::registry::CONSTRUCTORS`] registers.
///
/// A positional optional parameter rather than an options bag: ADR 0063 R2
/// puts a bag last for a member with *several* settings, and this class has
/// exactly one thing to say about itself. The spec's own § 9 prose calls it
/// "a comparator given at construction", which is this.
pub(crate) const NEW: CoreMethod = CoreMethod {
    name: "constructor",
    params: &[CoreTy::Nullable(&CoreTy::Callable)],
    defaults: &[Const::Null],
    return_ty: CoreTy::Instance(NAME),
    symbol: NEW_SYMBOL,
};

/// `Core\Heap<T>` — docs/spec/01-core-library.md § 9's third row.
///
/// Five members, all of them instance members: a heap is reached through a
/// value, and the only static entry point is the constructor, which is not a
/// member at all ([`crate::registry::CONSTRUCTORS`]).
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[],
    instance: &[
        CoreMethod {
            name: "push",
            params: &[CoreTy::Var("T")],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "mwl_core_heap_push",
        },
        CoreMethod {
            name: "peek",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Var("T"),
            symbol: "mwl_core_heap_peek",
        },
        CoreMethod {
            name: "pop",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Var("T"),
            symbol: "mwl_core_heap_pop",
        },
        CoreMethod {
            name: "count",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Uint,
            symbol: "mwl_core_heap_count",
        },
        CoreMethod {
            name: "isEmpty",
            params: &[],
            defaults: &[],
            return_ty: CoreTy::Bool,
            symbol: "mwl_core_heap_is_empty",
        },
    ],
    slots: &["entries", "comparator"],
    constants: &[],
};

/// [`CLASS`]'s slots, by index.
const ENTRIES: usize = 0;
/// The `callable` given at construction, or `null`.
const COMPARATOR: usize = 1;

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        NEW_SYMBOL => (mwl_core_heap_new as *const ()).cast(),
        "mwl_core_heap_push" => (mwl_core_heap_push as *const ()).cast(),
        "mwl_core_heap_peek" => (mwl_core_heap_peek as *const ()).cast(),
        "mwl_core_heap_pop" => (mwl_core_heap_pop as *const ()).cast(),
        "mwl_core_heap_count" => (mwl_core_heap_count as *const ()).cast(),
        "mwl_core_heap_is_empty" => (mwl_core_heap_is_empty as *const ()).cast(),
        _ => return None,
    })
}

/// The receiver of one of this class's instance members.
///
/// # Errors
///
/// [`crate::instance::receiver`]'s, unchanged.
fn heap_of(value: Value, member: &str) -> Result<*mut ObjHeader, Fault> {
    crate::instance::receiver(value, &CLASS, member)
}

/// The store key of tree position `index`.
fn key(index: usize) -> Vec<u8> {
    index.to_string().into_bytes()
}

/// How many elements the heap holds.
///
/// # Errors
///
/// [`store::borrow`]'s, unchanged.
fn len(receiver: *mut ObjHeader, member: &str) -> Result<usize, Fault> {
    Ok(store::borrow(receiver, ENTRIES, &CLASS, member)?.count())
}

/// The comparator given at construction, borrowed — `null` when there is none.
fn comparator_of(receiver: *mut ObjHeader) -> Value {
    crate::instance::slot(receiver, COMPARATOR)
}

/// A comparison of the values at two tree positions, retained for the length
/// of the call and released after it.
///
/// Nothing borrowed from the store is held across the comparison itself: see
/// this module's docs for why that is the whole of what makes a re-entrant
/// comparator safe rather than merely unlikely.
///
/// # Errors
///
/// Whatever the ordering in force throws, plus [`store::borrow`]'s.
fn precedes(
    ctx: &mut Ctx,
    receiver: *mut ObjHeader,
    member: &str,
    left: usize,
    right: usize,
) -> Result<bool, Fault> {
    let (a, b) = {
        let entries = store::borrow(receiver, ENTRIES, &CLASS, member)?;
        let (Some(a), Some(b)) = (entries.get(&key(left)), entries.get(&key(right))) else {
            return Err(Fault::fatal(format!(
                "{NAME}::{member} compared positions {left} and {right} of a heap holding {}",
                entries.count()
            )));
        };
        #[expect(
            unsafe_code,
            reason = "both values are borrowed from the store, which a \
                      comparison may re-enter and empty, so each needs a \
                      reference of this frame's own for the call's length"
        )]
        unsafe {
            a.retain();
            b.retain();
        }
        (a, b)
    };
    let verdict = compare(ctx, receiver, a, b, member);
    #[expect(
        unsafe_code,
        reason = "this frame owns exactly the two references the retains above \
                  produced"
    )]
    unsafe {
        a.release();
        b.release();
    }
    Ok(verdict? == Ordering::Less)
}

/// Which of two values sorts first, by whichever of the three orderings this
/// module's docs list is in force.
///
/// # Errors
///
/// A [`Fault::Pending`] carrying whatever the comparator or `compareTo` threw,
/// or [`compare_values`]'s throw for a pair with no natural order.
fn compare(
    ctx: &mut Ctx,
    receiver: *mut ObjHeader,
    left: Value,
    right: Value,
    member: &str,
) -> Result<Ordering, Fault> {
    let member = format!("{NAME}::{member}");
    let comparator = comparator_of(receiver);
    if comparator.tag() == Some(Tag::Object) {
        let verdict = mwl_runtime::call_closure(ctx, comparator, &[left, right])?;
        return sign_of(verdict, &member);
    }
    // Both sides, because a `compareTo` declares its parameter at the class
    // that wrote it: handing it a scalar is a call the checker never saw, and
    // the pair has a natural order below or no order at all.
    if left.obj_ptr().is_some()
        && right.obj_ptr().is_some()
        && let Some(verdict) = mwl_runtime::call_method(ctx, left, COMPARE_TO, &[right], &member)?
    {
        return sign_of(verdict, &member);
    }
    compare_values(&left, &right, &member)
}

/// A verdict's sign, releasing the verdict itself.
///
/// Both call paths above hand back a *fresh* reference, so a `compareTo`
/// answering a heap value would otherwise leak one per comparison.
fn sign_of(verdict: Value, member: &str) -> Result<Ordering, Fault> {
    let sign = comparator_sign(verdict, member);
    #[expect(
        unsafe_code,
        reason = "the verdict is a fresh value this frame owns and has not \
                  handed anywhere"
    )]
    unsafe {
        verdict.release();
    }
    sign
}

/// Exchanges the values at two tree positions.
///
/// # Errors
///
/// [`store::edit`]'s, unchanged.
fn swap(receiver: *mut ObjHeader, member: &str, left: usize, right: usize) -> Result<(), Fault> {
    store::edit(receiver, ENTRIES, &CLASS, member, |entries| {
        let (left, right) = (key(left), key(right));
        let (Some(a), Some(b)) = (entries.get(&left), entries.get(&right)) else {
            return;
        };
        #[expect(
            unsafe_code,
            reason = "each value is borrowed from the store, and `set` takes \
                      over a reference — so each write is handed one of this \
                      frame's own and the store's own is what `set` releases"
        )]
        unsafe {
            a.retain();
            b.retain();
        }
        entries.set(MwlStr::new(&left), b);
        entries.set(MwlStr::new(&right), a);
    })
}

/// Moves the element at `index` up until its parent precedes it — the half of
/// the invariant a `push` can break.
fn sift_up(
    ctx: &mut Ctx,
    receiver: *mut ObjHeader,
    member: &str,
    index: usize,
) -> Result<(), Fault> {
    let mut index = index;
    while index > 0 {
        let parent = (index - 1) / 2;
        if !precedes(ctx, receiver, member, index, parent)? {
            break;
        }
        swap(receiver, member, index, parent)?;
        index = parent;
    }
    Ok(())
}

/// Moves the element at `index` down until it precedes both its children —
/// the half a `pop` can break.
fn sift_down(
    ctx: &mut Ctx,
    receiver: *mut ObjHeader,
    member: &str,
    index: usize,
) -> Result<(), Fault> {
    let mut index = index;
    loop {
        // Re-read rather than carried: a comparison can re-enter this heap,
        // and a stale count is the one thing that would index past the store.
        let count = len(receiver, member)?;
        let left = index * 2 + 1;
        if left >= count {
            break;
        }
        let right = left + 1;
        let child = if right < count && precedes(ctx, receiver, member, right, left)? {
            right
        } else {
            left
        };
        if !precedes(ctx, receiver, member, child, index)? {
            break;
        }
        swap(receiver, member, index, child)?;
        index = child;
    }
    Ok(())
}

/// The value at the root, retained for the caller.
///
/// # Errors
///
/// A [`Fault::thrown`] naming the member when the heap is empty — see this
/// module's docs for why that is not a `?T`.
fn root(receiver: *mut ObjHeader, member: &str) -> Result<Value, Fault> {
    let entries = store::borrow(receiver, ENTRIES, &CLASS, member)?;
    let top = entries.get(&key(0)).ok_or_else(|| {
        Fault::thrown(format!(
            "{NAME}::{member}() on an empty heap; `isEmpty()` is the question that has an answer \
             for one"
        ))
    })?;
    #[expect(
        unsafe_code,
        reason = "the value is owned by the store, and a helper's result is a \
                  reference of its caller's own"
    )]
    unsafe {
        top.retain();
    }
    Ok(top)
}

mwl_runtime::mwl_helper! {
    /// `new Core\Heap<T>(?callable $comparator = null)` — a fresh empty heap,
    /// ordered by `$comparator` when one is given.
    ///
    /// Reached as a symbol rather than as a registered `constructor` member:
    /// a `Core` class has no constructor a program could resolve, so `mwl-ir`
    /// lowers `new` on one straight to this helper ([`crate::instance`]'s
    /// module docs). Unlike § 9's other two, this one takes an argument —
    /// `mwl_stdlib::registry::CONSTRUCTORS` is what gives the checker a
    /// signature to hold it to.
    fn mwl_core_heap_new(_ctx, args: [1]) {
        let comparator = args[0];
        match comparator.tag() {
            Some(Tag::Null | Tag::Object) => {}
            _ => {
                return Err(Fault::fatal(format!(
                    "{NAME}'s constructor expected a closure or nothing for `comparator`, got \
                     tag {}",
                    comparator.tag_byte()
                )));
            }
        }
        #[expect(
            unsafe_code,
            reason = "the argument is borrowed from the caller's frame, so the \
                      copy the new instance keeps needs a reference of its own"
        )]
        unsafe {
            comparator.retain();
        }
        Ok(crate::instance::build(
            &CLASS,
            [Value::array(MwlArray::new()), comparator],
        ))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Heap<T>::push(T $value): void` — adds `$value`, keeping the heap
    /// ordered.
    ///
    /// Appended at the end and sifted up, which is the O(log n) half of what
    /// § 9 says these types exist for. A heap holds duplicates: it is a
    /// priority queue, not a set — `Core\ObjectSet` is the one that answers
    /// "already there".
    fn mwl_core_heap_push(ctx, args: [2]) {
        let receiver = heap_of(args[0], "push")?;
        let value = args[1];
        let at = store::edit(receiver, ENTRIES, &CLASS, "push", |entries| {
            let at = entries.count();
            #[expect(
                unsafe_code,
                reason = "the argument is borrowed from the caller's frame, so \
                          the copy the store keeps needs a reference of its own"
            )]
            unsafe {
                value.retain();
            }
            entries.set(MwlStr::new(&key(at)), value);
            at
        })?;
        sift_up(ctx, receiver, "push", at)?;
        Ok(Value::null())
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Heap<T>::peek(): T` — the element `pop` would answer with, left
    /// where it is.
    fn mwl_core_heap_peek(_ctx, args: [1]) {
        let receiver = heap_of(args[0], "peek")?;
        root(receiver, "peek")
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Heap<T>::pop(): T` — removes and answers with the first element
    /// under the ordering in force.
    ///
    /// The last element takes the root's place and sifts down, which is the
    /// standard removal and the reason the store stays dense: every position
    /// from `0` to `count - 1` is occupied, so [`key`] is a total map onto
    /// the tree.
    fn mwl_core_heap_pop(ctx, args: [1]) {
        let receiver = heap_of(args[0], "pop")?;
        let top = root(receiver, "pop")?;
        store::edit(receiver, ENTRIES, &CLASS, "pop", |entries| {
            let last = key(entries.count() - 1);
            if last != key(0)
                && let Some(tail) = entries.get(&last)
            {
                #[expect(
                    unsafe_code,
                    reason = "the value is borrowed from the store, and `set` \
                              takes over a reference — the store's own is \
                              released by the `unset` below"
                )]
                unsafe {
                    tail.retain();
                }
                entries.set(MwlStr::new(&key(0)), tail);
            }
            entries.unset(&last);
        })?;
        sift_down(ctx, receiver, "pop", 0)?;
        Ok(top)
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Heap<T>::count(): uint` — how many elements the heap holds.
    fn mwl_core_heap_count(_ctx, args: [1]) {
        let receiver = heap_of(args[0], "count")?;
        let count = u64::try_from(len(receiver, "count")?).expect("an entry count fits in a `uint`");
        Ok(Value::uint(count))
    }
}

mwl_runtime::mwl_helper! {
    /// `Core\Heap<T>::isEmpty(): bool` — whether the heap holds nothing.
    ///
    /// The member `peek` and `pop` point at: they throw on an empty heap, and
    /// this is how a caller asks first.
    fn mwl_core_heap_is_empty(_ctx, args: [1]) {
        let receiver = heap_of(args[0], "isEmpty")?;
        Ok(Value::bool(len(receiver, "isEmpty")? == 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use mwl_runtime::call;

    /// Runs `member` with `heap` as the receiver and `rest` past it.
    fn on(heap: Value, member: mwl_runtime::MwlFn, rest: &[Value]) -> Value {
        let mut ctx = Ctx::buffered();
        let mut args = vec![heap];
        args.extend_from_slice(rest);
        call(member, &mut ctx, &args).expect("a heap member over ints does not throw")
    }

    /// A fresh heap with no comparator, ordered naturally.
    fn natural() -> Value {
        let mut ctx = Ctx::buffered();
        call(mwl_core_heap_new, &mut ctx, &[Value::null()]).expect("a fresh heap does not throw")
    }

    /// Releases a heap this frame owns the only reference to.
    fn drop_heap(heap: Value) {
        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference the constructor \
                      produced, and releasing it is what the compiled caller \
                      would do"
        )]
        unsafe {
            heap.release();
        }
    }

    /// Pushing in the worst order still pops in ascending order — the whole
    /// contract, pinned over a size that forces both sifts to move more than
    /// one level.
    #[test]
    fn a_natural_heap_pops_in_ascending_order() {
        let heap = natural();
        for n in [8, 3, 9, 1, 7, 2, 6, 4, 5] {
            on(heap, mwl_core_heap_push, &[Value::int(n)]);
        }
        assert_eq!(on(heap, mwl_core_heap_count, &[]).as_uint(), Some(9));

        let mut popped = Vec::new();
        while on(heap, mwl_core_heap_is_empty, &[]).as_bool() == Some(false) {
            popped.push(on(heap, mwl_core_heap_pop, &[]).as_int());
        }
        assert_eq!(
            popped,
            (1..=9).map(Some).collect::<Vec<Option<i64>>>(),
            "a heap ordered by `compare_values` answers smallest first"
        );
        drop_heap(heap);
    }

    /// `peek` leaves the heap alone, and both it and `pop` throw once there
    /// is nothing left rather than answering `null`.
    #[test]
    fn peek_leaves_the_element_and_an_empty_heap_throws() {
        let heap = natural();
        on(heap, mwl_core_heap_push, &[Value::int(4)]);
        assert_eq!(on(heap, mwl_core_heap_peek, &[]).as_int(), Some(4));
        assert_eq!(on(heap, mwl_core_heap_count, &[]).as_uint(), Some(1));
        assert_eq!(on(heap, mwl_core_heap_pop, &[]).as_int(), Some(4));

        let mut ctx = Ctx::buffered();
        assert!(
            call(mwl_core_heap_peek, &mut ctx, &[heap]).is_err(),
            "`peek` on an empty heap throws"
        );
        drop_heap(heap);
    }

    /// A duplicate is held, not folded away: this is a priority queue, and
    /// `Core\ObjectSet` is the member of § 9 that answers "already there".
    #[test]
    fn a_heap_holds_duplicates() {
        let heap = natural();
        for n in [2, 1, 2, 1] {
            on(heap, mwl_core_heap_push, &[Value::int(n)]);
        }
        assert_eq!(on(heap, mwl_core_heap_count, &[]).as_uint(), Some(4));
        assert_eq!(on(heap, mwl_core_heap_pop, &[]).as_int(), Some(1));
        assert_eq!(on(heap, mwl_core_heap_pop, &[]).as_int(), Some(1));
        assert_eq!(on(heap, mwl_core_heap_pop, &[]).as_int(), Some(2));
        assert_eq!(on(heap, mwl_core_heap_pop, &[]).as_int(), Some(2));
        drop_heap(heap);
    }
}
