//! Novis's class instance: one heap allocation, a four-word header, and the
//! object's fields inline behind it — plus the [`ClassDesc`] every instance
//! points back at.
//!
//! This is the representation `nvs_ir::ty::Ty::Object` lowers to, and the one
//! thing nearly everything else in M4 waits on (`docs/agent/loop-goal.md`). It
//! follows [`crate::string`]'s shape deliberately: one allocation, a
//! [`Cell`]-refcounted header, and the payload behind it at a fixed offset
//! compiled code computes rather than asks for.
//!
//! # Layout
//!
//! ```text
//! offset 0     offset 8      offset 16  offset 24  offset FIELDS_OFFSET
//! +----------+ +-----------+ +--------+ +--------+ +------------------------------+
//! | refcount | | *ClassDesc| | next   | | prev   | | field_count * 16-byte Values |
//! +----------+ +-----------+ +--------+ +--------+ +------------------------------+
//! ```
//!
//! Only the first two words are compiled code's: [`OBJ_REFCOUNT_OFFSET`] and
//! [`OBJ_CLASS_OFFSET`] are unchanged by the two behind them, and the field
//! slots move because [`FIELDS_OFFSET`] is the header's size rather than a
//! literal. A **debug** build adds a fifth word behind `prev`
//! ([`ObjHeader::owner`]) for the same reason, and it costs nothing anywhere
//! else: compiled code asks `nvs-codegen`, which asks this constant, so the two
//! profiles agree with themselves rather than with each other.
//!
//! ## Decision: a field slot is a whole 16-byte [`Value`]
//!
//! Not a native-width slot sized to the field's declared type. An `int` field
//! therefore costs 16 bytes rather than 8, and writing one stores a tag byte
//! nothing reads back.
//!
//! That is [AGENTS.md](/AGENTS.md)'s priority 5 spent on its priorities
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
//!   (`rule:types/declaration`), so codegen
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
//! valid for every subclass — which is what lets `nvs_ir::InstKind::FieldGet`
//! keep naming the *declaring* class rather than the receiver's runtime one.
//!
//! # Decision: `ClassDesc` is opaque, and its address is the class identity
//!
//! Compiled code never reads a field of one. It passes the pointer to
//! [`nvs_object_new`] and [`nvs_object_instanceof`], and those are the only
//! operations that exist. So the struct is an ordinary Rust type, not a
//! `#[repr(C)]` one, and `nvs-codegen` bakes each descriptor's address into
//! the code it emits as a constant — the normal JIT move, and the reason there
//! is no registry lookup on the allocation path.
//!
//! The descriptors themselves are owned by a [`ClassTable`], one per compiled
//! unit, which the unit must keep alive for as long as its code is callable.
//!
//! # Decision: the called class travels in the receiver slot, and a descriptor
//! carries a method table
//!
//! Late static binding ([`docs/implementation-plan.md`](/docs/implementation-plan.md)'s
//! M4: `new static()` through two levels of inheritance returns the *called*
//! class) needs two things compiled code did not have: the called class at the
//! callee, and a way to find that class's own method from it.
//!
//! **The called class travels in argument slot 0.** Every lowered method
//! already has an implicit receiver there, and a static method's caller
//! already fills it — with `null`, because a static method has no `$this`.
//! That slot now carries the late-static-binding class instead: the tag byte
//! stays [`Tag::Null`] (as an Novis *value* the slot still holds nothing, so
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
//! plus every one it inherits, own-first — and `nvs-codegen` fills it in after
//! `finalize_definitions`, which is the first moment a compiled function has
//! an address.
//!
//! That is a *name* lookup, not a vtable index, and deliberately: the only
//! call shape that reaches it today is `static::method()`/`new static()`,
//! where the class is unknown until run time. An ordinary `$obj->method()` is
//! still resolved statically from the receiver's declared type
//! (`nvs-codegen`'s known gap 1), so nothing on the hot path pays for this.
//! Turning that gap into real virtual dispatch wants a compile-time slot index
//! rather than a name — a separate decision, on a table this one already
//! builds.
//!
//! Cost, as [AGENTS.md](/AGENTS.md) requires: one `(String, *const u8)`
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
//! A refcounted representation's null pointer means Novis's `null`, and every
//! retain/release primitive treats it as a no-op — [`nvs_object_retain`],
//! [`nvs_object_release`], and [`crate::nvs_str_retain`]/
//! [`crate::nvs_str_release`] alike.
//!
//! This is not a defensive check. It is the state a field slot is *in* between
//! [`NvsObj::new`] zeroing it and the constructor's first assignment — unless
//! the property declared a default, which [`NvsObj::new`] writes over the zero
//! before anything else runs ([`ClassDesc::defaults`]). Either way that first
//! assignment releases whatever the slot previously held
//! (`nvs_ir::lower::lower_reassignment`), and on the first write there is
//! nothing there. Compiled code reads the payload half of the slot without
//! consulting its tag — the field's static type already settled what it holds
//! — so what reaches the primitive is a null pointer, not a `Tag::Null`
//! [`Value`].
//!
//! The same rule is what a nullable `?T` will lower to, so paying one
//! perfectly-predicted branch per refcount operation buys both cases at once.
//! The alternative — teaching lowering which assignment is a property's
//! *first* — needs `nvs_types::ctor_init`'s flow analysis threaded into the
//! IR, to remove a branch that costs nothing measurable.
//!
//! # What a shape write checks
//!
//! `rule:types/erased-member-access` requires a
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
//!   alone** (`$shape{x,y}`, `nvs_ir::lower::shape_class_label`), so `{x: 1}`
//!   and `{x: "s"}` are one class. A declared *type* per slot would have to
//!   mint a class per name-and-type tuple — a bigger class table, a second
//!   naming scheme for `nvs_stdlib` to keep in step with, and all of it read
//!   by one instruction.
//! * A tag closes the failure that matters most: representation confusion,
//!   where a slot's payload is loaded as the wrong machine type. That is a
//!   priority 1 and 2 question ([AGENTS.md](/AGENTS.md)); what is left
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
//! 4. **Two records of one shape that share their field names but not their
//!    types** fall back to case 3 for the slots they disagree on;
//!    `nvs_ir::lower::Lowering::record_shape_class` degrades the tag rather
//!    than picking whichever record it saw first, and `nvs_ir::lower::lower_file`
//!    merges the same way across frames. Two literals are one such pair;
//!    `Core\Task::all`'s argument and its result are the other, the result's
//!    own representations being recorded at the call site
//!    (`nvs_ir::lower::Lowering::record_core_result_shape`).
//! 5. **A class with no layout of its own** — a closure's environment, a
//!    generator's state — carries no tags, because nothing declares its slots
//!    in source for a type to come from. A *named* class does carry them: an
//!    erased receiver reaches any class at all, so `nvs_ir::lower`'s
//!    `field_slots` joins every layout's slots against the declared property
//!    types the checker recorded.
//!
//! # Decision: every object is on its context's live list
//!
//! Refcounting alone frees only what the counts say is dead, and a cycle's
//! members hold each other above zero. An object is the one shape that can
//! close one — a string is immutable and an array copies on write
//! ([`crate::graph`]'s identity decision) — so every object links into its
//! context's [`LiveList`] in [`NvsObj::alloc`] and out again in [`dismantle`],
//! and whatever the root drain leaves on that list at teardown is exactly the
//! cyclic garbage. [`sweep`] dismantles it through the same worklist, so
//! native teardown runs there too rather than the memory being abandoned.
//! `rule:security/isolate-teardown-is-a-drain-then-a-sweep`
//! is the decision; this module is the mechanism.
//!
//! What it spends, as [AGENTS.md](/AGENTS.md) requires: **two pointers
//! per live object** — 16 bytes, charged to the request that allocated it —
//! plus a thread-local read and three non-atomic stores at each object's
//! birth, and two more at its death. Nothing on the read path pays, and no
//! decrement pays: [`unlink`] reaches its neighbours through the object's own
//! links and never through the context.
//!
//! **The crossing relinks, and the relink is structural.**
//! [`crate::graph`]'s `Live` carrier is the one place in the runtime an
//! allocation changes owners — its adopt-at-refcount-1 move is what makes a
//! crossing a pointer handoff rather than a rebuild (`rule:security/isolate-values-cross-by-copy`) — so
//! [`relink_to_current`] is called from inside that one implementation and from
//! no call site, because then there is no call site to get it wrong. An object
//! left on the source list is one the source's teardown sweep may take apart
//! while the destination still holds it, which is a use-after-free at
//! [AGENTS.md](/AGENTS.md)'s priority 1. The destination is the context
//! *running* at the crossing, which is the receiving one on the way out of an
//! isolate — `nvs_host`'s `finish` copies on the child's stack while the parent
//! is current. On the way **in** there is no destination context yet, so an
//! adopted argument stays on the parent's list; that is safe for the one reason
//! the direction is asymmetric at all, that a parent outlives the child it
//! spawned.
//!
//! **A debug build makes any future drift loud.** Every header carries the list
//! it was last linked into ([`ObjHeader::owner`], `debug_assertions` only, one
//! word), and both [`dismantle`] and [`sweep`] assert that an object is linked
//! on the list it names. A second place that moves an object between lists —
//! which is the only way the relink can be got wrong, since
//! [`LiveList::link`] writes the stamp and the links together — therefore
//! panics naming the invariant in every `cargo test` run and every WSL valgrind
//! leg, both of which are debug builds, rather than corrupting a parent's heap
//! wherever the block was reused. [`assert_linked_where_it_says`] is the home
//! of what that does and does not cover.
//!
//! **The shape teardown does not reach is collected in flight instead**: a
//! long-running script that builds cycles *between* teardowns has them
//! reclaimed by [`collect`], from the poll at which its allocations crossed
//! `rule:errors/on-limit`'s memory ceiling. Two bounds come with that door and
//! neither is a gap. A request under **no** ceiling arms no threshold, so
//! nothing ever asks it to collect and its cycles wait for teardown as before;
//! and a request *refused* an allocation is over its ceiling by
//! [`crate::budget`]'s sticky verdict, which a collection afterwards does not
//! take back, since the bytes were never handed over for it to give back.
//!
//! # Decision: an immortal instance is on no list at all
//!
//! `rule:core-classes/html-literal` folds a hole-free `` html`…` `` into a
//! `Core\Html\Markup` the compiled unit carries in its own data section,
//! exactly as [`crate::string`]'s § *An immortal string* already carries a
//! string literal. [`immortal_object_bytes`] is the header such a constant is
//! written with, and its reference count is [`crate::IMMORTAL_REFCOUNT`]:
//! [`bump`] and [`drop_one`] compare against it first and return without
//! writing.
//!
//! That compare is not an optimization either path may skip. A compiled unit
//! is the one thing requests share (`docs/adr/README.md`'s project-start
//! decisions), so an immortal header is reachable from every core at once, and
//! what keeps the [`Cell`]s sound is the narrower sentence [`crate::string`]
//! makes one representation over: **no reference count two threads can reach
//! is ever written**. No other word of such a header is written either. The
//! `class` word is fixed while compiling — which is why a `Core` class's
//! descriptor is one address for the process rather than one per core, and
//! `nvs_stdlib::instance` is that decision's home. `next` and `prev` stay null
//! because the constant never enters a [`LiveList`], so no teardown sweep
//! reaches it and [`dismantle`] never runs on it. And a slot is only ever
//! read: the one emitter is the literal fold, whose carrier has no member that
//! writes one, so an immortal instance cannot come to hold — or to close — a
//! cycle.
//!
//! **What it spends**, as [AGENTS.md](/AGENTS.md) requires: the header and its
//! slots once per literal, charged to the compiled unit rather than to a
//! request and freed with it, plus one compare and a not-taken branch on each
//! retain and release. That is the memory rule's bound met the way it asks —
//! bounded, attributable, and growing with neither traffic nor cores.

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
    /// The class's rendered name, as `nvs_ir` labels it (`Class` or
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
    /// `rule:types/erased-member-access`'s
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
    /// every inherited one — as a [`MethodRow`], sorted by name so
    /// [`ClassDesc::method`] is a binary search. Empty until
    /// [`ClassTable::set_methods`] fills it, which `nvs-codegen` does after
    /// the unit is finalized: see this module's docs for why a name and not a
    /// slot index.
    methods: Vec<MethodRow>,
    /// Every property hook an instance of this class answers — its own plus
    /// every inherited one — as a [`HookRow`], sorted by property name and
    /// then by accessor so [`ClassDesc::hook_row`] is a binary search. Empty
    /// until [`ClassTable::set_hooks`] fills it, on
    /// [`ClassTable::set_methods`]' terms exactly.
    ///
    /// Beside the method table rather than in it: `rule:classes/property-hooks`
    /// makes a hook an accessor of a property and not a method, so it is
    /// unreachable by `Class::name()` and invisible to
    /// `Core\Reflect\ClassInfo::methods`. What reaches it is an access through
    /// an *erased* receiver, where the site that wrote `$obj->prop` had no
    /// class to resolve the hook against — the same read
    /// [`ClassDesc::field_slot`] answers, which is why the roster rides here.
    hooks: Vec<HookRow>,
    /// `rule:core-classes/derive-attribute`'s derived JSON
    /// field list, in declaration order — empty for every class not carrying
    /// `#[Json\Derive]`, which is the default and costs one empty `Vec` per
    /// descriptor.
    ///
    /// Compiled in rather than reflected: `nvs_types::derive` reads the
    /// attribute, `nvs-codegen` copies the answer here, and
    /// `nvs_stdlib::json`'s encoder and decoder walk it. Filled by
    /// [`ClassTable::set_codec`].
    codec: Vec<CodecField>,
    /// One entry per [`Self::codec`] field: the descriptor a
    /// [`CodecTy::Class`] field decodes into, and a null pointer for every
    /// other wire type.
    ///
    /// A parallel vector rather than a pointer inside [`CodecField`] because
    /// that struct crosses the front-end crates, which have no descriptor to
    /// put there — the label they *can* write is [`CodecField::class`], and
    /// this is the resolved half, filled by the same
    /// [`ClassTable::set_codec`] call and on exactly [`Self::conforms`]'
    /// terms.
    codec_classes: Vec<*const ClassDesc>,
    /// One entry per [`Self::codec`] field: the contract a [`CodecTy::Shape`]
    /// field decodes against, and a null pointer for every other wire type.
    ///
    /// [`Self::codec_classes`]' terms exactly, for the half a class label
    /// cannot carry. There is no row twin: `rule:core-classes/db-column-types`
    /// maps no column to an object, so `nvs_types::derive` refuses an inline
    /// shape at the declaration of a `#[Db\Derive]` field.
    codec_shapes: Vec<*const ShapeCodec>,
    /// `rule:core-classes/derive-attribute`'s derived **row**
    /// field list — [`Self::codec`]'s twin for `#[Db\Derive]`, and empty for
    /// every class not carrying it.
    ///
    /// A second list rather than a second reading of the first, because the
    /// two formats are only usually the same: `#[Json\Field(name: "userId")]`
    /// and `#[Db\Field(name: "user_id")]` are both legal on one property, and
    /// § 3's `skip` is per format too. A class carrying one attribute pays one
    /// empty `Vec` for the other, which is the footprint `rule:programs/memory-priority`'s ordering
    /// spends to keep the two mappings from having to agree.
    db_codec: Vec<CodecField>,
    /// One entry per [`Self::db_codec`] field, on [`Self::codec_classes`]'
    /// exact terms and filled by [`ClassTable::set_db_codec`].
    db_codec_classes: Vec<*const ClassDesc>,
    /// How many parameters this class's `constructor` declares — what a
    /// derived *decoder* has to fill before it can run one, and zero for
    /// every class with no codec.
    ///
    /// Carried beside [`Self::codec`] rather than derived from it because a
    /// skipped field (`rule:core-classes/derive-field-list`) leaves a parameter no field names, and a decoder that silently
    /// shortened its argument list would call the constructor with the wrong
    /// arity.
    ctor_arity: usize,
    /// Each field slot this class arms at construction, as `(slot, recipe)`
    /// in slot order — empty for a class that arms none, which is most of
    /// them. Filled by [`ClassTable::set_defaults`].
    ///
    /// Two kinds of entry, and [`FieldDefault::Unset`] says why they share
    /// one list: a declared `= expr` default, and the never-written marker
    /// `rule:classes/an-unwritten-property-read-throws` owes a `lateinit` slot.
    ///
    /// This is the whole of what a property initializer *is* at run time:
    /// [`NvsObj::new`] writes these slots straight after nulling them, so a
    /// default reaches an instance however it was built — a compiled `new`,
    /// [`construct`] from native code, or `rule:core-classes/derive-attribute`'s derived decoder. Compiled
    /// code emits no initializer at all; `nvs_types::defaults` owns why.
    defaults: Vec<(usize, FieldDefault)>,
    /// The one [`Tag`] each field slot's *declared* type admits, in slot
    /// order, or `None` for a slot whose declared type admits more than one —
    /// a union, a `?T`, a `mixed`. **Empty** for a class nothing has told —
    /// one the compiler synthesized rather than laid out from a declaration,
    /// a closure's environment or a generator's state; an empty list means
    /// "unknown", never "no field admits anything". Every class an
    /// `rule:types/erased-member-access`
    /// write can name from source carries one entry per slot.
    ///
    /// This is the whole of § 4's *"a write's incoming value is checked
    /// against the field's real, concrete declared type"* — see
    /// [`nvs_object_slot_set`], which is its only reader, and this module's
    /// docs § *What a shape write checks* for what a tag does not catch and
    /// why the declared type itself is not here. Filled by
    /// [`ClassTable::set_field_tags`]. **Cost:** one byte-sized `Option<Tag>`
    /// per field per class, once per process, not per instance.
    field_tags: Vec<Option<Tag>>,
    /// Whether each field slot's *declared* type carries `rule:security/secret-qualifier`'s
    /// `secret` qualifier, in slot order. **Empty** for a class nothing has
    /// told, on [`Self::field_tags`]' own terms, and an empty list reads as
    /// "no slot is `secret`" rather than as "unknown" — see
    /// [`ClassDesc::field_is_secret`] for why that direction is the safe one.
    ///
    /// This is the property half of
    /// `rule:errors/record-transformations`'s redaction row, and it is carried rather than computed for the
    /// reason nothing below the checker could compute it: `secret` is a
    /// qualifier on a declared type, and a `secret string` is byte-identical
    /// to a `string` in every representation under it. `nvs_types` decides it,
    /// `nvs_ir::ir::Class::secret_fields` carries it, and
    /// `nvs_stdlib::debug`'s walk is its one reader. Filled by
    /// [`ClassTable::set_secret_fields`]. **Cost:** one `bool` per field per
    /// class, once per process, not per instance.
    secret_fields: Vec<bool>,
    /// Whether each field slot is readable from outside this class, in slot
    /// order. **Empty** for a class nothing has told, and an empty list reads
    /// as "no slot is readable" rather than as "unknown" — see
    /// [`ClassDesc::field_is_public`] for why that direction is the safe one.
    ///
    /// This is the property half of
    /// `rule:security/reflection-enforces-visibility`
    /// — a reflective read faces the check ordinary code at that site
    /// faces — and it is carried rather than computed for the reason nothing
    /// below the checker could compute it: visibility is a keyword on a
    /// declaration, and a `private int $n` is byte-identical to a `public int
    /// $n` in every representation under it. `nvs_types::layout` decides it,
    /// `nvs_ir::ir::Class::public_fields` carries it, and
    /// `nvs_stdlib::reflect`'s walk is its one reader. Filled by
    /// [`ClassTable::set_public_fields`]. **Cost:** one `bool` per field per
    /// class, once per process, not per instance.
    public_fields: Vec<bool>,
    /// Whether each field slot is declared `protected`, in slot order.
    /// **Empty** for a class nothing has told, which reads as "no slot is
    /// `protected`" on [`Self::public_fields`]' terms exactly: both bits false
    /// is `private`, and refusing is the safe direction for a question
    /// `rule:security/reflection-enforces-visibility`
    /// makes a privilege check.
    ///
    /// Beside the readable bit rather than folded into it, because the rule is
    /// stated over the *level*: a `protected` member is reached from every
    /// class in the hierarchy that declares it, where a `private` one answers
    /// to the declaring class alone, and one bit cannot tell those apart.
    /// `nvs_types::layout` decides it, `nvs_ir::ir::Class::protected_fields`
    /// carries it, [`ClassTable::set_protected_fields`] fills it and
    /// [`crate::Ctx::field_is_visible_from`] is its one reader. **Cost:** one
    /// `bool` per field per class, once per process, not per instance.
    protected_fields: Vec<bool>,
    /// Each field slot's declared type, spelled as the declaration spells it —
    /// empty for a class no declaration laid out, on [`Self::public_fields`]'
    /// terms exactly.
    ///
    /// The type fact [`Self::field_tags`] cannot carry, and the reason both
    /// exist: a tag is what compiled code checks a store against, so `?int`,
    /// `int` and a literal `7` are one tag and a class, an array and a shape
    /// are one more — while `Core\Reflect\PropertyInfo` reports what the
    /// program *wrote*. `nvs_types::layout` spells it, `nvs_ir::ir::Class`
    /// carries it, and [`ClassTable::set_field_types`] fills it. **Cost:** one
    /// `String` per field per class, once per process, not per instance.
    field_types: Vec<String>,
    /// Every class constant a program can name on this class, flattened over
    /// its ancestors where `nvs_types::layout` flattened it — empty for a class
    /// no declaration laid out, on [`Self::public_fields`]' terms exactly.
    ///
    /// Beside the field rosters rather than among them, because a constant
    /// claims no slot: it belongs to the class and never to an instance, which
    /// is why nothing here is indexed against [`Self::fields`] and why the
    /// roster is keyed by name alone. `nvs_types::layout` folds it,
    /// `nvs_ir::ir::Class::constants` carries it and
    /// [`ClassTable::set_class_constants`] fills it;
    /// `Core\Reflect\ClassInfo::constants` is what reads it back.
    ///
    /// **Cost:** one [`ConstantDesc`] per constant per class, once per process,
    /// not per instance and not per request.
    constants: Vec<ConstantDesc>,
    /// Every `#[...]` written on this class's own declaration or on one of its
    /// own members, in source order — empty for a class no declaration laid
    /// out, on [`Self::public_fields`]' terms exactly.
    ///
    /// Beside the constants for their reason: an attach site claims no slot
    /// either. Own-only where that roster is flattened, which
    /// `nvs_types::layout::ClassLayout::attributes` owns:
    /// an attribute is a fact about where it was written, so a base class's
    /// site is never this class's. `nvs_ir::ir::Class::attributes` carries it,
    /// [`ClassTable::set_class_attributes`] fills it, and
    /// `Core\Reflect\ClassInfo::attributes` is what reads it back.
    ///
    /// **Cost:** one [`AttributeDesc`] per attach site per class, plus one
    /// [`ConstantValue`] per payload field, once per process, not per instance
    /// and not per request.
    attributes: Vec<AttributeDesc>,
    /// The address of the **native** function that renders an instance of this
    /// class as a `string`, or null for every class that has none — which is
    /// every class a program declares, and every `Core` class the spec gives
    /// no `toString`.
    ///
    /// Not a [`Self::methods`] row, and the difference is the calling
    /// convention rather than the lookup: that table holds *compiled* Novis
    /// functions, which release their parameters, while a native `Core` member
    /// is an `rule:errors/propagation` helper and **borrows** its arguments. One table cannot
    /// hold both without a caller having to know which it drew — so the
    /// convention is encoded in which field the address came out of. Filled by
    /// [`ClassTable::set_render`], which only `nvs-stdlib` calls; read by
    /// [`crate::dispatch::call_render`], which [`crate::stringify`] asks
    /// before the method table. **Cost:** one pointer per class, once per
    /// process, not per instance.
    render: *const u8,
    /// The address of the **native** function that orders two instances of
    /// this class — `rule:classes/comparable`'s `compareTo` — or null for
    /// every class that carries none, which is every class a program declares
    /// and every `Core` class the spec gives no such member.
    ///
    /// [`Self::render`]'s field, one member along, and for its reason exactly:
    /// the address is a native helper that **borrows** both operands, while a
    /// [`Self::methods`] row is a compiled function that releases its
    /// parameters, so which field the address came out of is what tells a
    /// caller the convention to call it under. A program's own `Comparable`
    /// class is on the method table and answers through
    /// [`crate::dispatch::call_method`]; a `Core` one is here. Filled by
    /// [`ClassTable::set_compare`], which only `nvs-stdlib` calls; read by
    /// [`crate::dispatch::call_compare_to`], which asks it before the method
    /// table. **Cost:** one pointer per class, once per process, not per
    /// instance.
    compare: *const u8,
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
    /// Whether an instance of this class is a
    /// `rule:types/callable-is-a-closure` closure — its
    /// [`crate::closure::CLOSURE_INVOKE`] the compiled body of a closure
    /// literal and its fields that literal's captures — rather than an object
    /// of a class a program declared.
    ///
    /// Carried rather than asked of the method table, because the structural
    /// test is "does this class declare an `invoke`" and a program is free to
    /// declare one: that name decides whether
    /// [`crate::closure::call_closure`] jumps into a value's code at all, and
    /// whether `rule:classes/graph-copy`'s walk refuses the value as a
    /// closure, so a user class spelling it would be both called through and
    /// refused. `nvs_ir::lower` mints a closure's environment class,
    /// `nvs_ir::ir::Class::is_closure` carries the bit down and `nvs-codegen`
    /// hands it to [`ClassTable::set_closure`] — which is also what native
    /// code building a closure for a `Core` member to call answers. **Cost:**
    /// one `bool` per class, once per process, not per instance.
    is_closure: bool,
    /// Whether an instance of this class holds a **host handle** — a key into
    /// one [`Ctx`]'s own table of the open files, sockets, readers, database
    /// connections and children that request opened, which is what
    /// `rule:security/isolate-values-cross-by-copy` refuses at a copy boundary.
    ///
    /// Carried for [`Self::is_closure`]'s reason and one of its own: a key is a
    /// `uint` slot like every other, so nothing about the value says it
    /// addresses a table, and the table it addresses belongs to the side that
    /// opened it. `nvs_stdlib::instance` writes it through
    /// [`ClassTable::set_host_handle`] for every `Core` class whose slot
    /// carries one, and it is `false` for every class a program declares:
    /// `rule:types/grammar`'s "there is no `resource` type" leaves a handle
    /// reachable only as one of those `Core` instances. **Cost:** one `bool`
    /// per class, once per process, not per instance.
    holds_host_handle: bool,
}

/// One row of a [`ClassDesc`]'s method table: a name, the compiled address it
/// answers with, and the **declared shape** a caller holding only tagged
/// values needs in order to fill that callee's slots safely.
///
/// Nothing has to be marshalled between a tagged site and a compiled callee —
/// `rule:errors/propagation` makes one calling convention normative, so `nvs-codegen` already
/// writes every argument and every return *with* its tag and a typed callee
/// reads only the payload half. What is missing at a site that knows no class
/// is the callee's own shape, without which slot *i* is reinterpreted at the
/// callee's representation and an `int` handed to a `string` parameter is an
/// arbitrary dereference rather than a fault — the identical hole
/// [`crate::closure`]'s module docs describe for `callable`, arrived at from
/// the other side. So the row carries what a closure object already carries in
/// `nvs_ir::lower`'s `FN_ARITY` and `FN_PARAM_TAGS` slots, in the same
/// encoding, and [`crate::closure`]'s `check_param_tags` is the one
/// implementation both paths share rather than a second copy of `rule:types/conversion`'s
/// `int`-into-`float` widening. `docs/adr/README.md` § *Decisions taken at
/// project start* owns why this rides on the descriptor rather than on a
/// per-method thunk.
///
/// **Cost:** the two words, the `bool`s and the two roster headers are 64
/// bytes per method per class, plus one `String` for each parameter a source
/// declaration named and one for each type it wrote — once per process and not
/// per instance, and still short of what the thunk this replaces would have
/// spent a compiled function each on.
#[derive(Clone, Debug)]
pub struct MethodRow {
    /// The method name, as written. The table is sorted on this.
    pub name: String,
    /// The compiled function's address — what [`ClassDesc::method`] answers
    /// and what every caller through this table jumps to.
    pub code: *const u8,
    /// How many parameters the callee declares, **not** counting the implicit
    /// receiver in slot 0: the count a site that wrote the argument list is
    /// judged against, exactly as a closure's `FN_ARITY` is.
    pub arity: u32,
    /// Which runtime [`Tag`] each declared parameter requires, one nibble per
    /// parameter and the receiver excluded, parameter 0 in the least
    /// significant nibble — `nvs_ir::lower`'s `FN_PARAM_TAGS` encoding
    /// verbatim, `nvs_ir::lower::FN_PARAM_TAG_ANY` for a parameter whose
    /// representation is itself a tag. Sixteen parameters fit; a callee
    /// declaring more is refused rather than passed an argument nothing
    /// checked, which is [`crate::closure`]'s own rule.
    pub param_tags: u64,
    /// What each declared parameter is *called*, in the same order and with
    /// the receiver excluded the same way — the `$` sigil not included.
    ///
    /// Either [`Self::arity`] names long or **empty**, on
    /// `nvs_types::layout::ClassLayout::methods`' terms exactly, where empty
    /// reads as "no declaration was read for this row" rather than "the method
    /// takes nothing": a synthesized member and a native row are the rows
    /// nothing spelled. It is carried because it is the one parameter fact
    /// with no source below the front end — [`Self::arity`] counts parameters
    /// and [`Self::param_tags`] types them, and neither can name one — and
    /// `Core\Reflect\MethodInfo::parameters` is what asks for it.
    pub param_names: Vec<String>,
    /// What each declared parameter's type is *spelled* as, in
    /// [`Self::param_names`]' order and on its terms exactly: either
    /// [`Self::arity`] names long or **empty**, the receiver excluded, and the
    /// **empty string** for a parameter that wrote no type at all, which on a
    /// method is source the front end has already refused.
    /// The text is the declaration's own, whitespace collapsed — `?int`,
    /// `array<User>`, `uint` — which is
    /// `nvs_types::layout::ClassLayout::field_types`' currency for a property.
    ///
    /// Beside [`Self::param_tags`] rather than derived from it, because a tag
    /// is a representation and a declaration is a type: one nibble cannot tell
    /// `int` from `uint`, cannot name an enum and cannot separate two classes.
    /// `Core\Command`'s help page is what asks — a page that knows a command's
    /// handler is `Class::method` reaches this table for the row and has no
    /// other source for the word a flag's type is written with
    /// (`rule:tooling/commands-are-compiled`).
    ///
    /// **Cost:** one `String` per declared parameter per method per class, once
    /// per process and never per instance — the same figure
    /// [`Self::param_names`] beside it spends, and a row nothing spelled holds
    /// an empty vector's header alone.
    pub param_types: Vec<String>,
    /// Whether the member is `public` — the one visibility question a receiver
    /// that names no class can ask, since such a site is outside every class
    /// by construction. Answered at the declaration by `nvs_types::layout` and
    /// carried down; a row nothing told is public, which is what keeps a
    /// synthesized method (an exception constructor, a generator's state
    /// machine, an `rule:classes/delegation-by-field` forward) callable.
    pub public: bool,
    /// Whether the member is `protected` — [`Self::public`]'s neighbour, and
    /// the bit that makes the level readable rather than just the
    /// readable/not pair: a `protected` method is called from every class in
    /// the hierarchy that declares it, where a `private` one answers to the
    /// declaring class alone. Both bits false is `private`, and a row nothing
    /// told is `public`, so a synthesized method stays callable on
    /// [`Self::public`]'s terms exactly.
    /// [`crate::Ctx::method_is_visible_from`] is what reads the pair.
    pub protected: bool,
    /// Whether [`Self::code`] is a **native** `rule:errors/propagation` helper rather than a
    /// compiled Novis function — true for exactly the `Core`-owned members
    /// `nvs_stdlib::instance` puts in this table.
    ///
    /// The two differ in ownership, which is the same difference that made
    /// [`ClassDesc::render`] its own field rather than a row here: a native
    /// member *borrows* argument 0 where a compiled method owns its
    /// parameters, so a caller that transferred a reference would leave the
    /// receiver leaked and one that did not would free it twice. Neither
    /// [`Self::arity`] nor [`Self::param_tags`] describes such a row at all —
    /// this crate is handed an address and no signature — so the bit is what a
    /// caller reaching this table without a class in hand refuses on, rather
    /// than a shape it could believe.
    pub native: bool,
}

/// One property hook's compiled address and the shape a caller holding only
/// tagged values has to check the incoming value against — [`MethodRow`]'s
/// twin for `rule:classes/property-hooks`' accessors.
///
/// Its own row type rather than a [`MethodRow`] under a mangled name, because
/// the two are looked up by different keys and answer different questions: a
/// method is found by the name a call site wrote, while a hook is found by the
/// *property* an access named plus which of the two accessors that access is.
/// A name that carried both would have to be parsed back apart at every
/// lookup, and `nvs_types::signatures::hook_label`'s spelling is a compile-time
/// fact this crate has no business re-reading.
///
/// **Cost:** the string, the pointer and the two flags per hook per class,
/// once per process and not per instance — and nothing at all for a class
/// declaring no hook, which is most of them.
#[derive(Clone, Debug)]
pub struct HookRow {
    /// The hooked property's name, without the `$` sigil — the name an access
    /// through an erased receiver wrote, and what this table is sorted on
    /// first.
    pub property: String,
    /// Whether this row is the property's `set` hook; `false` is its `get`.
    /// The second half of the key, and what the table is sorted on after
    /// [`Self::property`].
    pub set: bool,
    /// The compiled hook body's address — an ordinary Novis function taking
    /// the receiver in parameter slot 0, exactly as a method does
    /// (`nvs_ir::lower::lower_property_hook`).
    pub code: *const u8,
    /// Which runtime [`Tag`] the incoming value must have, in
    /// [`MethodRow::param_tags`]' encoding verbatim — parameter 0 in the least
    /// significant nibble, which for a `set` hook is the value being stored.
    ///
    /// No arity rides beside it, unlike a method's, because the language fixes
    /// one: `get` declares no parameter and `set` declares exactly the value
    /// (`rule:classes/property-hooks`), so a caller knows how many arguments it
    /// is passing before it reads the row.
    pub param_tags: u64,
}

/// The name of the resume-to-unwind entry point on a generator's state class,
/// which is the whole of what [`dismantle`] knows about
/// `nvs_ir::lower::generator`'s transform — that module's `GEN_UNWIND_METHOD`
/// is the same string, and its doc comment owns why the name is unspellable.
///
/// Restated here rather than shared: this crate depends on neither `nvs-ir`
/// nor `nvs-codegen`, and the class label and field names of that transform
/// are restated in the same direction for the same reason.
pub const GENERATOR_UNWIND_METHOD: &str = "gen#unwind";

/// One property default's already-evaluated value — the closed set
/// `nvs_types::defaults::ConstArg` can reach from a *written* property
/// declaration, which is that enum minus the shapes only `Core`'s own
/// signature table produces.
///
/// Deliberately a recipe rather than a ready-made [`Value`]: a `Value` holding
/// a string would make the descriptor a refcount owner, and every instance
/// would then have to be careful to retain it — an invariant paid for on every
/// allocation, in exchange for saving one `strlen`-sized copy on a class that
/// declares a string default. AGENTS.md's ordering puts that the other way
/// round. **Cost:** one [`crate::NvsStr`] allocation per instance per
/// string-defaulted property, and one empty [`crate::NvsArray`] per instance
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
    /// [`crate::NvsStr`] holds.
    Str(String),
    /// `[]` — a fresh empty array, which is the only array constant there is
    /// (`nvs_types::defaults::ConstArg::EmptyArray`).
    EmptyArray,
    /// **Not a default at all**: the "never written" marker
    /// `rule:classes/an-unwritten-property-read-throws` owes a slot no constructor is obliged to fill — today a `lateinit`
    /// property's (`rule:classes/lateinit`), which is the one declaration `rule:classes/definite-property-initialization`
    /// exempts.
    ///
    /// It rides in this list rather than in a second one beside it because it
    /// is the same pass: [`NvsObj::new`] already walks one `(slot, recipe)`
    /// list and stores one value per entry, and a class that declares neither
    /// a default nor a `lateinit` property still walks an empty list. What it
    /// materializes is [`Value::unset`], which owns nothing.
    Unset,
}

impl FieldDefault {
    /// A fresh [`Value`] for this default, owning one reference to whatever it
    /// allocated.
    ///
    /// Reachable outside this crate because a decoder is the second reader:
    /// [`CodecField::default`] is this same recipe carried to a door with no
    /// call site to emit the constant at, and `nvs_stdlib::json` materializes
    /// it there.
    #[must_use]
    pub fn materialize(&self) -> Value {
        match self {
            Self::Bool(v) => Value::bool(*v),
            Self::Int(v) => Value::int(*v),
            Self::Uint(v) => Value::uint(*v),
            Self::Float(v) => Value::float(*v),
            Self::Str(s) => Value::str(crate::NvsStr::new(s.as_bytes())),
            Self::EmptyArray => Value::array(crate::NvsArray::new()),
            Self::Unset => Value::unset(),
        }
    }
}

/// What one [`CodecField`] decodes to: the closed set of runtime
/// representations `rule:core-classes/derive-field-list`'s
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
    /// `decimal` — `rule:types/decimal`'s exact scalar, and never [`Self::Float`]:
    /// the two are not assignable to each other in the language, and a decoder
    /// that reached one through the other would round away the digits the type
    /// exists to keep.
    Decimal,
    /// `string`.
    Str,
    /// `bytes` — `rule:types/bytes`'s octet string. A column type and not a JSON
    /// one, which is why this is a wire type of its own rather than
    /// [`Self::Str`]: `rule:core-classes/db-column-types` maps the binary
    /// families to it, and `nvs_types::derive`'s JSON half never produces it.
    Bytes,
    /// `Core\Time\Instant` — a `Core` value type carried as a value rather than
    /// as a nested object, so it is not [`Self::Class`]: it declares no codec
    /// for a decoder to run. A driver builds the instance out of the column's
    /// own components before hydration reads it, and what a decode owes is the
    /// question every other wire type owes — is this what the field declared.
    Instant,
    /// `mixed` — whatever the document held, unchecked
    /// (`rule:types/declaration`).
    Mixed,
    /// Another class that carries a codec of its own — `rule:core-classes/derive-field-list`'s "another
    /// class that itself has a codec", decoded by running that class's own
    /// field list over the nested JSON object.
    ///
    /// *Which* class is not in this enum, because a wire type is `Copy` and a
    /// class identity is a pointer somebody has to resolve: the label rides on
    /// [`CodecField::class`] and the descriptor it resolves to on
    /// [`ClassDesc::codec_class`].
    Class,
    /// An `array<T>` of one of the wire types above — `rule:core-classes/derive-field-list`'s list
    /// field, decoded by running the *element's* wire type once per position.
    ///
    /// The element rides on [`CodecField::element`] rather than inside this
    /// variant for the same reason a class identity does not: this enum is
    /// `Copy` and a recursive variant is not. What nests instead is
    /// [`CodecElement`], so an `array<array<T>>` describes every level of
    /// itself and a list element is itself a `List` as often as the
    /// declaration says.
    List,
    /// An enum — `rule:core-classes/derive-field-list`'s enum field, decoded as a membership test
    /// rather than as a construction: `rule:enums/representation` reserves an enum tag that
    /// nothing writes, so a case at run time *is* the integer behind it (see
    /// [`crate::value_truthy`]'s own note), and what a decode produces is that
    /// integer in the enum's backing type.
    ///
    /// The roster of accepted values rides on [`CodecField::cases`] for
    /// [`Self::Class`]'s reason: this enum is `Copy` and a case list is not.
    Enum,
    /// An inline shape reached as a field — `rule:types/shape-type`'s
    /// `{name: T}` written as a property's declared type, decoded by running
    /// that shape's own contract over the nested JSON object.
    ///
    /// **Two resolved pointers, where every other wire type needs at most
    /// one**, which is why this is not [`Self::Class`]: a shape class is keyed
    /// on its field *names* alone (`nvs_ir::lower::shape_class_label`), so the
    /// descriptor a decode constructs rides on [`CodecField::class`] exactly as
    /// a nested class's does, and the per-field wire types the class cannot
    /// carry ride beside it on [`CodecField::shape`], as the key of the
    /// [`ShapeCodec`] a `decodeAs<{n: int}>` call site is handed.
    Shape,
    /// A declared type no wire type describes — a `callable`, an `object`, a
    /// union of two non-`null` arms.
    ///
    /// `rule:core-classes/derive-field-list`'s reachable test refuses such a
    /// field at the declaration that wrote it, so this is what the erasure
    /// answers *while* that refusal is still being collected rather than a
    /// decoder's missing case: nothing the reachable set admits erases here,
    /// every depth of `array<…>` and every inline shape included. A decoder
    /// that meets one faults naming the field rather than guessing a value.
    Opaque,
}

/// The closed set of backing values a [`CodecTy::Enum`] accepts, and which of
/// `rule:enums/one-backing-type`'s two integer types they are.
///
/// A decode is a membership test against this and nothing else. There is no
/// object to construct and no case name to look up, because a case is
/// indistinguishable from its backing integer by the time it is a [`Value`];
/// a document holding an integer no case declares is a bad document, reported
/// as one of `rule:core-classes/derive-reports-every-field`'s issues rather than as a fault.
#[derive(Clone, Debug)]
pub struct EnumCases {
    /// Whether the enum is `uint`-backed, which is the whole of what decides
    /// between a [`Value::int`] and a [`Value::uint`] on a hit.
    pub unsigned: bool,
    /// Every declared case's value, ascending, so a lookup is a binary search.
    ///
    /// Widened to `i128` so one field answers for both backings without a
    /// lossy cast — `nvs_ir::lower::convert` normalizes an enum's cases the
    /// same way, for the same reason.
    pub values: Vec<i128>,
}

/// One declared `enum`'s whole shape — its rendered name, which of
/// `rule:enums/one-backing-type`'s two integer types its cases are constants
/// of, and every case by name.
///
/// [`ClassDesc`]'s counterpart for the one type that has no descriptor:
/// `rule:enums/representation` makes an enum case *be* the integer behind it at
/// run time, so nothing about a running value can be asked what enum it came
/// from and there is no class to hang the answer on. `rule:enums/reflection` is
/// the one reader — `Core\Reflect\EnumInfo::of` reports a name and a closed
/// case list as ordinary structural metadata — and the shape is carried here
/// because the front end is the only place it exists: `nvs_types::enums`
/// resolves it, `nvs_ir::ir::Program::enums` carries it down, and
/// [`ClassTable::define_enum`] is what fills this.
///
/// It grants an enum nothing: no dispatch, no identity, nothing that acts on a
/// value. Everything here is what the declaration already published to the
/// checker.
///
/// **Cost:** one `String` per case plus the value, per declared enum, once per
/// process and never per instance — an enum has none.
#[derive(Clone, Debug)]
pub struct EnumDesc {
    /// The enum's rendered name, spelled as `Name::class` renders it and as
    /// `nvs_ir` labels a class — `Enum` or `Ns\Enum`.
    name: String,
    /// Whether the backing type is `uint`, which is the whole of what decides
    /// whether a case's value reads back as a [`Value::int`] or a
    /// [`Value::uint`].
    unsigned: bool,
    /// Every declared case as `(name, value)`, ascending by value and then by
    /// name. Ordered rather than hashed because it is answered as a list: the
    /// declaration's own order is not carried by anything below the parser, and
    /// a total order the runtime can state is worth more to a caller than one
    /// it would have to sort itself.
    ///
    /// Widened to `i128` for [`EnumCases::values`]' reason exactly — one field
    /// answers for both backings with no lossy cast.
    cases: Vec<(String, i128)>,
}

impl EnumDesc {
    /// The enum's rendered name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the cases are constants of `uint` rather than of `int`.
    #[must_use]
    pub const fn unsigned(&self) -> bool {
        self.unsigned
    }

    /// Every declared case as `(name, value)`, in [`Self::cases`]' stated
    /// order.
    #[must_use]
    pub fn cases(&self) -> &[(String, i128)] {
        &self.cases
    }

    /// The value of the case named `case`, or `None` if this enum declares
    /// none.
    #[must_use]
    pub fn value_of(&self, case: &str) -> Option<i128> {
        self.cases
            .iter()
            .find(|(name, _)| name == case)
            .map(|(_, value)| *value)
    }
}

/// One class constant on a [`ClassDesc`] — everything
/// `Core\Reflect\ClassInfo`'s two constant members answer from, and nothing a
/// running program's `Foo::BAR` reads: an ordinary use of a constant is folded
/// at compile time and never reaches a descriptor.
///
/// It grants a class nothing on [`EnumDesc`]'s terms exactly. Every field is
/// what the declaration already published to the checker, and the value is the
/// one `nvs_types::consts`' fold made of it.
#[derive(Clone, Debug)]
pub struct ConstantDesc {
    /// The constant's name, as its declaration writes it.
    pub name: String,
    /// Whether code outside the declaring class may name it.
    pub public: bool,
    /// Whether it is declared `protected` — the second bit, on
    /// [`ClassDesc::protected_fields`]' terms, so a reflective read from a
    /// subclass's body reaches what an ordinary one there reaches.
    pub protected: bool,
    /// Whether the declaration's annotation carries `secret`, which is what
    /// makes a reflective read of the value refuse rather than launder:
    /// `nvs_types::layout::ClassConstant::secret` owns why the bit travels.
    pub secret: bool,
    /// The folded value, or [`ConstantValue::Opaque`] where the declaration's
    /// right-hand side is not one of the four literals that fold.
    pub value: ConstantValue,
}

/// A class constant's compile-time value, in the four shapes
/// `rule:types/constant-in-type-position`'s fold produces plus the absence of
/// one.
///
/// The runtime's own currency rather than `nvs_types::consts::ConstValue`,
/// which this crate cannot see: `nvs-codegen` converts once per declared
/// constant per compiled unit, and the shapes are one-for-one so the
/// conversion is total and can never pick a wrong case.
#[derive(Clone, Debug, PartialEq)]
pub enum ConstantValue {
    /// A `string` constant, already cooked — the bytes a `Value::str` holds.
    Str(String),
    /// An `int` constant, in `int`'s own range.
    Int(i64),
    /// A `bool` constant.
    Bool(bool),
    /// A `float` constant.
    Float(f64),
    /// Declared, and not one of the four above: an `array` or object constant,
    /// or an integer whose magnitude no `int` holds.
    ///
    /// **A stated bound, not a lost value.** Novis folds a constant that *is* a
    /// literal and runs no second constant-expression evaluator
    /// (`nvs_types::consts`), so a reflective read of one of these reports that
    /// it has no compile-time value rather than guessing at one — which is the
    /// same answer the checker gives a use of it in type position.
    Opaque,
}

/// One attached attribute on a [`ClassDesc`] — which declaration it is written
/// on, the name the named form gave it, and its payload folded field by field.
///
/// [`ConstantDesc`]'s shape one declaration out, and the visibility bits are
/// what it does not need: an attribute is written where the declaration is and
/// carries no modifier of its own, and a `secret` value cannot reach a payload
/// at all — `rule:attributes/payload-is-a-compile-time-constant` refuses one
/// where it is written, so there is no qualifier for a reflective read to
/// launder away. `nvs_types::layout::ClassAttribute` is the row this is
/// converted from, and owns the rest of why.
#[derive(Clone, Debug)]
pub struct AttributeDesc {
    /// The member the attribute is written on — the empty string for the class
    /// or interface declaration itself, a property's or method's name
    /// otherwise.
    pub member: String,
    /// The parameter of [`Self::member`] it is written on, or the empty string
    /// for every other attach site.
    pub parameter: String,
    /// The `type` alias the named form names, or the empty string for the bare
    /// form.
    pub name: String,
    /// The payload's fields in source order, each folded to the same currency
    /// a class constant travels in.
    ///
    /// [`ConstantValue::Opaque`] is a value's absence and not a field's, on
    /// [`ConstantDesc::value`]'s terms exactly: the field keeps its row, and
    /// `Core\Reflect\AttributeInfo::field` reports the bound rather than a name
    /// it does not know.
    pub fields: Vec<(String, ConstantValue)>,
}

/// What one position of a [`CodecTy::List`] field holds — the element's own
/// wire type, and, where that is another list, its element in turn.
///
/// The recursion is the whole point: an `array<array<Tag>>` is two of these
/// and a `Tag` at the bottom, so a decoder walks down the chain it was handed
/// instead of stopping one level in.
///
/// **The labels a wire type loses stay on the field**, not here. A chain of
/// lists bottoms out in exactly one node that is not a `List`, and that
/// terminal node is the only one that can name a class, a case roster or a
/// contract — so [`CodecField::class`], [`CodecField::cases`] and
/// [`CodecField::shape`] answer for it, and `nvs-codegen` resolves the two
/// pointers once per field at the depth it already resolves them at.
///
/// **What it spends**, per `rule:programs/memory-priority`: one allocation per
/// nesting level of a list field, in the compiled unit's own descriptor —
/// O(declarations in the program), never per request and never per document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodecElement {
    /// The element's wire type.
    pub ty: CodecTy,
    /// The element's own element, where [`Self::ty`] is [`CodecTy::List`], and
    /// `None` for every other one.
    pub element: Option<Box<CodecElement>>,
}

impl CodecElement {
    /// A leaf: an element that is not itself a list.
    #[must_use]
    pub const fn leaf(ty: CodecTy) -> Self {
        Self { ty, element: None }
    }

    /// The wire type at the bottom of this chain — the one node that is not a
    /// [`CodecTy::List`], which is the node [`CodecField::class`] and its two
    /// siblings carry the labels of.
    #[must_use]
    pub fn terminal(&self) -> CodecTy {
        let mut at = self;
        while let Some(inner) = &at.element {
            at = inner;
        }
        at.ty
    }

    /// Whether any node of this chain satisfies `test` — how a decoder asks
    /// its "do I have a case for every level of this" question once rather
    /// than per level.
    #[must_use]
    pub fn any(&self, test: impl Fn(CodecTy) -> bool) -> bool {
        let mut at = self;
        loop {
            if test(at.ty) {
                return true;
            }
            match &at.element {
                Some(inner) => at = inner,
                None => return false,
            }
        }
    }
}

/// One field of a class's derived JSON codec: the wire key, the slot it is
/// read from, the constructor position it is written to, and what a decode
/// must produce for it.
///
/// One struct shared by every crate that touches it — `nvs_types::derive`
/// produces the declaration half, `nvs_ir::lower::lower_file` joins the slot
/// and constructor indices in, `nvs-codegen` copies it here — so a field
/// added to the wire contract cannot reach the runtime under a different
/// shape than it left the checker.
#[derive(Clone, Debug)]
pub struct CodecField {
    /// The JSON key: the property's own name, or `#[Json\Field(name: "…")]`.
    pub key: String,
    /// The field slot an encode reads and a decode's `new` ends up writing.
    pub slot: usize,
    /// This field's position in the constructor's parameter list — `rule:core-classes/derive-field-list`'s "every field is a same-named constructor parameter", resolved to
    /// an index so a decoder needs no name lookup.
    pub param: usize,
    /// What a decode has to produce for this field.
    pub ty: CodecTy,
    /// What each position holds when [`Self::ty`] is [`CodecTy::List`], and
    /// `None` for every other one.
    ///
    /// Itself a [`CodecTy::List`] as often as the declaration nests one, which
    /// is what [`CodecElement`] exists for.
    pub element: Option<CodecElement>,
    /// The class's label when a class identity is what the erasure above
    /// dropped: the field's own class for a [`CodecTy::Class`], the *terminal
    /// element's* for a [`CodecTy::List`] at any depth, and `None` for every
    /// other wire type.
    ///
    /// The *declaration* half of a nested field, which is all a front-end
    /// crate can say: a descriptor does not exist until `nvs-codegen` has
    /// defined every class of the unit, and a class may hold a field of its
    /// own type. [`ClassTable::set_codec`] takes the resolved pointers beside
    /// this list, and [`ClassDesc::codec_class`] reads one back.
    pub class: Option<String>,
    /// The accepted backing values where [`Self::ty`] is [`CodecTy::Enum`],
    /// or where [`Self::element`]'s terminal is — the *element's* roster in
    /// that second case, exactly as [`Self::class`] holds the element's label.
    /// `None` for every other wire type.
    pub cases: Option<EnumCases>,
    /// The wire contract's key when [`Self::ty`] is [`CodecTy::Shape`] — or
    /// when [`Self::element`]'s terminal is, an `array<{n: int}>` naming its
    /// element's contract here exactly as it names that element's class on
    /// [`Self::class`]. `None` for every other wire type —
    /// `nvs_ir::lower::shape_codec_key`'s
    /// rendering of the nested shape, which `nvs-codegen` resolves to a
    /// [`ShapeCodec`] address in the second pass [`Self::class`] is resolved in.
    ///
    /// A second label beside [`Self::class`] rather than one label answering
    /// for both, because the two sharings are not the same sharing: `{n: int}`
    /// and `{n: string}` are one class and two contracts, so a shape field
    /// names its class *and* this.
    pub shape: Option<String>,
    /// Whether the declared type admits `null` — `rule:core-api/required-optional-and-nullable`'s second
    /// column, which is a property of the *type* and says nothing about
    /// whether the key may be absent.
    pub nullable: bool,
    /// Whether a document must carry [`Self::key`] at all —
    /// `rule:core-api/required-optional-and-nullable`'s first column, and the
    /// independent question [`Self::nullable`] deliberately does not answer.
    ///
    /// A derived field is required exactly when its constructor parameter
    /// declares no default, which is that rule's "optionality belongs to the
    /// default" read off the one declaration that carries it. A shape field is
    /// required exactly when it was written without `?`
    /// (`rule:types/shape-type`), so the two spellings reach one decoder
    /// through one column rather than through a branch on which of them built
    /// the descriptor.
    pub required: bool,
    /// The constant the constructor parameter's own `= <literal>` default
    /// evaluates to, which is what fills [`Self::key`] when a document does not
    /// carry it — `rule:core-api/required-optional-and-nullable`'s
    /// default-bearing rows, answered at a door that is not a call site.
    ///
    /// `Some` exactly where a derived field's parameter declares a default, so
    /// it is `None` for every required field, for every shape field — a shape
    /// declares no constructor, and an absent optional key of one is the
    /// never-written marker rather than a value — and for a field whose
    /// declaration named no parameter at all, which `nvs_types::derive` has
    /// already refused.
    ///
    /// **What it spends**, per `rule:programs/memory-priority`: one
    /// [`FieldDefault`] per defaulted field in the unit's own descriptor —
    /// O(declarations in the program), never per request — plus, where the
    /// constant is a string, the one allocation per decode the omitted
    /// argument would itself have made.
    pub default: Option<FieldDefault>,
}

/// The wire contract of one **inline shape** written as a type argument —
/// `rule:types/shape-type`'s `{name: T}` read as a codec, which is what a
/// member hydrating into a shape decodes against.
///
/// The same list [`ClassDesc::codec`] holds for a class, carried *beside* a
/// descriptor instead of on one, because a shape class is keyed on its field
/// names alone (`nvs_ir::lower::shape_class_label`): `{n: int}` and
/// `{n: string}` are one class and one [`ClassDesc`], so the per-field wire
/// types have nowhere on the descriptor to live. `nvs-ir`'s module docs own
/// that choice, the one it was taken against, and what it costs.
///
/// One of these per distinct shape *type*, not per call site — whoever defines
/// them decides that sharing, since this table has no way to compare two field
/// lists for the same meaning.
#[derive(Debug)]
pub struct ShapeCodec {
    /// Every field of the shape, in the sorted field-name order the shape
    /// class lays its slots out in — so a field's index is its slot and its
    /// [`CodecField::param`] alike, and nothing has to join two orders.
    fields: Vec<CodecField>,
    /// One entry per [`Self::fields`] entry, null except where the field names
    /// a class — [`ClassDesc::codec_class`]'s convention, resolved in the same
    /// second pass and for the same reason.
    classes: Vec<*const ClassDesc>,
    /// One entry per [`Self::fields`] entry, null except where the field is
    /// itself an inline shape — [`ClassDesc::codec_shapes`]' convention, which
    /// is what makes `{outer: {inner: int}}` decode rather than stop one level
    /// down.
    shapes: Vec<*const ShapeCodec>,
}

impl ShapeCodec {
    /// Every field this shape decodes, in slot order.
    #[must_use]
    pub fn fields(&self) -> &[CodecField] {
        &self.fields
    }

    /// The descriptor the `index`th field decodes into, or `None` where that
    /// field names no class — [`ClassDesc::codec_class`], indexed the same way
    /// and for its reason.
    #[must_use]
    pub fn class(&self, index: usize) -> Option<*const ClassDesc> {
        match self.classes.get(index) {
            Some(desc) if !desc.is_null() => Some(*desc),
            _ => None,
        }
    }

    /// The contract the `index`th field decodes against, or `None` where that
    /// field is not itself an inline shape — [`Self::class`]'s twin, on
    /// [`ClassDesc::codec_shape`]'s terms.
    #[must_use]
    pub fn shape(&self, index: usize) -> Option<*const ShapeCodec> {
        match self.shapes.get(index) {
            Some(codec) if !codec.is_null() => Some(*codec),
            _ => None,
        }
    }
}

impl ClassDesc {
    /// The class's rendered name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether this class is the one an
    /// `rule:types/object-literal` shape
    /// literal constructs, rather than one a `class` declaration named.
    ///
    /// Read off the label `nvs_ir::lower::shape_class_label` mints —
    /// `$shape{x,y}` — because that label is the only mark a shape class
    /// carries. Every structural test that would answer the same question
    /// ("no methods, no codec, no constructor") is also true of an ordinary
    /// class a program wrote and did not opt into a wire format, and
    /// `rule:core-classes/derive-generates-what-is-missing` requires
    /// those to stay refused. `$` cannot start an Novis identifier, so no
    /// declared class collides with the prefix — the guarantee the label
    /// itself already relies on.
    ///
    /// The one reader is `nvs_stdlib::json`'s encoder, which spells a shape as
    /// a JSON object; `tests/conformance/core/json-encodes-a-shape-as-an-object.nvst`
    /// is what holds this spelling and the label's together, since the two
    /// crates cannot see each other.
    #[must_use]
    pub fn is_shape(&self) -> bool {
        self.name.starts_with("$shape{")
    }

    /// Whether an instance of this class is a closure rather than an object of
    /// a declared class — the bit [`ClassTable::set_closure`] writes, and the
    /// one answer [`crate::closure::call_closure`] and
    /// `rule:classes/graph-copy`'s walk both ask.
    ///
    /// Not a question about the method table: the field's own docs say why a
    /// declared `invoke` is the wrong test, and a class carrying this bit is
    /// the only kind either reader treats as callable.
    #[must_use]
    pub fn is_closure(&self) -> bool {
        self.is_closure
    }

    /// Whether an instance of this class holds a host handle — the bit
    /// [`ClassTable::set_host_handle`] writes, and the second of the two marks
    /// `rule:classes/graph-copy`'s walk refuses a value on.
    #[must_use]
    pub fn holds_host_handle(&self) -> bool {
        self.holds_host_handle
    }

    /// How many [`Value`] slots an instance of this class has, including every
    /// ancestor's.
    #[must_use]
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// The slot `name` occupies on an instance of this class, or `None` if
    /// this class has no such field —
    /// `rule:types/erased-member-access`'s
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

    /// The name of the field at slot `index`, or `None` past the last slot.
    ///
    /// The inverse of [`Self::field_slot`], and it exists for the one reader
    /// that walks *every* slot rather than resolving one name: `rule:errors/diagnostic-record`'s
    /// Object node carries a class's declared properties in slot order, which
    /// is what `nvs_stdlib::debug` builds. Compiled code still never reaches
    /// here — a `$obj->prop` on a named class is a fixed offset.
    #[must_use]
    pub fn field_name(&self, index: usize) -> Option<&str> {
        self.fields.get(index).map(String::as_str)
    }

    /// The one [`Tag`] slot `index`'s declared type admits, or `None` where
    /// it admits several or where nothing told this class its field types —
    /// see [`ClassDesc::field_tags`], which owns both readings of `None`.
    #[must_use]
    pub fn field_tag(&self, index: usize) -> Option<Tag> {
        self.field_tags.get(index).copied().flatten()
    }

    /// Whether slot `index`'s declared type carries `rule:security/secret-qualifier`'s `secret`
    /// qualifier — `rule:errors/record-transformations`'s redaction row, asked of an instance because
    /// that is all a dump has.
    ///
    /// `false` for a slot nothing told this class about, and for every slot of
    /// a class the compiler synthesized rather than laid out from a
    /// declaration. That is the safe direction here only because a *declared*
    /// `secret` property always reaches the join that fills this — see
    /// [`Self::secret_fields`]. It is deliberately not
    /// [`Self::field_tag`]'s `Option`, which distinguishes "unknown" from
    /// "admits several": there is no third answer to whether a type carries a
    /// qualifier.
    #[must_use]
    pub fn field_is_secret(&self, index: usize) -> bool {
        self.secret_fields.get(index).copied().unwrap_or(false)
    }

    /// Whether slot `index` is readable from outside this class — `rule:security/reflection-enforces-visibility`
    /// 's visibility check, asked of an instance because that is all a
    /// reflective walk has.
    ///
    /// `false` for a slot nothing told this class about, which is the opposite
    /// direction from [`Self::field_is_secret`] and the safe one in both cases:
    /// a class with no answer here is one no declaration laid out — a closure's
    /// environment, a generator's state, a `Core` class's own slots — and none
    /// of those has a property a program is entitled to read. A *declared*
    /// property always reaches the join that fills this, so the fallback is
    /// never the answer for a class a program wrote.
    #[must_use]
    pub fn field_is_public(&self, index: usize) -> bool {
        self.public_fields.get(index).copied().unwrap_or(false)
    }

    /// Whether slot `index` is declared `protected` — the bit that tells a
    /// member a subclass's own bodies reach from one only the declaring class
    /// does, which is the difference
    /// `rule:security/reflection-enforces-visibility`
    /// rests on and [`Self::field_is_public`] alone cannot carry.
    ///
    /// `false` for a slot nothing told this class, in [`Self::field_is_public`]'s
    /// direction and for its reason: the pair both bits are false for is
    /// `private`, so an unanswered slot is the one that refuses.
    /// [`crate::Ctx::field_is_visible_from`] is what asks.
    #[must_use]
    pub fn field_is_protected(&self, index: usize) -> bool {
        self.protected_fields.get(index).copied().unwrap_or(false)
    }

    /// Every class constant this class answers, in declaration order with each
    /// ancestor's after its own — see [`Self::constants`].
    #[must_use]
    pub fn constants(&self) -> &[ConstantDesc] {
        &self.constants
    }

    /// Every attach site this class's own declaration carries, in source order
    /// — see [`Self::attributes`].
    #[must_use]
    pub fn attributes(&self) -> &[AttributeDesc] {
        &self.attributes
    }

    /// The constant named `name`, or `None` where this class declares and
    /// inherits none.
    ///
    /// By name rather than by index, because a constant claims no slot: there
    /// is no ordinal for a caller to have and nothing for one to be aligned
    /// against. The scan is over a roster of the size a declaration wrote, and
    /// it is reached only from a reflective member.
    #[must_use]
    pub fn constant(&self, name: &str) -> Option<&ConstantDesc> {
        self.constants.iter().find(|c| c.name == name)
    }

    /// The type slot `index` is declared with, as its declaration spells it —
    /// `"?int"`, `"array<string>"`, `"App\\User"`.
    ///
    /// `None` where no declaration named one: a slot of a class the compiler
    /// synthesized, and a slot of the exception tree `nvs_hir::errors` names,
    /// whose types `nvs_types::error_lib` seeds as interned ids rather than as
    /// text. That is an absence a caller reports as such — unlike
    /// [`Self::field_is_public`], where the unknown case is a privilege
    /// question and so answers `false` rather than "unknown".
    #[must_use]
    pub fn field_type(&self, index: usize) -> Option<&str> {
        self.field_types
            .get(index)
            .map(String::as_str)
            .filter(|ty| !ty.is_empty())
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

    /// [`Self::conforms_to`] asked **by name** — the same flattened set, read
    /// as the strings a written class name compares against rather than as the
    /// addresses only a holder of the table has.
    ///
    /// Safe where its twin is `unsafe` because it names no descriptor of its
    /// own: every pointer it follows is a [`Self::conforms`] entry, which
    /// belongs to the same [`ClassTable`] as this descriptor and so outlives
    /// it by that field's own invariant.
    ///
    /// Its caller is [`crate::Ctx::pending_conforms_to`], and through it ADR
    /// 0079 § 4's `Core\Test::assertThrows`, whose expectation arrives as
    /// `ParseError::class`'s folded `string` constant — the same spelling this
    /// table names a class with.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "a `conforms` entry belongs to the same table as this \
                  descriptor and outlives it — this type's own invariant, \
                  stated at the field"
    )]
    pub fn conforms_to_name(&self, name: &str) -> bool {
        self.name == name
            || self
                .conforms
                .iter()
                .any(|&other| unsafe { (*other).name == name })
    }

    /// The compiled address of the method this class answers `name` with —
    /// its own override if it declares one, otherwise the nearest ancestor's
    /// — or `None` if nothing in the chain declares a *body* for it.
    ///
    /// `None` is an ordinary answer, not a failure: an interface method with
    /// no default body has no code, and [`nvs_class_method`]'s caller supplies
    /// the statically resolved target as the fallback.
    #[must_use]
    pub fn method(&self, name: &str) -> Option<*const u8> {
        self.method_row(name).map(|row| row.code)
    }

    /// The whole [`MethodRow`] `name` names — the address *and* the declared
    /// shape a caller holding only tagged values has to check its arguments
    /// against before it jumps.
    ///
    /// The same binary search [`ClassDesc::method`] makes, which is why that
    /// one is written over this rather than beside it: two searches over one
    /// table are two places for the precedence order to be read differently.
    #[must_use]
    pub fn method_row(&self, name: &str) -> Option<&MethodRow> {
        self.methods
            .binary_search_by(|row| row.name.as_str().cmp(name))
            .ok()
            .map(|index| &self.methods[index])
    }

    /// The [`HookRow`] for `property`'s `set` hook when `set`, its `get` hook
    /// otherwise — or `None` when the property declares that accessor nowhere
    /// in the chain, which is the ordinary answer for every unhooked property.
    ///
    /// `None` means "write the slot" rather than "the write fails": a property
    /// with no hook is its own storage, and
    /// `rule:types/erased-member-access`'s slot access is what a caller falls
    /// back to.
    #[must_use]
    pub fn hook_row(&self, property: &str, set: bool) -> Option<&HookRow> {
        self.hooks
            .binary_search_by(|row| row.property.as_str().cmp(property).then(row.set.cmp(&set)))
            .ok()
            .map(|index| &self.hooks[index])
    }

    /// How many methods this descriptor answers for — its own plus every
    /// inherited one.
    #[must_use]
    pub fn method_count(&self) -> usize {
        self.methods.len()
    }

    /// The method at `index` in name order, or `None` past the last one.
    ///
    /// [`Self::field_name`]'s twin, and it exists for the same single reader:
    /// the walk that wants *every* row rather than the one a dispatch names.
    /// `Core\Reflect\ClassInfo::methods` is that walk, and the order it
    /// inherits is the sort [`Self::methods`] is already kept in for
    /// [`Self::method_row`]'s binary search — which is what makes a reflective
    /// roster the same list on every run and every machine.
    #[must_use]
    pub fn method_at(&self, index: usize) -> Option<&MethodRow> {
        self.methods.get(index)
    }

    /// `rule:core-classes/derive-attribute`'s derived JSON field list, in declaration order — empty for a
    /// class carrying no `#[Json\Derive]`.
    ///
    /// Declaration order is the encode order, which is what makes an encoded
    /// document byte-deterministic across runs and machines (that ADR § 2).
    #[must_use]
    pub fn codec(&self) -> &[CodecField] {
        &self.codec
    }

    /// The descriptor the `index`th codec field decodes into, or `None` where
    /// that field names no class — see [`Self::codec_classes`].
    ///
    /// Indexed by position in [`Self::codec`] rather than reached through the
    /// [`CodecField`] itself, because the field is shared with front-end
    /// crates that hold no descriptor to put in it.
    #[must_use]
    pub fn codec_class(&self, index: usize) -> Option<*const ClassDesc> {
        match self.codec_classes.get(index) {
            Some(desc) if !desc.is_null() => Some(*desc),
            _ => None,
        }
    }

    /// The contract the `index`th codec field decodes against, or `None` where
    /// that field names no inline shape — [`Self::codec_class`]'s twin, indexed
    /// the same way and for its reason.
    #[must_use]
    pub fn codec_shape(&self, index: usize) -> Option<*const ShapeCodec> {
        match self.codec_shapes.get(index) {
            Some(codec) if !codec.is_null() => Some(*codec),
            _ => None,
        }
    }

    /// `rule:core-classes/derive-attribute`'s derived **row** field list, in declaration order — empty for
    /// a class carrying no `#[Db\Derive]`.
    ///
    /// Unlike [`Self::codec`] the order is not an output order: there is no
    /// encoding half at all (spec § 18's `Core\Db\Codec` declares `fromRow`
    /// and nothing else), so this is read to *fill* a constructor and the
    /// order it is read in is the constructor's.
    #[must_use]
    pub fn db_codec(&self) -> &[CodecField] {
        &self.db_codec
    }

    /// The descriptor the `index`th [`Self::db_codec`] field decodes into, or
    /// `None` where that field names no class — [`Self::codec_class`]'s twin.
    #[must_use]
    pub fn db_codec_class(&self, index: usize) -> Option<*const ClassDesc> {
        match self.db_codec_classes.get(index) {
            Some(desc) if !desc.is_null() => Some(*desc),
            _ => None,
        }
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

    /// The address of this class's native `compareTo`, or `None` for a class
    /// carrying none — see [`Self::compare`] for why it is not a method row.
    #[must_use]
    pub fn comparer(&self) -> Option<*const u8> {
        if self.compare.is_null() {
            None
        } else {
            Some(self.compare)
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
/// [`ClassTable::desc`] hand out a pointer `nvs-codegen` embeds in machine
/// code.
///
/// The table must outlive every instance of every class it defines, and every
/// compiled function that can allocate one. `nvs-codegen`'s compiled unit is
/// the natural owner.
#[derive(Debug, Default)]
pub struct ClassTable {
    /// Boxed individually, and deliberately so: `ClassTable::desc` hands out a
    /// raw pointer that `nvs-codegen` bakes into machine code, and a
    /// `Vec<ClassDesc>` would move every descriptor the next `define` reallocs.
    /// `clippy::vec_box` cannot see that the indirection *is* the point.
    #[expect(
        clippy::vec_box,
        reason = "each descriptor's address must survive later `define` calls;                   see the field's own comment"
    )]
    classes: Vec<Box<ClassDesc>>,
    /// Every inline shape's wire contract this unit's call sites wrote —
    /// boxed for the field above's reason, since `nvs-codegen` bakes one of
    /// these addresses too. Owned here rather than beside the descriptors
    /// because a shape's contract belongs to no class: see [`ShapeCodec`].
    #[expect(
        clippy::vec_box,
        reason = "each contract's address must survive later definitions;                   see `classes` above"
    )]
    shape_codecs: Vec<Box<ShapeCodec>>,
    /// Every `enum` the unit's files declare, plus every `Core` enum the
    /// checker seeded its table with — see [`EnumDesc`].
    ///
    /// Unboxed, unlike the two above, and that is the whole difference between
    /// them: no address of one is ever baked into machine code, because an enum
    /// case at run time is the integer behind it and compiled code never
    /// reaches this at all. The one reader is `Core\Reflect\EnumInfo::of`,
    /// which arrives by name.
    enums: Vec<EnumDesc>,
}

/// A table is `Send` and `Sync` because a compiled unit is read by every core.
///
/// `rule:security/isolate-shares-nothing` lets exactly one thing cross an
/// isolate boundary — immutable compiled code — and this table is the runtime
/// half of it: `nvs_codegen::Unit` owns one behind a `std::sync::Arc`, and every
/// core resolving a request against that unit reads the same descriptors through
/// the same pointers. Without these two impls the `Arc` cannot leave the core
/// that compiled it, and the unit cache would have to be one per core.
///
/// The claim these make, and each half of why it holds:
///
/// - **Nothing here is interiorly mutable.** Every field of a [`ClassDesc`], a
///   [`MethodRow`], a [`ShapeCodec`] and an [`EnumDesc`] is a plain owned value
///   or a raw pointer;
///   the `Cell`s in this module are all in [`ObjHeader`] and [`LiveList`], which
///   are per-instance and per-request and reach nothing a table owns. So two
///   cores holding `&ClassTable` are two readers of frozen memory.
/// - **Mutation needs exclusive access, and the compiler is done before the
///   sharing starts.** Every method on this table that writes a descriptor takes
///   `&mut self` — an `Arc` hands that out only through `Arc::get_mut`, and only
///   while it is the sole handle. `nvs-codegen` runs all of them before it wraps
///   the table, so no core ever observes a half-filled descriptor.
/// - **The `*const ClassDesc` pointers cannot dangle on another thread**, because
///   they point into *this* table's own boxed descriptors ([`ClassDesc`]'s own
///   doc is the home of that invariant) and each box is individually allocated,
///   so the address a reader follows is alive for exactly as long as the handle
///   it followed it from.
/// - **A [`MethodRow::code`] address is executable pages the same unit keeps
///   mapped**, and moving the address between threads is not the operation that
///   needs a contract — calling through it is, and every caller already carries
///   that `unsafe` itself.
///
/// What this does **not** say is that a `ClassDesc` may cross on its own: a bare
/// `*const ClassDesc` is still `!Send`, and the table is the unit of sharing
/// precisely because it is the thing that owns what the pointers point at.
#[expect(
    unsafe_code,
    reason = "the table is frozen before it is shared and owns everything its \
              raw pointers address; the four paragraphs above are the argument"
)]
// SAFETY: see the doc comment above — no interior mutability, every mutator
// takes `&mut self` and runs before the table is wrapped, and every pointer
// addresses memory this same table owns.
unsafe impl Send for ClassTable {}

#[expect(
    unsafe_code,
    reason = "shared reads of a frozen table, on `Send`'s argument above"
)]
// SAFETY: as `Send` above. `&ClassTable` exposes reads only, so N concurrent
// readers see the same immutable descriptors.
unsafe impl Sync for ClassTable {}

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
            hooks: Vec::new(),
            codec: Vec::new(),
            codec_classes: Vec::new(),
            codec_shapes: Vec::new(),
            db_codec: Vec::new(),
            db_codec_classes: Vec::new(),
            ctor_arity: 0,
            defaults: Vec::new(),
            field_tags: Vec::new(),
            secret_fields: Vec::new(),
            public_fields: Vec::new(),
            protected_fields: Vec::new(),
            field_types: Vec::new(),
            constants: Vec::new(),
            attributes: Vec::new(),
            render: std::ptr::null(),
            compare: std::ptr::null(),
            unwind: std::ptr::null(),
            is_closure: false,
            holds_host_handle: false,
        }));
        id
    }

    /// Marks `id` as a closure's environment class — see
    /// [`ClassDesc::is_closure()`].
    ///
    /// A setter rather than a [`ClassTable::define`] parameter because the
    /// answer is `false` for every class a program declares and every `Core`
    /// class, and a parameter would make each of those call sites say so. What
    /// calls this is `nvs-codegen`, for a class `nvs_ir::lower` minted from a
    /// closure literal, and native code hand-building a closure for a `Core`
    /// member to call back into.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table.
    pub fn set_closure(&mut self, id: ClassId) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        desc.is_closure = true;
    }

    /// Marks `id` as a class whose instances hold a host handle — see
    /// [`ClassDesc::holds_host_handle()`].
    ///
    /// A setter on [`ClassTable::set_closure`]'s exact terms, and the one
    /// caller is `nvs_stdlib::instance`: it builds the process's `Core`
    /// descriptors, and its crate is the only one that knows which slot of
    /// which class carries a key filed by [`Ctx::hold_open_file`] and its
    /// siblings.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table.
    pub fn set_host_handle(&mut self, id: ClassId) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        desc.holds_host_handle = true;
    }

    /// Fills in `id`'s per-slot declared tags — see [`ClassDesc::field_tags`].
    ///
    /// Separate from [`ClassTable::define`] on [`ClassTable::set_defaults`]'
    /// terms exactly: a slot's *name* is the same fact as its existence, while
    /// what its declared type admits is a second table's answer that
    /// `nvs-codegen` maps out of `nvs_ir::Ty` on the way here.
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

    /// Fills in `id`'s per-slot `secret` bits — see
    /// [`ClassDesc::secret_fields`].
    ///
    /// Separate from [`ClassTable::set_field_tags`] rather than folded into it
    /// because the two answer different questions of the same declaration and
    /// a class may carry one without the other: a synthesized class has slot
    /// representations and no declared qualifier at all.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table, or if `secret` is not one entry
    /// per slot — a length disagreement would redact one property in place of
    /// another, which discloses the value it was meant to hide.
    pub fn set_secret_fields(&mut self, id: ClassId, secret: Vec<bool>) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        assert!(
            secret.len() == desc.fields.len(),
            "`{}` has {} field slots but {} declared `secret` bits",
            desc.name,
            desc.fields.len(),
            secret.len()
        );
        desc.secret_fields = secret;
    }

    /// Fills in `id`'s per-slot visibility bits — see
    /// [`ClassDesc::public_fields`].
    ///
    /// Separate from [`ClassTable::set_secret_fields`] on that method's own
    /// terms: the two are different questions of the same declaration, and a
    /// class may carry one without the other.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table, or if `public` is not one entry
    /// per slot — a length disagreement would answer one property's visibility
    /// with another's, which opens the member `rule:security/reflection-enforces-visibility` exists to keep shut.
    pub fn set_public_fields(&mut self, id: ClassId, public: Vec<bool>) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        assert!(
            public.len() == desc.fields.len(),
            "`{}` has {} field slots but {} declared visibility bits",
            desc.name,
            desc.fields.len(),
            public.len()
        );
        desc.public_fields = public;
    }

    /// Fills in `id`'s per-slot `protected` bits — see
    /// [`ClassDesc::protected_fields`].
    ///
    /// Separate from [`ClassTable::set_public_fields`] for that method's own
    /// reason: the two are different readings of one declaration, and a class
    /// may carry the first without the second — which is exactly what a class
    /// with no `protected` property is.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table, or if `protected` is not one
    /// entry per slot — a length disagreement would answer one property's
    /// level with another's, which opens the member
    /// `rule:security/reflection-enforces-visibility` exists to keep shut.
    pub fn set_protected_fields(&mut self, id: ClassId, protected: Vec<bool>) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        assert!(
            protected.len() == desc.fields.len(),
            "`{}` has {} field slots but {} declared `protected` bits",
            desc.name,
            desc.fields.len(),
            protected.len()
        );
        desc.protected_fields = protected;
    }

    /// Fills in `id`'s class constants — see [`ClassDesc::constants`].
    ///
    /// No length assertion, where the three setters around it each make one:
    /// this roster is keyed by name and aligned to nothing, so there is no
    /// count it could disagree with and no way for a short list to answer one
    /// constant's value under another's name. A class that never reaches this
    /// setter declares none, which is the same answer an empty list gives.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table.
    pub fn set_class_constants(&mut self, id: ClassId, constants: Vec<ConstantDesc>) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        desc.constants = constants;
    }

    /// Fills in `id`'s attach sites — see [`ClassDesc::attributes`].
    ///
    /// No length assertion, on [`ClassTable::set_class_constants`]' terms: this
    /// roster is aligned to nothing either, each row naming the declaration it
    /// was written on rather than occupying a position that stands for one.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table.
    pub fn set_class_attributes(&mut self, id: ClassId, attributes: Vec<AttributeDesc>) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        desc.attributes = attributes;
    }

    /// Fills in `id`'s per-slot declared type names — see
    /// [`ClassDesc::field_types`].
    ///
    /// Separate from [`ClassTable::set_public_fields`] on that method's own
    /// terms: a synthesized class carries a visibility bit for no slot and a
    /// type name for no slot either, but the two are different readings of the
    /// declaration and a class may reach this table with one and not the
    /// other.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table, or if `types` is not one entry
    /// per slot — a length disagreement would answer one property's type with
    /// another's, which is the same misreport a name-keyed roster exists to
    /// prevent.
    pub fn set_field_types(&mut self, id: ClassId, types: Vec<String>) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        assert!(
            types.len() == desc.fields.len(),
            "`{}` has {} field slots but {} declared type names",
            desc.name,
            desc.fields.len(),
            types.len()
        );
        desc.field_types = types;
    }

    /// Fills in `id`'s declared property defaults — see [`ClassDesc::defaults`].
    ///
    /// Separate from [`ClassTable::define`] on exactly [`ClassTable::set_codec`]'s
    /// terms: the fact comes from a different `nvs-ir` table, and there is no
    /// compiled address to wait for.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table, or if a slot index is out of
    /// range for the class — which would mean `nvs-ir` joined a default
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

    /// Fills in `id`'s `rule:core-classes/derive-attribute` derived-codec field list — see
    /// [`ClassDesc::codec`].
    ///
    /// Separate from [`ClassTable::define`] because the two facts come from
    /// two different `nvs-ir` tables, and — unlike [`ClassTable::set_defaults`]
    /// — because `classes` names descriptors this table may not have defined
    /// yet: `rule:core-classes/derive-field-list`'s nested field admits a class of the unit's own
    /// making, its own type included, so `nvs-codegen` calls this in a second
    /// pass over classes it has all defined rather than while defining one.
    ///
    /// `classes` is one entry per `codec` field, null except where the field's
    /// [`CodecField::ty`] is [`CodecTy::Class`] — see
    /// [`ClassDesc::codec_class`] — and `shapes` is one entry per field on the
    /// same terms, null except where that wire type is [`CodecTy::Shape`].
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table, or if either resolved list is not
    /// one entry per field — a length disagreement would decode one field
    /// against another field's contract, which builds an object out of the
    /// wrong constructor.
    pub fn set_codec(
        &mut self,
        id: ClassId,
        codec: Vec<CodecField>,
        ctor_arity: usize,
        classes: Vec<*const ClassDesc>,
        shapes: Vec<*const ShapeCodec>,
    ) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        assert!(
            classes.len() == codec.len(),
            "`{}` has {} codec field(s) but {} resolved nested class(es)",
            desc.name,
            codec.len(),
            classes.len()
        );
        assert!(
            shapes.len() == codec.len(),
            "`{}` has {} codec field(s) but {} resolved nested contract(s)",
            desc.name,
            codec.len(),
            shapes.len()
        );
        desc.codec = codec;
        desc.codec_classes = classes;
        desc.codec_shapes = shapes;
        desc.ctor_arity = ctor_arity;
    }

    /// Fills in `id`'s `rule:core-classes/derive-attribute` derived **row** field list — [`set_codec`]'s
    /// twin for `#[Db\Derive]`, on the same terms and called from the same
    /// second pass.
    ///
    /// `ctor_arity` is written by whichever of the two runs, and both carry
    /// the same number: it is a property of the class's `constructor` and not
    /// of either mapping, and a class carrying only one attribute would
    /// otherwise leave the decoder an arity of zero.
    ///
    /// # Panics
    ///
    /// On [`set_codec`]'s two conditions, for its reasons.
    ///
    /// [`set_codec`]: ClassTable::set_codec
    pub fn set_db_codec(
        &mut self,
        id: ClassId,
        codec: Vec<CodecField>,
        ctor_arity: usize,
        classes: Vec<*const ClassDesc>,
    ) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        assert!(
            classes.len() == codec.len(),
            "`{}` has {} row codec field(s) but {} resolved nested class(es)",
            desc.name,
            codec.len(),
            classes.len()
        );
        desc.db_codec = codec;
        desc.db_codec_classes = classes;
        desc.ctor_arity = ctor_arity;
    }

    /// Takes ownership of one inline shape's wire contract and hands back its
    /// address, for `nvs-codegen` to bake into the call site that wrote the
    /// shape — [`ShapeCodec`]'s own docs say why a shape's contract cannot ride
    /// on a [`ClassDesc`] the way a class's does.
    ///
    /// Boxed for [`ClassTable::desc`]'s reason, unchanged: the address is
    /// handed to compiled code, so it has to survive every later definition
    /// this table takes. It is filled in the same second pass
    /// [`ClassTable::set_codec`] is, and for the same reason — a nested field
    /// names a class that may not be defined yet.
    ///
    /// # Panics
    ///
    /// If either resolved list is not one entry per field, which would decode
    /// one field against another field's class or contract.
    pub fn define_shape_codec(
        &mut self,
        fields: Vec<CodecField>,
        classes: Vec<*const ClassDesc>,
        shapes: Vec<*const ShapeCodec>,
    ) -> *const ShapeCodec {
        assert!(
            classes.len() == fields.len(),
            "a shape codec has {} field(s) but {} resolved nested class(es)",
            fields.len(),
            classes.len()
        );
        assert!(
            shapes.len() == fields.len(),
            "a shape codec has {} field(s) but {} resolved nested contract(s)",
            fields.len(),
            shapes.len()
        );
        self.shape_codecs.push(Box::new(ShapeCodec {
            fields,
            classes,
            shapes,
        }));
        let codec: &ShapeCodec = self
            .shape_codecs
            .last()
            .expect("the entry just pushed is the last one");
        std::ptr::from_ref(codec)
    }

    /// Fills in `id`'s method table — one [`MethodRow`] per name, which this
    /// sorts by name so [`ClassDesc::method`] can binary-search them.
    ///
    /// Separate from [`ClassTable::define`] because a compiled function has no
    /// address until its module is finalized, which is long after every class
    /// is defined. `nvs-codegen` calls this in its own `finish`.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table.
    pub fn set_methods(&mut self, id: ClassId, methods: Vec<MethodRow>) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        desc.methods = methods;
        desc.methods.sort_by(|a, b| a.name.cmp(&b.name));
        desc.methods.dedup_by(|a, b| a.name == b.name);
        // Resolved once per class here rather than once per dying instance in
        // `dismantle` — see `ClassDesc::unwind`.
        desc.unwind = desc
            .method(GENERATOR_UNWIND_METHOD)
            .unwrap_or(std::ptr::null());
    }

    /// Fills in `id`'s hook table — one [`HookRow`] per `(property,
    /// accessor)`, which this sorts on that pair so [`ClassDesc::hook_row`]
    /// can binary-search them.
    ///
    /// Separate from [`ClassTable::define`] for [`ClassTable::set_methods`]'
    /// reason exactly, and separate from *it* because a hook is not a method:
    /// the two tables are keyed differently and a `Class::name()` call must
    /// never find an accessor.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table.
    pub fn set_hooks(&mut self, id: ClassId, hooks: Vec<HookRow>) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        desc.hooks = hooks;
        desc.hooks
            .sort_by(|a, b| a.property.cmp(&b.property).then(a.set.cmp(&b.set)));
        desc.hooks
            .dedup_by(|a, b| a.property == b.property && a.set == b.set);
    }

    /// Fills in `id`'s native renderer — see [`ClassDesc::renderer`].
    ///
    /// `address` is an `rule:errors/propagation` helper that takes the instance as its one
    /// argument and **borrows** it, which is what separates this from
    /// [`ClassTable::set_methods`]; `nvs_stdlib::instance` is its only caller,
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

    /// Fills in `id`'s native `compareTo` — see [`ClassDesc::comparer`].
    ///
    /// `address` is an `rule:errors/propagation` helper taking the two
    /// instances to order and **borrowing** both, which is what separates this
    /// from [`ClassTable::set_methods`]; `nvs_stdlib::instance` is its only
    /// caller, for [`ClassTable::set_render`]'s reason.
    ///
    /// # Panics
    ///
    /// If `id` does not belong to this table.
    pub fn set_compare(&mut self, id: ClassId, address: *const u8) {
        let desc = self
            .classes
            .get_mut(id.0)
            .expect("a class id always belongs to the table that handed it out");
        desc.compare = address;
    }

    /// The descriptor `name` names, **borrowed** — the same one
    /// [`ClassTable::desc`] hands out as a raw pointer, read by a caller that
    /// is not compiled code.
    ///
    /// A borrow rather than a pointer because the two callers want opposite
    /// things: machine code needs an address that outlives every frame, while
    /// a crate reading a class's tables — a test, a tool, `nvs-codegen`
    /// itself — needs an answer it can reach without writing `unsafe`, which
    /// `#![deny(unsafe_code)]` puts out of its reach entirely. The lifetime is
    /// this table's, which is exactly the guarantee the pointer already
    /// carried informally.
    #[must_use]
    pub fn desc_of(&self, name: &str) -> Option<&ClassDesc> {
        let id = self.id_of(name)?;
        self.classes.get(id.0).map(Box::as_ref)
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

    /// Records one declared `enum`'s shape — [`EnumDesc`]'s whole write path.
    ///
    /// `cases` arrives as the declaration resolved it and is sorted here, by
    /// value and then by name, so [`EnumDesc::cases`]' stated order is a
    /// property of the table rather than of whichever pass filled it.
    ///
    /// Hands back no token, unlike [`ClassTable::define`]: there is nothing to
    /// fill in a second pass, because an enum names no other type.
    pub fn define_enum(
        &mut self,
        name: impl Into<String>,
        unsigned: bool,
        cases: Vec<(String, i128)>,
    ) {
        let mut cases = cases;
        cases.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        self.enums.push(EnumDesc {
            name: name.into(),
            unsigned,
            cases,
        });
    }

    /// The enum named `name`, or `None` if this unit declares none.
    ///
    /// A linear scan, on [`ClassTable::id_of`]'s reasoning: the one caller is
    /// `Core\Reflect\EnumInfo::of`, which a program reaches for to describe a
    /// type rather than in a loop over values, and a second index would cost
    /// every unit to save that call nothing it can measure.
    #[must_use]
    pub fn enum_desc(&self, name: &str) -> Option<&EnumDesc> {
        self.enums.iter().find(|desc| desc.name == name)
    }

    /// How many classes are defined.
    #[must_use]
    pub fn len(&self) -> usize {
        self.classes.len()
    }

    /// The id of the class named `name`, or `None` if this table defines none.
    ///
    /// A linear scan: the one caller is `nvs-codegen` looking up a single
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

/// The header sitting in front of every Novis object's field slots.
///
/// `#[repr(C)]` because compiled code reads these fields at fixed offsets.
/// Never construct one by value — it is only ever the first
/// `size_of::<ObjHeader>()` bytes of a larger allocation made by
/// [`NvsObj::new`], and moving it would leave the fields behind.
#[repr(C)]
#[derive(Debug)]
pub struct ObjHeader {
    /// How many owners hold this allocation. Reaching `0` frees it.
    refcount: Cell<usize>,
    /// The class this is an instance of. Immutable for the allocation's life.
    class: *const ClassDesc,
    /// The next object on the owning context's [`LiveList`], or null at its
    /// end — see this module's *Decision: every object is on its context's
    /// live list*.
    next: Cell<*mut ObjHeader>,
    /// The link that points *at* this object: either the [`LiveList`]'s own
    /// head cell or the predecessor's [`next`](Self::next). Null while the
    /// object is on no list, which is what an object allocated with no context
    /// current stays for its whole life.
    ///
    /// A pointer to the *link* rather than to the predecessor, so that
    /// unlinking needs no access to the head and therefore no way back to the
    /// context — a decrement carries none, which is
    /// [`crate::ctx::CurrentCtx`]'s own decision.
    prev: Cell<*const Cell<*mut ObjHeader>>,
    /// Debug builds only: the list this object was last linked into, which is
    /// the context that owns it — see this module's *Decision* section.
    ///
    /// Written by [`LiveList::link`] alone, so that the stamp and the links are
    /// made in one place and a relink cannot move one without the other, and
    /// **not cleared by [`unlink`]**: an object that outlives its context still
    /// says which one it came from, which is what makes a foreign dismantle
    /// detectable at all. Null for an object allocated with no context current,
    /// which is every object a Rust test builds by hand.
    ///
    /// The pointer is compared and never dereferenced — the list it names may
    /// be gone, which is exactly the case the assertion exists to catch.
    #[cfg(debug_assertions)]
    owner: Cell<*const LiveList>,
}

/// Every object one [`Ctx`] has allocated and not yet dismantled, as the
/// intrusive doubly-linked list
/// `rule:security/isolate-teardown-is-a-drain-then-a-sweep`
/// 's teardown sweep walks.
///
/// **Its own allocation, held by the context through an `Rc`**, rather than a
/// field of [`Ctx`]: an object links itself in from [`nvs_object_new`] while a
/// helper above it may be holding `&mut Ctx`, and reaching into that
/// reference's allocation behind its back is exactly the aliasing a `&mut`
/// promises does not happen. A second allocation per context is the cheapest
/// thing that makes the write land somewhere no `&mut Ctx` claims.
#[derive(Debug, Default)]
pub struct LiveList {
    /// The most recently allocated object, or null when nothing is live.
    head: Cell<*mut ObjHeader>,
    /// Whether [`reclaim`] is walking this list right now, so that a walk
    /// reached from inside one is refused rather than entered.
    ///
    /// The re-entry is reachable in flight and unreachable at teardown, and
    /// what makes it reachable is the dismantling itself: an abandoned
    /// generator's unwind entry point is Novis code, and that code's own back
    /// edges poll the safepoint that asks for a collection. A second walk
    /// would be *sound* — every member the outer one condemned holds the
    /// outer walk's own reference, so the inner tally reads it as held from
    /// outside — but it is unbounded, since the inner walk dismantles too.
    /// One word, written by a walk and read by nothing else.
    walking: Cell<bool>,
}

impl LiveList {
    /// Puts `object` at the front of the list.
    ///
    /// # Safety
    ///
    /// `object` must refer to a live allocation that is on no list, and must
    /// leave through [`unlink`] before it is freed.
    #[expect(
        unsafe_code,
        reason = "the allocation's liveness and its absence from any other \
                  list are the caller's obligations to state"
    )]
    unsafe fn link(&self, object: *mut ObjHeader) {
        let head = self.head.get();
        #[expect(
            unsafe_code,
            reason = "`object` is a live allocation the caller just made, and \
                      `head` was linked by this same routine and has not been \
                      unlinked, since unlinking is what takes it off this list"
        )]
        unsafe {
            (*object).next.set(head);
            (*object).prev.set(&self.head);
            if !head.is_null() {
                (*head).prev.set(&(*object).next);
            }
            // The stamp is written here and nowhere else, so an object's idea
            // of its owner and the list it is actually on are made by one
            // statement — see [`ObjHeader::owner`].
            #[cfg(debug_assertions)]
            (*object).owner.set(std::ptr::from_ref(self));
        }
        self.head.set(object);
    }

    /// Every object on the list, head first — [`sweep`]'s snapshot, taken
    /// before anything it does can free one.
    fn members(&self) -> Vec<*mut ObjHeader> {
        let mut members = Vec::new();
        let mut member = self.head.get();
        while !member.is_null() {
            members.push(member);
            #[expect(
                unsafe_code,
                reason = "every member was linked from a live allocation and \
                          leaves the list in `unlink` before it is freed"
            )]
            unsafe {
                member = (*member).next.get();
            }
        }
        members
    }

    /// How many objects are on the list — what [`sweep`] would have to
    /// consider if the context ended now.
    #[cfg(test)]
    pub(crate) fn count(&self) -> usize {
        self.members().len()
    }
}

/// Takes `object` off whatever [`LiveList`] holds it, if any.
///
/// Idempotent, and a no-op for an object that was never linked — which is
/// every object allocated with no context current, the shape a Rust test that
/// builds one by hand takes.
///
/// # Safety
///
/// `object` must refer to a live allocation, as must the neighbours its own
/// links name.
#[expect(
    unsafe_code,
    reason = "the allocation's liveness is the caller's obligation to state"
)]
unsafe fn unlink(object: *mut ObjHeader) {
    #[expect(
        unsafe_code,
        reason = "`object` is live by the caller's contract, and its links name \
                  either the list head or a neighbour that has not been freed, \
                  because freeing one is what runs this routine on it"
    )]
    unsafe {
        let prev = (*object).prev.get();
        if prev.is_null() {
            return;
        }
        let next = (*object).next.get();
        (*prev).set(next);
        if !next.is_null() {
            (*next).prev.set(prev);
        }
        (*object).prev.set(std::ptr::null());
        (*object).next.set(std::ptr::null_mut());
    }
}

/// Moves `object` onto the live list of the context running now, because a
/// crossing just handed that context the allocation itself — see this module's
/// *Decision* section, which owns why this lives inside
/// [`crate::graph`]'s adopt and is called from nowhere else.
///
/// A no-op when no context is running: the destination has no list to join, and
/// taking the object off the one it is on would hide it from the only sweep
/// that can reclaim it.
///
/// # Safety
///
/// `object` must refer to a live allocation whose one reference the caller
/// holds, as must the neighbours its own links name.
#[expect(
    unsafe_code,
    reason = "the allocation's liveness and unique ownership are the caller's \
              obligations to state"
)]
pub(crate) unsafe fn relink_to_current(object: *mut ObjHeader) {
    let list = crate::ctx::current_live_list();
    if list.is_null() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "`object` is live by the caller's contract, and `list` names \
                  the `LiveList` allocation the current context holds an `Rc` \
                  to, which outlives this call"
    )]
    unsafe {
        unlink(object);
        (*list).link(object);
    }
}

/// Debug builds only: `object` is linked on the list it says owns it.
///
/// # What this catches, and what it deliberately does not
///
/// The stamp and the links are written by one statement in [`LiveList::link`],
/// so they can only disagree if some *other* code moves an object between
/// lists — a second relink site, or a splice. That is the drift
/// [`relink_to_current`] exists to stop anyone writing, and it is a
/// use-after-free waiting to happen: a context's sweep takes apart what its own
/// list holds, so an object linked on a list that is not its owner's is one a
/// foreign teardown may dismantle under its holder.
///
/// It is **not** a check that the context releasing an object is the one that
/// allocated it. That is routinely false and legitimately so: `nvs_host`'s
/// `finish` drops a child's exception object and copies its answer out while
/// the *parent* is the installed context, and a value that outlives its whole
/// context — [`sweep`]'s own docs name the ways one does — is released later
/// still.
///
/// # Safety
///
/// `object` must refer to a live allocation, as must the neighbour its `prev`
/// link names.
#[cfg(debug_assertions)]
#[expect(
    unsafe_code,
    reason = "the allocation's liveness is the caller's obligation to state"
)]
unsafe fn assert_linked_where_it_says(object: *mut ObjHeader) {
    #[expect(
        unsafe_code,
        reason = "`object` is live by the caller's contract, and the link it \
                  names is either a live list's head cell or a live \
                  neighbour's `next`"
    )]
    unsafe {
        let prev = (*object).prev.get();
        if prev.is_null() {
            // On no list: never linked, or detached by [`Detach`] when the
            // context that held it went down.
            return;
        }
        let owner = (*object).owner.get();
        assert!(
            !owner.is_null(),
            "an object is on a live list with no owner stamped — \
             `LiveList::link` writes both, so this is a second linking site"
        );
        // Its neighbour, which is a live member of whatever list this object is
        // really on. The stamp is not dereferenced anywhere here: a wrong one
        // is the thing being asserted about, so it may name a list that is
        // already gone.
        let next = (*object).next.get();
        if !next.is_null() {
            assert!(
                std::ptr::eq((*next).owner.get(), owner),
                "an object is linked on one context's live list and stamped \
                 with another's: `crate::graph`'s adopt is the one place \
                 ownership changes, and it relinks — see this module's docs"
            );
        }
        // The last member of a list has no neighbour to agree with, and
        // reaching its predecessor would mean trusting the stamp to say
        // whether `prev` is a head cell or a `next` — which is the thing in
        // doubt. It is left unchecked here and checked by [`sweep`] instead,
        // where the list itself is in hand.
    }
}

/// Dismantles what the root drain left on `list` and could not free —
/// `rule:security/isolate-teardown-is-a-drain-then-a-sweep`
/// 's cyclic garbage — and empties the list of whatever survived that.
///
/// The teardown half of [`reclaim`], whose docs own the walk the two halves
/// share. The [`Detach`] guard is the whole of what makes it the teardown
/// half: a survivor is taken off a list that is about to be freed with the
/// context, which is exactly what a context still running must not do to its
/// own.
pub(crate) fn sweep(list: &LiveList) {
    // Survivors are detached before this returns, whichever way it leaves.
    let _detach = Detach(list);
    reclaim(list);
}

/// Reclaims the cyclic garbage on `list` **while its context is still
/// running**, and answers how many objects that was.
///
/// The in-flight half of [`reclaim`], and the whole of the difference is that
/// nothing is detached: a survivor stays linked where it is, because the list
/// it sits on belongs to a context that goes on allocating into it.
///
/// **Reached only near the memory ceiling**, which is the decision this
/// collector is built under. [`crate::budget`]'s threshold raises
/// [`SafepointFlags::COLLECT`](crate::SafepointFlags) at the allocation that
/// crossed the ceiling, and `Ctx::collect_if_asked` — the one door in — runs
/// this from the poll that would otherwise have reported the breach. A request
/// that stays under its ceiling walks no list, and a request under no ceiling
/// arms no threshold and therefore never asks, which is what the normal
/// request path pays for a collector: nothing.
///
/// **Why a *running* context may be walked at all**, given that the walk frees
/// what no member's field slot accounts for: every reference a compiled frame
/// holds is a counted one, so an object some frame is standing on reads as
/// held from outside the list and is marked live along with everything under
/// it. A borrow reaches its value through a root that some frame counts, so
/// keeping the root is what keeps what the borrow names.
pub(crate) fn collect(list: &LiveList) -> usize {
    reclaim(list)
}

/// Dismantles the members of `list` that nothing outside it can reach, and
/// answers how many that was — [`sweep`]'s walk and [`collect`]'s, which are
/// one walk because they are one question asked at two moments.
///
/// # Why this is not simply "everything still on the list"
///
/// That reading is the one the ADR's sentence invites, and it frees memory
/// somebody is still holding. A `Value` **does** leave a context: `crate::abi`'s
/// `call` answers one to its Rust caller, and a `Core` member that builds an
/// instance answers one to the helper that asked for it. Neither has released
/// it by the time the context goes down. Freeing those would be a
/// use-after-free at [AGENTS.md](/AGENTS.md)'s priority 1, which is
/// never traded, so the sweep frees only what it can *show* is unreachable and
/// leaves anything it cannot.
///
/// # The five walks
///
/// 1. **Snapshot the list**, so the tallies below can be indexed rather than
///    hashed twice.
/// 2. **Tally, per member, how many of its references come from another
///    member's field slot or from an array that member solely owns.** A
///    member whose count that tally does not
///    *exactly* account for is reachable from something outside the list — or
///    is held in a way this walk does not model, which is the same answer.
/// 3. **Mark** those members live, and everything reachable from them: an
///    object a live member points at is live however its own tally reads.
/// 4. **Every unmarked member gains one reference**, the sweep's own, so that
///    walk 5's decrements cannot free a member out from under the walk
///    standing on it — and then drops every reference it holds, through
///    [`crate::release`]'s one worklist. An array or a string a cycle was
///    keeping alive is freed there, by the ordinary path, rather than
///    abandoned.
/// 5. **Every unmarked member is released once**, which is now its last
///    reference. Its slots are already null, so each dismantle is the header
///    alone — but it is a dismantle, so a member's own teardown runs.
///
/// **A reference held through a uniquely owned array is tallied too.** An
/// array a member's field slot holds at a reference count of one has that
/// member as its only owner, so an object in one of its elements is held by
/// that member exactly as a field slot's reference would be — and the same
/// reading carries down into a nested array the outer one is the sole owner
/// of. An array at a higher count is *shared* — with another member, with a
/// copy-on-write sibling, with a Rust caller — and this walk cannot show with
/// whom, so its elements are not tallied at all: the objects behind them read
/// as externally held and are left exactly where they were. That is the only
/// direction this widening is allowed to move anything, and a
/// `debug_assertions` assertion pins it, because a tally that came out *above*
/// an object's reference count would be claiming holds that are not there and
/// would free something somebody still has.
///
/// Both walks read the same edges: an object reachable from a *live* member
/// only through that member's array is marked live by walk 3, or widening the
/// tally would have turned a survivor into garbage rather than the reverse.
///
/// **At teardown no user code runs**, which is what makes the order within a
/// dead cycle unobservable there, as the ADR says: a suspended generator's
/// unwind entry point reaches its context through
/// [`crate::ctx::with_current`], and at a context's own teardown there is
/// none. In flight there is, and walk 5 runs it — the same entry point the
/// ordinary release of an abandoned generator already runs between two
/// statements, on the same terms. What that code must not do is re-enter this
/// walk, and [`LiveList::walking`] is what stops it.
///
/// **What it spends:** one `Vec` and one `HashMap` sized by the number of live
/// objects, plus one pass over their field slots per walk — spent only where
/// the request was about to stop anyway. At teardown, which is already
/// O(live values) by `rule:security/isolate-teardown-is-a-drain-then-a-sweep`;
/// and in flight at a poll whose ceiling the allocator had already crossed,
/// where the alternative was a `FATAL`. A request inside its ceiling pays
/// nothing, per `rule:programs/memory-priority`.
fn reclaim(list: &LiveList) -> usize {
    if list.walking.replace(true) {
        // Reached from inside walk 5's own dismantling — see
        // [`LiveList::walking`]. The outer walk owns every member it condemned
        // and is about to free them; there is nothing here for a second one.
        return 0;
    }
    let _walking = Walking(list);
    let members = list.members();
    if members.is_empty() {
        return 0;
    }
    let mut seat = std::collections::HashMap::with_capacity(members.len());
    for (at, &member) in members.iter().enumerate() {
        // The other half of the stamp, and the exact one: every member of this
        // list says this list is where it belongs. A member that says
        // otherwise was moved between lists by something other than
        // `relink_to_current`, and this sweep is about to decide the fate of an
        // object another context believes is its own.
        #[cfg(debug_assertions)]
        #[expect(unsafe_code, reason = "every member of the list is a live allocation")]
        unsafe {
            assert!(
                std::ptr::eq((*member).owner.get(), std::ptr::from_ref(list)),
                "an object on one context's live list is stamped with another's \
                 — see `crates/nvs-runtime/src/object.rs`'s module docs"
            );
        }
        seat.insert(member, at);
    }

    let mut held = vec![0_usize; members.len()];
    for &member in &members {
        for target in member_targets(member) {
            if let Some(&at) = seat.get(&target) {
                held[at] += 1;
            }
        }
    }

    // The widening's one bound, per this function's docs: every reference the
    // tally counts is a reference that is really there, so a tally can only
    // ever come out at or below the count it is compared against. A tally
    // above it would be double-counting an edge — an array reached from two
    // members, say — and the member it overshot would read as fully internal
    // while something outside the list still held it.
    #[cfg(debug_assertions)]
    for (at, &member) in members.iter().enumerate() {
        assert!(
            held[at] <= refcount(member),
            "the walk's tally counted more references to an object than it \
             has — see `reclaim`'s docs in `crates/nvs-runtime/src/object.rs`"
        );
    }

    let mut live = vec![false; members.len()];
    let mut reachable: Vec<usize> = (0..members.len())
        .filter(|&at| refcount(members[at]) != held[at])
        .collect();
    while let Some(at) = reachable.pop() {
        if std::mem::replace(&mut live[at], true) {
            continue;
        }
        for target in member_targets(members[at]) {
            if let Some(&next) = seat.get(&target)
                && !live[next]
            {
                reachable.push(next);
            }
        }
    }

    let garbage: Vec<*mut ObjHeader> = members
        .into_iter()
        .enumerate()
        .filter_map(|(at, member)| (!live[at]).then_some(member))
        .collect();
    if garbage.is_empty() {
        return 0;
    }
    let reclaimed = garbage.len();
    for &member in &garbage {
        bump(member);
    }
    for &member in &garbage {
        #[expect(
            unsafe_code,
            reason = "the walk above holds one reference to every member of \
                      `garbage`, so none can be freed here; each slot was \
                      initialized by `new` and holds exactly the one reference \
                      being dropped"
        )]
        unsafe {
            for index in 0..field_count(member) {
                crate::release::release_value(field_ptr(member, index).replace(Value::null()));
            }
        }
    }
    for member in garbage {
        #[expect(
            unsafe_code,
            reason = "nothing outside `garbage` referred to this member, and \
                      every reference from inside it was dropped above, so the \
                      walk's own reference is the last one"
        )]
        unsafe {
            crate::release::release_value(Value::from_obj_ptr(member));
        }
    }
    reclaimed
}

/// Lowers [`LiveList::walking`] however the walk it brackets leaves, so that a
/// panic out of a dismantle does not leave a context unable to collect again.
struct Walking<'list>(&'list LiveList);

impl Drop for Walking<'_> {
    fn drop(&mut self) {
        self.0.walking.set(false);
    }
}

/// Empties `list` of whatever [`sweep`] did not free, taking each survivor off
/// it rather than leaving it pointing at a list that is about to go.
///
/// **This is not tidying — it is the whole of why an object may outlive its
/// context at all.** A member's `prev` names the link that points *at* it,
/// which for the first member is the [`LiveList`]'s own head cell; that cell
/// lives in the `Rc` the context holds and dies with the context. A survivor
/// left linked would write eight bytes into that freed allocation the next time
/// its reference count reached zero, which is a heap corruption whose symptom
/// surfaces in whatever allocation the block was reused for.
///
/// A guard rather than a call at the end, so that it also runs on the panicking
/// path out of a sweep — a detached survivor is freed by its own refcount, so
/// the worst this leaves is what the ordinary rules already leave.
struct Detach<'list>(&'list LiveList);

impl Drop for Detach<'_> {
    fn drop(&mut self) {
        let mut member = self.0.head.get();
        while !member.is_null() {
            #[expect(
                unsafe_code,
                reason = "every member of the list is a live allocation: it \
                          leaves the list in `unlink` before it is freed"
            )]
            unsafe {
                let next = (*member).next.get();
                (*member).prev.set(std::ptr::null());
                (*member).next.set(std::ptr::null_mut());
                member = next;
            }
        }
        self.0.head.set(std::ptr::null_mut());
    }
}

/// How many field slots the object at `ptr` has.
///
/// Not `unsafe` to call from this module for [`bump`]'s reason: every caller
/// here already holds a live handle.
fn field_count(ptr: *mut ObjHeader) -> usize {
    #[expect(
        unsafe_code,
        reason = "every caller in this module holds a live reference to `ptr`, \
                  so its descriptor is live too"
    )]
    unsafe {
        (*NvsObj::class_of(ptr)).fields.len()
    }
}

/// This object's current reference count — [`sweep`]'s one reader, which needs
/// it beside a tally rather than as a `bool`.
fn refcount(ptr: *mut ObjHeader) -> usize {
    #[expect(
        unsafe_code,
        reason = "every caller in this module holds a live reference to `ptr`"
    )]
    unsafe {
        (*ptr).refcount.get()
    }
}

/// Every object `ptr` holds a reference to: one per field slot that names an
/// object, plus one per element of an array it is the sole owner of, in slot
/// order and then in element order.
///
/// This is [`sweep`]'s one reader of the object graph, used by both its tally
/// and its mark walk so that the two agree on what an edge is. Collected
/// rather than borrowed so that the tally may be written while this is read;
/// a class's slot count is small, an array's walk is one pass, and this runs
/// at teardown alone.
///
/// **A reference count of one is what makes an array's elements countable**,
/// and the check is exact rather than conservative in the other direction: the
/// reference this walk just found *is* that one owner, so nothing else can be
/// holding the array and no other member's walk can reach it. That also bounds
/// the descent — a uniquely owned array cannot contain itself, since being its
/// own element would be a second reference — so the arrays reached from one
/// object form a tree and the worklist below drains.
fn member_targets(ptr: *mut ObjHeader) -> Vec<*mut ObjHeader> {
    /// Files one value under whichever walk can use it: an object is an edge,
    /// a solely owned array is more edges to go and read, and anything else
    /// holds no object reference at all.
    fn file(
        value: Value,
        targets: &mut Vec<*mut ObjHeader>,
        arrays: &mut Vec<*mut crate::array::ArrayHeader>,
    ) {
        match value.tag() {
            Some(Tag::Object) => {
                if let Some(target) = value.obj_ptr().filter(|target| !target.is_null()) {
                    targets.push(target);
                }
            }
            Some(Tag::Array) => {
                #[expect(
                    unsafe_code,
                    reason = "the value was read out of a live object's slot or \
                              a live array's element, so its allocation is live"
                )]
                if let Some(array) = value.array_ptr().filter(|array| !array.is_null())
                    && unsafe { crate::array::NvsArray::refcount_of(array) } == 1
                {
                    arrays.push(array);
                }
            }
            _ => {}
        }
    }

    let mut targets = Vec::new();
    let mut arrays = Vec::new();
    for index in 0..field_count(ptr) {
        #[expect(
            unsafe_code,
            reason = "every caller in this module holds a live reference to \
                      `ptr`, and every slot was initialized by `new`"
        )]
        let value = unsafe { *field_ptr(ptr, index) };
        file(value, &mut targets, &mut arrays);
    }
    while let Some(array) = arrays.pop() {
        #[expect(
            unsafe_code,
            reason = "the array is live — a slot or element this walk read \
                      holds the one reference to it — and nothing here \
                      releases a borrowed value"
        )]
        for value in unsafe { crate::array::borrowed_values(array) } {
            file(value, &mut targets, &mut arrays);
        }
    }
    targets
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
/// This is the one place the arithmetic lives, so `nvs-codegen` queries the
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

/// The alignment an [`ObjHeader`] must be written at.
///
/// Published for `nvs-codegen`, which places one in a data section rather than
/// in an allocation and so has to state the alignment [`obj_layout`] would
/// otherwise have handed to `alloc` — [`crate::HEADER_ALIGN`]'s twin one
/// representation over.
pub const OBJ_ALIGN: usize = std::mem::align_of::<ObjHeader>();

/// The bytes a compiled unit writes for an instance that must never be freed —
/// [`crate::immortal_header_bytes`]'s twin one representation over, and this
/// module's § *Decision: an immortal instance is on no list at all* for why it
/// is sound to share one.
///
/// `slots` is one runtime [`Tag`] per field, in slot order. Each slot's tag
/// byte is written and its payload left zero, and so is the `class` word: a
/// payload that is an address is a **relocation** the emitting side fills in,
/// not a number this side could know, since the unit is a relocatable object
/// before it is a loaded one
/// (`rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`).
/// The list links stay zero, which is the state "on no list" is spelled in.
///
/// Host byte order, for [`crate::immortal_header_bytes`]'s reason exactly: this
/// JIT compiles for the machine it runs on.
#[must_use]
pub fn immortal_object_bytes(slots: &[Tag]) -> Vec<u8> {
    let mut bytes = vec![0_u8; field_offset(slots.len())];
    bytes[OBJ_REFCOUNT_OFFSET..OBJ_REFCOUNT_OFFSET + std::mem::size_of::<usize>()]
        .copy_from_slice(&crate::IMMORTAL_REFCOUNT.to_ne_bytes());
    for (index, tag) in slots.iter().enumerate() {
        bytes[field_offset(index) + Value::TAG_OFFSET] = *tag as u8;
    }
    bytes
}

/// The allocation shape for an instance with `field_count` slots.
fn obj_layout(field_count: usize) -> Layout {
    let size = field_offset(field_count);
    Layout::from_size_align(size, std::mem::align_of::<ObjHeader>())
        .expect("an object layout is always valid: alignment is a power of two")
}

const _: () = assert!(std::mem::align_of::<Value>() <= std::mem::align_of::<ObjHeader>());

/// An owning handle to one reference of an Novis object.
///
/// Cloning retains, dropping releases — so Rust-side code (helpers, tests, and
/// eventually `nvs-stdlib`) manipulates objects without writing a refcount
/// operation by hand, exactly the way [`crate::NvsStr`] already works.
/// Compiled code instead calls the
/// [`nvs_object_new`]/[`nvs_object_retain`]/[`nvs_object_release`] primitives.
///
/// Neither `Send` nor `Sync`, by construction — see [`crate::string`]'s own
/// docs for the reasoning, which is identical here.
#[repr(transparent)]
pub struct NvsObj {
    ptr: NonNull<ObjHeader>,
}

impl NvsObj {
    /// Allocates a fresh instance of `class` with a reference count of one and
    /// every field slot `null`.
    ///
    /// The slots start `null` rather than uninitialized so that an object
    /// released *before* its constructor finished — a `throw` partway through
    /// one — sweeps well-formed values.
    /// `rule:classes/definite-property-initialization`
    /// makes that state unobservable to Novis code; this only makes it safe to
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
    /// fails — see [`crate::NvsStr::new`] for why that is the honest behaviour
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
        // Every armed slot — a declared `= expr` default, or `rule:classes/an-unwritten-property-read-throws`'s
        // never-written marker — written over the null the slot was just
        // given; see [`ClassDesc::defaults`]. `set_field` releases what it
        // overwrites, which is a `null` here and therefore free.
        for (slot, default) in &desc.defaults {
            object.set_field(*slot, default.materialize());
        }
        object
    }

    /// The allocation half of [`NvsObj::new`]: a fresh instance with every
    /// slot `null` and **no** default applied.
    ///
    /// Its own entry point for exactly one caller — [`nvs_object_clone`],
    /// which overwrites every slot with the source's value and would otherwise
    /// allocate a default string only to release it one line later.
    ///
    /// # Safety
    ///
    /// As [`NvsObj::new`].
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
                next: Cell::new(std::ptr::null_mut()),
                prev: Cell::new(std::ptr::null()),
                #[cfg(debug_assertions)]
                owner: Cell::new(std::ptr::null()),
            });
            let slots = raw.add(FIELDS_OFFSET).cast::<Value>();
            for index in 0..field_count {
                slots.add(index).write(Value::null());
            }
        }
        // The live list this object belongs to, if a context is running at all
        // — see this module's *Decision: every object is on its context's live
        // list*. A Rust caller with none stays unlinked and is freed by its
        // refcount alone.
        let list = crate::ctx::current_live_list();
        if !list.is_null() {
            #[expect(
                unsafe_code,
                reason = "the pointer names the `LiveList` allocation the \
                          current context holds an `Rc` to, and that context \
                          outlives this call; the object is fresh, so it is on \
                          no other list"
            )]
            unsafe {
                (*list).link(ptr.as_ptr());
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
            reason = "an object's descriptor outlives it by `NvsObj::new`'s own \
                      safety contract"
        )]
        unsafe {
            (*self.class()).fields.len()
        }
    }

    /// Reads field slot `index` **without** taking a reference to whatever it
    /// holds — the borrow-shaped read `nvs_ir::InstKind::FieldGet` performs.
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
    /// the way `nvs_ir::InstKind::FieldSet`'s caller-side retain arranges.
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
            reason = "an object's descriptor outlives it by `NvsObj::new`'s own \
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
    /// pointer to [`nvs_object_release`] or [`NvsObj::from_raw`].
    #[must_use]
    pub fn into_raw(self) -> *mut ObjHeader {
        let ptr = self.ptr.as_ptr();
        std::mem::forget(self);
        ptr
    }

    /// Reclaims a reference previously given up by [`NvsObj::into_raw`].
    ///
    /// # Safety
    ///
    /// `ptr` must be a pointer produced by [`NvsObj::into_raw`] (or by
    /// [`nvs_object_new`]) whose reference has not already been released, and
    /// it must not be reclaimed twice.
    ///
    /// # Panics
    ///
    /// If `ptr` is null, which no Novis object pointer ever is.
    #[must_use]
    #[expect(
        unsafe_code,
        reason = "reclaiming a reference is the caller's obligation to state"
    )]
    pub unsafe fn from_raw(ptr: *mut ObjHeader) -> Self {
        Self {
            ptr: NonNull::new(ptr).expect("an Novis object pointer is never null"),
        }
    }

    /// How many owners hold the object `ptr` refers to.
    ///
    /// # Safety
    ///
    /// `ptr` must refer to a live Novis object allocation.
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
    /// `ptr` must refer to a live Novis object allocation.
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

impl Clone for NvsObj {
    fn clone(&self) -> Self {
        bump(self.ptr.as_ptr());
        Self { ptr: self.ptr }
    }
}

impl Drop for NvsObj {
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

impl fmt::Debug for NvsObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NvsObj")
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
/// `ptr` must refer to a live Novis object allocation with more than `index`
/// field slots.
#[expect(
    unsafe_code,
    reason = "the pointee's liveness and the slot's existence are the caller's \
              obligation to state"
)]
unsafe fn field_ptr(ptr: *mut ObjHeader, index: usize) -> *mut Value {
    // A debug-only bound, and deliberately not a release one. `nvs-codegen`
    // resolves the slot at compile time from the same `ir::Class::fields` list
    // that built this object's descriptor (`Classes::define`), so the two
    // cannot disagree about a *count*; what this catches is the narrower case
    // where a static class label names a layout the receiver does not have —
    // a subclass whose slots stopped being a prefix of its ancestor's, or a
    // receiver type the checker got wrong. Carrying it into release would put
    // a load and a compare on every field access, which buys too little for
    // the price when the conformance suite and the fuzz targets run debug.
    // The erased path does not need this: `nvs_object_slot_get` reads the
    // slot off the receiver's own descriptor by name.
    #[cfg(debug_assertions)]
    {
        #[expect(
            unsafe_code,
            reason = "the caller guarantees the allocation is live, so its \
                      descriptor is too"
        )]
        let count = unsafe { (*NvsObj::class_of(ptr)).field_count() };
        assert!(
            index < count,
            "field slot {index} is out of range for a class with {count} slots"
        );
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the slot is inside the allocation, and \
                  `NvsObj::new` initialized every one of them"
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
    let count = header.refcount.get();
    // A constant in a compiled unit's data section is reachable from every
    // core, so its count is read and never written — see this module's
    // § *Decision: an immortal instance is on no list at all*.
    if count == crate::IMMORTAL_REFCOUNT {
        return;
    }
    header.refcount.set(
        count
            .checked_add(1)
            .expect("an Novis object's reference count cannot overflow a usize"),
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
/// `ptr` must refer to a live Novis object allocation whose reference the caller
/// owns.
#[expect(
    unsafe_code,
    reason = "owning the reference is the caller's obligation to state"
)]
pub(crate) unsafe fn drop_one(ptr: *mut ObjHeader) -> bool {
    #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
    let header = unsafe { &*ptr };
    let count = header.refcount.get();
    // [`bump`]'s compare, and the same sentence: an immortal constant is never
    // the last reference because it is never a reference this side owns.
    if count == crate::IMMORTAL_REFCOUNT {
        return false;
    }
    let remaining = count - 1;
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
/// `root` must refer to a live Novis object allocation whose reference the
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
///   0 (`nvs_ir::lower::generator::lower_generator_unwind` owns why), so it
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
/// `ptr` must refer to an Novis object allocation whose reference count reached
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
        // The other end of the pair `NvsObj::alloc` opened: an object leaves
        // its context's live list exactly when it stops existing, so what the
        // list still holds at teardown is what nothing freed. Asserted before
        // the links go, because the stamp is what says whose list this was.
        #[cfg(debug_assertions)]
        assert_linked_where_it_says(ptr);
        unlink(ptr);
        let class = NvsObj::class_of(ptr);
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
/// `ptr` must refer to an Novis object allocation whose reference count reached
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
// none of these can fail, so none of them wears `rule:errors/propagation`'s checked-return
// shape. Every one is `extern "C"` and never `extern "C-unwind"`.

/// Allocates a fresh instance of `class` with a reference count of one, every
/// field slot `null`, and then every slot that declares one holding its
/// default ([`ClassDesc::defaults`]) — `nvs_ir::InstKind::New`'s allocation
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
pub unsafe extern "C" fn nvs_object_new(class: *const ClassDesc) -> *mut ObjHeader {
    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    unsafe {
        NvsObj::new(class).into_raw()
    }
}

/// `rule:classes/clone-is-shallow`'s `clone`: a fresh instance of the *same* class whose every slot holds
/// what the original's held, with a reference count of one.
///
/// **Shallow, same-heap, single-level** — PHP's own rule, kept exactly. A slot
/// holding an object ends up pointing at that same object from both copies,
/// with one more reference taken, so mutating `$copy->child->name` is visible
/// through the original. That is the whole of what `clone` means; deep copying
/// is `serialize`/`unserialize`'s recursive graph copy, a different operation.
///
/// No hook runs. `rule:classes/two-copy-depths` makes `__clone` one of the magic methods Novis does
/// not have, so this is the entire operation — nothing here can throw, which
/// is why it wears no checked-return shape.
///
/// # Safety
///
/// `ptr` must refer to a live Novis object allocation the caller holds a
/// reference to for the duration of the call.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw object pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_clone(ptr: *mut ObjHeader) -> *mut ObjHeader {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees `ptr` is a live allocation it holds a \
                  reference to, so borrowing it for this copy is sound"
    )]
    let (source, copy) = unsafe {
        let source = NvsObj::from_raw(ptr);
        let copy = NvsObj::alloc(source.class());
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

crate::nvs_helper! {
    /// `nvs_ir::Helper::CloneOperandNotAnObject` — a `clone` whose operand
    /// reached run time as a tag, carrying something [`nvs_object_clone`] has no
    /// instance to copy.
    ///
    /// **Never returns `Ok`.** The tag test in front of it already decided the
    /// answer; this exists to carry PHP's own wording, whose type name is the
    /// tag the operand arrived with and therefore not a compile-time fact — see
    /// that `nvs_ir::ir::Helper` variant. The name is [`Tag::describe`]'s, which
    /// is the Novis type a program would have written rather than PHP's
    /// spelling of it, so a `uint` or a `decimal` is named as itself.
    ///
    /// **It releases `args[0]`**, the one argument in this file a helper owns
    /// rather than borrows. A helper that never returns leaves its caller no
    /// reachable point to release one at, so `nvs_ir::lower` retains a borrowed
    /// operand in front of the call and hands the reference over here.
    fn nvs_clone_not_an_object(_ctx, args: [1]) {
        let given = args[0].tag().map_or("null", Tag::describe);
        let message =
            format!("clone(): Argument #1 ($object) must be of type object, {given} given");
        #[expect(
            unsafe_code,
            reason = "the operand's reference is this call's, retained by the \
                      lowering when the frame did not already hold a fresh one, \
                      and no reachable instruction follows a helper that never \
                      returns"
        )]
        unsafe {
            args[0].release();
        }
        Err(Fault::thrown_as(crate::ThrownClass::Logic, message))
    }
}

/// Adds a reference — `nvs_ir::InstKind::Retain` for a `Ty::Object` operand.
///
/// # Safety
///
/// `ptr` must refer to a live Novis object allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw object pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_retain(ptr: *mut ObjHeader) {
    if ptr.is_null() {
        return;
    }
    bump(ptr);
}

/// Drops a reference, freeing the object and everything it solely owns if it
/// was the last — `nvs_ir::InstKind::Release` for a `Ty::Object` operand.
///
/// # Safety
///
/// `ptr` must refer to a live Novis object allocation whose reference this
/// caller owns, and must not be released twice.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw object pointer whose ownership the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_release(ptr: *mut ObjHeader) {
    #[expect(unsafe_code, reason = "the caller guarantees it owns the reference")]
    unsafe {
        release_graph(ptr);
    }
}

/// `$obj instanceof Class`, and the type test a typed `catch` clause performs.
///
/// A null `ptr` answers `false`, the same "a null payload *is* `null`"
/// treatment every retain/release primitive here already gives one — and the
/// reason a `catch` dispatch stays safe when `nvs_take_thrown` hands back
/// nothing (see `crate::throwable`).
///
/// # Safety
///
/// `ptr` must be null or refer to a live Novis object allocation, and `class`
/// to a live descriptor.
#[expect(
    unsafe_code,
    reason = "compiled code passes an object pointer and a descriptor pointer \
              whose liveness the signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_instanceof(
    ptr: *const ObjHeader,
    class: *const ClassDesc,
) -> bool {
    if ptr.is_null() {
        return false;
    }
    #[expect(unsafe_code, reason = "the caller guarantees both pointees are live")]
    unsafe {
        (*NvsObj::class_of(ptr)).conforms_to(class)
    }
}

/// [`nvs_object_instanceof`] over a subject whose tag nothing proved — a
/// `mixed`, or a `?Box` no test narrowed, which is the shape `$x instanceof
/// Box` exists to interrogate.
///
/// The subject arrives as a whole [`Value`] by address, the same shape
/// [`nvs_object_slot_get`]'s receiver takes and for the same reason: an
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
pub unsafe extern "C" fn nvs_value_instanceof(
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
        nvs_object_instanceof(ptr, class)
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
pub unsafe extern "C" fn nvs_class_method(
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

/// The one method [`construct`] runs — `rule:classes/no-leading-underscore-identifiers` fixes the spelling.
pub const CONSTRUCTOR: &str = "constructor";

/// `rule:classes/comparable`'s one member, which a class opts into by
/// implementing the interface.
///
/// The spelling's one home: `nvs_types::iter_lib` seeds the declaration,
/// [`ClassDesc::comparer`] carries a `Core` class's native address under it,
/// and `nvs_stdlib::registry::implements_comparable` reads a member roster for
/// it. A second copy is how the three come to disagree about one string.
pub const COMPARE_TO: &str = "compareTo";

/// Builds an instance of `class` by running its own `constructor` — what a
/// native member does where compiled code would emit
/// `nvs_ir::ir::InstKind::New`.
///
/// **Every argument's reference is transferred**, exactly as an ordinary Novis
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
/// `rule:classes/definite-property-initialization`
/// gives every class exactly one nameable constructor, and the caller reads
/// its arity off the same descriptor.
///
/// # Safety
///
/// `class` must refer to a live descriptor whose method table `nvs-codegen`
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
                  `nvs-codegen` fills only with compiled functions of exactly this \
                  signature"
    )]
    let target: crate::abi::NvsFn = unsafe { std::mem::transmute::<*const u8, _>(target) };

    #[expect(unsafe_code, reason = "the caller guarantees the descriptor is live")]
    let object = unsafe { NvsObj::new(class) };
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

crate::nvs_helper! {
    /// The floor under [`nvs_class_method`]: what a call to a method with no
    /// body reaches when the receiver's own class declares no override either.
    ///
    /// An `abstract` method and a bodiless interface method name no compiled
    /// function, so a call resolving to one has no static target to fall back
    /// to — `nvs_ir::ir::InstKind::CallVirtual` passes this instead of a null
    /// pointer, which would turn a compiler bug into a jump to address zero.
    /// Reaching it is an engine fault, never user error: the checker refuses a
    /// concrete class that leaves an interface method unimplemented.
    ///
    /// Takes no arguments *by declaration* — it is called with whatever the
    /// original call site passed, and reads none of them.
    fn nvs_abstract_method(_ctx, _args: [0]) {
        Err(Fault::fatal(
            "internal error: a method with no body was called, and no class in the \
             receiver's chain declared one"
                .to_owned(),
        ))
    }
}

/// The class name of the object at `ptr`, as a fresh Novis string —
/// `Core\Reflect`'s eventual `nameOf`, and what a diagnostic renders.
///
/// # Safety
///
/// `ptr` must refer to a live Novis object allocation.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw object pointer whose liveness the \
              signature cannot express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_class_name(
    ptr: *const ObjHeader,
) -> *mut crate::string::StrHeader {
    #[expect(unsafe_code, reason = "the caller guarantees the allocation is live")]
    let name = unsafe { (*NvsObj::class_of(ptr)).name() };
    crate::string::NvsStr::new(name.as_bytes()).into_raw()
}

/// Reads field slot `index` off the object at `ptr`, **without** retaining
/// what it holds — `nvs_ir::InstKind::FieldGet`'s out-of-line form.
///
/// Compiled code loads the slot inline using [`field_offset`] once
/// `nvs-codegen` emits the arithmetic; this exists so the layout is exercised
/// from Rust and so a `mixed`-typed read has a single entry point.
///
/// # Safety
///
/// `ptr` must refer to a live Novis object allocation whose class has more than
/// `index` field slots.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw object pointer and a slot index that \
              the signature cannot bound"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_field_get(ptr: *mut ObjHeader, index: usize) -> Value {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the allocation is live and the slot is \
                  in range; `NvsObj::new` initialized every slot"
    )]
    unsafe {
        *field_ptr(ptr, index)
    }
}

/// Reads the field *named* `name` off the object at `ptr`, writing what the
/// slot holds to `out` — `rule:types/erased-member-access`'s name-keyed fetch, and `nvs_ir::InstKind::SlotGet`'s whole emission.
///
/// The name arrives as static bytes `nvs-codegen` put in the unit's data
/// section rather than as an [`crate::NvsStr`]: a read through a shape must not
/// cost an allocation, and the name is a compile-time constant on every path
/// that reaches here.
///
/// **Borrows.** `out` receives the slot's value without a retain, exactly as
/// an inline `FieldGet` load does, so a consumer that outlives the receiver
/// owes it the retain — see `nvs_ir::InstKind::SlotGet`.
///
/// `hint` is the slot the static type said the field was at; see
/// [`ClassDesc::field_slot`] for what it buys and when it is wrong.
///
/// The receiver arrives as a whole [`Value`] by address rather than as a bare
/// pointer, because § 4's erased half includes a `mixed` — `rule:types/conversion`'s
/// one unchecked position, whose tag nothing before this proved. The tag is
/// therefore checked here, where the *name* is already checked, and an
/// unchecked untag in compiled code (which would dereference an `int` payload)
/// is what that buys.
///
/// # Errors
///
/// A [`Fault::Thrown`] naming the field and the concrete class when that class
/// has no such field — `rule:types/erased-member-access`'s "checked, catchable throw; never a
/// silent value, never PHP's warning-plus-`null`". Reached only through a
/// widened or erased view, since a field the receiver's own shape lists is
/// proven present.
///
/// A second [`Fault::Thrown`] when the receiver is not an object at all, in
/// PHP's own wording — a `mixed` is the only receiver that reaches it, every
/// other non-object being `E0495` at check time (`rule:php-migration/every-divergence-is-deliberate-and-listed` row 13).
///
/// A third when the slot was never written — `rule:classes/an-unwritten-property-read-throws`, whose storage
/// state is [`Tag::Unset`] and whose only reachable declaration is a
/// `lateinit` property (`rule:classes/lateinit`).
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
pub unsafe extern "C" fn nvs_object_slot_get(
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
    let body = move |ctx: &mut Ctx, _args: &[Value]| -> crate::HelperResult {
        let name = std::str::from_utf8(name)
            .map_err(|_| Fault::fatal("internal error: a field name that is not UTF-8"))?;
        #[expect(
            unsafe_code,
            reason = "the caller guarantees this points at one initialized value"
        )]
        let receiver = unsafe { *receiver };
        read_erased_property_hinted(ctx, receiver, name, hint, AbsentField::Throws)
    };
    #[expect(
        unsafe_code,
        reason = "the caller's contract is exactly `run_helper`'s"
    )]
    unsafe {
        crate::run_helper(ctx, std::ptr::null(), 0, out, body)
    }
}

/// [`nvs_object_slot_get`] with exactly one of its refusals replaced: a name
/// the receiver's concrete class does not carry answers `null` instead of
/// throwing. That is the read `rule:types/shape-type`'s optional field needs
/// when it is reached under a guard — `??`, `isset` or `empty`, the spellings
/// that ask whether a key is there without wanting the throw absence
/// otherwise raises. Which of the two a site wants is
/// `nvs_ir::InstKind::SlotGet`'s to carry; nothing decides it here.
///
/// **Only that one refusal moves.** A receiver that is not an object, and a
/// slot that was never written (`rule:classes/an-unwritten-property-read-throws`), throw exactly as they do
/// on the throwing path: neither is a question about whether the key is
/// present, so answering `null` for either would swallow a fault the guard
/// never asked about.
///
/// **Borrows**, the `hint`, and the receiver arriving as a whole [`Value`] by
/// address are all [`nvs_object_slot_get`]'s, for its reasons.
///
/// # Errors
///
/// [`nvs_object_slot_get`]'s non-object-receiver and [`Tag::Unset`] throws.
/// Its missing-field throw is this function's `null`.
///
/// # Safety
///
/// [`nvs_object_slot_get`]'s contract, unchanged.
#[expect(
    unsafe_code,
    reason = "compiled code passes a value by address and a static byte \
              range, neither of which the signature can bound"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_slot_optional_get(
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
    let body = move |ctx: &mut Ctx, _args: &[Value]| -> crate::HelperResult {
        let name = std::str::from_utf8(name)
            .map_err(|_| Fault::fatal("internal error: a field name that is not UTF-8"))?;
        #[expect(
            unsafe_code,
            reason = "the caller guarantees this points at one initialized value"
        )]
        let receiver = unsafe { *receiver };
        read_erased_property_hinted(ctx, receiver, name, hint, AbsentField::Null)
    };
    #[expect(
        unsafe_code,
        reason = "the caller's contract is exactly `run_helper`'s"
    )]
    unsafe {
        crate::run_helper(ctx, std::ptr::null(), 0, out, body)
    }
}

/// `$x is {path: string}`'s presence question: **is `name` readable off this
/// receiver right now**, answered as a `bool` and never as a throw.
///
/// True exactly when [`nvs_object_slot_get`] over the same receiver and the
/// same name would answer a value. The three states that make it false are the
/// three that function throws on — a receiver holding no object, a concrete
/// class carrying no field of that name, and a slot that was never written
/// ([`Tag::Unset`]) — and they collapse into one answer because the caller is
/// `rule:types/type-test`'s shape walk, which declines on all three alike. Both
/// read their answer off [`slot_state`], so the read and the presence question
/// cannot disagree about where a field is. An object tag over a null pointer
/// answers `false` as well, where the read raises the engine's own fatal: a
/// total operator has nowhere to report one, and any read of that receiver
/// still does.
///
/// **No `ctx`, no `out` and no status.** `is` is total, so this raises nothing,
/// allocates nothing, reaches no safepoint and borrows its receiver — the shape
/// walk pays one call and, on the common case, one name comparison per field,
/// which is `rule:types/type-test`'s stated O(n).
///
/// `hint` is [`nvs_object_slot_get`]'s slot hint, for its reason.
///
/// # Safety
///
/// `receiver` must point at one initialized [`Value`] its caller still owns,
/// and `name`/`len` must describe initialized bytes that live for the call.
#[expect(
    unsafe_code,
    reason = "compiled code passes a value by address and a static byte \
              range, neither of which the signature can bound"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_slot_probe(
    receiver: *const Value,
    name: *const u8,
    len: usize,
    hint: usize,
) -> bool {
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the byte range is initialized and outlives \
                  this call"
    )]
    let name = unsafe { std::slice::from_raw_parts(name, len) };
    let Ok(name) = std::str::from_utf8(name) else {
        return false;
    };
    #[expect(
        unsafe_code,
        reason = "the caller guarantees this points at one initialized value"
    )]
    let receiver = unsafe { *receiver };
    let Some(ptr) = receiver.obj_ptr() else {
        return false;
    };
    if ptr.is_null() {
        return false;
    }
    #[expect(
        unsafe_code,
        reason = "the caller guarantees the allocation is live, so its descriptor \
                  is too"
    )]
    let desc = unsafe { &*NvsObj::class_of(ptr) };
    #[expect(
        unsafe_code,
        reason = "`desc` is this object's own descriptor, which is `slot_state`'s \
                  whole contract"
    )]
    let state = unsafe { slot_state(desc, ptr, name, hint) };
    matches!(state, SlotState::Present(_))
}

/// `$issue->path = "x";` — [`nvs_object_slot_get`]'s write half, and
/// `rule:types/erased-member-access`'s whole
/// write rule: the slot is found by **name** on the receiver's own descriptor,
/// the incoming value is checked against what that class declares the field to
/// hold, and **no field is ever created** — a name the concrete class does not
/// carry is the same catchable throw a read raises, never a new slot.
///
/// **Borrows.** Unlike [`nvs_object_field_set`], which takes over its caller's
/// reference, this retains what it stores and leaves the caller's own alone:
/// the write can throw *after* its operands are in hand, and a transferred
/// reference on that edge has no owner left to release it. `nvs_ir::lower`
/// stages a freshly-built value as an ordinary temporary instead, which both
/// exits already sweep — see `nvs_ir::InstKind::SlotSet`.
///
/// What the slot held is released, so the caller emits no read-back-and-drop
/// pair the way an inline `FieldSet` needs one.
///
/// `hint` is the slot the static type said the field was at, exactly as in
/// [`nvs_object_slot_get`]; `out` receives a null and exists only because
/// [`crate::run_helper`] writes one.
///
/// # Errors
///
/// A [`Fault::Thrown`] when the concrete class has no such field, and a second
/// when it has one whose declared type does not admit this value's tag — the
/// case a *widened* view creates, since `rule:types/shape-type` checks a shape's field
/// types by ordinary assignability and a shape value is aliased rather than
/// copied. See [`ClassDesc::field_tags`] for the granularity of that check and
/// this module's docs for what it does not catch.
///
/// A third when the receiver is not an object at all, in PHP's own wording —
/// [`nvs_object_slot_get`]'s own third, and reachable for the same one reason.
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
pub unsafe extern "C" fn nvs_object_slot_set(
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
    let body = move |ctx: &mut Ctx, _args: &[Value]| -> crate::HelperResult {
        let name = std::str::from_utf8(name)
            .map_err(|_| Fault::fatal("internal error: a field name that is not UTF-8"))?;
        #[expect(
            unsafe_code,
            reason = "the caller guarantees each points at one initialized value"
        )]
        let (receiver, value) = unsafe { (*receiver, *value) };
        write_erased_property(ctx, receiver, name, hint, value)
    };
    #[expect(
        unsafe_code,
        reason = "the caller's contract is exactly `run_helper`'s"
    )]
    unsafe {
        crate::run_helper(ctx, std::ptr::null(), 0, out, body)
    }
}

/// `$obj->$key` —
/// `rule:types/property-key-access`'s keyed read, which is [`nvs_object_slot_get`] with the field name
/// arriving as a **value** rather than as a static byte range.
///
/// § 5 decided that these are one lookup and not two: a key *is* a name, so
/// what the access needs is exactly the by-name search on the receiver's own
/// descriptor that `rule:types/erased-member-access` already performs, and every rule that read
/// states holds here unchanged — the same catchable throw for a name the
/// concrete class does not carry, the same `rule:classes/an-unwritten-property-read-throws` refusal for a slot
/// never written, the same borrow. There is no `hint`: the caller has no static
/// name to have taken a slot position from, which is the whole of what makes
/// this access keyed.
///
/// **A hooked property is read through its `get` hook**, as the write side
/// reaches its `set`: the accessor is found on the descriptor
/// ([`ClassDesc::hook_row`]) and run with the `Ctx` this helper is already
/// handed. It is owned by [`read_erased_property_hinted`] rather than here, so
/// every spelling of an erased read answers the hook at once — this, the
/// statically named one, and the optional form — which is the reason § 5 routes
/// a key through that path rather than answering it in a way of its own.
///
/// # Errors
///
/// [`nvs_object_slot_get`]'s, which are its own documentation's, plus a
/// [`Fault::fatal`] where the key is not a string at all — unreachable from
/// compiled code, since `property<T>` erases to `nvs_ir`'s `Ty::Str`.
///
/// # Safety
///
/// [`nvs_object_slot_get`]'s, with `key` in place of `name`/`len`: it must
/// point at one initialized [`Value`] its caller still owns.
#[expect(
    unsafe_code,
    reason = "compiled code passes three values by address, none of which the \
              signature can bound"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_key_get(
    ctx: *mut Ctx,
    receiver: *const Value,
    key: *const Value,
    out: *mut Value,
) -> i32 {
    let body = move |ctx: &mut Ctx, _args: &[Value]| -> crate::HelperResult {
        #[expect(
            unsafe_code,
            reason = "the caller guarantees each points at one initialized value"
        )]
        let (receiver, key) = unsafe { (*receiver, *key) };
        let name = key_name(&key)?;
        read_erased_property(ctx, receiver, name)
    };
    #[expect(
        unsafe_code,
        reason = "the caller's contract is exactly `run_helper`'s"
    )]
    unsafe {
        crate::run_helper(ctx, std::ptr::null(), 0, out, body)
    }
}

/// `$obj->$key = v;` — [`nvs_object_key_get`]'s write half, and `rule:types/property-key-access`'s
/// checked erased store: [`nvs_object_slot_set`] with the field name arriving
/// as a value, over the same [`write_erased_property`] a reflective write
/// reaches.
///
/// # Errors
///
/// [`nvs_object_slot_set`]'s, plus [`nvs_object_key_get`]'s fatal for a key
/// that is not a string.
///
/// # Safety
///
/// [`nvs_object_slot_set`]'s, with `key` in place of `name`/`len`.
#[expect(
    unsafe_code,
    reason = "compiled code passes four values by address, none of which the \
              signature can bound"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_key_set(
    ctx: *mut Ctx,
    receiver: *const Value,
    key: *const Value,
    value: *const Value,
    out: *mut Value,
) -> i32 {
    let body = move |ctx: &mut Ctx, _args: &[Value]| -> crate::HelperResult {
        #[expect(
            unsafe_code,
            reason = "the caller guarantees each points at one initialized value"
        )]
        let (receiver, key, value) = unsafe { (*receiver, *key, *value) };
        let name = key_name(&key)?;
        write_erased_property(ctx, receiver, name, 0, value)
    };
    #[expect(
        unsafe_code,
        reason = "the caller's contract is exactly `run_helper`'s"
    )]
    unsafe {
        crate::run_helper(ctx, std::ptr::null(), 0, out, body)
    }
}

/// The member name an `rule:types/property-key-access` key holds, as the `&str` both halves of the
/// access are keyed on.
///
/// A key erases to a `string` and nothing else can be written where one is
/// expected, so a non-string here is a miscompilation rather than a program
/// error — [`Fault::fatal`], which no `catch` sees, and not the throw a missing
/// field raises.
fn key_name(key: &Value) -> Result<&str, Fault> {
    key.as_text()
        .ok_or_else(|| Fault::fatal("internal error: a property key that is not a string"))
}

/// `rule:types/erased-member-access`'s erased *read*, factored out of [`nvs_object_slot_get`] so
/// that `rule:types/property-key-access`'s keyed read is the same lookup and not a second copy of
/// its rules — the shape [`write_erased_property`] already has on the write
/// side, and for its reason.
///
/// `hint` is the caller's, and a keyed access passes `0` because it has none.
///
/// # Errors
///
/// [`nvs_object_slot_get`]'s, which its own documentation owns.
fn read_erased_property(ctx: &mut Ctx, receiver: Value, name: &str) -> crate::HelperResult {
    read_erased_property_hinted(ctx, receiver, name, 0, AbsentField::Throws)
}

/// What a read answers for a name the receiver's concrete class does not
/// carry, which is the one thing [`nvs_object_slot_get`] and
/// [`nvs_object_slot_optional_get`] disagree about.
///
/// It is a presence question and nothing else: every other way a read can
/// fail throws under both, so this decides one `else` arm rather than a mode
/// the whole lookup runs in.
#[derive(Clone, Copy)]
enum AbsentField {
    /// `rule:types/erased-member-access`'s throw naming the field and the concrete class.
    Throws,
    /// `null`, for a read a `??`, `isset` or `empty` guard already covers.
    Null,
}

/// [`read_erased_property`] with the caller's slot hint — see
/// [`ClassDesc::field_slot`] for what one buys — and its answer for a name
/// the class does not carry.
///
/// **A hooked property is read through its `get` hook**, found on the
/// descriptor exactly as [`write_erased_property`] finds the `set` one and
/// called with the same receiver-in-slot-0 convention a compiled site uses.
/// That is what makes `rule:types/erased-member-access`'s "hooks behave here
/// exactly as they do anywhere" true of the read as well: an access spelling
/// that reached storage past a `get` would let anything holding a `mixed` see
/// a backing slot the class publishes nothing about.
///
/// # Errors
///
/// [`read_erased_property`]'s, less the missing-field throw under
/// [`AbsentField::Null`], plus [`Fault::Pending`] where a `get` hook throws.
fn read_erased_property_hinted(
    ctx: &mut Ctx,
    receiver: Value,
    name: &str,
    hint: usize,
    absent: AbsentField,
) -> crate::HelperResult {
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
    let desc = unsafe { &*NvsObj::class_of(ptr) };
    // Before the slot, and for [`write_erased_property`]'s reason in the other
    // direction: where a property declares a `get`, the slot is that hook's to
    // read and the value it answers is the property's.
    if let Some(row) = desc.hook_row(name, false) {
        return crate::dispatch::call_at(ctx, receiver, row.code, &[]);
    }
    #[expect(
        unsafe_code,
        reason = "`desc` is this object's own descriptor, which is `slot_state`'s \
                  whole contract"
    )]
    match unsafe { slot_state(desc, ptr, name, hint) } {
        SlotState::Present(held) => Ok(held),
        SlotState::Absent => match absent {
            AbsentField::Null => Ok(Value::null()),
            AbsentField::Throws => Err(Fault::thrown(format!(
                "`{}` has no field `{name}`",
                desc.name()
            ))),
        },
        // `rule:classes/an-unwritten-property-read-throws`: a slot that was
        // never written reads as a throw, never as a value standing in for one.
        // A `lateinit` property (`rule:classes/lateinit`) and an absent optional
        // field of a shape are the two declarations that reach the state; the
        // compiled read makes the same refusal from the payload alone,
        // `nvs_ir::lower`'s `emit_never_written_guard` owning that half.
        //
        // On a shape class the state means something the `lateinit` one does
        // not: the key was absent from the subject a hydration read, which is
        // exactly the presence question `??`, `isset` and `empty` ask
        // (`rule:types/shape-type`). So a guarded read of one answers `null`
        // where the same read of a `lateinit` slot still throws — the two are
        // told apart by [`ClassDesc::is_shape`], since no `lateinit` property
        // can be declared on a class only a shape hydration mints.
        SlotState::Unwritten => {
            if matches!(absent, AbsentField::Null) && desc.is_shape() {
                return Ok(Value::null());
            }
            Err(Fault::thrown(format!(
                "`{}`'s property `${name}` is read before it is written",
                desc.name()
            )))
        }
    }
}

/// What the name a reader is keyed on currently is on the receiver's
/// *concrete* class — [`nvs_object_slot_get`], its guarded twin and
/// [`nvs_object_slot_probe`] all read their answers off this one lookup, so a
/// read and the presence question that guards it cannot disagree about where a
/// field is or whether it holds anything.
#[derive(Clone, Copy)]
enum SlotState {
    /// The class carries no field of that name. `rule:types/shape-type`'s width
    /// subtyping is why a static shape does not settle this: the value's own
    /// class need not be the one the receiver was typed as.
    Absent,
    /// It carries one, and the slot was never written — [`Tag::Unset`], the
    /// storage state [`Tag`]'s own docs keep off the roster of values.
    Unwritten,
    /// What the slot holds, borrowed exactly as the slot holds it: no retain,
    /// so a consumer outliving the receiver owes it one.
    Present(Value),
}

/// [`SlotState`] for `name` on the object at `ptr`, trying `hint` first — see
/// [`ClassDesc::field_slot`] for what a hint buys.
///
/// # Safety
///
/// `ptr` must point at one live object whose class is `desc`.
#[expect(
    unsafe_code,
    reason = "the object and its descriptor are a pairing only the caller can \
              establish"
)]
unsafe fn slot_state(desc: &ClassDesc, ptr: *mut ObjHeader, name: &str, hint: usize) -> SlotState {
    let Some(slot) = desc.field_slot(name, hint) else {
        return SlotState::Absent;
    };
    #[expect(
        unsafe_code,
        reason = "the slot came out of this object's own descriptor, so it is \
                  inside the allocation and was initialized by `new`"
    )]
    let held = unsafe { *field_ptr(ptr, slot) };
    if held.tag() == Some(Tag::Unset) {
        return SlotState::Unwritten;
    }
    SlotState::Present(held)
}

/// `rule:types/erased-member-access`'s erased write and
/// `rule:classes/property-observer-pipeline`'s observer step
/// over it — the whole of what [`nvs_object_slot_set`] does, written here so
/// that a *reflective* write reaches the same code rather than a second copy of
/// its rules. `nvs_stdlib::reflect`'s `Core\Reflect\ClassInfo::set` is the
/// other caller, and `rule:security/reflection-enforces-visibility`'s "fails the same way an ordinary write
/// would" is why it is a caller rather than a reimplementation — the same
/// reading `crate::dispatch::call_erased_method` already carries for the call
/// half.
///
/// **The observer step is here rather than at the call site**, unlike every
/// other property write in the language. `rule:classes/property-observer-costs-nothing-when-unused` answers "does this class
/// implement `PropertyObserver`" from the *declaration*, so where the
/// receiver's class is known `nvs_ir::lower` emits `onPropertySet` beside the
/// `FieldSet` itself and nothing reaches this function. An erased receiver has
/// no such answer to emit from, and § 3 says the observer sees **every** write
/// — so the question is simply asked later, of the descriptor the value
/// arrived with, once the store has committed. A class that implements nothing
/// pays one name comparison against a conformance list `new` already built.
///
/// **A hooked property is written through its `set` hook here too.** Where the
/// receiver's class is known, such a write lowers to a call to the hook and
/// never reaches a slot store at all; an erased receiver has no class to lower
/// against, so the hook is found on the descriptor instead
/// ([`ClassDesc::hook_row`]) and called with the same receiver-in-slot-0
/// convention a compiled site uses. `rule:classes/property-hooks` makes every
/// write of a hooked property a call to its `set` *at every access spelling
/// alike*, and writing past one would make a class's own invariant advisory
/// for anything holding a `mixed` — the hole this closes, and the reason the
/// value is judged against the hook's declared parameter rather than against
/// the field's declared tag.
///
/// The hook's body commits the value, so nothing below it runs for that write:
/// its store to the backing slot is an ordinary in-class `FieldSet`, carrying
/// the observer step the same lowering gives every other write inside the
/// class. Where no hook is declared the value written is exactly the value
/// committed, and it is the one the observer is told about.
///
/// # Errors
///
/// [`nvs_object_slot_set`]'s throws, which are its own documentation's, plus
/// [`Fault::Pending`] when the observer itself throws: § 3's "a throwing
/// `onPropertySet` still fails the overall write", even though the store has
/// already happened.
pub fn write_erased_property(
    ctx: &mut Ctx,
    receiver: Value,
    name: &str,
    hint: usize,
    value: Value,
) -> Result<Value, Fault> {
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
        reason = "the caller owns a reference to this object, so the allocation \
                  is live and its descriptor is too"
    )]
    let desc = unsafe { &*NvsObj::class_of(ptr) };
    // Before the slot, because for a hooked property the slot is the hook's
    // to write: this door is the access spelling that had no class to lower
    // against, not a second way of storing.
    if let Some(row) = desc.hook_row(name, true) {
        let callee = format!("`{}::${name}::set`", desc.name());
        // The check converts as well as refuses — a widened `int` has to reach
        // the hook as the `float` it declares — so it runs over this frame's
        // own copy, exactly as `dispatch::call_erased_method_from`'s does.
        let mut passed = [value];
        crate::closure::check_param_tags(&callee, row.param_tags, &mut passed)?;
        let answered = crate::dispatch::call_at(ctx, receiver, row.code, &passed)?;
        #[expect(
            unsafe_code,
            reason = "the hook's return is a fresh reference this frame owns, \
                      and a `set` hook returns nothing for anyone to read"
        )]
        unsafe {
            answered.release();
        }
        return Ok(Value::null());
    }
    let Some(slot) = desc.field_slot(name, hint) else {
        return Err(Fault::thrown(format!(
            "`{}` has no field `{name}`",
            desc.name()
        )));
    };
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
    // Read before the store, so nothing borrows the descriptor across the call
    // below. `PropertyObserver` is spelled as a literal because its one home,
    // `nvs_hir::interfaces::PROPERTY_OBSERVER`, is above this crate.
    let observed = desc.conforms_to_name("PropertyObserver");
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
    if !observed {
        return Ok(Value::null());
    }
    let told = Value::str(crate::NvsStr::new(name.as_bytes()));
    let answered =
        crate::dispatch::call_erased_method(ctx, receiver, "onPropertySet", &[told, value]);
    #[expect(
        unsafe_code,
        reason = "this frame made that string and still owns it — the call \
                  retained what it passed on and the callee's exit sweep \
                  released that reference, not this one"
    )]
    unsafe {
        told.release();
    }
    #[expect(
        unsafe_code,
        reason = "the observer's return is a fresh reference this frame owns, \
                  and § 3 makes it a `void` nothing reads"
    )]
    unsafe {
        answered?.release();
    }
    Ok(Value::null())
}

/// Overwrites field slot `index` on the object at `ptr`, releasing whatever it
/// held and taking over `value`'s reference —
/// `nvs_ir::InstKind::FieldSet`'s out-of-line form.
///
/// # Safety
///
/// `ptr` must refer to a live Novis object allocation whose class has more than
/// `index` field slots, and `value` must own the reference it transfers.
#[expect(
    unsafe_code,
    reason = "compiled code passes a raw object pointer, a slot index the \
              signature cannot bound, and a value whose ownership it cannot \
              express"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nvs_object_field_set(ptr: *mut ObjHeader, index: usize, value: Value) {
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
    use crate::array::NvsArray;
    use crate::counting_alloc;
    use crate::string::NvsStr;

    /// One method row of a shape nothing here is testing: a public, nullary
    /// method. The tests that are about the shape write it out.
    fn row(name: &str, code: *const u8) -> MethodRow {
        MethodRow {
            name: name.to_owned(),
            code,
            arity: 0,
            param_tags: 0,
            param_names: Vec::new(),
            param_types: Vec::new(),
            public: true,
            protected: false,
            native: false,
        }
    }

    /// A table with `Animal`, `Dog extends Animal`, and a `Greets` interface
    /// `Dog` implements — the shape `examples/objects.nvs` needs.
    fn hierarchy() -> (ClassTable, ClassId, ClassId, ClassId) {
        let mut table = ClassTable::new();
        let greets = table.define("Greets", &[] as &[&str], &[]);
        let animal = table.define("Animal", &["name"], &[]);
        let dog = table.define("Dog", &["name", "breed"], &[animal, greets]);
        (table, animal, dog, greets)
    }

    /// One field of a shape's wire contract: the `index`th slot, read under
    /// its own name, holding whatever `ty` says.
    fn shape_field(index: usize, name: &str, ty: CodecTy) -> CodecField {
        CodecField {
            key: name.to_owned(),
            // A shape lays its slots out in the sorted field order its codec
            // is built in, so one index answers for both — `ShapeCodec`.
            slot: index,
            param: index,
            ty,
            element: None,
            class: None,
            cases: None,
            shape: None,
            nullable: false,
            required: true,
            default: None,
        }
    }

    /// The bytes `nvs-codegen` puts in a unit's data section for one folded
    /// literal, in a `Vec<u64>` so the header lands at the alignment it has
    /// there, with the relocated words filled in by hand — this is the only
    /// way this crate's own tests can hold an immortal instance, since nothing
    /// here constructs one. `crate::string`'s `immortal_unit` is the same
    /// helper one representation over.
    fn immortal_unit(class: *const ClassDesc, text: *mut crate::StrHeader) -> Vec<u64> {
        assert!(OBJ_ALIGN <= std::mem::size_of::<u64>());
        let bytes = immortal_object_bytes(&[Tag::Str]);
        let mut words = vec![0_u64; bytes.len().div_ceil(std::mem::size_of::<u64>())];
        #[expect(
            unsafe_code,
            reason = "`words` owns `bytes.len()` bytes at a u64's alignment, \
                      which is the whole point of allocating it as one; the \
                      view ends with this function"
        )]
        let view =
            unsafe { std::slice::from_raw_parts_mut(words.as_mut_ptr().cast::<u8>(), bytes.len()) };
        view.copy_from_slice(&bytes);
        // The two relocations `emit` writes: the class word, and the one
        // slot's payload. The tag byte `immortal_object_bytes` already wrote is
        // what says which representation that payload is.
        view[OBJ_CLASS_OFFSET..OBJ_CLASS_OFFSET + std::mem::size_of::<usize>()]
            .copy_from_slice(&class.expose_provenance().to_ne_bytes());
        view[field_offset(0) + Value::BITS_OFFSET
            ..field_offset(0) + Value::BITS_OFFSET + std::mem::size_of::<u64>()]
            .copy_from_slice(&text.expose_provenance().to_ne_bytes());
        words
    }

    /// `rule:core-classes/html-literal`'s folded literal, from the side this
    /// crate owns: an immortal instance costs nothing to retain or release, is
    /// never freed however many times it is released, and — the half that keeps
    /// the plain `Cell` sound — is never *written*.
    ///
    /// It is also on no list, which is what this module's § *Decision: an
    /// immortal instance is on no list at all* promises a teardown sweep: the
    /// links `immortal_object_bytes` leaves null are the state `unlink` reads
    /// as "on no list", and nothing here ever links it into one.
    #[test]
    fn an_immortal_instance_is_never_written_freed_or_allocated_for() {
        let (table, animal, _dog, _greets) = hierarchy();
        let text = NvsStr::new(b"<b>hi</b>").into_raw();
        let mut unit = immortal_unit(table.desc(animal), text);
        let ptr = unit.as_mut_ptr().cast::<ObjHeader>();

        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        unsafe {
            assert_eq!(NvsObj::refcount_of(ptr), crate::IMMORTAL_REFCOUNT);
            assert_eq!((*ptr).next.get(), std::ptr::null_mut());

            let before = counting_alloc::allocated_bytes();
            nvs_object_retain(ptr);
            nvs_object_release(ptr);
            nvs_object_release(ptr);
            // One more release than there were references: an immortal cannot
            // be over-released, which is what lets compiled code transfer one
            // into an array or a `Value` with no special case.
            nvs_object_release(ptr);
            assert_eq!(counting_alloc::allocated_bytes() - before, 0);
            assert_eq!(NvsObj::refcount_of(ptr), crate::IMMORTAL_REFCOUNT);

            // The slot survives every one of those, because nothing reached
            // the field sweep that would have released it.
            assert_eq!(NvsStr::bytes_of(text), b"<b>hi</b>");
            crate::nvs_str_release(text);
        }
    }

    #[test]
    fn a_shape_codec_keeps_its_address_and_answers_its_nested_class() {
        let (mut table, animal, _dog, _greets) = hierarchy();
        let first = table.define_shape_codec(
            vec![shape_field(0, "n", CodecTy::Int)],
            vec![std::ptr::null()],
            vec![std::ptr::null()],
        );
        let nested = table.desc(animal);
        let second = table.define_shape_codec(
            vec![
                shape_field(0, "pet", CodecTy::Class),
                shape_field(1, "seen", CodecTy::Bool),
            ],
            vec![nested, std::ptr::null()],
            vec![std::ptr::null(), std::ptr::null()],
        );
        // A field that is itself a shape decodes against the contract beside
        // it, which is the pointer a class label cannot carry.
        let third = table.define_shape_codec(
            vec![shape_field(0, "inner", CodecTy::Shape)],
            vec![std::ptr::null()],
            vec![first],
        );
        // Every later definition — of a class or of another contract — leaves
        // all three addresses where compiled code was told they are.
        table.define("Later", &["x"], &[]);
        table.define_shape_codec(Vec::new(), Vec::new(), Vec::new());
        #[expect(
            unsafe_code,
            reason = "both pointers came from this table, which owns its contracts for its whole life"
        )]
        let (first, second, third) = unsafe { (&*first, &*second, &*third) };
        assert_eq!(first.fields().len(), 1);
        assert_eq!(first.fields()[0].key, "n");
        assert!(first.class(0).is_none());
        assert_eq!(second.class(0), Some(nested));
        assert!(second.class(1).is_none(), "a `bool` field names no class");
        assert!(
            second.class(2).is_none(),
            "and neither does no field at all"
        );
        assert_eq!(third.shape(0), Some(std::ptr::from_ref(first)));
        assert!(
            second.shape(0).is_none(),
            "a class field names a descriptor, not a contract"
        );
    }

    #[test]
    fn a_shape_codec_rides_an_argument_slot_the_way_a_descriptor_does() {
        let mut table = ClassTable::new();
        let codec = table.define_shape_codec(
            vec![shape_field(0, "n", CodecTy::Int)],
            vec![std::ptr::null()],
            vec![std::ptr::null()],
        );
        let slot = Value::shape_codec(codec);
        assert_eq!(slot.as_shape_codec(), Some(codec));
        // The convention is one `Tag::Null` byte over an address, so nothing
        // sweeping this slot can mistake it for a heap reference.
        assert!(!slot.tag().is_some_and(Tag::is_refcounted));
        assert_eq!(Value::null().as_shape_codec(), None);
    }

    #[test]
    fn the_layout_constants_describe_the_real_header() {
        assert_eq!(OBJ_REFCOUNT_OFFSET, 0);
        assert_eq!(OBJ_CLASS_OFFSET, std::mem::size_of::<usize>());
        // Four words: the two compiled code reads, and the two the live list
        // threads through — plus the debug build's owner stamp, which is the
        // one thing about this layout the two profiles disagree on. See this
        // module's *Layout*.
        let words = if cfg!(debug_assertions) { 5 } else { 4 };
        assert_eq!(FIELDS_OFFSET, words * std::mem::size_of::<usize>());
        assert_eq!(FIELD_STRIDE, 16);
        assert_eq!(field_offset(0), FIELDS_OFFSET);
        assert_eq!(field_offset(3), FIELDS_OFFSET + 48);
    }

    #[test]
    fn a_fresh_object_has_one_reference_and_null_fields() {
        let (table, _animal, dog, _greets) = hierarchy();
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { NvsObj::new(table.desc(dog)) };
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
        let base: *const u8 = (nvs_object_new as *const ()).cast();
        let over: *const u8 = (nvs_object_retain as *const ()).cast();
        let miss: *const u8 = (nvs_object_release as *const ()).cast();
        table.set_methods(animal, vec![row("describe", base)]);
        table.set_methods(dog, vec![row("describe", over), row("bark", base)]);

        #[expect(unsafe_code, reason = "the table outlives every borrow here")]
        unsafe {
            assert_eq!((*table.desc(animal)).method("describe"), Some(base));
            assert_eq!((*table.desc(dog)).method("describe"), Some(over));
            assert_eq!((*table.desc(dog)).method_count(), 2);
            assert_eq!((*table.desc(animal)).method("bark"), None);

            let name = "describe";
            assert_eq!(
                nvs_class_method(table.desc(dog), name.as_ptr(), name.len(), miss),
                over
            );
            let absent = "bark";
            assert_eq!(
                nvs_class_method(table.desc(animal), absent.as_ptr(), absent.len(), miss),
                miss
            );
            assert_eq!(
                nvs_class_method(std::ptr::null(), name.as_ptr(), name.len(), miss),
                miss
            );
        }
    }

    #[test]
    fn a_method_row_carries_the_callees_declared_shape_past_the_sort() {
        // The row is what a caller holding only tagged values reads before it
        // fills the callee's slots — `MethodRow` owns why. What is under test
        // is that the shape survives the name sort and the override dedup:
        // both are written over the name alone, so a row that lost its arity
        // or its `public` bit there would still answer the right address.
        let mut table = ClassTable::new();
        let id = table.define("Greeter", &[] as &[&str], &[]);
        let own: *const u8 = (nvs_object_new as *const ()).cast();
        let inherited: *const u8 = (nvs_object_retain as *const ()).cast();
        table.set_methods(
            id,
            vec![
                MethodRow {
                    name: "greet".to_owned(),
                    code: own,
                    arity: 2,
                    // `string` then `int`, parameter 0 in the low nibble.
                    param_tags: 0x25,
                    param_names: Vec::new(),
                    param_types: Vec::new(),
                    public: true,
                    protected: false,
                    native: false,
                },
                // The superclass's own `greet`, appended after it exactly as
                // `nvs_types::layout` flattens a chain — the override wins.
                MethodRow {
                    name: "greet".to_owned(),
                    code: inherited,
                    arity: 0,
                    param_tags: 0,
                    param_names: Vec::new(),
                    param_types: Vec::new(),
                    public: false,
                    protected: false,
                    native: false,
                },
                MethodRow {
                    name: "hidden".to_owned(),
                    code: inherited,
                    arity: 0,
                    param_tags: 0,
                    param_names: Vec::new(),
                    param_types: Vec::new(),
                    public: false,
                    protected: false,
                    native: false,
                },
            ],
        );

        #[expect(unsafe_code, reason = "the table outlives every borrow here")]
        unsafe {
            let desc = &*table.desc(id);
            let greet = desc.method_row("greet").expect("the class declares it");
            assert_eq!(greet.code, own);
            assert_eq!(greet.arity, 2);
            assert_eq!(greet.param_tags, 0x25);
            assert!(greet.public);
            assert!(!desc.method_row("hidden").expect("declared").public);
            assert!(desc.method_row("absent").is_none());
        }
    }

    #[test]
    fn a_class_with_no_fields_is_still_a_real_allocation() {
        let mut table = ClassTable::new();
        let marker = table.define("Marker", &[] as &[&str], &[]);
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { NvsObj::new(table.desc(marker)) };
        assert_eq!(object.field_count(), 0);
        assert_eq!(object.refcount(), 1);
    }

    #[test]
    fn cloning_retains_and_dropping_releases() {
        let (table, animal, _dog, _greets) = hierarchy();
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { NvsObj::new(table.desc(animal)) };
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
            let pet = NvsObj::new(table.desc(dog));
            assert!(pet.is_instance_of(table.desc(dog)));
            assert!(pet.is_instance_of(table.desc(animal)));
            assert!(pet.is_instance_of(table.desc(greets)));

            let plain = NvsObj::new(table.desc(animal));
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
            let object = NvsObj::new(table.desc(leaf));
            for ancestor in [leaf, mid, base, named] {
                assert!(object.is_instance_of(table.desc(ancestor)));
            }
        }
    }

    #[test]
    fn setting_a_field_releases_what_it_replaced() {
        let (table, animal, _dog, _greets) = hierarchy();
        let name = NvsStr::new(b"rex");
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { NvsObj::new(table.desc(animal)) };

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
        let name = NvsStr::new(b"cat");
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { NvsObj::new(table.desc(animal)) };
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
            let raw = nvs_object_new(table.desc(animal));
            nvs_object_retain(raw);
            assert_eq!(NvsObj::refcount_of(raw), 2);
            assert!(nvs_object_instanceof(raw, table.desc(animal)));

            nvs_object_field_set(raw, 0, Value::int(7));
            assert_eq!(nvs_object_field_get(raw, 0).as_int(), Some(7));

            let class_name = NvsStr::from_raw(nvs_object_class_name(raw));
            assert_eq!(class_name.as_bytes(), b"Animal");

            nvs_object_release(raw);
            assert_eq!(NvsObj::refcount_of(raw), 1);
            nvs_object_release(raw);
        }
    }

    #[test]
    fn a_field_holding_another_object_keeps_it_alive() {
        let (table, animal, dog, _greets) = hierarchy();
        #[expect(unsafe_code, reason = "the table outlives the objects")]
        unsafe {
            let inner = NvsObj::new(table.desc(animal));
            let outer = NvsObj::new(table.desc(dog));
            outer.set_field(0, Value::object(inner.clone()));
            assert_eq!(inner.refcount(), 2);
            drop(outer);
            assert_eq!(inner.refcount(), 1);
        }
    }

    /// `rule:classes/clone-is-shallow`: `clone` is shallow, same-heap and single-level. The two
    /// halves that matter are that the copy is a *different* allocation and
    /// that a slot holding an object ends up shared, with one more reference,
    /// rather than copied — mutating through one is visible through the other,
    /// which is exactly PHP's rule.
    #[test]
    fn clone_copies_the_slots_and_shares_what_they_point_at() {
        let (table, animal, dog, _greets) = hierarchy();
        #[expect(unsafe_code, reason = "the table outlives every object below")]
        unsafe {
            let shared = NvsObj::new(table.desc(animal));
            let original = NvsObj::new(table.desc(dog));
            original.set_field(0, Value::object(shared.clone()));
            original.set_field(1, Value::str(NvsStr::new(b"name")));
            assert_eq!(shared.refcount(), 2);

            // Through raw pointers rather than `NvsObj::clone`, which would
            // bump the very counts this is measuring.
            let original_ptr = original.into_raw();
            let copy_ptr = nvs_object_clone(original_ptr);
            assert_ne!(
                copy_ptr.cast_const(),
                original_ptr.cast_const(),
                "`clone` must be a second allocation"
            );
            let original = NvsObj::from_raw(original_ptr);
            let copy = NvsObj::from_raw(copy_ptr);
            assert_eq!(copy.class_name(), original.class_name());
            assert_eq!(copy.refcount(), 1);
            // The slot is shared, not copied — a third owner of `shared`.
            assert_eq!(shared.refcount(), 3);
            assert_eq!(copy.field(0).obj_ptr(), original.field(0).obj_ptr());
            assert_eq!(copy.field(1).as_str_bytes(), Some(b"name".as_slice()));

            // Overwriting a slot on the copy leaves the original's alone —
            // "single-level" is the other half of the rule.
            copy.set_field(1, Value::str(NvsStr::new(b"copy")));
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
                let shared = NvsObj::new(table.desc(animal));
                shared.set_field(0, Value::str(NvsStr::new(b"shared")));
                let original = NvsObj::new(table.desc(dog));
                original.set_field(0, Value::object(shared.clone()));
                original.set_field(1, Value::str(NvsStr::new(b"name")));
                drop(shared);

                let raw = original.into_raw();
                let copy = NvsObj::from_raw(nvs_object_clone(raw));
                drop(copy);
                drop(NvsObj::from_raw(raw));
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
                let shared = NvsObj::new(table.desc(animal));
                shared.set_field(0, Value::str(NvsStr::new(b"shared")));

                let mut head = NvsObj::new(table.desc(dog));
                head.set_field(0, Value::object(shared.clone()));
                head.set_field(1, Value::str(NvsStr::new(b"node 0")));
                for depth in 1..512 {
                    let next = NvsObj::new(table.desc(dog));
                    next.set_field(0, Value::object(head));
                    next.set_field(
                        1,
                        Value::str(NvsStr::new(format!("node {depth}").as_bytes())),
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
            let mut head = NvsObj::new(table.desc(link));
            for _ in 1..200_000 {
                let next = NvsObj::new(table.desc(link));
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
        let object = unsafe { NvsObj::new(table.desc(animal)) };
        let rendered = format!("{object:?}");
        assert!(rendered.contains("Animal"), "{rendered}");
        assert!(rendered.contains("refcount: 1"), "{rendered}");
    }

    /// `rule:types/erased-member-access`'s name-keyed fetch: the hint is tried first and is right
    /// where the receiver's shape is the value's own, wrong through a widened
    /// view — and a name the class does not carry answers `None`, which is
    /// what `nvs_object_slot_get` turns into a catchable throw.
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

    /// `nvs_object_slot_probe` answers `true` in exactly the cases the
    /// throwing read hands a value back, which is what lets
    /// `rule:types/type-test`'s shape walk guard a read it then knows cannot
    /// throw. Asserted as an **agreement** over every state a name can be in
    /// rather than case by case, so a probe that grew a rule of its own fails
    /// here while still looking right on each line.
    #[test]
    fn the_slot_probe_answers_true_in_exactly_the_cases_the_read_does_not_throw() {
        let mut table = ClassTable::new();
        let id = table.define("Point", &["x", "y"], &[]);
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { NvsObj::new(table.desc(id)) };
        object.set_field(0, Value::int(3));
        // The state an absent optional shape field and an unwritten `lateinit`
        // property both reach, and the one a fresh object's `null` is not.
        object.set_field(1, Value::unset());
        let receiver = Value::object(object.clone());
        // The read takes one so that a hooked property can run its `get`; no
        // class here declares a hook, so it is never the context a call is made
        // on.
        let mut ctx = Ctx::new(crate::OutputSink::Buffer(Vec::new()));

        // Present, present through a hint that misses, unwritten, a name the
        // class does not carry, and a receiver holding no object at all.
        let subjects = [
            (receiver, "x", 0_usize),
            (receiver, "x", 9),
            (receiver, "y", 1),
            (receiver, "nope", 0),
            (Value::int(7), "x", 0),
        ];
        for (subject, name, hint) in subjects {
            #[expect(
                unsafe_code,
                reason = "the receiver and the name both outlive the call, which \
                          is the probe's whole contract"
            )]
            let probed = unsafe {
                nvs_object_slot_probe(&raw const subject, name.as_ptr(), name.len(), hint)
            };
            let read =
                read_erased_property_hinted(&mut ctx, subject, name, hint, AbsentField::Throws);
            assert_eq!(
                probed,
                read.is_ok(),
                "the probe and the read disagree about `{name}` at hint {hint}"
            );
        }
    }

    #[test]
    fn an_object_that_dies_by_refcount_leaves_the_live_list() {
        // Item 36's whole claim: the list is maintained at *both* ends, so an
        // object that the refcounts already freed is not still on it waiting
        // for a sweep to free it a second time.
        let (table, animal, _dog, _greets) = hierarchy();
        let mut ctx = Ctx::new(crate::ctx::OutputSink::Sink);
        let current = crate::ctx::CurrentCtx::install(&mut ctx);
        assert_eq!(ctx.live_objects(), 0);
        {
            #[expect(unsafe_code, reason = "the table outlives the object")]
            let object = unsafe { NvsObj::new(table.desc(animal)) };
            assert_eq!(ctx.live_objects(), 1);
            drop(object);
        }
        assert_eq!(ctx.live_objects(), 0);
        drop(current);
    }

    #[test]
    fn an_adopted_object_moves_to_the_destinations_live_list() {
        // Item 36's crossing: the walk adopts at refcount 1, and adopting is
        // the allocation changing owners — so the object leaves the source's
        // list and joins the destination's. Left behind, the source's teardown
        // sweep would meet an object the destination is holding.
        let (table, animal, _dog, _greets) = hierarchy();
        let mut source = Ctx::new(crate::ctx::OutputSink::Sink);
        let mut destination = Ctx::new(crate::ctx::OutputSink::Sink);

        let inside = crate::ctx::CurrentCtx::install(&mut source);
        #[expect(unsafe_code, reason = "the table outlives the object")]
        let object = unsafe { NvsObj::new(table.desc(animal)) };
        let value = Value::object(object);
        assert_eq!(source.live_objects(), 1);
        drop(inside);

        // The shape `nvs_host`'s `finish` is in: the copy runs while the
        // *receiving* context is the one installed.
        let across = crate::ctx::CurrentCtx::install(&mut destination);
        let crossed = crate::graph::copy_graph(value).expect("an `Animal` crosses");
        assert_eq!(
            source.live_objects(),
            0,
            "the adopted object stayed on the source's list"
        );
        assert_eq!(destination.live_objects(), 1);
        // The same allocation, which is what makes this a move rather than a
        // copy that happens to be on the right list.
        assert_eq!(crossed.obj_ptr(), value.obj_ptr());
        #[expect(
            unsafe_code,
            reason = "the crossing answered the one reference to this \
                      allocation, and this is it being given up"
        )]
        unsafe {
            crate::release::release_value(crossed);
        }
        drop(across);
    }

    #[test]
    #[should_panic(expected = "stamped with")]
    #[cfg(debug_assertions)]
    fn dismantling_through_a_foreign_context_panics_in_debug() {
        // The stamp from the other side: an object linked on one context's
        // list while claiming to belong to another is one that a second relink
        // site moved half-way, and the foreign context's sweep is then free to
        // dismantle it under its holder. The drift is written by hand here
        // because no code in the crate can produce it — `LiveList::link`
        // writes the stamp and the links in one statement, which is the whole
        // design — and a release build would take the corruption silently, so
        // the guard is debug-only, as every `cargo test` run and the WSL
        // valgrind leg are.
        // Two lists and no context, so that the unwind out of the panic below
        // meets nothing that would assert a second time — a panic during a
        // panic aborts, and the abort would say none of this.
        let (table, animal, _dog, _greets) = hierarchy();
        let owner = LiveList::default();
        let elsewhere = LiveList::default();
        #[expect(unsafe_code, reason = "the table outlives the objects")]
        let (behind, object) = unsafe {
            (
                NvsObj::new(table.desc(animal)),
                NvsObj::new(table.desc(animal)),
            )
        };
        #[expect(
            unsafe_code,
            reason = "both objects are fresh and on no list, and the stamp \
                      written last is the drift this guard exists for: moved \
                      while the links stay where they are"
        )]
        unsafe {
            owner.link(behind.ptr.as_ptr());
            owner.link(object.ptr.as_ptr());
            (*object.ptr.as_ptr())
                .owner
                .set(std::ptr::from_ref(&elsewhere));
        }
        drop(object);
        drop(behind);
    }

    #[test]
    fn a_cyclic_object_graph_is_reclaimed_when_its_context_drops() {
        // Measured by the allocator, the way `crate::array`'s acyclic guard
        // is, and for a sharper reason: here no reference count ever reaches
        // zero, so agreeing with itself is exactly what the bookkeeping does
        // while the memory stays out.
        let (table, animal, dog, _greets) = hierarchy();
        // A context built and dropped before the baseline is taken, because
        // the first one on a thread warms per-thread state the counter sees
        // and nothing frees — measuring from a cold thread would charge that
        // to the sweep.
        drop(Ctx::new(crate::ctx::OutputSink::Sink));
        let before = counting_alloc::live_bytes();
        {
            let mut ctx = Ctx::new(crate::ctx::OutputSink::Sink);
            let current = crate::ctx::CurrentCtx::install(&mut ctx);
            for _ in 0..64 {
                #[expect(unsafe_code, reason = "the table outlives the objects")]
                unsafe {
                    let left = NvsObj::new(table.desc(animal));
                    let right = NvsObj::new(table.desc(dog));
                    left.set_field(0, Value::object(right.clone()));
                    right.set_field(0, Value::object(left.clone()));
                }
            }
            // Nothing was freed on the way: both handles went out of scope
            // holding each other at one.
            assert_eq!(ctx.live_objects(), 128);
            drop(current);
        }
        assert_eq!(counting_alloc::live_bytes(), before);
    }

    #[test]
    fn a_swept_cycles_native_teardown_runs() {
        // The difference between dismantling a cycle and abandoning the pages
        // it sat on: a swept member's own fields go through `crate::release`,
        // so what it was holding is released rather than leaked with it. The
        // string is named from outside the cycle so that the assertion is
        // about the member's teardown rather than about the header's bytes.
        let (table, animal, dog, _greets) = hierarchy();
        let held = NvsStr::new(b"a string only the cycle holds");
        let mut ctx = Ctx::new(crate::ctx::OutputSink::Sink);
        let current = crate::ctx::CurrentCtx::install(&mut ctx);
        #[expect(unsafe_code, reason = "the table outlives the objects")]
        unsafe {
            let left = NvsObj::new(table.desc(animal));
            let right = NvsObj::new(table.desc(dog));
            left.set_field(0, Value::object(right.clone()));
            right.set_field(0, Value::object(left.clone()));
            right.set_field(1, Value::str(held.clone()));
        }
        assert_eq!(held.refcount(), 2);
        drop(current);
        drop(ctx);
        assert_eq!(held.refcount(), 1);
    }

    #[test]
    fn a_cycle_closed_through_an_array_element_is_swept_at_teardown() {
        // The only edge closing this ring is an *element* of `left`'s array,
        // so a tally reading field slots alone would account for neither
        // object's reference and would leave the pair — and the array under
        // them — out for the life of the process. Measured by the allocator,
        // like its acyclic sibling above, because no reference count here
        // ever reaches zero on its own.
        let (table, animal, dog, _greets) = hierarchy();
        drop(Ctx::new(crate::ctx::OutputSink::Sink));
        let before = counting_alloc::live_bytes();
        {
            let mut ctx = Ctx::new(crate::ctx::OutputSink::Sink);
            let current = crate::ctx::CurrentCtx::install(&mut ctx);
            #[expect(unsafe_code, reason = "the table outlives the objects")]
            unsafe {
                let left = NvsObj::new(table.desc(animal));
                let right = NvsObj::new(table.desc(dog));
                let mut ring = NvsArray::new();
                ring.append(Value::object(right.clone()));
                left.set_field(0, Value::array(ring));
                right.set_field(0, Value::object(left.clone()));
            }
            assert_eq!(ctx.live_objects(), 2);
            drop(current);
        }
        assert_eq!(counting_alloc::live_bytes(), before);
    }

    #[test]
    fn a_cycle_closed_through_a_nested_array_is_swept_too() {
        // One level further down, which is the case the goal's standing
        // decision listed as the one a walk might not be able to prove. It
        // can: the outer array is `left`'s alone and the inner is the outer's
        // alone, so each reference on the way down is accounted for and the
        // descent is a tree rather than something needing its own cycle check.
        let (table, animal, dog, _greets) = hierarchy();
        drop(Ctx::new(crate::ctx::OutputSink::Sink));
        let before = counting_alloc::live_bytes();
        {
            let mut ctx = Ctx::new(crate::ctx::OutputSink::Sink);
            let current = crate::ctx::CurrentCtx::install(&mut ctx);
            #[expect(unsafe_code, reason = "the table outlives the objects")]
            unsafe {
                let left = NvsObj::new(table.desc(animal));
                let right = NvsObj::new(table.desc(dog));
                let mut inner = NvsArray::new();
                inner.append(Value::object(right.clone()));
                let mut outer = NvsArray::new();
                outer.append(Value::array(inner));
                left.set_field(0, Value::array(outer));
                right.set_field(0, Value::object(left.clone()));
            }
            assert_eq!(ctx.live_objects(), 2);
            drop(current);
        }
        assert_eq!(counting_alloc::live_bytes(), before);
    }

    #[test]
    fn an_object_held_only_by_a_live_members_array_survives_the_sweep() {
        // The direction the widening is not allowed to move anything. Nothing
        // outside the list refers to `kept` — its one reference is an element
        // of `holder`'s array — so the tally now accounts for it exactly, and
        // only the mark walk following the same edge keeps it. `holder` itself
        // is a survivor of the kind `sweep`'s docs name: a handle a Rust
        // caller still holds when the context goes down.
        //
        // The string is what the assertion reads, for
        // `a_swept_cycles_native_teardown_runs`' reason: it is named from
        // outside, so its count says whether `kept`'s own teardown ran without
        // this test reading a header the sweep may have freed.
        let (table, animal, dog, _greets) = hierarchy();
        let named = NvsStr::new(b"a string only the array's object holds");
        let mut ctx = Ctx::new(crate::ctx::OutputSink::Sink);
        #[expect(unsafe_code, reason = "the table outlives the objects")]
        let holder = unsafe {
            let current = crate::ctx::CurrentCtx::install(&mut ctx);
            let holder = NvsObj::new(table.desc(animal));
            let kept = NvsObj::new(table.desc(dog));
            kept.set_field(1, Value::str(named.clone()));
            let mut element = NvsArray::new();
            // Moved, not cloned: the array takes the handle's one reference,
            // so nothing but the element refers to `kept` from here on.
            element.append(Value::object(kept));
            holder.set_field(0, Value::array(element));
            drop(current);
            holder
        };
        assert_eq!(named.refcount(), 2);
        drop(ctx);
        assert_eq!(named.refcount(), 2);
        // And the graph is still a graph: releasing the survivor releases the
        // array, the array releases `kept`, and `kept`'s teardown runs then.
        drop(holder);
        assert_eq!(named.refcount(), 1);
    }

    #[test]
    fn a_cycle_is_reclaimed_near_the_memory_ceiling_while_the_request_runs() {
        // The collector's whole reason to exist, end to end: the allocation
        // that crossed `rule:errors/on-limit`'s ceiling raises the poll and the
        // collection together, and the poll reclaims what the refcounts could
        // not before it reads the counter — so a request whose ceiling a dead
        // cycle was holding carries on instead of stopping.
        let (table, animal, dog, _greets) = hierarchy();
        // Named from outside the ring, for `a_swept_cycles_native_teardown_runs`'
        // reason: its count is what says the members' own teardown ran, without
        // this test reading a header the walk has freed. Allocated before the
        // ceiling is armed, so the one allocation here that asks
        // `crate::budget::affords` cannot be the one refused — a refusal is
        // sticky and no collection takes it back.
        let held = NvsStr::new(b"a string only the abandoned cycle holds");
        let mut ctx = Ctx::new(crate::ctx::OutputSink::Sink);
        // Just above what the request already holds, so the pair below is what
        // crosses it. Two headers and their slots are more than this slack.
        ctx.set_memory_limit(ctx.memory_used() + 64);
        let current = crate::ctx::CurrentCtx::install(&mut ctx);
        #[expect(unsafe_code, reason = "the table outlives the objects")]
        unsafe {
            let left = NvsObj::new(table.desc(animal));
            let right = NvsObj::new(table.desc(dog));
            left.set_field(0, Value::object(right.clone()));
            right.set_field(0, Value::object(left.clone()));
            right.set_field(1, Value::str(held.clone()));
        }
        drop(current);
        // Both handles went out of scope holding each other at one, so nothing
        // outside the ring refers to either and no reference count says so.
        assert_eq!(ctx.live_objects(), 2);
        assert_eq!(held.refcount(), 2);
        assert!(
            ctx.safepoint_flags()
                .contains(crate::SafepointFlags::COLLECT),
            "the allocator asks for the collection where it crosses the ceiling"
        );
        assert!(
            ctx.memory_breach().is_some(),
            "the request is over its ceiling"
        );

        #[expect(unsafe_code, reason = "exercising the compiled-code entry point")]
        let status = unsafe { crate::nvs_safepoint(&raw mut ctx) };

        assert_eq!(
            status,
            crate::OK,
            "the collection is what leaves it running"
        );
        assert_eq!(ctx.live_objects(), 0);
        assert_eq!(held.refcount(), 1);
        assert!(ctx.pending().is_none());
        assert!(
            ctx.safepoint_flags().is_empty(),
            "a request back inside its ceiling leaves the poll with nothing standing"
        );
    }

    #[test]
    fn an_in_flight_collection_leaves_a_survivor_on_its_live_list() {
        // The one difference between the two walks, and the direction that
        // corrupts a heap if it is got wrong: `sweep` detaches what it did not
        // free because the list dies with the context, and a collection must
        // not, because the context goes on allocating into it. A survivor left
        // detached would reach its own dismantle with nothing to unlink from.
        let (table, animal, dog, _greets) = hierarchy();
        let mut ctx = Ctx::new(crate::ctx::OutputSink::Sink);
        let current = crate::ctx::CurrentCtx::install(&mut ctx);
        #[expect(unsafe_code, reason = "the table outlives the objects")]
        let survivor = unsafe {
            let survivor = NvsObj::new(table.desc(animal));
            let left = NvsObj::new(table.desc(dog));
            let right = NvsObj::new(table.desc(dog));
            left.set_field(0, Value::object(right.clone()));
            right.set_field(0, Value::object(left.clone()));
            survivor
        };
        drop(current);
        assert_eq!(ctx.live_objects(), 3);

        ctx.request_safepoint(crate::SafepointFlags::COLLECT);
        assert!(ctx.collect_if_asked(), "the ring is reclaimed");
        assert_eq!(
            ctx.live_objects(),
            1,
            "and the survivor is still on the list"
        );

        // Which is what lets it leave by the ordinary door: the last reference
        // goes, `dismantle` unlinks it, and the list is empty without the
        // context having ended.
        drop(survivor);
        assert_eq!(ctx.live_objects(), 0);
    }

    #[test]
    fn a_table_reports_what_it_holds() {
        let mut table = ClassTable::new();
        assert!(table.is_empty());
        table.define("One", &[] as &[&str], &[]);
        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());
    }

    /// The one thing `rule:security/isolate-shares-nothing` lets cross has to
    /// be able to: a compiled unit's descriptors are shared by every core, so
    /// an `Arc<ClassTable>` is what a second core resolves a request through.
    ///
    /// A compile-time assertion rather than a runtime one, because what it
    /// pins is a trait bound — if either `unsafe impl` above is removed, this
    /// stops building rather than staying green while the cache quietly
    /// becomes one per core.
    #[test]
    fn a_class_table_crosses_a_core_boundary_behind_an_arc() {
        const fn crosses<T: Send + Sync>() {}
        crosses::<ClassTable>();
        crosses::<std::sync::Arc<ClassTable>>();
        // The recipes travel with it, and are plain data — no `unsafe impl`
        // buys this one, so a `FieldDefault` growing a pointer fails here.
        crosses::<std::sync::Arc<[Option<FieldDefault>]>>();
    }
}
