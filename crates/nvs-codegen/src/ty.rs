//! The whole of the IR-representation-to-machine-representation mapping.
//!
//! [`nvs_ir::Ty`] is already the *representation* lattice — `nvs-ir`'s own
//! `ty` module explains why the checker's richer `Ty` does not survive past
//! `check_program` — so this is a table, not an analysis.

use cranelift::prelude::*;
use nvs_ir::Ty;
use nvs_ir::ty::EnumRepr;
use nvs_runtime::Tag;

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
/// the two halves of a 16-byte [`nvs_runtime::Value`] on a little-endian
/// target — the low half its tag word, the high half its payload. That is
/// what makes materializing one into a call's argument slot two plain stores.
/// Every producer of an `I128` here keeps the tag word **zero outside the
/// bytes the representation actually uses** — the tag byte alone for every tag
/// but one, and the tag plus scale, sign and mantissa-low for
/// [`Tag::Decimal`]. That is what lets `Emitter::emit_is_null` compare the
/// whole word against zero without masking: `Tag::Null` is the only tag whose
/// byte is zero, so no other value can produce a zero word.
#[must_use]
pub fn clif_ty(ty: Ty) -> Option<Type> {
    Some(match ty {
        Ty::Bool => types::I8,
        Ty::Int | Ty::Uint => types::I64,
        Ty::Float => types::F64,
        Ty::Str | Ty::Bytes | Ty::Array | Ty::Object | Ty::ClassDesc => types::I64,
        // The whole 16-byte tagged value, in a register pair — see this
        // function's own doc comment and `nvs_ir::Ty::Tagged`.
        //
        // A `decimal` is the same width for the same reason: it *is* a
        // `Value`, carrying `Tag::Decimal`, with its 96-bit mantissa spread
        // across the bytes a `Value` otherwise calls padding. That is what
        // makes `Tag`/`Untag` the identity on one — see
        // `nvs_runtime::decimal`'s own module docs for the layout.
        Ty::Tagged | Ty::Decimal => types::I128,
        // `null`'s payload is always zero, but it still travels in a register
        // like every other representation rather than in a shape of its own.
        Ty::Null => types::I64,
        // A by-reference parameter's staged-slot address — see `Ty::Ref`.
        Ty::Ref => types::I64,
        // `rule:enums/representation`: an enum value *is* its backing integer.
        Ty::Enum(_) => types::I64,
        Ty::Void => return None,
        _ => types::I64,
    })
}

/// The [`Tag`] a value of this representation carries once it is materialized
/// into a 16-byte [`nvs_runtime::Value`].
///
/// [`Ty::Str`] and [`Ty::Bytes`] share one *heap* shape and take two tags
/// anyway: the tag is what tells them apart once the static type is gone, and
/// `nvs-runtime`'s § *`bytes` is a tag, not a second heap shape* owns that
/// decision. Both still retain and release through `nvs_str_retain`/
/// `nvs_str_release`, which is why `Emitter::retain_release_symbol` keeps one
/// arm for the pair.
///
/// # Errors
///
/// [`CodegenError::Internal`] for [`Ty::Tagged`], whose tag is by
/// definition not a function of its static representation — it carries its own
/// (see [`clif_ty`]), so every path that materializes one reads it from the
/// value instead and never asks here. Reaching this arm means a caller
/// forgot that. Likewise for [`Ty::Void`], which is not a value at all.
/// Neither is a shape the language refuses, which is why neither is a
/// [`CodegenError::Unsupported`]: no program reaches either one, so no item
/// on `bun nv holes`'s worklist could ever close it.
/// An exception is an ordinary [`Tag::Object`], with no case of its own.
pub(crate) fn tag_of(ty: Ty) -> Result<Tag, CodegenError> {
    Ok(match ty {
        Ty::Bool => Tag::Bool,
        Ty::Int => Tag::Int,
        Ty::Uint => Tag::Uint,
        Ty::Float => Tag::Float,
        // Never actually reached: `Emitter::store_value` writes a `decimal`
        // as its two words, the way it writes a tagged value, because the tag
        // byte is only one of the sixteen this representation fills.
        Ty::Decimal => Tag::Decimal,
        Ty::Str => Tag::Str,
        Ty::Bytes => Tag::Bytes,
        // The one representation whose tag is the whole of it — see
        // `nvs_ir::Ty::Null`.
        Ty::Null => Tag::Null,
        // `rule:enums/representation`: a case in a tagged value carries an enum
        // tag of its own, one per backing type, so a `mixed` tells it from an
        // integer and judges it truthy. The payload is the backing integer's.
        Ty::Enum(EnumRepr::Int) => Tag::EnumInt,
        Ty::Enum(EnumRepr::Uint) => Tag::EnumUint,
        Ty::Array => Tag::Array,
        Ty::Object => Tag::Object,
        // Not an Novis value at all: a class descriptor rides in the payload
        // half of an otherwise-`null` slot, so nothing sweeping a `Value` can
        // mistake it for a heap reference. `nvs_runtime::object`'s module docs
        // own that decision; `nvs_ir::Ty::ClassDesc` restates the consequence.
        Ty::ClassDesc => Tag::Null,
        // Not an Novis value either, and for the same reason as `ClassDesc`
        // above: a by-reference parameter's staged-slot address rides in the
        // payload half of an otherwise-`null` slot, so nothing sweeping a
        // `Value` can mistake it for a heap reference. `nvs_ir::Ty::Ref` owns
        // the decision.
        Ty::Ref => Tag::Null,
        Ty::Tagged => {
            return Err(CodegenError::Internal(
                "asked for the static tag of a tagged value, which carries its own".to_owned(),
            ));
        }
        _ => {
            return Err(CodegenError::Internal(format!(
                "asked for the tag of representation {ty:?}, which is not a value"
            )));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_ir::lower::{FN_PARAM_TAG_ANY, param_tag_nibble};

    /// A closure object records one nibble per parameter and
    /// `nvs_runtime::call_closure` compares it against the tag an argument
    /// actually carries — so `nvs_ir::lower::param_tag_nibble` has to answer
    /// exactly the byte [`tag_of`] answers. Neither of those crates can name
    /// the other, and this one names both: a representation whose two answers
    /// drift apart makes every call through such a closure either refuse a
    /// good argument or accept a mismatched one, which is the priority-1 hole
    /// the nibble exists to close.
    #[test]
    fn param_tag_nibbles_are_the_runtime_tag_bytes() {
        let reprs = [
            Ty::Bool,
            Ty::Int,
            Ty::Uint,
            Ty::Float,
            Ty::Decimal,
            Ty::Null,
            Ty::Object,
            Ty::Str,
            Ty::Bytes,
            Ty::Array,
            Ty::Enum(EnumRepr::Int),
            Ty::Enum(EnumRepr::Uint),
            Ty::Ref,
            Ty::ClassDesc,
        ];
        for ty in reprs {
            let tag = tag_of(ty).expect("every representation above has a tag");
            assert_eq!(
                param_tag_nibble(ty),
                tag as u8,
                "the nibble recorded for a {ty:?} parameter is not the tag one carries"
            );
        }
    }

    /// The other half: the one nibble that is not a tag has to stay out of the
    /// roster, or a `mixed` parameter would read back as a demand for whatever
    /// tag happened to take that number.
    #[test]
    fn the_any_nibble_denotes_no_tag_at_all() {
        assert!(Tag::from_byte(FN_PARAM_TAG_ANY).is_none());
        assert_eq!(param_tag_nibble(Ty::Tagged), FN_PARAM_TAG_ANY);
        assert!(tag_of(Ty::Tagged).is_err());
    }
}
