//! MWL's class instance: one heap allocation, a two-word header, and the
//! object's fields inline behind it — plus the [`ClassDesc`] every instance
//! points back at.
//!
//! This is the representation `mwl_ir::ty::Ty::Object` lowers to, and the one
//! thing nearly everything else in M4 waits on (`docs/agent/loop-goal.md`). It
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
//! That is [AGENTS.md](../../../AGENTS.md)'s priority 5 spent on its priorities
//! 2 and 4, which is the direction the ordering permits:
//!
//! * **Releasing an object needs no per-field type table.** The sweep in
//!   [`release_graph`] branches on each slot's own tag. With unboxed slots it
//!   would have to walk a parallel `Vec<Ty>` on the [`ClassDesc`] — a second
//!   structure that must agree with the layout codegen emitted, i.e. exactly
//!   the "invariant every future contributor must remember" AGENTS.md's
//!   memory section names as the wrong trade.
//! * **`mixed` and `?T` fields need no special case.** They are already a
//!   tagged value; a uniform slot is the only representation that holds one
//!   without a second, boxed layout beside the first.
//! * **The read side pays nothing.** A field's static type is known
//!   ([ADR 0007](../../../docs/adr/0007-explicit-type-system.md)), so codegen
//!   loads the payload half directly and never checks the tag on a read. Only
//!   a write pays, and it pays one extra store.
//!
//! The cost is stated as AGENTS.md requires: **8 extra bytes per declared
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
//! Cost, as [AGENTS.md](../../../AGENTS.md) requires: one `(String, *const u8)`
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
//! [`MwlObj::new`] zeroing it and the constructor's first assignment — unless
//! the property declared a default, which [`MwlObj::new`] writes over the zero
//! before anything else runs ([`ClassDesc::defaults`]). Either way that first
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
//! # What a shape write checks
//!
//! [ADR 0036](../../../docs/adr/0036-anonymous-object-shapes.md) § 4 requires a
//! write through an erased or widened view to check the incoming value against
//! the field's *real, concrete declared type*, because § 3 compares a shape's
//! field types by ordinary assignability — so `{n: int}` satisfies a
//! `{n: int|string}` binding, and a shape value is aliased rather than copied.
//! Without the check, a write through the wider view would leave a `string` in
//! a slot the narrow view still loads as an `int`.
//!
//! What is checked is the **tag**, from [`ClassDesc::field_tags`], and the
//! declared type itself is deliberately not carried:
//!
//! * The class a shape literal constructs is named for its **field names
//!   alone** (`$shape{x,y}`, `mwl_ir::lower::shape_class_label`), so `{x: 1}`
//!   and `{x: "s"}` are one class. A declared *type* per slot would have to
//!   mint a class per name-and-type tuple — a bigger class table, a second
//!   naming scheme for `mwl_stdlib` to keep in step with, and all of it read
//!   by one instruction.
//! * A tag closes the failure that matters most: representation confusion,
//!   where a slot's payload is loaded as the wrong machine type. That is a
//!   priority 1 and 2 question ([AGENTS.md](../../../AGENTS.md)); what is left
//!   below is a priority 2 one with no memory-safety edge to it.
//!
//! **What a tag therefore does not catch**, and these are known gaps rather
//! than decisions:
//!
//! 1. **Class identity.** [`Tag::Object`] answers for every class, so a
//!    `{pet: Animal}` view of a `{pet: Dog}` value accepts an `Animal`.
//! 2. **An array's element type.** [`Tag::Array`] answers for every
//!    `array<T>`, so an `array<Animal>` reaches an `array<Dog>` slot.
//! 3. **A field whose declared type admits several tags** — a union, a `?T`,
//!    a `mixed` — is unchecked entirely, because there is no one tag to
//!    compare against and the check must not reject a legal write.
//! 4. **Two literals that share their field names but not their types** fall
//!    back to case 3 for the slots they disagree on;
//!    `mwl_ir::lower::Lowering::record_shape_class` degrades the tag rather
//!    than picking whichever literal it saw first.
//! 5. **A class with no layout of its own** — a closure's environment, a
//!    generator's state — carries no tags, because nothing declares its slots
//!    in source for a type to come from. A *named* class does carry them: an
//!    erased receiver reaches any class at all, so `mwl_ir::lower`'s
//!    `field_reprs` joins every layout's slots against the declared property
//!    types the checker recorded.
//!
//! # Decision: no cycle collector
//!
//! Refcounting only, per `docs/agent/loop-goal.md`. A cyclic object graph is
//! retained until the process exits; see [`crate`]'s own known gaps for the
//! boundary and where the eventual collector belongs.

use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
use std::cell::Cell;
use std::fmt;
use std::ptr::NonNull;

use crate::abi::Fault;
use crate::ctx::Ctx;
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
    /// Every field slot's own name, in slot order, including every ancestor's
    /// — see this module's docs. Its length *is* the slot count, which is why
    /// there is no separate `field_count`: the two could then disagree, and a
    /// `Vec`'s length is the same load a `usize` field would have been.
    ///
    /// Compiled code never reaches these: a `$obj->prop` on a named class is
    /// resolved to a fixed offset at compile time and loads inline. What
    /// needs them is a read through an *erased* view —
    /// [ADR 0036](../../../docs/adr/0036-anonymous-object-shapes.md) § 4's
    /// name-keyed fetch, [`ClassDesc::field_slot`] — where the receiver's
    /// static shape is not the concrete value's own layout. **Cost:** one
    /// `String` per field per class, once per process, not per instance.
    fields: Vec<String>,
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
    /// [ADR 0071](../../../docs/adr/0071-derived-codecs.md)'s derived JSON
    /// field list, in declaration order — empty for every class not carrying
    /// `#[Json\Derive]`, which is the default and costs one empty `Vec` per
    /// descriptor.
    ///
    /// Compiled in rather than reflected: `mwl_types::derive` reads the
    /// attribute, `mwl-codegen` copies the answer here, and
    /// `mwl_stdlib::json`'s encoder and decoder walk it. Filled by
    /// [`ClassTable::set_codec`].
    codec: Vec<CodecField>,
    /// How many parameters this class's `constructor` declares — what a
    /// derived *decoder* has to fill before it can run one, and zero for
    /// every class with no codec.
    ///
    /// Carried beside [`Self::codec`] rather than derived from it because a
    /// skipped field ([ADR 0071](../../../docs/adr/0071-derived-codecs.md)
    /// § 3) leaves a parameter no field names, and a decoder that silently
    /// shortened its argument list would call the constructor with the wrong
    /// arity.
    ctor_arity: usize,
    /// Each field slot that carries a declared `= expr` default, as `(slot,
    /// value)` in slot order — empty for a class that declares none, which is
    /// most of them. Filled by [`ClassTable::set_defaults`].
    ///
    /// This is the whole of what a property initializer *is* at run time:
    /// [`MwlObj::new`] writes these slots straight after nulling them, so a
    /// default reaches an instance however it was built — a compiled `new`,
    /// [`construct`] from native code, or ADR 0071's derived decoder. Compiled
    /// code emits no initializer at all; `mwl_types::defaults` owns why.
    defaults: Vec<(usize, FieldDefault)>,
    /// The one [`Tag`] each field slot's *declared* type admits, in slot
    /// order, or `None` for a slot whose declared type admits more than one —
    /// a union, a `?T`, a `mixed`. **Empty** for a class nothing has told —
    /// one the compiler synthesized rather than laid out from a declaration,
    /// a closure's environment or a generator's state; an empty list means
    /// "unknown", never "no field admits anything". Every class an
    /// [ADR 0036](../../../docs/adr/0036-anonymous-object-shapes.md) § 4
    /// write can name from source carries one entry per slot.
    ///
    /// This is the whole of § 4's *"a write's incoming value is checked
    /// against the field's real, concrete declared type"* — see
    /// [`mwl_object_slot_set`], which is its only reader, and this module's
    /// docs § *What a shape write checks* for what a tag does not catch and
    /// why the declared type itself is not here. Filled by
    /// [`ClassTable::set_field_tags`]. **Cost:** one byte-sized `Option<Tag>`
    /// per field per class, once per process, not per instance.
    field_tags: Vec<Option<Tag>>,
    /// The address of the **native** function that renders an instance of this
    /// class as a `string`, or null for every class that has none — which is
    /// every class a program declares, and every `Core` class the spec gives
    /// no `toString`.
    ///
    /// Not a [`Self::methods`] row, and the difference is the calling
    /// convention rather than the lookup: that table holds *compiled* MWL
    /// functions, which release their parameters, while a native `Core` member
    /// is an ADR 0002 helper and **borrows** its arguments. One table cannot
    /// hold both without a caller having to know which it drew — so the
    /// convention is encoded in which field the address came out of. Filled by
    /// [`ClassTable::set_render`], which only `mwl-stdlib` calls; read by
    /// [`crate::dispatch::call_render`], which [`crate::stringify`] asks
    /// before the method table. **Cost:** one pointer per class, once per
    /// process, not per instance.
    render: *const u8,
    /// The address of this class's [`GENERATOR_UNWIND_METHOD`] entry point, or
    /// null for every class that answers no such name — which is every class
    /// but a generator's synthesized state class.
    ///
    /// A cached copy of one [`Self::method`] row rather than a second lookup,
    /// because its reader is [`dismantle`]: every dying object would otherwise
    /// pay a binary search over its whole method table to learn that it is not
    /// a generator, which is AGENTS.md's priority 3 spent on a question
    /// answered once per class at [`ClassTable::set_methods`] time. **Cost:**
    /// one pointer per class, once per process, not per instance.
    unwind: *const u8,
}

/// The name of the resume-to-unwind entry point on a generator's state class,
/// which is the whole of what [`dismantle`] knows about
/// `mwl_ir::lower::generator`'s transform — that module's `GEN_UNWIND_METHOD`
/// is the same string, and its doc comment owns why the name is unspellable.
///
/// Restated here rather than shared: this crate depends on neither `mwl-ir`
/// nor `mwl-codegen`, and the class label and field names of that transform
/// are restated in the same direction for the same reason.
pub const GENERATOR_UNWIND_METHOD: &str = "gen#unwind";

/// One property default's already-evaluated value — the closed set
/// `mwl_types::defaults::ConstArg` can reach from a *written* property
/// declaration, which is that enum minus the shapes only `Core`'s own
/// signature table produces.
///
/// Deliberately a recipe rather than a ready-made [`Value`]: a `Value` holding
/// a string would make the descriptor a refcount owner, and every instance
/// would then have to be careful to retain it — an invariant paid for on every
/// allocation, in exchange for saving one `strlen`-sized copy on a class that
/// declares a string default. AGENTS.md's ordering puts that the other way
/// round. **Cost:** one [`crate::MwlStr`] allocation per instance per
/// string-defaulted property, and one empty [`crate::MwlArray`] per instance
/// per array-defaulted one — the same allocation the constructor assignment it
/// replaces was already making.
#[derive(Clone, Debug)]
pub enum FieldDefault {
    /// `bool`
    Bool(bool),
    /// `int`
    Int(i64),
    /// `uint`
    Uint(u64),
    /// `float`
    Float(f64),
    /// `string`, already cooked — the octets the slot's fresh
    /// [`crate::MwlStr`] holds.
    Str(String),
    /// `[]` — a fresh empty array, which is the only array constant there is
    /// (`mwl_types::defaults::ConstArg::EmptyArray`).
    EmptyArray,
}

impl FieldDefault {
    /// A fresh [`Value`] for this default, owning one reference to whatever it
    /// allocated.
    #[must_use]
    pub(crate) fn materialize(&self) -> Value {
        match self {
            Self::Bool(v) => Value::bool(*v),
            Self::Int(v) => Value::int(*v),
            Self::Uint(v) => Value::uint(*v),
            Self::Float(v) => Value::float(*v),
            Self::Str(s) => Value::str(crate::MwlStr::new(s.as_bytes())),
            Self::EmptyArray => Value::array(crate::MwlArray::new()),
        }
    }
}

/// What one [`CodecField`] decodes to: the closed set of runtime
/// representations [ADR 0071](../../../docs/adr/0071-derived-codecs.md) § 2's
/// codec-reachable types collapse to once the checker's qualifiers and
/// nominal identity are erased.
///
/// Deliberately not `Tag`: a wire type is a *decode target* rather than a
/// value's current shape, so `Mixed` and `Opaque` have no tag and `Tag::Str`
/// would answer for both `string` and `bytes`, only one of which JSON can
/// carry.
///
/// Deliberately **not** `#[non_exhaustive]`: a variant added here is a wire
/// type gained, and every decoder in the workspace should stop compiling until
/// it has a case for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CodecTy {
    /// `bool`.
    Bool,
    /// `int`.
    Int,
    /// `uint` — the same JSON number as [`Self::Int`], range-checked.
    Uint,
    /// `float`; a JSON integer widens into one.
    Float,
    /// `string`.
    Str,
    /// `mixed` — whatever the document held, unchecked
    /// ([ADR 0007](../../../docs/adr/0007-explicit-type-system.md)).
    Mixed,
    /// A declared type this decoder has no case for yet — an `array<T>`, a
    /// nested class, an enum, a `decimal`, an `Instant`. Encoding one still
    /// works; decoding into one is `mwl_stdlib::json`'s own known gap, and it
    /// faults naming the field rather than guessing a value.
    Opaque,
}

/// One field of a class's derived JSON codec: the wire key, the slot it is
/// read from, the constructor position it is written to, and what a decode
/// must produce for it.
///
/// One struct shared by all four crates that touch it — `mwl_types::derive`
/// produces the declaration half, `mwl_ir::lower::lower_file` joins the slot
/// and constructor indices in, `mwl-codegen` copies it here — so a field
/// added to the wire contract cannot reach the runtime under a different
/// shape than it left the checker.
#[derive(Clone, Debug)]
pub struct CodecField {
    /// The JSON key: the property's own name, or `#[Json\Field(name: "…")]`.
    pub key: String,
    /// The field slot an encode reads and a decode's `new` ends up writing.
    pub slot: usize,
    /// This field's position in the constructor's parameter list — ADR 0071
    /// § 2's "every field is a same-named constructor parameter", resolved to
    /// an index so a decoder needs no name lookup.
    pub param: usize,
    /// What a decode has to produce for this field.
    pub ty: CodecTy,
    /// Whether the declared type admits `null` — ADR 0071 § 4's second
    /// column, which is a property of the *type* and says nothing about
    /// whether the key may be absent.
    pub nullable: bool,
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
        self.fields.len()
    }

    /// The slot `name` occupies on an instance of this class, or `None` if
    /// this class has no such field —
    /// [ADR 0036](../../../docs/adr/0036-anonymous-object-shapes.md) § 4's
    /// name-keyed fetch, which is what a read through an erased or widened
    /// view resolves through.
    ///
    /// `hint` is the slot the *static* type said the field was at, tried
    /// first: where the receiver's shape is the value's own shape — a literal
    /// read straight back — that is one length-and-bytes comparison and the
    /// scan never runs. Where it is a widened view it is simply wrong, and the
    /// scan below is the answer. A linear scan and not a sorted index because
    /// a class's slot count is small and the hint carries the common case;
    /// [`ClassDesc::method`]'s binary search exists because a dispatch has no
    /// equivalent hint.
    #[must_use]
    pub fn field_slot(&self, name: &str, hint: usize) -> Option<usize> {
        if self.fields.get(hint).is_some_and(|field| field == name) {
            return Some(hint);
        }
        self.fields.iter().position(|field| field == name)
    }

    /// The one [`Tag`] slot `index`'s declared type admits, or `None` where
    /// it admits several or where nothing told this class its field types —
    /// see [`ClassDesc::field_tags`], which owns both readings of `None`.
    #[must_use]
    pub fn field_tag(&self, index: usize) -> Option<Tag> {
        self.field_tags.get(index).copied().flatten()
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

    /// ADR 0071's derived JSON field list, in declaration order — empty for a
    /// class carrying no `#[Json\Derive]`.
    ///
    /// Declaration order is the encode order, which is what makes an encoded
    /// document byte-deterministic across runs and machines (that ADR § 2).
    #[must_use]
    pub fn codec(&self) -> &[CodecField] {
        &self.codec
    }

    /// How many parameters this class's `constructor` declares — see
    /// [`Self::codec`]'s companion field.
    #[must_use]
    pub fn ctor_arity(&self) -> usize {
        self.ctor_arity
    }

    /// The address of this class's native renderer, or `None` for a class
    /// carrying none — see [`Self::render`] for why it is not a method row.
    #[must_use]
    pub fn renderer(&self) -> Option<*const u8> {
        if self.render.is_null() {
            None
        } else {
            Some(self.render)
        }
    }

    /// The address of this class's [`GENERATOR_UNWIND_METHOD`] entry point, or
    /// `None` for a class that answers no such name — see [`Self::unwind`].
    #[must_use]
    pub fn unwind_entry(&self) -> Option<*const u8> {
        if self.unwind.is_null() {
            None
        } else {
            Some(self.unwind)
        }
    }
}

impl fmt::Debug for ClassDesc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClassDesc")
            .field("name", &self.name)
            .field("field_count", &self.fields.len())
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
    /// `fields` names every slot in slot order, *including* every ancestor's
    /// (see this module's docs), so its length is the total slot count; an
    /// interface's is empty, since nothing instantiates one. `parents` names
    /// the direct superclass and every directly implemented interface — each
    /// must already be defined in this same table, which the checker's own
    /// hierarchy pass already orders.
    ///
    /// The names are taken here rather than filled in afterwards the way
    /// [`ClassTable::set_codec`] and [`ClassTable::set_methods`] are: those
    /// two carry facts from a different table, or an address that does not
    /// exist yet, whereas a slot's name is the same fact as its existence.
    ///
    /// # Panics
    ///
    /// If a `parents` entry does not belong to this table.
    pub fn define(
        &mut self,
        name: impl Into<String>,
        fields: &[impl AsRef<str>],
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
            fields: fields.iter().map(|f| f.as_ref().to_owned()).collect(),
            conforms,
            methods: Vec::new(),
            codec: Vec::new(),
            ctor_arity: 0,
            defaults: Vec::new(),
            field_tags: Vec::new(),
            render: std::ptr::null(),
            unwind: std::ptr::null(),
        }));
        id
    }

    /// Fills in `id`'s per-slot declared tags — see [`ClassDesc::field_tags`].
    ///
    /// Separate from [`ClassTable::define`] on [`ClassTable::set_defaults`]'
    /// terms exactly: a slot's *name* is the same fact as its existence, while
    /// what its declared type admits is a second table's answer that
    /// `mwl-codegen` maps out of `mwl_ir::Ty` on the way here.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table, or if `tags` is not one entry
    /// per slot — a length disagreement would silently check one field against
    /// another's type, which is worse than checking nothing.
    pub fn set_field_tags(&mut self, id: ClassId, tags: Vec<Option<Tag>>) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        assert!(
            tags.len() == desc.fields.len(),
            "`{}` has {} field slots but {} declared tags",
            desc.name,
            desc.fields.len(),
            tags.len()
        );
        desc.field_tags = tags;
    }

    /// Fills in `id`'s declared property defaults — see [`ClassDesc::defaults`].
    ///
    /// Separate from [`ClassTable::define`] on exactly [`ClassTable::set_codec`]'s
    /// terms: the fact comes from a different `mwl-ir` table, and there is no
    /// compiled address to wait for.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table, or if a slot index is out of
    /// range for the class — which would mean `mwl-ir` joined a default
    /// against the wrong layout, and writing past the allocation is not a
    /// failure to discover at run time.
    pub fn set_defaults(&mut self, id: ClassId, defaults: Vec<(usize, FieldDefault)>) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        assert!(
            defaults.iter().all(|(slot, _)| *slot < desc.fields.len()),
            "a property default names a slot `{}` does not have",
            desc.name
        );
        desc.defaults = defaults;
    }

    /// Fills in `id`'s ADR 0071 derived-codec field list — see
    /// [`ClassDesc::codec`].
    ///
    /// Separate from [`ClassTable::define`] only because the two facts come
    /// from two different `mwl-ir` tables; unlike [`ClassTable::set_methods`]
    /// there is no address to wait for, so `mwl-codegen` calls this straight
    /// after defining the class.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table.
    pub fn set_codec(&mut self, id: ClassId, codec: Vec<CodecField>, ctor_arity: usize) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        desc.codec = codec;
        desc.ctor_arity = ctor_arity;
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
        // Resolved once per class here rather than once per dying instance in
        // `dismantle` — see `ClassDesc::unwind`.
        desc.unwind = desc
            .method(GENERATOR_UNWIND_METHOD)
            .unwrap_or(std::ptr::null());
    }

    /// Fills in `id`'s native renderer — see [`ClassDesc::renderer`].
    ///
    /// `address` is an ADR 0002 helper that takes the instance as its one
    /// argument and **borrows** it, which is what separates this from
    /// [`ClassTable::set_methods`]; `mwl_stdlib::instance` is its only caller,
    /// because a class whose renderer is native is a `Core` class by
    /// definition and that crate is where the registry saying so lives.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table.
    pub fn set_render(&mut self, id: ClassId, address: *const u8) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        desc.render = address;
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
        let object = unsafe { Self::alloc(class) };
        #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
        let desc = unsafe { &*class };
        // Every declared `= expr` default, written over the null the slot was
        // just given — see [`ClassDesc::defaults`]. `set_field` releases what
        // it overwrites, which is a `null` here and therefore free.
        for (slot, default) in &desc.defaults {
            object.set_field(*slot, default.materialize());
        }
        object
    }

    /// The allocation half of [`MwlObj::new`]: a fresh instance with every
    /// slot `null` and **no** default applied.
    ///
    /// Its own entry point for exactly one caller — [`mwl_object_clone`],
    /// which overwrites every slot with the source's value and would otherwise
    /// allocate a default string only to release it one line later.
    ///
    /// # Safety
    ///
    /// As [`MwlObj::new`].
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "the descriptor's liveness is the caller's obligation to state"
    )]
    unsafe fn alloc(class: *const ClassDesc) -> Self {
        #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
        let field_count = unsafe { (*class).fields.len() };
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
            (*self.class()).fields.len()
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
    // A debug-only bound, and deliberately not a release one. `mwl-codegen`
    // resolves the slot at compile time from the same `ir::Class::fields` list
    // that built this object's descriptor (`Classes::define`), so the two
    // cannot disagree about a *count*; what this catches is the narrower case
    // where a static class label names a layout the receiver does not have —
    // a subclass whose slots stopped being a prefix of its ancestor's, or a
    // receiver type the checker got wrong. Measured at ~1.4% of a
    // field-heavy program to carry into release, which buys too little for
    // the price when the conformance suite and both fuzz targets run debug.
    // The erased path does not need this: `mwl_object_slot_get` reads the
    // slot off the receiver's own descriptor by name.
    #[cfg(debug_assertions)]
    {
        #[expect(
            unsafe_code,
            reason = "the caller guarantees the allocation is live, so its \
                      descriptor is too"
        )]
        let count = unsafe { (*MwlObj::class_of(ptr)).field_count() };
        assert!(
            index < count,
            "field slot {index} is out of range for a class with {count} slots"
        );
    }
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
/// # An abandoned generator runs its `finally` first
///
/// A class that answers [`GENERATOR_UNWIND_METHOD`] is a generator's state
/// class, and one reaching this while still suspended has `finally` bodies the
/// program entered and never left. They run **before** the field sweep below,
/// because that is where their parked locals still are: the unwind entry point
/// resumes `advance()`, which reloads those very slots.
///
/// Two things make calling compiled code from a release path safe, and neither
/// is optional:
///
/// - **The count is resurrected to one first.** `gen#unwind` borrows argument
///   0 (`mwl_ir::lower::generator::lower_generator_unwind` owns why), so it
///   retains and `advance()` releases on its way out — a pair that would cross
///   zero, and re-enter the release path on the allocation already being
///   dismantled, if it started from the zero this function is handed. The
///   allocation is freed below whatever the count then reads: nothing else can
///   observe it, since it reached zero once already.
/// - **The context is taken from the thread rather than passed.** A decrement
///   carries none — [`crate::ctx::CurrentCtx`] is that decision's home. With
///   no compiled frame running there is no context, no unwind runs, and the
///   sweep proceeds; that is the shape a Rust test dropping a handle takes.
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
        let class = MwlObj::class_of(ptr);
        if let Some(target) = (*class).unwind_entry() {
            unwind_abandoned(ptr, target);
        }
        let field_count = (*class).fields.len();
        for index in 0..field_count {
            if let Some(dying) = crate::release::step_field(*field_ptr(ptr, index)) {
                work.push(dying);
            }
        }
        dealloc(ptr.cast::<u8>(), obj_layout(field_count));
    }
}

/// Runs the unwind entry point at `target` on the dying generator at `ptr` —
/// [`dismantle`]'s first half, whose doc comment owns both of the rules below.
///
/// # Safety
///
/// `ptr` must refer to an MWL object allocation whose reference count reached
/// zero, whose class carries `target` as its [`GENERATOR_UNWIND_METHOD`] row.
#[expect(
    unsafe_code,
    reason = "reaching zero, and the address being this class's own, are both \
              the caller's obligations to state"
)]
unsafe fn unwind_abandoned(ptr: *mut ObjHeader, target: *const u8) {
    crate::ctx::with_current(|ctx| {
        // The resurrection: `gen#unwind` retains and `advance()` releases, and
        // that pair must not cross zero.
        bump(ptr);
        ctx.with_pending_set_aside(|ctx| {
            // The reference the resurrection just made is the one this
            // borrows for the length of the call.
            let _ = crate::dispatch::call_unwind(ctx, Value::from_obj_ptr(ptr), target);
        });
    });
}

// ---------------------------------------------------------------------------
// The primitives compiled code calls
// ---------------------------------------------------------------------------
//
// The same split `crate::string`'s own primitives are on, for the same reason:
// none of these can fail, so none of them wears ADR 0002's checked-return
// shape. Every one is `extern "C"` and never `extern "C-unwind"`.

/// Allocates a fresh instance of `class` with a reference count of one, every
/// field slot `null`, and then every slot that declares one holding its
/// default ([`ClassDesc::defaults`]) — `mwl_ir::InstKind::New`'s allocation
/// half.
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

/// [ADR 0023](../../../docs/adr/0023-clone-serialize-and-cross-boundary-copy.md)
/// § 1's `clone`: a fresh instance of the *same* class whose every slot holds
/// what the original's held, with a reference count of one.
///
/// **Shallow, same-heap, single-level** — PHP's own rule, kept exactly. A slot
/// holding an object ends up pointing at that same object from both copies,
/// with one more reference taken, so mutating `$copy->child->name` is visible
/// through the original. That is the whole of what `clone` means; deep copying
/// is `serialize`/`unserialize`'s recursive graph copy, a different operation.
///
/// No hook runs. ADR 0023 makes `__clone` one of the magic methods MWL does
/// not have, so this is the entire operation — nothing here can throw, which
/// is why it wears no checked-return shape.
///
/// # Safety
///
/// `ptr` must refer to a live MWL object allocation the caller holds a
/// reference to for the duration of the call.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw object pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_object_clone(ptr: *mut ObjHeader) -> *mut ObjHeader {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ptr` is a live allocation it holds a \
                  reference to, so borrowing it for this copy is sound"
    )]
    let (source, copy) = unsafe {
        let source = MwlObj::from_raw(ptr);
        let copy = MwlObj::alloc(source.class());
        (source, copy)
    };
    for index in 0..source.field_count() {
        let value = source.field(index);
        #[expect(
            unsafe_code,
            reason = "the source slot's payload is live because the source is, \
                      and the copy's slot becomes a second owner of it"
        )]
        unsafe {
            value.retain();
        }
        copy.set_field(index, value);
    }
    // The borrow ends here: `from_raw` adopted the caller's reference, which
    // the caller still owns, so it must not be dropped with `source`.
    std::mem::forget(source);
    copy.into_raw()
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

/// [`mwl_object_instanceof`] over a subject whose tag nothing proved — a
/// `mixed`, or a `?Box` no test narrowed, which is the shape `$x instanceof
/// Box` exists to interrogate.
///
/// The subject arrives as a whole [`Value`] by address, the same shape
/// [`mwl_object_slot_get`]'s receiver takes and for the same reason: an
/// unchecked untag in compiled code would dereference an `int` payload. Unlike
/// that fetch there is nothing to throw about — a tag that is not an object
/// simply answers `false`, which is PHP's own answer, and a subject whose
/// *declared* type can hold no object was `E0497` at check time.
///
/// # Safety
///
/// `subject` must point at one initialized [`Value`] its caller still owns,
/// and `class` to a live descriptor.
#[expect(
    unsafe_code,
    reason = "compiled code passes a value by address and a descriptor pointer, \
              neither of whose liveness the signature can express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_value_instanceof(
    subject: *const Value,
    class: *const ClassDesc,
) -> bool {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees this points at one initialized value"
    )]
    let subject = unsafe { *subject };
    let Some(ptr) = subject.obj_ptr() else {
        return false;
    };
    #[expect(unsafe_code, reason = "the caller guarantees both pointees are live")]
    unsafe {
        mwl_object_instanceof(ptr, class)
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

/// The one method [`construct`] runs — ADR 0030 fixes the spelling.
pub const CONSTRUCTOR: &str = "constructor";

/// Builds an instance of `class` by running its own `constructor` — what a
/// native member does where compiled code would emit
/// `mwl_ir::ir::InstKind::New`.
///
/// **Every argument's reference is transferred**, exactly as an ordinary MWL
/// call's is: the callee releases each of its parameters at scope exit, so the
/// caller hands over values it owns and never releases them again. That holds
/// on the error paths too — an argument is consumed whether or not the
/// constructor ran. The returned value is one fresh reference the caller owns.
///
/// This is the second half of the mismatch [`crate::closure::call_closure`]
/// exists for, in the other direction: a helper *borrows* its own arguments
/// and a compiled method *owns* its parameters, so the reconciliation lives
/// here once rather than in each `Core` member that builds an object.
///
/// # Errors
///
/// [`Fault::Pending`] carrying the callee's status when the constructor
/// throws, so the exception it recorded in `ctx` reaches the request
/// unchanged. [`Fault::Fatal`] when `class` declares no [`CONSTRUCTOR`], or
/// when the argument count is not its declared arity — both engine faults:
/// [ADR 0022](../../../docs/adr/0022-definite-property-initialization.md)
/// gives every class exactly one nameable constructor, and the caller reads
/// its arity off the same descriptor.
///
/// # Safety
///
/// `class` must refer to a live descriptor whose method table `mwl-codegen`
/// has already filled.
#[expect(
    unsafe_code,
    reason = "the caller owes the liveness of a descriptor no signature can express"
)]
pub unsafe fn construct(
    ctx: &mut Ctx,
    class: *const ClassDesc,
    args: &[Value],
) -> Result<Value, Fault> {
    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    let desc = unsafe { &*class };
    let Some(target) = desc.method(CONSTRUCTOR) else {
        release_all(args);
        return Err(Fault::fatal(format!(
            "internal error: `{}` was constructed from native code and declares no \
             `{CONSTRUCTOR}`",
            desc.name()
        )));
    };
    #[expect(
        unsafe_code,
        reason = "the address came out of a live descriptor's method table, which \
                  `mwl-codegen` fills only with compiled functions of exactly this \
                  signature"
    )]
    let target: crate::abi::MwlFn = unsafe { std::mem::transmute::<*const u8, _>(target) };

    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    let object = unsafe { MwlObj::new(class) };
    let value = Value::object(object);
    #[expect(
        unsafe_code,
        reason = "the allocation is one line old and this frame holds its only \
                  reference; the second is the one the callee will release"
    )]
    unsafe {
        value.retain();
    }

    let mut slots = Vec::with_capacity(args.len() + 1);
    slots.push(value);
    slots.extend_from_slice(args);
    match crate::abi::call(target, ctx, &slots) {
        Ok(_) => Ok(value),
        Err(status) => {
            #[expect(
                unsafe_code,
                reason = "the callee released the reference it was given; this is the \
                          other one, which nothing will ever read now"
            )]
            unsafe {
                value.release();
            }
            Err(Fault::Pending(status))
        }
    }
}

/// Releases every reference in `values` — [`construct`]'s "an argument is
/// consumed whether or not the constructor ran".
fn release_all(values: &[Value]) {
    for value in values {
        #[expect(
            unsafe_code,
            reason = "each value's reference was transferred to this frame by the \
                      caller, so this frame owes exactly one release for it"
        )]
        unsafe {
            value.release();
        }
    }
}

crate::mwl_helper! {
    /// The floor under [`mwl_class_method`]: what a call to a method with no
    /// body reaches when the receiver's own class declares no override either.
    ///
    /// An `abstract` method and a bodiless interface method name no compiled
    /// function, so a call resolving to one has no static target to fall back
    /// to — `mwl_ir::ir::InstKind::CallVirtual` passes this instead of a null
    /// pointer, which would turn a compiler bug into a jump to address zero.
    /// Reaching it is an engine fault, never user error: the checker refuses a
    /// concrete class that leaves an interface method unimplemented.
    ///
    /// Takes no arguments *by declaration* — it is called with whatever the
    /// original call site passed, and reads none of them.
    fn mwl_abstract_method(_ctx, _args: [0]) {
        Err(Fault::fatal(
            "internal error: a method with no body was called, and no class in the \
             receiver's chain declared one"
                .to_owned(),
        ))
    }
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

/// Reads the field *named* `name` off the object at `ptr`, writing what the
/// slot holds to `out` — [ADR 0036](../../../docs/adr/0036-anonymous-object-shapes.md)
/// § 4's name-keyed fetch, and `mwl_ir::InstKind::SlotGet`'s whole emission.
///
/// The name arrives as static bytes `mwl-codegen` put in the unit's data
/// section rather than as an [`crate::MwlStr`]: a read through a shape must not
/// cost an allocation, and the name is a compile-time constant on every path
/// that reaches here.
///
/// **Borrows.** `out` receives the slot's value without a retain, exactly as
/// an inline `FieldGet` load does, so a consumer that outlives the receiver
/// owes it the retain — see `mwl_ir::InstKind::SlotGet`.
///
/// `hint` is the slot the static type said the field was at; see
/// [`ClassDesc::field_slot`] for what it buys and when it is wrong.
///
/// The receiver arrives as a whole [`Value`] by address rather than as a bare
/// pointer, because § 4's erased half now includes a `mixed` — ADR 0007 § 2's
/// one unchecked position, whose tag nothing before this proved. The tag is
/// therefore checked here, where the *name* is already checked, and an
/// unchecked untag in compiled code (which would dereference an `int` payload)
/// is what that buys.
///
/// # Errors
///
/// A [`Fault::Thrown`] naming the field and the concrete class when that class
/// has no such field — ADR 0036 § 4's "checked, catchable throw; never a
/// silent value, never PHP's warning-plus-`null`". Reached only through a
/// widened or erased view, since a field the receiver's own shape lists is
/// proven present.
///
/// A second [`Fault::Thrown`] when the receiver is not an object at all, in
/// PHP's own wording — a `mixed` is the only receiver that reaches it, every
/// other non-object being `E0495` at check time (ADR 0007 § 7 row 13).
///
/// # Safety
///
/// `ctx` and `out` must satisfy [`crate::run_helper`]'s contract, `receiver`
/// must point at one initialized [`Value`] its caller still owns, and
/// `name`/`len` must describe initialized bytes that live for the call.
#[expect(
    unsafe_code,
    reason = "compiled code passes a value by address and a static byte \
              range, neither of which the signature can bound"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_object_slot_get(
    ctx: *mut Ctx,
    receiver: *const Value,
    name: *const u8,
    len: usize,
    hint: usize,
    out: *mut Value,
) -> i32 {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the byte range is initialized and outlives \
                  this call"
    )]
    let name = unsafe { std::slice::from_raw_parts(name, len) };
    let body = move |_ctx: &mut Ctx, _args: &[Value]| -> crate::HelperResult {
        let name = std::str::from_utf8(name)
            .map_err(|_| Fault::fatal("internal error: a field name that is not UTF-8"))?;
        #[expect(
            unsafe_code,
            reason = "the caller guarantees this points at one initialized value"
        )]
        let receiver = unsafe { *receiver };
        let Some(ptr) = receiver.obj_ptr() else {
            return Err(Fault::thrown(format!(
                "attempt to read property `{name}` on {}",
                receiver.tag().map_or("a malformed value", Tag::describe)
            )));
        };
        if ptr.is_null() {
            return Err(Fault::fatal(format!(
                "internal error: `->{name}` reached a null receiver"
            )));
        }
        #[expect(
            unsafe_code,
            reason = "the caller guarantees the allocation is live, so its descriptor \
                      is too"
        )]
        let desc = unsafe { &*MwlObj::class_of(ptr) };
        let Some(slot) = desc.field_slot(name, hint) else {
            return Err(Fault::thrown(format!(
                "`{}` has no field `{name}`",
                desc.name()
            )));
        };
        #[expect(
            unsafe_code,
            reason = "the slot came out of this object's own descriptor, so it is \
                      inside the allocation and was initialized by `new`"
        )]
        Ok(unsafe { *field_ptr(ptr, slot) })
    };
    #[expect(
        unsafe_code,
        reason = "the caller's contract is exactly `run_helper`'s"
    )]
    unsafe {
        crate::run_helper(ctx, std::ptr::null(), 0, out, body)
    }
}

/// `$issue->path = "x";` — [`mwl_object_slot_get`]'s write half, and
/// [ADR 0036](../../../docs/adr/0036-anonymous-object-shapes.md) § 4's whole
/// write rule: the slot is found by **name** on the receiver's own descriptor,
/// the incoming value is checked against what that class declares the field to
/// hold, and **no field is ever created** — a name the concrete class does not
/// carry is the same catchable throw a read raises, never a new slot.
///
/// **Borrows.** Unlike [`mwl_object_field_set`], which takes over its caller's
/// reference, this retains what it stores and leaves the caller's own alone:
/// the write can throw *after* its operands are in hand, and a transferred
/// reference on that edge has no owner left to release it. `mwl_ir::lower`
/// stages a freshly-built value as an ordinary temporary instead, which both
/// exits already sweep — see `mwl_ir::InstKind::SlotSet`.
///
/// What the slot held is released, so the caller emits no read-back-and-drop
/// pair the way an inline `FieldSet` needs one.
///
/// `hint` is the slot the static type said the field was at, exactly as in
/// [`mwl_object_slot_get`]; `out` receives a null and exists only because
/// [`crate::run_helper`] writes one.
///
/// # Errors
///
/// A [`Fault::Thrown`] when the concrete class has no such field, and a second
/// when it has one whose declared type does not admit this value's tag — the
/// case a *widened* view creates, since ADR 0036 § 3 checks a shape's field
/// types by ordinary assignability and a shape value is aliased rather than
/// copied. See [`ClassDesc::field_tags`] for the granularity of that check and
/// this module's docs for what it does not catch.
///
/// A third when the receiver is not an object at all, in PHP's own wording —
/// [`mwl_object_slot_get`]'s own third, and reachable for the same one reason.
///
/// # Safety
///
/// `ctx` and `out` must satisfy [`crate::run_helper`]'s contract, `name`/`len`
/// must describe initialized bytes that live for the call, and `receiver` and
/// `value` must each point at one initialized [`Value`] its caller still owns.
#[expect(
    unsafe_code,
    reason = "compiled code passes a static byte range and two values by \
              address, neither of which the signature can bound"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mwl_object_slot_set(
    ctx: *mut Ctx,
    receiver: *const Value,
    name: *const u8,
    len: usize,
    hint: usize,
    value: *const Value,
    out: *mut Value,
) -> i32 {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the byte range is initialized and outlives \
                  this call"
    )]
    let name = unsafe { std::slice::from_raw_parts(name, len) };
    let body = move |_ctx: &mut Ctx, _args: &[Value]| -> crate::HelperResult {
        let name = std::str::from_utf8(name)
            .map_err(|_| Fault::fatal("internal error: a field name that is not UTF-8"))?;
        #[expect(
            unsafe_code,
            reason = "the caller guarantees this points at one initialized value"
        )]
        let receiver = unsafe { *receiver };
        let Some(ptr) = receiver.obj_ptr() else {
            return Err(Fault::thrown(format!(
                "attempt to assign property `{name}` on {}",
                receiver.tag().map_or("a malformed value", Tag::describe)
            )));
        };
        if ptr.is_null() {
            return Err(Fault::fatal(format!(
                "internal error: `->{name} =` reached a null receiver"
            )));
        }
        #[expect(
            unsafe_code,
            reason = "the caller guarantees the allocation is live, so its descriptor \
                      is too"
        )]
        let desc = unsafe { &*MwlObj::class_of(ptr) };
        let Some(slot) = desc.field_slot(name, hint) else {
            return Err(Fault::thrown(format!(
                "`{}` has no field `{name}`",
                desc.name()
            )));
        };
        #[expect(
            unsafe_code,
            reason = "the caller guarantees this points at one initialized value"
        )]
        let value = unsafe { *value };
        if let Some(declared) = desc.field_tag(slot) {
            let actual = value.tag();
            if actual != Some(declared) {
                return Err(Fault::thrown(format!(
                    "`{}` declares field `{name}` as {}, so a {} cannot be written to it",
                    desc.name(),
                    declared.describe(),
                    actual.map_or("malformed value", Tag::describe)
                )));
            }
        }
        #[expect(
            unsafe_code,
            reason = "the slot came out of this object's own descriptor, so it is \
                      inside the allocation and was initialized by `new`; the \
                      retain pairs with the reference the slot now owns"
        )]
        unsafe {
            value.retain();
            let slot = field_ptr(ptr, slot);
            (*slot).release();
            slot.write(value);
        }
        Ok(Value::null())
    };
    #[expect(
        unsafe_code,
        reason = "the caller's contract is exactly `run_helper`'s"
    )]
    unsafe {
        crate::run_helper(ctx, std::ptr::null(), 0, out, body)
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
        let greets = table.define("Greets", &[] as &[&str], &[]);
        let animal = table.define("Animal", &["name"], &[]);
        let dog = table.define("Dog", &["name", "breed"], &[animal, greets]);
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
        let marker = table.define("Marker", &[] as &[&str], &[]);
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
        let named = table.define("Named", &[] as &[&str], &[]);
        let base = table.define("Base", &[] as &[&str], &[named]);
        let mid = table.define("Mid", &[] as &[&str], &[base]);
        let leaf = table.define("Leaf", &[] as &[&str], &[mid]);
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

    /// ADR 0023 § 1: `clone` is shallow, same-heap and single-level. The two
    /// halves that matter are that the copy is a *different* allocation and
    /// that a slot holding an object ends up shared, with one more reference,
    /// rather than copied — mutating through one is visible through the other,
    /// which is exactly PHP's rule.
    #[test]
    fn clone_copies_the_slots_and_shares_what_they_point_at() {
        let (table, animal, dog, _greets) = hierarchy();
        #[expect(unsafe_code, reason = "the table outlives every object below")]
        unsafe {
            let shared = MwlObj::new(table.desc(animal));
            let original = MwlObj::new(table.desc(dog));
            original.set_field(0, Value::object(shared.clone()));
            original.set_field(1, Value::str(MwlStr::new(b"name")));
            assert_eq!(shared.refcount(), 2);

            // Through raw pointers rather than `MwlObj::clone`, which would
            // bump the very counts this is measuring.
            let original_ptr = original.into_raw();
            let copy_ptr = mwl_object_clone(original_ptr);
            assert_ne!(
                copy_ptr.cast_const(),
                original_ptr.cast_const(),
                "`clone` must be a second allocation"
            );
            let original = MwlObj::from_raw(original_ptr);
            let copy = MwlObj::from_raw(copy_ptr);
            assert_eq!(copy.class_name(), original.class_name());
            assert_eq!(copy.refcount(), 1);
            // The slot is shared, not copied — a third owner of `shared`.
            assert_eq!(shared.refcount(), 3);
            assert_eq!(copy.field(0).obj_ptr(), original.field(0).obj_ptr());
            assert_eq!(copy.field(1).as_str_bytes(), Some(b"name".as_slice()));

            // Overwriting a slot on the copy leaves the original's alone —
            // "single-level" is the other half of the rule.
            copy.set_field(1, Value::str(MwlStr::new(b"copy")));
            assert_eq!(original.field(1).as_str_bytes(), Some(b"name".as_slice()));
        }
    }

    /// The leak guard's sibling: `clone` takes a reference to everything it
    /// copies, so a cloned graph must release exactly as cleanly as the
    /// original one does.
    #[test]
    fn a_cloned_object_graph_releases_every_allocation() {
        let (table, animal, dog, _greets) = hierarchy();
        let before = counting_alloc::live_bytes();
        {
            #[expect(unsafe_code, reason = "the table outlives every object below")]
            unsafe {
                let shared = MwlObj::new(table.desc(animal));
                shared.set_field(0, Value::str(MwlStr::new(b"shared")));
                let original = MwlObj::new(table.desc(dog));
                original.set_field(0, Value::object(shared.clone()));
                original.set_field(1, Value::str(MwlStr::new(b"name")));
                drop(shared);

                let raw = original.into_raw();
                let copy = MwlObj::from_raw(mwl_object_clone(raw));
                drop(copy);
                drop(MwlObj::from_raw(raw));
            }
        }
        assert_eq!(
            counting_alloc::live_bytes(),
            before,
            "a cloned object graph left allocations behind"
        );
    }

    #[test]
    fn an_acyclic_object_graph_releases_every_allocation() {
        // The Stage 5 guard `docs/agent/loop-goal.md` names. A refcount protocol
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
        let link = table.define("Link", &["next"], &[]);
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

    /// ADR 0036 § 4's name-keyed fetch: the hint is tried first and is right
    /// where the receiver's shape is the value's own, wrong through a widened
    /// view — and a name the class does not carry answers `None`, which is
    /// what `mwl_object_slot_get` turns into a catchable throw.
    #[test]
    fn a_field_is_found_by_name_whether_or_not_the_hint_is_right() {
        let mut table = ClassTable::new();
        let id = table.define("Shape", &["x", "y"], &[]);
        #[expect(unsafe_code, reason = "the table outlives this borrow")]
        let desc = unsafe { &*table.desc(id) };

        assert_eq!(desc.field_slot("y", 1), Some(1));
        // A `{y: int}` view puts `y` at slot 0; the scan corrects it.
        assert_eq!(desc.field_slot("y", 0), Some(1));
        // A hint past the end is not an index error.
        assert_eq!(desc.field_slot("x", 9), Some(0));
        assert_eq!(desc.field_slot("z", 0), None);
    }

    #[test]
    fn a_table_reports_what_it_holds() {
        let mut table = ClassTable::new();
        assert!(table.is_empty());
        table.define("One", &[] as &[&str], &[]);
        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());
    }
}
