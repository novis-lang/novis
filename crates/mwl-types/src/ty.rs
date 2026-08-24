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
    /// `decimal` — ADR 0054 § 1: a 96-bit signed mantissa and a scale of 0 to
    /// 28. Distinct from [`Self::Float`] and never assignable to or from it,
    /// which is what makes § 3's `decimal ⊕ float` compile error expressible.
    Decimal,
    /// `string`
    String,
    /// `bytes` — ADR 0009.
    Bytes,
    /// `tainted string` — ADR 0024 § 1.
    TaintedString,
    /// `tainted bytes` — ADR 0024 § 1.
    TaintedBytes,
    /// `secret string` — ADR 0033 § 1. Independent of `tainted`: a value can
    /// carry either qualifier, both, or neither.
    SecretString,
    /// `secret bytes` — ADR 0033 § 1, the `Bytes` counterpart of
    /// [`Self::SecretString`].
    SecretBytes,
    /// `secret tainted string` — ADR 0033 § 1: both qualifiers composed.
    SecretTaintedString,
    /// `secret tainted bytes` — ADR 0033 § 1, the `Bytes` counterpart of
    /// [`Self::SecretTaintedString`].
    SecretTaintedBytes,
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
    /// `callable`, plus the name of the type variable its **result** binds —
    /// `U` in `Core\Arr::map(array<T> $a, callable $fn): array<U>`.
    ///
    /// The third type in this enum no source text can spell, and the only one
    /// that is not really a type at all: it accepts exactly what
    /// [`Self::Callable`] accepts (ADR 0027 § 2 keeps a `callable` opaque, and
    /// this changes nothing about that), and exists only to say *where a
    /// variable comes from* at a position whose own type cannot say it. It
    /// enters the interner only from `mwl_stdlib::registry`'s `CoreTy::CallableTo`
    /// through [`crate::core_lib`].
    ///
    /// Like [`Self::TypeVar`], it never survives a call site:
    /// [`crate::generics::substitute`] rewrites it to [`Self::Callable`], so
    /// `mwl-ir` and every diagnostic only ever meet the plain type. That is also
    /// why [`TypeInterner::describe`] renders it as `callable` — the variable
    /// name is a fact about the registry row, and a message quoting it would be
    /// naming something no program can write.
    CallableTo(String),
    /// A resolved class or interface name, plus the type arguments it was
    /// written with — the type grammar does not distinguish a class from an
    /// interface (ADR 0007 § 3); which one `QName` names is a question for
    /// [`mwl_hir::SymbolTable`], not this representation.
    ///
    /// The argument list is empty for all but two names.
    /// [ADR 0053](../../../docs/adr/0053-iteration-and-generators.md) § 2
    /// lets a *compiler-owned* generic interface be written at a concrete
    /// type — `Iterator<int>` — and `mwl_hir::interfaces::RESERVED` is the
    /// closed roster of what may be. Anything else written with arguments is
    /// refused by [`crate::lower`] before it ever interns, so a non-empty
    /// list here always names one of those two interfaces.
    ///
    /// Interning is structural, so `Iterator<int>` and `Iterator<string>` are
    /// two distinct `TypeId`s while `Counter` and `Counter` are one — which
    /// is the whole point of carrying the arguments in the type rather than
    /// beside it. They are erased at the `mwl-ir` boundary
    /// (`mwl_ir::lower::lower_checked_ty` maps every class to one pointer
    /// type), exactly as a [`Self::TypeVar`] is erased at a call site: a type
    /// argument constrains what the checker accepts and never what the
    /// runtime stores.
    Class(QName, Vec<TypeId>),
    /// A resolved enum name (ADR 0010), together with the underlying integer
    /// type its cases are constants of.
    ///
    /// The backing type rides in the type itself rather than in a side table
    /// because it *is* part of what the type is: ADR 0010 § 2 gives every enum
    /// exactly one underlying integer type, and § 6 makes an enum value that
    /// integer's representation with names attached. Carrying it here is what
    /// lets `mwl-ir` lower an enum-typed binding to a machine integer without
    /// re-resolving the declaration (`mwl_ir::lower::lower_checked_ty`). It is
    /// a function of the `QName`, so it never splits one enum into two
    /// interned types.
    Enum(QName, crate::enums::EnumBacking),
    /// `{name: T, ...}` — ADR 0036 § 3, MWL's one structurally-checked type.
    /// Fields are sorted by name (see [`TypeInterner::shape`]) so two shapes
    /// naming the same fields in a different written order intern to the
    /// same `TypeId`; unlike [`Self::Union`]/[`Self::Intersection`] there is
    /// no flattening to do, since a shape field's type is never itself
    /// required to be a shape.
    Shape(Vec<(String, TypeId)>),
    /// ADR 0063 R2's trailing options bag — `{step?: int}`, one entry per
    /// declared option.
    ///
    /// The second type in this enum no source text can spell (see
    /// [`Self::TypeVar`] for the first): it only ever enters the interner from
    /// `mwl_stdlib::registry`'s `CoreTy::Options` through [`crate::core_lib`].
    /// A *value* of this type is still written by hand — an ADR 0036 object
    /// literal at the call site — but the type itself is never written, which
    /// is why there is no `?` in the surface type grammar.
    ///
    /// Deliberately not a [`Self::Shape`]. A shape is checked by ADR 0036 § 3's
    /// **width** subtyping, which accepts a field the target does not name; an
    /// options bag must refuse one, because a mistyped option name that is
    /// silently ignored is exactly the failure ADR 0063 R2 exists to prevent.
    /// [`crate::expr`] owns that check.
    ///
    /// Fields keep their **declared order** rather than being sorted the way
    /// [`TypeInterner::shape`] sorts a shape's: that order is the order
    /// `mwl_ir::lower::lower_call_args` flattens the bag into ABI arguments,
    /// so two members whose options differ only in order are genuinely two
    /// different types and must not intern to one.
    Options(Vec<(String, TypeId)>),
    /// `A|B|...` — flattened, deduplicated, and sorted by member `TypeId`.
    /// Always at least two members; a one-member union collapses to that
    /// member directly (see [`TypeInterner::make_union`]).
    Union(Vec<TypeId>),
    /// `A&B&...` — same canonicalization as [`Self::Union`].
    Intersection(Vec<TypeId>),
    /// A *type variable*, named — `T` in `Core\Arr::count(array<T> $a): uint`.
    ///
    /// The one type in this enum no source text can spell. ADR 0007 parks
    /// user-declared generics and `docs/agent/loop-goal.md` keeps type variables
    /// compiler-owned, so a `TypeVar` only ever enters the interner from
    /// `mwl_stdlib::registry`'s `Core` signatures ([`crate::core_lib`]) or
    /// ADR 0053 § 1's two iteration interfaces ([`crate::iter_lib`]) —
    /// [`crate::lower`] has no arm producing one, which is what makes that a
    /// property of the code rather than a convention. `Iterator<int>` written
    /// in source produces [`Self::Class`] with a concrete argument, never
    /// this.
    ///
    /// It never survives a call site. [`crate::signatures::MethodSig`]'s own
    /// docs own the substitution rule: a generic signature is unified against
    /// the actual argument types and rewritten before anything checks an
    /// argument or records a `ResolvedCall`, so every later pass — including
    /// every `mwl-ir` lowering — only ever sees concrete types. A variable
    /// that no argument bound is the one exception, and substitutes to
    /// [`Self::Mixed`]: the honest answer for "this position's type is
    /// unconstrained by the call," and the only one that keeps a later pass
    /// from meeting a variable it has no rule for.
    TypeVar(String),
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
            Ty::Decimal => "decimal".to_owned(),
            Ty::String => "string".to_owned(),
            Ty::Bytes => "bytes".to_owned(),
            Ty::TaintedString => "tainted string".to_owned(),
            Ty::TaintedBytes => "tainted bytes".to_owned(),
            Ty::SecretString => "secret string".to_owned(),
            Ty::SecretBytes => "secret bytes".to_owned(),
            Ty::SecretTaintedString => "secret tainted string".to_owned(),
            Ty::SecretTaintedBytes => "secret tainted bytes".to_owned(),
            Ty::Array(elem) => format!("array<{}>", self.describe(*elem)),
            Ty::Object => "object".to_owned(),
            Ty::Mixed => "mixed".to_owned(),
            Ty::Void => "void".to_owned(),
            Ty::Never => "never".to_owned(),
            Ty::True => "true".to_owned(),
            Ty::False => "false".to_owned(),
            Ty::Iterable => "iterable".to_owned(),
            // Deliberately the same rendering as `Ty::Callable` — see that
            // variant's own doc comment for why the bound variable's name is
            // never quoted at a user.
            Ty::Callable | Ty::CallableTo(_) => "callable".to_owned(),
            Ty::Enum(q, _) => q.to_string(),
            Ty::Class(q, args) if args.is_empty() => q.to_string(),
            Ty::Class(q, args) => {
                let inner = args
                    .iter()
                    .map(|arg| self.describe(*arg))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{q}<{inner}>")
            }
            Ty::Shape(fields) => {
                let inner = fields
                    .iter()
                    .map(|(name, ty)| format!("{name}: {}", self.describe(*ty)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{inner}}}")
            }
            Ty::Options(options) => {
                let inner = options
                    .iter()
                    .map(|(name, ty)| format!("{name}?: {}", self.describe(*ty)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{inner}}}")
            }
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
            Ty::TypeVar(name) => name.clone(),
        }
    }

    /// The interned type variable named `name` — see [`Ty::TypeVar`], which
    /// owns why nothing outside a `Core` signature ever calls this.
    pub fn type_var(&mut self, name: impl Into<String>) -> TypeId {
        self.intern(Ty::TypeVar(name.into()))
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

    /// The interned `decimal` singleton — ADR 0054 § 1.
    #[must_use]
    pub fn decimal(&mut self) -> TypeId {
        self.intern(Ty::Decimal)
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

    /// The interned `secret string` singleton.
    #[must_use]
    pub fn secret_string(&mut self) -> TypeId {
        self.intern(Ty::SecretString)
    }

    /// The interned `secret bytes` singleton.
    #[must_use]
    pub fn secret_bytes(&mut self) -> TypeId {
        self.intern(Ty::SecretBytes)
    }

    /// The interned `secret tainted string` singleton.
    #[must_use]
    pub fn secret_tainted_string(&mut self) -> TypeId {
        self.intern(Ty::SecretTaintedString)
    }

    /// The interned `secret tainted bytes` singleton.
    #[must_use]
    pub fn secret_tainted_bytes(&mut self) -> TypeId {
        self.intern(Ty::SecretTaintedBytes)
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

    /// The interned `callable` that binds `name` from its result — see
    /// [`Ty::CallableTo`], which owns why nothing outside a `Core` signature
    /// ever calls this.
    pub fn callable_to(&mut self, name: impl Into<String>) -> TypeId {
        self.intern(Ty::CallableTo(name.into()))
    }

    /// Interns `array<elem>`.
    #[must_use]
    pub fn array(&mut self, elem: TypeId) -> TypeId {
        self.intern(Ty::Array(elem))
    }

    /// Interns a resolved class/interface name with no type arguments —
    /// every name but ADR 0053 § 2's two generic interfaces.
    #[must_use]
    pub fn class(&mut self, qname: QName) -> TypeId {
        self.intern(Ty::Class(qname, Vec::new()))
    }

    /// Interns a resolved interface name applied to concrete type arguments
    /// — `Iterator<int>`. See [`Ty::Class`] for the closed set of names this
    /// is reachable for.
    #[must_use]
    pub fn generic_class(&mut self, qname: QName, args: Vec<TypeId>) -> TypeId {
        self.intern(Ty::Class(qname, args))
    }

    /// Interns a resolved enum name together with its backing type — see
    /// [`Ty::Enum`] for why the two travel as one.
    #[must_use]
    pub fn enum_(&mut self, qname: QName, backing: crate::enums::EnumBacking) -> TypeId {
        self.intern(Ty::Enum(qname, backing))
    }

    /// Interns `{name: T, ...}` — ADR 0036 § 3. Sorts `fields` by name first,
    /// so `{x: int, y: string}` and `{y: string, x: int}` intern to the same
    /// `TypeId` regardless of how each was written (same canonicalization
    /// idea as [`Self::make_union`], applied to field order instead of
    /// member order).
    #[must_use]
    pub fn shape(&mut self, mut fields: Vec<(String, TypeId)>) -> TypeId {
        fields.sort_by(|a, b| a.0.cmp(&b.0));
        self.intern(Ty::Shape(fields))
    }

    /// Interns ADR 0063 R2's options bag — see [`Ty::Options`], which owns why
    /// `options` is interned in the order given rather than sorted the way
    /// [`Self::shape`] sorts, and why nothing outside [`crate::core_lib`]
    /// calls this.
    #[must_use]
    pub fn options(&mut self, options: Vec<(String, TypeId)>) -> TypeId {
        self.intern(Ty::Options(options))
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
    fn a_shape_is_order_insensitive() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let string = i.string();
        let a = i.shape(vec![("x".to_owned(), int), ("y".to_owned(), string)]);
        let b = i.shape(vec![("y".to_owned(), string), ("x".to_owned(), int)]);
        assert_eq!(a, b);
    }

    #[test]
    fn describe_renders_a_shape_with_its_fields() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let s = i.shape(vec![("x".to_owned(), int)]);
        assert_eq!(i.describe(s), "{x: int}");
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
