//! Interned type representation (ADR 0007 §§ 3-5): every distinct type the
//! program uses gets one [`TypeId`], and two structurally identical types
//! share it — the "descriptors are interned process-wide" requirement,
//! scoped for this milestone to one [`TypeInterner`] per type-check run
//! rather than truly process-wide, since nothing yet needs a check run to
//! outlive the request that produced it.
//!
//! [`Ty`] mirrors [`mwl_syntax::ast::TypeAtom`] closely, but resolved: a
//! `self`/`static`/`parent`/`type`-alias/class-name atom has already been
//! turned into a concrete [`QName`] by [`crate::lower::lower_type`] before it
//! reaches here, and a union/intersection is already canonicalized —
//! flattened, deduplicated, and ordered by its members' own `TypeId`s (which
//! are themselves already canonical, since interning is structural) rather
//! than by source order, so `int|string` and `string|int|int` intern to the
//! same `TypeId` regardless of how each was written.

use mwl_hir::QName;
use rustc_hash::FxHashMap;

/// A type, interned. Cheap to copy and compare — two `TypeId`s are equal
/// exactly when the types they name are structurally identical.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TypeId(u32);

/// The resolved shape one [`TypeId`] names.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum Ty {
    /// `null`
    Null,
    /// `bool`
    Bool,
    /// `int`
    Int,
    /// `uint` — ADR 0007 § 4.
    Uint,
    /// `float`
    Float,
    /// `string`
    String,
    /// `bytes` — ADR 0009.
    Bytes,
    /// `tainted string` — ADR 0024 § 1.
    TaintedString,
    /// `tainted bytes` — ADR 0024 § 1.
    TaintedBytes,
    /// `array<T>`. A bare `array` is `Array` of the interned `Mixed` id —
    /// ADR 0007 § 3: "`array` with no argument is exactly `array<mixed>`."
    Array(TypeId),
    /// `object`
    Object,
    /// `mixed` — the one unchecked position.
    Mixed,
    /// `void` — return-position only.
    Void,
    /// `never` — return-position only.
    Never,
    /// `true`
    True,
    /// `false`
    False,
    /// `iterable`
    Iterable,
    /// `callable`
    Callable,
    /// A resolved class, interface or trait name — the type grammar does not
    /// distinguish them (ADR 0007 § 3); which one `QName` names is a
    /// question for [`mwl_hir::SymbolTable`], not this representation.
    Class(QName),
    /// A resolved enum name (ADR 0010).
    Enum(QName),
    /// `A|B|...` — flattened, deduplicated, and sorted by member `TypeId`.
    /// Always at least two members; a one-member union collapses to that
    /// member directly (see [`TypeInterner::make_union`]).
    Union(Vec<TypeId>),
    /// `A&B&...` — same canonicalization as [`Self::Union`].
    Intersection(Vec<TypeId>),
}

/// Interns [`Ty`] values, giving structurally identical types the same
/// [`TypeId`].
#[derive(Debug, Default)]
pub struct TypeInterner {
    types: Vec<Ty>,
    index: FxHashMap<Ty, TypeId>,
}

impl TypeInterner {
    /// An interner with nothing interned yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Interns `ty`, returning its `TypeId` — the same one a structurally
    /// identical `Ty` interned earlier already has.
    pub fn intern(&mut self, ty: Ty) -> TypeId {
        if let Some(&id) = self.index.get(&ty) {
            return id;
        }
        let id = TypeId(
            u32::try_from(self.types.len())
                .expect("far fewer than u32::MAX types are ever interned in one compilation"),
        );
        self.types.push(ty.clone());
        self.index.insert(ty, id);
        id
    }

    /// The `Ty` a `TypeId` names.
    ///
    /// # Panics
    /// Panics if `id` was not produced by this interner.
    #[must_use]
    pub fn get(&self, id: TypeId) -> &Ty {
        &self.types[id.0 as usize]
    }

    /// Builds a canonicalized union out of already-interned members:
    /// flattens a member that is itself a union, sorts and deduplicates by
    /// `TypeId`, and collapses to the single member directly if only one
    /// remains — ADR 0007 § 3's canonicalization, generalised to also cover
    /// the degenerate one-member case a `?T` or `T|T` source expression can
    /// produce.
    pub fn make_union(&mut self, members: impl IntoIterator<Item = TypeId>) -> TypeId {
        let mut ids: Vec<TypeId> = Vec::new();
        for id in members {
            match self.get(id) {
                Ty::Union(inner) => ids.extend(inner.iter().copied()),
                _ => ids.push(id),
            }
        }
        ids.sort_by_key(|id| id.0);
        ids.dedup();
        if ids.len() == 1 {
            return ids[0];
        }
        self.intern(Ty::Union(ids))
    }

    /// Same canonicalization as [`Self::make_union`], for `&`.
    pub fn make_intersection(&mut self, members: impl IntoIterator<Item = TypeId>) -> TypeId {
        let mut ids: Vec<TypeId> = Vec::new();
        for id in members {
            match self.get(id) {
                Ty::Intersection(inner) => ids.extend(inner.iter().copied()),
                _ => ids.push(id),
            }
        }
        ids.sort_by_key(|id| id.0);
        ids.dedup();
        if ids.len() == 1 {
            return ids[0];
        }
        self.intern(Ty::Intersection(ids))
    }

    /// Every union member of `id`, or `[id]` itself if it does not name a
    /// union — a plain type satisfies "a union of one" for callers that want
    /// to treat both shapes uniformly.
    #[must_use]
    pub fn union_members(&self, id: TypeId) -> Vec<TypeId> {
        match self.get(id) {
            Ty::Union(members) => members.clone(),
            _ => vec![id],
        }
    }

    /// A human-readable rendering of `id`, for a diagnostic message —
    /// `"array<int|string>"`, `"App\\User"`, and so on.
    #[must_use]
    pub fn describe(&self, id: TypeId) -> String {
        match self.get(id) {
            Ty::Null => "null".to_owned(),
            Ty::Bool => "bool".to_owned(),
            Ty::Int => "int".to_owned(),
            Ty::Uint => "uint".to_owned(),
            Ty::Float => "float".to_owned(),
            Ty::String => "string".to_owned(),
            Ty::Bytes => "bytes".to_owned(),
            Ty::TaintedString => "tainted string".to_owned(),
            Ty::TaintedBytes => "tainted bytes".to_owned(),
            Ty::Array(elem) => format!("array<{}>", self.describe(*elem)),
            Ty::Object => "object".to_owned(),
            Ty::Mixed => "mixed".to_owned(),
            Ty::Void => "void".to_owned(),
            Ty::Never => "never".to_owned(),
            Ty::True => "true".to_owned(),
            Ty::False => "false".to_owned(),
            Ty::Iterable => "iterable".to_owned(),
            Ty::Callable => "callable".to_owned(),
            Ty::Class(q) | Ty::Enum(q) => q.to_string(),
            Ty::Union(members) => members
                .iter()
                .map(|m| self.describe(*m))
                .collect::<Vec<_>>()
                .join("|"),
            Ty::Intersection(members) => members
                .iter()
                .map(|m| self.describe(*m))
                .collect::<Vec<_>>()
                .join("&"),
        }
    }

    /// The interned `null` singleton.
    #[must_use]
    pub fn null(&mut self) -> TypeId {
        self.intern(Ty::Null)
    }

    /// The interned `bool` singleton.
    #[must_use]
    pub fn bool_ty(&mut self) -> TypeId {
        self.intern(Ty::Bool)
    }

    /// The interned `int` singleton.
    #[must_use]
    pub fn int(&mut self) -> TypeId {
        self.intern(Ty::Int)
    }

    /// The interned `uint` singleton.
    #[must_use]
    pub fn uint(&mut self) -> TypeId {
        self.intern(Ty::Uint)
    }

    /// The interned `float` singleton.
    #[must_use]
    pub fn float(&mut self) -> TypeId {
        self.intern(Ty::Float)
    }

    /// The interned `string` singleton.
    #[must_use]
    pub fn string(&mut self) -> TypeId {
        self.intern(Ty::String)
    }

    /// The interned `bytes` singleton.
    #[must_use]
    pub fn bytes(&mut self) -> TypeId {
        self.intern(Ty::Bytes)
    }

    /// The interned `tainted string` singleton.
    #[must_use]
    pub fn tainted_string(&mut self) -> TypeId {
        self.intern(Ty::TaintedString)
    }

    /// The interned `tainted bytes` singleton.
    #[must_use]
    pub fn tainted_bytes(&mut self) -> TypeId {
        self.intern(Ty::TaintedBytes)
    }

    /// The interned `object` singleton.
    #[must_use]
    pub fn object(&mut self) -> TypeId {
        self.intern(Ty::Object)
    }

    /// The interned `mixed` singleton.
    #[must_use]
    pub fn mixed(&mut self) -> TypeId {
        self.intern(Ty::Mixed)
    }

    /// The interned `void` singleton.
    #[must_use]
    pub fn void(&mut self) -> TypeId {
        self.intern(Ty::Void)
    }

    /// The interned `never` singleton.
    #[must_use]
    pub fn never(&mut self) -> TypeId {
        self.intern(Ty::Never)
    }

    /// The interned `true` singleton.
    #[must_use]
    pub fn true_ty(&mut self) -> TypeId {
        self.intern(Ty::True)
    }

    /// The interned `false` singleton.
    #[must_use]
    pub fn false_ty(&mut self) -> TypeId {
        self.intern(Ty::False)
    }

    /// The interned `iterable` singleton.
    #[must_use]
    pub fn iterable(&mut self) -> TypeId {
        self.intern(Ty::Iterable)
    }

    /// The interned `callable` singleton.
    #[must_use]
    pub fn callable(&mut self) -> TypeId {
        self.intern(Ty::Callable)
    }

    /// Interns `array<elem>`.
    #[must_use]
    pub fn array(&mut self, elem: TypeId) -> TypeId {
        self.intern(Ty::Array(elem))
    }

    /// Interns a resolved class/interface/trait name.
    #[must_use]
    pub fn class(&mut self, qname: QName) -> TypeId {
        self.intern(Ty::Class(qname))
    }

    /// Interns a resolved enum name.
    #[must_use]
    pub fn enum_(&mut self, qname: QName) -> TypeId {
        self.intern(Ty::Enum(qname))
    }

    /// Whether `id` is `null` itself, or a union with `null` as one of its
    /// members — i.e. whether it was written with a leading `?` (or expands
    /// to one through a `type` alias). ADR 0022 § 1: this is the one thing
    /// that exempts a property from that ADR's definite-assignment
    /// obligation, since nullability already promises "may legitimately hold
    /// no value."
    #[must_use]
    pub fn is_nullable(&self, id: TypeId) -> bool {
        match self.get(id) {
            Ty::Null => true,
            Ty::Union(members) => members.iter().any(|m| matches!(self.get(*m), Ty::Null)),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_atoms_share_one_id() {
        let mut i = TypeInterner::new();
        assert_eq!(i.int(), i.int());
    }

    #[test]
    fn a_union_is_order_insensitive() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let string = i.string();
        let a = i.make_union([int, string]);
        let b = i.make_union([string, int]);
        assert_eq!(a, b);
    }

    #[test]
    fn a_union_deduplicates_repeated_members() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let string = i.string();
        let a = i.make_union([int, string, int]);
        let b = i.make_union([int, string]);
        assert_eq!(a, b);
    }

    #[test]
    fn a_union_of_one_collapses_to_that_member() {
        let mut i = TypeInterner::new();
        let int = i.int();
        assert_eq!(i.make_union([int, int]), int);
    }

    #[test]
    fn a_nested_union_flattens() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let string = i.string();
        let float = i.float();
        let inner = i.make_union([int, string]);
        let outer = i.make_union([inner, float]);
        let direct = i.make_union([int, string, float]);
        assert_eq!(outer, direct);
    }

    #[test]
    fn describe_renders_a_union_with_a_pipe() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let float = i.float();
        let u = i.make_union([int, float]);
        assert_eq!(i.describe(u), "int|float");
    }

    #[test]
    fn describe_renders_nested_arrays() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let inner = i.array(int);
        let outer = i.array(inner);
        assert_eq!(i.describe(outer), "array<array<int>>");
    }
}
