//! The whole of the IR-representation-to-machine-representation mapping.
//!
//! [`mwl_ir::Ty`] is already the *representation* lattice — `mwl-ir`'s own
//! `ty` module explains why the checker's richer `Ty` does not survive past
//! `check_program` — so this is a table, not an analysis.

use cranelift::prelude::*;
use mwl_ir::Ty;
use mwl_ir::ty::EnumRepr;
use mwl_runtime::Tag;

use crate::CodegenError;

/// The machine type a value of this IR representation lives in, or `None` for
/// [`Ty::Void`], which no register ever holds.
///
/// A `bool` is `I8` rather than `I64` because that is what a Cranelift
/// comparison produces; every refcounted or opaque representation is a bare
/// pointer.
///
/// [`Ty::Tagged`] is the one that is not a scalar at all: it is `I128`, which
/// Cranelift legalizes to a register pair, and whose two halves are exactly
/// the two halves of a 16-byte [`mwl_runtime::Value`] on a little-endian
/// target — the low half its tag word, the high half its payload. That is
/// what makes materializing one into a call's argument slot two plain stores.
/// Every producer of an `I128` here keeps the tag word **zero-extended from
/// the tag byte**, which is the invariant `Emitter::emit_is_null` relies on to
/// compare without masking.
#[must_use]
pub fn clif_ty(ty: Ty) -> Option<Type> {
    Some(match ty {
        Ty::Bool => types::I8,
        Ty::Int | Ty::Uint => types::I64,
        Ty::Float => types::F64,
        Ty::Str | Ty::Bytes | Ty::Array | Ty::Object | Ty::ClassDesc => types::I64,
        // The whole 16-byte tagged value, in a register pair — see this
        // function's own doc comment and `mwl_ir::Ty::Tagged`.
        Ty::Tagged => types::I128,
        // `null`'s payload is always zero, but it still travels in a register
        // like every other representation rather than in a shape of its own.
        Ty::Null => types::I64,
        // A by-reference parameter's staged-slot address — see `Ty::Ref`.
        Ty::Ref => types::I64,
        // ADR 0010 § 6: an enum value *is* its backing integer.
        Ty::Enum(_) => types::I64,
        Ty::Void => return None,
        _ => types::I64,
    })
}

/// The [`Tag`] a value of this representation carries once it is materialized
/// into a 16-byte [`mwl_runtime::Value`].
///
/// [`Ty::Bytes`] shares [`Tag::Str`]: the two are one runtime representation
/// with different content, which is exactly what
/// [`mwl_runtime::Tag::Str`]'s own doc comment says.
///
/// # Errors
///
/// [`CodegenError::Unsupported`] for [`Ty::Tagged`], whose tag is by
/// definition not a function of its static representation — it carries its own
/// (see [`clif_ty`]), so every path that materializes one reads it from the
/// value instead and never asks here. Reaching this arm means a caller
/// forgot that. Likewise for [`Ty::Void`], which is not a value at all.
/// An exception is an ordinary [`Tag::Object`] now, with no case of its own.
pub(crate) fn tag_of(ty: Ty) -> Result<Tag, CodegenError> {
    Ok(match ty {
        Ty::Bool => Tag::Bool,
        Ty::Int => Tag::Int,
        Ty::Uint => Tag::Uint,
        Ty::Float => Tag::Float,
        Ty::Str | Ty::Bytes => Tag::Str,
        // The one representation whose tag is the whole of it — see
        // `mwl_ir::Ty::Null`.
        Ty::Null => Tag::Null,
        // ADR 0010 § 6 reserves a tag of its own for an enum; this uses the
        // backing type's instead, deliberately. A tag only has to answer
        // "which type is this?" where the static type does not — the `mixed`
        // case below, whose representation is still open. Deciding an enum's
        // tag before that would be deciding half the same question twice.
        // `mwl_ir::Ty::Enum`'s own doc comment records this.
        Ty::Enum(EnumRepr::Int) => Tag::Int,
        Ty::Enum(EnumRepr::Uint) => Tag::Uint,
        Ty::Array => Tag::Array,
        Ty::Object => Tag::Object,
        // Not an MWL value at all: a class descriptor rides in the payload
        // half of an otherwise-`null` slot, so nothing sweeping a `Value` can
        // mistake it for a heap reference. `mwl_runtime::object`'s module docs
        // own that decision; `mwl_ir::Ty::ClassDesc` restates the consequence.
        Ty::ClassDesc => Tag::Null,
        // Not an MWL value either, and for the same reason as `ClassDesc`
        // above: a by-reference parameter's staged-slot address rides in the
        // payload half of an otherwise-`null` slot, so nothing sweeping a
        // `Value` can mistake it for a heap reference. `mwl_ir::Ty::Ref` owns
        // the decision.
        Ty::Ref => Tag::Null,
        Ty::Tagged => {
            return Err(CodegenError::Unsupported(
                "the static tag of a tagged value, which carries its own".to_owned(),
            ));
        }
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "a value of representation {ty:?} crossing a call boundary"
            )));
        }
    })
}
