//! The IR's own value-representation type lattice — deliberately smaller and
//! flatter than `mwl_types::ty::Ty`, and not the same type.
//!
//! The checker's `Ty` exists to make `is_assignable` reject the *wrong*
//! program: it carries qualifiers (`tainted`, `secret`), full nominal class
//! identity, shape structure, and canonicalized unions — everything ADR 0007's
//! grammar promises a developer. None of that survives past `check_program`:
//! by the time a function reaches this crate it has already been proven to
//! type-check, so lowering only needs to know how a value is *represented*
//! for codegen — which native width and instruction it needs, not which
//! source-level type produced it. (An analogous split exists in most
//! compilers with a rich front-end type system and a small backend one — e.g.
//! Rust's `ty::Ty` versus a codegen backend's handful of scalar/ABI kinds.)
//!
//! # Known gaps
//!
//! Scoped to exactly what's lowered so far — see the crate's own module docs
//! for the full list. [`Ty::Str`] is the first refcounted, heap-allocated
//! representation to land (a `string`-typed local, parameter or return —
//! see [`crate::lower`]'s module docs for the retain/release insertion
//! policy that makes it safe). [`Ty::Bytes`] is the mechanical follow-on that
//! doc comment anticipated: same representation shape, same
//! [`crate::lower`] retain/release insertion points, no new policy needed —
//! only nothing in the grammar constructs a *fresh* one yet (no literal
//! syntax exists for `bytes`; see [`Self::Bytes`]'s own doc comment).
//! [`Ty::Array`] is next: a bare, opaque representation exactly like
//! [`Ty::Object`] — no boxed/interned element type — since no lowering
//! decision made so far needs to branch on an array's *element* type at this
//! IR level (`mwl_types::ty::Ty::Array(TypeId)` already enforces that at
//! check time; erasing it here is the same "representation, not identity"
//! split [`Ty::Object`] already draws for a class/enum). [`Ty::Object`] is
//! the one other non-scalar representation that exists, still reserved
//! rather than functional (see its own doc comment) — widening lowering
//! further adds variants to this enum; it does not replace the "erase
//! checker qualifiers" design itself. [`Ty::Mixed`] is the newest, and the
//! first variant that is reserved *by design* rather than only until a later
//! slice gets to it: unlike every representation above, there is no obvious
//! "next slice" that makes it functional without first deciding a runtime
//! type-tag representation — see its own doc comment for exactly what that
//! open design question is and what round-trips through it already.

/// One IR value's representation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Ty {
    /// `bool`.
    Bool,
    /// `int` — signed.
    Int,
    /// `uint` — ADR 0007 § 4.
    Uint,
    /// `float`.
    Float,
    /// A function returning nothing.
    Void,
    /// A reference to a class instance — every `mwl_types::ty::Ty::Class`/
    /// `Ty::Enum` erases to this one opaque representation, with no class
    /// identity carried in the IR at all: a call's or `new`'s actual target
    /// is already resolved to a concrete label by
    /// `mwl_types::expr_table::ExprTypeTable` before lowering ever reaches
    /// it (see `crate::lower`'s module docs), so nothing downstream of that
    /// needs to ask "which class is this?" again. Reserved rather than fully
    /// modeled: no refcount operations exist yet for a value of this
    /// representation (the milestone's "refcount operations" ingredient,
    /// still a known gap — see the crate docs), and no field/property layout
    /// exists either (property access is still unsupported). What *is*
    /// modeled: `new` constructing one, and passing/returning one through a
    /// resolved call.
    Object,
    /// A reference-counted, heap-allocated `string` — ADR 0009 (still
    /// *Proposed*, not *Accepted*) has not settled that type's indexing
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
    /// A reference-counted, heap-allocated `bytes` value — ADR 0009 (still
    /// *Proposed*). The same representation shape as [`Self::Str`], different
    /// content: same retain/release treatment at a local's declare/reassign/
    /// scope-exit lifecycle, a call argument/parameter, a returned value, and
    /// a compile-time-known property read/write. Unlike [`Self::Str`],
    /// nothing in `mwl-syntax`'s grammar constructs a *fresh* `bytes` value
    /// yet — there is no `bytes` literal syntax (no `b"..."` form or
    /// equivalent), so every `bytes` value a fixture can lower today
    /// originates as a parameter or a property read, both
    /// [`crate::lower::is_aliasing_read`] shapes. A `Core\Bytes`-producing
    /// conversion or constructor, once one exists, would be the first fresh
    /// producer.
    Bytes,
    /// A reference-counted, heap-allocated `array<T>` — ADR 0007 § 5's
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
    /// index now lower too, to [`crate::ir::InstKind::ArrayGet`]/
    /// [`crate::ir::InstKind::ArraySet`] — see those variants' own doc
    /// comments for the int/uint-to-string key normalization this needed and
    /// their own known gaps (append syntax, a non-int/uint/string key, a
    /// `mixed`-erased base).
    Array,
    /// A reference-counted, runtime-owned exception —
    /// `mwl_runtime::ThrowableHeader`, opaque here as everywhere else.
    ///
    /// Deliberately *not* [`Self::Object`]. ADR 0020 § 1's global
    /// `Throwable`/`Exception`/`Error` have no source declaration, so they
    /// have no field layout for an object representation to describe; what
    /// they have instead is a closed set of runtime entry points
    /// (`getMessage()`, `getTraceAsString()`). Giving them their own
    /// representation is what lets an exception work in M3 while a
    /// user-declared class still waits for M4 — see the crate docs' known
    /// gaps. Refcounted like [`Self::Str`], through
    /// `mwl_throwable_retain`/`mwl_throwable_release`, with the identical
    /// [`crate::lower::is_aliasing_read`]-keyed insertion policy.
    Throwable,
    /// `mixed` — ADR 0007 § 3's one unchecked position. Bare and opaque,
    /// like [`Self::Object`]/[`Self::Array`]: a `mixed`-typed value's actual
    /// runtime shape (`int`, a `string`, an array, an object, ...) needs a
    /// runtime type tag to distinguish, and this variant deliberately does
    /// not decide that representation yet — the design question the
    /// milestone text names is "how a `mixed` value's runtime type tag is
    /// represented," and picking one is real, non-mechanical work belonging
    /// to whichever slice first needs to branch on it (a runtime-helper call
    /// dispatching on `mixed`'s actual type, ADR 0035's `null`/`mixed` truthy
    /// case, or arithmetic's `mixed` fallback — see [`crate::lower`]'s crate
    /// docs for all three). What *this* slice lands is narrower: enough
    /// representation for a `mixed`-typed local, parameter, return value or
    /// call argument to exist and round-trip through [`crate::lower`]'s
    /// existing local-bind/call-argument/return machinery, which keys
    /// entirely off [`Self::is_refcounted`] and needs no `mixed`-specific
    /// insertion point to do that — see [`Self::is_refcounted`]'s own doc
    /// comment for why this variant is excluded there too. Reading, writing
    /// or converting a `mixed` value in any way that needs to know its actual
    /// runtime type (arithmetic, `.` concatenation, an `if`/`while`
    /// condition, indexing) still panics naming the case: this slice adds a
    /// representation to erase into, not a way to see through it again.
    /// Deliberately *not* what a union or a nullable (`?T`) type lowers to:
    /// [`crate::lower::lower_decl_type`]/[`crate::lower::lower_checked_ty`]
    /// only route the bare `TypeAtom::Mixed`/`CheckedTy::Mixed` atom here —
    /// a union/`?T` still panics unchanged, since folding either into this
    /// same representation would be its own decision (they are narrower than
    /// fully-erased `mixed`, and `null`'s own IR representation is a
    /// separate, still-open gap named elsewhere in this crate) rather than a
    /// mechanical extension of it.
    Mixed,
}

impl Ty {
    /// Whether a value of this representation is a reference-counted heap
    /// allocation that needs a matching retain/release around every point it
    /// is copied into or dropped from a durable slot — see
    /// [`crate::lower`]'s module docs for exactly what "durable slot" means
    /// today. [`Self::Object`] is deliberately *not* included yet: nothing
    /// allocates or frees the memory behind one so far (see that variant's
    /// own doc comment), so there is nothing yet for a retain/release to do.
    /// [`Self::Mixed`] is excluded for the same reason, one level further
    /// removed: a `mixed` value's *actual* runtime type might itself be
    /// refcounted (a `string`, an array, an object) or not (a scalar), but
    /// nothing decides that runtime type tag yet (see that variant's own doc
    /// comment) — so there is no way to know *whether* a retain/release is
    /// even needed for a given `mixed` value today, let alone emit the right
    /// one. A `mixed`-typed local/parameter/return still round-trips
    /// correctly without one: PHP's/ADR 0007's own semantics don't ask this
    /// crate to free anything behind a value it never inspects, and every
    /// insertion point this method gates already treats "not refcounted" as
    /// "nothing to do here," not "assume no cleanup is ever needed" — the
    /// distinction that will matter once a real tag representation lands.
    #[must_use]
    pub fn is_refcounted(self) -> bool {
        matches!(self, Ty::Str | Ty::Bytes | Ty::Array | Ty::Throwable)
    }
}
