//! Which anonymous functions satisfy a written `callable(...)` signature — the
//! table `rule:types/type-test`'s `$x is callable(int): string` is answered
//! from.
//!
//! # What a callable carries at run time, and what it does not
//!
//! A callable carries no signature, and this pass gives it none.
//! [`crate::ty::Ty::CallableSig`] is checked where the call is written, so the
//! compiled object holds only what the *dynamic* call path reads: a parameter
//! count and one tag nibble per parameter (`nvs_ir::lower`'s `FN_ARITY` and
//! `FN_PARAM_TAGS`). A tag word has no room for a class, a return type or a
//! union, so it cannot answer this question, and widening it would spend a
//! word per callable on every program to answer a test most programs never
//! write.
//!
//! What *does* name an anonymous function's signature exactly is the class
//! `nvs-ir` synthesizes for it: one per `fn` expression, so class identity decides the
//! signature. The answer is therefore an ordinary supertype — one marker class
//! per signature the program tests for, conformed to by every anonymous function whose
//! own signature is assignable to it, which makes the test the `callable`
//! row's own shape: the descriptor walk `$x is C` already emits, with no
//! field read and no second table. `nvs_ir::lower::CALLABLE_MARKER` is the same
//! device one step less specific.
//!
//! The relation is [`crate::expr::is_assignable`] and not a second reading of
//! it, so `$f is callable(int): string` answers `true` for exactly the values
//! a `callable(int): string` binding would accept — an anonymous function declaring a
//! wider parameter or a narrower return included, those being the directions
//! that rule makes sound.
//!
//! **What it spends** (`rule:programs/memory-priority`): nothing per request
//! or per task. Per compiled unit, one field-less descriptor per tested
//! signature, and one conformance edge per anonymous function that satisfies one.
//!
//! # Why it is one pass at the end
//!
//! [`crate::links`]' reason: an `is` in the entry file routinely tests a
//! signature an anonymous function in a later file satisfies, so a table filled as the
//! walk descends would answer by source order. Both halves are already
//! recorded by then — every anonymous function through
//! [`crate::expr_table::ExprTypeTable::record_callable_value`], and every
//! tested type through [`crate::expr_table::ExprInfo::TypeTest`].

use nvs_hir::ClassGraph;

use crate::expr_table::ExprTypeTable;
use crate::signatures::SignatureTable;
use crate::ty::{Ty, TypeId, TypeInterner};

/// The label of the marker class every callable satisfying `sig` conforms to.
///
/// Rendered from the signature rather than numbered, on
/// [`crate::derive::shape_class_label`]'s terms: `$` cannot start a Novis
/// identifier, so no declaration can collide with it, and a label a reader of
/// `--dump-ir` can match against the type they wrote is worth the length.
/// Interning is what makes the rendering one label per signature — two
/// spellings of one type share a [`TypeId`], and therefore a label.
#[must_use]
pub(crate) fn marker_label(sig: TypeId, interner: &TypeInterner) -> String {
    format!("${}", interner.describe(sig))
}

/// Records the marker class each tested written signature is walked for, and
/// which literals conform to it. The module docs own the design.
pub(crate) fn resolve(
    exprs: &mut ExprTypeTable,
    interner: &mut TypeInterner,
    graph: &ClassGraph,
    signatures: &SignatureTable,
) {
    let mut tested = Vec::new();
    for test in exprs.tested_types().collect::<Vec<_>>() {
        collect_signatures(test, interner, &mut tested);
    }
    if tested.is_empty() {
        return;
    }
    let values = exprs.callable_values().collect::<Vec<_>>();
    for sig in tested {
        let label = marker_label(sig, interner);
        let conformers = values
            .iter()
            .filter(|(_, held)| crate::expr::is_assignable(*held, sig, interner, graph, signatures))
            .map(|(span, _)| *span)
            .collect::<Vec<_>>();
        // Recorded whether or not anything conforms, for
        // `nvs_ir::lower::CALLABLE_MARKER`'s reason: a walk needs a descriptor
        // to compare against before it can answer `false`, so a unit that
        // emitted the marker only where something satisfied it would leave
        // `is` naming a class the unit does not declare.
        exprs.record_callable_conformance(sig, label, &conformers);
    }
}

/// Every written signature reachable inside `tested` as a type a value is
/// asked to *hold*, appended to `out` once each.
///
/// The walk is `nvs_ir::lower`'s `test_shape` recursion read the other way
/// round: that function builds a union's, an array's and a shape's rows out of
/// their members' own, so a signature written one level down inside any of
/// them is tested exactly as a bare one is and needs the same marker. A
/// signature's own parameters and return are not walked — those are checked
/// where a call through the value is written, and nothing asks whether a value
/// holds them.
fn collect_signatures(tested: TypeId, interner: &TypeInterner, out: &mut Vec<TypeId>) {
    match interner.get(tested) {
        Ty::CallableSig { .. } => {
            if !out.contains(&tested) {
                out.push(tested);
            }
        }
        Ty::Union(members) | Ty::Intersection(members) => {
            for member in members {
                collect_signatures(*member, interner, out);
            }
        }
        Ty::Array(element) => collect_signatures(*element, interner, out),
        Ty::Shape(fields) => {
            for field in fields {
                collect_signatures(field.ty, interner, out);
            }
        }
        _ => {}
    }
}
