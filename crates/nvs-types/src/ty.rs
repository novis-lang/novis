//! Interned type representation (`rule:types/grammar`, `rule:types/arithmetic` and `rule:types/arrays`): every distinct type the
//! program uses gets one [`TypeId`], and two structurally identical types
//! share it — the "descriptors are interned process-wide" requirement,
//! scoped for this milestone to one [`TypeInterner`] per type-check run
//! rather than truly process-wide, since nothing yet needs a check run to
//! outlive the request that produced it.
//!
//! [`Ty`] mirrors [`nvs_syntax::ast::TypeAtom`] closely, but resolved: a
//! `self`/`static`/`parent`/`type`-alias/class-name atom has already been
//! turned into a concrete [`QName`] by [`crate::lower::lower_type`] before it
//! reaches here, and a union/intersection is already canonicalized —
//! flattened, deduplicated, and ordered by its members' own `TypeId`s (which
//! are themselves already canonical, since interning is structural) rather
//! than by source order, so `int|string` and `string|int|int` intern to the
//! same `TypeId` regardless of how each was written.

use nvs_hir::QName;
use nvs_stdlib::registry::Qual;
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
    /// `uint` — `rule:types/arithmetic`.
    Uint,
    /// `float`
    Float,
    /// `decimal` — `rule:types/decimal`: a 96-bit signed mantissa and a scale of 0 to
    /// 28. Distinct from [`Self::Float`] and never assignable to or from it,
    /// which is what makes § 3's `decimal ⊕ float` compile error expressible.
    Decimal,
    /// `string`
    String,
    /// `bytes` — `rule:types/bytes`.
    Bytes,
    /// `tainted string` — `rule:security/tainted-qualifier`.
    TaintedString,
    /// `tainted bytes` — `rule:security/tainted-qualifier`.
    TaintedBytes,
    /// `secret string` — `rule:security/secret-qualifier`. Independent of `tainted`: a value can
    /// carry either qualifier, both, or neither.
    SecretString,
    /// `secret bytes` — `rule:security/secret-qualifier`, the `Bytes` counterpart of
    /// [`Self::SecretString`].
    SecretBytes,
    /// `secret tainted string` — `rule:security/secret-qualifier`: both qualifiers composed.
    SecretTaintedString,
    /// `secret tainted bytes` — `rule:security/secret-qualifier`, the `Bytes` counterpart of
    /// [`Self::SecretTaintedString`].
    SecretTaintedBytes,
    /// `array<T>`. A bare `array` is `Array` of the interned `Mixed` id —
    /// `rule:types/grammar`: "`array` with no argument is exactly `array<mixed>`."
    Array(TypeId),
    /// `class<T>` — `rule:types/class-reference`'s class reference. Its *value* is a run-time
    /// class descriptor, and the argument bounds which descriptors it can be:
    /// `T` itself or any class that is a `T`.
    ///
    /// The argument is always a [`Self::Class`] id. `crate::lower`'s atom
    /// refuses anything else where it is written
    /// (`E_CLASS_REF_ARGUMENT_NOT_A_CLASS`), so nothing downstream has to ask
    /// whether a class reference's argument names a class before reading it as
    /// one.
    ///
    /// **Covariant in that argument** (`rule:types/class-reference-variance`), which makes it the
    /// second generic name in the language that is: `class<Dog>` widens to
    /// `class<Animal>` wherever `Dog` widens to `Animal`, and never back. It
    /// needs none of the reasoning [`Self::Array`]'s covariance needs, because
    /// the trap that reasoning answers cannot be set here — a descriptor has
    /// no write side at all, so the argument is a pure output position and
    /// there is nothing a widened view could store for a narrow one to read
    /// back.
    ClassRef(TypeId),
    /// `property<T>` — `rule:types/property-key`'s property key. Its *values* are the names
    /// of `T`'s public declared properties, and the argument bounds which
    /// receiver the key may be applied to.
    ///
    /// The argument is always a [`Self::Class`] id naming a class, for
    /// [`Self::ClassRef`]'s reason and by the same guard: `crate::lower`'s atom
    /// refuses anything else where it is written
    /// (`E_PROPERTY_KEY_ARGUMENT_NOT_A_CLASS`).
    ///
    /// **Contravariant in that argument** (`rule:types/property-key-variance`), which makes it the
    /// only generic name in the language that is, and the inversion is exactly
    /// [`Self::ClassRef`]'s covariance seen from the other side: a class
    /// reference is produced against its bound, while a key is *consumed* by a
    /// receiver. A subclass only ever adds properties, so `property<Animal>`'s
    /// names are all valid on a `Dog` and `property<Dog>`'s are not all valid
    /// on an `Animal` — the widening therefore runs `property<Animal>` to
    /// `property<Dog>`, and never back.
    PropertyKey(TypeId),
    /// `object`
    Object,
    /// `mixed` — the one unchecked position.
    Mixed,
    /// `void` — return-position only.
    Void,
    /// `never` — return-position only.
    Never,
    /// `true` — `rule:types/grammar`'s atom, and `bool`'s half of `rule:types/literal-types`: the
    /// type inhabited by exactly one value, which is what
    /// [`Self::StringLiteral`] generalises to `string`'s. It is placed the
    /// same way ([`crate::expr::literals::placed_literal`]) — a `true`
    /// expression is an ordinary [`Self::Bool`] unless the position names
    /// this type — widens the same way ([`TypeInterner::literal_base`]), and
    /// erases the same way, to [`Self::Bool`]'s own representation
    /// (§ 5, `nvs_ir::lower::erase_checked_ty`).
    True,
    /// `false` — [`Self::True`]'s other half, in every respect.
    False,
    /// `"a"` — `rule:types/literal-types`: the type inhabited by exactly one string, the
    /// generalisation of [`Self::True`]/[`Self::False`] from `bool`'s two
    /// values to `string`'s.
    ///
    /// Holds the **cooked** value, not the source text: `crate::lower` runs
    /// the literal through [`crate::string_lit::cook_string_literal`], the
    /// same decoder every string literal in a *value* position goes through,
    /// so `"a\n"` and a literal `"a"` followed by a real newline intern to one
    /// type and the language never grows a second escape grammar. Interning is
    /// structural over that value, which is exactly the singleton-ness the
    /// type claims.
    ///
    /// `rule:types/literal-types`: no runtime representation of its own — it erases to
    /// [`Self::String`] at the `nvs-ir` boundary
    /// (`nvs_ir::lower::erase_checked_ty`), and the singleton-ness is enforced
    /// entirely by the checker wherever the static type is known.
    StringLiteral(String),
    /// `1`, `-1` — `rule:types/literal-types`'s `int` counterpart of
    /// [`Self::StringLiteral`], erasing to [`Self::Int`] the same way.
    ///
    /// `i64`, so the value is always one an `int` can hold: a magnitude past
    /// `int`'s range is diagnosed where the atom is lowered rather than
    /// widened to `uint` here, because `rule:types/literal-types` gives the atom one base
    /// type and a second one would make `1`'s meaning depend on its
    /// neighbours. There is deliberately no `float` counterpart (§ 7).
    IntLiteral(i64),
    /// `Mode::Read` — `rule:types/enum-case-type`: a subtype of the enum inhabited by exactly
    /// one of its cases, carrying the enum's `QName`, its backing type, and
    /// the case's own name.
    ///
    /// Deliberately **not** [`Self::IntLiteral`] of the case's backing value,
    /// which is the whole of § 3: folding it that way would let a bare `int`
    /// satisfy an enum-typed parameter, reopening the hole
    /// `rule:types/conversion` closed
    /// by making `int → Mode` a checked conversion. An enum-case type and an
    /// int literal type that happen to share a value are never unified by
    /// canonicalisation, because they are not the same `Ty`.
    ///
    /// The backing type rides along for [`Self::Enum`]'s own reason — it is
    /// part of what the type *is*, and it is what lets this erase to the
    /// enum's existing zero-byte tag at the `nvs-ir` boundary with no
    /// re-resolution. The case *name* rides along rather than its value
    /// because § 6's diagnostic names `Mode::Read`, and the value is one
    /// [`crate::enums::EnumTable::case`] lookup away for anything that needs
    /// it.
    EnumCase(QName, crate::enums::EnumBacking, String),
    /// `iterable`
    Iterable,
    /// `callable`
    Callable,
    /// `callable(User, string): string` — `rule:types/callable-signature`'s
    /// written signature: the parameter types left to right, and the return
    /// type the grammar makes mandatory.
    ///
    /// A variant of its own rather than an option on [`Self::Callable`], for
    /// the reason [`nvs_syntax::ast::TypeAtom::CallableSig`] is an atom of its
    /// own: bare `callable` is the **top** of the callable lattice and goes on
    /// meaning exactly what it meant — a closure whose signature is unknown,
    /// reached by a dynamic call whose arguments `nvs_runtime::closure` checks
    /// one tag at a time — while a written signature is checked where the call
    /// is written and pays nothing at run time. The two never collapse.
    ///
    /// An empty `params` is `callable(): void`, which promises *no*
    /// parameters; that is a different type from bare `callable`, which
    /// promises nothing.
    ///
    /// This is a type in every way the two variants below are not: source text
    /// can spell it, [`TypeInterner::describe`] renders it back as what was
    /// written, and it survives a call site all the way to `nvs-ir`, where it
    /// erases to the object pointer [`Self::Callable`] already erases to.
    CallableSig {
        /// The parameter types, left to right.
        params: Vec<TypeId>,
        /// The return type.
        ret: TypeId,
    },
    /// `callable`, plus the name of the type variable its **result** binds —
    /// `U` in `Core\Arr::map(array<T> $a, callable $fn): array<U>`.
    ///
    /// A **shape whose every field is a callable**, plus the name of the type
    /// variable the shape of those callables' *results* binds — `S` in
    /// `Core\Task::all({...}): S`.
    ///
    /// The one type in this enum no source text can spell.
    /// `rule:concurrency/all-answers-a-typed-shape` is the whole reason it exists —
    /// `Task::all`'s answer keeps each field's own declared return type rather
    /// than collapsing to `array<mixed>`, and no writable type says "the same
    /// field names, one call layer off", because the field names belong to the
    /// call site rather than to the signature.
    ///
    /// **An ordinary type in both halves.** What it accepts is decided by
    /// [`crate::expr::assign`]'s assignability relation — a [`Self::Shape`]
    /// whose every field satisfies [`Self::Callable`] — so an argument that is
    /// not one is refused by the mismatch every other parameter reports. What
    /// it binds is decided by [`crate::generics::bind`], which reads each
    /// field's [`Self::CallableSig`] return type off the argument's own type;
    /// a field typed bare `callable` has none to read and binds `mixed`, which
    /// is that field's honest answer rather than a refusal.
    ///
    /// It enters the interner only from `nvs_stdlib::registry`'s
    /// `CoreTy::ShapeOfCallables` through [`crate::core_lib`], and unlike a
    /// [`Self::TypeVar`] it **survives substitution**: it is the parameter the
    /// argument is checked against, so erasing it would erase the check.
    ShapeOfCallables(String),
    /// A resolved class or interface name, plus the type arguments it was
    /// written with — the type grammar does not distinguish a class from an
    /// interface (`rule:types/grammar`); which one `QName` names is a question for
    /// [`nvs_hir::SymbolTable`], not this representation.
    ///
    /// The argument list is empty for all but two names.
    /// `rule:iteration/concrete-generic-implements`
    /// lets a *compiler-owned* generic interface be written at a concrete
    /// type — `Iterator<int>` — and `nvs_hir::interfaces::RESERVED` is the
    /// closed roster of what may be. Anything else written with arguments is
    /// refused by [`crate::lower`] before it ever interns, so a non-empty
    /// list here always names one of those two interfaces.
    ///
    /// Interning is structural, so `Iterator<int>` and `Iterator<string>` are
    /// two distinct `TypeId`s while `Counter` and `Counter` are one — which
    /// is the whole point of carrying the arguments in the type rather than
    /// beside it. They are erased at the `nvs-ir` boundary
    /// (`nvs_ir::lower::erase_checked_ty` maps every class to one pointer
    /// type), exactly as a [`Self::TypeVar`] is erased at a call site: a type
    /// argument constrains what the checker accepts and never what the
    /// runtime stores.
    Class(QName, Vec<TypeId>),
    /// A resolved enum name (`rule:enums/closed-integer-type`), together with the underlying integer
    /// type its cases are constants of.
    ///
    /// The backing type rides in the type itself rather than in a side table
    /// because it *is* part of what the type is: `rule:enums/one-backing-type` gives every enum
    /// exactly one underlying integer type, and § 6 makes an enum value that
    /// integer's representation with names attached. Carrying it here is what
    /// lets `nvs-ir` lower an enum-typed binding to a machine integer without
    /// re-resolving the declaration (`nvs_ir::lower::erase_checked_ty`). It is
    /// a function of the `QName`, so it never splits one enum into two
    /// interned types.
    Enum(QName, crate::enums::EnumBacking),
    /// `{name: T, name?: T, ...}` — `rule:types/shape-type`, Novis's one
    /// structurally-checked type.
    /// Fields are sorted by name (see [`TypeInterner::shape`]) so two shapes
    /// naming the same fields in a different written order intern to the
    /// same `TypeId`; unlike [`Self::Union`]/[`Self::Intersection`] there is
    /// no flattening to do, since a shape field's type is never itself
    /// required to be a shape.
    ///
    /// [`ShapeField::required`] carries the written `?`, and it is part of what
    /// the type *is*: `{a?: int}` and `{a: int}` intern apart, and so do
    /// `{a?: int}` and `{a: ?int}`, which ask different questions of a value.
    Shape(Vec<ShapeField>),
    /// A `Core` parameter whose keys are fixed — `rule:core-api/shape-rules` R2's trailing options
    /// bag (`{step?: int}`) and `rule:core-api/shape-parameter`'s fixed-key shape parameter, which are
    /// **one** checked type because they differ in call-site rules rather than
    /// in checking (`rule:core-api/one-checked-shape-type`).
    ///
    /// Named for the shape and not for the bag because the bag is the narrower
    /// of the two uses: it is this type with every
    /// [`required`](CoreShapeField::required) false. Both registry spellings
    /// arrive through [`crate::core_lib`], which is the one place either
    /// translation happens — `CoreTy::Options` one field per declared option,
    /// `CoreTy::Shape` its arms **merged in declaration order and deduplicated
    /// by name**, which is `rule:core-api/shape-flattens-at-the-abi`'s ABI.
    ///
    /// The second type in this enum no source text can spell (see
    /// [`Self::TypeVar`] for the first). A *value* of this type is still
    /// written by hand — an `rule:types/object-top` object literal at the call site — but the
    /// type itself is never written, which is why there is no `?` in the
    /// surface type grammar.
    ///
    /// Deliberately not a [`Self::Shape`]. A shape is checked by `rule:types/shape-type`'s
    /// **width** subtyping, which accepts a field the target does not name;
    /// this one must refuse one, because a mistyped key that is silently
    /// ignored is exactly the failure `rule:core-api/shape-rules` R2 exists to prevent.
    /// [`crate::expr`] owns that check.
    ///
    /// Fields keep their **declared order** rather than being sorted the way
    /// [`TypeInterner::shape`] sorts a shape's: that order is the order
    /// `nvs_ir::lower::lower_call_args` flattens them into ABI arguments, so
    /// two members whose keys differ only in order are genuinely two different
    /// types and must not intern to one.
    ///
    /// A key the merged list marks [`CoreShapeField::required`] must be
    /// written, and a key it does not declare at all is refused —
    /// `crate::expr::args::check_options_arg` reports both, one code each.
    ///
    /// That list is the *widest* statement of what the parameter takes, and it
    /// is deliberately not the rule a written literal is held to: `rule:core-api/shape-arms-are-disjoint`
    /// selects one of [`CoreShape::arms`] and holds the literal to that arm's
    /// keys and that arm's types. See [`CoreShape`] for why the type carries
    /// both.
    CoreShape(CoreShape),
    /// `A|B|...` — flattened, deduplicated, and sorted by member `TypeId`.
    /// Always at least two members; a one-member union collapses to that
    /// member directly (see [`TypeInterner::make_union`]).
    Union(Vec<TypeId>),
    /// `A&B&...` — same canonicalization as [`Self::Union`].
    Intersection(Vec<TypeId>),
    /// A *type variable*, named — `T` in `Core\Arr::count(array<T> $a): uint`.
    ///
    /// The one type in this enum no source text can spell. `rule:types/declaration` parks
    /// user-declared generics and `docs/agent/loop-goal.md` keeps type variables
    /// compiler-owned, so a `TypeVar` only ever enters the interner from
    /// `nvs_stdlib::registry`'s `Core` signatures ([`crate::core_lib`]) or
    /// `rule:iteration/two-interfaces`'s two iteration interfaces ([`crate::iter_lib`]) —
    /// [`crate::lower`] has no arm producing one, which is what makes that a
    /// property of the code rather than a convention. `Iterator<int>` written
    /// in source produces [`Self::Class`] with a concrete argument, never
    /// this.
    ///
    /// It never survives a call site. [`crate::signatures::MethodSig`]'s own
    /// docs own the substitution rule: a generic signature is unified against
    /// the actual argument types and rewritten before anything checks an
    /// argument or records a `ResolvedCall`, so every later pass — including
    /// every `nvs-ir` lowering — only ever sees concrete types. A variable
    /// that no argument bound is the one exception, and substitutes to
    /// [`Self::Mixed`]: the honest answer for "this position's type is
    /// unconstrained by the call," and the only one that keeps a later pass
    /// from meeting a variable it has no rule for.
    TypeVar(String),
}

/// `rule:core-api/shape-parameter`'s fixed-key shape parameter, both ways round: § 3's merged field
/// list, which is the ABI, and § 2's arms, which are what a written literal is
/// actually checked against.
///
/// **Both, because neither states the other.** The merged list cannot state
/// *exactly one arm accepts it*: a two-arm shape merges to the union of its
/// arms' keys, so a literal drawing keys from both arms would pass and a key
/// only one arm requires would be required by neither. The arms cannot state
/// the ABI: which slot a key occupies is a property of the merge, and
/// `nvs_ir::lower::lower_call_args` flattens the argument by that order alone.
/// Deriving either from the other at every call site would re-run the merge
/// per call, so both are interned once, here.
///
/// A key more than one arm declares appears once in [`Self::fields`], typed as
/// the union of the arms' declarations for it, and once per declaring arm in
/// [`Self::arms`], typed as that arm declares it. `Db\Settings`'s `driver` is
/// the worked case: one ABI slot typed `Driver`, and per arm the four cases
/// that take a host or the one that does not, which is the whole of what ADR
/// 0135 § 2 means by *discriminated*.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CoreShape {
    /// § 3's merged list: the arms in declaration order, each arm's fields in
    /// declaration order, a name a previous arm already emitted skipped. One
    /// ABI argument per entry, in this order, and
    /// [`required`](CoreShapeField::required) here means *every* arm requires
    /// it.
    pub fields: Vec<CoreShapeField>,
    /// § 2's arms, in declaration order, each arm's own fields in declaration
    /// order and typed as that arm declares them.
    ///
    /// **Never empty, and exactly one arm for a bag or a one-arm shape** —
    /// whose single arm is [`Self::fields`] itself, so the selection rule has
    /// no armless case to special-case and a bag takes the same path a
    /// two-armed shape does.
    pub arms: Vec<Vec<CoreShapeField>>,
}

/// One `name: T` field of a [`Ty::Shape`] — `rule:types/shape-type`.
///
/// A named struct rather than the pair this replaced, for the reason
/// [`CoreShapeField`] gives for being one: the added member is a bare `bool`,
/// and `("host", id, false)` at a construction site says nothing about which
/// way round the flag runs. The polarity is deliberately
/// [`CoreShapeField::required`]'s rather than a second, opposite spelling —
/// both answer "must a value carry this key", and one of them reading the
/// other way is a bug waiting at every site that moves a field between them.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ShapeField {
    /// The key, as it is written on the left of the `:`.
    pub name: String,
    /// Its declared type. A field's type is never itself a shape's business to
    /// restrict — ordinary assignability decides it (`rule:types/shape-type`).
    pub ty: TypeId,
    /// Whether a value must carry the key at all. A written `?` clears it, and
    /// **that is not nullability**: `{a?: int}` accepts a value with no `a`,
    /// `{a: ?int}` demands an `a` that may hold `null`, and the two intern
    /// apart because they accept different values.
    pub required: bool,
}

impl ShapeField {
    /// The field every shape but a written one is built from: nothing seeded by
    /// [`crate::error_lib`], inferred from an object literal, or substituted
    /// through a type variable has a `?` to carry, so those sites say which
    /// kind they mean once rather than repeating `required: true`.
    #[must_use]
    pub fn required(name: String, ty: TypeId) -> Self {
        Self {
            name,
            ty,
            required: true,
        }
    }
}

/// One key of a [`Ty::CoreShape`] — the checked half of
/// `nvs_stdlib::registry`'s `CoreOption` and `CoreField`, which are the two
/// registry spellings `rule:core-api/one-checked-shape-type` collapses into this one.
///
/// A named struct rather than the pair this replaced, because the third member
/// is a bare `bool`: `("host", id, false)` at a construction site says nothing
/// about which way round the flag runs, and there are enough sites to make that
/// a real reading cost.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CoreShapeField {
    /// The key a call site writes on the left of the `:`, `camelCase` per ADR
    /// 0029 — an option's own name, or a field's.
    pub name: String,
    /// Its declared type, already lowered. Never itself a [`Ty::CoreShape`],
    /// and nullable exactly where the row's default is the never-written
    /// marker: `nvs_stdlib::registry`'s `a_shape_is_only_ever_a_whole_parameter`
    /// and `a_nullable_shape_field_omits_as_the_never_written_marker` hold both
    /// over the rows this is built from, so nothing here re-checks them.
    pub ty: TypeId,
    /// Whether a call site must write this key. **False for every field of an
    /// options bag** — `rule:core-api/shape-rules` R2 makes the whole bag omittable, so an option
    /// that had to be written could not exist — and for a shape field it is
    /// `rule:core-api/shape-parameter`'s `CoreField::default` read the other way round: a field
    /// with no default is required.
    pub required: bool,
    /// `rule:core-api/shape-flattens-at-the-abi`'s qualifier classification, which lands on the **field**
    /// and never on the parameter: `Db\Settings`'s `host` is a [`Qual::Sink`]
    /// because `rule:core-classes/db-capabilities` makes an address one, while the parameter holding
    /// it classifies nothing at all.
    ///
    /// `None` where the registry's own type carries no classification — every
    /// field whose type is not one of the qualifiable atoms, and every option
    /// of a bag [`crate::error_lib`] seeds rather than a registry row.
    ///
    /// It is here for the *diagnostic* and not for the check: the declared
    /// [`ty`](Self::ty) already refuses a qualified value at a
    /// [`Qual::Sink`], and what the mark adds is that
    /// `crate::expr::args::check_shape_field` can say what the way through is
    /// rather than leaving a reader with a bare `expected string, found
    /// tainted string`.
    pub qual: Option<Qual>,
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
    /// remains — `rule:types/grammar`'s canonicalization, generalised to also cover
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
        // `never` is the bottom type — no value ever has it — so a union that
        // offers it *alongside* something else offers exactly that something
        // else. Absorbing it here rather than at each consumer is what lets
        // `$v ?? throw new LogicError("…")` and `$v ?? exit(1)` satisfy a
        // plain `string` position: the branch contributing the `never` cannot
        // reach the join, so the join's type was never a union at all. Kept
        // when it is the only member, which is the honest type of an
        // expression that does not complete.
        if ids.len() > 1 {
            ids.retain(|id| !matches!(self.get(*id), Ty::Never));
        }
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
            Ty::ClassRef(inner) => format!("class<{}>", self.describe(*inner)),
            Ty::PropertyKey(inner) => format!("property<{}>", self.describe(*inner)),
            Ty::Object => "object".to_owned(),
            Ty::Mixed => "mixed".to_owned(),
            Ty::Void => "void".to_owned(),
            Ty::Never => "never".to_owned(),
            Ty::True => "true".to_owned(),
            Ty::False => "false".to_owned(),
            Ty::StringLiteral(value) => quote_string_literal(value),
            Ty::IntLiteral(value) => value.to_string(),
            Ty::EnumCase(q, _, case) => format!("{q}::{case}"),
            Ty::Iterable => "iterable".to_owned(),
            Ty::Callable => "callable".to_owned(),
            // Rendered as it is written, so a diagnostic about a signature
            // quotes text the program's author can find — which is why the
            // parameters are joined in their own order and the return type
            // keeps its colon.
            Ty::CallableSig { params, ret } => {
                let inner = params
                    .iter()
                    .map(|param| self.describe(*param))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("callable({inner}): {}", self.describe(*ret))
            }
            // The spelling is what a call site writes rather than a type name,
            // because there is no type name — see [`Ty::ShapeOfCallables`],
            // which survives substitution precisely so that a mismatch here
            // gets rendered.
            Ty::ShapeOfCallables(_) => "{name: callable(): T, ...}".to_owned(),
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
                    .map(|field| {
                        let opt = if field.required { "" } else { "?" };
                        format!("{}{opt}: {}", field.name, self.describe(field.ty))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{inner}}}")
            }
            Ty::CoreShape(shape) => {
                let inner = shape
                    .fields
                    .iter()
                    .map(|field| {
                        let opt = if field.required { "" } else { "?" };
                        format!("{}{opt}: {}", field.name, self.describe(field.ty))
                    })
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

    /// The interned `decimal` singleton — `rule:types/decimal`.
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

    /// Interns `rule:types/literal-types`'s string literal type — see
    /// [`Ty::StringLiteral`], which owns why `value` is the cooked string
    /// rather than the source text.
    #[must_use]
    pub fn string_literal(&mut self, value: impl Into<String>) -> TypeId {
        self.intern(Ty::StringLiteral(value.into()))
    }

    /// Interns `rule:types/literal-types`'s int literal type.
    #[must_use]
    pub fn int_literal(&mut self, value: i64) -> TypeId {
        self.intern(Ty::IntLiteral(value))
    }

    /// Interns `rule:types/enum-case-type`'s enum-case type — see [`Ty::EnumCase`] for why
    /// this is a type of its own rather than [`Self::int_literal`] of the
    /// case's backing value.
    #[must_use]
    pub fn enum_case(
        &mut self,
        qname: QName,
        backing: crate::enums::EnumBacking,
        case: impl Into<String>,
    ) -> TypeId {
        self.intern(Ty::EnumCase(qname, backing, case.into()))
    }

    /// The type `rule:types/literal-types`'s first four rows widen `id` to: a literal type's
    /// base type, an enum-case type's enum, and anything else unchanged.
    ///
    /// The checker-side counterpart of `nvs_ir::lower::erase_checked_ty`'s
    /// erasure — § 5 gives a literal type no representation of its own, so
    /// every question about what a value of one can *do* is a question about
    /// its base. A union is widened member-wise, which is what makes
    /// `"a"|"b"` widen to `string` rather than to itself.
    pub fn literal_base(&mut self, id: TypeId) -> TypeId {
        match self.get(id).clone() {
            Ty::StringLiteral(_) => self.string(),
            Ty::IntLiteral(_) => self.int(),
            // `rule:types/grammar`'s two `bool` singletons are literal types under
            // `rule:types/literal-types`'s own reading of them (see [`Ty::True`]), so they
            // widen here rather than anywhere of their own — which is what
            // makes `bool $b = $x as true;` an ordinary assignment.
            Ty::True | Ty::False => self.bool_ty(),
            Ty::EnumCase(q, backing, _) => self.enum_(q, backing),
            Ty::Union(members) => {
                let widened: Vec<TypeId> = members.iter().map(|m| self.literal_base(*m)).collect();
                self.make_union(widened)
            }
            _ => id,
        }
    }

    /// The interned `callable` singleton.
    #[must_use]
    pub fn callable(&mut self) -> TypeId {
        self.intern(Ty::Callable)
    }

    /// The interned `callable` carrying the signature `params` → `ret` — see
    /// [`Ty::CallableSig`], which owns why it is a separate type from the bare
    /// one above rather than a fuller description of it.
    pub fn callable_sig(&mut self, params: Vec<TypeId>, ret: TypeId) -> TypeId {
        self.intern(Ty::CallableSig { params, ret })
    }

    /// The interned shape-of-callables parameter that binds `name` from the
    /// shape of its fields' results — see [`Ty::ShapeOfCallables`], which owns
    /// why nothing outside a `Core` signature ever calls this.
    pub fn shape_of_callables(&mut self, name: impl Into<String>) -> TypeId {
        self.intern(Ty::ShapeOfCallables(name.into()))
    }

    /// Interns `array<elem>`.
    #[must_use]
    pub fn array(&mut self, elem: TypeId) -> TypeId {
        self.intern(Ty::Array(elem))
    }

    /// Interns `class<inner>`, where `inner` is a class or interface id —
    /// see [`Ty::ClassRef`] for who guarantees that.
    #[must_use]
    pub fn class_ref(&mut self, inner: TypeId) -> TypeId {
        self.intern(Ty::ClassRef(inner))
    }

    /// Interns `property<inner>`, where `inner` is a class id — see
    /// [`Ty::PropertyKey`] for who guarantees that.
    #[must_use]
    pub fn property_key(&mut self, inner: TypeId) -> TypeId {
        self.intern(Ty::PropertyKey(inner))
    }

    /// Interns a resolved class/interface name with no type arguments —
    /// every name but `rule:iteration/concrete-generic-implements`'s two generic interfaces.
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

    /// Interns `{name: T, ...}` — `rule:types/shape-type`. Sorts `fields` by name first,
    /// so `{x: int, y: string}` and `{y: string, x: int}` intern to the same
    /// `TypeId` regardless of how each was written (same canonicalization
    /// idea as [`Self::make_union`], applied to field order instead of
    /// member order).
    #[must_use]
    pub fn shape(&mut self, mut fields: Vec<ShapeField>) -> TypeId {
        fields.sort_by(|a, b| a.name.cmp(&b.name));
        self.intern(Ty::Shape(fields))
    }

    /// Interns `rule:core-api/shape-rules` R2's options bag — every option optional, which is what
    /// makes a bag the narrow case of [`Ty::CoreShape`] rather than a second
    /// type (`rule:core-api/one-checked-shape-type`). That variant owns why the order given is kept
    /// rather than sorted the way [`Self::shape`] sorts.
    ///
    /// Two callers, and both seed a signature the program did not write:
    /// [`crate::core_lib`] for a `Core` member's bag, and [`crate::error_lib`]
    /// for the exception constructor's `{previous}`. Each option carries its
    /// own [`CoreShapeField::qual`] — a bag option is classified exactly as a
    /// shape field is, `Core\Cli\Progress::advance`'s `label` being a
    /// [`Qual::Launder`] — and `None` where the caller has no registry row
    /// behind it.
    #[must_use]
    pub fn options(&mut self, options: Vec<(String, TypeId, Option<Qual>)>) -> TypeId {
        let fields: Vec<CoreShapeField> = options
            .into_iter()
            .map(|(name, ty, qual)| CoreShapeField {
                name,
                ty,
                required: false,
                qual,
            })
            .collect();
        // A bag is `rule:core-api/shape-arms-are-disjoint`'s one-arm case, and carries that arm rather
        // than an empty list: the selection rule then reaches a bag unchanged,
        // which is [`CoreShape::arms`]' own reason for never being empty.
        self.intern(Ty::CoreShape(CoreShape {
            arms: vec![fields.clone()],
            fields,
        }))
    }

    /// Interns `rule:core-api/shape-parameter`'s fixed-key shape parameter, `shape` already carrying
    /// § 3's merged list and § 2's arms — [`crate::core_lib`] is the only
    /// caller that builds one from a registry row, and does both.
    ///
    /// Separate from [`Self::options`] only in what it is handed: a bag has no
    /// required key to state and no arm to choose between, so making it pass
    /// one `false` per option and a copy of its own field list would be two
    /// lies every call site had to write.
    #[must_use]
    pub fn core_shape(&mut self, shape: CoreShape) -> TypeId {
        self.intern(Ty::CoreShape(shape))
    }

    /// Whether `id` is `null` itself, or a union with `null` as one of its
    /// members — i.e. whether it was written with a leading `?` (or expands
    /// to one through a `type` alias). `rule:classes/no-undefined-value`: this is the one thing
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

    /// `id` with `null` removed — `?T` becomes `T`, `A|B|null` becomes `A|B`,
    /// and anything not [`Self::is_nullable`] comes back unchanged.
    ///
    /// What `$a ?? $b` and a `!== null` narrowing both need: the type the
    /// value actually holds once the `null` arm is ruled out. `null` alone has
    /// nothing left to be, so it stays `null` rather than becoming `never` —
    /// the arm is unreachable either way, and `never` would make every caller
    /// handle a type that cannot arrive.
    #[must_use]
    pub fn without_null(&mut self, id: TypeId) -> TypeId {
        let Ty::Union(members) = self.get(id) else {
            return id;
        };
        let kept: Vec<TypeId> = members
            .iter()
            .copied()
            .filter(|m| !matches!(self.get(*m), Ty::Null))
            .collect();
        if kept.is_empty() {
            return id;
        }
        self.make_union(kept)
    }
}

/// Renders a [`Ty::StringLiteral`]'s cooked value back as the double-quoted
/// literal a program would write it as — what a diagnostic naming the accepted
/// set has to print (`rule:types/literal-types`).
///
/// Only the four characters that would end or re-open the literal are escaped.
/// This is a *rendering* for a message, not a round-trip through
/// [`crate::string_lit`]: a value carrying some other control character prints
/// it as-is, exactly as every other quoted fragment in a diagnostic does.
fn quote_string_literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
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
        let a = i.shape(vec![
            ShapeField::required("x".to_owned(), int),
            ShapeField::required("y".to_owned(), string),
        ]);
        let b = i.shape(vec![
            ShapeField::required("y".to_owned(), string),
            ShapeField::required("x".to_owned(), int),
        ]);
        assert_eq!(a, b);
    }

    #[test]
    fn describe_renders_a_shape_with_its_fields() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let s = i.shape(vec![ShapeField::required("x".to_owned(), int)]);
        assert_eq!(i.describe(s), "{x: int}");
    }

    /// `rule:types/shape-type`: an optional key and a nullable one are
    /// different types, so they intern apart and describe apart — the `?`
    /// before the `:` is the key's, the one after it the type's.
    #[test]
    fn an_optional_field_is_a_different_type_from_a_nullable_one() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let null = i.null();
        let nullable = i.make_union([int, null]);
        let optional = i.shape(vec![ShapeField {
            name: "a".to_owned(),
            ty: int,
            required: false,
        }]);
        let required = i.shape(vec![ShapeField::required("a".to_owned(), int)]);
        let of_nullable = i.shape(vec![ShapeField::required("a".to_owned(), nullable)]);
        assert_ne!(optional, required);
        assert_ne!(optional, of_nullable);
        assert_eq!(i.describe(optional), "{a?: int}");
        assert_eq!(i.describe(required), "{a: int}");
    }

    /// `rule:types/callable-signature`: a signature renders as the source
    /// spelling it came from, and the empty parameter list is a signature
    /// promising none rather than the bare type promising nothing — so the two
    /// intern apart and describe apart.
    #[test]
    fn describe_renders_a_callable_signature_as_it_is_written() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let string = i.string();
        let sig = i.callable_sig(vec![int, string], string);
        assert_eq!(i.describe(sig), "callable(int, string): string");
        let void = i.void();
        let none = i.callable_sig(Vec::new(), void);
        assert_eq!(i.describe(none), "callable(): void");
        assert_ne!(none, i.callable());
    }

    #[test]
    fn describe_renders_a_union_with_a_pipe() {
        let mut i = TypeInterner::new();
        let int = i.int();
        let float = i.float();
        let u = i.make_union([int, float]);
        assert_eq!(i.describe(u), "int|float");
    }

    /// `rule:types/literal-types`: a diagnostic names the accepted set by printing the type
    /// itself, so each atom has to render as the source spelling it came from.
    #[test]
    fn describe_renders_adr_0047s_three_atoms() {
        let mut i = TypeInterner::new();
        let s = i.string_literal("a");
        assert_eq!(i.describe(s), "\"a\"");
        let quoted = i.string_literal("say \"hi\"\n");
        assert_eq!(i.describe(quoted), "\"say \\\"hi\\\"\\n\"");
        let n = i.int_literal(-1);
        assert_eq!(i.describe(n), "-1");
        let case = i.enum_case(
            QName::parse("App\\Mode"),
            crate::enums::EnumBacking::Int,
            "Read",
        );
        assert_eq!(i.describe(case), "App\\Mode::Read");
    }

    /// § 3's whole point, at the representation: a case and an int literal of
    /// its backing value are two `TypeId`s, so nothing can unify them by
    /// accident.
    #[test]
    fn an_enum_case_type_never_interns_as_its_backing_value() {
        let mut i = TypeInterner::new();
        let case = i.enum_case(QName::parse("Mode"), crate::enums::EnumBacking::Int, "Read");
        let zero = i.int_literal(0);
        assert_ne!(case, zero);
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
