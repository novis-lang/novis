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
//! checker qualifiers" design itself.

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
    /// `...spread` element, a `&value` element, and reading/writing an
    /// existing array by index are all still unsupported).
    Array,
}

impl Ty {
    /// Whether a value of this representation is a reference-counted heap
    /// allocation that needs a matching retain/release around every point it
    /// is copied into or dropped from a durable slot — see
    /// [`crate::lower`]'s module docs for exactly what "durable slot" means
    /// today. [`Self::Object`] is deliberately *not* included yet: nothing
    /// allocates or frees the memory behind one so far (see that variant's
    /// own doc comment), so there is nothing yet for a retain/release to do.
    #[must_use]
    pub fn is_refcounted(self) -> bool {
        matches!(self, Ty::Str | Ty::Bytes | Ty::Array)
    }
}
