//! `rule:classes/revocable-objects`'s runtime half: the revoked flag in an
//! object's count word, the hidden slot of a revocable class, and the thrower
//! of `RevokedObjectError`.
//!
//! # The flag is the count word's top bit
//!
//! [`REVOKED`] is set once, by [`revoke`], and never cleared. Every read that
//! compares or reports a count masks it off ([`count_of`]), so a revoked
//! object is freed with its last reference, exactly as one that is not. The
//! immortal count is all ones and has the bit set too, so [`count_of`] leaves
//! it whole, and [`revoke`] never writes an immortal header: no revocable
//! class has an immortal instance, and the check keeps a read-only header
//! unwritten if one ever did.
//!
//! Compiled code's inline release takes the decrement only for a count from 2
//! up to one below the flag, which is one unsigned compare, as it was. A
//! revoked object's count is above that bound, so each release of one calls
//! `nvs_object_release`, which masks. An object that is not revoked runs the
//! instructions it ran before.
//!
//! # The hidden slot
//!
//! A revocable class's slot 0 is the hidden slot. Only a root class may
//! implement `Revocable`, and a subclass's slots follow its parent's, so slot
//! 0 is the same slot in every class of the tree. It holds a `null` [`Value`]
//! whose payload is:
//!
//! - zero, while the object is in no collection;
//! - a pointer to its [`Memberships`], while it is in one and not revoked;
//! - the site blob of the first `revoke()` call, once it is revoked.
//!
//! The flag says which of the last two the payload is, so no bit of the
//! payload is borrowed. A sweep reads the slot as `null` and releases nothing.
//!
//! **What it spends:** one 16-byte slot per revocable object, and nothing per
//! object of any other class. The memberships are allocated on the request's
//! heap, charged to it and freed with it.

use std::cell::Cell;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{Ctx, NvsObj, ObjHeader, ThrownClass, Value};

/// The revoked flag: the top bit of an object's count word.
pub const REVOKED: usize = 1 << (usize::BITS - 1);

// The flag sits above every count a program can reach, and the immortal count
// is the one word that has it and is not revoked.
const _: () = assert!(crate::IMMORTAL_REFCOUNT & REVOKED != 0);

/// The slot index of a revocable class's hidden slot.
pub const HIDDEN_SLOT: usize = 0;

/// The name the hidden slot has in a class's field list. No property can have
/// it, because a property's name is an identifier and this starts with `#`.
/// [`crate::ClassDesc::field_slot`] never returns the hidden slot, so no lookup
/// by a name a program gives reaches it.
pub const HIDDEN_SLOT_NAME: &str = "#revocable";

/// The reference count a count word holds, without the revoked flag. The
/// immortal count is returned unchanged.
#[must_use]
pub const fn count_of(word: usize) -> usize {
    if word == crate::IMMORTAL_REFCOUNT {
        word
    } else {
        word & !REVOKED
    }
}

/// One collection entry that holds a revocable object.
#[derive(Debug)]
pub struct Membership {
    /// The collection object.
    pub collection: *mut ObjHeader,
    /// Which part of the entry the object is.
    pub kind: MembershipKind,
    /// The entry's key, for a map value; `null` otherwise.
    pub key: Value,
}

/// Which part of a collection entry a revocable object is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MembershipKind {
    /// A `Core\ObjectMap` key.
    MapKey,
    /// A `Core\ObjectMap` value.
    MapValue,
    /// A `Core\ObjectSet` member.
    SetMember,
    /// A `Core\Heap` entry.
    HeapEntry,
}

/// Every collection entry that holds one revocable object, which the hidden
/// slot points to while the object is in a collection.
#[derive(Debug, Default)]
pub struct Memberships {
    /// The entries, in the order they were added.
    pub entries: Vec<Membership>,
}

/// What [`revoke`] and a release ask of a collection class, which only
/// `nvs_stdlib` can answer: the runtime knows the memberships, and the class
/// knows its stores. `nvs_stdlib::instance` writes one for `Core\ObjectMap`,
/// `Core\ObjectSet` and `Core\Heap` through
/// [`crate::ClassTable::set_collection_hooks`].
///
/// No hook may reach compiled code, retain or release the collection it is
/// given, or throw: each runs inside `revoke()` or a release.
#[derive(Clone, Copy, Debug)]
pub struct CollectionHooks {
    /// Takes one entry out for a revoked object: a map's or set's entry with
    /// its own `remove`, and a heap's by lowering its count, the entry staying
    /// in place. It is handed the collection, the revoked object, the
    /// membership's kind and, for a map value, the entry's key.
    pub drop_entry: fn(collection: Value, object: Value, kind: MembershipKind, key: Value),
    /// Unlinks every revocable object the collection holds, run as the
    /// collection is freed while its stores are still there. A slot that is
    /// already `null` is skipped.
    pub unlink_all: fn(collection: Value),
    /// Links every revocable object the collection holds, run for a copy that
    /// `clone` or a graph copy made.
    pub link_all: fn(collection: Value),
}

/// Whether this process has defined a revocable class. A collection skips its
/// membership work while it is `false`, so a program with no revocable class
/// pays one load per write and nothing else.
static IN_USE: AtomicBool = AtomicBool::new(false);

/// Whether this process has defined a revocable class — see [`IN_USE`].
#[must_use]
#[inline]
pub fn in_use() -> bool {
    IN_USE.load(Ordering::Relaxed)
}

/// Records that a revocable class exists. [`crate::ClassTable::set_revocable`]
/// calls it.
pub(crate) fn mark_in_use() {
    IN_USE.store(true, Ordering::Relaxed);
}

thread_local! {
    /// How many objects this thread has revoked, never decreasing. A loop over
    /// a collection's store keeps the figure from its start, and checks each
    /// entry for a revoked object only once it moves ([`revokes`]).
    static REVOKES: Cell<u64> = const { Cell::new(0) };
}

/// How many objects this thread has revoked, never decreasing. A collection's
/// loop shares the store it began with, so an entry revoked during the loop is
/// still in that store. The loop skips such an entry, and while this figure is
/// what it was when the loop began there is none, so the loop checks nothing.
#[must_use]
pub fn revokes() -> u64 {
    REVOKES.with(Cell::get)
}

/// Whether `value` is a revoked object.
#[must_use]
pub fn is_revoked_value(value: Value) -> bool {
    value.obj_ptr().filter(|ptr| !ptr.is_null()).is_some_and(|ptr| {
        #[expect(
            unsafe_code,
            reason = "a Value that holds an object holds a reference to it, so \
                      the object is live"
        )]
        unsafe {
            is_revoked(ptr)
        }
    })
}

/// The object `value` holds when it is a live object of a revocable class
/// that is not revoked: the one kind of value that gets a membership.
fn linkable(value: Value) -> Option<*mut ObjHeader> {
    let ptr = value.obj_ptr().filter(|ptr| !ptr.is_null())?;
    #[expect(
        unsafe_code,
        reason = "a Value that holds an object holds a reference to it, so the \
                  object is live"
    )]
    let usable = unsafe { is_revocable(ptr) && !is_revoked(ptr) };
    usable.then_some(ptr)
}

/// Records that `collection` holds `object` as `kind`, with `key` for a map
/// value. Anything but an object of a revocable class that is not revoked is
/// left as it is, so a collection calls this for every value it stores.
///
/// A map value's key is retained unless it is an object: the map holds an
/// object key for as long as the entry exists, and a reference from here
/// would hide a cycle from the collector.
pub fn link(object: Value, collection: *mut ObjHeader, kind: MembershipKind, key: Value) {
    let Some(ptr) = linkable(object) else {
        return;
    };
    if key.obj_ptr().is_none() {
        #[expect(
            unsafe_code,
            reason = "the key is the caller's, live for the call, and the \
                      membership keeps a reference of its own"
        )]
        unsafe {
            key.retain();
        }
    }
    #[expect(
        unsafe_code,
        reason = "the class is revocable, so slot 0 is the hidden slot, and it \
                  holds zero or the `Memberships` box this module leaked into it"
    )]
    unsafe {
        let slot = hidden_slot(ptr);
        let held = slot.read().bits();
        let list = if held == 0 {
            let fresh = Box::into_raw(Box::<Memberships>::default());
            slot.write(Value::source_const(fresh.cast::<u8>()));
            fresh
        } else {
            usize::try_from(held).unwrap_or(0) as *mut Memberships
        };
        // Most objects are in one or two collections, so a short list grows
        // by one entry at a time, and a long one as a `Vec` does.
        let entries = &mut (*list).entries;
        if entries.len() < 4 {
            entries.reserve_exact(1);
        }
        entries.push(Membership {
            collection,
            kind,
            key,
        });
    }
}

/// Removes the membership [`link`] wrote for `object` in `collection` as
/// `kind`, and for a map value the one under `key`. A revoked object, an
/// object in no collection and anything that is not a revocable object are
/// left as they are. The box is freed with its last entry.
pub fn unlink(object: Value, collection: *mut ObjHeader, kind: MembershipKind, key: Value) {
    let Some(ptr) = linkable(object) else {
        return;
    };
    crate::budget::count_membership_walk();
    #[expect(
        unsafe_code,
        reason = "the class is revocable and the object is not revoked, so the \
                  hidden slot holds zero or this module's `Memberships` box"
    )]
    unsafe {
        let slot = hidden_slot(ptr);
        let held = slot.read().bits();
        if held == 0 {
            return;
        }
        let list = usize::try_from(held).unwrap_or(0) as *mut Memberships;
        let entries = &mut (*list).entries;
        let found = entries.iter().position(|entry| {
            ptr::eq(entry.collection, collection)
                && entry.kind == kind
                && (kind != MembershipKind::MapValue
                    || crate::identity::value_identical(entry.key, key))
        });
        if let Some(at) = found {
            let gone = entries.swap_remove(at);
            if gone.key.obj_ptr().is_none() {
                gone.key.release();
            }
        }
        if entries.is_empty() {
            drop(Box::from_raw(list));
            slot.write(Value::null());
        }
    }
}

/// The hidden slot of the object at `ptr`.
///
/// # Safety
///
/// `ptr` must refer to a live object of a revocable class.
#[expect(
    unsafe_code,
    reason = "the pointee's liveness and class are the caller's obligation"
)]
unsafe fn hidden_slot(ptr: *mut ObjHeader) -> *mut Value {
    #[expect(
        unsafe_code,
        reason = "a revocable class has at least one slot, `set_revocable`'s \
                  assert, and slot 0 is inside the allocation"
    )]
    unsafe {
        ptr.cast::<u8>()
            .add(crate::field_offset(HIDDEN_SLOT))
            .cast::<Value>()
    }
}

/// Whether the object at `ptr` is of a revocable class.
///
/// # Safety
///
/// `ptr` must refer to a live object.
#[expect(unsafe_code, reason = "the pointee's liveness is the caller's obligation")]
unsafe fn is_revocable(ptr: *mut ObjHeader) -> bool {
    #[expect(
        unsafe_code,
        reason = "a live object's descriptor is live, `NvsObj::new`'s contract"
    )]
    unsafe {
        (*NvsObj::class_of(ptr)).is_revocable()
    }
}

/// Whether the object at `ptr` was revoked.
///
/// # Safety
///
/// `ptr` must refer to a live object.
#[must_use]
#[expect(unsafe_code, reason = "the pointee's liveness is the caller's obligation")]
pub unsafe fn is_revoked(ptr: *mut ObjHeader) -> bool {
    #[expect(unsafe_code, reason = "the caller guarantees the object is live")]
    let word = unsafe { NvsObj::count_word(ptr) }.get();
    word != crate::IMMORTAL_REFCOUNT && word & REVOKED != 0
}

/// Revokes the object at `ptr`, which `revoke()` runs: it sets the flag, takes
/// the object out of every collection that holds it, frees its memberships,
/// and keeps `site` in the hidden slot. A second call does nothing, so the
/// first site stays. An object of a class that is not revocable is left as it
/// is.
///
/// The flag and the site are written first, so an [`unlink`] a removal makes
/// for this object finds it revoked and leaves the list being walked alone.
/// The object and every collection are retained for the walk: a removal
/// releases what the entry held, and that can free another collection on the
/// list.
///
/// # Safety
///
/// `ptr` must refer to a live object, and `site` must be null or an address
/// [`crate::source::encode`]'s bytes were baked at.
#[expect(
    unsafe_code,
    reason = "the pointee's liveness and the blob's are the caller's obligation"
)]
pub unsafe fn revoke(ptr: *mut ObjHeader, site: *const u8) {
    #[expect(unsafe_code, reason = "the caller guarantees the object is live")]
    let revocable = unsafe { is_revocable(ptr) };
    #[expect(unsafe_code, reason = "the caller guarantees the object is live")]
    let word = unsafe { NvsObj::count_word(ptr) };
    if !revocable || word.get() == crate::IMMORTAL_REFCOUNT || word.get() & REVOKED != 0 {
        return;
    }
    word.set(word.get() | REVOKED);
    #[expect(
        unsafe_code,
        reason = "the class is revocable, so slot 0 is the hidden slot"
    )]
    let slot = unsafe { hidden_slot(ptr) };
    #[expect(
        unsafe_code,
        reason = "the hidden slot holds a `null` Value whose payload is zero or \
                  the `Memberships` box this module leaked into it"
    )]
    let list = unsafe {
        let held = slot.read().bits();
        slot.write(Value::source_const(site));
        (held != 0).then(|| Box::from_raw(usize::try_from(held).unwrap_or(0) as *mut Memberships))
    };
    REVOKES.with(|count| count.set(count.get().wrapping_add(1)));
    let Some(list) = list else {
        return;
    };
    crate::budget::count_membership_walk();
    let object = Value::from_obj_ptr(ptr);
    #[expect(
        unsafe_code,
        reason = "the object and every collection on its list are live: a \
                  collection unlinks its members before it is freed. Each is \
                  retained here and released once the walk ends"
    )]
    unsafe {
        object.retain();
        for entry in &list.entries {
            Value::from_obj_ptr(entry.collection).retain();
        }
        for entry in &list.entries {
            let class = &*NvsObj::class_of(entry.collection);
            if let Some(hooks) = class.collection_hooks() {
                (hooks.drop_entry)(
                    Value::from_obj_ptr(entry.collection),
                    object,
                    entry.kind,
                    entry.key,
                );
            }
        }
        for entry in &list.entries {
            if entry.key.obj_ptr().is_none() {
                entry.key.release();
            }
            Value::from_obj_ptr(entry.collection).release();
        }
        object.release();
    }
}

/// The site blob of the first `revoke()` of the object at `ptr`, or null when
/// it is not revoked or was revoked with no site.
///
/// # Safety
///
/// `ptr` must refer to a live object.
#[must_use]
#[expect(unsafe_code, reason = "the pointee's liveness is the caller's obligation")]
pub unsafe fn revoke_site(ptr: *mut ObjHeader) -> *const u8 {
    #[expect(unsafe_code, reason = "the caller guarantees the object is live")]
    let revoked = unsafe { is_revoked(ptr) } && unsafe { is_revocable(ptr) };
    if !revoked {
        return ptr::null();
    }
    #[expect(
        unsafe_code,
        reason = "the class is revocable, so slot 0 is the hidden slot"
    )]
    let held = unsafe { hidden_slot(ptr).read() };
    held.as_source_const().unwrap_or(ptr::null())
}

/// `RevokedObjectError`'s message for the object at `ptr`.
///
/// # Safety
///
/// `ptr` must refer to a live object, and its hidden slot's site, if any, to a
/// blob a compiled unit baked.
#[must_use]
#[expect(
    unsafe_code,
    reason = "the pointee's liveness and the blob's are the caller's obligation"
)]
pub unsafe fn revoked_message(ptr: *mut ObjHeader) -> String {
    #[expect(unsafe_code, reason = "the caller guarantees the object is live")]
    let class = unsafe { (*NvsObj::class_of(ptr)).name().to_owned() };
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the site is a baked blob or null"
    )]
    let source = unsafe { crate::source::decode(revoke_site(ptr)) };
    match source {
        Some(source) if !source.file.is_empty() => format!(
            "This `{class}` object was revoked in `{}` on line {}. A revoked object cannot be used.",
            source.file, source.line
        ),
        _ => format!("This `{class}` object was revoked. A revoked object cannot be used."),
    }
}

/// `Err` with `RevokedObjectError` when `value` is a revoked object, and `Ok`
/// for anything else. A use through `mixed`, `Core\Json`, `Core\Reflect`, a
/// graph copy and `clone` asks this before it reads the object.
///
/// # Errors
///
/// The `RevokedObjectError` fault, naming the class and the first revoke site.
pub fn revoked_use(value: &Value) -> Result<(), crate::abi::Fault> {
    revoked_text(value).map_or(Ok(()), |message| {
        Err(crate::abi::Fault::thrown_as(ThrownClass::Revoked, message))
    })
}

/// `RevokedObjectError`'s message when `value` is a revoked object, and `None`
/// for anything else. A graph copy refuses a revoked object with this message.
#[must_use]
pub fn revoked_text(value: &Value) -> Option<String> {
    let ptr = value.obj_ptr().filter(|ptr| !ptr.is_null())?;
    #[expect(
        unsafe_code,
        reason = "a Value that holds an object holds a reference to it, so the \
                  object is live, and its hidden slot holds a baked blob or null"
    )]
    unsafe {
        is_revoked(ptr).then(|| revoked_message(ptr))
    }
}

/// `revoke()`, as compiled code calls it. `site` is the call's own site blob.
///
/// # Safety
///
/// As [`revoke`].
#[expect(unsafe_code, reason = "compiled code passes an object and a blob")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_revoke(object: *mut ObjHeader, site: *const u8) {
    #[expect(unsafe_code, reason = "forwarding this function's own contract")]
    unsafe {
        revoke(object, site);
    }
}

/// `isRevoked()`, as compiled code calls it: `1` when the object was revoked,
/// `0` otherwise.
///
/// # Safety
///
/// `object` must refer to a live object.
#[expect(unsafe_code, reason = "compiled code passes an object")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_is_revoked(object: *mut ObjHeader) -> u8 {
    #[expect(unsafe_code, reason = "forwarding this function's own contract")]
    u8::from(unsafe { is_revoked(object) })
}

/// Throws `RevokedObjectError` for a use of the revoked object `object`, and
/// returns [`crate::THROWN`]. `site` is the use's own site blob, which names
/// the frame the throw starts in.
///
/// # Safety
///
/// `ctx` must be non-null, aligned and valid for the call, `object` must refer
/// to a live object, and `site` must be null or a baked blob.
#[expect(
    unsafe_code,
    reason = "compiled code passes its context, an object and a blob"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_throw_revoked(
    ctx: *mut Ctx,
    object: *mut ObjHeader,
    site: *const u8,
) -> i32 {
    // Contained like `nvs_deprecated_use`: the message is Rust that allocates.
    // The closure runs inside this block, so the object and the blob it reads
    // are the caller's contract, passed on unchanged.
    #[expect(unsafe_code, reason = "the caller's contract, passed on unchanged")]
    unsafe {
        crate::abi::contain_panic(ctx, |ctx| {
            let message = revoked_message(object);
            ctx.set_pending_as(ThrownClass::Revoked, message);
            ctx.seed_raise_site(site);
            crate::THROWN
        })
    }
}

#[cfg(test)]
mod tests {
    // covers: lang:classes/revocable-objects
    use super::*;
    use crate::ClassTable;
    use nvs_render::Source;

    /// A revocable root class with one property behind its hidden slot.
    fn revocable() -> (ClassTable, crate::ClassId) {
        let mut table = ClassTable::new();
        let post = table.define("Blog\\Post", &["", "title"], &[]);
        table.set_revocable(post);
        (table, post)
    }

    fn blob(line: u32) -> Vec<u8> {
        crate::source::encode(&Source {
            file: "src/Blog/Post.nvs".to_owned(),
            line,
            member: None,
        })
    }

    #[test]
    fn the_revoked_flag_is_masked_from_every_count_read() {
        let (table, post) = revocable();
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { NvsObj::new(table.desc(post)) };
        let second = object.clone();
        let ptr = object.into_raw();
        let site = blob(3);
        #[expect(unsafe_code, reason = "a live object and a live blob")]
        unsafe {
            revoke(ptr, site.as_ptr());
            assert!(is_revoked(ptr));
            assert_eq!(NvsObj::refcount_of(ptr), 2);
            assert_eq!(NvsObj::count_word(ptr).get(), REVOKED | 2);
        }
        assert_eq!(second.refcount(), 2);
        assert!(format!("{second:?}").contains("refcount: 2"));
        #[expect(unsafe_code, reason = "the reference `into_raw` gave up")]
        drop(unsafe { NvsObj::from_raw(ptr) });
        assert_eq!(second.refcount(), 1);
    }

    #[test]
    fn a_revoked_object_is_freed_with_its_last_reference() {
        let (table, post) = revocable();
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { NvsObj::new(table.desc(post)) };
        let title = Value::text("a title that is not inline");
        #[expect(unsafe_code, reason = "the test holds a second reference")]
        unsafe {
            title.retain();
        }
        object.set_field(1, title);
        assert_eq!(title.refcount(), Some(2));
        let ptr = object.into_raw();
        #[expect(unsafe_code, reason = "a live object")]
        unsafe {
            revoke(ptr, ptr::null());
            crate::nvs_object_release(ptr);
        }
        // The object was freed, and freeing it released its title.
        assert_eq!(title.refcount(), Some(1));
        #[expect(unsafe_code, reason = "the test's own reference")]
        unsafe {
            title.release();
        }
    }

    #[test]
    fn a_second_revoke_keeps_the_first_site() {
        let (table, post) = revocable();
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { NvsObj::new(table.desc(post)) };
        let ptr = object.into_raw();
        let (first, second) = (blob(42), blob(77));
        let mut ctx = Ctx::buffered();
        #[expect(unsafe_code, reason = "a live object, live blobs and a live context")]
        unsafe {
            revoke(ptr, first.as_ptr());
            revoke(ptr, second.as_ptr());
            assert_eq!(revoke_site(ptr), first.as_ptr());
            assert_eq!(
                nvs_throw_revoked(&raw mut ctx, ptr, ptr::null()),
                crate::THROWN
            );
            crate::nvs_object_release(ptr);
        }
        assert_eq!(
            ctx.pending().as_deref(),
            Some(
                "This `Blog\\Post` object was revoked in `src/Blog/Post.nvs` on line 42. \
                 A revoked object cannot be used."
            )
        );
    }
}
