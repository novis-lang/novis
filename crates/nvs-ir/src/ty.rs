//! The IR's own value-representation type lattice — deliberately smaller and
//! flatter than `nvs_types::ty::Ty`, and not the same type.
//!
//! The checker's `Ty` exists to make `is_assignable` reject the *wrong*
//! program: it carries qualifiers (`tainted`, `secret`), full nominal class
//! identity, shape structure, and canonicalized unions — everything `rule:types/declaration`'s
//! grammar promises a developer. None of that survives past `check_program`:
//! by the time a function reaches this crate it has already been proven to
//! type-check, so lowering only needs to know how a value is *represented*
//! for codegen — which native width and instruction it needs, not which
//! source-level type produced it. (An analogous split exists in most
//! compilers with a rich front-end type system and a small backend one — e.g.
//! Rust's `ty::Ty` versus a codegen backend's handful of scalar/ABI kinds.)
//!
//! # What this lattice does not carry
//!
//! Scoped to exactly what's lowered so far — see the crate's own module docs
//! for the gaps that are still owed. [`Ty::Str`] and [`Ty::Bytes`] are refcounted,
//! heap-allocated representations sharing one shape and one set of
//! [`crate::lower`] retain/release insertion points — see [`crate::lower`]'s
//! module docs for the policy that makes them safe — except that nothing in
//! the grammar constructs a *fresh* `bytes` value, there being no literal
//! syntax for one (see [`Ty::Bytes`]'s own doc comment).
//! [`Ty::Array`] is a bare, opaque representation exactly like
//! [`Ty::Object`] — no boxed/interned element type — since no lowering
//! decision needs to branch on an array's *element* type at this
//! IR level (`nvs_types::ty::Ty::Array(TypeId)` already enforces that at
//! check time; erasing it here is the same "representation, not identity"
//! split [`Ty::Object`] draws for a class/enum). [`Ty::Object`] is
//! the other non-scalar representation:
//! `nvs_runtime::object` gives it a heap shape, and
//! [`crate::ir::Program::classes`] carries the per-class slot order this
//! per-value lattice has no room for. Widening lowering further adds
//! variants to this enum; it does not replace the "erase checker
//! qualifiers" design itself. [`Ty::Tagged`] is the one variant that is not a
//! machine representation of a single Novis type at all — it is the *tagged*
//! representation every type whose runtime shape is not statically known
//! erases to: `mixed`, `?T`, and any other union. Its own doc comment is the
//! home for that decision, its cost, and the instructions that widen
//! into it, narrow out of it and test it.

/// One IR value's representation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Ty {
    /// `bool`.
    Bool,
    /// `int` — signed.
    Int,
    /// `uint` — `rule:types/arithmetic`.
    Uint,
    /// `float`.
    Float,
    /// `decimal` — `rule:types/decimal`'s
    /// scalar, sign plus a 96-bit mantissa plus a scale of 0 to 28.
    ///
    /// # The representation
    ///
    /// **A `nvs_runtime::Value` carrying `Tag::Decimal`**, which is to say the
    /// same register pair [`Self::Tagged`] travels in and the same sixteen
    /// bytes — not a shape of its own. The mantissa does not fit the payload
    /// half alone, so it spills into the bytes a `Value` otherwise calls
    /// padding; `nvs_runtime::decimal`'s own module docs are the one home for
    /// the bit positions and for why one shape was chosen over two.
    ///
    /// The consequence worth knowing here: [`crate::ir::InstKind::Tag`] and
    /// [`crate::ir::InstKind::Untag`] are the **identity** on a `decimal`, so
    /// a `decimal` inside a `mixed`, a `?decimal` or any other union is the
    /// same bits at the same width, and nothing has to be rebuilt at the
    /// boundary.
    ///
    /// # What it spends
    ///
    /// Sixteen bytes per value against eight for a `float` — ADR 0054
    /// § *Consequences*' own figure, and memory footprint spent so decimal
    /// arithmetic is exact.
    /// Nothing is allocated: this is **not** [`Self::is_refcounted`], so no
    /// insertion point keyed on that predicate has a `decimal` arm.
    ///
    /// # Getting work done with one
    ///
    /// Every operator is an out-of-line [`crate::ir::Helper`] — arithmetic,
    /// comparison, truthiness, and both directions of every conversion — for
    /// the same reason [`crate::ir::Helper::TaggedToString`] is one: the
    /// arithmetic needs a wider intermediate than the value itself, which
    /// ADR 0054 § *Consequences* names as the real implementation cost.
    /// Inlining the equal-scale `+`, `-` and comparison that ADR anticipates
    /// is a backend optimization, recorded as a known gap in
    /// `nvs_runtime::decimal`, not a semantic difference.
    Decimal,
    /// `null` — the one value of its own type, and the whole of what an
    /// **absent** argument is.
    ///
    /// Deliberately *not* the same thing as [`Self::Tagged`]. This is the
    /// static type of the literal `null` and of an **absent** argument: one
    /// value, known at compile time, represented as a `Tag::Null` tag byte
    /// over a zero payload — exactly what `nvs_runtime::Value::null` builds
    /// and what every helper already receives in the receiver slot of a
    /// static call. A binding *declared* `?T` is the other thing, and is
    /// [`Self::Tagged`]: it has to hold either representation at different
    /// points, so it carries its tag at runtime. [`crate::ir::InstKind::Tag`]
    /// is the widening between them, and costs one register pair to build.
    ///
    /// **Not refcounted**, and nothing is ever allocated for it.
    Null,
    /// A function returning nothing.
    Void,
    /// A reference to a class instance — every `nvs_types::ty::Ty::Class`/
    /// `Ty::Enum` erases to this one opaque representation, with no class
    /// identity carried in the IR at all: a call's or `new`'s actual target
    /// is already resolved to a concrete label by
    /// `nvs_types::expr_table::ExprTypeTable` before lowering ever reaches
    /// it (see `crate::lower`'s module docs), so nothing downstream of that
    /// needs to ask "which class is this?" again. Its heap shape is
    /// `nvs_runtime::object`'s: a refcounted header plus one uniform
    /// 16-byte slot per declared property, laid out ancestors-first. The slot
    /// *order* is the one class fact this representation deliberately does
    /// not carry — it is per-class rather than per-value, so it lives in
    /// [`crate::ir::Program::classes`] instead, which
    /// [`crate::ir::InstKind::FieldGet`]'s `class`/`field` labels index into.
    Object,
    /// A reference-counted, heap-allocated `string` — `rule:types/string-is-utf8` has not
    /// settled that type's default length/indexing
    /// granularity, but nothing lowered so far needs indexing at all: only a
    /// literal's *bytes*, which are granularity-independent. [`crate::lower`]
    /// cooks a literal directly to [`crate::ir::InstKind::ConstStr`] and
    /// inserts [`crate::ir::InstKind::Retain`]/[`crate::ir::InstKind::Release`]
    /// around a local's declare/reassign/scope-exit lifecycle, a call
    /// argument/parameter, a returned value, and a compile-time-known
    /// property read — see [`crate::lower`]'s own docs for the exact policy
    /// (keyed on [`crate::lower::is_aliasing_read`]) and its known gaps
    /// (string concatenation and `tainted`/`secret`-qualified string
    /// variants are still unsupported).
    Str,
    /// A reference-counted, heap-allocated `bytes` value — `rule:types/bytes`'s own
    /// primitive. The same representation shape as [`Self::Str`], different
    /// content: same retain/release treatment at a local's declare/reassign/
    /// scope-exit lifecycle, a call argument/parameter, a returned value, and
    /// a compile-time-known property read/write. Unlike [`Self::Str`],
    /// nothing in `nvs-syntax`'s grammar constructs a *fresh* `bytes` value
    /// yet — there is no `bytes` literal syntax (no `b"..."` form or
    /// equivalent), so every `bytes` value a fixture can lower today
    /// originates as a parameter or a property read, both
    /// [`crate::lower::is_aliasing_read`] shapes. A `Core\Bytes`-producing
    /// conversion or constructor, once one exists, would be the first fresh
    /// producer.
    Bytes,
    /// A reference-counted, heap-allocated `array<T>` — `rule:types/arrays`'s
    /// insertion-ordered, string-keyed hash with copy-on-write value
    /// semantics. Bare and opaque, carrying no element type at all: see this
    /// module's own doc comment for why, mirroring [`Self::Object`]'s
    /// "representation, not identity" erasure. [`crate::lower`] cooks a
    /// literal (`[...]`/legacy `array(...)`) directly to
    /// [`crate::ir::InstKind::ArrayNew`] and applies the same
    /// [`crate::lower::is_aliasing_read`]-keyed retain policy to each element
    /// that [`Self::is_refcounted`] — see that instruction's own doc comment
    /// for the exact policy and its known gaps (an explicit `key =>` entry, a
    /// `...spread` element, and a `&value` element are all still
    /// unsupported). Reading and writing an existing array by a known-type
    /// index lower too, to [`crate::ir::InstKind::ArrayGet`]/
    /// [`crate::ir::InstKind::ArraySet`] — see those variants' own doc
    /// comments for the int/uint-to-string key normalization they apply and
    /// their own known gaps (append syntax, a non-int/uint/string key, a
    /// `mixed`-erased base).
    Array,
    /// A **tagged** value: one whose runtime type is carried with it rather
    /// than known statically. Every checker type that admits more than one
    /// runtime shape erases to this one representation — `rule:types/grammar`'s
    /// `mixed`, a nullable `?T`, and any other union.
    ///
    /// # The representation
    ///
    /// Exactly `nvs_runtime::Value`: a tag byte, seven bytes of padding, and
    /// an eight-byte payload. In a register it is **one register pair** — the
    /// low half is the value's first eight bytes (the tag byte and its
    /// padding), the high half is the payload — which is the little-endian
    /// memory image of that struct, so materializing one into a call's
    /// argument slot is one store per half and no reshuffling at all.
    /// `nvs_codegen::ty::clif_ty` names the machine type; the *layout* is
    /// `nvs_runtime::value`'s, and this variant deliberately adds no second
    /// one.
    ///
    /// # Why `mixed` and `?T` are one representation, not two
    ///
    /// Both need the same thing: a discriminant read at runtime. That
    /// discriminant is the tag byte every `rule:errors/propagation` call
    /// boundary already carries, so reusing it costs no new invariant,
    /// while two shapes would mean two widen/narrow protocols, two
    /// refcount paths and two ways for a `?mixed` to be ambiguous. A `?T` is
    /// *narrower* than `mixed`, but only in what the **checker** will let a
    /// program do with it; nothing downstream of `check_program` reads that
    /// difference, which is this module's whole premise.
    ///
    /// # What it spends
    ///
    /// Sixteen bytes per live tagged value instead of eight — one extra
    /// machine register, or eight extra stack bytes, per tagged value in
    /// flight. Nothing is allocated: the tag rides *with* the value, so a
    /// `?int` is still a register pair rather than a pointer to a box. On top
    /// of that a retain or release of one is an out-of-line call to
    /// `nvs_runtime::nvs_value_retain`/`nvs_value_release`, which branches on
    /// the tag, where a statically-typed value calls the exact primitive its
    /// representation names. That is memory footprint, and a little latency,
    /// spent so a value whose type is decided at run time carries that type
    /// with it, on the same terms the 16-byte `Value` itself was bought.
    ///
    /// # Getting in and out
    ///
    /// Nothing *compiled inline* may read a tag:
    /// [`crate::ir::InstKind::Tag`] widens a statically-typed value into one,
    /// [`crate::ir::InstKind::Untag`] narrows one back to a representation the
    /// checker already proved it holds, and
    /// [`crate::ir::InstKind::IsNull`] tests it for `null`.
    /// [`crate::lower::Lowering::coerce`] is the one place the widening and
    /// narrowing are emitted, at every boundary carrying a declared type. A
    /// [`crate::ir::Helper`] may branch on the tag as well, but only out of
    /// line, inside `nvs-runtime` — [`crate::ir::Helper::TaggedToString`] is
    /// one such, and that arrangement is what keeps the number of tag layouts
    /// compiled code knows about at exactly one.
    ///
    /// **Refcounted** ([`Self::is_refcounted`]), because it may hold a
    /// payload that is — which is precisely what the runtime branch above is
    /// for. Everything keyed on that predicate (a local's declare/reassign/
    /// scope-exit lifecycle, a call argument, a return value, a property
    /// read) therefore needs no tagged-specific insertion point.
    ///
    /// # Known gap
    ///
    /// Reading a tagged value in a way that needs its *actual* type without a
    /// checker-proven narrowing still panics naming the case, everywhere but
    /// one: rendering it — `.`, `echo` and `as string` — goes through
    /// [`crate::ir::Helper::TaggedToString`]. Arithmetic on a `mixed`,
    /// `rule:expressions/truthy-positions`'s truthy table and an array access through a `mixed`-erased
    /// base are the ones left, and each closes the same way, by
    /// adding a [`crate::ir::Helper`] variant that dispatches on the tag
    /// rather than a second representation.
    Tagged,
    /// An enum value — `rule:enums/closed-integer-type`'s
    /// closed, named integer type.
    ///
    /// Its *representation* is exactly the backing integer it carries, which
    /// is why [`crate::ir::InstKind::Reinterpret`] is all `$e as int` costs:
    /// `rule:types/conversion` calls that conversion "total, free ... same
    /// representation, reinterpreted." So this variant is not here to describe
    /// a different machine value. It is here because one rule reads an enum
    /// differently from the integer under it:
    /// `rule:enums/truthiness` makes
    /// an enum case **always truthy**, never judged by its backing value — a
    /// case backed by `0` is `true` in a condition, where a plain `int` `0`
    /// is `false`. Erasing an enum to [`Self::Int`] here would silently get
    /// that wrong, and the checker cannot fix it afterwards, because by then
    /// nothing distinguishes the two.
    ///
    /// Not refcounted, no allocation, no descriptor: `rule:enums/no-class-machinery`'s "a case is
    /// an integer constant, inlined at every use site."
    ///
    /// Materialized into a `nvs_runtime::Value` it takes a tag of its own,
    /// `Tag::EnumInt` or `Tag::EnumUint` over the same payload
    /// (`rule:enums/representation`), so the same rule still holds once the
    /// static type is gone.
    Enum(EnumRepr),
    /// The address of one 16-byte `nvs_runtime::Value` cell — what a `&T`
    /// parameter is, and the only thing this representation is ever used for.
    ///
    /// # The representation decision
    ///
    /// A `&T` parameter is a **caller-staged one-slot temporary**, not a
    /// pointer into the caller's own storage. At each call site the caller
    /// allocates one `Value`-sized stack slot
    /// ([`crate::ir::InstKind::RefSlot`]), copies the holder's current value
    /// into it, passes its address, and copies whatever is in it back to the
    /// holder after the call ([`crate::ir::InstKind::RefLoad`] plus
    /// `crate::lower::Lowering::write_back_ref`). Inside the callee the
    /// parameter is that address: every read is a
    /// [`crate::ir::InstKind::RefLoad`] and every write a
    /// [`crate::ir::InstKind::RefStore`].
    ///
    /// The alternative — true aliasing, a pointer to the caller's own storage,
    /// which is what PHP does — was rejected on cost. It requires demoting
    /// every local that is ever the target of `inout` out of SSA into an
    /// addressable stack slot, which reaches phis, `crate::lower::Env` and the
    /// refcount policy all at once; and it has no answer at all for
    /// `inout $arr[0]`, since `rule:types/arrays`'s copy-on-write gives an array element
    /// no stable address. Staging costs one stack slot per by-reference
    /// argument per call site and two copies per call — bought against no SSA
    /// demotion anywhere (the address is loop-invariant, so the callee needs
    /// no phi it did not already need) and no constraint whatsoever on what
    /// the caller's holder may be.
    ///
    /// # Refcounting
    ///
    /// **The slot owns exactly one reference at every point in its life**,
    /// when the pointee representation [`Self::is_refcounted`]:
    ///
    /// - [`crate::ir::InstKind::RefSlot`] retains the value it stages, so the
    ///   holder and the slot each own one.
    /// - [`crate::ir::InstKind::RefStore`] releases what the slot held and
    ///   stores an owned value in its place — exactly
    ///   `crate::lower::Lowering::bind_local`'s policy, against a slot instead
    ///   of an `Env` entry.
    /// - The copy-back releases the *holder's* previous value and transfers
    ///   the slot's reference into it. The two ends balance: the staging
    ///   retain pays for the copy-back release.
    ///
    /// A `&T` parameter is therefore never released at the callee's exit, and
    /// needs no exclusion to arrange that — this representation is not
    /// [`Self::is_refcounted`], so `crate::lower::Lowering::release_all_locals`
    /// already skips it.
    ///
    /// ## Known gap: a callee that throws
    ///
    /// The copy-back sits on the normal edge only, so a callee that throws
    /// past it leaves the staging retain unpaid — one leaked reference per
    /// by-reference argument with a refcounted pointee, and the caller's
    /// holder keeps its pre-call value instead of the callee's partial write
    /// (PHP would keep the write). Never a dangling reference: that is what
    /// the staging retain buys, and why it is unconditional rather than
    /// keyed on whether the callee writes. Closing it means emitting the same
    /// [`crate::ir::InstKind::RefLoad`] and write-back into
    /// `crate::lower::Lowering::landing_block`'s block, ahead of its release
    /// sweep, and updating the `crate::lower::TryFrame` edge's captured `Env`
    /// so a `catch` handler's phis see the written-back binding.
    ///
    /// Materialized into a `nvs_runtime::Value` it takes the same shape
    /// [`Self::ClassDesc`] does — a `Tag::Null` tag byte with the address in
    /// the payload half — and for the same reason: it is not an Novis value at
    /// all, so nothing sweeping a `Value` may mistake it for a heap reference.
    Ref,
    /// A `nvs_runtime::ClassDesc` address — the *class* a frame was called on,
    /// and, per
    /// `rule:types/class-reference`,
    /// the value of a `class<T>` binding.
    ///
    /// This is late static binding's whole representation. It is produced by
    /// [`crate::ir::InstKind::ClassDescConst`] (a class named in source),
    /// [`crate::ir::InstKind::ClassDescOf`] (an instance's own class),
    /// [`crate::ir::InstKind::ClassDescIn`] (`rule:types/class-reference`'s two checked `as`
    /// rows) and by a static method's [`crate::ir::InstKind::Param`] 0, and
    /// consumed by [`crate::ir::InstKind::NewDynamic`] and
    /// [`crate::ir::InstKind::CallVirtual`].
    ///
    /// **A declared type lowers to it, and exactly one does.** `rule:types/class-reference`
    /// makes `class<T>` a type written wherever a type is written, so a
    /// descriptor reaches a local, a parameter, a field and a return value
    /// — which costs nothing beyond the word it already is, since a
    /// descriptor is immortal and process-wide (§ 1's own sentence) and the
    /// slot holding one needs no lifecycle at all. What the erasure drops is
    /// the `T`: two `class<T>`s over different bounds are one representation
    /// here, exactly as [`Self::Object`] drops which class an instance is, and
    /// for the same reason — every decision that needs `T` is `rule:types/class-reference-sites`'s
    /// and is taken by the checker, above this boundary.
    ///
    /// **It is the one representation other than [`Self::Tagged`] that can
    /// hold `null`, and `?class<T>` is why.** `rule:expressions/nullable-conversion`'s sugar interns as
    /// `Union([Null, ClassRef])` and [`crate::lower`]'s `shared_erasure` folds
    /// that pair to this variant rather than tagging it: no class lives at
    /// address zero, so a descriptor already has a spare value meaning "no
    /// class", and [`crate::ir::InstKind::ClassDescIn`] already produces it on
    /// a miss. Tagging instead is not available — a descriptor materializes as
    /// a `Tag::Null` byte over its address, so a tagged `?class<T>` could not
    /// tell its two answers apart without a `Tag` of its own, and a tag would
    /// make a descriptor an Novis value, which the paragraph below says it is
    /// not.
    ///
    /// The cost is that a [`Self::Tagged`] operand is not the only one that
    /// can hold `null`, so every site that asks carries a row of
    /// its own, all of them the same test — the word, through
    /// `Lowering::class_desc_word`, against zero:
    /// `Lowering::lower_null_identity` (`$c == null` against the written
    /// literal), `Lowering::lower_binary` (`==`/`!=` between two of them),
    /// `Lowering::lower_coalesce` (`??`), `Lowering::lower_isset_operand`
    /// (`isset`), `Lowering::truthy_convert` (a condition, `!` and `empty`),
    /// and `Lowering::coerce` (a written `null` reaching a `?class<T>`
    /// binding). Each is written for a [`Self::ClassDesc`] operand whether or
    /// not its checked type is nullable, since a non-nullable one is never
    /// the zero word: the comparison then answers the constant a
    /// non-nullable operand demands anyway, so there is no second rule to
    /// keep in step.
    ///
    /// Not refcounted — a descriptor is owned by the compiled unit's class
    /// table for that unit's whole life (`nvs_runtime::object`), so there is
    /// nothing to retain and nothing to free. Materialized into a
    /// `nvs_runtime::Value` it keeps a `Tag::Null` tag byte and carries the
    /// address in the payload half, which is why nothing sweeping a `Value`
    /// can ever mistake one for a heap reference — see
    /// `nvs_runtime::object`'s own docs for that decision.
    ClassDesc,
}

/// Which integer an [`Ty::Enum`] value is represented by — `rule:enums/one-backing-type`'s
/// underlying type, `int` unless the declaration wrote `: uint`.
///
/// A second, two-variant enum rather than `Ty::EnumInt`/`Ty::EnumUint` so that
/// every "is this an enum at all" test stays one pattern.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnumRepr {
    /// An `int`-backed enum.
    Int,
    /// A `uint`-backed enum.
    Uint,
}

impl Ty {
    /// Whether a value of this representation is a reference-counted heap
    /// allocation that needs a matching retain/release around every point it
    /// is copied into or dropped from a durable slot — see
    /// [`crate::lower`]'s module docs for exactly what "durable slot" means
    /// today. [`Self::Object`] is on the list because `nvs_runtime::object`
    /// gives an instance a real allocation to free: an object local, argument,
    /// return value or field carries exactly the retain/release a string
    /// does, through `nvs_object_retain`/`nvs_object_release`.
    /// [`Self::Tagged`] is on the list for a reason one level further removed:
    /// a tagged value's *actual* payload might be refcounted (a string, an
    /// array, an object) or not (a scalar, `null`), and which one it is is
    /// exactly what its tag says — so the retain/release is emitted
    /// unconditionally and the **runtime** branches, through
    /// `nvs_runtime::nvs_value_retain`/`nvs_value_release`. That is what makes
    /// every insertion point this method gates need no tagged-specific arm.
    #[must_use]
    pub fn is_refcounted(self) -> bool {
        matches!(
            self,
            Ty::Str | Ty::Bytes | Ty::Array | Ty::Object | Ty::Tagged
        )
    }
}
