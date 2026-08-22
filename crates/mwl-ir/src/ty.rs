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
//! for the full list. Not modeled yet, deliberately: `string`/`bytes` (need a
//! runtime representation and a refcounting decision first) and `array<T>`.
//! [`Ty::Object`] is the one non-scalar representation that does exist,
//! reserved rather than functional (see its own doc comment) — widening
//! lowering further adds variants to this enum; it does not replace the
//! "erase checker qualifiers" design itself.

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
}
