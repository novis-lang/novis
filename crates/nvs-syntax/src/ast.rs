//! The AST: what the parser produces from a token stream.
//!
//! # What is here so far
//!
//! Types ([`Type`]/[`TypeKind`]/[`TypeAtom`], `rule:types/grammar`) and expressions
//! ([`Expr`]/[`ExprKind`], every construct M1's plan names — `match`, closures
//! and arrow functions, `spawn script`, `require`, named arguments,
//! spread, nullsafe, first-class callable syntax, the `as` conversion
//! operator). [`Block`]/[`Stmt`]/[`StmtKind`] now also cover every
//! control-flow statement (`if`, `while`, `do`/`while`, `for`, `foreach`'s
//! mandatory typed bindings, `switch`, `break`/`continue`, `try`/`catch`/
//! `finally`), `echo`, `unset`, `rule:types/grammar`.1's typed local declaration,
//! § 3.3's destructuring statement, and the statement-shaped rejects
//! (`global`, `goto`, function-scope `static`). Classes, interfaces and
//! enums ([`ClassDecl`]/[`InterfaceDecl`]/[`EnumDecl`]), their members
//! ([`ClassMember`]: properties with PHP 8.4's hooks, consts, methods),
//! attributes ([`AttributeGroup`]), and the file-scope declarations that sit
//! alongside them ([`NamespaceDecl`]/[`UseDecl`]/[`TypeAliasDecl`]) round out
//! M1's last chunk. There is no `trait` declaration and no class-body
//! `use Trait, ...;` — `rule:classes/no-traits` replaces both with an `interface` method that
//! carries a body (§§ 2-3) and `by $field` delegation on an
//! [`ImplementsClause`] (§ 4); `trait`/class-body `use`/`insteadof` are all
//! parse-time-rejected instead (`E_TRAIT_NOT_SUPPORTED`), with no AST node
//! left to carry them.
//!
//! # Conventions
//!
//! An AST node stores [`Span`]s, never owned text — exactly the discipline
//! [`crate::token`] already documents for [`Token`](crate::Token): a literal's
//! value is "cooked" (an integer parsed, a string unescaped) only once a later
//! stage actually needs it, so the parser itself never allocates a `String` or
//! decides what a number means.

use nvs_diagnostics::Span;

// ============================================================================
// Names
// ============================================================================

/// A possibly-namespace-qualified name: `Foo`, `Core\Bytes`, `App\Models\User`.
///
/// The lexer emits no combined token for this — it is an [`Ident`](crate::TokenKind::Ident)
/// optionally interspersed with [`Backslash`](crate::TokenKind::Backslash) — so
/// the parser folds the whole run into one span. A *leading* separator is not
/// part of the spelling: it is refused where the name is parsed
/// (`rule:statements/a-leading-separator-does-not-parse`), so
/// a `Name`'s span never opens on one. Splitting it into segments and resolving
/// it to a declaration is name resolution's job (M2), not this stage's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Name {
    /// The full qualified spelling, backslashes included.
    pub span: Span,
}

// ============================================================================
// Types (`rule:types/grammar`)
// ============================================================================

/// A type expression: `int`, `array<uint>`, `int|string`, `?User`, `A&B`.
#[derive(Clone, Debug, PartialEq)]
pub struct Type {
    /// What kind of type expression this is.
    pub kind: TypeKind,
    /// The full span of the type expression.
    pub span: Span,
}

/// The shape of a [`Type`].
///
/// Mirrors `rule:types/grammar`'s grammar directly: `type := union`,
/// `union := intersection ('|' intersection)*`,
/// `intersection := atom ('&' atom)* | '(' union ')'`. [`Self::Paren`] preserves
/// an explicit `(...)` grouping used to nest a union inside an intersection for
/// DNF (`(A&B)|C`); it carries no meaning beyond what it wraps.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TypeKind {
    /// `?T`, sugar for `T|null`.
    Nullable(Box<Type>),
    /// `A|B|C`. Always at least two members — a lone operand is never wrapped.
    Union(Vec<Type>),
    /// `A&B&C`. Always at least two members — a lone operand is never wrapped.
    Intersection(Vec<Type>),
    /// An explicit `(...)` grouping.
    Paren(Box<Type>),
    /// A single type atom.
    Atom(TypeAtom),
}

/// One type atom — the leaves `union`/`intersection` combine.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TypeAtom {
    /// `null`
    Null,
    /// `bool`
    Bool,
    /// `int`
    Int,
    /// `uint` — new, unsigned, `rule:types/arithmetic`.
    Uint,
    /// `float`
    Float,
    /// `decimal` — `rule:types/decimal`: a scalar, not a class, and never a spelling
    /// of `float`. It is its own atom for the same reason `uint` is: § 3 makes
    /// `decimal ⊕ float` a compile error, which is only expressible if the two
    /// never collapse to one type.
    Decimal,
    /// `string`
    String,
    /// `bytes` — `rule:types/bytes`.
    Bytes,
    /// `tainted string` — `rule:security/tainted-qualifier`. A compile-time qualifier on `String`,
    /// erased before codegen; kept as its own atom (rather than a generic
    /// wrapper) because the grammar restricts `tainted` to exactly `string`
    /// and `bytes`, and the parser enforces that restriction before this
    /// variant is ever produced.
    TaintedString,
    /// `tainted bytes` — `rule:security/tainted-qualifier`, the `Bytes` counterpart of
    /// [`Self::TaintedString`].
    TaintedBytes,
    /// `secret string` — `rule:security/secret-qualifier`. A second compile-time qualifier,
    /// independent of `tainted`: same "own atom, not a generic wrapper"
    /// shape as [`Self::TaintedString`], for the same reason (the grammar
    /// restricts `secret` to exactly `string`/`bytes`).
    SecretString,
    /// `secret bytes` — `rule:security/secret-qualifier`, the `Bytes` counterpart of
    /// [`Self::SecretString`].
    SecretBytes,
    /// `secret tainted string` — `rule:security/secret-qualifier`: both qualifiers composed.
    /// `secret` must be spelled first; `tainted secret string` is a
    /// diagnostic, not a second valid spelling of this atom.
    SecretTaintedString,
    /// `secret tainted bytes` — `rule:security/secret-qualifier`, the `Bytes` counterpart of
    /// [`Self::SecretTaintedString`].
    SecretTaintedBytes,
    /// `array`, or `array<T>` when a type argument is given.
    Array(Option<Box<Type>>),
    /// `class<T>` — `rule:types/class-reference`'s class reference, whose value is the
    /// run-time class descriptor of a class that is a `T`.
    ///
    /// The argument is held as a whole [`Type`] rather than as a [`Name`],
    /// even though `rule:types/grammar`'s production admits only a `Name` there: an
    /// argument that is not a class or interface name is refused by the
    /// checker, where the name has been resolved and the refusal can say what
    /// it resolved *to*. The parser refusing it would have to report on the
    /// spelling alone, and `class<int>` and `class<Undeclared>` are two
    /// different mistakes.
    ///
    /// There is no argument-less form: `class` alone is the declaration
    /// keyword, and a reference to "some class" with no bound is what
    /// [`Self::Object`] already is.
    ClassRef(Box<Type>),
    /// `property<T>` — `rule:types/property-key`'s property key, whose values are the names
    /// of `T`'s public declared properties.
    ///
    /// The argument is held as a whole [`Type`] for [`Self::ClassRef`]'s
    /// reason, and the two atoms differ in one way the parser can see:
    /// `property` is not a keyword. It is recognised by spelling, and only in
    /// front of a `<` in type position, so `$property`, a member named
    /// `property` and a class named `Property` are all untouched — which is
    /// what the ADR promises when it makes this a keyword "only there".
    PropertyKey(Box<Type>),
    /// `object`
    Object,
    /// `{name: T, ...}` — `rule:types/shape-type`: an inline structural shape type,
    /// checked by width subtyping rather than nominal `implements` — Novis's
    /// one deliberate exception to otherwise fully nominal typing. May be
    /// empty (`{}`), which carries the same "no field promised" meaning as
    /// plain [`Self::Object`]; unlike [`ExprKind::ObjectLiteral`], there is
    /// no expression/block ambiguity in type position forcing a non-empty
    /// rule here.
    Shape(Vec<ShapeField>),
    /// `mixed` — the one unchecked position.
    Mixed,
    /// `void` — return-position only.
    Void,
    /// `never` — return-position only.
    Never,
    /// `true`, the type inhabited by exactly the literal `true`.
    True,
    /// `false`, the type inhabited by exactly the literal `false`.
    False,
    /// `"a"` — the type inhabited by exactly that one string, `rule:types/literal-types`:
    /// the generalisation of [`Self::True`]/[`Self::False`] from `bool`'s two
    /// values to `string`'s. The span covers the whole literal, quotes
    /// included, exactly as [`ExprKind::Str`]'s does, so one decoder serves
    /// both positions; an interpolated `"$x"` is refused in the parser, since
    /// a type has nothing to interpolate from.
    StringLiteral(Span),
    /// `1`, `-1` — the type inhabited by exactly that one integer,
    /// `rule:types/literal-types`. The span covers a leading `-` when one was written.
    /// There is deliberately no `float` counterpart (§ 7).
    IntLiteral(Span),
    /// `Foo::BAR` in type position — `rule:types/constant-in-type-position` and `rule:types/enum-case-type`. The [`Name`] is the
    /// class or enum, the [`Span`] the member identifier after `::`.
    ///
    /// One atom, two meanings, and the parser cannot tell them apart: a
    /// *class constant* folds to its own literal type (§ 2) while an *enum
    /// case* stays a narrowed subtype of its enum (§ 3), which needs the name
    /// resolved. That is the same division of labour [`Self::Name`] already
    /// has, for the same reason.
    Member(Name, Span),
    /// `iterable`
    Iterable,
    /// `callable`
    Callable,
    /// `callable(T, U): R` — `rule:types/callable-signature`'s written
    /// signature.
    ///
    /// Its own atom rather than an option on [`Self::Callable`], because bare
    /// `callable` is the top of the callable lattice and keeps meaning exactly
    /// what it meant: the two never collapse, and a reader that understands
    /// only the top still answers for every value written against it.
    ///
    /// The return type is not optional here, since the grammar makes it
    /// mandatory and a signature written without one is refused where it is
    /// written. Parameters carry no names: a name in the type would imply
    /// calling through the value by name, which nothing supports.
    CallableSig {
        /// The parameter types, left to right. Empty for `callable(): void`,
        /// which is a signature promising no parameters rather than a bare
        /// `callable` promising nothing.
        params: Vec<Type>,
        /// The mandatory return type.
        ret: Box<Type>,
    },
    /// `self`
    SelfTy,
    /// `static`
    StaticTy,
    /// `parent`
    Parent,
    /// A class, interface, enum or `type`-alias name — the checker (not the
    /// parser) decides which kind of atom it resolves to — together with any
    /// `<...>` type-argument list written after it.
    ///
    /// The argument list is almost always empty: `rule:types/declaration` parks
    /// user-declared type parameters, and `rule:iteration/concrete-generic-implements` opens one door for a
    /// *compiler-owned* generic interface (`Iterator<int>`). The parser
    /// accepts the syntax on any name and records what it saw; refusing it on
    /// a name that is not generic is the checker's call, since only the
    /// checker knows what the name resolves to.
    Name(Name, Vec<Type>),
}

/// One `name: T` or `name?: T` field of a [`TypeAtom::Shape`] —
/// `rule:types/shape-type`.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeField {
    /// The field's name.
    pub name: Span,
    /// The field's declared type.
    pub ty: Type,
    /// Whether a value must carry this key at all. A written `?` after the
    /// name clears it, and that is a different question from the type being
    /// nullable: `{a?: int}` admits a value with no `a`, while `{a: ?int}`
    /// demands one holding `null`. The polarity is `nvs_types::ty::ShapeField`'s
    /// so nothing flips it on the way down.
    pub required: bool,
    /// The whole field, name and all.
    pub span: Span,
}

// ============================================================================
// Shared expression pieces
// ============================================================================

/// A prefix unary operator, `⊕expr`.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[expect(
    missing_docs,
    reason = "each variant is exactly its operator spelling; see `ExprKind::Unary`'s use below"
)]
pub enum UnaryOp {
    Neg,
    Plus,
    Not,
    BitNot,
    /// `@expr` — error suppression, and the one variant here **nothing
    /// constructs**: the parser refuses `@` where it is written (`E0236`,
    /// `rule:errors/escalation-ladder` leaving it nothing to suppress) and yields [`ExprKind::Error`]
    /// instead. It is kept so the operator has a name to be refused under
    /// rather than becoming an unrecognized token.
    Suppress,
}

/// `++`/`--`, whichever side of the operand it appears on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IncDecOp {
    /// `++`
    Inc,
    /// `--`
    Dec,
}

/// A binary operator, `lhs ⊕ rhs`.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[expect(
    missing_docs,
    reason = "each variant is exactly its operator spelling; precedence lives in the parser, not here"
)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    /// `.`
    Concat,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    /// `&&`
    And,
    /// `||`
    Or,
    // `==` and `!=` are the whole of equality: `rule:expressions/one-equality-operator` makes `===`/`!==`
    // a rejected spelling the lexer names, so there is no second pair here.
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    /// `<=>`
    Cmp,
    /// `??`
    Coalesce,
}

impl BinaryOp {
    /// Whether this operator may leave its right operand unevaluated.
    ///
    /// `&&` and `||` stop at a left operand that already decides the answer,
    /// and `??` stops at one that is not null. Every other operator evaluates
    /// both — which is why `nvs_ir::lower` gives exactly these three a branch
    /// and a phi rather than one instruction. A checking pass that has to know
    /// which operand a path is proven to have run asks here, rather than
    /// carrying a second copy of the set that would drift the first time an
    /// operator is added.
    #[must_use]
    pub fn short_circuits(self) -> bool {
        matches!(self, Self::And | Self::Or | Self::Coalesce)
    }
}

/// An assignment operator, `target ⊕= value` (or plain `=`).
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[expect(missing_docs, reason = "each variant is exactly its operator spelling")]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    PowAssign,
    ConcatAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
    CoalesceAssign,
}

impl AssignOp {
    /// The binary operator a compound assignment applies, or `None` for the
    /// plain `=`.
    ///
    /// `$x ⊕= e` means `$x = $x ⊕ e` for every variant here, so this is the
    /// one place that pairing is written down: `nvs_types::expr::assign::check_assign`
    /// types a compound assignment through it, and `nvs_ir::lower` desugars
    /// through the same answer, rather than each carrying its own copy of a
    /// fourteen-row table that would drift apart the first time an operator
    /// is added.
    #[must_use]
    pub fn binary_op(self) -> Option<BinaryOp> {
        Some(match self {
            Self::Assign => return None,
            Self::AddAssign => BinaryOp::Add,
            Self::SubAssign => BinaryOp::Sub,
            Self::MulAssign => BinaryOp::Mul,
            Self::DivAssign => BinaryOp::Div,
            Self::ModAssign => BinaryOp::Mod,
            Self::PowAssign => BinaryOp::Pow,
            Self::ConcatAssign => BinaryOp::Concat,
            Self::BitAndAssign => BinaryOp::BitAnd,
            Self::BitOrAssign => BinaryOp::BitOr,
            Self::BitXorAssign => BinaryOp::BitXor,
            Self::ShlAssign => BinaryOp::Shl,
            Self::ShrAssign => BinaryOp::Shr,
            Self::CoalesceAssign => BinaryOp::Coalesce,
        })
    }
}

/// The name on the right of `->`, `?->` or `::` — almost always a plain
/// identifier, but PHP also allows a dynamic member name.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum MemberName {
    /// `->prop`, `->method()`, `::CONST`, `::method()` — the ordinary case.
    Ident(Span),
    /// A name the parser invented because one was required and none was
    /// written — `$u->` with the caret after the arrow. The span is the
    /// insertion point the name would have occupied, and the *variant*, never
    /// that span's width, is what says the name is missing
    /// (`rule:ide/recovery-is-explicit`): a consumer that has to tell an
    /// identifier the user wrote from one the parser invented — member
    /// completion above all — reads this rather than inferring it.
    Missing(Span),
    /// `->$name` — the member name is the value of a variable.
    Variable(Box<Expr>),
    /// `->{expr}` — the member name is an arbitrary computed expression.
    Expr(Box<Expr>),
}

/// One call argument: `value`, `...value` (spread) or `name: value` (named).
#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    /// `name:` for a named argument.
    pub name: Option<Span>,
    /// Whether this argument is `...value`.
    pub spread: bool,
    /// Whether this argument is written `inout value` — `rule:statements/inout-is-written-at-the-call`'s
    /// call-site marker. Whether it is *correct* here needs the callee's
    /// signature, so both mistakes are `nvs_types`' (E0713/E0714).
    pub inout: bool,
    /// The argument's value.
    pub value: Expr,
    /// The whole argument, name and all.
    pub span: Span,
}

/// The argument list of a call, or the first-class callable marker.
#[derive(Clone, Debug, PartialEq)]
pub enum CallArgs {
    /// An ordinary, possibly empty, argument list.
    List(Vec<Arg>),
    /// `(...)` — first-class callable syntax: the call site names a callable
    /// value rather than invoking one.
    FirstClassCallable,
}

/// One element of an array literal: `value`, `key => value`, `...value`
/// (spread) or `&value` (by-reference).
#[derive(Clone, Debug, PartialEq)]
pub struct ArrayItem {
    /// `key =>`, if present.
    pub key: Option<Expr>,
    /// The element's value.
    pub value: Expr,
    /// Whether this element is `...value`.
    pub spread: bool,
    /// Whether this element is `&value`.
    pub by_ref: bool,
    /// The whole element, key and all.
    pub span: Span,
}

/// One `name: value` field of an [`ExprKind::ObjectLiteral`] — `rule:types/object-literal`.
/// Unlike [`ArrayItem`], there is no shorthand, no spread and no computed
/// key: every field name is a static identifier, full stop.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectLiteralField {
    /// The field's name.
    pub name: Span,
    /// The field's value.
    pub value: Expr,
    /// The whole field, name and all.
    pub span: Span,
}

/// One piece of an interpolated string: literal text, or an interpolated
/// expression (`$name`, `$obj->prop`, `$arr[key]`, or a full `{$…}` expression).
#[derive(Clone, Debug, PartialEq)]
pub enum StringPart {
    /// A run of literal text, escapes included but not yet unescaped.
    Text(Span),
    /// One interpolation site, already reduced to an ordinary expression.
    Expr(Expr),
}

/// `#[Attr, Attr2(args)]` — one bracketed group, possibly naming several
/// attributes. Every declaration site (a class, a property, a method, a
/// parameter, ...) may carry any number of these groups, in source order.
#[derive(Clone, Debug, PartialEq)]
pub struct AttributeGroup {
    /// The attributes named in this one `#[...]` group.
    pub attributes: Vec<Attribute>,
    /// The whole group, `#[` through the matching `]`.
    pub span: Span,
}

/// One attribute inside an [`AttributeGroup`] — `rule:attributes/attach-sites-and-forms`'s two forms,
/// `Name(field: value, ...)` and a bare `{field: value, ...}`.
///
/// Both attach the *same* thing: `rule:types/object-literal`'s anonymous object literal. The named form is sugar for a name
/// immediately followed by that literal, so the payload is
/// [`ObjectLiteralField`]s in either case rather than a [`CallArgs`] list —
/// an attribute payload has no positional argument, no shorthand and no
/// computed key, which is the object literal's rule and not a second one.
#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
    /// The `type` alias the named form names, or `None` for the bare form.
    pub name: Option<Name>,
    /// The attached literal's fields, in source order.
    pub fields: Vec<ObjectLiteralField>,
    /// The payload literal alone — the parenthesized list or the braced one,
    /// and the name itself where the named form wrote no list at all.
    pub payload: Span,
    /// The whole attribute, name and payload.
    pub span: Span,
}

/// A declaration modifier — visibility, `readonly`, `static`, `abstract` or
/// `final`. The parser accepts any of these anywhere a modifier list is
/// parsed (a parameter, a property, a method, a class header, ...);
/// restricting which combinations, and which positions, make sense is a
/// later check, not a grammar rule — the same discipline chunk 1 already
/// applied to a promoted constructor parameter's modifiers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[expect(missing_docs, reason = "each variant is exactly its keyword spelling")]
pub enum Modifier {
    Public,
    Protected,
    Private,
    Readonly,
    Static,
    Abstract,
    Final,
    /// `private(set)`/`protected(set)`/`public(set)` — PHP 8.4's asymmetric
    /// visibility: a separate, always-at-least-as-strict visibility for
    /// writes.
    SetVisibility(Visibility),
    /// `lateinit` — `rule:classes/lateinit`: defers a non-nullable, class/interface-typed
    /// property's first assignment past the constructor. Which
    /// types/positions actually accept it (a scalar, `?T`, a promoted
    /// parameter, `readonly`) is `nvs-types`' job, same discipline as every
    /// other modifier here.
    Lateinit,
}

/// One of the three visibility levels, as named by
/// [`Modifier::SetVisibility`] (the plain `public`/`protected`/`private`
/// modifiers are their own `Modifier` variants instead, since they can also
/// stand alone).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[expect(missing_docs, reason = "each variant is exactly its keyword spelling")]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

/// A modifier, and where it was written.
///
/// A declaration's own `modifiers` list answers what a checker asks — is this
/// one `static`? — and needs no positions to do it. A tool that writes the
/// source back out needs the other half: `nvs fmt` puts a list into
/// `rule:tooling/fmt-base-style-is-per`'s one canonical order by writing each
/// keyword where one of the others was written, so the parse has to say where
/// those places are. [`crate::Parsed`]'s `modifiers` is where a file's lists
/// are collected, and no compile path builds them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WrittenModifier {
    /// Which modifier this is.
    pub modifier: Modifier,
    /// The keyword, `(set)` suffix and all: `private(set)` is one modifier and
    /// one span.
    pub span: Span,
}

/// One parameter of a function, method or closure.
///
/// A declaration's parameter names its type (`rule:types/declaration`), and one
/// written without it is reported where it is written, `ty` staying `None` so
/// the parameter is still a node. A **closure literal's** parameter may leave
/// the type out and take it from the position the literal appears in
/// (`rule:types/callable-literal-inference`), which is the one `None` nothing
/// was reported for.
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    /// The whole parameter.
    pub span: Span,
    /// `#[...]` attribute groups, if any.
    pub attributes: Vec<AttributeGroup>,
    /// Promoted-property modifiers (constructor parameters only).
    pub modifiers: Vec<Modifier>,
    /// The declared type, or `None` where the parameter left it out — inferred
    /// on a closure literal, already reported anywhere else.
    pub ty: Option<Type>,
    /// Whether this parameter binds by reference — `inout int $x`, `rule:statements/inout-is-the-by-reference-spelling`. The mechanism is copy-in/copy-out at the call site, which is why
    /// the word is `inout` rather than `ref`.
    pub inout: bool,
    /// Whether this is a variadic parameter (`...$x`).
    pub variadic: bool,
    /// The parameter's name, `$`-sigil included.
    pub name: Span,
    /// The default value, if any.
    pub default: Option<Expr>,
}

impl Param {
    /// Whether this parameter declares a property rather than only a binding
    /// — PHP 8's constructor promotion, which
    /// `rule:classes/delegation-by-field`'s own worked example spells `constructor(private Clock $clock)`.
    ///
    /// A **visibility** keyword is what promotes, exactly as in PHP: it is
    /// the only modifier that says where the property may be read from, and
    /// `readonly` alone declares nothing to be read. The predicate lives here
    /// rather than in each of the three crates that asks it, because a
    /// promoted parameter is a property in all of them — the signature table
    /// (`nvs_types::signatures`), the slot list (`nvs_types::layout`), the
    /// `$this->x` existence check (`nvs_hir::members`) and the store the
    /// constructor makes (`nvs_ir::lower::promoted_stores`) have to agree on
    /// which parameters they are, and a fourth spelling is how they would
    /// stop agreeing.
    ///
    /// Whether the enclosing method is actually a `constructor` is not asked
    /// here: that is one caller's question, and the callers that matter are
    /// looking at a constructor already.
    #[must_use]
    pub fn is_promoted(&self) -> bool {
        self.modifiers.iter().any(|m| {
            matches!(
                m,
                Modifier::Public | Modifier::Protected | Modifier::Private
            )
        })
    }
}

/// The body of an `fn` closure literal (`rule:types/closure-literal`): an expression with an
/// implicit return, or a block requiring an explicit `return`.
#[derive(Clone, Debug, PartialEq)]
pub enum FnBody {
    /// `fn(...) => expr`
    Expr(Box<Expr>),
    /// `fn(...) => { ... }`
    Block(Block),
}

/// `fn [name] (...): T => expr` or `fn [name] (...): T => { ... }`, optionally
/// `static` — the one closure literal `rule:types/closure-literal` keeps. There is no `use`
/// clause: every outer variable the body reads is captured automatically, by
/// value (`rule:types/implicit-capture`). `name` is the optional self-name for recursion
/// (`rule:types/closure-self-name`), resolvable only inside this closure's own body.
#[derive(Clone, Debug, PartialEq)]
pub struct FnExpr {
    /// Whether declared `static` (no `$this` binding) — rejected with a
    /// diagnostic per `rule:statements/a-closure-binds-this-only-where-it-uses-it`, but still parsed so the caller can build the node and keep going.
    pub is_static: bool,
    /// The optional self-name, visible only inside `body`.
    pub name: Option<Span>,
    /// The parameter list.
    pub params: Vec<Param>,
    /// The declared return type, if written.
    pub return_type: Option<Type>,
    /// The closure's body.
    pub body: FnBody,
}

/// One arm of a `match` expression.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchArm {
    /// The comma-separated conditions, or `None` for the `default` arm.
    pub conditions: Option<Vec<Expr>>,
    /// The arm's result expression.
    pub body: Expr,
    /// The whole arm.
    pub span: Span,
}

/// One arm of an expression-level `catch` — `rule:expressions/catch-expression`.
///
/// Deliberately not a [`CatchClause`]: an arm's body is an [`Expr`] where a
/// clause's is a [`Block`], and that one difference is the whole of `rule:expressions/catch-arm-is-an-expression` — `throw` is admitted because it is already an expression, and `return`
/// is refused because it is not.
#[derive(Clone, Debug, PartialEq)]
pub struct CatchArm {
    /// The caught class, one per arm. A union parses and is refused where a
    /// clause's is, since the binding still carries one static type.
    pub ty: Type,
    /// The bound variable, if named.
    pub var: Option<Span>,
    /// The arm's result expression.
    pub body: Expr,
    /// The whole arm, `catch` through the end of the body.
    pub span: Span,
}

/// What `new` instantiates.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum NewTarget {
    /// `new Foo(...)`, `new Core\Bytes(...)`.
    Name(Name),
    /// `new self(...)`
    SelfTy,
    /// `new static(...)`
    StaticTy,
    /// `new parent(...)`
    ParentTy,
    /// `new $class(...)`, `new (expr)(...)` — a runtime-computed class value.
    Expr(Box<Expr>),
    /// `new class (...) extends X implements Y { ... }` — an anonymous
    /// class declaration used directly as a `new` target.
    AnonClass(Box<AnonClassDecl>),
}

/// The body of an anonymous class (`new class { ... }`): everything
/// [`ClassDecl`] has except a name and its own `abstract`/`final` modifiers,
/// neither of which PHP allows here.
#[derive(Clone, Debug, PartialEq)]
pub struct AnonClassDecl {
    /// The whole declaration, `class` through the closing brace.
    pub span: Span,
    /// The single superclass, if any.
    pub extends: Option<Name>,
    /// The implemented interfaces, in source order.
    pub implements: Vec<Name>,
    /// The class body's members, in source order.
    pub members: Vec<ClassMember>,
}

/// The key of one `spawn script … with(…)` option
/// ([`docs/spec/00-overview.md` § 2](/docs/spec/00-overview.md)).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[expect(
    missing_docs,
    reason = "each variant is exactly its `with(...)` key spelling"
)]
pub enum SpawnOptionKey {
    Args,
    Limits,
    Grants,
    Output,
    On,
}

/// One `key: value` entry of a `spawn script … with(…)` clause.
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnOption {
    /// Which option this is.
    pub key: SpawnOptionKey,
    /// The option's value expression.
    pub value: Expr,
    /// The whole `key: value` entry.
    pub span: Span,
}

// ============================================================================
// Expressions
// ============================================================================

/// One expression, with the span it was parsed from.
#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    /// What kind of expression this is.
    pub kind: ExprKind,
    /// The expression's full source span.
    pub span: Span,
}

impl Expr {
    /// This expression with every layer of `(` … `)` peeled off — the one home
    /// for "parentheses group, and never change what an expression *is*".
    ///
    /// Every rule that asks what an expression **is** rather than what it
    /// evaluates to goes through this, because the answer for `($a)` is always
    /// the answer for `$a`: whether a subscript chain's root is a place to
    /// write a separated array back into
    /// (`nvs_types::expr::assign::check_write_target`), and whether a read
    /// aliases storage something else already owns
    /// (`nvs_ir::lower::Lowering::aliasing_read`). Both answered as if a
    /// parenthesised local were a temporary before this existed, which made
    /// `($a)["0"] = "y"` panic in `nvs-ir` and `array<string> $b = ($a);`
    /// release the array twice.
    ///
    /// What an expression evaluates *to* never needs this: lowering has a
    /// `Paren` arm that recurses, so the value falls out of the ordinary walk.
    #[must_use]
    pub fn unparenthesized(&self) -> &Self {
        let mut e = self;
        while let ExprKind::Paren(inner) = &e.kind {
            e = inner;
        }
        e
    }
}

/// Every expression form the parser produces.
///
/// What stands on the right of `is` (`rule:types/type-test`).
///
/// **One token decides which arm this is.** A `$variable` opens [`Self::Value`]
/// — the dynamic class test, whose operand holds a `class<T>` — and every other
/// token starts a type, parsed by the production `as` uses. Deciding on the
/// sigil rather than on what happens to parse is what keeps a DNF type's
/// opening `(` a type.
#[derive(Clone, Debug, PartialEq)]
pub enum TestOperand {
    /// A written type: `$x is int`, `$x is Request`, `$x is {x: int}`.
    Type(Type),
    /// A value carrying the class descriptor to test against: `$x is $cls`.
    Value(Box<Expr>),
}

impl TestOperand {
    /// The span of whichever side was written, which is what the `is`
    /// expression's own span runs to.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Type(ty) => ty.span,
            Self::Value(expr) => expr.span,
        }
    }
}

/// Literal payloads are spans, not cooked values — see the module docs.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    /// `null`
    Null,
    /// `true` / `false`
    Bool(bool),
    /// An integer literal; the digits are cooked later.
    Int(Span),
    /// A float literal; the digits are cooked later.
    Float(Span),
    /// A duration literal — `30s`, `1h30m`
    /// (`rule:types/duration-literal`).
    ///
    /// A span like every other literal, cooked by
    /// [`crate::duration::parse`] wherever the value is wanted. The lexer has
    /// already run that same parser and reported any failure, so cooking one
    /// cannot fail.
    ///
    /// Its type is `Core\Time\Duration` and nothing places it, unlike
    /// `rule:types/decimal`'s fractional
    /// literal — the suffix *is* the type (`rule:types/duration-literal`).
    Duration(Span),
    /// A single-quoted string, or a double-quoted/heredoc/nowdoc string with
    /// no interpolation in it — both are plain literal text, uncooked.
    Str(Span),
    /// A double-quoted string or heredoc with at least one interpolation site.
    Interpolated(Vec<StringPart>),
    /// ``html`…` `` — a markup literal (`rule:core-classes/html-literal`).
    ///
    /// The same [`StringPart`] vector [`Interpolated`](Self::Interpolated)
    /// carries, and for the same reason: a segment is literal text and a hole
    /// is an ordinary expression. What differs is the type it is given and
    /// what happens to a hole on the way out — the segments are trusted
    /// because the author wrote them, and every hole is escaped. A literal
    /// with no holes is one [`StringPart::Text`] rather than an
    /// [`Str`](Self::Str), because the node, not the part count, is what says
    /// this is `Core\Html\Markup`.
    Markup(Vec<StringPart>),
    /// `$name`
    Variable(Span),
    /// A bare constant fetch: `FOO`, `Core\Bytes::class`'s `Core\Bytes` part is
    /// not this — this is only a name used where a value is expected directly,
    /// e.g. `FOO` in `$x = FOO;`.
    ConstFetch(Name),
    /// `self`
    SelfExpr,
    /// `static`
    StaticExpr,
    /// `parent`
    ParentExpr,
    /// `[...]` or legacy `array(...)`.
    ArrayLiteral(Vec<ArrayItem>),
    /// A prefix unary operator.
    Unary {
        /// Which operator.
        op: UnaryOp,
        /// The operand.
        expr: Box<Expr>,
    },
    /// `++$x` / `--$x` — the prefix form; the postfix form is
    /// [`ExprKind::PostIncDec`].
    PreIncDec {
        /// Which operator.
        op: IncDecOp,
        /// The operand.
        expr: Box<Expr>,
    },
    /// `$x++` / `$x--` — the postfix form; the prefix form is
    /// [`ExprKind::PreIncDec`].
    PostIncDec {
        /// Which operator.
        op: IncDecOp,
        /// The operand.
        expr: Box<Expr>,
    },
    /// A binary operator.
    Binary {
        /// Which operator.
        op: BinaryOp,
        /// The left operand.
        lhs: Box<Expr>,
        /// The right operand.
        rhs: Box<Expr>,
    },
    /// An assignment, `=` or a compound form.
    Assign {
        /// Which operator.
        op: AssignOp,
        /// The assignment target.
        target: Box<Expr>,
        /// The value assigned.
        value: Box<Expr>,
        /// Whether this is `target = &value` — binds `target` as a reference
        /// to `value` rather than copying it. Only ever set alongside
        /// `AssignOp::Assign`; PHP has no reference form of a compound
        /// operator like `+=`.
        by_ref: bool,
    },
    /// `cond ? then : else`, or the Elvis form `cond ?: else` when `then` is
    /// `None`.
    Ternary {
        /// The condition.
        cond: Box<Expr>,
        /// The `then` branch, or `None` for `?:`.
        then: Option<Box<Expr>>,
        /// The `else` branch.
        else_: Box<Expr>,
    },
    /// `expr as Type` — the checked conversion operator
    /// ([`docs/spec/00-overview.md` § 3.4](/docs/spec/00-overview.md)).
    Conversion {
        /// The value being converted.
        expr: Box<Expr>,
        /// The target type.
        ty: Type,
    },
    /// `expr is Type` and `expr is $cls` — the one type test
    /// (`rule:php-migration/one-type-test`). Which of the two the right side
    /// is, is [`TestOperand`]'s question.
    TypeTest {
        /// The value being tested.
        expr: Box<Expr>,
        /// The type, or the class reference, it is tested against.
        against: TestOperand,
    },
    /// `callee(...)`
    Call {
        /// The value being called.
        callee: Box<Expr>,
        /// The call's arguments.
        args: CallArgs,
    },
    /// `object->method(...)` / `object?->method(...)`
    MethodCall {
        /// The receiver.
        object: Box<Expr>,
        /// Whether this is the nullsafe form (`?->`).
        nullsafe: bool,
        /// The method being called.
        method: MemberName,
        /// The type arguments written between the name and the `(`, empty
        /// when none were — see [`ExprKind::StaticCall::type_args`].
        type_args: Vec<Type>,
        /// The call's arguments.
        args: CallArgs,
    },
    /// `Class::method(...)`
    StaticCall {
        /// The class the method is called on.
        class: Box<Expr>,
        /// The method being called.
        method: MemberName,
        /// The type arguments written between the name and the `(` —
        /// `<User>` in `Core\Json::decodeAs<User>($body)` — and empty when
        /// none were written.
        ///
        /// Grammar only: which members accept one, how many, and what a
        /// written argument binds are all the checker's, and
        /// `rule:attributes/call-site-type-argument` keeps type variables
        /// compiler-owned, so a user-declared method never takes one.
        type_args: Vec<Type>,
        /// The call's arguments.
        args: CallArgs,
    },
    /// `object->prop` / `object?->prop`
    PropertyAccess {
        /// The receiver.
        object: Box<Expr>,
        /// Whether this is the nullsafe form (`?->`).
        nullsafe: bool,
        /// The property being accessed.
        property: MemberName,
    },
    /// `Class::$prop`
    StaticPropertyAccess {
        /// The class the property is accessed on.
        class: Box<Expr>,
        /// The property's name, `$`-sigil included.
        name: Span,
    },
    /// `Class::CONST`
    ClassConstAccess {
        /// The class the constant is accessed on.
        class: Box<Expr>,
        /// The constant's name.
        name: Span,
    },
    /// `Class::class`
    ClassNameConst {
        /// The class named.
        class: Box<Expr>,
    },
    /// `base[index]`, or `base[]` (append — `index` is `None`) as an
    /// assignment target.
    Index {
        /// The array or offsettable value.
        base: Box<Expr>,
        /// The subscript, or `None` for `$a[] = …`.
        index: Option<Box<Expr>>,
    },
    /// `new Target(...)`
    New {
        /// What is being instantiated.
        target: NewTarget,
        /// The type arguments written between the target and the `(` —
        /// `<Tag>` in `new Core\ObjectSet<Tag>()` — and empty when none were
        /// written. Read by the same `parse_call_type_args` a call site's own
        /// list goes through, so the ambiguity trade and
        /// the requirement that a `(` follow are shared verbatim with
        /// [`ExprKind::StaticCall::type_args`].
        ///
        /// Grammar only, again: `rule:attributes/call-site-type-argument`
        /// keeps type variables compiler-owned, so the only target that
        /// accepts one is a compiler-owned generic class and every other
        /// written list is the checker's to refuse.
        type_args: Vec<Type>,
        /// The constructor's arguments.
        args: CallArgs,
    },
    /// `clone expr`
    Clone(Box<Expr>),
    /// `fn (...) => expr` or `fn (...) => { ... }` — the one closure literal
    /// (`rule:types/closure-literal`).
    Fn(FnExpr),
    /// `match (subject) { ... }`
    Match {
        /// The value being matched.
        subject: Box<Expr>,
        /// The arms, in source order.
        arms: Vec<MatchArm>,
    },
    /// `expr catch (T $e) => expr`, with any number of arms — `rule:expressions/catch-expression`.
    Catch {
        /// The one guarded expression: everything the ternary level parsed.
        guarded: Box<Expr>,
        /// The arms, in source order. They are clauses of this one guard and
        /// not guards of each other, so the first whose class matches what
        /// `guarded` threw runs, and no arm guards the arm before it.
        arms: Vec<CatchArm>,
    },
    /// `yield`, `yield expr`, `yield key => expr`.
    Yield {
        /// The key expression, for `yield key => value`.
        key: Option<Box<Expr>>,
        /// The yielded value, or `None` for a bare `yield`.
        value: Option<Box<Expr>>,
    },
    /// `yield from expr`
    YieldFrom(Box<Expr>),
    /// `print expr`
    Print(Box<Expr>),
    /// `throw expr` — PHP 8's throw-as-expression.
    Throw(Box<Expr>),
    /// `isset(expr, ...)`
    Isset(Vec<Expr>),
    /// `empty(expr)`
    Empty(Box<Expr>),
    /// `exit`, optionally with a status/message expression. `die` is a
    /// rejected synonym (`rule:statements/exit-is-the-only-termination-keyword`) and never reaches this variant.
    Exit(Option<Box<Expr>>),
    /// `spawn script path with(...)`
    /// ([`docs/spec/00-overview.md` § 2](/docs/spec/00-overview.md)).
    SpawnScript {
        /// The path expression, evaluated once at the spawn site.
        path: Box<Expr>,
        /// The `with(...)` options, if a `with` clause was given.
        options: Vec<SpawnOption>,
    },
    /// `await expr` — the prefix half of the same grammar
    /// ([`docs/spec/00-overview.md` § 2](/docs/spec/00-overview.md)):
    /// the awaitable handle a `spawn script` produced becomes a
    /// `ScriptResult`. Contextual like `spawn` itself — see
    /// [`crate::token`]'s module docs — so it is this variant only where an
    /// operand follows, and a bare `await` is still an ordinary name.
    Await(Box<Expr>),
    /// `require` — an expression, not a statement, per
    /// [`docs/spec/00-overview.md` § 2](/docs/spec/00-overview.md):
    /// `$x = require 'a.nvs';` is legal.
    /// `rule:statements/require-is-the-only-inclusion-construct`
    /// is why this is the only same-frame inclusion keyword left — `include`,
    /// `include_once` and `require_once` are rejected at parse time instead
    /// of reaching the AST at all.
    Require {
        /// The path expression.
        path: Box<Expr>,
    },
    /// A parenthesized expression, `(expr)`. Kept as its own node — rather
    /// than discarded in favour of the inner expression — only so its span
    /// covers the parentheses; it carries no other meaning.
    Paren(Box<Expr>),
    /// `{name: value, ...}` — `rule:types/object-literal`: an anonymous, methodless object
    /// literal. May be empty (`{a: 1}`'s fields are the common case, but
    /// `{}` is not rejected here). The two positions where `{` already means
    /// a block (a bare statement, an arrow-bodied `fn`'s body) never reach
    /// this variant for an empty `{}` — the disambiguating lookahead only
    /// recognizes a *non*-empty literal attempt (`{ident :`), so an empty
    /// `{}` there stays an empty block, exactly as before this ADR.
    ObjectLiteral(Vec<ObjectLiteralField>),
    /// A placeholder produced during error recovery, carrying the span of what
    /// it stood in for: the construct that was refused, or the insertion point
    /// where a required expression was not written. A diagnostic was already
    /// reported there.
    ///
    /// It is carried on the variant rather than left to be read off the
    /// [`Expr`] around it, because a consumer matching on a kind is the one
    /// that has to know: the variant says the node was invented, and the span
    /// says what it replaced. Neither span's *width* is ever the signal — a
    /// required expression that was never written stands in for a caret
    /// (`rule:ide/recovery-is-explicit`).
    Error(Span),
}

// ============================================================================
// Statements
// ============================================================================

/// A `{ ... }` block of statements.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// The statements, in source order.
    pub stmts: Vec<Stmt>,
    /// The whole block, braces included.
    pub span: Span,
}

/// One statement.
#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    /// What kind of statement this is.
    pub kind: StmtKind,
    /// The statement's full source span.
    pub span: Span,
}

/// One binding of a `foreach` header (`rule:types/grammar`.2): a mandatory type and
/// a name. A reference marker only ever applies to the *value* binding, never
/// the key, so it lives on [`StmtKind::Foreach`] rather than here.
#[derive(Clone, Debug, PartialEq)]
pub struct ForeachBinding {
    /// The declared type, or `None` if omitted (a diagnostic was already
    /// reported for the omission, mirroring [`Param::ty`]).
    pub ty: Option<Type>,
    /// The bound variable's name, `$`-sigil included.
    pub name: Span,
    /// The whole binding.
    pub span: Span,
}

/// A `for` header's init clause, `rule:iteration/for-init-clause`. Either one typed local
/// declaration — `rule:types/grammar`.1's, unchanged and in full, including
/// `rule:types/var-inference`'s `var` spelling — or the comma-separated expression list PHP's
/// own `for` grammar has, which is what the condition and step clauses still
/// are. Never both and never two declarations; `E0124` and `E0125` are what
/// those two shapes are told (`rule:iteration/for-init-refusals`).
///
/// Scope is unchanged by the declaration form: the counter is function-scoped
/// exactly as every other binding is (`rule:iteration/for-counter-scope`), which is why this carries
/// a whole [`Stmt`] rather than a loop-scoped binding of its own.
#[derive(Clone, Debug, PartialEq)]
pub enum ForInit {
    /// `for (int $i = 0; …)` — exactly one [`StmtKind::LocalDecl`], boxed as a
    /// whole statement so a walker reaches it through the arm it already has
    /// for a declaration written anywhere else.
    Decl(Box<Stmt>),
    /// `for ($i = 0, $j = 1; …)`, empty for `for (;;)`.
    Exprs(Vec<Expr>),
}

impl ForInit {
    /// The clause's expressions — empty for the declaration form, whose
    /// initializer is reached through [`Self::decl`] instead. A walker that
    /// only cares about expressions keeps its existing `chain` this way.
    #[must_use]
    pub fn exprs(&self) -> &[Expr] {
        match self {
            Self::Decl(_) => &[],
            Self::Exprs(exprs) => exprs,
        }
    }

    /// The clause's declaration, if it is the declaration form.
    #[must_use]
    pub fn decl(&self) -> Option<&Stmt> {
        match self {
            Self::Decl(decl) => Some(decl),
            Self::Exprs(_) => None,
        }
    }
}

/// One `catch` clause of a `try` statement. PHP 8's multi-type catch
/// (`Type|Type $e`) and optional variable both fall out of reusing the
/// ordinary type grammar and an `Option` — no separate type-list is needed.
#[derive(Clone, Debug, PartialEq)]
pub struct CatchClause {
    /// The caught type, or types via an ordinary union (`A|B`).
    pub ty: Type,
    /// The bound variable, if named.
    pub var: Option<Span>,
    /// The clause's body.
    pub body: Block,
    /// The whole clause, `catch` through the closing brace.
    pub span: Span,
}

/// One `case`/`default` arm of a `switch`.
#[derive(Clone, Debug, PartialEq)]
pub struct SwitchCase {
    /// The `case` value, or `None` for `default`.
    pub cond: Option<Expr>,
    /// The statements up to the next `case`/`default`/closing brace.
    /// Fallthrough is simply not inserting a `break` — there is no separate
    /// representation for it.
    pub body: Vec<Stmt>,
    /// The whole arm, `case`/`default` through its last statement.
    pub span: Span,
}

/// One leaf, nested target, or empty slot inside a destructuring pattern
/// (`rule:types/grammar`.3).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DestructureElement {
    /// An empty slot, `[, $b] = …` — skips one position without binding it.
    Skip,
    /// A typed leaf binding: `(key '=>')? 'inout'? type '$' identifier`.
    Leaf {
        /// `key =>`, if present.
        key: Option<Expr>,
        /// The declared type, or `None` if omitted (diagnostic already
        /// reported).
        ty: Option<Type>,
        /// Whether this leaf binds by reference (`inout int $a`).
        inout: bool,
        /// The bound variable's name, `$`-sigil included.
        name: Span,
        /// The whole element.
        span: Span,
    },
    /// A nested destructuring target: `(key '=>')? '[' ... ']'`.
    Nested {
        /// `key =>`, if present.
        key: Option<Expr>,
        /// The nested pattern.
        target: DestructureTarget,
        /// The whole element, key included.
        span: Span,
    },
}

/// A destructuring pattern: `[...]`, or `list(...)` as a second spelling with
/// the same typed-leaf requirement — both produce this same shape, since the
/// two spellings differ only in delimiter, never in what they accept inside.
#[derive(Clone, Debug, PartialEq)]
pub struct DestructureTarget {
    /// The pattern's elements, in source order.
    pub elements: Vec<DestructureElement>,
    /// The whole pattern, delimiters included.
    pub span: Span,
}

/// One `$name (= default)?` binding of a rejected function-scope `static`
/// declaration ([`StmtKind::StaticLocal`]). Parsed only for a precise
/// diagnostic — `rule:statements/no-function-static-and-no-global` gives this construct no replacement syntax, so
/// nothing downstream ever acts on this shape.
#[derive(Clone, Debug, PartialEq)]
pub struct StaticVar {
    /// The variable's name, `$`-sigil included.
    pub name: Span,
    /// The initializer, if any.
    pub default: Option<Expr>,
}

/// Every statement form the parser produces.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    /// A bare expression statement, `expr;`.
    Expr(Expr),
    /// `return;` or `return expr;`.
    Return(Option<Expr>),
    /// A nested `{ ... }` block.
    Block(Block),
    /// An empty statement, a lone `;` — most often a loop's empty body
    /// (`while ($more_work());`), and also the marker the parser emits for a
    /// bare code-tag token (`<?nvs`, `?>`, or the rejected `<?php` — see
    /// `rule:statements/nvs-is-the-only-open-tag`) encountered where a statement was expected — see
    /// [`Self::InlineHtml`]'s doc for why that token, and not this one, is
    /// where the interesting span lives.
    Empty,
    /// A run of literal HTML/text between two code tags, emitted verbatim —
    /// spec `00-overview.md` § 1. Unlike [`Self::Echo`], this text is never
    /// escaped or interpreted: the span points straight at the source bytes.
    /// Reachable anywhere a statement is expected, not just at file scope,
    /// because `?>`/`<?nvs` can appear inside a block
    /// (`if ($x) { ?>html<?nvs }` is legal, exactly as in PHP).
    InlineHtml(Span),
    /// `if (cond) then (elseif (cond2) then2)* (else else_)?`. An `elseif`
    /// or an `else if` both collapse to the same shape: a nested `If` inside
    /// `else_`, indistinguishable from a source-level `else { if (...) }`.
    If {
        /// The condition.
        cond: Expr,
        /// The branch taken when `cond` is true.
        then: Box<Stmt>,
        /// The branch taken otherwise, if any.
        else_: Option<Box<Stmt>>,
    },
    /// `while (cond) body`.
    While {
        /// The loop condition, tested before each iteration.
        cond: Expr,
        /// The loop body.
        body: Box<Stmt>,
    },
    /// `do body while (cond);`.
    DoWhile {
        /// The loop body, run at least once.
        body: Box<Stmt>,
        /// The loop condition, tested after each iteration.
        cond: Expr,
    },
    /// `for (init; cond; step) body`. The condition and step clauses are each
    /// a comma-separated list of expressions, any of which may be empty —
    /// PHP's own `for` grammar. The init clause is [`ForInit`], which adds
    /// `rule:iteration/for-init-clause`'s declaration form to that list.
    For {
        /// The initializer, run once before the first iteration: one typed
        /// local declaration or a list of expressions (`rule:iteration/for-init-clause`).
        init: ForInit,
        /// The condition expressions; only the last one's truthiness is
        /// tested, exactly as PHP evaluates a comma list here.
        cond: Vec<Expr>,
        /// The step expressions, run after each iteration.
        step: Vec<Expr>,
        /// The loop body.
        body: Box<Stmt>,
    },
    /// `foreach (subject as key? value) body`, `rule:types/grammar`.2's mandatory
    /// typed bindings. `value_inout` is the one place a reference marker
    /// may appear; a key binding never carries one.
    Foreach {
        /// The value being iterated.
        subject: Expr,
        /// The key binding, if the header names one.
        key: Option<ForeachBinding>,
        /// The value binding.
        value: ForeachBinding,
        /// Whether the value binds by reference (`inout int $v`).
        value_inout: bool,
        /// The loop body.
        body: Box<Stmt>,
    },
    /// `switch (subject) { case ... default ... }`.
    Switch {
        /// The value being matched against each `case`.
        subject: Expr,
        /// The arms, in source order.
        cases: Vec<SwitchCase>,
    },
    /// `break;` or `break level;`.
    Break(Option<Expr>),
    /// `continue;` or `continue level;`.
    Continue(Option<Expr>),
    /// `try { ... } (catch (...) { ... })* (finally { ... })?`.
    Try {
        /// The protected body.
        body: Block,
        /// The `catch` clauses, in source order.
        catches: Vec<CatchClause>,
        /// The `finally` block, if any.
        finally: Option<Block>,
    },
    /// `echo expr, expr, ...;`.
    Echo(Vec<Expr>),
    /// `unset(expr, ...);` — kept as an ordinary accepted builtin, unlike
    /// `extract`/`settype`.
    Unset(Vec<Expr>),
    /// `rule:types/grammar`.1's typed local declaration:
    /// `type '$' identifier ('=' expr)? ';'`. Exactly one binding per
    /// statement — there is no comma-separated multi-declaration form.
    ///
    /// `rule:types/var-inference` adds a second spelling, `'var' '$' identifier '=' expr ';'`,
    /// with no type written at all — [`None`] here means "infer it from
    /// `value`'s own checked type," never "no type." `value` is mandatory in
    /// that case; the parser never produces `ty: None, value: None`.
    LocalDecl {
        /// The declared type, or [`None`] for `var`'s inferred spelling.
        ty: Option<Type>,
        /// The declared variable's name, `$`-sigil included.
        name: Span,
        /// The initializer, if any (always present when `ty` is [`None`]).
        value: Option<Expr>,
    },
    /// `rule:types/grammar`.3's destructuring statement:
    /// `destructure-target '=' expr ';'`.
    Destructure {
        /// The left-hand pattern.
        target: DestructureTarget,
        /// The value being destructured.
        value: Expr,
    },
    /// `global $x, $y;` — rejected, `rule:statements/no-function-static-and-no-global`. Still parses to the
    /// variables named, for a precise diagnostic.
    Global(Vec<Span>),
    /// `goto label;` — rejected: makes the control-flow graph unstructured.
    /// Still parses to the label named, for a precise diagnostic.
    Goto(Span),
    /// A function-scope `static $x (= expr)?, ...;` declaration — rejected,
    /// `rule:statements/no-function-static-and-no-global`. `ty` is `Some` only for the illustrative-but-still-
    /// rejected typed spelling that ADR's own diagnostic wording uses
    /// (`static int $calls = 0;`); ordinary PHP's untyped
    /// `static $calls = 0;` leaves it `None`.
    StaticLocal {
        /// The declared type, if the rejected construct was written with
        /// one.
        ty: Option<Type>,
        /// The bindings declared.
        vars: Vec<StaticVar>,
    },
    /// A class declaration.
    ClassDecl(ClassDecl),
    /// An interface declaration.
    InterfaceDecl(InterfaceDecl),
    /// An enum declaration (`rule:enums/closed-integer-type`).
    EnumDecl(EnumDecl),
    /// A `namespace` declaration, either form.
    NamespaceDecl(NamespaceDecl),
    /// A `use Path\To\Name;` import.
    UseDecl(UseDecl),
    /// An `autoload` declaration, either form (`rule:programs/autoload`).
    AutoloadDecl(AutoloadDecl),
    /// `type Name = TypeExpr;` (`rule:types/grammar`.5 / `rule:types/type-alias`), at
    /// file/namespace scope.
    TypeAliasDecl(TypeAliasDecl),
    /// `function foo() { ... }` outside any class body — rejected, `rule:classes/no-free-functions-or-constants`. Still parses to a full [`MethodMember`] shape, for a precise
    /// diagnostic; nothing downstream ever acts on it.
    TopLevelFunction(MethodMember),
    /// `const FOO = 1 (, BAR = 2)*;` outside any class body — rejected,
    /// `rule:classes/no-free-functions-or-constants`. Still parses to full [`ConstMember`] shapes (one per
    /// comma-separated declarator), for a precise diagnostic; nothing
    /// downstream ever acts on it.
    TopLevelConst(Vec<ConstMember>),
    /// A placeholder produced during error recovery.
    Error,
}

// ============================================================================
// Declarations: classes, interfaces, traits, enums, and their members
// (`rule:enums/closed-integer-type`, `rule:classes/no-free-functions-or-constants`, `rule:classes/property-observer`); `namespace`, `use` and `type`-alias
// declarations (`rule:types/grammar`.5, `rule:statements/nothing-gets-a-second-name`)
// ============================================================================

/// The `///` run a declaration carries
/// (`rule:tooling/doc-comment-attaches-to-the-next-declaration`).
///
/// Lines rather than one blob, and spans rather than text, for the same reason
/// every other node here holds a [`Span`]: the source is the text. Each line
/// covers its own `///` through the last byte before the newline, marker
/// included, so the one place that decides where the prose starts is the
/// consumer that strips it — nothing downstream has to agree with the parser
/// about a column.
#[derive(Clone, Debug, PartialEq)]
pub struct DocComment {
    /// The whole run, the first line's first slash through the last line's end.
    pub span: Span,
    /// The run's lines, in source order. Never empty.
    pub lines: Vec<Span>,
    /// The `@see` and `@example` lines, in source order. Any other tag was
    /// refused where it was written, so this holds only the two.
    pub tags: Vec<DocTag>,
}

/// One tag line of a [`DocComment`]
/// (`rule:tooling/doc-comment-tags-are-see-and-example`).
#[derive(Clone, Debug, PartialEq)]
pub struct DocTag {
    /// Which of the two this is.
    pub kind: DocTagKind,
    /// The `@` through the end of the line.
    pub span: Span,
    /// What the tag names — a member for `@see`, a path for `@example` — with
    /// the whitespace around it already off. Empty where nothing was written
    /// after the tag, which the check that resolves it answers rather than the
    /// grammar: a tag naming nothing and a tag naming something absent are the
    /// same mistake and deserve the same diagnostic.
    pub argument: Span,
}

/// The closed set of doc-comment tags.
///
/// Deliberately not `#[non_exhaustive]`, for
/// [`TriviaKind`](crate::TriviaKind)'s reason: the set is closed by
/// `rule:tooling/doc-comment-tags-are-see-and-example`, and a third tag has to
/// fail to compile everywhere one is read rather than fall into a `_` arm that
/// renders it as prose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocTagKind {
    /// `@see <member>` — a cross-reference that must resolve.
    See,
    /// `@example <path>` — a file that must exist and must be one the test
    /// corpus walks, so it cannot stop compiling unnoticed.
    Example,
}

/// `class Name (extends Base)? (implements Iface, ...)? { ... }`.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassDecl {
    /// The whole declaration, `class` through the closing brace (attributes
    /// and modifiers, if any, are not included).
    pub span: Span,
    /// The `///` run above it, if one is attached.
    pub doc: Option<DocComment>,
    /// `#[...]` attribute groups, if any.
    pub attributes: Vec<AttributeGroup>,
    /// `abstract`/`final`, if written. The parser accepts any [`Modifier`]
    /// here; which ones make sense on a class is a later check.
    pub modifiers: Vec<Modifier>,
    /// The declared name.
    pub name: Name,
    /// The single superclass, if any.
    pub extends: Option<Name>,
    /// The implemented interfaces, in source order, each with its optional
    /// `by $field` delegation suffix (`rule:classes/delegation-by-field`).
    pub implements: Vec<ImplementsClause>,
    /// The class body's members, in source order.
    pub members: Vec<ClassMember>,
}

/// One entry of a class's `implements` list (`rule:classes/delegation-by-field`): the interface
/// named, its `<...>` type arguments if any, plus its optional `by $field`
/// delegation suffix. `by_field` is recorded as a span and nothing more —
/// checking that `$field` is a declared property whose type actually
/// satisfies `name` (`E_DELEGATE_TYPE_MISMATCH`) needs the signature table
/// and the class graph, so it is `nvs_types::conformance`'s job rather than
/// the parser's.
#[derive(Clone, Debug, PartialEq)]
pub struct ImplementsClause {
    /// The interface named.
    pub name: Name,
    /// `implements Iterable<int>` — `rule:iteration/concrete-generic-implements`'s one narrow extension:
    /// a class may fix a *compiler-owned* generic interface's parameter at a
    /// concrete type here. Empty for every other `implements` entry, and the
    /// checker refuses a non-empty list on a name that is not generic. See
    /// [`TypeAtom::Name`]'s own docs.
    pub type_args: Vec<Type>,
    /// `by $field`, if written — the property every method `name` requires
    /// is forwarded to.
    pub by_field: Option<Span>,
    /// The whole entry, name through the closing `>` or the delegated-to
    /// property, whichever was written last.
    pub span: Span,
}

/// `interface Name (extends Base, ...)? { ... }`. PHP allows an interface to
/// extend more than one other interface, unlike a class's single `extends`.
#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceDecl {
    /// The whole declaration, `interface` through the closing brace.
    pub span: Span,
    /// The `///` run above it, if one is attached.
    pub doc: Option<DocComment>,
    /// `#[...]` attribute groups, if any.
    pub attributes: Vec<AttributeGroup>,
    /// The declared name.
    pub name: Name,
    /// The extended interfaces, in source order.
    pub extends: Vec<Name>,
    /// The interface body's members (method signatures, consts, and PHP
    /// 8.4's abstract property hooks), in source order.
    pub members: Vec<ClassMember>,
}

/// `enum Name (: BackingType)? (implements Iface, ...)? { cases... }`
/// (`rule:enums/closed-integer-type`). `implements`, and any [`ClassMember`] other than a case, are
/// rejected — both still parse, so the diagnostic can be precise.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumDecl {
    /// The whole declaration, `enum` through the closing brace.
    pub span: Span,
    /// The `///` run above it, if one is attached.
    pub doc: Option<DocComment>,
    /// `#[...]` attribute groups, if any.
    pub attributes: Vec<AttributeGroup>,
    /// The declared name.
    pub name: Name,
    /// The `: Type` backing-type clause, if written. Parsed with the full
    /// `rule:types/grammar` grammar; that only `int`/`uint` are legal (no `string`,
    /// no other atom) is enforced only for the one case `rule:enums/no-class-machinery` names
    /// explicitly (`string`) — anything else is a later check.
    pub backing: Option<Type>,
    /// `implements ...` — always rejected (`rule:enums/no-class-machinery`).
    pub implements: Vec<Name>,
    /// The declared cases, in source order.
    pub cases: Vec<EnumCase>,
    /// Any member other than a case. A `type` alias the enum owns is kept
    /// ([`ClassMemberKind::TypeAlias`]); every other member shape is rejected
    /// (`rule:enums/no-class-machinery`), an enum declaring only cases, an
    /// optional backing type and its own aliases.
    pub members: Vec<ClassMember>,
}

/// One `Name (= expr)?` case of an [`EnumDecl`].
#[derive(Clone, Debug, PartialEq)]
pub struct EnumCase {
    /// The whole case, name through its optional value.
    pub span: Span,
    /// The `///` run above it, if one is attached.
    pub doc: Option<DocComment>,
    /// `#[...]` attribute groups, if any.
    pub attributes: Vec<AttributeGroup>,
    /// The case's name.
    pub name: Name,
    /// The explicit value, if written; omitted, a case takes the previous
    /// case's value plus one (`rule:enums/declaration`) — a later stage's job, not the
    /// parser's.
    pub value: Option<Expr>,
}

/// One member of a class, interface, trait or (rejected, except for
/// [`ClassMemberKind::Error`]-free recovery) enum body.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassMember {
    /// What kind of member this is.
    pub kind: ClassMemberKind,
    /// The member's full span, attributes and modifiers included.
    pub span: Span,
    /// The `///` run above it, if one is attached. One declaration can produce
    /// several members (`public int $a, $b;`), and the run documents the
    /// declaration, so every member it produced carries the same one.
    pub doc: Option<DocComment>,
}

/// The shape of a [`ClassMember`].
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ClassMemberKind {
    /// A property declaration, with or without hooks.
    Property(PropertyMember),
    /// A class constant declaration.
    Const(ConstMember),
    /// A method declaration, abstract (`body: None`) or concrete.
    Method(MethodMember),
    /// A `type Name = TypeExpr;` alias the body owns, reached as `Owner::Name`
    /// from anywhere and as a bare `Name` inside the owner
    /// (`rule:types/type-alias`).
    TypeAlias(TypeAliasDecl),
    /// A placeholder produced during error recovery — also what a rejected
    /// class-body `use TraitName, ...;` becomes, since `rule:classes/no-traits` leaves no
    /// AST node to carry it.
    Error,
}

/// `modifiers type $name (= expr)?;`, or the hooked form
/// `modifiers type $name { hooks... }` (PHP 8.4 property hooks, feeding
/// `PropertyObserver` — `rule:classes/property-observer`). A declaration naming several properties
/// at once (`public int $a, $b;`) is flattened into one [`ClassMember`] per
/// name at parse time — hooks apply to exactly one property, so this loses
/// nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyMember {
    /// `#[...]` attribute groups, if any.
    pub attributes: Vec<AttributeGroup>,
    /// Visibility, `readonly`, `static`, and PHP 8.4's asymmetric-visibility
    /// `(set)` modifier, in any combination the parser accepts permissively.
    pub modifiers: Vec<Modifier>,
    /// The declared type.
    pub ty: Type,
    /// The property's name, `$`-sigil included.
    pub name: Span,
    /// The default value, if any. Never present together with `hooks`.
    pub default: Option<Expr>,
    /// The `{ get ...; set ...; }` hook block, if written; `None` for an
    /// ordinary, `;`- or `= expr;`-terminated property.
    pub hooks: Option<Vec<PropertyHook>>,
}

/// One `get`/`set` hook inside a [`PropertyMember`]'s hook block.
#[derive(Clone, Debug, PartialEq)]
pub struct PropertyHook {
    /// The whole hook, `get`/`set` through its body.
    pub span: Span,
    /// `#[...]` attribute groups, if any.
    pub attributes: Vec<AttributeGroup>,
    /// Whether this is `get` or `set`.
    pub kind: PropertyHookKind,
    /// `set(Type $name)`'s parameter, if given explicitly. Unlike an
    /// ordinary [`Param`], its type may be omitted with no diagnostic —
    /// PHP 8.4 infers it from the property's own type.
    pub param: Option<Param>,
    /// The hook's body; `None` for an abstract hook (`get;`), legal only in
    /// an interface or an abstract class.
    pub body: Option<PropertyHookBody>,
}

/// Which accessor a [`PropertyHook`] is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropertyHookKind {
    /// `get`
    Get,
    /// `set`
    Set,
}

/// The body of a [`PropertyHook`] that isn't abstract.
#[derive(Clone, Debug, PartialEq)]
pub enum PropertyHookBody {
    /// `=> expr;` — the short form.
    Expr(Box<Expr>),
    /// `{ ... }` — the block form.
    Block(Block),
}

/// `modifiers const type? Name = expr;` (`rule:types/declaration`'s
/// `public const int MAX = 10;`, PHP 8.3's optional type). A declaration
/// naming several constants at once (`public const int A = 1, B = 2;`) is
/// flattened into one [`ClassMember`] per name at parse time, same as
/// [`PropertyMember`].
///
/// Also used, with an empty `modifiers` list, for a rejected top-level
/// `const` declaration ([`StmtKind::TopLevelConst`]) — the shape is
/// identical, only the legality of where it sits differs.
#[derive(Clone, Debug, PartialEq)]
pub struct ConstMember {
    /// `#[...]` attribute groups, if any.
    pub attributes: Vec<AttributeGroup>,
    /// Visibility, if written.
    pub modifiers: Vec<Modifier>,
    /// The declared type, if written — optional per PHP 8.3.
    pub ty: Option<Type>,
    /// The constant's name (no sigil).
    pub name: Span,
    /// The constant's value.
    pub value: Expr,
}

/// `modifiers function '&'? name(params) (: ReturnType)? (block | ';')`.
///
/// Also used, with an empty `modifiers` list, for a rejected top-level
/// `function` declaration ([`StmtKind::TopLevelFunction`]) — the shape is
/// identical, only the legality of where it sits differs.
#[derive(Clone, Debug, PartialEq)]
pub struct MethodMember {
    /// `#[...]` attribute groups, if any.
    pub attributes: Vec<AttributeGroup>,
    /// Visibility, `static`, `abstract`, `final`, in any combination the
    /// parser accepts permissively.
    pub modifiers: Vec<Modifier>,
    /// The method's name (no sigil) — a keyword-shaped spelling (`list`,
    /// `default`, ...) is accepted, same as a member name after `->`/`::`.
    pub name: Span,
    /// The parameter list.
    pub params: Vec<Param>,
    /// The declared return type, if written.
    pub return_type: Option<Type>,
    /// The method's body; `None` for an abstract method or an interface's
    /// method signature, both of which end in `;` instead.
    pub body: Option<Block>,
}

/// `namespace Name;` or `namespace Name? { ... }`.
#[derive(Clone, Debug, PartialEq)]
pub struct NamespaceDecl {
    /// The whole declaration.
    pub span: Span,
    /// The declared namespace, or `None` for the unnamed/global form
    /// (`namespace { ... }`).
    pub name: Option<Name>,
    /// `Some` for the bracketed form (scoped to the block); `None` for the
    /// statement form, which applies to the rest of the enclosing scope.
    pub body: Option<Block>,
}

/// `use Path\To\Name;`.
#[derive(Clone, Debug, PartialEq)]
pub struct UseDecl {
    /// The whole declaration.
    pub span: Span,
    /// The imported path.
    pub path: Name,
    /// `as Alias`, if written — always rejected (`rule:statements/nothing-gets-a-second-name`): an import
    /// cannot be renamed. Parsed anyway, for a precise diagnostic.
    pub alias: Option<Span>,
}

/// `autoload 'Prefix' from 'a', 'b';` or `autoload discover 'glob';` — the
/// two forms of `rule:programs/autoload`, whose grammar [`docs/spec/00-overview.md` § 2](/docs/spec/00-overview.md)
/// owns.
///
/// Every string here is a *span*, not a cooked value, exactly as
/// [`ExprKind::Str`]'s is: the parser records what was written and
/// `nvs_hir` decodes it when it builds the map, so one decoder
/// (`nvs_hir::requires`'s) serves this and `require` alike.
#[derive(Clone, Debug, PartialEq)]
pub struct AutoloadDecl {
    /// The whole declaration, `autoload` through the `;`.
    pub span: Span,
    /// Which of the two forms was written.
    pub kind: AutoloadKind,
}

/// The two shapes an [`AutoloadDecl`] takes.
#[derive(Clone, Debug, PartialEq)]
pub enum AutoloadKind {
    /// `autoload 'Prefix' from 'root', 'root2';` — one namespace prefix and
    /// one or more roots, probed in the order written (`rule:programs/autoload`'s
    /// Composer rule).
    Prefix {
        /// The namespace prefix's literal, quotes included.
        prefix: Span,
        /// The root paths' literals, quotes included, in declaration order.
        /// Never empty in a well-formed declaration; a malformed one that
        /// reported [`code::E_AUTOLOAD_PATH_NOT_LITERAL`](nvs_diagnostics::code::E_AUTOLOAD_PATH_NOT_LITERAL)
        /// still records what it could read.
        roots: Vec<Span>,
    },
    /// `autoload discover '../../*/src';` — each directory the glob matches
    /// becomes a root whose matched segment is its own prefix. `discover` is
    /// contextual, not a reserved word: it means this only after `autoload`.
    Discover {
        /// The glob's literal, quotes included. Whether it holds exactly one
        /// `*` occupying a whole segment is `nvs_hir`'s check, not the
        /// parser's — the answer needs the cooked string.
        glob: Span,
    },
}

/// `type Name = TypeExpr;` (`rule:types/grammar`.5 / `rule:types/type-alias`), at
/// file/namespace scope or as a member of a class, interface or enum body
/// ([`ClassMemberKind::TypeAlias`]). The two sites parse identically: `TypeExpr`
/// uses the full `rule:types/grammar` grammar unconditionally, and the
/// restriction that it may not be a single bare class/interface/enum atom
/// (`rule:types/alias-is-never-a-bare-class`) is a resolution-time check, not a
/// parse-time one, so `type Id = SomeClass;` parses exactly like any other
/// alias.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeAliasDecl {
    /// The whole declaration.
    pub span: Span,
    /// The `///` run above it, if one is attached. A body member's run hangs on
    /// its [`ClassMember`] instead, so the member form leaves this `None`.
    pub doc: Option<DocComment>,
    /// The alias's declared name.
    pub name: Name,
    /// The type it stands for.
    pub ty: Type,
}

/// Whether `body` is a generator's body — `rule:iteration/generators`'s rule that "a
/// function whose body contains `yield` is a generator".
///
/// A purely syntactic question, which is why it lives here rather than in
/// `nvs-types` or `nvs-ir`: both of those need the same answer, and a fact
/// with two consumers gets one home.
///
/// # What counts, and what deliberately does not
///
/// The scan walks *statements* — every nesting construct a body can contain
/// — and recognises a `yield` written as a whole expression statement,
/// through any number of parentheses. That is the only shape the language
/// actually supports: `rule:iteration/one-way-only` gives a generator no `send()`, so `yield`
/// produces nothing for a surrounding expression to consume, and a `yield`
/// buried inside one is refused where it is *checked*, not here.
///
/// It never descends into a nested `fn` body, because a closure appears only
/// as an *expression* and this walk visits none — so `rule:types/closure-literal`'s closures
/// cannot make their enclosing method a generator, which is exactly `rule:iteration/generators`'s "`yield` is lexically confined to the generator's own body".
#[must_use]
pub fn is_generator_body(body: &Block) -> bool {
    body.stmts.iter().any(stmt_yields)
}

fn stmt_yields(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Expr(e) => is_yield_expr(e),
        StmtKind::Block(b) => is_generator_body(b),
        StmtKind::If { then, else_, .. } => {
            stmt_yields(then) || else_.as_deref().is_some_and(stmt_yields)
        }
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Foreach { body, .. } => stmt_yields(body),
        StmtKind::Switch { cases, .. } => cases.iter().any(|c| c.body.iter().any(stmt_yields)),
        StmtKind::Try {
            body,
            catches,
            finally,
        } => {
            is_generator_body(body)
                || catches.iter().any(|c| is_generator_body(&c.body))
                || finally.as_ref().is_some_and(is_generator_body)
        }
        _ => false,
    }
}

fn is_yield_expr(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Yield { .. } | ExprKind::YieldFrom(_) => true,
        ExprKind::Paren(inner) => is_yield_expr(inner),
        _ => false,
    }
}
