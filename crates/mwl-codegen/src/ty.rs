//! The whole of the IR-representation-to-machine-representation mapping.
//!
//! [`mwl_ir::Ty`] is already the *representation* lattice — `mwl-ir`'s own
//! `ty` module explains why the checker's richer `Ty` does not survive past
//! `check_program` — so this is a table, not an analysis.

use cranelift::prelude::*;
use mwl_ir::Ty;
use mwl_runtime::Tag;

use crate::CodegenError;

/// The machine type a value of this IR representation lives in, or `None` for
/// [`Ty::Void`], which no register ever holds.
///
/// A `bool` is `I8` rather than `I64` because that is what a Cranelift
/// comparison produces; every refcounted or opaque representation is a bare
/// pointer.
#[must_use]
pub fn clif_ty(ty: Ty) -> Option<Type> {
    Some(match ty {
        Ty::Bool => types::I8,
        Ty::Int | Ty::Uint => types::I64,
        Ty::Float => types::F64,
        Ty::Str | Ty::Bytes | Ty::Array | Ty::Object | Ty::Mixed => types::I64,
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
/// [`CodegenError::Unsupported`] for [`Ty::Mixed`], whose runtime type tag is
/// still undecided (`mwl-ir`'s known gap 5) — a `mixed` value's tag is by
/// definition not knowable from its static representation, which is the whole
/// of the open question — and for [`Ty::Void`], which is not a value at all.
/// An exception is an ordinary [`Tag::Object`] now, with no case of its own.
pub(crate) fn tag_of(ty: Ty) -> Result<Tag, CodegenError> {
    Ok(match ty {
        Ty::Bool => Tag::Bool,
        Ty::Int => Tag::Int,
        Ty::Uint => Tag::Uint,
        Ty::Float => Tag::Float,
        Ty::Str | Ty::Bytes => Tag::Str,
        Ty::Array => Tag::Array,
        Ty::Object => Tag::Object,
        Ty::Mixed => {
            return Err(CodegenError::Unsupported(
                "a `mixed` value crossing a call boundary — its runtime type \
                 tag is still an open representation question"
                    .to_owned(),
            ));
        }
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "a value of representation {ty:?} crossing a call boundary"
            )));
        }
    })
}
