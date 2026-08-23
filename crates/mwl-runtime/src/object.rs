//! MWL's class instance: one heap allocation, a two-word header, and the
//! object's fields inline behind it — plus the [`ClassDesc`] every instance
//! points back at.
//!
//! This is the representation `mwl_ir::ty::Ty::Object` lowers to, and the one
//! thing nearly everything else in M4 waits on (`.claude/loop-goal.md`). It
//! follows [`crate::string`]'s shape deliberately: one allocation, a
//! [`Cell`]-refcounted header, and the payload behind it at a fixed offset
//! compiled code computes rather than asks for.
//!
//! # Layout
//!
//! ```text
//! offset 0        offset 8         offset FIELDS_OFFSET
//! +-------------+ +--------------+ +------------------------------+
//! | refcount    | | *ClassDesc   | | field_count * 16-byte Values |
//! +-------------+ +--------------+ +------------------------------+
//! ```
//!
//! ## Decision: a field slot is a whole 16-byte [`Value`]
//!
//! Not a native-width slot sized to the field's declared type. An `int` field
//! therefore costs 16 bytes rather than 8, and writing one stores a tag byte
//! nothing reads back.
//!
//! That is [CLAUDE.md](../../../CLAUDE.md)'s priority 5 spent on its priorities
//! 2 and 4, which is the direction the ordering permits:
//!
//! * **Releasing an object needs no per-field type table.** The sweep in
//!   [`release_graph`] branches on each slot's own tag. With unboxed slots it
//!   would have to walk a parallel `Vec<Ty>` on the [`ClassDesc`] — a second
//!   structure that must agree with the layout codegen emitted, i.e. exactly
//!   the "invariant every future contributor must remember" CLAUDE.md's
//!   memory section names as the wrong trade.
//! * **`mixed` and `?T` fields need no special case.** They are already a
//!   tagged value; a uniform slot is the only representation that holds one
//!   without a second, boxed layout beside the first.
//! * **The read side pays nothing.** A field's static type is known
//!   ([ADR 0007](../../../docs/adr/0007-explicit-type-system.md)), so codegen
//!   loads the payload half directly and never checks the tag on a read. Only
//!   a write pays, and it pays one extra store.
//!
//! The cost is stated as CLAUDE.md requires: **8 extra bytes per declared
//! property per live object**, charged to the request that allocated it.
//!
//! ## Decision: a subclass's slots follow its parent's
//!
//! A [`ClassDesc`]'s [`field_count`](ClassDesc::field_count) is the *total*
//! across the whole inheritance chain, and a class's own properties occupy the
//! slots after its parent's. So a slot index computed against a base class is
//! valid for every subclass — which is what lets `mwl_ir::InstKind::FieldGet`
//! keep naming the *declaring* class rather than the receiver's runtime one.
//!
//! # Decision: `ClassDesc` is opaque, and its address is the class identity
//!
//! Compiled code never reads a field of one. It passes the pointer to
//! [`mwl_object_new`] and [`mwl_object_instanceof`], and those are the only
//! two operations that exist. So the struct is an ordinary Rust type, not a
//! `#[repr(C)]` one, and `mwl-codegen` bakes each descriptor's address into
//! the code it emits as a constant — the normal JIT move, and the reason there
//! is no registry lookup on the allocation path.
//!
//! The descriptors themselves are owned by a [`ClassTable`], one per compiled
//! unit, which the unit must keep alive for as long as its code is callable.
//!
//! # Decision: the called class travels in the receiver slot, and a descriptor
//! carries a method table
//!
//! Late static binding ([`docs/implementation-plan.md`](../../../docs/implementation-plan.md)'s
//! M4: `new static()` through two levels of inheritance returns the *called*
//! class) needs two things compiled code did not have: the called class at the
//! callee, and a way to find that class's own method from it.
//!
//! **The called class travels in argument slot 0.** Every lowered method
//! already has an implicit receiver there, and a static method's caller
//! already fills it — with `null`, because a static method has no `$this`.
//! That slot now carries the late-static-binding class instead: the tag byte
//! stays [`Tag::Null`] (as an MWL *value* the slot still holds nothing, so
//! nothing sweeping a `Value` ever sees a class descriptor) and the payload
//! half carries the [`ClassDesc`] address. Compiled code reads the payload
//! without consulting the tag, exactly as it already does for every other
//! statically-typed slot. So late static binding costs **no new parameter, no
//! second calling convention and not one extra instruction** at a call site
//! that does not use it — the store was always there. Inside an *instance*
//! method the called class is the receiver's own, one load at
//! [`OBJ_CLASS_OFFSET`], so nothing is passed at all.
//!
//! **A descriptor carries its methods by name.** [`ClassDesc::method`] binary-
//! searches a flattened, name-sorted table — every method the class declares
//! plus every one it inherits, own-first — and `mwl-codegen` fills it in after
//! `finalize_definitions`, which is the first moment a compiled function has
//! an address.
//!
//! That is a *name* lookup, not a vtable index, and deliberately: the only
//! call shape that reaches it today is `static::method()`/`new static()`,
//! where the class is unknown until run time. An ordinary `$obj->method()` is
//! still resolved statically from the receiver's declared type
//! (`mwl-codegen`'s known gap 1), so nothing on the hot path pays for this.
//! Turning that gap into real virtual dispatch wants a compile-time slot index
//! rather than a name — a separate decision, on a table this one already
//! builds.
//!
//! Cost, as [CLAUDE.md](../../../CLAUDE.md) requires: one `(String, *const u8)`
//! pair per method *reachable* on each class — so a deep hierarchy holds its
//! ancestors' entries once per descendant — charged to the compiled unit, not
//! to a request, and freed with it.
//!
//! # Decision: releasing a graph is iterative, never recursive
//!
//! [`release_graph`] drives an explicit worklist. A recursive release would
//! make the depth of a user's data structure — a linked list, a parse tree —
//! decide whether the process survives freeing it, and a stack overflow aborts
//! the process rather than failing one request. That is priority 1, so the
//! worklist's allocation is not optional.
//!
//! # Decision: a null payload *is* `null`
//!
//! A refcounted representation's null pointer means MWL's `null`, and every
//! retain/release primitive treats it as a no-op — [`mwl_object_retain`],
//! [`mwl_object_release`], and [`crate::mwl_str_retain`]/
//! [`crate::mwl_str_release`] alike.
//!
//! This is not a defensive check. It is the state a field slot is *in* between
//! [`MwlObj::new`] zeroing it and the constructor's first assignment: that
//! assignment releases whatever the slot previously held
//! (`mwl_ir::lower::lower_reassignment`), and on the first write there is
//! nothing there. Compiled code reads the payload half of the slot without
//! consulting its tag — the field's static type already settled what it holds
//! — so what reaches the primitive is a null pointer, not a `Tag::Null`
//! [`Value`].
//!
//! The same rule is what a nullable `?T` will lower to, so paying one
//! perfectly-predicted branch per refcount operation buys both cases at once.
//! The alternative — teaching lowering which assignment is a property's
//! *first* — needs `mwl_types::ctor_init`'s flow analysis threaded into the
//! IR, to remove a branch that costs nothing measurable.
//!
//! # Decision: no cycle collector
//!
//! Refcounting only, per `.claude/loop-goal.md`. A cyclic object graph is
//! retained until the process exits; see [`crate`]'s own known gaps for the
//! boundary and where the eventual collector belongs.

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::cell::Cell;
use std::fmt;
use std::ptr::NonNull;

use crate::value::{Tag, Value};

/// What every instance of one class shares: its name, how many field slots it
/// has, and which other classes and interfaces it is a subtype of.
///
/// Opaque to compiled code — see this module's docs. Never constructed
/// directly: a [`ClassTable`] owns every descriptor, so the `conforms`
/// pointers below always refer to descriptors that outlive this one.
pub struct ClassDesc {
    /// The class's rendered name, as `mwl_ir` labels it (`Class` or
    /// `Ns\Class`). Used by diagnostics and by `Core\Reflect` later; never by
    /// dispatch, which is resolved at compile time.
    name: String,
    /// Total field slots, including every ancestor's — see this module's docs.
    field_count: usize,
    /// Every *other* class and interface an instance of this one also is,
    /// flattened at definition time so `instanceof` is one linear scan of a
    /// short slice rather than a chain walk plus a per-level interface search.
    /// Does not include this descriptor itself; [`ClassDesc::conforms_to`]
    /// checks identity first.
    conforms: Vec<*const ClassDesc>,
    /// Every method callable on an instance of this class — its own plus
    /// every inherited one — as `(name, code address)`, sorted by name so
    /// [`ClassDesc::method`] is a binary search. Empty until
    /// [`ClassTable::set_methods`] fills it, which `mwl-codegen` does after
    /// the unit is finalized: see this module's docs for why a name and not a
    /// slot index.
    methods: Vec<(String, *const u8)>,
}

impl ClassDesc {
    /// The class's rendered name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// How many [`Value`] slots an instance of this class has, including every
    /// ancestor's.
    #[must_use]
    pub fn field_count(&self) -> usize {
        self.field_count
    }

    /// Whether an instance of this class is also an instance of `other` —
    /// `instanceof`'s whole test, and a typed `catch`'s.
    ///
    /// # Safety
    ///
    /// `other` must refer to a descriptor that is still live, which every
    /// descriptor a live [`ClassTable`] owns is.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "comparing addresses reads nothing, but the caller still owes \
                  the liveness of the descriptor it names"
    )]
    pub unsafe fn conforms_to(&self, other: *const ClassDesc) -> bool {
        std::ptr::eq(self, other) || self.conforms.contains(&other)
    }

    /// The compiled address of the method this class answers `name` with —
    /// its own override if it declares one, otherwise the nearest ancestor's
    /// — or `None` if nothing in the chain declares a *body* for it.
    ///
    /// `None` is an ordinary answer, not a failure: an interface method with
    /// no default body has no code, and [`mwl_class_method`]'s caller supplies
    /// the statically resolved target as the fallback.
    #[must_use]
    pub fn method(&self, name: &str) -> Option<*const u8> {
        self.methods
            .binary_search_by(|(have, _)| have.as_str().cmp(name))
            .ok()
            .map(|index| self.methods[index].1)
    }

    /// How many methods this descriptor answers for — its own plus every
    /// inherited one.
    #[must_use]
    pub fn method_count(&self) -> usize {
        self.methods.len()
    }
}

impl fmt::Debug for ClassDesc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClassDesc")
            .field("name", &self.name)
            .field("field_count", &self.field_count)
            .field("conforms", &self.conforms.len())
            .field("methods", &self.methods.len())
            .finish()
    }
}

/// An index into a [`ClassTable`] — how Rust-side code names a class before it
/// has a raw descriptor pointer to bake into compiled code.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ClassId(usize);

/// Every class one compiled unit declares, and the owner of their
/// [`ClassDesc`]s.
///
/// Each descriptor is boxed, so its address is stable for the table's whole
/// life even as later classes are defined — which is what lets
/// [`ClassTable::desc`] hand out a pointer `mwl-codegen` embeds in machine
/// code.
///
/// The table must outlive every instance of every class it defines, and every
/// compiled function that can allocate one. `mwl-codegen`'s compiled unit is
/// the natural owner.
#[derive(Debug, Default)]
pub struct ClassTable {
    /// Boxed individually, and deliberately so: `ClassTable::desc` hands out a
    /// raw pointer that `mwl-codegen` bakes into machine code, and a
    /// `Vec<ClassDesc>` would move every descriptor the next `define` reallocs.
    /// `clippy::vec_box` cannot see that the indirection *is* the point.
    #[expect(
        clippy::vec_box,
        reason = "each descriptor's address must survive later `define` calls;                   see the field's own comment"
    )]
    classes: Vec<Box<ClassDesc>>,
}

impl ClassTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Defines a class or interface.
    ///
    /// `field_count` is the *total* slot count including every ancestor's (see
    /// this module's docs); an interface's is zero, since nothing instantiates
    /// one. `parents` names the direct superclass and every directly
    /// implemented interface — each must already be defined in this same
    /// table, which the checker's own hierarchy pass already orders.
    ///
    /// # Panics
    ///
    /// If a `parents` entry does not belong to this table.
    pub fn define(
        &mut self,
        name: impl Into<String>,
        field_count: usize,
        parents: &[ClassId],
    ) -> ClassId {
        let mut conforms: Vec<*const ClassDesc> = Vec::new();
        for parent in parents {
            let desc = self
                .classes
                .get(parent.0)
                .expect("a parent class id always belongs to the table defining its child");
            let ptr: *const ClassDesc = &raw const **desc;
            if !conforms.contains(&ptr) {
                conforms.push(ptr);
            }
            for inherited in &desc.conforms {
                if !conforms.contains(inherited) {
                    conforms.push(*inherited);
                }
            }
        }
        let id = ClassId(self.classes.len());
        self.classes.push(Box::new(ClassDesc {
            name: name.into(),
            field_count,
            conforms,
            methods: Vec::new(),
        }));
        id
    }

    /// Fills in `id`'s method table — `(name, code address)` pairs, which this
    /// sorts by name so [`ClassDesc::method`] can binary-search them.
    ///
    /// Separate from [`ClassTable::define`] because a compiled function has no
    /// address until its module is finalized, which is long after every class
    /// is defined. `mwl-codegen` calls this in its own `finish`.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table.
    pub fn set_methods(&mut self, id: ClassId, methods: Vec<(String, *const u8)>) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        desc.methods = methods;
        desc.methods.sort_by(|(a, _), (b, _)| a.cmp(b));
        desc.methods.dedup_by(|(a, _), (b, _)| a == b);
    }

    /// The descriptor pointer for `id` — the token compiled code holds.
    ///
    /// Stays valid for as long as this table is alive and not dropped.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table.
    #[must_use]
    pub fn desc(&self, id: ClassId) -> *const ClassDesc {
        let desc = self
            .classes
            .get(id.0)
            .expect("a class id always belongs to the table that handed it out");
        &raw const **desc
    }

    /// How many classes are defined.
    #[must_use]
    pub fn len(&self) -> usize {
        self.classes.len()
    }

    /// The id of the class named `name`, or `None` if this table defines none.
    ///
    /// A linear scan: the one caller is `mwl-codegen` looking up a single
    /// compiler-owned class once per compiled unit, which is not a place a
    /// second index would pay for itself.
    #[must_use]
    pub fn id_of(&self, name: &str) -> Option<ClassId> {
        self.classes
            .iter()
            .position(|desc| desc.name == name)
            .map(ClassId)
    }

    /// Whether no class is defined.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }
}

/// The header sitting in front of every MWL object's field slots.
///
/// `#[repr(C)]` because compiled code reads these fields at fixed offsets.
/// Never construct one by value — it is only ever the first
/// `size_of::<ObjHeader>()` bytes of a larger allocation made by
/// [`MwlObj::new`], and moving it would leave the fields behind.
#[repr(C)]
#[derive(Debug)]
pub struct ObjHeader {
    /// How many owners hold this allocation. Reaching `0` frees it.
    refcount: Cell<usize>,
    /// The class this is an instance of. Immutable for the allocation's life.
    class: *const ClassDesc,
}

/// Byte offset of the reference count within [`ObjHeader`].
pub const OBJ_REFCOUNT_OFFSET: usize = std::mem::offset_of!(ObjHeader, refcount);

/// Byte offset of the class-descriptor pointer within [`ObjHeader`].
pub const OBJ_CLASS_OFFSET: usize = std::mem::offset_of!(ObjHeader, class);

/// Byte offset of field slot zero, relative to the [`ObjHeader`] pointer.
pub const FIELDS_OFFSET: usize = std::mem::size_of::<ObjHeader>();

/// Byte stride between consecutive field slots — one whole [`Value`].
pub const FIELD_STRIDE: usize = std::mem::size_of::<Value>();

/// Byte offset of field slot `index`, relative to the [`ObjHeader`] pointer.
///
/// This is the one place the arithmetic lives, so `mwl-codegen` queries the
/// layout rather than restating it.
///
/// # Panics
///
/// If the offset overflows a `usize`, which no real class reaches.
#[must_use]
pub const fn field_offset(index: usize) -> usize {
    match index.checked_mul(FIELD_STRIDE) {
        Some(scaled) => match FIELDS_OFFSET.checked_add(scaled) {
            Some(offset) => offset,
            None => panic!("an object's field offset overflows the address space"),
        },
        None => panic!("an object's field offset overflows the address space"),
    }
}

/// The allocation shape for an instance with `field_count` slots.
fn obj_layout(field_count: usize) -> Layout {
    let size = field_offset(field_count);
    Layout::from_size_align(size, std::mem::align_of::<ObjHeader>())
        .expect("an object layout is always valid: alignment is a power of two")
}

const _: () = assert!(std::mem::align_of::<Value>() <= std::mem::align_of::<ObjHeader>());

/// An owning handle to one reference of an MWL object.
///
/// Cloning retains, dropping releases — so Rust-side code (helpers, tests, and
/// eventually `mwl-stdlib`) manipulates objects without writing a refcount
/// operation by hand, exactly the way [`crate::MwlStr`] already works.
/// Compiled code instead calls the
/// [`mwl_object_new`]/[`mwl_object_retain`]/[`mwl_object_release`] primitives.
///
/// Neither `Send` nor `Sync`, by construction — see [`crate::string`]'s own
/// docs for the reasoning, which is identical here.
#[repr(transparent)]
pub struct MwlObj {
    ptr: NonNull<ObjHeader>,
}

impl MwlObj {
    /// Allocates a fresh instance of `class` with a reference count of one and
    /// every field slot `null`.
    ///
    /// The slots start `null` rather than uninitialized so that an object
    /// released *before* its constructor finished — a `throw` partway through
    /// one — sweeps well-formed values.
    /// [ADR 0022](../../../docs/adr/0022-definite-property-initialization.md)
    /// makes that state unobservable to MWL code; this only makes it safe to
    /// free.
    ///
    /// # Safety
    ///
    /// `class` must refer to a descriptor that stays live for at least as long
    /// as the object.
    ///
    /// # Panics
    ///
    /// Aborts the process through [`handle_alloc_error`] if the allocator
    /// fails — see [`crate::MwlStr::new`] for why that is the honest behaviour
    /// until a per-request arena exists.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the descriptor's liveness is the caller's obligation to state"
    )]
    pub unsafe fn new(class: *const ClassDesc) -> Self {
        #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
        let field_count = unsafe { (*class).field_count };
        let layout = obj_layout(field_count);
        #[expect(
            unsafe_code,
            reason = "a flexible-array-member allocation cannot be expressed in \
                      safe Rust; `layout` is non-zero-sized because \
                      FIELDS_OFFSET > 0, which is `alloc`'s one precondition"
        )]
        let raw = unsafe { alloc(layout) };
        let Some(ptr) = NonNull::new(raw.cast::<ObjHeader>()) else {
            handle_alloc_error(layout)
        };
        #[expect(
            unsafe_code,
            reason = "`ptr` is a fresh, uninitialized, correctly aligned \
                      allocation of exactly `layout`, so writing the header and \
                      then `field_count` Values behind it stays inside it; \
                      `Value` is no more aligned than `ObjHeader`, asserted above"
        )]
        unsafe {
            ptr.as_ptr().write(ObjHeader {
                refcount: Cell::new(1),
                class,
            });
            let slots = raw.add(FIELDS_OFFSET).cast::<Value>();
            for index in 0..field_count {
                slots.add(index).write(Value::null());
            }
        }
        Self { ptr }
    }

    /// The class this is an instance of.
    #[must_use]
    pub fn class(&self) -> *const ClassDesc {
        self.header().class
    }

    /// How many field slots this instance has.
    #[must_use]
    pub fn field_count(&self) -> usize {
        #[expect(
            unsafe_code,
            reason = "an object's descriptor outlives it by `MwlObj::new`'s own \
                      safety contract"
        )]
        unsafe {
            (*self.class()).field_count
        }
    }

    /// Reads field slot `index` **without** taking a reference to whatever it
    /// holds — the borrow-shaped read `mwl_ir::InstKind::FieldGet` performs.
    ///
    /// # Panics
    ///
    /// If `index` is out of range for this object's class.
    #[must_use]
    pub fn field(&self, index: usize) -> Value {
        assert!(
            index < self.field_count(),
            "field slot {index} is out of range for {}",
            self.class_name()
        );
        #[expect(
            unsafe_code,
            reason = "the bound was just checked, and every slot was initialized \
                      by `new`; this handle owns a reference keeping it live"
        )]
        unsafe {
            *field_ptr(self.ptr.as_ptr(), index)
        }
    }

    /// Reads field slot `index` and leaves `null` there, **transferring** the
    /// reference the slot held to the caller.
    ///
    /// The one operation [`Self::field`] (a borrow) and [`Self::set_field`] (a
    /// release-then-store) cannot compose into: `crate::throwable`'s backtrace
    /// append needs the array's *only* reference in hand so copy-on-write does
    /// not separate it, and retaining first would do exactly that.
    ///
    /// # Panics
    ///
    /// If `index` is out of range for this object's class.
    #[must_use]
    pub(crate) fn take_field(&self, index: usize) -> Value {
        assert!(
            index < self.field_count(),
            "field slot {index} is out of range for {}",
            self.class_name()
        );
        #[expect(
            unsafe_code,
            reason = "the bound was just checked; the slot held a well-formed \
                      Value whose single reference is moved to the caller, and \
                      `null` needs none"
        )]
        unsafe {
            let slot = field_ptr(self.ptr.as_ptr(), index);
            let held = *slot;
            slot.write(Value::null());
            held
        }
    }

    /// Overwrites field slot `index` with `value`, releasing whatever it held.
    ///
    /// Takes over `value`'s reference: the object owns it afterwards, exactly
    /// the way `mwl_ir::InstKind::FieldSet`'s caller-side retain arranges.
    ///
    /// # Panics
    ///
    /// If `index` is out of range for this object's class.
    pub fn set_field(&self, index: usize, value: Value) {
        assert!(
            index < self.field_count(),
            "field slot {index} is out of range for {}",
            self.class_name()
        );
        #[expect(
            unsafe_code,
            reason = "the bound was just checked; the slot held a well-formed \
                      Value whose reference this object owned, so releasing it \
                      before overwriting is exactly one release"
        )]
        unsafe {
            let slot = field_ptr(self.ptr.as_ptr(), index);
            (*slot).release();
            slot.write(value);
        }
    }

    /// How many owners currently hold this allocation.
    #[must_use]
    pub fn refcount(&self) -> usize {
        self.header().refcount.get()
    }

    /// The class's rendered name.
    #[must_use]
    pub fn class_name(&self) -> &str {
        #[expect(
            unsafe_code,
            reason = "an object's descriptor outlives it by `MwlObj::new`'s own \
                      safety contract"
        )]
        unsafe {
            (*self.class()).name()
        }
    }

    /// Whether this object is an instance of `class` — `instanceof`.
    ///
    /// # Safety
    ///
    /// `class` must refer to a live descriptor.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the named descriptor's liveness is the caller's obligation"
    )]
    pub unsafe fn is_instance_of(&self, class: *const ClassDesc) -> bool {
        #[expect(
            unsafe_code,
            reason = "this object's own descriptor is live by `new`'s contract, \
                      and the caller guarantees the named one is"
        )]
        unsafe {
            (*self.class()).conforms_to(class)
        }
    }

    fn header(&self) -> &ObjHeader {
        #[expect(
            unsafe_code,
            reason = "`self.ptr` is live for `&self`'s borrow: this handle owns \
                      one of the references keeping it alive"
        )]
        unsafe {
            self.ptr.as_ref()
        }
    }

    /// Gives up ownership of this handle's reference, yielding the raw pointer
    /// compiled code holds.
    ///
    /// The caller now owns exactly one reference and must eventually pass the
    /// pointer to [`mwl_object_release`] or [`MwlObj::from_raw`].
    #[must_use]
    pub fn into_raw(self) -> *mut ObjHeader {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }

    /// Reclaims a reference previously given up by [`MwlObj::into_raw`].
    ///
    /// # Safety
    ///
    /// `ptr` must be a pointer produced by [`MwlObj::into_raw`] (or by
    /// [`mwl_object_new`]) whose reference has not already been released, and
    /// it must not be reclaimed twice.
    ///
    /// # Panics
    ///
    /// If `ptr` is null, which no MWL object pointer ever is.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "reclaiming a reference is the caller's obligation to state"
    )]
    pub unsafe fn from_raw(ptr: *mut ObjHeader) -> Self {
        Self {
            ptr: NonNull::new(ptr).expect("an MWL object pointer is never null"),
        }
    }

    /// How many owners hold the object `ptr` refers to.
    ///
    /// # Safety
    ///
    /// `ptr` must refer to a live MWL object allocation.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the pointee's liveness is the caller's obligation to state"
    )]
    pub unsafe fn refcount_of(ptr: *const ObjHeader) -> usize {
        #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
        unsafe {
            (*ptr).refcount.get()
        }
    }

    /// The class of the object `ptr` refers to.
    ///
    /// # Safety
    ///
    /// `ptr` must refer to a live MWL object allocation.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the pointee's liveness is the caller's obligation to state"
    )]
    pub unsafe fn class_of(ptr: *const ObjHeader) -> *const ClassDesc {
        #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
        unsafe {
            (*ptr).class
        }
    }
}

impl Clone for MwlObj {
    fn clone(&self) -> Self {
        bump(self.ptr.as_ptr());
        Self { ptr: self.ptr }
    }
}

impl Drop for MwlObj {
    fn drop(&mut self) {
        #[expect(
            unsafe_code,
            reason = "this handle owns exactly the reference being dropped"
        )]
        unsafe {
            release_graph(self.ptr.as_ptr());
        }
    }
}

impl fmt::Debug for MwlObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MwlObj")
            .field("class", &self.class_name())
            .field("refcount", &self.refcount())
            .field("fields", &self.field_count())
            .finish()
    }
}

/// The address of field slot `index` within the object at `ptr`.
///
/// # Safety
///
/// `ptr` must refer to a live MWL object allocation with more than `index`
/// field slots.
#[expect(
    unsafe_code,
    reason = "the pointee's liveness and the slot's existence are the caller's \
              obligation to state"
)]
unsafe fn field_ptr(ptr: *mut ObjHeader, index: usize) -> *mut Value {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the slot is inside the allocation, and \
                  `MwlObj::new` initialized every one of them"
    )]
    unsafe {
        ptr.cast::<u8>().add(field_offset(index)).cast::<Value>()
    }
}

/// Adds one reference to the object at `ptr`.
///
/// Not itself `unsafe` to *call* from this module because every caller here
/// already holds a live handle; the pointer dereference is the usual "a live
/// allocation" obligation stated at each call site.
fn bump(ptr: *mut ObjHeader) {
    #[expect(
        unsafe_code,
        reason = "every caller in this module holds a live reference to `ptr`"
    )]
    let header = unsafe { &*ptr };
    header.refcount.set(
        header
            .refcount
            .get()
            .checked_add(1)
            .expect("an MWL object's reference count cannot overflow a usize"),
    );
}

/// Drops one reference to the object at `ptr`, reporting whether that was the
/// last.
///
/// Does **not** free: [`crate::release`] owns that step, so a nested field's
/// release never recurses.
///
/// # Safety
///
/// `ptr` must refer to a live MWL object allocation whose reference the caller
/// owns.
#[expect(
    unsafe_code,
    reason = "owning the reference is the caller's obligation to state"
)]
pub(crate) unsafe fn drop_one(ptr: *mut ObjHeader) -> bool {
    #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
    let header = unsafe { &*ptr };
    let remaining = header.refcount.get() - 1;
    header.refcount.set(remaining);
    remaining == 0
}

/// Drops one reference to `root`, freeing it and everything it solely owns.
///
/// Iterative by construction — [`crate::release`] owns the worklist, and its
/// own docs are the one home for why the depth of a user's data structure must
/// not decide whether the process survives freeing it.
///
/// # Safety
///
/// `root` must refer to a live MWL object allocation whose reference the
/// caller owns, and must not be released twice.
#[expect(
    unsafe_code,
    reason = "owning the reference is the caller's obligation to state"
)]
pub unsafe fn release_graph(root: *mut ObjHeader) {
    if root.is_null() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees it owns `root`'s reference"
    )]
    unsafe {
        crate::release::release_value(Value::from_obj_ptr(root));
    }
}

/// Frees an object allocation whose count reached zero, handing every field
/// slot's value to `work` rather than releasing it here — see
/// [`crate::release`].
///
/// # Safety
///
/// `ptr` must refer to an MWL object allocation whose reference count reached
/// zero in [`drop_one`], and must be dismantled exactly once.
#[expect(
    unsafe_code,
    reason = "reaching zero exactly once is the caller's obligation to state"
)]
pub(crate) unsafe fn dismantle(ptr: *mut ObjHeader, work: &mut Vec<crate::release::Dying>) {
    #[expect(
        unsafe_code,
        reason = "the count reached zero, so nothing else can observe the \
                  allocation; every slot was initialized by `new`, and the \
                  layout is recomputed from the same field count `new` \
                  allocated with, before the header is freed"
    )]
    unsafe {
        let field_count = (*MwlObj::class_of(ptr)).field_count;
        for index in 0..field_count {
            if let Some(dying) = crate::release::step_field(*field_ptr(ptr, index)) {
                work.push(dying);
            }
        }
        dealloc(ptr.cast::<u8>(), obj_layout(field_count));
    }
}

// ---------------------------------------------------------------------------
// The primitives compiled code calls
// ---------------------------------------------------------------------------
//
// The same split `crate::string`'s own primitives are on, for the same reason:
// none of these can fail, so none of them wears ADR 0002's checked-return
// shape. Every one is `extern "C"` and never `extern "C-unwind"`.

/// Allocates a fresh instance of `class` with a reference count of one and
/// every field slot `null` — `mwl_ir::InstKind::New`'s allocation half.
///
/// # Safety
///
/// `class` must refer to a descriptor that stays live for at least as long as
/// the object.
#[expect(
    unsafe_code,
    reason = "compiled code passes a descriptor pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_object_new(class: *const ClassDesc) -> *mut ObjHeader {
    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    unsafe {
        MwlObj::new(class).into_raw()
    }
}

/// Adds a reference — `mwl_ir::InstKind::Retain` for a `Ty::Object` operand.
///
/// # Safety
///
/// `ptr` must refer to a live MWL object allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw object pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_object_retain(ptr: *mut ObjHeader) {
    if ptr.is_null() {
        return;
    }
    bump(ptr);
}

/// Drops a reference, freeing the object and everything it solely owns if it
/// was the last — `mwl_ir::InstKind::Release` for a `Ty::Object` operand.
///
/// # Safety
///
/// `ptr` must refer to a live MWL object allocation whose reference this
/// caller owns, and must not be released twice.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw object pointer whose ownership the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_object_release(ptr: *mut ObjHeader) {
    #[expect(unsafe_code, reason = "the caller guarantees it owns the reference")]
    unsafe {
        release_graph(ptr);
    }
}

/// `$obj instanceof Class`, and the type test a typed `catch` clause performs.
///
/// A null `ptr` answers `false`, the same "a null payload *is* `null`"
/// treatment every retain/release primitive here already gives one — and the
/// reason a `catch` dispatch stays safe when `mwl_take_thrown` hands back
/// nothing (see `crate::throwable`).
///
/// # Safety
///
/// `ptr` must be null or refer to a live MWL object allocation, and `class`
/// to a live descriptor.
#[expect(
    unsafe_code,
    reason = "compiled code passes an object pointer and a descriptor pointer \
              whose liveness the signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_object_instanceof(
    ptr: *const ObjHeader,
    class: *const ClassDesc,
) -> bool {
    if ptr.is_null() {
        return false;
    }
    #[expect(unsafe_code, reason = "the caller guarantees both pointees are live")]
    unsafe {
        (*MwlObj::class_of(ptr)).conforms_to(class)
    }
}

/// The code address `class` answers the method `name` with, or `fallback` if
/// it answers none — the whole of `static::method(...)`'s dispatch, and of
/// `new static(...)`'s constructor lookup.
///
/// `name` is a UTF-8 byte range in the compiled unit's own data section, so it
/// is neither owned nor freed here. A null `class` answers `fallback`, the
/// same "a null payload *is* `null`" treatment every primitive in this module
/// gives one.
///
/// `fallback` is what the compiler statically resolved for the same call site.
/// A subclass can override a method but never remove one, so the fallback is
/// unreachable for any class the unit compiled a table for — it is what keeps
/// a descriptor the unit never filled in (an interface, a class from another
/// unit) a correct call rather than a jump through null.
///
/// # Safety
///
/// `class` must be null or refer to a live descriptor, and `name`/`len` must
/// describe a live, initialized byte range.
#[must_use]
#[expect(
    unsafe_code,
    reason = "compiled code passes a descriptor pointer and a data-section \
              byte range whose liveness the signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_class_method(
    class: *const ClassDesc,
    name: *const u8,
    len: usize,
    fallback: *const u8,
) -> *const u8 {
    if class.is_null() {
        return fallback;
    }
    #[expect(unsafe_code, reason = "the caller guarantees both pointees are live")]
    let (desc, bytes) = unsafe { (&*class, std::slice::from_raw_parts(name, len)) };
    let Ok(name) = std::str::from_utf8(bytes) else {
        return fallback;
    };
    desc.method(name).unwrap_or(fallback)
}

/// The class name of the object at `ptr`, as a fresh MWL string —
/// `Core\Reflect`'s eventual `nameOf`, and what a diagnostic renders.
///
/// # Safety
///
/// `ptr` must refer to a live MWL object allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw object pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_object_class_name(
    ptr: *const ObjHeader,
) -> *mut crate::string::StrHeader {
    #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
    let name = unsafe { (*MwlObj::class_of(ptr)).name() };
    crate::string::MwlStr::new(name.as_bytes()).into_raw()
}

/// Reads field slot `index` off the object at `ptr`, **without** retaining
/// what it holds — `mwl_ir::InstKind::FieldGet`'s out-of-line form.
///
/// Compiled code loads the slot inline using [`field_offset`] once
/// `mwl-codegen` emits the arithmetic; this exists so the layout is exercised
/// from Rust and so a `mixed`-typed read has a single entry point.
///
/// # Safety
///
/// `ptr` must refer to a live MWL object allocation whose class has more than
/// `index` field slots.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw object pointer and a slot index that \
              the signature cannot bound"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_object_field_get(ptr: *mut ObjHeader, index: usize) -> Value {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the allocation is live and the slot is \
                  in range; `MwlObj::new` initialized every slot"
    )]
    unsafe {
        *field_ptr(ptr, index)
    }
}

/// Overwrites field slot `index` on the object at `ptr`, releasing whatever it
/// held and taking over `value`'s reference —
/// `mwl_ir::InstKind::FieldSet`'s out-of-line form.
///
/// # Safety
///
/// `ptr` must refer to a live MWL object allocation whose class has more than
/// `index` field slots, and `value` must own the reference it transfers.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw object pointer, a slot index the \
              signature cannot bound, and a value whose ownership it cannot \
              express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_object_field_set(ptr: *mut ObjHeader, index: usize, value: Value) {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the allocation is live and the slot is \
                  in range; the slot held a well-formed Value this object owned"
    )]
    unsafe {
        let slot = field_ptr(ptr, index);
        (*slot).release();
        slot.write(value);
    }
}

const _: () = assert!(Tag::Object as u8 == 7);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counting_alloc;
    use crate::string::MwlStr;

    /// A table with `Animal`, `Dog extends Animal`, and a `Greets` interface
    /// `Dog` implements — the shape `examples/objects.mwl` needs.
    fn hierarchy() -> (ClassTable, ClassId, ClassId, ClassId) {
        let mut table = ClassTable::new();
        let greets = table.define("Greets", 0, &[]);
        let animal = table.define("Animal", 1, &[]);
        let dog = table.define("Dog", 2, &[animal, greets]);
        (table, animal, dog, greets)
    }

    #[test]
    fn the_layout_constants_describe_the_real_header() {
        assert_eq!(OBJ_REFCOUNT_OFFSET, 0);
        assert_eq!(OBJ_CLASS_OFFSET, std::mem::size_of::<usize>());
        assert_eq!(FIELDS_OFFSET, 2 * std::mem::size_of::<usize>());
        assert_eq!(FIELD_STRIDE, 16);
        assert_eq!(field_offset(0), FIELDS_OFFSET);
        assert_eq!(field_offset(3), FIELDS_OFFSET + 48);
    }

    #[test]
    fn a_fresh_object_has_one_reference_and_null_fields() {
        let (table, _animal, dog, _greets) = hierarchy();
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { MwlObj::new(table.desc(dog)) };
        assert_eq!(object.refcount(), 1);
        assert_eq!(object.class_name(), "Dog");
        assert_eq!(object.field_count(), 2);
        assert_eq!(object.field(0).tag(), Some(Tag::Null));
        assert_eq!(object.field(1).tag(), Some(Tag::Null));
    }

    #[test]
    fn a_descriptors_method_table_answers_by_name_and_falls_back() {
        // The dispatch `static::method(...)` and `new static(...)` run on.
        // Two distinct addresses stand in for two compiled functions; what is
        // under test is that a name reaches the right one, that an unknown
        // name reaches the caller's fallback rather than null, and that a
        // descriptor nobody filled in is a fallback rather than a crash.
        let (mut table, animal, dog, _greets) = hierarchy();
        let base: *const u8 = (mwl_object_new as *const ()).cast();
        let over: *const u8 = (mwl_object_retain as *const ()).cast();
        let miss: *const u8 = (mwl_object_release as *const ()).cast();
        table.set_methods(animal, vec![("describe".to_owned(), base)]);
        table.set_methods(
            dog,
            vec![("describe".to_owned(), over), ("bark".to_owned(), base)],
        );

        #[expect(unsafe_code, reason = "the table outlives every borrow here")]
        unsafe {
            assert_eq!((*table.desc(animal)).method("describe"), Some(base));
            assert_eq!((*table.desc(dog)).method("describe"), Some(over));
            assert_eq!((*table.desc(dog)).method_count(), 2);
            assert_eq!((*table.desc(animal)).method("bark"), None);

            let name = "describe";
            assert_eq!(
                mwl_class_method(table.desc(dog), name.as_ptr(), name.len(), miss),
                over
            );
            let absent = "bark";
            assert_eq!(
                mwl_class_method(table.desc(animal), absent.as_ptr(), absent.len(), miss),
                miss
            );
            assert_eq!(
                mwl_class_method(std::ptr::null(), name.as_ptr(), name.len(), miss),
                miss
            );
        }
    }

    #[test]
    fn a_class_with_no_fields_is_still_a_real_allocation() {
        let mut table = ClassTable::new();
        let marker = table.define("Marker", 0, &[]);
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { MwlObj::new(table.desc(marker)) };
        assert_eq!(object.field_count(), 0);
        assert_eq!(object.refcount(), 1);
    }

    #[test]
    fn cloning_retains_and_dropping_releases() {
        let (table, animal, _dog, _greets) = hierarchy();
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { MwlObj::new(table.desc(animal)) };
        let second = object.clone();
        assert_eq!(object.refcount(), 2);
        drop(second);
        assert_eq!(object.refcount(), 1);
    }

    #[test]
    fn instanceof_sees_the_parent_and_the_interface() {
        let (table, animal, dog, greets) = hierarchy();
        #[expect(unsafe_code, reason = "the table outlives the objects")]
        unsafe {
            let pet = MwlObj::new(table.desc(dog));
            assert!(pet.is_instance_of(table.desc(dog)));
            assert!(pet.is_instance_of(table.desc(animal)));
            assert!(pet.is_instance_of(table.desc(greets)));

            let plain = MwlObj::new(table.desc(animal));
            assert!(plain.is_instance_of(table.desc(animal)));
            assert!(!plain.is_instance_of(table.desc(dog)));
            assert!(!plain.is_instance_of(table.desc(greets)));
        }
    }

    #[test]
    fn a_grandchild_conforms_to_every_ancestor_and_their_interfaces() {
        let mut table = ClassTable::new();
        let named = table.define("Named", 0, &[]);
        let base = table.define("Base", 0, &[named]);
        let mid = table.define("Mid", 0, &[base]);
        let leaf = table.define("Leaf", 0, &[mid]);
        #[expect(unsafe_code, reason = "the table outlives the object")]
        unsafe {
            let object = MwlObj::new(table.desc(leaf));
            for ancestor in [leaf, mid, base, named] {
                assert!(object.is_instance_of(table.desc(ancestor)));
            }
        }
    }

    #[test]
    fn setting_a_field_releases_what_it_replaced() {
        let (table, animal, _dog, _greets) = hierarchy();
        let name = MwlStr::new(b"rex");
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { MwlObj::new(table.desc(animal)) };

        object.set_field(0, Value::str(name.clone()));
        assert_eq!(name.refcount(), 2);
        assert_eq!(object.field(0).as_str_bytes(), Some(&b"rex"[..]));

        object.set_field(0, Value::int(4));
        assert_eq!(name.refcount(), 1);
        assert_eq!(object.field(0).as_int(), Some(4));
    }

    #[test]
    fn dropping_an_object_releases_every_field_it_owns() {
        let (table, animal, _dog, _greets) = hierarchy();
        let name = MwlStr::new(b"cat");
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { MwlObj::new(table.desc(animal)) };
        object.set_field(0, Value::str(name.clone()));
        assert_eq!(name.refcount(), 2);
        drop(object);
        assert_eq!(name.refcount(), 1);
    }

    #[test]
    fn the_raw_primitives_move_the_same_count() {
        let (table, animal, _dog, _greets) = hierarchy();
        #[expect(unsafe_code, reason = "exercising the compiled-code entry points")]
        unsafe {
            let raw = mwl_object_new(table.desc(animal));
            mwl_object_retain(raw);
            assert_eq!(MwlObj::refcount_of(raw), 2);
            assert!(mwl_object_instanceof(raw, table.desc(animal)));

            mwl_object_field_set(raw, 0, Value::int(7));
            assert_eq!(mwl_object_field_get(raw, 0).as_int(), Some(7));

            let class_name = MwlStr::from_raw(mwl_object_class_name(raw));
            assert_eq!(class_name.as_bytes(), b"Animal");

            mwl_object_release(raw);
            assert_eq!(MwlObj::refcount_of(raw), 1);
            mwl_object_release(raw);
        }
    }

    #[test]
    fn a_field_holding_another_object_keeps_it_alive() {
        let (table, animal, dog, _greets) = hierarchy();
        #[expect(unsafe_code, reason = "the table outlives the objects")]
        unsafe {
            let inner = MwlObj::new(table.desc(animal));
            let outer = MwlObj::new(table.desc(dog));
            outer.set_field(0, Value::object(inner.clone()));
            assert_eq!(inner.refcount(), 2);
            drop(outer);
            assert_eq!(inner.refcount(), 1);
        }
    }

    #[test]
    fn an_acyclic_object_graph_releases_every_allocation() {
        // The Stage 5 guard `.claude/loop-goal.md` names. A refcount protocol
        // written by hand is exactly where a leak hides, so this measures the
        // allocator rather than trusting a refcount to have reached zero: the
        // process's live byte count must return to what it was before the
        // graph existed.
        let (table, animal, dog, _greets) = hierarchy();

        let before = counting_alloc::live_bytes();
        {
            #[expect(unsafe_code, reason = "the table outlives every object below")]
            unsafe {
                // A chain 512 deep, each node also owning a string, plus one
                // node referenced twice so a shared subgraph is covered too.
                let shared = MwlObj::new(table.desc(animal));
                shared.set_field(0, Value::str(MwlStr::new(b"shared")));

                let mut head = MwlObj::new(table.desc(dog));
                head.set_field(0, Value::object(shared.clone()));
                head.set_field(1, Value::str(MwlStr::new(b"node 0")));
                for depth in 1..512 {
                    let next = MwlObj::new(table.desc(dog));
                    next.set_field(0, Value::object(head));
                    next.set_field(
                        1,
                        Value::str(MwlStr::new(format!("node {depth}").as_bytes())),
                    );
                    head = next;
                }
                drop(shared);
                drop(head);
            }
        }
        assert_eq!(
            counting_alloc::live_bytes(),
            before,
            "the object graph left allocations behind"
        );
    }

    #[test]
    fn a_deep_chain_is_freed_without_recursing() {
        // 200_000 links: deep enough that a recursive release would overflow
        // the stack on every platform CI runs on. Passing is the whole claim.
        let mut table = ClassTable::new();
        let link = table.define("Link", 1, &[]);
        #[expect(unsafe_code, reason = "the table outlives every object below")]
        unsafe {
            let mut head = MwlObj::new(table.desc(link));
            for _ in 1..200_000 {
                let next = MwlObj::new(table.desc(link));
                next.set_field(0, Value::object(head));
                head = next;
            }
            drop(head);
        }
    }

    #[test]
    fn a_debug_rendering_names_the_class() {
        let (table, animal, _dog, _greets) = hierarchy();
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { MwlObj::new(table.desc(animal)) };
        let rendered = format!("{object:?}");
        assert!(rendered.contains("Animal"), "{rendered}");
        assert!(rendered.contains("refcount: 1"), "{rendered}");
    }

    #[test]
    fn a_table_reports_what_it_holds() {
        let mut table = ClassTable::new();
        assert!(table.is_empty());
        table.define("One", 0, &[]);
        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());
    }
}
