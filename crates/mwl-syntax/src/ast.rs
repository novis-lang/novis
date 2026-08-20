//! The AST: what the parser produces from a token stream.
//!
//! # What is here so far
//!
//! Types ([`Type`]/[`TypeKind`]/[`TypeAtom`], ADR 0007 § 3) and expressions
//! ([`Expr`]/[`ExprKind`], every construct M1's plan names — `match`, closures
//! and arrow functions, `spawn script`, `include`/`require`, named arguments,
//! spread, nullsafe, first-class callable syntax, the `as` conversion
//! operator). [`Block`]/[`Stmt`]/[`StmtKind`] now also cover every
//! control-flow statement (`if`, `while`, `do`/`while`, `for`, `foreach`'s
//! mandatory typed bindings, `switch`, `break`/`continue`, `try`/`catch`/
//! `finally`), `echo`, `unset`, ADR 0007 § 3.1's typed local declaration,
//! § 3.3's destructuring statement, and the statement-shaped rejects
//! (`global`, `goto`, function-scope `static`). Class/interface/trait/enum
//! declarations — plus `namespace`, `use` and the `type`-alias declaration —
//! are the parser's next and last M1 chunk.
//!
//! # Conventions
//!
//! An AST node stores [`Span`]s, never owned text — exactly the discipline
//! [`crate::token`] already documents for [`Token`](crate::Token): a literal's
//! value is "cooked" (an integer parsed, a string unescaped) only once a later
//! stage actually needs it, so the parser itself never allocates a `String` or
//! decides what a number means.

use mwl_diagnostics::Span;

// ============================================================================
// Names
// ============================================================================

/// A possibly-namespace-qualified name: `Foo`, `Core\Bytes`, `\Fully\Qualified`.
///
/// The lexer emits no combined token for this — it is an [`Ident`](crate::TokenKind::Ident)
/// optionally preceded by, and optionally interspersed with,
/// [`Backslash`](crate::TokenKind::Backslash) — so the parser folds the whole
/// run into one span. Splitting it into segments and resolving it to a
/// declaration is name resolution's job (M2), not this stage's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Name {
    /// The full qualified spelling, backslashes included.
    pub span: Span,
}

// ============================================================================
// Types (ADR 0007 § 3)
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
/// Mirrors ADR 0007 § 3's grammar directly: `type := union`,
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
    /// `uint` — new, unsigned, ADR 0007 § 4.
    Uint,
    /// `float`
    Float,
    /// `string`
    String,
    /// `bytes` — ADR 0009.
    Bytes,
    /// `array`, or `array<T>` when a type argument is given.
    Array(Option<Box<Type>>),
    /// `object`
    Object,
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
    /// `iterable`
    Iterable,
    /// `callable`
    Callable,
    /// `self`
    SelfTy,
    /// `static`
    StaticTy,
    /// `parent`
    Parent,
    /// A class, interface, enum or `type`-alias name — the checker (not the
    /// parser) decides which kind of atom it resolves to.
    Name(Name),
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
    /// `@expr` — error suppression.
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

/// PHP's legacy cast syntax, `(T)expr` — accepted as a second spelling of
/// `expr as T`, carrying `as`'s checked semantics rather than PHP's lossy ones
/// (ADR 0007 § 2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[expect(
    missing_docs,
    reason = "each variant is exactly the cast keyword it names"
)]
pub enum CastType {
    Int,
    Uint,
    Float,
    String,
    Bool,
    Array,
    Object,
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
    /// `and` — the low-precedence keyword form, distinct from `&&`.
    LowAnd,
    /// `or` — the low-precedence keyword form, distinct from `||`.
    LowOr,
    /// `xor` — the low-precedence keyword form; there is no `^^` operator.
    LowXor,
    Eq,
    NotEq,
    /// `===`
    Identical,
    /// `!==`
    NotIdentical,
    Lt,
    LtEq,
    Gt,
    GtEq,
    /// `<=>`
    Cmp,
    /// `??`
    Coalesce,
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

/// The name on the right of `->`, `?->` or `::` — almost always a plain
/// identifier, but PHP also allows a dynamic member name.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum MemberName {
    /// `->prop`, `->method()`, `::CONST`, `::method()` — the ordinary case.
    Ident(Span),
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

/// One piece of an interpolated string: literal text, or an interpolated
/// expression (`$name`, `$obj->prop`, `$arr[key]`, or a full `{$…}` expression).
#[derive(Clone, Debug, PartialEq)]
pub enum StringPart {
    /// A run of literal text, escapes included but not yet unescaped.
    Text(Span),
    /// One interpolation site, already reduced to an ordinary expression.
    Expr(Expr),
}

/// A modifier on a promoted constructor parameter — `public`, `protected`,
/// `private` or `readonly`. The parser accepts these on any parameter;
/// restricting them to a constructor is a later check, not a grammar rule.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[expect(missing_docs, reason = "each variant is exactly its keyword spelling")]
pub enum Modifier {
    Public,
    Protected,
    Private,
    Readonly,
}

/// One parameter of a function, method, closure or arrow function.
///
/// Every parameter's type is mandatory per ADR 0007 § 1; `ty` is `Option` only
/// so a parameter written without one still parses into a node — the missing
/// type is reported as a diagnostic at the point of parsing, not silently
/// accepted.
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    /// The whole parameter.
    pub span: Span,
    /// Promoted-property modifiers (constructor parameters only).
    pub modifiers: Vec<Modifier>,
    /// The declared type, or `None` if omitted (a diagnostic was already
    /// reported for the omission).
    pub ty: Option<Type>,
    /// Whether this parameter binds by reference (`&$x`).
    pub by_ref: bool,
    /// Whether this is a variadic parameter (`...$x`).
    pub variadic: bool,
    /// The parameter's name, `$`-sigil included.
    pub name: Span,
    /// The default value, if any.
    pub default: Option<Expr>,
}

/// One `use (...)` capture in a closure literal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClosureUse {
    /// Whether captured by reference (`&$x`).
    pub by_ref: bool,
    /// The captured variable's name, `$`-sigil included.
    pub name: Span,
}

/// `function (...) use (...): T { ... }`, optionally `static`.
#[derive(Clone, Debug, PartialEq)]
pub struct ClosureExpr {
    /// Whether declared `static` (no `$this` binding).
    pub is_static: bool,
    /// Whether the closure returns by reference.
    pub by_ref: bool,
    /// The parameter list.
    pub params: Vec<Param>,
    /// The `use (...)` capture list.
    pub uses: Vec<ClosureUse>,
    /// The declared return type, if written.
    pub return_type: Option<Type>,
    /// The closure's body.
    pub body: Block,
}

/// `fn (...): T => expr`, optionally `static`.
#[derive(Clone, Debug, PartialEq)]
pub struct ArrowFnExpr {
    /// Whether declared `static`.
    pub is_static: bool,
    /// The parameter list.
    pub params: Vec<Param>,
    /// The declared return type, if written.
    pub return_type: Option<Type>,
    /// The single expression the arrow function evaluates to.
    pub body: Box<Expr>,
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
}

/// Which of PHP's four same-frame code-inclusion keywords an
/// [`ExprKind::Include`] spells
/// ([`docs/spec/00-overview.md` § 2](../../../docs/spec/00-overview.md)). All
/// four share one AST shape — only `require`'s missing-file behaviour
/// (throwing instead of warning) and the `_once` guard differ, and neither is
/// a parse-time concern.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[expect(missing_docs, reason = "each variant is exactly its keyword spelling")]
pub enum IncludeKind {
    Include,
    IncludeOnce,
    Require,
    RequireOnce,
}

/// The key of one `spawn script … with(…)` option
/// ([`docs/spec/00-overview.md` § 2](../../../docs/spec/00-overview.md)).
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

/// Every expression form the parser produces.
///
/// Literal payloads are spans, not cooked values — see the module docs. An
/// anonymous class (`new class { ... }`) is deliberately not part of
/// [`NewTarget`] yet: it needs class-body parsing, which arrives with the
/// declarations chunk.
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
    /// A single-quoted string, or a double-quoted/heredoc/nowdoc string with
    /// no interpolation in it — both are plain literal text, uncooked.
    Str(Span),
    /// A double-quoted string or heredoc with at least one interpolation site.
    Interpolated(Vec<StringPart>),
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
    /// PHP's legacy `(T)expr` cast syntax.
    Cast {
        /// The target type.
        ty: CastType,
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
    /// ([`docs/spec/00-overview.md` § 3.4](../../../docs/spec/00-overview.md)).
    Conversion {
        /// The value being converted.
        expr: Box<Expr>,
        /// The target type.
        ty: Type,
    },
    /// `expr instanceof ClassOrExpr`
    InstanceOf {
        /// The value being tested.
        expr: Box<Expr>,
        /// The class name or expression on the right.
        class: Box<Expr>,
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
        /// The call's arguments.
        args: CallArgs,
    },
    /// `Class::method(...)`
    StaticCall {
        /// The class the method is called on.
        class: Box<Expr>,
        /// The method being called.
        method: MemberName,
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
        /// The constructor's arguments.
        args: CallArgs,
    },
    /// `clone expr`
    Clone(Box<Expr>),
    /// `function (...) { ... }`
    Closure(ClosureExpr),
    /// `fn (...) => expr`
    ArrowFn(ArrowFnExpr),
    /// `match (subject) { ... }`
    Match {
        /// The value being matched.
        subject: Box<Expr>,
        /// The arms, in source order.
        arms: Vec<MatchArm>,
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
    /// `exit`/`die`, optionally with a status/message expression.
    ExitOrDie(Option<Box<Expr>>),
    /// `spawn script path with(...)`
    /// ([`docs/spec/00-overview.md` § 2](../../../docs/spec/00-overview.md)).
    SpawnScript {
        /// The path expression, evaluated once at the spawn site.
        path: Box<Expr>,
        /// The `with(...)` options, if a `with` clause was given.
        options: Vec<SpawnOption>,
    },
    /// `include`/`include_once`/`require`/`require_once` — an expression,
    /// not a statement, per
    /// [`docs/spec/00-overview.md` § 2](../../../docs/spec/00-overview.md):
    /// `$x = include 'a.php';` is legal.
    Include {
        /// Which of the four keywords this is.
        kind: IncludeKind,
        /// The path expression.
        path: Box<Expr>,
    },
    /// A parenthesized expression, `(expr)`. Kept as its own node — rather
    /// than discarded in favour of the inner expression — only so its span
    /// covers the parentheses; it carries no other meaning.
    Paren(Box<Expr>),
    /// A placeholder produced during error recovery. Carries no meaning beyond
    /// "parsing failed here"; a diagnostic was already reported at this span.
    Error,
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

/// One binding of a `foreach` header (ADR 0007 § 3.2): a mandatory type and
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
/// (ADR 0007 § 3.3).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DestructureElement {
    /// An empty slot, `[, $b] = …` — skips one position without binding it.
    Skip,
    /// A typed leaf binding: `(key '=>')? type '&'? '$' identifier`.
    Leaf {
        /// `key =>`, if present.
        key: Option<Expr>,
        /// The declared type, or `None` if omitted (diagnostic already
        /// reported).
        ty: Option<Type>,
        /// Whether this leaf binds by reference (`&$x`).
        by_ref: bool,
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
/// diagnostic — ADR 0008 § 5 gives this construct no replacement syntax, so
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
    /// (`while ($more_work());`).
    Empty,
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
    /// `for (init; cond; step) body`. Each clause is a comma-separated list
    /// of expressions, any of which may be empty — PHP's own `for` grammar,
    /// not a new one.
    For {
        /// The initializer expressions, run once before the first iteration.
        init: Vec<Expr>,
        /// The condition expressions; only the last one's truthiness is
        /// tested, exactly as PHP evaluates a comma list here.
        cond: Vec<Expr>,
        /// The step expressions, run after each iteration.
        step: Vec<Expr>,
        /// The loop body.
        body: Box<Stmt>,
    },
    /// `foreach (subject as key? value) body`, ADR 0007 § 3.2's mandatory
    /// typed bindings. `value_by_ref` is the one place a reference marker
    /// may appear; a key binding never carries one.
    Foreach {
        /// The value being iterated.
        subject: Expr,
        /// The key binding, if the header names one.
        key: Option<ForeachBinding>,
        /// The value binding.
        value: ForeachBinding,
        /// Whether the value binds by reference (`&$v`).
        value_by_ref: bool,
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
    /// ADR 0007 § 3.1's typed local declaration:
    /// `type '$' identifier ('=' expr)? ';'`. Exactly one binding per
    /// statement — there is no comma-separated multi-declaration form.
    LocalDecl {
        /// The declared type.
        ty: Type,
        /// The declared variable's name, `$`-sigil included.
        name: Span,
        /// The initializer, if any.
        value: Option<Expr>,
    },
    /// ADR 0007 § 3.3's destructuring statement:
    /// `destructure-target '=' expr ';'`.
    Destructure {
        /// The left-hand pattern.
        target: DestructureTarget,
        /// The value being destructured.
        value: Expr,
    },
    /// `global $x, $y;` — rejected, ADR 0008 § 5. Still parses to the
    /// variables named, for a precise diagnostic.
    Global(Vec<Span>),
    /// `goto label;` — rejected: makes the control-flow graph unstructured.
    /// Still parses to the label named, for a precise diagnostic.
    Goto(Span),
    /// A function-scope `static $x (= expr)?, ...;` declaration — rejected,
    /// ADR 0008 § 5. `ty` is `Some` only for the illustrative-but-still-
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
    /// A placeholder produced during error recovery.
    Error,
}
