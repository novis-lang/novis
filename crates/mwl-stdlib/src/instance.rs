//! The value behind [`registry::CoreTy::Instance`]: what a native member
//! returns when the spec writes a `Core`-owned object, and how another member
//! reads one back.
//!
//! # Decision: a `Core` instance is an ordinary MWL object
//!
//! Not a native handle, not a boxed `dyn Any`, not a tag of its own. A
//! `Core\Regex\Match` is exactly what `new Point(1, 2)` produces — one
//! [`mwl_runtime::MwlObj`] allocation, a [`ClassDesc`] pointer, and 16-byte
//! [`Value`] slots behind it — so **nothing below this line learns that `Core`
//! owns a class**: refcounting, the release sweep, strict identity, `Tag`/
//! `Untag` for a `?T` return and `mwl-codegen`'s argument slots all meet a
//! shape they already had.
//!
//! The cost is that every slot must be a value MWL can already hold, so a
//! member whose state is genuinely native (a compiled `regex::Regex`) has to
//! keep that state somewhere else — for `Core\Regex` that is the pattern text
//! plus [`crate::regex`]'s per-core compiled-pattern cache, which is a lookup
//! rather than a second representation. The rejected alternative was a
//! `Tag::Resource` handle into a per-request table: it buys native state
//! directly and costs a second heap shape, a second release path, and a
//! liveness rule every future `Core` class would have to restate. ADR 0052's
//! closed door on stream wrappers is the same instinct — an engine-owned
//! handle is a thing a program can hold and nothing can check.
//!
//! **What it spends:** one object allocation per instance, `FIELDS_OFFSET`
//! bytes plus 16 per declared slot, charged to the request that produced it and
//! released with it.
//!
//! # Decision: the descriptors are one leaked table per core
//!
//! A [`ClassDesc`]'s *address* is its identity, and it must outlive every
//! instance made from it. A compiled unit's own [`ClassTable`] cannot own these
//! — a `Core` class is not in any program's class list, and an instance can
//! outlive the unit that produced it in a hot-reload swap
//! ([ADR 0017](../../../../docs/adr/0017-hot-reload-without-restart.md)). So
//! this module builds one table per thread, on first use, and **leaks** it.
//!
//! **What it spends:** one descriptor per `Core` instance class per core —
//! O(cores × classes), fixed at two classes' worth today and never growing with
//! traffic, which is the property
//! [AGENTS.md](../../../../AGENTS.md)'s memory rule actually asks for. Leaked
//! rather than dropped at thread exit because a dangling descriptor is a
//! use-after-free and the bytes are bounded by a compile-time roster; per-core
//! rather than shared because the runtime is thread-per-core and shared-nothing,
//! so a table behind a lock would be paying for a race that cannot happen.

use std::cell::Cell;

use mwl_runtime::{ClassDesc, ClassTable, Fault, MwlObj, ObjHeader, Tag, Value};

use crate::registry::{self, CoreClass};

thread_local! {
    /// This core's descriptors, built on first use and never dropped — see
    /// this module's docs. A `Cell<Option<&'static _>>` rather than a
    /// `RefCell<ClassTable>`: the table is written once and read from every
    /// construction, and a shared reference to a leaked table needs no borrow
    /// tracking at all.
    static DESCRIPTORS: Cell<Option<&'static ClassTable>> = const { Cell::new(None) };
}

/// This core's table, building and leaking it if this is the first call.
fn descriptors() -> &'static ClassTable {
    DESCRIPTORS.with(|held| {
        if let Some(table) = held.get() {
            return table;
        }
        let mut table = ClassTable::new();
        for class in registry::CLASSES {
            if class.slots.is_empty() {
                continue;
            }
            // No parents: a `Core` class is not part of any hierarchy, so
            // `instanceof` on one answers only for itself.
            table.define(class.name, class.slots.len(), &[]);
        }
        let table: &'static ClassTable = Box::leak(Box::new(table));
        held.set(Some(table));
        table
    })
}

/// `class`'s descriptor on this core.
///
/// # Panics
///
/// Panics naming the class if it declares no slots — a namespace class has no
/// instances, and asking for one is a build-time oversight in this crate.
fn descriptor(class: &CoreClass) -> *const ClassDesc {
    let table = descriptors();
    let id = table
        .id_of(class.name)
        .unwrap_or_else(|| panic!("{} is not a `Core` class with instances", class.name));
    table.desc(id)
}

/// A fresh instance of `class`, its slots filled from `slots` in declaration
/// order, as the [`Value`] a helper returns.
///
/// Takes over each slot value's reference, exactly as
/// [`MwlObj::set_field`](mwl_runtime::MwlObj::set_field) does — so a caller
/// builds the values it means and hands them straight over.
///
/// # Panics
///
/// Panics if `slots` is not exactly as long as `class.slots`: the layout is
/// this crate's on both sides, so a mismatch is a paste error rather than
/// anything a program can cause.
pub(crate) fn build<const N: usize>(class: &CoreClass, slots: [Value; N]) -> Value {
    assert_eq!(
        N,
        class.slots.len(),
        "{} has {} slots, filled with {N} values",
        class.name,
        class.slots.len()
    );
    #[expect(
        unsafe_code,
        reason = "the descriptor is owned by this core's leaked table, so it \
                  outlives every instance made from it — which is `MwlObj::new`'s \
                  whole safety obligation"
    )]
    let object = unsafe { MwlObj::new(descriptor(class)) };
    for (index, value) in slots.into_iter().enumerate() {
        object.set_field(index, value);
    }
    Value::object(object)
}

thread_local! {
    /// This core's *shape* descriptors — see [`shape`]. A second table beside
    /// [`DESCRIPTORS`] rather than more rows in it, because a shape is not a
    /// [`CoreClass`]: it has no members, no name a program resolves, and no
    /// registry row.
    static SHAPES: Cell<Option<&'static ClassTable>> = const { Cell::new(None) };
}

/// Every ADR 0036 shape a `Core` member builds a value of, as
/// `(descriptor name, fields in slot order)`.
///
/// **Slot order is the field name order, sorted** — `mwl_types::ty::Ty::Shape`
/// canonicalizes `{y: …, x: …}` and `{x: …, y: …}` to one interned type by
/// sorting, so the runtime layout has to be the same order or a written shape
/// type and a built value would disagree about which slot is which.
const SHAPE_ROSTER: &[(&str, &[&str])] = &[(crate::issue::SHAPE, crate::issue::FIELDS)];

/// `name`'s shape descriptor on this core, built and leaked on first use.
///
/// # Panics
///
/// Panics naming the shape if it is not in [`SHAPE_ROSTER`].
fn shape_descriptor(name: &str) -> *const ClassDesc {
    let table = SHAPES.with(|held| {
        if let Some(table) = held.get() {
            return table;
        }
        let mut table = ClassTable::new();
        for (shape, fields) in SHAPE_ROSTER {
            // No parents, and no methods: ADR 0036 § 2 makes a shape value an
            // anonymous *methodless* instance, so there is nothing to inherit
            // and nothing to dispatch.
            table.define(*shape, fields.len(), &[]);
        }
        let table: &'static ClassTable = Box::leak(Box::new(table));
        held.set(Some(table));
        table
    });
    let id = table
        .id_of(name)
        .unwrap_or_else(|| panic!("{name} is not a `Core`-built shape"));
    table.desc(id)
}

/// A fresh ADR 0036 shape value — `{path: "…", message: "…"}` — its slots
/// filled from `slots` in [`SHAPE_ROSTER`]'s order.
///
/// The same anonymous methodless instance an MWL `{…}` literal builds, so
/// nothing downstream learns that `Core` produced this one. Takes over each
/// slot value's reference, exactly as [`build`] does.
///
/// # Panics
///
/// Panics if `slots` is not exactly as long as the shape's field list, or if
/// `name` is not in [`SHAPE_ROSTER`].
pub(crate) fn shape<const N: usize>(name: &str, slots: [Value; N]) -> Value {
    let fields = SHAPE_ROSTER
        .iter()
        .find(|(shape, _)| *shape == name)
        .map_or(&[][..], |(_, fields)| fields);
    assert_eq!(
        N,
        fields.len(),
        "{name} has {} slots, filled with {N} values",
        fields.len()
    );
    #[expect(
        unsafe_code,
        reason = "the descriptor is owned by this core's leaked table, so it \
                  outlives every instance made from it — which is `MwlObj::new`'s \
                  whole safety obligation"
    )]
    let object = unsafe { MwlObj::new(shape_descriptor(name)) };
    for (index, value) in slots.into_iter().enumerate() {
        object.set_field(index, value);
    }
    Value::object(object)
}

/// The receiver of an instance member: argument slot 0, as a raw object
/// pointer live for the length of the call.
///
/// # Errors
///
/// A `Fault::fatal` if the slot does not hold an object — the same treatment
/// every other mistyped argument slot gets, since compiled code wrote the tag
/// and `mwl_types` already checked the receiver's declared type.
pub(crate) fn receiver(
    value: Value,
    class: &CoreClass,
    member: &str,
) -> Result<*mut ObjHeader, Fault> {
    value.obj_ptr().ok_or_else(|| {
        Fault::fatal(format!(
            "{}::{member} expected {:?} for its receiver, got tag {}",
            class.name,
            Tag::Object,
            value.tag_byte()
        ))
    })
}

/// Slot `index` of `receiver`, **borrowed** — the caller takes no reference,
/// exactly as `mwl_ir::InstKind::FieldGet` does not.
///
/// A member handing the result back to MWL code owes it a
/// [`Value::retain`](mwl_runtime::Value::retain) first; one reading it in
/// passing owes nothing.
pub(crate) fn slot(receiver: *mut ObjHeader, index: usize) -> Value {
    #[expect(
        unsafe_code,
        reason = "the receiver argument owns a reference to a live allocation, so \
                  it is live for the length of the call, and the index is one of \
                  this crate's own slot constants"
    )]
    unsafe {
        mwl_runtime::mwl_object_field_get(receiver, index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every registered class with slots gets a descriptor of the right width,
    /// and asking twice on one core answers with the same address — which is
    /// what makes a descriptor's address an identity rather than a per-call
    /// accident.
    #[test]
    fn one_descriptor_per_instance_class_per_core() {
        for class in registry::CLASSES {
            if class.slots.is_empty() {
                continue;
            }
            let first = descriptor(class);
            assert!(std::ptr::eq(first, descriptor(class)));
            #[expect(
                unsafe_code,
                reason = "the leaked table owns the descriptor for the whole \
                          process, so this borrow is sound for any lifetime"
            )]
            let desc = unsafe { &*first };
            assert_eq!(desc.name(), class.name);
            assert_eq!(desc.field_count(), class.slots.len());
        }
    }

    /// A built instance holds exactly what it was given, and one reference —
    /// the property every member returning one depends on, since the compiled
    /// caller releases that one reference and nothing else does.
    #[test]
    fn a_built_instance_owns_one_reference_and_its_slots() {
        let value = build(&crate::regex::MATCH, [Value::null(), Value::int(7)]);
        let ptr = value.obj_ptr().expect("a built instance is an object");
        assert_eq!(slot(ptr, 1).as_int(), Some(7));
        #[expect(
            unsafe_code,
            reason = "this frame owns the one reference `build` produced, and \
                      releasing it is what the compiled caller would do"
        )]
        unsafe {
            value.release();
        }
    }
}
