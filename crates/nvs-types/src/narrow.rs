//! The type the editor's narrowing action writes into an `array<mixed>`
//! annotation (`rule:ide/narrow-an-annotation-to-its-literal`).
//!
//! The literal's type comes from the synthesis `var` already runs
//! (`rule:ide/no-compile-path-calls-the-synthesis`), which
//! [`crate::expr_table::ExprTypeTable::literal_type`] records. This module
//! turns that type into the one an annotation states:
//! [`narrowest`] widens each element to its base
//! (`rule:ide/narrowest-means-narrowest-base-type`) and declines a type a
//! person could not have written, and [`written`] prints it.
//!
//! Only `nvs-lsp` calls either function. A test below holds every other crate
//! and this one to that, so no compile path changes what it types.
//!
//! # What it spends
//!
//! Nothing at runtime and nothing on a compile. Per offer, one walk of the
//! literal's type and the interned widened types, dropped with the analysis.

use crate::lower::MAX_ARRAY_DEPTH;
use crate::ty::{Ty, TypeId, TypeInterner};

/// The type an `array<mixed>` annotation narrows to, given `synthesized`, the
/// type the synthesis gave its literal.
///
/// Every single-value type is widened to its base and every enum case to its
/// enum, at every depth, and a union is canonicalised again after it. `None`
/// where the answer would be no narrower or not writable: a `mixed` anywhere
/// in it, an element type no annotation can state, an array nested past
/// [`MAX_ARRAY_DEPTH`], and the empty literal's `array<never>`.
pub fn narrowest(interner: &mut TypeInterner, synthesized: TypeId) -> Option<TypeId> {
    let Ty::Array(elem) = *interner.get(synthesized) else {
        return None;
    };
    let elem = widened(interner, elem, 1)?;
    if matches!(interner.get(elem), Ty::Never) {
        return None;
    }
    Some(interner.array(elem))
}

/// `id` with every single-value type in it widened to its base, or `None`
/// where it holds something [`narrowest`] declines. `depth` counts the arrays
/// around `id`.
fn widened(interner: &mut TypeInterner, id: TypeId, depth: u32) -> Option<TypeId> {
    match interner.get(id).clone() {
        Ty::Array(elem) => {
            if depth >= MAX_ARRAY_DEPTH {
                return None;
            }
            let elem = widened(interner, elem, depth + 1)?;
            Some(interner.array(elem))
        }
        Ty::Union(members) => {
            let members = members
                .into_iter()
                .map(|member| widened(interner, member, depth))
                .collect::<Option<Vec<_>>>()?;
            Some(interner.make_union(members))
        }
        Ty::Class(_, args) => {
            for arg in args {
                widened(interner, arg, depth)?;
            }
            Some(id)
        }
        Ty::SingleValueString(_)
        | Ty::SingleValueInt(_)
        | Ty::True
        | Ty::False
        | Ty::EnumCase(..) => Some(interner.single_value_base(id)),
        Ty::Null
        | Ty::Bool
        | Ty::Int
        | Ty::Uint
        | Ty::Float
        | Ty::Decimal
        | Ty::String
        | Ty::Bytes
        | Ty::TaintedString
        | Ty::TaintedBytes
        | Ty::SecretString
        | Ty::SecretBytes
        | Ty::SecretTaintedString
        | Ty::SecretTaintedBytes
        | Ty::ClassRef(_)
        | Ty::Object
        | Ty::Never
        | Ty::Iterable
        | Ty::Callable
        | Ty::Enum(..) => Some(id),
        // `mixed` is the answer that is already written, and an error's
        // stand-in prints as `mixed` too. The rest are types the checker
        // gives a value that no annotation is written as, or one whose
        // printed form is not the syntax a declaration uses.
        Ty::Mixed
        | Ty::Error
        | Ty::Void
        | Ty::PropertyKey(_)
        | Ty::PropertyPath(_)
        | Ty::StringHoles { .. }
        | Ty::CallableSig { .. }
        | Ty::ShapeOfCallables(_)
        | Ty::Shape(_)
        | Ty::CoreShape(_)
        | Ty::Intersection(_)
        | Ty::TypeVar(_) => None,
    }
}

/// `id` as an annotation writes it: [`TypeInterner::describe`]'s text, with a
/// union of one type and `null` written `?T`, at every depth.
///
/// A union's members are written in the order of their text. The interner
/// orders them by when each type was first interned, which differs between
/// two documents, and the edit is text a person reads in a diff.
///
/// A class or an enum is its fully qualified name. The editor writes each one
/// the way the file it edits names it.
#[must_use]
pub fn written(interner: &TypeInterner, id: TypeId) -> String {
    match interner.get(id) {
        Ty::Array(elem) => format!("array<{}>", written(interner, *elem)),
        Ty::Union(members) => {
            if let [a, b] = members.as_slice() {
                match (interner.get(*a), interner.get(*b)) {
                    (Ty::Null, Ty::Union(_)) | (Ty::Union(_), Ty::Null) => {}
                    (Ty::Null, _) => return format!("?{}", written(interner, *b)),
                    (_, Ty::Null) => return format!("?{}", written(interner, *a)),
                    _ => {}
                }
            }
            let mut members: Vec<String> = members
                .iter()
                .map(|member| written(interner, *member))
                .collect();
            members.sort();
            members.join("|")
        }
        _ => interner.describe(id),
    }
}

#[cfg(test)]
mod tests {
    use nvs_hir::QName;

    use super::{narrowest, written};
    use crate::enums::EnumBacking;
    use crate::ty::{TypeId, TypeInterner};

    /// What [`narrowest`] gives the literal type `array<elem>`, printed.
    fn offered(interner: &mut TypeInterner, elem: TypeId) -> Option<String> {
        let synthesized = interner.array(elem);
        narrowest(interner, synthesized).map(|id| written(interner, id))
    }

    #[test]
    fn a_homogeneous_nest_narrows_at_three_levels() {
        let mut interner = TypeInterner::new();
        let float = interner.float();
        let inner = interner.array(float);
        let middle = interner.array(inner);
        assert_eq!(
            offered(&mut interner, middle).as_deref(),
            Some("array<array<array<float>>>")
        );
    }

    #[test]
    fn a_heterogeneous_literal_narrows_to_the_canonical_union() {
        let mut interner = TypeInterner::new();
        let int = interner.int();
        let string = interner.string();
        let union = interner.make_union([string, int]);
        let reversed = interner.make_union([int, string]);
        assert_eq!(union, reversed);
        assert_eq!(
            offered(&mut interner, union).as_deref(),
            Some("array<int|string>")
        );

        let null = interner.null();
        let maybe = interner.make_union([int, null]);
        assert_eq!(
            offered(&mut interner, maybe).as_deref(),
            Some("array<?int>")
        );
    }

    #[test]
    fn an_integer_literal_widens_to_int() {
        let mut interner = TypeInterner::new();
        let one = interner.single_value_int(1);
        let three = interner.single_value_int(3);
        let union = interner.make_union([one, three]);
        assert_eq!(offered(&mut interner, union).as_deref(), Some("array<int>"));
    }

    #[test]
    fn an_enum_case_widens_to_its_enum() {
        let mut interner = TypeInterner::new();
        let rank = QName::parse("Cards\\Rank");
        let ace = interner.enum_case(rank.clone(), EnumBacking::Int, "Ace");
        let king = interner.enum_case(rank, EnumBacking::Int, "King");
        let union = interner.make_union([ace, king]);
        assert_eq!(
            offered(&mut interner, union).as_deref(),
            Some("array<Cards\\Rank>")
        );
    }

    #[test]
    fn an_empty_literal_is_offered_nothing() {
        let mut interner = TypeInterner::new();
        let never = interner.never();
        assert_eq!(offered(&mut interner, never), None);
    }

    #[test]
    fn a_mixed_element_is_offered_nothing() {
        let mut interner = TypeInterner::new();
        let mixed = interner.mixed();
        let int = interner.int();
        let union = interner.make_union([int, mixed]);
        assert_eq!(offered(&mut interner, mixed), None);
        assert_eq!(offered(&mut interner, union), None);
        let nested = interner.array(mixed);
        assert_eq!(offered(&mut interner, nested), None);
    }

    #[test]
    fn a_literal_past_depth_32_is_offered_nothing() {
        let mut interner = TypeInterner::new();
        let mut elem = interner.int();
        for _ in 0..30 {
            elem = interner.array(elem);
        }
        assert!(
            offered(&mut interner, elem).is_some(),
            "31 arrays deep is written"
        );
        elem = interner.array(elem);
        assert!(
            offered(&mut interner, elem).is_some(),
            "32 arrays deep is written"
        );
        elem = interner.array(elem);
        assert_eq!(offered(&mut interner, elem), None, "33 arrays deep is not");
    }

    /// `rule:ide/no-compile-path-calls-the-synthesis`: the editor's narrowing
    /// reads what the synthesis recorded, and nothing that compiles a program
    /// calls this module. Every crate but `nvs-lsp` is searched, this file
    /// excepted.
    #[test]
    fn only_the_editor_calls_the_narrowing() {
        let crates = nvs_repo::path("crates");
        let mut callers = Vec::new();
        let mut dirs = vec![crates.clone()];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                if path.is_dir() {
                    if name != "target" && path != crates.join("nvs-lsp") {
                        dirs.push(path);
                    }
                    continue;
                }
                if name == "narrow.rs" || path.extension().is_none_or(|ext| ext != "rs") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                if text.contains("narrow::narrowest") || text.contains("narrow::written") {
                    callers.push(path.display().to_string());
                }
            }
        }
        assert!(
            callers.is_empty(),
            "a compile path reads the narrowing: {callers:?}"
        );
    }
}
