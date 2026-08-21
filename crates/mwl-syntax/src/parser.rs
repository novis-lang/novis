//! The recursive-descent parser: types, expressions and statements.
//!
//! # What is here so far
//!
//! The full type grammar (ADR 0007 § 3, including nested `array<T>` and DNF
//! unions/intersections) and the full expression grammar M1's plan names:
//! every operator at its PHP precedence, `as` conversion (binding tighter
//! than any binary operator per ADR 0007 § 2), `match`, closures and arrow
//! functions, generators (`yield`/`yield from`), named arguments, spread,
//! nullsafe, first-class callable syntax, `include`/`require` (an
//! expression, not a statement), and `spawn script … with(…)`
//! ([`docs/spec/00-overview.md` § 2](../../../docs/spec/00-overview.md)).
//!
//! Every control-flow statement (`if`/`elseif`/`else`, `while`, `do`/`while`,
//! `for`, `foreach` with ADR 0007 § 3.2's mandatory typed bindings, `switch`,
//! `break`/`continue`, `try`/`catch`/`finally`), `echo`, `unset`, § 3.1's
//! typed local declaration and § 3.3's destructuring statement, and the
//! statement-shaped rejects (`global`, `goto`, function-scope `static`) are
//! also here.
//!
//! M1's last chunk is also here: classes, interfaces, traits and enums
//! ([`Parser::parse_class_decl`]/[`Parser::parse_interface_decl`]/
//! [`Parser::parse_trait_decl`]/[`Parser::parse_enum_decl`]), their members
//! (properties with PHP 8.4's hooks, consts, methods, trait `use` and its
//! `insteadof`/`as` adaptations), attributes (`#[...]`,
//! [`Parser::parse_attribute_groups`]), and the file-scope declarations that
//! sit alongside them rather than being executable statements —
//! `namespace`, `use`, and the `type`-alias declaration. See
//! [`crate::ast`]'s module docs for the node shapes.
//!
//! # Backtracking
//!
//! Every production above is a single, committed pass — except two statement
//! forms whose grammar is ambiguous on a token prefix alone: a typed local
//! declaration versus an ordinary expression statement that happens to start
//! with a name (`Foo $x = ...;` versus `Foo::bar();`), and a destructuring
//! target versus a plain array-literal expression statement (`[int $a] =
//! $p;` versus `[1, 2, 3];`). Both trial-parse the more specific production
//! and [`Parser::restore`] a [`Checkpoint`] if the deciding token — a
//! `Variable` after the type, a `=` after the pattern — doesn't show up,
//! rather than a hand-written lookahead classifier that would duplicate
//! [`Parser::parse_type`]'s grammar and drift from it. See
//! [`Parser::parse_stmt_maybe_local_decl`] and
//! [`Parser::parse_stmt_maybe_destructure`].
//!
//! # Error recovery
//!
//! Every `parse_*` method always returns *something* — never a `Result`, so a
//! caller never has to decide whether to keep going. A missing token is
//! [`code::E_EXPECTED_TOKEN`] reported at the empty span where it should have
//! been, without consuming whatever actually follows; a missing expression is
//! [`code::E_EXPECTED_EXPR`] and an [`ExprKind::Error`] node. Because neither
//! consumes the offending token, that alone is not enough for any loop that
//! parses a bare sequence of items with no separator to fall back on (a
//! block's statements, a class body's members, a `switch`'s cases and each
//! case's own statement list, and similar) — those additionally force a
//! token of progress if one iteration consumed none at all, mirroring
//! [`Parser::parse_block`]'s own copy of the guard, so a malformed body
//! cannot hang the parser or grow its result vector without bound. A
//! comma-separated list (call arguments, array items, parameters, ...)
//! never needs this: it already breaks out the moment no comma follows.
//!
//! A separate concern from either of the above: how *deep* one construct can
//! nest inside another. [`Parser::enter_recursive`] bounds every kind of
//! nesting (an expression in an expression, a type in a type, a statement in
//! a statement, a destructuring target in one, a postfix chain link) with
//! one shared counter and a sticky give-up flag, so a fuzzer-shaped input —
//! thousands of nested `(`/`[`, or a long `->`/`[...]` chain — can neither
//! overflow the native call stack nor, once it gives up once, cost more than
//! one more token of work per token remaining in the file. See that method's
//! docs for the full reasoning and `PARSER_HANDOFF.md` for how a fuzz run
//! found this.

use std::collections::VecDeque;

use mwl_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};

use crate::ast::{
    AnonClassDecl, Arg, ArrayItem, ArrowFnExpr, AssignOp, Attribute, AttributeGroup, BinaryOp,
    Block, CallArgs, CastType, CatchClause, ClassDecl, ClassMember, ClassMemberKind, ClosureExpr,
    ClosureUse, ConstMember, DestructureElement, DestructureTarget, EnumCase, EnumDecl, Expr,
    ExprKind, ForeachBinding, IncDecOp, IncludeKind, InterfaceDecl, MatchArm, MemberName,
    MethodMember, Modifier, Name, NamespaceDecl, NewTarget, Param, PropertyHook, PropertyHookBody,
    PropertyHookKind, PropertyMember, SpawnOption, SpawnOptionKey, StaticVar, Stmt, StmtKind,
    StringPart, SwitchCase, TraitAdaptation, TraitAdaptationKind, TraitDecl, TraitMethodRef, Type,
    TypeAliasDecl, TypeAtom, TypeKind, UnaryOp, UseDecl, UseTraitMember, Visibility,
};
use crate::lexer::Lexer;
use crate::token::{Keyword, Token, TokenKind};

/// Produces an AST from a source file, one construct at a time.
///
/// Holds its own small lookahead buffer over the [`Lexer`] — up to three
/// tokens, for the `(` *cast-keyword* `)` lookahead that tells a cast apart
/// from a parenthesized expression.
#[derive(Debug)]
pub struct Parser<'src, 'd> {
    file: &'src SourceFile,
    lexer: Lexer<'src>,
    diags: &'d mut Diagnostics,
    lookahead: VecDeque<Token>,
    /// The span of the most recently consumed token, so a caller that just
    /// finished a sub-production can build `first.span.to(self.last_span)`
    /// without threading a running span through every helper.
    last_span: Span,
    /// Set only while parsing a `foreach` header's subject: `as` there
    /// belongs to `foreach`, not the conversion operator, so
    /// [`Self::parse_postfix`]'s loop must not consume a bare one — see
    /// [`Self::parse_expr_no_top_as`].
    suppress_as: bool,
    /// How many nested recursive-descent entries deep the parser currently
    /// is, across every kind of nesting (expressions, types, statements,
    /// destructuring targets) — one shared counter, since they all draw on
    /// the same physical call stack. See [`Self::enter_recursive`].
    depth: u32,
    /// Set once [`Self::enter_recursive`] first refuses to recurse further,
    /// and never cleared — see that method's docs for why staying tripped
    /// for the rest of this parse (rather than only refusing *this*
    /// recursion) is the part that actually keeps pathological input to
    /// bounded total work, not just bounded stack depth.
    depth_exceeded: bool,
}

/// How deep [`Parser::enter_recursive`] lets recursive-descent parsing go
/// before it bails out instead of recursing further. Thousands of nested
/// `[`/`(` (or a long `->`/`[...]` chain) in adversarial or fuzzed input
/// previously overflowed the native call stack outright — a real crash a
/// fuzz run found (see `PARSER_HANDOFF.md`).
///
/// This one counter is shared by several unrelated axes — a delimiter
/// nesting one level (an array literal, a parenthesized expression) costs
/// five increments in one pass down the precedence chain
/// (`parse_low_or`/`_assignment`/`_coalesce`/`_not`/`_unary` each guard a
/// distinct chained-operator vector and all sit on that one path), while a
/// chained `!`/`??`/`=`/cast costs one increment per repetition — so this
/// number is *not* "how many levels of legitimate nesting we allow": for
/// the delimiter case it is roughly `/5`. A real, unremarkable numerical
/// formula in the corpus this parser is validated against (a Chebyshev
/// polynomial approximation, eleven parenthesized levels deep) already
/// used close to 64 of a 64-deep budget before this was raised to 96 — so
/// treat headroom for that axis, not raw stack safety, as the binding
/// constraint when tuning this, and re-check with a corpus, not just a
/// stack-depth probe. 96 is still comfortably under the ~120 this parser's
/// debug build can survive on a 1 MiB thread stack (measured directly:
/// nested-paren inputs stop crashing somewhere between 118 and 120), and
/// was re-confirmed crash-free under `cargo fuzz`'s ASan-instrumented
/// release build too, which has larger per-frame overhead than either.
const MAX_RECURSION_DEPTH: u32 = 96;

/// A saved parser position, for the one place this parser backtracks: a
/// statement whose grammar is genuinely ambiguous on a token prefix alone
/// (a type-then-`$name` local declaration versus an ordinary expression
/// statement; a destructuring target versus a plain array literal). Trying
/// the more specific production and restoring on a mismatch is simpler and
/// far less error-prone than hand-writing a lookahead classifier that
/// duplicates the type grammar.
struct Checkpoint<'src> {
    lexer: Lexer<'src>,
    lookahead: VecDeque<Token>,
    last_span: Span,
    diags_len: usize,
}

impl<'src, 'd> Parser<'src, 'd> {
    /// Starts parsing `file`, reporting lexical and syntactic errors into
    /// `diags`.
    #[must_use]
    pub fn new(file: &'src SourceFile, diags: &'d mut Diagnostics) -> Self {
        let start = Span::at(file.id(), 0);
        Self {
            file,
            lexer: Lexer::new(file),
            diags,
            lookahead: VecDeque::new(),
            last_span: start,
            suppress_as: false,
            depth: 0,
            depth_exceeded: false,
        }
    }

    /// Guards every independent recursive-descent re-entry point (an
    /// expression nested in an expression, a type nested in a type, a
    /// statement nested in a statement, a destructuring target nested in
    /// one, a postfix chain link) against unbounded stack growth. Returns
    /// `true` once the shared depth counter — incremented on entry, always
    /// decremented by the caller before returning — has gone past
    /// [`MAX_RECURSION_DEPTH`]; the caller must then stop recursing and
    /// return an error node instead of parsing further, exactly as if it
    /// had reported any other diagnostic.
    ///
    /// The first time this trips, it also sets the sticky
    /// [`Self::depth_exceeded`] flag and returns `true` unconditionally on
    /// every call for the rest of this parse, *without* touching `depth` —
    /// so [`Self::exit_recursive`] must check the same flag before
    /// decrementing, or the two would drift out of the pairing `guarded`
    /// relies on. Bailing out once and staying bailed out (rather than only
    /// refusing to recurse *this* deep before letting the next sibling
    /// attempt its own full-depth recursion) is what keeps a single
    /// pathological run of nesting to O(depth) work instead of O(depth ×
    /// remaining input): every recursive descent function still runs on
    /// every leftover token, but sees the flag first and returns
    /// immediately, so the outer statement/block loop's own force-progress
    /// guard is what actually consumes the rest, one token at a time. A
    /// fuzz run's minimized reproducer for this — plain nested parens, no
    /// checkpointing involved at all — took over two minutes on 100,000
    /// levels before this fix; it is not merely a large constant.
    fn enter_recursive(&mut self) -> bool {
        if self.depth_exceeded {
            return true;
        }
        self.depth += 1;
        if self.depth > MAX_RECURSION_DEPTH {
            self.depth_exceeded = true;
            let span = self.peek().span.shrink_to_start();
            self.diags.report(
                Diagnostic::error(
                    code::E_TOO_DEEPLY_NESTED,
                    "parsing nested too deeply and was stopped",
                )
                .with_primary(span, "nested past the limit here")
                .with_note(format!(
                    "the recursion limit is {MAX_RECURSION_DEPTH} levels; legitimate source never \
                     comes close to it"
                )),
            );
            true
        } else {
            false
        }
    }

    fn exit_recursive(&mut self) {
        if !self.depth_exceeded {
            self.depth -= 1;
        }
    }

    /// Runs `body` under [`Self::enter_recursive`]'s guard, calling
    /// `fallback` instead once the depth limit is hit. Every self- or
    /// mutually-recursive precedence-tier function is written as a thin
    /// wrapper around this rather than checking the guard inline, so the
    /// matching [`Self::exit_recursive`] can never be missed on some
    /// early-return path the guard was added after the fact.
    fn guarded<T>(
        &mut self,
        fallback: impl FnOnce(&mut Self) -> T,
        body: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let result = if self.enter_recursive() {
            fallback(self)
        } else {
            body(self)
        };
        self.exit_recursive();
        result
    }

    fn error_expr_here(&mut self) -> Expr {
        let span = self.peek().span.shrink_to_start();
        Expr {
            span,
            kind: ExprKind::Error,
        }
    }

    /// Saves the current position, so a speculative parse can be undone by
    /// [`Self::restore`] if it turns out to be the wrong production.
    fn checkpoint(&self) -> Checkpoint<'src> {
        Checkpoint {
            lexer: self.lexer.clone(),
            lookahead: self.lookahead.clone(),
            last_span: self.last_span,
            diags_len: self.diags.len(),
        }
    }

    /// Undoes every token consumed and every diagnostic reported since
    /// `cp` was taken.
    fn restore(&mut self, cp: Checkpoint<'src>) {
        self.lexer = cp.lexer;
        self.lookahead = cp.lookahead;
        self.last_span = cp.last_span;
        self.diags.truncate(cp.diags_len);
    }

    // --- cursor -------------------------------------------------------------

    fn fill(&mut self, upto: usize) {
        while self.lookahead.len() <= upto {
            let tok = self.lexer.next_token(self.diags);
            self.lookahead.push_back(tok);
        }
    }

    fn peek(&mut self) -> Token {
        self.fill(0);
        self.lookahead[0]
    }

    fn peek_at(&mut self, n: usize) -> Token {
        self.fill(n);
        self.lookahead[n]
    }

    fn bump(&mut self) -> Token {
        self.fill(0);
        let tok = self.lookahead.pop_front().expect("just filled");
        self.last_span = tok.span;
        tok
    }

    fn push_front(&mut self, tok: Token) {
        self.lookahead.push_front(tok);
    }

    fn at(&mut self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Span> {
        if self.at(kind) {
            Some(self.bump().span)
        } else {
            None
        }
    }

    fn expect(&mut self, kind: TokenKind, what: &str) -> Span {
        self.eat(kind).unwrap_or_else(|| self.error_expected(what))
    }

    fn at_keyword(&mut self, kw: Keyword) -> bool {
        matches!(self.peek().kind, TokenKind::Keyword(k) if k == kw)
    }

    fn eat_keyword(&mut self, kw: Keyword) -> Option<Span> {
        if self.at_keyword(kw) {
            Some(self.bump().span)
        } else {
            None
        }
    }

    fn expect_keyword(&mut self, kw: Keyword, what: &str) -> Span {
        self.eat_keyword(kw)
            .unwrap_or_else(|| self.error_expected(what))
    }

    /// The lower-cased text of an identifier-shaped span, for matching a
    /// contextual keyword (`spawn`, `script`, `with`) by spelling rather than
    /// token kind — see [`crate::token`]'s module docs for why these three
    /// stay plain [`TokenKind::Ident`]s instead of reserved words.
    fn ident_text_ci(&self, span: Span) -> String {
        self.file
            .span_text(span)
            .unwrap_or_default()
            .to_ascii_lowercase()
    }

    fn at_contextual(&mut self, word: &str) -> bool {
        if !matches!(self.peek().kind, TokenKind::Ident) {
            return false;
        }
        let span = self.peek().span;
        self.ident_text_ci(span) == word
    }

    fn error_expected(&mut self, what: &str) -> Span {
        let span = self.peek().span.shrink_to_start();
        self.diags.report(
            Diagnostic::error(code::E_EXPECTED_TOKEN, format!("expected {what}"))
                .with_primary(span, "not found here"),
        );
        span
    }

    fn error_expected_expr(&mut self) -> Span {
        let span = self.peek().span.shrink_to_start();
        self.diags.report(
            Diagnostic::error(code::E_EXPECTED_EXPR, "expected an expression")
                .with_primary(span, "not found here"),
        );
        span
    }

    /// ADR 0008 § 5: `static` is not a closure modifier — a closure captures
    /// `$this` only if it uses it, so the keyword has nothing left to mean.
    /// Reports and keeps going; the caller still builds the closure node.
    fn report_static_closure_modifier(&mut self, span: Span) {
        self.diags.report(
            Diagnostic::error(
                code::E_STATIC_CLOSURE_UNSUPPORTED,
                "`static` is not a closure modifier",
            )
            .with_primary(
                span,
                "a closure already captures `$this` only if it uses it",
            )
            .with_help("drop `static` (ADR 0008 § 5)"),
        );
    }

    /// The `Core` replacement ADR 0012 § 8 names for a PHP superglobal, or
    /// `None` if `name` (the raw `$…` text) is not one.
    fn superglobal_replacement(name: &str) -> Option<&'static str> {
        Some(match name {
            "$GLOBALS" => {
                "`$GLOBALS` does not exist; declare a `static` property, a constant, or pass the \
                 value as a parameter"
            }
            "$_REQUEST" => {
                "`$_REQUEST` does not exist; read `Core\\Request::query()`, `::post()` or \
                 `::cookie()` explicitly, so the source is visible at the call site"
            }
            "$_GET" | "$_POST" | "$_COOKIE" | "$_FILES" => "use `Core\\Request`",
            "$_SERVER" => "use `Core\\Server`",
            "$_SESSION" => "use `Core\\Session`, after calling `Core\\Session::start()`",
            "$_ENV" => "use `Core\\Env`",
            "$argv" | "$argc" => "use `Core\\Cli`",
            "$_ARGS" => "use `Core\\Script::args()`",
            _ => return None,
        })
    }

    /// ADR 0012: no variable is ever populated by the host. Reports and keeps
    /// going — the caller still builds the ordinary `Variable` node.
    fn check_superglobal(&mut self, span: Span) {
        let name = self.file.span_text(span).unwrap_or_default();
        if let Some(replacement) = Self::superglobal_replacement(name) {
            self.diags.report(
                Diagnostic::error(
                    code::E_SUPERGLOBAL_UNSUPPORTED,
                    format!("`{name}` is not supported"),
                )
                .with_primary(span, "no variable is ever populated by the host")
                .with_help(replacement),
            );
        }
    }

    // ========================================================================
    // Types (ADR 0007 § 3)
    // ========================================================================

    /// Whether the current token could begin a type expression — used to
    /// decide, without committing, whether a mandatory type was actually
    /// omitted (e.g. a parameter written without one).
    fn can_start_type(&mut self) -> bool {
        Self::token_starts_type(self.peek().kind)
    }

    /// The token-kind half of [`Self::can_start_type`], factored out so a
    /// statement-position lookahead ([`Self::at_function_scope_static`]) can
    /// ask the same question one token further ahead without a second,
    /// drifting copy of this list.
    fn token_starts_type(kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Keyword(
                Keyword::Null
                    | Keyword::Bool
                    | Keyword::Int
                    | Keyword::Uint
                    | Keyword::Float
                    | Keyword::String
                    | Keyword::Bytes
                    | Keyword::Array
                    | Keyword::Object
                    | Keyword::Mixed
                    | Keyword::Void
                    | Keyword::Never
                    | Keyword::True
                    | Keyword::False
                    | Keyword::Iterable
                    | Keyword::Callable
                    | Keyword::SelfKw
                    | Keyword::Static
                    | Keyword::Parent
            ) | TokenKind::Ident
                | TokenKind::Backslash
                | TokenKind::Question
                | TokenKind::LParen
        )
    }

    /// Whether `static` at the current position starts the rejected
    /// function-scope storage declaration (ADR 0008 § 5) rather than an
    /// ordinary `static::`/`static function`/`static fn`/bare-`static`
    /// expression — decided by one token of lookahead past `static` itself:
    /// a `$name` (the ordinary untyped PHP spelling) or a type-start token
    /// (the ADR's own illustrative typed spelling) both mean this is the
    /// rejected construct.
    fn at_function_scope_static(&mut self) -> bool {
        matches!(self.peek_at(1).kind, TokenKind::Variable)
            || Self::token_starts_type(self.peek_at(1).kind)
    }

    /// Parses one type expression: `union`.
    pub(crate) fn parse_type(&mut self) -> Type {
        self.parse_type_union()
    }

    fn parse_type_union(&mut self) -> Type {
        let first = self.parse_type_intersection();
        if !self.at(TokenKind::Pipe) {
            return first;
        }
        let mut items = vec![first];
        while self.eat(TokenKind::Pipe).is_some() {
            items.push(self.parse_type_intersection());
        }
        let span = items[0].span.to(items[items.len() - 1].span);
        Type {
            kind: TypeKind::Union(items),
            span,
        }
    }

    fn parse_type_intersection(&mut self) -> Type {
        let first = self.parse_type_operand();
        if !self.at_intersection_amp() {
            return first;
        }
        let mut items = vec![first];
        while self.at_intersection_amp() {
            self.bump();
            items.push(self.parse_type_operand());
        }
        let span = items[0].span.to(items[items.len() - 1].span);
        Type {
            kind: TypeKind::Intersection(items),
            span,
        }
    }

    /// Whether an `&` here continues an intersection type (`A&B`) rather
    /// than being a by-reference marker that happens to follow a type with
    /// nothing between them (`int &$x` — a parameter, a `foreach` value
    /// binding, a destructuring leaf). Both shapes start identically; the
    /// deciding token is one further ahead: an intersection member is always
    /// another type atom, never a bare `$name`.
    fn at_intersection_amp(&mut self) -> bool {
        self.at(TokenKind::Amp) && Self::token_starts_type(self.peek_at(1).kind)
    }

    /// Self-recursive both for a nested `?` (`??int`, absurd but legal
    /// grammar) and via [`Self::parse_type_union`] for a parenthesized type
    /// (`((((int))))`) — needs the same recursion guard as
    /// [`Self::parse_assignment`], and guarding it alone is enough to bound
    /// the paren cycle too, since `parse_type_union`/`_intersection` always
    /// lead straight back here with no branching in between.
    fn parse_type_operand(&mut self) -> Type {
        self.guarded(Self::error_type_here, Self::parse_type_operand_inner)
    }

    fn error_type_here(&mut self) -> Type {
        let span = self.peek().span.shrink_to_start();
        Type {
            kind: TypeKind::Atom(TypeAtom::Mixed),
            span,
        }
    }

    fn parse_type_operand_inner(&mut self) -> Type {
        if let Some(q) = self.eat(TokenKind::Question) {
            let inner = self.parse_type_operand();
            let span = q.to(inner.span);
            return Type {
                kind: TypeKind::Nullable(Box::new(inner)),
                span,
            };
        }
        if let Some(lp) = self.eat(TokenKind::LParen) {
            let inner = self.parse_type_union();
            let rp = self.expect(TokenKind::RParen, "`)`");
            let span = lp.to(rp);
            return Type {
                kind: TypeKind::Paren(Box::new(inner)),
                span,
            };
        }
        self.parse_type_atom()
    }

    /// Closes an `array<...>` type argument. `>` is the ordinary case; `>>`,
    /// `>=` and `>>=` are split in place, so `array<array<uint>>` closes
    /// correctly even though the lexer already committed to the longer
    /// operator — `array<T>` is parsed only in type position precisely so
    /// this is the one place that ambiguity has to be resolved (ADR 0007 § 3).
    fn expect_type_close_angle(&mut self) -> Span {
        let (first_len, rest_kind): (u32, TokenKind) = match self.peek().kind {
            TokenKind::Gt => return self.bump().span,
            TokenKind::GtEquals => (1, TokenKind::Equals),
            TokenKind::GtGt => (1, TokenKind::Gt),
            TokenKind::GtGtEquals => (1, TokenKind::GtEquals),
            _ => return self.expect(TokenKind::Gt, "`>`"),
        };
        let tok = self.bump();
        let mid = tok.span.start + first_len;
        let first = Span::new(tok.span.file, tok.span.start, mid);
        let rest = Span::new(tok.span.file, mid, tok.span.end);
        self.push_front(Token::new(rest_kind, rest));
        first
    }

    fn parse_type_atom(&mut self) -> Type {
        let tok = self.peek();
        let start = tok.span;
        macro_rules! atom {
            ($variant:ident) => {{
                self.bump();
                Type {
                    kind: TypeKind::Atom(TypeAtom::$variant),
                    span: start,
                }
            }};
        }
        match tok.kind {
            TokenKind::Keyword(Keyword::Null) => atom!(Null),
            TokenKind::Keyword(Keyword::Bool) => atom!(Bool),
            TokenKind::Keyword(Keyword::Int) => atom!(Int),
            TokenKind::Keyword(Keyword::Uint) => atom!(Uint),
            TokenKind::Keyword(Keyword::Float) => atom!(Float),
            TokenKind::Keyword(Keyword::String) => atom!(String),
            TokenKind::Keyword(Keyword::Bytes) => atom!(Bytes),
            TokenKind::Keyword(Keyword::Object) => atom!(Object),
            TokenKind::Keyword(Keyword::Mixed) => atom!(Mixed),
            TokenKind::Keyword(Keyword::Void) => atom!(Void),
            TokenKind::Keyword(Keyword::Never) => atom!(Never),
            TokenKind::Keyword(Keyword::True) => atom!(True),
            TokenKind::Keyword(Keyword::False) => atom!(False),
            TokenKind::Keyword(Keyword::Iterable) => atom!(Iterable),
            TokenKind::Keyword(Keyword::Callable) => atom!(Callable),
            TokenKind::Keyword(Keyword::SelfKw) => atom!(SelfTy),
            TokenKind::Keyword(Keyword::Static) => atom!(StaticTy),
            TokenKind::Keyword(Keyword::Parent) => atom!(Parent),
            TokenKind::Keyword(Keyword::Array) => {
                self.bump();
                if self.eat(TokenKind::Lt).is_some() {
                    let inner = self.parse_type_union();
                    let close = self.expect_type_close_angle();
                    Type {
                        kind: TypeKind::Atom(TypeAtom::Array(Some(Box::new(inner)))),
                        span: start.to(close),
                    }
                } else {
                    Type {
                        kind: TypeKind::Atom(TypeAtom::Array(None)),
                        span: start,
                    }
                }
            }
            TokenKind::Ident | TokenKind::Backslash => {
                let name = self.parse_name();
                Type {
                    kind: TypeKind::Atom(TypeAtom::Name(name)),
                    span: name.span,
                }
            }
            _ => {
                let span = self.error_expected("a type");
                Type {
                    kind: TypeKind::Atom(TypeAtom::Mixed),
                    span,
                }
            }
        }
    }

    /// A possibly-namespace-qualified name: `Foo`, `Core\Bytes`,
    /// `\Fully\Qualified`.
    ///
    /// A segment may be a reserved word's spelling — `Core\Bytes` collides
    /// with the `bytes` type keyword under the lexer's case-insensitive
    /// keyword matching, exactly the way `Core\Bytes::fromHex` already needs
    /// to parse per
    /// [`docs/spec/00-overview.md` § 5](../../../docs/spec/00-overview.md).
    /// Once a name is expected at all (this is only ever called after an
    /// `Ident` or `Backslash` was already seen), a keyword spelling here is
    /// unambiguous — the same reasoning already applied to a member name
    /// after `->`/`::` in [`Self::parse_member_name`].
    fn parse_name(&mut self) -> Name {
        let start = self.peek().span;
        self.eat(TokenKind::Backslash);
        let mut last = self.expect_name_segment();
        while self.at(TokenKind::Backslash) && Self::is_name_segment(self.peek_at(1).kind) {
            self.bump();
            last = self.bump().span;
        }
        Name {
            span: start.to(last),
        }
    }

    fn is_name_segment(kind: TokenKind) -> bool {
        matches!(kind, TokenKind::Ident | TokenKind::Keyword(_))
    }

    fn expect_name_segment(&mut self) -> Span {
        if Self::is_name_segment(self.peek().kind) {
            self.bump().span
        } else {
            self.error_expected("a name")
        }
    }

    // ========================================================================
    // Expressions — precedence chain, lowest to highest
    // ========================================================================

    /// Parses one expression, from the lowest-precedence `or`/`xor`/`and`
    /// keywords down through assignment, the ternary, and every binary and
    /// unary operator, to a primary expression. Every other production that
    /// needs "an expression" calls this.
    #[must_use]
    pub fn parse_expr(&mut self) -> Expr {
        // Any nested expression (a call argument, an array item, a
        // parenthesized group, ...) is unambiguous again once its own
        // delimiters bound it, regardless of whether an enclosing `foreach`
        // header suppressed `as` for the expression it is nested inside —
        // see `parse_expr_no_top_as`.
        let prev = self.suppress_as;
        self.suppress_as = false;
        let e = self.parse_low_or();
        self.suppress_as = prev;
        e
    }

    /// Parses a `foreach` header's subject. Identical to [`Self::parse_expr`]
    /// except that [`Self::parse_postfix`]'s loop will not consume a bare
    /// `as` — that keyword belongs to `foreach` itself
    /// ([`docs/spec/00-overview.md` § 3.2](../../../docs/spec/00-overview.md)).
    /// Converting the subject still works, just parenthesized:
    /// `foreach (($m as array<int>) as int $v)` — the parens start a fresh
    /// [`Self::parse_expr`] call, which lifts the suppression for its own
    /// duration.
    fn parse_expr_no_top_as(&mut self) -> Expr {
        let prev = self.suppress_as;
        self.suppress_as = true;
        let e = self.parse_low_or();
        self.suppress_as = prev;
        e
    }

    fn parse_left_assoc(
        &mut self,
        next: fn(&mut Self) -> Expr,
        ops: &[(TokenKind, BinaryOp)],
    ) -> Expr {
        let mut lhs = next(self);
        loop {
            let kind = self.peek().kind;
            let Some(&(_, op)) = ops.iter().find(|(tk, _)| *tk == kind) else {
                break;
            };
            self.bump();
            let rhs = next(self);
            let span = lhs.span.to(rhs.span);
            lhs = Expr {
                span,
                kind: ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            };
        }
        lhs
    }

    /// The shared re-entry point for every nested expression (an array
    /// item, a call argument, a parenthesized group, ...), so guarding it
    /// alone would catch that whole class of unbounded nesting — but not a
    /// long flat chain of one repeated prefix/infix operator, which
    /// recurses through [`Self::parse_assignment`]/[`Self::parse_ternary`]/
    /// [`Self::parse_not`]/[`Self::parse_unary`]/[`Self::parse_power`]
    /// without ever coming back through here. Each of those is guarded
    /// individually for that reason.
    fn parse_low_or(&mut self) -> Expr {
        self.guarded(Self::error_expr_here, |p| {
            p.parse_left_assoc(
                Self::parse_low_xor,
                &[(TokenKind::Keyword(Keyword::Or), BinaryOp::LowOr)],
            )
        })
    }

    fn parse_low_xor(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_low_and,
            &[(TokenKind::Keyword(Keyword::Xor), BinaryOp::LowXor)],
        )
    }

    fn parse_low_and(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_assignment,
            &[(TokenKind::Keyword(Keyword::And), BinaryOp::LowAnd)],
        )
    }

    /// Right-recursive on its own operand (`$a = $b = $c = ...`), so a long
    /// chain of `=` needs the same recursion guard as the array-literal
    /// nesting that originally motivated it — see [`Self::guarded`].
    fn parse_assignment(&mut self) -> Expr {
        self.guarded(Self::error_expr_here, Self::parse_assignment_inner)
    }

    fn parse_assignment_inner(&mut self) -> Expr {
        let target = self.parse_ternary();
        let op = match self.peek().kind {
            TokenKind::Equals => AssignOp::Assign,
            TokenKind::PlusEquals => AssignOp::AddAssign,
            TokenKind::MinusEquals => AssignOp::SubAssign,
            TokenKind::StarEquals => AssignOp::MulAssign,
            TokenKind::SlashEquals => AssignOp::DivAssign,
            TokenKind::PercentEquals => AssignOp::ModAssign,
            TokenKind::StarStarEquals => AssignOp::PowAssign,
            TokenKind::DotEquals => AssignOp::ConcatAssign,
            TokenKind::AmpEquals => AssignOp::BitAndAssign,
            TokenKind::PipeEquals => AssignOp::BitOrAssign,
            TokenKind::CaretEquals => AssignOp::BitXorAssign,
            TokenKind::LtLtEquals => AssignOp::ShlAssign,
            TokenKind::GtGtEquals => AssignOp::ShrAssign,
            TokenKind::QuestionQuestionEquals => AssignOp::CoalesceAssign,
            _ => return target,
        };
        self.bump();
        if !is_assignable(&target) {
            self.diags.report(
                Diagnostic::error(
                    code::E_INVALID_ASSIGN_TARGET,
                    "this expression cannot be assigned to",
                )
                .with_primary(target.span, "not a valid assignment target"),
            );
        }
        // `target = &value` binds by reference; PHP has no `&`-form of a
        // compound operator (`+=&` is not a thing), so this only applies to
        // plain `=`.
        let by_ref = op == AssignOp::Assign && self.eat(TokenKind::Amp).is_some();
        let value = self.parse_assignment();
        let span = target.span.to(value.span);
        Expr {
            span,
            kind: ExprKind::Assign {
                op,
                target: Box::new(target),
                value: Box::new(value),
                by_ref,
            },
        }
    }

    fn parse_ternary(&mut self) -> Expr {
        let cond = self.parse_coalesce();
        if self.eat(TokenKind::Question).is_none() {
            return cond;
        }
        if self.eat(TokenKind::Colon).is_some() {
            let else_ = self.parse_assignment();
            let span = cond.span.to(else_.span);
            return Expr {
                span,
                kind: ExprKind::Ternary {
                    cond: Box::new(cond),
                    then: None,
                    else_: Box::new(else_),
                },
            };
        }
        let then = self.parse_assignment();
        self.expect(TokenKind::Colon, "`:`");
        let else_ = self.parse_assignment();
        let span = cond.span.to(else_.span);
        Expr {
            span,
            kind: ExprKind::Ternary {
                cond: Box::new(cond),
                then: Some(Box::new(then)),
                else_: Box::new(else_),
            },
        }
    }

    /// Right-recursive on its own operand (`$a ?? $b ?? $c ?? ...`) — needs
    /// its own guard for the same reason [`Self::parse_assignment`] does;
    /// nothing else in the chain passes back through here.
    fn parse_coalesce(&mut self) -> Expr {
        self.guarded(Self::error_expr_here, Self::parse_coalesce_inner)
    }

    fn parse_coalesce_inner(&mut self) -> Expr {
        let lhs = self.parse_logic_or();
        if self.eat(TokenKind::QuestionQuestion).is_some() {
            let rhs = self.parse_coalesce();
            let span = lhs.span.to(rhs.span);
            return Expr {
                span,
                kind: ExprKind::Binary {
                    op: BinaryOp::Coalesce,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            };
        }
        lhs
    }

    fn parse_logic_or(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_logic_and,
            &[(TokenKind::PipePipe, BinaryOp::Or)],
        )
    }

    fn parse_logic_and(&mut self) -> Expr {
        self.parse_left_assoc(Self::parse_bit_or, &[(TokenKind::AmpAmp, BinaryOp::And)])
    }

    fn parse_bit_or(&mut self) -> Expr {
        self.parse_left_assoc(Self::parse_bit_xor, &[(TokenKind::Pipe, BinaryOp::BitOr)])
    }

    fn parse_bit_xor(&mut self) -> Expr {
        self.parse_left_assoc(Self::parse_bit_and, &[(TokenKind::Caret, BinaryOp::BitXor)])
    }

    fn parse_bit_and(&mut self) -> Expr {
        self.parse_left_assoc(Self::parse_equality, &[(TokenKind::Amp, BinaryOp::BitAnd)])
    }

    fn parse_equality(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_relational,
            &[
                (TokenKind::EqualsEquals, BinaryOp::Eq),
                (TokenKind::BangEquals, BinaryOp::NotEq),
                (TokenKind::EqualsEqualsEquals, BinaryOp::Identical),
                (TokenKind::BangEqualsEquals, BinaryOp::NotIdentical),
                (TokenKind::Spaceship, BinaryOp::Cmp),
            ],
        )
    }

    fn parse_relational(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_concat,
            &[
                (TokenKind::Lt, BinaryOp::Lt),
                (TokenKind::LtEquals, BinaryOp::LtEq),
                (TokenKind::Gt, BinaryOp::Gt),
                (TokenKind::GtEquals, BinaryOp::GtEq),
            ],
        )
    }

    fn parse_concat(&mut self) -> Expr {
        self.parse_left_assoc(Self::parse_shift, &[(TokenKind::Dot, BinaryOp::Concat)])
    }

    fn parse_shift(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_additive,
            &[
                (TokenKind::LtLt, BinaryOp::Shl),
                (TokenKind::GtGt, BinaryOp::Shr),
            ],
        )
    }

    fn parse_additive(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_multiplicative,
            &[
                (TokenKind::Plus, BinaryOp::Add),
                (TokenKind::Minus, BinaryOp::Sub),
            ],
        )
    }

    fn parse_multiplicative(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_not,
            &[
                (TokenKind::Star, BinaryOp::Mul),
                (TokenKind::Slash, BinaryOp::Div),
                (TokenKind::Percent, BinaryOp::Mod),
            ],
        )
    }

    /// `!expr` — stacks (`!!expr` is `!(!expr)`), and otherwise defers to
    /// `instanceof`, which binds tighter. The stacking is unbounded self-
    /// recursion (`!!!!!!...`), so it needs the same guard as
    /// [`Self::parse_assignment`].
    fn parse_not(&mut self) -> Expr {
        self.guarded(Self::error_expr_here, Self::parse_not_inner)
    }

    fn parse_not_inner(&mut self) -> Expr {
        if let Some(start) = self.eat(TokenKind::Bang) {
            let expr = self.parse_not();
            let span = start.to(expr.span);
            return Expr {
                span,
                kind: ExprKind::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                },
            };
        }
        self.parse_instanceof()
    }

    fn parse_instanceof(&mut self) -> Expr {
        let mut lhs = self.parse_unary();
        while self.eat_keyword(Keyword::InstanceOf).is_some() {
            let class = self.parse_unary();
            let span = lhs.span.to(class.span);
            lhs = Expr {
                span,
                kind: ExprKind::InstanceOf {
                    expr: Box::new(lhs),
                    class: Box::new(class),
                },
            };
        }
        lhs
    }

    /// Whether `(` at the current position opens a legacy `(T)expr` cast
    /// rather than a parenthesized expression — decided by a 3-token
    /// lookahead, since both start identically.
    fn peek_cast_type(&mut self) -> Option<CastType> {
        let ty = match self.peek_at(1).kind {
            TokenKind::Keyword(Keyword::Int) => CastType::Int,
            TokenKind::Keyword(Keyword::Uint) => CastType::Uint,
            TokenKind::Keyword(Keyword::Float) => CastType::Float,
            TokenKind::Keyword(Keyword::String) => CastType::String,
            TokenKind::Keyword(Keyword::Bool) => CastType::Bool,
            TokenKind::Keyword(Keyword::Array) => CastType::Array,
            TokenKind::Keyword(Keyword::Object) => CastType::Object,
            _ => return None,
        };
        (self.peek_at(2).kind == TokenKind::RParen).then_some(ty)
    }

    /// Self-recursive on its own operand for every prefix form it handles
    /// (a cast, `-`/`+`/`~`/`@`/`!`, `++`/`--`), and mutually recursive with
    /// [`Self::parse_power`] for a chain of `**` — needs the same guard as
    /// [`Self::parse_assignment`], and guarding it alone is enough to bound
    /// that `**` cycle too, since every trip around it passes back through
    /// here.
    fn parse_unary(&mut self) -> Expr {
        self.guarded(Self::error_expr_here, Self::parse_unary_inner)
    }

    fn parse_unary_inner(&mut self) -> Expr {
        if self.at(TokenKind::LParen)
            && let Some(ty) = self.peek_cast_type()
        {
            let start = self.bump().span; // '('
            self.bump(); // the cast keyword
            self.bump(); // ')'
            let expr = self.parse_unary();
            let span = start.to(expr.span);
            return Expr {
                span,
                kind: ExprKind::Cast {
                    ty,
                    expr: Box::new(expr),
                },
            };
        }
        macro_rules! prefix {
            ($op:ident) => {{
                let start = self.bump().span;
                let expr = self.parse_unary();
                let span = start.to(expr.span);
                return Expr {
                    span,
                    kind: ExprKind::Unary {
                        op: UnaryOp::$op,
                        expr: Box::new(expr),
                    },
                };
            }};
        }
        match self.peek().kind {
            TokenKind::Minus => prefix!(Neg),
            TokenKind::Plus => prefix!(Plus),
            TokenKind::Tilde => prefix!(BitNot),
            TokenKind::At => prefix!(Suppress),
            // `!` normally binds looser than a cast/unary op ([`Self::parse_not`]
            // is the tier that reaches it first in the ordinary chain), but a
            // cast or another unary op recurses straight into this function for
            // its own operand, skipping past `parse_not` entirely — so without
            // this arm, `(int) !$x` or `-!$x` would find no expression to parse
            // at all rather than nesting the way real PHP accepts.
            TokenKind::Bang => prefix!(Not),
            TokenKind::PlusPlus => {
                let start = self.bump().span;
                let expr = self.parse_unary();
                let span = start.to(expr.span);
                Expr {
                    span,
                    kind: ExprKind::PreIncDec {
                        op: IncDecOp::Inc,
                        expr: Box::new(expr),
                    },
                }
            }
            TokenKind::MinusMinus => {
                let start = self.bump().span;
                let expr = self.parse_unary();
                let span = start.to(expr.span);
                Expr {
                    span,
                    kind: ExprKind::PreIncDec {
                        op: IncDecOp::Dec,
                        expr: Box::new(expr),
                    },
                }
            }
            _ => self.parse_power(),
        }
    }

    /// `**`, right-associative: the right operand re-enters at the unary
    /// tier, so `2 ** -2` and chained `a ** b ** c` both come out right.
    fn parse_power(&mut self) -> Expr {
        let lhs = self.parse_postfix();
        if self.eat(TokenKind::StarStar).is_some() {
            let rhs = self.parse_unary();
            let span = lhs.span.to(rhs.span);
            return Expr {
                span,
                kind: ExprKind::Binary {
                    op: BinaryOp::Pow,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            };
        }
        lhs
    }

    // ========================================================================
    // Postfix chain: member access, calls, indexing, `as`, `++`/`--`
    // ========================================================================

    /// Chains `->`/`?->`/`::`/`[...]`/`(...)`/`++`/`--`/`as` onto a primary
    /// expression in a loop, not recursion — so unlike every guarded
    /// function above, an unbounded chain here (`$x[0][0][0]...` thousands
    /// deep) never risks overflowing *this* function's own stack. It still
    /// builds a `Box`-nested `Expr` exactly that deep, though, and *that*
    /// structure is what a fuzz run actually found overflowing the stack —
    /// not here, but the first ordinary recursive walk over it afterwards
    /// (originally `{:#?}` while investigating a different crash; name
    /// resolution, type checking and codegen will all walk it the same way
    /// once they exist). Shares [`Self::enter_recursive`]'s counter and
    /// sticky give-up flag: one link consumed here counts the same as one
    /// level of expression nesting elsewhere, so `[$a[0][0]...[0]]` and
    /// `[[[...]]]` draw on the same bounded budget instead of two separate
    /// ones that would each individually look safe. `chain_len` tracks how
    /// many times *this* loop has incremented that counter, purely so it
    /// can give back exactly that many increments before returning — this
    /// loop does not recurse, so nothing else will.
    fn parse_postfix(&mut self) -> Expr {
        let mut e = self.parse_primary();
        let mut chain_len: u32 = 0;
        loop {
            if self.enter_recursive() {
                e = Expr {
                    span: e.span,
                    kind: ExprKind::Error,
                };
                break;
            }
            chain_len += 1;
            match self.peek().kind {
                TokenKind::Arrow | TokenKind::NullsafeArrow => {
                    let nullsafe = matches!(self.peek().kind, TokenKind::NullsafeArrow);
                    self.bump();
                    let member = self.parse_member_name();
                    e = if self.at(TokenKind::LParen) {
                        let args = self.parse_call_args();
                        let span = e.span.to(self.last_span);
                        Expr {
                            span,
                            kind: ExprKind::MethodCall {
                                object: Box::new(e),
                                nullsafe,
                                method: member,
                                args,
                            },
                        }
                    } else {
                        let span = e.span.to(self.last_span);
                        Expr {
                            span,
                            kind: ExprKind::PropertyAccess {
                                object: Box::new(e),
                                nullsafe,
                                property: member,
                            },
                        }
                    };
                }
                TokenKind::DoubleColon => {
                    self.bump();
                    e = self.parse_after_double_colon(e);
                }
                TokenKind::LBracket => {
                    self.bump();
                    let index = if self.at(TokenKind::RBracket) {
                        None
                    } else {
                        Some(Box::new(self.parse_expr()))
                    };
                    self.expect(TokenKind::RBracket, "`]`");
                    let span = e.span.to(self.last_span);
                    e = Expr {
                        span,
                        kind: ExprKind::Index {
                            base: Box::new(e),
                            index,
                        },
                    };
                }
                TokenKind::LParen => {
                    let args = self.parse_call_args();
                    let span = e.span.to(self.last_span);
                    e = Expr {
                        span,
                        kind: ExprKind::Call {
                            callee: Box::new(e),
                            args,
                        },
                    };
                }
                TokenKind::PlusPlus => {
                    self.bump();
                    let span = e.span.to(self.last_span);
                    e = Expr {
                        span,
                        kind: ExprKind::PostIncDec {
                            op: IncDecOp::Inc,
                            expr: Box::new(e),
                        },
                    };
                }
                TokenKind::MinusMinus => {
                    self.bump();
                    let span = e.span.to(self.last_span);
                    e = Expr {
                        span,
                        kind: ExprKind::PostIncDec {
                            op: IncDecOp::Dec,
                            expr: Box::new(e),
                        },
                    };
                }
                TokenKind::Keyword(Keyword::As) if !self.suppress_as => {
                    self.bump();
                    let ty = self.parse_type();
                    let span = e.span.to(ty.span);
                    e = Expr {
                        span,
                        kind: ExprKind::Conversion {
                            expr: Box::new(e),
                            ty,
                        },
                    };
                }
                _ => {
                    // No postfix operator here after all — this iteration's
                    // increment doesn't correspond to a real link, so give
                    // it straight back rather than counting it.
                    self.exit_recursive();
                    chain_len -= 1;
                    break;
                }
            }
        }
        for _ in 0..chain_len {
            self.exit_recursive();
        }
        e
    }

    fn parse_after_double_colon(&mut self, class: Expr) -> Expr {
        match self.peek().kind {
            TokenKind::Keyword(Keyword::Class) => {
                self.bump();
                let span = class.span.to(self.last_span);
                Expr {
                    span,
                    kind: ExprKind::ClassNameConst {
                        class: Box::new(class),
                    },
                }
            }
            TokenKind::Variable => {
                let name = self.bump().span;
                let span = class.span.to(name);
                Expr {
                    span,
                    kind: ExprKind::StaticPropertyAccess {
                        class: Box::new(class),
                        name,
                    },
                }
            }
            _ => {
                let member = self.parse_member_name();
                if self.at(TokenKind::LParen) {
                    let args = self.parse_call_args();
                    let span = class.span.to(self.last_span);
                    Expr {
                        span,
                        kind: ExprKind::StaticCall {
                            class: Box::new(class),
                            method: member,
                            args,
                        },
                    }
                } else if let MemberName::Ident(name) = member {
                    let span = class.span.to(name);
                    Expr {
                        span,
                        kind: ExprKind::ClassConstAccess {
                            class: Box::new(class),
                            name,
                        },
                    }
                } else {
                    let span = class.span.to(self.last_span);
                    self.diags.report(
                        Diagnostic::error(
                            code::E_EXPECTED_TOKEN,
                            "expected `(` to call a dynamic static member",
                        )
                        .with_primary(span.shrink_to_end(), "not found here"),
                    );
                    Expr {
                        span,
                        kind: ExprKind::Error,
                    }
                }
            }
        }
    }

    /// The name on the right of `->`/`?->`/`::`: an ordinary identifier (any
    /// keyword spelling accepted too, matching PHP's own allowance of
    /// keyword-named methods), `$name` (a dynamic member name) or `{expr}`
    /// (a fully computed one).
    fn parse_member_name(&mut self) -> MemberName {
        match self.peek().kind {
            TokenKind::Variable => {
                let span = self.bump().span;
                MemberName::Variable(Box::new(Expr {
                    span,
                    kind: ExprKind::Variable(span),
                }))
            }
            TokenKind::LBrace => {
                self.bump();
                let inner = self.parse_expr();
                self.expect(TokenKind::RBrace, "`}`");
                MemberName::Expr(Box::new(inner))
            }
            TokenKind::Ident | TokenKind::Keyword(_) => MemberName::Ident(self.bump().span),
            _ => MemberName::Ident(self.error_expected("a member name")),
        }
    }

    fn parse_call_args(&mut self) -> CallArgs {
        self.expect(TokenKind::LParen, "`(`");
        if self.at(TokenKind::Ellipsis) && self.peek_at(1).kind == TokenKind::RParen {
            self.bump();
            self.bump();
            return CallArgs::FirstClassCallable;
        }
        let mut args = Vec::new();
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            args.push(self.parse_arg());
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::RParen, "`)`");
        CallArgs::List(args)
    }

    fn parse_arg(&mut self) -> Arg {
        let start = self.peek().span;
        if self.eat(TokenKind::Ellipsis).is_some() {
            let value = self.parse_expr();
            let span = start.to(value.span);
            return Arg {
                name: None,
                spread: true,
                value,
                span,
            };
        }
        if matches!(self.peek().kind, TokenKind::Ident) && self.peek_at(1).kind == TokenKind::Colon
        {
            let name = self.bump().span;
            self.bump(); // ':'
            let value = self.parse_expr();
            let span = start.to(value.span);
            return Arg {
                name: Some(name),
                spread: false,
                value,
                span,
            };
        }
        let value = self.parse_expr();
        let span = start.to(value.span);
        Arg {
            name: None,
            spread: false,
            value,
            span,
        }
    }

    // ========================================================================
    // Primary expressions
    // ========================================================================

    fn parse_primary(&mut self) -> Expr {
        let start = self.peek().span;
        match self.peek().kind {
            TokenKind::Keyword(Keyword::Null) => {
                self.bump();
                Expr {
                    span: start,
                    kind: ExprKind::Null,
                }
            }
            TokenKind::Keyword(Keyword::True) => {
                self.bump();
                Expr {
                    span: start,
                    kind: ExprKind::Bool(true),
                }
            }
            TokenKind::Keyword(Keyword::False) => {
                self.bump();
                Expr {
                    span: start,
                    kind: ExprKind::Bool(false),
                }
            }
            TokenKind::IntLiteral => {
                self.bump();
                Expr {
                    span: start,
                    kind: ExprKind::Int(start),
                }
            }
            TokenKind::FloatLiteral => {
                self.bump();
                Expr {
                    span: start,
                    kind: ExprKind::Float(start),
                }
            }
            TokenKind::SingleQuotedString => {
                self.bump();
                Expr {
                    span: start,
                    kind: ExprKind::Str(start),
                }
            }
            TokenKind::DoubleQuoteOpen => self.parse_double_quoted_string(),
            TokenKind::HeredocOpen | TokenKind::NowdocOpen => self.parse_heredoc_string(),
            TokenKind::Variable => {
                self.bump();
                self.check_superglobal(start);
                Expr {
                    span: start,
                    kind: ExprKind::Variable(start),
                }
            }
            TokenKind::Dollar => {
                self.bump();
                let end = match self.peek().kind {
                    TokenKind::Variable => self.bump().span,
                    TokenKind::LBrace => {
                        self.bump();
                        let _ = self.parse_expr();
                        self.expect(TokenKind::RBrace, "`}`")
                    }
                    _ => start,
                };
                let span = start.to(end);
                self.diags.report(
                    Diagnostic::error(
                        code::E_VARIABLE_VARIABLE,
                        "variable variables (`$$name` / `${expr}`) are not supported",
                    )
                    .with_primary(span, "defeats name resolution and type inference")
                    .with_help("use an explicit `array<string, T>` keyed by name instead"),
                );
                Expr {
                    span,
                    kind: ExprKind::Error,
                }
            }
            TokenKind::Keyword(Keyword::SelfKw) => {
                self.bump();
                Expr {
                    span: start,
                    kind: ExprKind::SelfExpr,
                }
            }
            TokenKind::Keyword(Keyword::Parent) => {
                self.bump();
                Expr {
                    span: start,
                    kind: ExprKind::ParentExpr,
                }
            }
            TokenKind::Keyword(Keyword::Static) => match self.peek_at(1).kind {
                TokenKind::Keyword(Keyword::Function) => self.parse_closure(true),
                TokenKind::Keyword(Keyword::Fn) => self.parse_arrow_fn(true),
                _ => {
                    self.bump();
                    Expr {
                        span: start,
                        kind: ExprKind::StaticExpr,
                    }
                }
            },
            TokenKind::LBracket => self.parse_array_literal_brackets(),
            TokenKind::Keyword(Keyword::Array) => self.parse_array_literal_legacy(),
            TokenKind::LParen => {
                self.bump();
                let inner = self.parse_expr();
                self.expect(TokenKind::RParen, "`)`");
                let span = start.to(self.last_span);
                Expr {
                    span,
                    kind: ExprKind::Paren(Box::new(inner)),
                }
            }
            TokenKind::Keyword(Keyword::New) => self.parse_new(),
            TokenKind::Keyword(Keyword::Clone) => {
                self.bump();
                let e = self.parse_unary();
                let span = start.to(e.span);
                Expr {
                    span,
                    kind: ExprKind::Clone(Box::new(e)),
                }
            }
            TokenKind::Keyword(Keyword::Function) => self.parse_closure(false),
            TokenKind::Keyword(Keyword::Fn) => self.parse_arrow_fn(false),
            TokenKind::Keyword(Keyword::Match) => self.parse_match(),
            TokenKind::Keyword(Keyword::Yield) => self.parse_yield(),
            TokenKind::Keyword(Keyword::Print) => {
                self.bump();
                let e = self.parse_expr();
                let span = start.to(e.span);
                Expr {
                    span,
                    kind: ExprKind::Print(Box::new(e)),
                }
            }
            TokenKind::Keyword(Keyword::Throw) => {
                self.bump();
                let e = self.parse_expr();
                let span = start.to(e.span);
                Expr {
                    span,
                    kind: ExprKind::Throw(Box::new(e)),
                }
            }
            TokenKind::Keyword(Keyword::Isset) => self.parse_isset(),
            TokenKind::Keyword(Keyword::Empty) => self.parse_empty(),
            TokenKind::Keyword(Keyword::Exit | Keyword::Die) => self.parse_exit(),
            TokenKind::Keyword(Keyword::Eval) => self.parse_eval(),
            TokenKind::Keyword(Keyword::Extract) => self.parse_extract(),
            TokenKind::Keyword(Keyword::Settype) => self.parse_settype(),
            TokenKind::Keyword(Keyword::Include) => self.parse_include(IncludeKind::Include),
            TokenKind::Keyword(Keyword::IncludeOnce) => {
                self.parse_include(IncludeKind::IncludeOnce)
            }
            TokenKind::Keyword(Keyword::Require) => self.parse_include(IncludeKind::Require),
            TokenKind::Keyword(Keyword::RequireOnce) => {
                self.parse_include(IncludeKind::RequireOnce)
            }
            TokenKind::Ident
                if self.at_contextual("spawn") && self.peek_at(1).kind == TokenKind::Ident =>
            {
                let second = self.peek_at(1).span;
                if self.ident_text_ci(second) == "script" {
                    self.parse_spawn_script()
                } else {
                    self.parse_name_expr()
                }
            }
            TokenKind::Ident | TokenKind::Backslash => self.parse_name_expr(),
            _ => {
                let span = self.error_expected_expr();
                Expr {
                    span,
                    kind: ExprKind::Error,
                }
            }
        }
    }

    fn parse_name_expr(&mut self) -> Expr {
        let name = self.parse_name();
        Expr {
            span: name.span,
            kind: ExprKind::ConstFetch(name),
        }
    }

    // ========================================================================
    // Array literals
    // ========================================================================

    fn parse_array_literal_brackets(&mut self) -> Expr {
        let start = self.bump().span; // '['
        let items = self.parse_array_items(TokenKind::RBracket);
        self.expect(TokenKind::RBracket, "`]`");
        let span = start.to(self.last_span);
        Expr {
            span,
            kind: ExprKind::ArrayLiteral(items),
        }
    }

    fn parse_array_literal_legacy(&mut self) -> Expr {
        let start = self.bump().span; // 'array'
        self.expect(TokenKind::LParen, "`(`");
        let items = self.parse_array_items(TokenKind::RParen);
        self.expect(TokenKind::RParen, "`)`");
        let span = start.to(self.last_span);
        Expr {
            span,
            kind: ExprKind::ArrayLiteral(items),
        }
    }

    fn parse_array_items(&mut self, closer: TokenKind) -> Vec<ArrayItem> {
        let mut items = Vec::new();
        while !self.at(closer) && !self.at(TokenKind::Eof) {
            items.push(self.parse_array_item());
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        items
    }

    fn parse_array_item(&mut self) -> ArrayItem {
        let start = self.peek().span;
        if self.eat(TokenKind::Ellipsis).is_some() {
            let value = self.parse_expr();
            let span = start.to(value.span);
            return ArrayItem {
                key: None,
                value,
                spread: true,
                by_ref: false,
                span,
            };
        }
        let leading_ref = self.eat(TokenKind::Amp).is_some();
        let first = self.parse_expr();
        if !leading_ref && self.eat(TokenKind::FatArrow).is_some() {
            let by_ref = self.eat(TokenKind::Amp).is_some();
            let value = self.parse_expr();
            let span = start.to(value.span);
            return ArrayItem {
                key: Some(first),
                value,
                spread: false,
                by_ref,
                span,
            };
        }
        let span = start.to(first.span);
        ArrayItem {
            key: None,
            value: first,
            spread: false,
            by_ref: leading_ref,
            span,
        }
    }

    // ========================================================================
    // `new`
    // ========================================================================

    fn parse_new(&mut self) -> Expr {
        let start = self.bump().span; // 'new'
        if self.at_keyword(Keyword::Class) {
            return self.parse_new_anon_class(start);
        }
        let target = match self.peek().kind {
            TokenKind::Keyword(Keyword::SelfKw) => {
                self.bump();
                NewTarget::SelfTy
            }
            TokenKind::Keyword(Keyword::Static) => {
                self.bump();
                NewTarget::StaticTy
            }
            TokenKind::Keyword(Keyword::Parent) => {
                self.bump();
                NewTarget::ParentTy
            }
            TokenKind::Variable | TokenKind::LParen => {
                NewTarget::Expr(Box::new(self.parse_new_target_expr()))
            }
            TokenKind::Ident | TokenKind::Backslash => NewTarget::Name(self.parse_name()),
            _ => {
                let span = self.error_expected_expr();
                return Expr {
                    span: start.to(span),
                    kind: ExprKind::Error,
                };
            }
        };
        let args = if self.at(TokenKind::LParen) {
            self.parse_call_args()
        } else {
            CallArgs::List(Vec::new())
        };
        let span = start.to(self.last_span);
        Expr {
            span,
            kind: ExprKind::New { target, args },
        }
    }

    /// `new class (args)? (extends Base)? (implements Iface, ...)? { ... }`
    /// — an anonymous class declaration used directly as a `new` target.
    /// Unlike an ordinary `new Name(args)`, the argument list sits right
    /// after `class`, before `extends`/`implements`/the body, so it cannot
    /// reuse [`Self::parse_new`]'s generic post-target `args` parsing.
    fn parse_new_anon_class(&mut self, start: Span) -> Expr {
        self.bump(); // 'class'
        let args = if self.at(TokenKind::LParen) {
            self.parse_call_args()
        } else {
            CallArgs::List(Vec::new())
        };
        let extends = if self.eat_keyword(Keyword::Extends).is_some() {
            Some(self.parse_name())
        } else {
            None
        };
        let implements = if self.eat_keyword(Keyword::Implements).is_some() {
            self.parse_name_list()
        } else {
            Vec::new()
        };
        let members = self.parse_class_body();
        let span = start.to(self.last_span);
        Expr {
            span,
            kind: ExprKind::New {
                target: NewTarget::AnonClass(Box::new(AnonClassDecl {
                    span,
                    extends,
                    implements,
                    members,
                })),
                args,
            },
        }
    }

    /// The class-value expression of `new $expr(...)`/`new (expr)(...)`: a
    /// restricted postfix chain that stops before a call, because the
    /// trailing `(...)` belongs to `new` itself, not to this expression.
    fn parse_new_target_expr(&mut self) -> Expr {
        let mut e = self.parse_primary();
        loop {
            match self.peek().kind {
                TokenKind::Arrow | TokenKind::NullsafeArrow => {
                    let nullsafe = matches!(self.peek().kind, TokenKind::NullsafeArrow);
                    self.bump();
                    let member = self.parse_member_name();
                    let span = e.span.to(self.last_span);
                    e = Expr {
                        span,
                        kind: ExprKind::PropertyAccess {
                            object: Box::new(e),
                            nullsafe,
                            property: member,
                        },
                    };
                }
                TokenKind::DoubleColon => {
                    self.bump();
                    if let TokenKind::Variable = self.peek().kind {
                        let name = self.bump().span;
                        let span = e.span.to(name);
                        e = Expr {
                            span,
                            kind: ExprKind::StaticPropertyAccess {
                                class: Box::new(e),
                                name,
                            },
                        };
                    } else {
                        let name = self.expect(TokenKind::Ident, "a class constant name");
                        let span = e.span.to(name);
                        e = Expr {
                            span,
                            kind: ExprKind::ClassConstAccess {
                                class: Box::new(e),
                                name,
                            },
                        };
                    }
                }
                TokenKind::LBracket => {
                    self.bump();
                    let index = if self.at(TokenKind::RBracket) {
                        None
                    } else {
                        Some(Box::new(self.parse_expr()))
                    };
                    self.expect(TokenKind::RBracket, "`]`");
                    let span = e.span.to(self.last_span);
                    e = Expr {
                        span,
                        kind: ExprKind::Index {
                            base: Box::new(e),
                            index,
                        },
                    };
                }
                _ => break,
            }
        }
        e
    }

    // ========================================================================
    // `match`
    // ========================================================================

    fn parse_match(&mut self) -> Expr {
        let start = self.bump().span; // 'match'
        self.expect(TokenKind::LParen, "`(`");
        let subject = self.parse_expr();
        self.expect(TokenKind::RParen, "`)`");
        self.expect(TokenKind::LBrace, "`{`");
        let mut arms = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let arm_start = self.peek().span;
            let conditions = if self.eat_keyword(Keyword::Default).is_some() {
                None
            } else {
                let mut conds = vec![self.parse_expr()];
                while self.eat(TokenKind::Comma).is_some() && !self.at(TokenKind::FatArrow) {
                    conds.push(self.parse_expr());
                }
                Some(conds)
            };
            self.expect(TokenKind::FatArrow, "`=>`");
            let body = self.parse_expr();
            let span = arm_start.to(body.span);
            arms.push(MatchArm {
                conditions,
                body,
                span,
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::RBrace, "`}`");
        let span = start.to(self.last_span);
        Expr {
            span,
            kind: ExprKind::Match {
                subject: Box::new(subject),
                arms,
            },
        }
    }

    // ========================================================================
    // Closures and arrow functions
    // ========================================================================

    fn parse_params(&mut self) -> Vec<Param> {
        self.expect(TokenKind::LParen, "`(`");
        let mut params = Vec::new();
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            params.push(self.parse_param());
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::RParen, "`)`");
        params
    }

    fn parse_param(&mut self) -> Param {
        let start = self.peek().span;
        let attributes = self.parse_attribute_groups();
        let modifiers = self.parse_modifiers();
        let ty = if self.can_start_type() {
            Some(self.parse_type())
        } else {
            let span = self.peek().span.shrink_to_start();
            self.diags.report(
                Diagnostic::error(code::E_EXPECTED_TOKEN, "expected a parameter type")
                    .with_primary(span, "every parameter declares a type (ADR 0007)"),
            );
            None
        };
        let by_ref = self.eat(TokenKind::Amp).is_some();
        let variadic = self.eat(TokenKind::Ellipsis).is_some();
        let name = self.expect(TokenKind::Variable, "a parameter name");
        let default = if self.eat(TokenKind::Equals).is_some() {
            Some(self.parse_expr())
        } else {
            None
        };
        let span = start.to(self.last_span);
        Param {
            span,
            attributes,
            modifiers,
            ty,
            by_ref,
            variadic,
            name,
            default,
        }
    }

    fn parse_closure(&mut self, is_static: bool) -> Expr {
        let start = self.peek().span;
        if is_static {
            self.report_static_closure_modifier(start);
            self.bump();
        }
        self.expect(TokenKind::Keyword(Keyword::Function), "`function`");
        let by_ref = self.eat(TokenKind::Amp).is_some();
        let params = self.parse_params();
        let uses = if self.eat_keyword(Keyword::Use).is_some() {
            self.expect(TokenKind::LParen, "`(`");
            let mut list = Vec::new();
            while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
                let by_ref = self.eat(TokenKind::Amp).is_some();
                let name = self.expect(TokenKind::Variable, "a captured variable");
                list.push(ClosureUse { by_ref, name });
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "`)`");
            list
        } else {
            Vec::new()
        };
        let return_type = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type())
        } else {
            None
        };
        let body = self.parse_block();
        let span = start.to(body.span);
        Expr {
            span,
            kind: ExprKind::Closure(ClosureExpr {
                is_static,
                by_ref,
                params,
                uses,
                return_type,
                body,
            }),
        }
    }

    fn parse_arrow_fn(&mut self, is_static: bool) -> Expr {
        let start = self.peek().span;
        if is_static {
            self.report_static_closure_modifier(start);
            self.bump();
        }
        self.expect(TokenKind::Keyword(Keyword::Fn), "`fn`");
        let params = self.parse_params();
        let return_type = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type())
        } else {
            None
        };
        self.expect(TokenKind::FatArrow, "`=>`");
        let body = self.parse_expr();
        let span = start.to(body.span);
        Expr {
            span,
            kind: ExprKind::ArrowFn(ArrowFnExpr {
                is_static,
                params,
                return_type,
                body: Box::new(body),
            }),
        }
    }

    // ========================================================================
    // `yield`, `print`/`throw` already inlined above, `isset`/`empty`/`exit`
    // ========================================================================

    fn at_expr_terminator(&mut self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Semicolon
                | TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::RBrace
                | TokenKind::Comma
                | TokenKind::Eof
                | TokenKind::CloseTag
        )
    }

    fn parse_yield(&mut self) -> Expr {
        let start = self.bump().span; // 'yield'
        if self.at_contextual("from") {
            self.bump();
            let e = self.parse_expr();
            let span = start.to(e.span);
            return Expr {
                span,
                kind: ExprKind::YieldFrom(Box::new(e)),
            };
        }
        if self.at_expr_terminator() {
            return Expr {
                span: start,
                kind: ExprKind::Yield {
                    key: None,
                    value: None,
                },
            };
        }
        let first = self.parse_expr();
        if self.eat(TokenKind::FatArrow).is_some() {
            let value = self.parse_expr();
            let span = start.to(value.span);
            return Expr {
                span,
                kind: ExprKind::Yield {
                    key: Some(Box::new(first)),
                    value: Some(Box::new(value)),
                },
            };
        }
        let span = start.to(first.span);
        Expr {
            span,
            kind: ExprKind::Yield {
                key: None,
                value: Some(Box::new(first)),
            },
        }
    }

    fn parse_isset(&mut self) -> Expr {
        let start = self.bump().span;
        self.expect(TokenKind::LParen, "`(`");
        let mut args = vec![self.parse_expr()];
        while self.eat(TokenKind::Comma).is_some() && !self.at(TokenKind::RParen) {
            args.push(self.parse_expr());
        }
        self.expect(TokenKind::RParen, "`)`");
        let span = start.to(self.last_span);
        Expr {
            span,
            kind: ExprKind::Isset(args),
        }
    }

    fn parse_empty(&mut self) -> Expr {
        let start = self.bump().span;
        self.expect(TokenKind::LParen, "`(`");
        let e = self.parse_expr();
        self.expect(TokenKind::RParen, "`)`");
        let span = start.to(self.last_span);
        Expr {
            span,
            kind: ExprKind::Empty(Box::new(e)),
        }
    }

    fn parse_exit(&mut self) -> Expr {
        let start = self.bump().span;
        let arg = if self.eat(TokenKind::LParen).is_some() {
            let e = if self.at(TokenKind::RParen) {
                None
            } else {
                Some(Box::new(self.parse_expr()))
            };
            self.expect(TokenKind::RParen, "`)`");
            e
        } else {
            None
        };
        let span = start.to(self.last_span);
        Expr {
            span,
            kind: ExprKind::ExitOrDie(arg),
        }
    }

    // ========================================================================
    // Rejected calls that need their own argument-list parse, so their
    // diagnostic covers the whole call rather than "expected an expression"
    // at a keyword that shouldn't be one
    // ========================================================================

    /// Consumes `'(' expr (',' expr)* ')'` without keeping any of it — for a
    /// rejected pseudo-call whose arguments never reach the AST.
    fn skip_call_args(&mut self) {
        self.expect(TokenKind::LParen, "`(`");
        if !self.at(TokenKind::RParen) {
            let _ = self.parse_expr();
            while self.eat(TokenKind::Comma).is_some() && !self.at(TokenKind::RParen) {
                let _ = self.parse_expr();
            }
        }
        self.expect(TokenKind::RParen, "`)`");
    }

    /// `eval(...)` — ADR 0007 § 2: there is no such construct, since a string
    /// has no stable identity to compile ahead of time.
    fn parse_eval(&mut self) -> Expr {
        let start = self.bump().span;
        self.skip_call_args();
        let span = start.to(self.last_span);
        self.diags.report(
            Diagnostic::error(code::E_EVAL_UNSUPPORTED, "`eval` is not supported")
                .with_primary(span, "MWL compiles ahead of execution")
                .with_help(
                    "give the code a path: `include` it to share this frame, or `spawn script` \
                     it to isolate it",
                ),
        );
        Expr {
            span,
            kind: ExprKind::Error,
        }
    }

    /// `extract(...)` — ADR 0007 § 2: introduces bindings whose names are not
    /// known statically, which every later stage assumes it can enumerate.
    fn parse_extract(&mut self) -> Expr {
        let start = self.bump().span;
        self.skip_call_args();
        let span = start.to(self.last_span);
        self.diags.report(
            Diagnostic::error(code::E_EXTRACT_UNSUPPORTED, "`extract` is not supported")
                .with_primary(
                    span,
                    "introduces bindings whose names are not known statically",
                )
                .with_help("destructure the array explicitly, or index it by key"),
        );
        Expr {
            span,
            kind: ExprKind::Error,
        }
    }

    /// `settype(...)` — ADR 0007 § 2: no assignment, operator or call may
    /// change what a binding's declared type is; `as` converts into a new
    /// binding instead.
    fn parse_settype(&mut self) -> Expr {
        let start = self.bump().span;
        self.skip_call_args();
        let span = start.to(self.last_span);
        self.diags.report(
            Diagnostic::error(code::E_SETTYPE_UNSUPPORTED, "`settype` is not supported")
                .with_primary(
                    span,
                    "no binding's declared type ever changes after the fact",
                )
                .with_help("convert into a new, differently typed binding with `as` instead"),
        );
        Expr {
            span,
            kind: ExprKind::Error,
        }
    }

    /// `include`/`include_once`/`require`/`require_once` — an expression,
    /// not a statement, per
    /// [`docs/spec/00-overview.md` § 2](../../../docs/spec/00-overview.md):
    /// same frame, same globals, same statics as the caller. Precedence
    /// mirrors `print`/`throw` above: it consumes a full expression, not
    /// just a primary.
    fn parse_include(&mut self, kind: IncludeKind) -> Expr {
        let start = self.bump().span;
        let path = self.parse_expr();
        let span = start.to(path.span);
        Expr {
            span,
            kind: ExprKind::Include {
                kind,
                path: Box::new(path),
            },
        }
    }

    // ========================================================================
    // `spawn script … with(…)`
    // ========================================================================

    fn parse_spawn_script(&mut self) -> Expr {
        let start = self.bump().span; // 'spawn'
        self.bump(); // 'script'
        let path = self.parse_expr();
        let options = if self.at_contextual("with") {
            self.bump();
            self.expect(TokenKind::LParen, "`(`");
            let mut opts = Vec::new();
            while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
                opts.push(self.parse_spawn_option());
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            self.expect(TokenKind::RParen, "`)`");
            opts
        } else {
            Vec::new()
        };
        let span = start.to(self.last_span);
        Expr {
            span,
            kind: ExprKind::SpawnScript {
                path: Box::new(path),
                options,
            },
        }
    }

    fn parse_spawn_option(&mut self) -> SpawnOption {
        let start = self.peek().span;
        let key_span = self.expect(TokenKind::Ident, "a `with(...)` option name");
        let key_text = self.ident_text_ci(key_span);
        let key = match key_text.as_str() {
            "args" => SpawnOptionKey::Args,
            "limits" => SpawnOptionKey::Limits,
            "grants" => SpawnOptionKey::Grants,
            "output" => SpawnOptionKey::Output,
            "on" => SpawnOptionKey::On,
            _ => {
                self.diags.report(
                    Diagnostic::error(
                        code::E_EXPECTED_TOKEN,
                        format!("unknown `with(...)` option `{key_text}`"),
                    )
                    .with_primary(
                        key_span,
                        "expected one of `args`, `limits`, `grants`, `output`, `on`",
                    ),
                );
                SpawnOptionKey::Args
            }
        };
        self.expect(TokenKind::Colon, "`:`");
        let value = self.parse_expr();
        let span = start.to(value.span);
        SpawnOption { key, value, span }
    }

    // ========================================================================
    // Interpolated strings
    // ========================================================================

    fn parse_double_quoted_string(&mut self) -> Expr {
        let open = self.bump().span; // DoubleQuoteOpen
        let (parts, close) = self.parse_string_body(TokenKind::DoubleQuoteClose);
        collapse_string_parts(open.to(close), parts)
    }

    fn parse_heredoc_string(&mut self) -> Expr {
        let open = self.bump().span; // HeredocOpen or NowdocOpen
        let (parts, close) = self.parse_string_body(TokenKind::HeredocClose);
        collapse_string_parts(open.to(close), parts)
    }

    fn parse_string_body(&mut self, closer: TokenKind) -> (Vec<StringPart>, Span) {
        let mut parts = Vec::new();
        loop {
            let kind = self.peek().kind;
            if kind == closer {
                return (parts, self.bump().span);
            }
            match kind {
                TokenKind::StringPart => parts.push(StringPart::Text(self.bump().span)),
                TokenKind::Variable => {
                    parts.push(StringPart::Expr(self.parse_simple_interp_variable()))
                }
                TokenKind::ComplexInterpOpen => {
                    self.bump();
                    let e = self.parse_expr();
                    self.expect(TokenKind::ComplexInterpClose, "`}`");
                    parts.push(StringPart::Expr(e));
                }
                TokenKind::Eof => return (parts, self.peek().span),
                _ => parts.push(StringPart::Text(self.bump().span)),
            }
        }
    }

    /// PHP's "simple syntax" interpolation: `$name`, `$name->prop` (one level
    /// only) or `$name[offset]` — exactly the shapes the lexer already
    /// restricted itself to, see [`crate::lexer`]'s
    /// `lex_simple_interpolation`.
    fn parse_simple_interp_variable(&mut self) -> Expr {
        let var_span = self.bump().span; // Variable
        let var_expr = Expr {
            span: var_span,
            kind: ExprKind::Variable(var_span),
        };
        match self.peek().kind {
            TokenKind::Arrow if matches!(self.peek_at(1).kind, TokenKind::Ident) => {
                self.bump();
                let prop = self.bump().span;
                let span = var_span.to(prop);
                Expr {
                    span,
                    kind: ExprKind::PropertyAccess {
                        object: Box::new(var_expr),
                        nullsafe: false,
                        property: MemberName::Ident(prop),
                    },
                }
            }
            TokenKind::LBracket => {
                self.bump();
                let index = match self.peek().kind {
                    TokenKind::Variable => {
                        let s = self.bump().span;
                        Expr {
                            span: s,
                            kind: ExprKind::Variable(s),
                        }
                    }
                    TokenKind::Ident => {
                        let s = self.bump().span;
                        Expr {
                            span: s,
                            kind: ExprKind::Str(s),
                        }
                    }
                    _ => {
                        let span = self.error_expected("an array offset");
                        Expr {
                            span,
                            kind: ExprKind::Error,
                        }
                    }
                };
                self.expect(TokenKind::RBracket, "`]`");
                let span = var_span.to(self.last_span);
                Expr {
                    span,
                    kind: ExprKind::Index {
                        base: Box::new(var_expr),
                        index: Some(Box::new(index)),
                    },
                }
            }
            _ => var_expr,
        }
    }

    // ========================================================================
    // Statements
    // ========================================================================

    /// A `{ ... }` block. If a statement consumes no tokens at all (a
    /// construct not yet implemented, sitting at a token no statement form
    /// starts with), one token is force-consumed so a malformed body cannot
    /// hang the parser — see the module docs.
    pub(crate) fn parse_block(&mut self) -> Block {
        let start = self.expect(TokenKind::LBrace, "`{`");
        let mut stmts = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let before = self.peek().span;
            stmts.push(self.parse_statement());
            if self.peek().span == before && !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "`}`");
        let span = start.to(self.last_span);
        Block { stmts, span }
    }

    /// Statements nest into statements without bound (`{{{{...}}}}`,
    /// `if(1)if(1)if(1)...;`, ...) purely through the ordinary recursive-
    /// descent call graph — [`Self::parse_block`]'s own force-progress
    /// guard only stops a *malformed* body from hanging, it does nothing
    /// for input that is well-formed but absurdly deep. Needs the same
    /// depth guard as [`Self::parse_assignment`].
    fn parse_statement(&mut self) -> Stmt {
        self.guarded(Self::error_stmt_here, Self::parse_statement_inner)
    }

    fn error_stmt_here(&mut self) -> Stmt {
        let span = self.peek().span.shrink_to_start();
        Stmt {
            span,
            kind: StmtKind::Empty,
        }
    }

    fn parse_statement_inner(&mut self) -> Stmt {
        // --- HTML-mode round trip, spec 00-overview.md § 1 ---------------------
        // `?>`/`<?php`/`<?mwl` can reopen or reclose code mode anywhere a
        // statement is expected, not just at file scope — e.g.
        // `if ($x) { ?>html<?php }` is legal, exactly as in PHP. Skip every
        // bare tag token here; stop at the first token that is either real
        // inline HTML (one `InlineHtml` statement) or ordinary code. Nothing
        // here recurses into `self.parse_statement()`, so a tag run can't
        // loop forever even when the file ends right after `?>`.
        loop {
            match self.peek().kind {
                TokenKind::CloseTag | TokenKind::OpenTagMwl | TokenKind::OpenTagPhp => {
                    self.bump();
                }
                TokenKind::InlineHtml => {
                    let span = self.bump().span;
                    return Stmt {
                        span,
                        kind: StmtKind::InlineHtml(span),
                    };
                }
                TokenKind::OpenTagEcho => {
                    let start = self.peek().span;
                    return self.parse_short_echo_tag(start);
                }
                _ => break,
            }
        }

        let start = self.peek().span;
        match self.peek().kind {
            // A tag token above can leave us sitting on the enclosing block's
            // `}` or on EOF (`<?mwl if ($x) { ?><?php }`, or a file that ends
            // right after `?>`). The caller's own loop (`parse_block`,
            // `parse_file`) is what notices and stops, not this function.
            TokenKind::RBrace | TokenKind::Eof => Stmt {
                span: start,
                kind: StmtKind::Empty,
            },
            TokenKind::LBrace => {
                let block = self.parse_block();
                Stmt {
                    span: block.span,
                    kind: StmtKind::Block(block),
                }
            }
            TokenKind::Semicolon => {
                self.bump();
                Stmt {
                    span: start,
                    kind: StmtKind::Empty,
                }
            }
            TokenKind::Keyword(Keyword::Return) => {
                self.bump();
                let value = if self.at(TokenKind::Semicolon) {
                    None
                } else {
                    Some(self.parse_expr())
                };
                self.expect(TokenKind::Semicolon, "`;`");
                let span = start.to(self.last_span);
                Stmt {
                    span,
                    kind: StmtKind::Return(value),
                }
            }
            TokenKind::Keyword(Keyword::If) => {
                self.bump();
                self.finish_if(start)
            }
            TokenKind::Keyword(Keyword::While) => self.parse_while(start),
            TokenKind::Keyword(Keyword::Do) => self.parse_do_while(start),
            TokenKind::Keyword(Keyword::For) => self.parse_for(start),
            TokenKind::Keyword(Keyword::Foreach) => self.parse_foreach(start),
            TokenKind::Keyword(Keyword::Switch) => self.parse_switch(start),
            TokenKind::Keyword(Keyword::Break) => self.parse_break_continue(start, true),
            TokenKind::Keyword(Keyword::Continue) => self.parse_break_continue(start, false),
            TokenKind::Keyword(Keyword::Try) => self.parse_try(start),
            TokenKind::Keyword(Keyword::Echo) => self.parse_echo(start),
            TokenKind::Keyword(Keyword::Unset) => self.parse_unset_stmt(start),
            TokenKind::Keyword(Keyword::Global) => self.parse_global(start),
            TokenKind::Keyword(Keyword::Goto) => self.parse_goto(start),
            TokenKind::Keyword(Keyword::Static) if self.at_function_scope_static() => {
                self.parse_static_local(start)
            }
            TokenKind::Keyword(Keyword::List) => self.parse_destructure_from_list(start),
            TokenKind::LBracket => self.parse_stmt_maybe_destructure(start),
            TokenKind::AttributeOpen => self.parse_attributed_decl_stmt(start),
            TokenKind::Keyword(Keyword::Abstract | Keyword::Final | Keyword::Class) => {
                self.parse_class_decl(start)
            }
            TokenKind::Keyword(Keyword::Interface) => self.parse_interface_decl(start),
            TokenKind::Keyword(Keyword::Trait) => self.parse_trait_decl(start),
            TokenKind::Keyword(Keyword::Enum) => self.parse_enum_decl(start),
            TokenKind::Keyword(Keyword::Namespace) => self.parse_namespace_decl(start),
            TokenKind::Keyword(Keyword::Use) => self.parse_use_decl(start),
            TokenKind::Keyword(Keyword::Const) => {
                self.parse_toplevel_const_reject(start, Vec::new())
            }
            TokenKind::Keyword(Keyword::Function) if self.at_named_function_decl() => {
                self.parse_toplevel_function_reject(start, Vec::new())
            }
            _ if self.at_contextual("type")
                && self.peek_at(1).kind == TokenKind::Ident
                && self.peek_at(2).kind == TokenKind::Equals =>
            {
                self.parse_type_alias_decl(start)
            }
            _ if self.can_start_type() && !self.at_keyword(Keyword::Static) => {
                self.parse_stmt_maybe_local_decl(start)
            }
            _ => self.parse_expr_statement(start),
        }
    }

    fn parse_expr_statement(&mut self, start: Span) -> Stmt {
        let expr = self.parse_expr();
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::Expr(expr),
        }
    }

    // ------------------------------------------------------------------------
    // `if`/`elseif`/`else`
    // ------------------------------------------------------------------------

    /// Parses the rest of an `if`/`elseif` given its keyword was already
    /// consumed at `start`. `elseif` re-enters here directly — its keyword is
    /// a single token, not `else` followed by `if`, but produces exactly the
    /// same nested-`If`-inside-`else_` shape as the two-word spelling (which
    /// falls out for free: `else` bumps its own keyword, then
    /// `parse_statement` sees `if` next and recurses through the ordinary
    /// dispatch arm above).
    fn finish_if(&mut self, start: Span) -> Stmt {
        self.expect(TokenKind::LParen, "`(`");
        let cond = self.parse_expr();
        self.expect(TokenKind::RParen, "`)`");
        let then = Box::new(self.parse_statement());
        let else_ = if let Some(elseif_start) = self.eat_keyword(Keyword::Elseif) {
            Some(Box::new(self.finish_if(elseif_start)))
        } else if self.eat_keyword(Keyword::Else).is_some() {
            Some(Box::new(self.parse_statement()))
        } else {
            None
        };
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::If { cond, then, else_ },
        }
    }

    // ------------------------------------------------------------------------
    // Loops
    // ------------------------------------------------------------------------

    fn parse_while(&mut self, start: Span) -> Stmt {
        self.bump();
        self.expect(TokenKind::LParen, "`(`");
        let cond = self.parse_expr();
        self.expect(TokenKind::RParen, "`)`");
        let body = Box::new(self.parse_statement());
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::While { cond, body },
        }
    }

    fn parse_do_while(&mut self, start: Span) -> Stmt {
        self.bump();
        let body = Box::new(self.parse_statement());
        self.expect_keyword(Keyword::While, "`while`");
        self.expect(TokenKind::LParen, "`(`");
        let cond = self.parse_expr();
        self.expect(TokenKind::RParen, "`)`");
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::DoWhile { body, cond },
        }
    }

    /// A comma-separated list of expressions, any of which may be absent —
    /// one clause of a `for` header.
    fn parse_expr_list_until(&mut self, closer: TokenKind) -> Vec<Expr> {
        let mut list = Vec::new();
        if self.at(closer) {
            return list;
        }
        loop {
            list.push(self.parse_expr());
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        list
    }

    fn parse_for(&mut self, start: Span) -> Stmt {
        self.bump();
        self.expect(TokenKind::LParen, "`(`");
        let init = self.parse_expr_list_until(TokenKind::Semicolon);
        self.expect(TokenKind::Semicolon, "`;`");
        let cond = self.parse_expr_list_until(TokenKind::Semicolon);
        self.expect(TokenKind::Semicolon, "`;`");
        let step = self.parse_expr_list_until(TokenKind::RParen);
        self.expect(TokenKind::RParen, "`)`");
        let body = Box::new(self.parse_statement());
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::For {
                init,
                cond,
                step,
                body,
            },
        }
    }

    /// One `type '&'? '$' identifier` binding — the shared tail of both
    /// `foreach`-target alternatives (ADR 0007 § 3.2). The reference marker
    /// is parsed here and reported back to the caller, since only the
    /// *value* position may carry one; the key position never calls this
    /// with a marker present without the caller first checking for one.
    fn parse_foreach_binding(&mut self) -> (ForeachBinding, bool) {
        let start = self.peek().span;
        let ty = if self.can_start_type() {
            Some(self.parse_type())
        } else {
            let span = self.peek().span.shrink_to_start();
            self.diags.report(
                Diagnostic::error(
                    code::E_EXPECTED_TOKEN,
                    "expected a `foreach` binding's type",
                )
                .with_primary(
                    span,
                    "every `foreach` binding declares a type (ADR 0007 § 3.2)",
                ),
            );
            None
        };
        let by_ref = self.eat(TokenKind::Amp).is_some();
        let name = self.expect(TokenKind::Variable, "a `foreach` binding name");
        let span = start.to(self.last_span);
        (ForeachBinding { ty, name, span }, by_ref)
    }

    /// `foreach (subject as key? value) body`, ADR 0007 § 3.2. The header's
    /// own `as` is looked for explicitly after a suppressed-`as` subject
    /// parse (see [`Self::parse_expr_no_top_as`]), and the first binding is
    /// re-read as the key only once a `=>` confirms it was one — a reference
    /// marker right after the first binding's type can only mean the
    /// no-key, by-reference form (`foreach ($x as int &$v)`), since the
    /// two-binding form's marker sits after the *second* type instead.
    fn parse_foreach(&mut self, start: Span) -> Stmt {
        self.bump();
        self.expect(TokenKind::LParen, "`(`");
        let subject = self.parse_expr_no_top_as();
        self.expect_keyword(Keyword::As, "`as`");
        let (first, first_by_ref) = self.parse_foreach_binding();
        let (key, value, value_by_ref) = if !first_by_ref && self.eat(TokenKind::FatArrow).is_some()
        {
            let (value, value_by_ref) = self.parse_foreach_binding();
            (Some(first), value, value_by_ref)
        } else {
            (None, first, first_by_ref)
        };
        self.expect(TokenKind::RParen, "`)`");
        let body = Box::new(self.parse_statement());
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::Foreach {
                subject,
                key,
                value,
                value_by_ref,
                body,
            },
        }
    }

    // ------------------------------------------------------------------------
    // `switch`
    // ------------------------------------------------------------------------

    /// Mirrors [`Self::parse_block`]'s force-progress guard: a `switch` body
    /// with no well-formed `case`/`default` anywhere (so
    /// [`Self::parse_switch_case`] cannot even consume a keyword to start
    /// one) must not hang the parser — a fuzz run found exactly this input
    /// spinning forever, growing `cases` without bound.
    fn parse_switch(&mut self, start: Span) -> Stmt {
        self.bump();
        self.expect(TokenKind::LParen, "`(`");
        let subject = self.parse_expr();
        self.expect(TokenKind::RParen, "`)`");
        self.expect(TokenKind::LBrace, "`{`");
        let mut cases = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let before = self.peek().span;
            cases.push(self.parse_switch_case());
            if self.peek().span == before && !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "`}`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::Switch { subject, cases },
        }
    }

    fn parse_switch_case(&mut self) -> SwitchCase {
        let start = self.peek().span;
        let cond = if self.eat_keyword(Keyword::Default).is_some() {
            None
        } else {
            self.expect_keyword(Keyword::Case, "`case` or `default`");
            Some(self.parse_expr())
        };
        self.expect(TokenKind::Colon, "`:`");
        let mut body = Vec::new();
        // Same guard as above: `parse_statement` can fail to consume
        // anything on malformed input (that is what `parse_block`'s own
        // copy of this guard exists for), and this loop has no closing
        // brace of its own to eventually stop it — only the next
        // `case`/`default`/`}`/EOF.
        while !matches!(
            self.peek().kind,
            TokenKind::Keyword(Keyword::Case | Keyword::Default) | TokenKind::RBrace
        ) && !self.at(TokenKind::Eof)
        {
            let before = self.peek().span;
            body.push(self.parse_statement());
            if self.peek().span == before
                && !matches!(
                    self.peek().kind,
                    TokenKind::Keyword(Keyword::Case | Keyword::Default) | TokenKind::RBrace
                )
                && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
        }
        let span = start.to(self.last_span);
        SwitchCase { cond, body, span }
    }

    // ------------------------------------------------------------------------
    // `break`/`continue`
    // ------------------------------------------------------------------------

    fn parse_break_continue(&mut self, start: Span, is_break: bool) -> Stmt {
        self.bump();
        let level = if self.at(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expr())
        };
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: if is_break {
                StmtKind::Break(level)
            } else {
                StmtKind::Continue(level)
            },
        }
    }

    // ------------------------------------------------------------------------
    // `try`/`catch`/`finally`
    // ------------------------------------------------------------------------

    fn parse_try(&mut self, start: Span) -> Stmt {
        self.bump();
        let body = self.parse_block();
        let mut catches = Vec::new();
        while self.at_keyword(Keyword::Catch) {
            catches.push(self.parse_catch_clause());
        }
        let finally = self
            .eat_keyword(Keyword::Finally)
            .map(|_| self.parse_block());
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::Try {
                body,
                catches,
                finally,
            },
        }
    }

    /// `catch (Type ('|' Type)* '$'? identifier?) { ... }`. Multi-type catch
    /// falls out of reusing the ordinary type grammar's union — no separate
    /// type-list production is needed.
    fn parse_catch_clause(&mut self) -> CatchClause {
        let start = self.bump().span; // 'catch'
        self.expect(TokenKind::LParen, "`(`");
        let ty = self.parse_type();
        let var = self.eat(TokenKind::Variable);
        self.expect(TokenKind::RParen, "`)`");
        let body = self.parse_block();
        let span = start.to(body.span);
        CatchClause {
            ty,
            var,
            body,
            span,
        }
    }

    // ------------------------------------------------------------------------
    // `echo`, `unset`
    // ------------------------------------------------------------------------

    fn parse_echo(&mut self, start: Span) -> Stmt {
        self.bump();
        let mut exprs = vec![self.parse_expr()];
        while self.eat(TokenKind::Comma).is_some() {
            exprs.push(self.parse_expr());
        }
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::Echo(exprs),
        }
    }

    /// `<?= expr (';')? ?>` — spec § 1: exactly `<?mwl echo expr; ?>`, one
    /// expression, and unlike every other statement form the `;` is optional
    /// right before the closing tag. `?>` is left for `parse_statement`'s next
    /// call to consume, matching how an ordinary `echo` leaves the following
    /// token for its caller.
    fn parse_short_echo_tag(&mut self, start: Span) -> Stmt {
        self.bump();
        let expr = self.parse_expr();
        if !self.at(TokenKind::CloseTag) {
            self.expect(TokenKind::Semicolon, "`;`");
        } else {
            self.eat(TokenKind::Semicolon);
        }
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::Echo(vec![expr]),
        }
    }

    fn parse_unset_stmt(&mut self, start: Span) -> Stmt {
        self.bump();
        self.expect(TokenKind::LParen, "`(`");
        let mut exprs = vec![self.parse_expr()];
        while self.eat(TokenKind::Comma).is_some() && !self.at(TokenKind::RParen) {
            exprs.push(self.parse_expr());
        }
        self.expect(TokenKind::RParen, "`)`");
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::Unset(exprs),
        }
    }

    // ------------------------------------------------------------------------
    // The statement-shaped rejects: `global`, `goto`, function-scope `static`
    // ------------------------------------------------------------------------

    fn parse_global(&mut self, start: Span) -> Stmt {
        self.bump();
        let mut vars = vec![self.expect(TokenKind::Variable, "a variable name")];
        while self.eat(TokenKind::Comma).is_some() {
            vars.push(self.expect(TokenKind::Variable, "a variable name"));
        }
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        self.diags.report(
            Diagnostic::error(code::E_GLOBAL_UNSUPPORTED, "`global` is not supported")
                .with_primary(
                    span,
                    "no function may reach outside its own frame for state",
                )
                .with_help(
                    "pass it as a parameter, or make it a `static` property or a `const` \
                     (ADR 0008 § 5)",
                ),
        );
        Stmt {
            span,
            kind: StmtKind::Global(vars),
        }
    }

    fn parse_goto(&mut self, start: Span) -> Stmt {
        self.bump();
        let label = self.expect(TokenKind::Ident, "a label name");
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        self.diags.report(
            Diagnostic::error(code::E_GOTO_UNSUPPORTED, "`goto` is not supported")
                .with_primary(span, "makes the control-flow graph unstructured")
                .with_help("restructure with a loop, an early `return`, or a boolean flag"),
        );
        Stmt {
            span,
            kind: StmtKind::Goto(label),
        }
    }

    fn parse_static_var(&mut self) -> StaticVar {
        let name = self.expect(TokenKind::Variable, "a variable name");
        let default = self.eat(TokenKind::Equals).map(|_| self.parse_expr());
        StaticVar { name, default }
    }

    /// Function-scope `static` — always rejected, whether written in PHP's
    /// ordinary untyped spelling (`static $calls = 0;`) or the typed
    /// spelling ADR 0008 § 5's own diagnostic wording illustrates
    /// (`static int $calls = 0;`); [`Self::at_function_scope_static`]
    /// already confirmed one of those two shapes follows `static`.
    fn parse_static_local(&mut self, start: Span) -> Stmt {
        self.bump();
        let ty = if self.at(TokenKind::Variable) {
            None
        } else {
            Some(self.parse_type())
        };
        let mut vars = vec![self.parse_static_var()];
        while self.eat(TokenKind::Comma).is_some() {
            vars.push(self.parse_static_var());
        }
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        self.diags.report(
            Diagnostic::error(
                code::E_STATIC_LOCAL_UNSUPPORTED,
                "function-scope `static` is not supported",
            )
            .with_primary(span, "there is no per-function storage class")
            .with_help(
                "declare a `private static` property on a class, or pass the value as a \
                 parameter (ADR 0008 § 5)",
            ),
        );
        Stmt {
            span,
            kind: StmtKind::StaticLocal { ty, vars },
        }
    }

    // ------------------------------------------------------------------------
    // ADR 0007 § 3.1: typed local declaration, vs. an ordinary expression
    // statement that happens to start with the same tokens (a class name
    // used as a type, versus the same name used as a constant fetch or a
    // static-call receiver). The one deciding signal is structural — "the
    // type comes first, in the same position PHP already uses for a
    // parameter" — so a genuine trial parse of the type, checked against
    // whatever token follows it, resolves every case correctly without a
    // second, hand-written classifier that would drift from `parse_type`.
    // ------------------------------------------------------------------------

    fn parse_stmt_maybe_local_decl(&mut self, start: Span) -> Stmt {
        let cp = self.checkpoint();
        let ty = self.parse_type();
        // A malformed type (e.g. a parenthesized *expression* statement like
        // `($a || $b) ? f() : g();` — `(` also starts a type, so this trial
        // runs) can still land back on a `$variable` token by coincidence,
        // since error recovery in `parse_type_atom` doesn't consume the
        // offending token. Diagnostics reported during the trial are the
        // reliable signal that it wasn't actually a type, not just "does a
        // variable happen to follow."
        if !self.at(TokenKind::Variable) || self.diags.len() > cp.diags_len {
            self.restore(cp);
            return self.parse_expr_statement(start);
        }
        let name = self.bump().span;
        let value = self.eat(TokenKind::Equals).map(|_| self.parse_expr());
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::LocalDecl { ty, name, value },
        }
    }

    // ------------------------------------------------------------------------
    // ADR 0007 § 3.3: destructuring statement
    // ------------------------------------------------------------------------

    /// `list(...)` never means anything but a destructuring target — unlike
    /// `[...]`, it collides with no expression grammar — so no backtracking
    /// is needed here.
    fn parse_destructure_from_list(&mut self, start: Span) -> Stmt {
        let open = self.bump().span; // 'list'
        self.expect(TokenKind::LParen, "`(`");
        let elements = self.parse_destructure_elements(TokenKind::RParen);
        let close = self.expect(TokenKind::RParen, "`)`");
        let target = DestructureTarget {
            elements,
            span: open.to(close),
        };
        self.finish_destructure_stmt(start, target)
    }

    /// `[...]` at statement start is ambiguous with a plain array-literal
    /// expression statement (`[1, 2, 3];`, legal if useless) — trial-parse
    /// the more specific destructuring-target grammar and only keep it if a
    /// `=` actually follows, exactly the same backtracking shape as
    /// [`Self::parse_stmt_maybe_local_decl`].
    fn parse_stmt_maybe_destructure(&mut self, start: Span) -> Stmt {
        let cp = self.checkpoint();
        let target = self.parse_destructure_target();
        if !self.at(TokenKind::Equals) {
            self.restore(cp);
            return self.parse_expr_statement(start);
        }
        self.finish_destructure_stmt(start, target)
    }

    fn finish_destructure_stmt(&mut self, start: Span, target: DestructureTarget) -> Stmt {
        self.expect(TokenKind::Equals, "`=`");
        let value = self.parse_expr();
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::Destructure { target, value },
        }
    }

    /// Nested destructuring (`[[[[[$a]]]]] = ...`) recurses through
    /// [`Self::parse_destructure_element`] without bound — needs the same
    /// guard as [`Self::parse_assignment`].
    fn parse_destructure_target(&mut self) -> DestructureTarget {
        self.guarded(
            |p| {
                let span = p.peek().span.shrink_to_start();
                DestructureTarget {
                    elements: Vec::new(),
                    span,
                }
            },
            Self::parse_destructure_target_inner,
        )
    }

    fn parse_destructure_target_inner(&mut self) -> DestructureTarget {
        let open = self.expect(TokenKind::LBracket, "`[`");
        let elements = self.parse_destructure_elements(TokenKind::RBracket);
        let close = self.expect(TokenKind::RBracket, "`]`");
        DestructureTarget {
            elements,
            span: open.to(close),
        }
    }

    fn parse_destructure_elements(&mut self, closer: TokenKind) -> Vec<DestructureElement> {
        let mut elements = Vec::new();
        while !self.at(closer) && !self.at(TokenKind::Eof) {
            elements.push(self.parse_destructure_element());
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        elements
    }

    /// An optional `(string-literal | expr) '=>'` key. `[` never starts a key
    /// (an array cannot be a destructuring key), so it is excluded up front;
    /// otherwise a checkpointed trial parse decides — the same reasoning as
    /// [`Self::parse_stmt_maybe_local_decl`], now nested one level deeper.
    fn parse_destructure_key(&mut self) -> Option<Expr> {
        if self.at(TokenKind::LBracket) {
            return None;
        }
        let cp = self.checkpoint();
        let key = self.parse_expr();
        if self.eat(TokenKind::FatArrow).is_some() {
            return Some(key);
        }
        self.restore(cp);
        None
    }

    fn parse_destructure_element(&mut self) -> DestructureElement {
        if self.at(TokenKind::Comma) {
            return DestructureElement::Skip;
        }
        let start = self.peek().span;
        let key = self.parse_destructure_key();
        if self.at(TokenKind::LBracket) {
            let target = self.parse_destructure_target();
            let span = start.to(target.span);
            return DestructureElement::Nested { key, target, span };
        }
        let ty = if self.can_start_type() {
            Some(self.parse_type())
        } else {
            let span = self.peek().span.shrink_to_start();
            self.diags.report(
                Diagnostic::error(
                    code::E_EXPECTED_TOKEN,
                    "expected a destructuring leaf's type",
                )
                .with_primary(
                    span,
                    "every destructuring leaf declares a type (ADR 0007 § 3.3)",
                ),
            );
            None
        };
        let by_ref = self.eat(TokenKind::Amp).is_some();
        let name = self.expect(TokenKind::Variable, "a destructuring leaf's name");
        let span = start.to(self.last_span);
        DestructureElement::Leaf {
            key,
            ty,
            by_ref,
            name,
            span,
        }
    }

    // ========================================================================
    // Attributes (`#[...]`)
    // ========================================================================

    /// Zero or more `#[...]` groups, in source order — the standard prefix
    /// of every declaration site (a class, a property, a method, a
    /// parameter, an enum case, ...).
    fn parse_attribute_groups(&mut self) -> Vec<AttributeGroup> {
        let mut groups = Vec::new();
        while self.at(TokenKind::AttributeOpen) {
            groups.push(self.parse_attribute_group());
        }
        groups
    }

    fn parse_attribute_group(&mut self) -> AttributeGroup {
        let start = self.bump().span; // '#['
        let mut attributes = vec![self.parse_attribute()];
        while self.eat(TokenKind::Comma).is_some() && !self.at(TokenKind::RBracket) {
            attributes.push(self.parse_attribute());
        }
        let close = self.expect(TokenKind::RBracket, "`]`");
        AttributeGroup {
            attributes,
            span: start.to(close),
        }
    }

    fn parse_attribute(&mut self) -> Attribute {
        let start = self.peek().span;
        let name = self.parse_name();
        let args = if self.at(TokenKind::LParen) {
            Some(self.parse_call_args())
        } else {
            None
        };
        let span = start.to(self.last_span);
        Attribute { name, args, span }
    }

    // ========================================================================
    // Declaration modifiers
    // ========================================================================

    /// Every modifier the parser knows, in any combination and any order —
    /// a class header, a property, a constant, a method and a parameter all
    /// call this one loop. Which modifiers make sense in which position is
    /// a later check, not a grammar rule (see [`Modifier`]'s docs).
    fn parse_modifiers(&mut self) -> Vec<Modifier> {
        let mut modifiers = Vec::new();
        loop {
            let m = match self.peek().kind {
                TokenKind::Keyword(Keyword::Public) => {
                    self.parse_visibility_modifier(Visibility::Public)
                }
                TokenKind::Keyword(Keyword::Protected) => {
                    self.parse_visibility_modifier(Visibility::Protected)
                }
                TokenKind::Keyword(Keyword::Private) => {
                    self.parse_visibility_modifier(Visibility::Private)
                }
                TokenKind::Keyword(Keyword::Readonly) => {
                    self.bump();
                    Modifier::Readonly
                }
                TokenKind::Keyword(Keyword::Static) => {
                    self.bump();
                    Modifier::Static
                }
                TokenKind::Keyword(Keyword::Abstract) => {
                    self.bump();
                    Modifier::Abstract
                }
                TokenKind::Keyword(Keyword::Final) => {
                    self.bump();
                    Modifier::Final
                }
                _ => break,
            };
            modifiers.push(m);
        }
        modifiers
    }

    /// `public`/`protected`/`private`, optionally followed by PHP 8.4's
    /// asymmetric-visibility suffix `(set)` — `private(set)` etc. — which
    /// becomes [`Modifier::SetVisibility`] instead of the plain form.
    fn parse_visibility_modifier(&mut self, v: Visibility) -> Modifier {
        self.bump();
        if self.eat(TokenKind::LParen).is_none() {
            return match v {
                Visibility::Public => Modifier::Public,
                Visibility::Protected => Modifier::Protected,
                Visibility::Private => Modifier::Private,
            };
        }
        if self.at_contextual("set") {
            self.bump();
        } else {
            self.error_expected("`set`");
        }
        self.expect(TokenKind::RParen, "`)`");
        Modifier::SetVisibility(v)
    }

    // ========================================================================
    // Shared declaration helpers
    // ========================================================================

    /// One unqualified declared name — a class, interface, trait, enum,
    /// method, constant or `type`-alias name. Unlike [`Self::parse_name`],
    /// this never admits a `\`-qualified path: nothing is ever declared
    /// under a path, only referred to by one. A keyword-shaped spelling is
    /// accepted, same as a member name after `->`/`::`.
    fn parse_decl_name(&mut self, what: &str) -> Name {
        let span = if Self::is_name_segment(self.peek().kind) {
            self.bump().span
        } else {
            self.error_expected(what)
        };
        Name { span }
    }

    /// `extends`/`implements`'s comma-separated name list — shared by every
    /// declaration that has one.
    fn parse_name_list(&mut self) -> Vec<Name> {
        let mut names = vec![self.parse_name()];
        while self.eat(TokenKind::Comma).is_some() {
            names.push(self.parse_name());
        }
        names
    }

    /// ADR 0011 § 2: `Core` is reserved for built-ins. Reports and keeps
    /// going.
    fn check_reserved_core_namespace(&mut self, name: &Name) {
        let text = self.file.span_text(name.span).unwrap_or_default();
        let first_segment = text
            .trim_start_matches('\\')
            .split('\\')
            .next()
            .unwrap_or(text);
        if first_segment.eq_ignore_ascii_case("Core") {
            self.diags.report(
                Diagnostic::error(
                    code::E_RESERVED_CORE_NAMESPACE,
                    "`Core` is reserved for built-ins",
                )
                .with_primary(name.span, "not available to user code"),
            );
        }
    }

    // ========================================================================
    // `namespace`, `use`, `type` alias — file-scope declarations
    // ========================================================================

    fn parse_namespace_decl(&mut self, start: Span) -> Stmt {
        self.bump(); // 'namespace'
        let name = if matches!(self.peek().kind, TokenKind::Ident | TokenKind::Backslash) {
            Some(self.parse_name())
        } else {
            None
        };
        if let Some(name) = &name {
            self.check_reserved_core_namespace(name);
        }
        let body = if self.at(TokenKind::LBrace) {
            Some(self.parse_block())
        } else {
            self.expect(TokenKind::Semicolon, "`;`");
            None
        };
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::NamespaceDecl(NamespaceDecl { span, name, body }),
        }
    }

    fn parse_use_decl(&mut self, start: Span) -> Stmt {
        self.bump(); // 'use'
        let path = self.parse_name();
        let alias = if self.eat_keyword(Keyword::As).is_some() {
            Some(self.expect(TokenKind::Ident, "an alias name"))
        } else {
            None
        };
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        if let Some(alias) = alias {
            self.diags.report(
                Diagnostic::error(
                    code::E_IMPORT_ALIAS_UNSUPPORTED,
                    "an import cannot be renamed",
                )
                .with_primary(alias, "rename not supported")
                .with_help(
                    "refer to it by its declared short name, or use the fully-qualified path \
                     directly (ADR 0015 § 2)",
                ),
            );
        }
        Stmt {
            span,
            kind: StmtKind::UseDecl(UseDecl { span, path, alias }),
        }
    }

    fn parse_type_alias_decl(&mut self, start: Span) -> Stmt {
        self.bump(); // 'type' (contextual — see `Self::parse_statement`)
        let name = self.parse_decl_name("a type alias name");
        self.expect(TokenKind::Equals, "`=`");
        let ty = self.parse_type();
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::TypeAliasDecl(TypeAliasDecl { span, name, ty }),
        }
    }

    // ========================================================================
    // Classes, interfaces, traits (ADR 0011 §§ 1/4, ADR 0015 § 3)
    // ========================================================================

    fn parse_class_decl(&mut self, start: Span) -> Stmt {
        self.finish_class_decl(start, Vec::new())
    }

    fn finish_class_decl(&mut self, start: Span, attributes: Vec<AttributeGroup>) -> Stmt {
        let modifiers = self.parse_modifiers();
        self.expect_keyword(Keyword::Class, "`class`");
        let name = self.parse_decl_name("a class name");
        let extends = if self.eat_keyword(Keyword::Extends).is_some() {
            Some(self.parse_name())
        } else {
            None
        };
        let implements = if self.eat_keyword(Keyword::Implements).is_some() {
            self.parse_name_list()
        } else {
            Vec::new()
        };
        let members = self.parse_class_body();
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::ClassDecl(ClassDecl {
                span,
                attributes,
                modifiers,
                name,
                extends,
                implements,
                members,
            }),
        }
    }

    fn parse_interface_decl(&mut self, start: Span) -> Stmt {
        self.finish_interface_decl(start, Vec::new())
    }

    fn finish_interface_decl(&mut self, start: Span, attributes: Vec<AttributeGroup>) -> Stmt {
        self.bump(); // 'interface'
        let name = self.parse_decl_name("an interface name");
        let extends = if self.eat_keyword(Keyword::Extends).is_some() {
            self.parse_name_list()
        } else {
            Vec::new()
        };
        let members = self.parse_class_body();
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::InterfaceDecl(InterfaceDecl {
                span,
                attributes,
                name,
                extends,
                members,
            }),
        }
    }

    fn parse_trait_decl(&mut self, start: Span) -> Stmt {
        self.finish_trait_decl(start, Vec::new())
    }

    fn finish_trait_decl(&mut self, start: Span, attributes: Vec<AttributeGroup>) -> Stmt {
        self.bump(); // 'trait'
        let name = self.parse_decl_name("a trait name");
        let members = self.parse_class_body();
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::TraitDecl(TraitDecl {
                span,
                attributes,
                name,
                members,
            }),
        }
    }

    /// A `{ ... }` class/interface/trait body. Mirrors [`Self::parse_block`]'s
    /// force-progress guard exactly, for the same reason: a malformed member
    /// must not hang the parser.
    fn parse_class_body(&mut self) -> Vec<ClassMember> {
        self.expect(TokenKind::LBrace, "`{`");
        let mut members = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let before = self.peek().span;
            self.parse_class_member(&mut members);
            if self.peek().span == before && !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "`}`");
        members
    }

    fn parse_class_member(&mut self, out: &mut Vec<ClassMember>) {
        let start = self.peek().span;
        let attributes = self.parse_attribute_groups();
        self.parse_class_member_with_attrs(start, attributes, out);
    }

    /// One or more members can come from a single source construct — a
    /// property or constant may name several declarators at once
    /// (`public int $a, $b;`) — so this pushes into `out` rather than
    /// returning a single [`ClassMember`].
    fn parse_class_member_with_attrs(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
        out: &mut Vec<ClassMember>,
    ) {
        if self.at_keyword(Keyword::Use) {
            out.push(self.parse_use_trait_member(start, attributes));
            return;
        }
        let modifiers = self.parse_modifiers();
        if self.at_keyword(Keyword::Const) {
            let consts = self.parse_const_body(&attributes, &modifiers);
            let span = start.to(self.last_span);
            for c in consts {
                out.push(ClassMember {
                    span,
                    kind: ClassMemberKind::Const(c),
                });
            }
            return;
        }
        if self.at_keyword(Keyword::Function) {
            out.push(self.parse_method_member(start, attributes, modifiers));
            return;
        }
        if self.can_start_type() {
            self.parse_property_members(start, attributes, modifiers, out);
            return;
        }
        let span = self.error_expected("a class member");
        out.push(ClassMember {
            kind: ClassMemberKind::Error,
            span,
        });
    }

    /// `const (Type)? Name = expr (',' Name = expr)*;` — `const` and the
    /// trailing `;` are both consumed here, so this is the whole
    /// declaration regardless of whether it ends up wrapped as a class
    /// member or (with empty `modifiers`) rejected as a top-level `const`.
    fn parse_const_body(
        &mut self,
        attributes: &[AttributeGroup],
        modifiers: &[Modifier],
    ) -> Vec<ConstMember> {
        self.bump(); // 'const'
        let ty = if self.can_start_type() && !self.at_const_name_without_type() {
            Some(self.parse_type())
        } else {
            None
        };
        let mut members = Vec::new();
        loop {
            let name = self.parse_decl_name("a constant name").span;
            self.expect(TokenKind::Equals, "`=`");
            let value = self.parse_expr();
            members.push(ConstMember {
                attributes: attributes.to_vec(),
                modifiers: modifiers.to_vec(),
                ty: ty.clone(),
                name,
                value,
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::Semicolon, "`;`");
        members
    }

    /// Whether `const` is immediately followed by `Name '='` — PHP 8.3's
    /// untyped spelling — rather than a type. A bare `Ident` here is
    /// ambiguous with a class-name type atom; the deciding token is
    /// whether `=` follows it directly.
    fn at_const_name_without_type(&mut self) -> bool {
        matches!(self.peek().kind, TokenKind::Ident) && self.peek_at(1).kind == TokenKind::Equals
    }

    fn parse_method_member(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
        modifiers: Vec<Modifier>,
    ) -> ClassMember {
        let method = self.parse_method_body(attributes, modifiers);
        let span = start.to(self.last_span);
        ClassMember {
            span,
            kind: ClassMemberKind::Method(method),
        }
    }

    /// `function '&'? name(params) (: ReturnType)? (block | ';')` —
    /// `function` itself consumed here, exactly like
    /// [`Self::parse_const_body`] consumes `const`.
    fn parse_method_body(
        &mut self,
        attributes: Vec<AttributeGroup>,
        modifiers: Vec<Modifier>,
    ) -> MethodMember {
        self.bump(); // 'function'
        let by_ref = self.eat(TokenKind::Amp).is_some();
        let name = self.parse_decl_name("a method name").span;
        let params = self.parse_params();
        let return_type = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type())
        } else {
            None
        };
        let body = if self.at(TokenKind::LBrace) {
            Some(self.parse_block())
        } else {
            self.expect(TokenKind::Semicolon, "`;`");
            None
        };
        MethodMember {
            attributes,
            modifiers,
            by_ref,
            name,
            params,
            return_type,
            body,
        }
    }

    /// `Type '$'name (',' '$'name)* ';'`, or the single-declarator hooked
    /// form `Type '$'name '{' hooks '}'` (PHP 8.4 property hooks, feeding
    /// `PropertyObserver` — ADR 0014). A hooked property is never part of a
    /// comma list — real PHP requires it declared alone — so the hooked
    /// branch returns as soon as it is taken.
    fn parse_property_members(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
        modifiers: Vec<Modifier>,
        out: &mut Vec<ClassMember>,
    ) {
        let ty = self.parse_type();
        loop {
            let name = self.expect(TokenKind::Variable, "a property name");
            if self.at(TokenKind::LBrace) {
                let hooks = self.parse_property_hooks();
                let span = start.to(self.last_span);
                out.push(ClassMember {
                    span,
                    kind: ClassMemberKind::Property(PropertyMember {
                        attributes,
                        modifiers,
                        ty,
                        name,
                        default: None,
                        hooks: Some(hooks),
                    }),
                });
                return;
            }
            let default = self.eat(TokenKind::Equals).map(|_| self.parse_expr());
            let span = start.to(self.last_span);
            out.push(ClassMember {
                span,
                kind: ClassMemberKind::Property(PropertyMember {
                    attributes: attributes.clone(),
                    modifiers: modifiers.clone(),
                    ty: ty.clone(),
                    name,
                    default,
                    hooks: None,
                }),
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::Semicolon, "`;`");
    }

    /// `{ hook+ }` — PHP 8.4's property-hook block, kept exactly as PHP has
    /// it (ADR 0014 § 1: this ADR "adds no new syntax beyond an ordinary
    /// interface declaration"). Mirrors [`Self::parse_block`]'s
    /// force-progress guard.
    fn parse_property_hooks(&mut self) -> Vec<PropertyHook> {
        self.expect(TokenKind::LBrace, "`{`");
        let mut hooks = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let before = self.peek().span;
            hooks.push(self.parse_property_hook());
            if self.peek().span == before && !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "`}`");
        hooks
    }

    fn parse_property_hook(&mut self) -> PropertyHook {
        let start = self.peek().span;
        let attributes = self.parse_attribute_groups();
        let by_ref = self.eat(TokenKind::Amp).is_some();
        let kind = if self.at_contextual("set") {
            self.bump();
            PropertyHookKind::Set
        } else {
            if self.at_contextual("get") {
                self.bump();
            } else {
                self.error_expected("`get` or `set`");
            }
            PropertyHookKind::Get
        };
        let param = if kind == PropertyHookKind::Set && self.at(TokenKind::LParen) {
            Some(self.parse_hook_param())
        } else {
            None
        };
        let body = if self.eat(TokenKind::FatArrow).is_some() {
            let e = self.parse_expr();
            self.expect(TokenKind::Semicolon, "`;`");
            Some(PropertyHookBody::Expr(Box::new(e)))
        } else if self.at(TokenKind::LBrace) {
            Some(PropertyHookBody::Block(self.parse_block()))
        } else {
            self.expect(TokenKind::Semicolon, "`;`");
            None
        };
        let span = start.to(self.last_span);
        PropertyHook {
            span,
            attributes,
            kind,
            by_ref,
            param,
            body,
        }
    }

    /// `'(' Type? '$'name ')'` — a `set` hook's parameter. Unlike an
    /// ordinary [`Self::parse_param`], the type may be omitted with no
    /// diagnostic: PHP 8.4 infers it from the property's own declared type.
    fn parse_hook_param(&mut self) -> Param {
        let start = self.expect(TokenKind::LParen, "`(`");
        let attributes = self.parse_attribute_groups();
        let ty = if self.can_start_type() {
            Some(self.parse_type())
        } else {
            None
        };
        let name = self.expect(TokenKind::Variable, "the hook's parameter name");
        let close = self.expect(TokenKind::RParen, "`)`");
        Param {
            span: start.to(close),
            attributes,
            modifiers: Vec::new(),
            ty,
            by_ref: false,
            variadic: false,
            name,
            default: None,
        }
    }

    /// `use Trait, Trait2 (';' | '{' adaptations '}')` inside a class/trait
    /// body. Nothing in PHP's grammar, or any ADR, gives this an attribute
    /// position, so `attributes` (parsed uniformly by the caller before
    /// dispatching on `use`) is simply unused here.
    fn parse_use_trait_member(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
    ) -> ClassMember {
        let _ = attributes;
        self.bump(); // 'use'
        let traits = self.parse_name_list();
        let adaptations = if self.at(TokenKind::LBrace) {
            self.parse_trait_adaptations()
        } else {
            self.expect(TokenKind::Semicolon, "`;`");
            Vec::new()
        };
        let span = start.to(self.last_span);
        ClassMember {
            span,
            kind: ClassMemberKind::UseTrait(UseTraitMember {
                traits,
                adaptations,
            }),
        }
    }

    fn parse_trait_adaptations(&mut self) -> Vec<TraitAdaptation> {
        self.expect(TokenKind::LBrace, "`{`");
        let mut adaptations = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let before = self.peek().span;
            adaptations.push(self.parse_trait_adaptation());
            if self.peek().span == before && !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "`}`");
        adaptations
    }

    /// `(Trait '::')? method` — the `Trait::` qualifier is required for
    /// `insteadof` (it names which trait's method wins) but optional for a
    /// visibility-only `as` clause, where a single trait already makes the
    /// method unambiguous.
    fn parse_trait_method_ref(&mut self) -> TraitMethodRef {
        let first = self.parse_name();
        if self.eat(TokenKind::DoubleColon).is_some() {
            let method = self.expect_name_segment();
            TraitMethodRef {
                trait_name: Some(first),
                method,
            }
        } else {
            TraitMethodRef {
                trait_name: None,
                method: first.span,
            }
        }
    }

    fn parse_trait_adaptation(&mut self) -> TraitAdaptation {
        let start = self.peek().span;
        let method = self.parse_trait_method_ref();
        let kind = if self.eat_keyword(Keyword::Insteadof).is_some() {
            let over = self.parse_name_list();
            TraitAdaptationKind::InsteadOf { method, over }
        } else {
            self.expect_keyword(Keyword::As, "`insteadof` or `as`");
            let visibility = match self.peek().kind {
                TokenKind::Keyword(Keyword::Public) => {
                    self.bump();
                    Some(Modifier::Public)
                }
                TokenKind::Keyword(Keyword::Protected) => {
                    self.bump();
                    Some(Modifier::Protected)
                }
                TokenKind::Keyword(Keyword::Private) => {
                    self.bump();
                    Some(Modifier::Private)
                }
                _ => None,
            };
            let new_name = if Self::is_name_segment(self.peek().kind) {
                Some(self.bump().span)
            } else {
                None
            };
            TraitAdaptationKind::As {
                method,
                visibility,
                new_name,
            }
        };
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        if let TraitAdaptationKind::As {
            visibility,
            new_name,
            ..
        } = &kind
        {
            self.report_trait_as_rejection(span, visibility.is_some(), new_name.is_some());
        }
        TraitAdaptation { kind, span }
    }

    /// ADR 0015 § 3: both forms of a trait `use` block's `as` clause are
    /// rejected — renaming a method, and changing its visibility alone —
    /// while `insteadof` is kept. A rename takes priority in the message
    /// when both parts are written at once.
    fn report_trait_as_rejection(&mut self, span: Span, has_visibility: bool, has_rename: bool) {
        if has_rename {
            self.diags.report(
                Diagnostic::error(
                    code::E_TRAIT_METHOD_RENAME_UNSUPPORTED,
                    "a trait method cannot be renamed",
                )
                .with_primary(span, "rename not supported")
                .with_help(
                    "give the class its own method with the new name, calling the trait's \
                     method explicitly (ADR 0015 § 3)",
                ),
            );
        } else if has_visibility {
            self.diags.report(
                Diagnostic::error(
                    code::E_TRAIT_METHOD_VISIBILITY_UNSUPPORTED,
                    "a trait method's visibility cannot be changed by `as`",
                )
                .with_primary(span, "visibility change not supported")
                .with_help(
                    "override the method in the class with the visibility you want \
                     (ADR 0015 § 3)",
                ),
            );
        }
    }

    // ========================================================================
    // Enums (ADR 0010)
    // ========================================================================

    fn parse_enum_decl(&mut self, start: Span) -> Stmt {
        self.finish_enum_decl(start, Vec::new())
    }

    fn finish_enum_decl(&mut self, start: Span, attributes: Vec<AttributeGroup>) -> Stmt {
        self.bump(); // 'enum'
        let name = self.parse_decl_name("an enum name");
        let backing = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_enum_backing_type())
        } else {
            None
        };
        let implements = if self.eat_keyword(Keyword::Implements).is_some() {
            self.parse_name_list()
        } else {
            Vec::new()
        };
        if let (Some(first), Some(last)) = (implements.first(), implements.last()) {
            let span = first.span.to(last.span);
            self.diags.report(
                Diagnostic::error(
                    code::E_ENUM_IMPLEMENTS_UNSUPPORTED,
                    "an enum cannot implement an interface",
                )
                .with_primary(
                    span,
                    "an enum declares only cases and an optional backing type",
                )
                .with_help(
                    "give the enum's consumer a `static` method on some other class instead \
                     (ADR 0010 § 3)",
                ),
            );
        }
        let (cases, members) = self.parse_enum_body();
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::EnumDecl(EnumDecl {
                span,
                attributes,
                name,
                backing,
                implements,
                cases,
                members,
            }),
        }
    }

    /// The `: Type` backing-type clause, parsed with the full ADR 0007 § 3
    /// grammar — only the one rejection ADR 0010 § 3 names explicitly
    /// (`string`) is checked here; that the result is otherwise exactly
    /// `int` or `uint` is a later check, not the parser's.
    fn parse_enum_backing_type(&mut self) -> Type {
        let ty = self.parse_type();
        if matches!(ty.kind, TypeKind::Atom(TypeAtom::String)) {
            self.diags.report(
                Diagnostic::error(
                    code::E_ENUM_STRING_BACKING_UNSUPPORTED,
                    "an enum cannot be backed by `string`",
                )
                .with_primary(ty.span, "only `int`/`uint` back an enum")
                .with_help("use `: int` or `: uint`, or omit the backing type (ADR 0010 § 3)"),
            );
        }
        ty
    }

    /// An enum body mixes cases (bare names) with, if the input is
    /// malformed, member-shaped constructs that ADR 0010 § 3 rejects
    /// outright — a method, a property, a constant, a trait `use`. Both are
    /// parsed, since attributes may precede either and only the token after
    /// them tells them apart; mirrors [`Self::parse_block`]'s force-progress
    /// guard.
    fn parse_enum_body(&mut self) -> (Vec<EnumCase>, Vec<ClassMember>) {
        self.expect(TokenKind::LBrace, "`{`");
        let mut cases = Vec::new();
        let mut members = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let before = self.peek().span;
            let attributes = self.parse_attribute_groups();
            if matches!(self.peek().kind, TokenKind::Ident) {
                cases.push(self.finish_enum_case(before, attributes));
                self.eat(TokenKind::Comma);
            } else {
                self.parse_class_member_with_attrs(before, attributes, &mut members);
                let span = before.to(self.last_span);
                self.diags.report(
                    Diagnostic::error(
                        code::E_ENUM_MEMBER_UNSUPPORTED,
                        "an enum declares only cases and an optional backing type",
                    )
                    .with_primary(span, "not a case")
                    .with_help("move this to a separate class (ADR 0010 § 3)"),
                );
            }
            if self.peek().span == before && !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "`}`");
        (cases, members)
    }

    fn finish_enum_case(&mut self, start: Span, attributes: Vec<AttributeGroup>) -> EnumCase {
        let name = self.parse_decl_name("a case name");
        let value = self.eat(TokenKind::Equals).map(|_| self.parse_expr());
        let span = start.to(self.last_span);
        EnumCase {
            span,
            attributes,
            name,
            value,
        }
    }

    // ========================================================================
    // The statement-shaped rejects: a top-level `function`/`const`
    // (ADR 0011 § 1)
    // ========================================================================

    /// `#[...]` groups precede a class/interface/trait/enum declaration, a
    /// rejected top-level `function`/`const`, or nothing this parser
    /// recognizes yet — decided by the keyword that follows them.
    fn parse_attributed_decl_stmt(&mut self, start: Span) -> Stmt {
        let attributes = self.parse_attribute_groups();
        match self.peek().kind {
            TokenKind::Keyword(Keyword::Abstract | Keyword::Final | Keyword::Class) => {
                self.finish_class_decl(start, attributes)
            }
            TokenKind::Keyword(Keyword::Interface) => self.finish_interface_decl(start, attributes),
            TokenKind::Keyword(Keyword::Trait) => self.finish_trait_decl(start, attributes),
            TokenKind::Keyword(Keyword::Enum) => self.finish_enum_decl(start, attributes),
            TokenKind::Keyword(Keyword::Const) => {
                self.parse_toplevel_const_reject(start, attributes)
            }
            TokenKind::Keyword(Keyword::Function) if self.at_named_function_decl() => {
                self.parse_toplevel_function_reject(start, attributes)
            }
            _ => {
                self.error_expected("a declaration after `#[...]`");
                self.parse_statement()
            }
        }
    }

    /// Whether `function` at the current position starts a rejected
    /// top-level declaration (`function foo() { ... }`) rather than an
    /// anonymous closure used as a bare expression statement
    /// (`function () { ... };`) — decided by whether a name, not `(`,
    /// follows, skipping an optional by-reference `&`.
    fn at_named_function_decl(&mut self) -> bool {
        let idx = if self.peek_at(1).kind == TokenKind::Amp {
            2
        } else {
            1
        };
        Self::is_name_segment(self.peek_at(idx).kind)
    }

    fn parse_toplevel_function_reject(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
    ) -> Stmt {
        let method = self.parse_method_body(attributes, Vec::new());
        let span = start.to(self.last_span);
        self.diags.report(
            Diagnostic::error(
                code::E_TOPLEVEL_FUNCTION_UNSUPPORTED,
                "a function must be a method",
            )
            .with_primary(span, "not inside any class")
            .with_help("wrap it in a class as `public static function` (ADR 0011 § 1)"),
        );
        Stmt {
            span,
            kind: StmtKind::TopLevelFunction(method),
        }
    }

    fn parse_toplevel_const_reject(
        &mut self,
        start: Span,
        attributes: Vec<AttributeGroup>,
    ) -> Stmt {
        let consts = self.parse_const_body(&attributes, &[]);
        let span = start.to(self.last_span);
        self.diags.report(
            Diagnostic::error(
                code::E_TOPLEVEL_CONST_UNSUPPORTED,
                "a constant must belong to a class",
            )
            .with_primary(span, "not inside any class")
            .with_help("declare it `public const` on the class it belongs to (ADR 0011 § 1)"),
        );
        Stmt {
            span,
            kind: StmtKind::TopLevelConst(consts),
        }
    }
}

fn is_assignable(e: &Expr) -> bool {
    matches!(
        e.kind,
        ExprKind::Variable(_)
            | ExprKind::Index { .. }
            | ExprKind::PropertyAccess { .. }
            | ExprKind::StaticPropertyAccess { .. }
            | ExprKind::Error
    )
}

fn collapse_string_parts(span: Span, parts: Vec<StringPart>) -> Expr {
    let all_text = parts.iter().all(|p| matches!(p, StringPart::Text(_)));
    if all_text {
        Expr {
            span,
            kind: ExprKind::Str(span),
        }
    } else {
        Expr {
            span,
            kind: ExprKind::Interpolated(parts),
        }
    }
}

/// Parses a single expression from `file`, for tests and tools that want just
/// that much. Trailing tokens after the expression are left unconsumed.
#[must_use]
pub fn parse_expression(file: &SourceFile, diags: &mut Diagnostics) -> Expr {
    Parser::new(file, diags).parse_expr()
}

/// Parses a whole file top to bottom: the `mwl ast`/corpus-parse entry point
/// M1's plan names, and the one place the file always starts in HTML mode
/// (spec `00-overview.md` § 1) rather than a test's manually-bumped open tag.
/// Errors are reported into `diags` rather than stopping the parse — callers
/// that only care whether it parsed cleanly should check
/// [`Diagnostics::has_errors`] afterwards.
#[must_use]
pub fn parse_file(file: &SourceFile, diags: &mut Diagnostics) -> Vec<Stmt> {
    let mut parser = Parser::new(file, diags);
    let mut stmts = Vec::new();
    while !parser.at(TokenKind::Eof) {
        let before = parser.peek().span;
        stmts.push(parser.parse_statement());
        // Mirrors `parse_block`'s guard: if a statement consumed nothing (a
        // production bailed out on an error), force progress so a malformed
        // file can't hang the parser in an infinite loop.
        if parser.peek().span == before && !parser.at(TokenKind::Eof) {
            parser.bump();
        }
    }
    stmts
}

#[cfg(test)]
mod tests {
    use mwl_diagnostics::SourceMap;

    use super::*;

    /// Parses `src` as an expression (wrapped in `<?mwl `) and asserts no
    /// diagnostics were reported.
    fn parse_ok(src: &str) -> Expr {
        let mut map = SourceMap::new();
        let id = map.add("t.mwl", format!("<?mwl {src}"));
        let mut diags = Diagnostics::new();
        let mut p = Parser::new(map.file(id), &mut diags);
        p.bump(); // OpenTagMwl
        let e = p.parse_expr();
        assert!(
            !diags.has_errors(),
            "unexpected diagnostics for {src:?}: {diags:?}"
        );
        e
    }

    /// Parses `src` as an expression and returns it along with whatever
    /// diagnostics were reported, for tests that expect a reported error.
    fn parse_with_diags(src: &str) -> (Expr, Diagnostics) {
        let mut map = SourceMap::new();
        let id = map.add("t.mwl", format!("<?mwl {src}"));
        let mut diags = Diagnostics::new();
        let mut p = Parser::new(map.file(id), &mut diags);
        p.bump(); // OpenTagMwl
        let e = p.parse_expr();
        (e, diags)
    }

    fn text(map: &SourceMap, file: mwl_diagnostics::SourceId, span: Span) -> &str {
        map.get(file).and_then(|f| f.span_text(span)).unwrap_or("")
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        let e = parse_ok("1 + 2 * 3");
        let ExprKind::Binary {
            op: BinaryOp::Add,
            lhs,
            rhs,
        } = e.kind
        else {
            panic!("expected a top-level `+`: {e:?}");
        };
        assert!(matches!(lhs.kind, ExprKind::Int(_)));
        assert!(matches!(
            rhs.kind,
            ExprKind::Binary {
                op: BinaryOp::Mul,
                ..
            }
        ));
    }

    #[test]
    fn power_is_right_associative() {
        let e = parse_ok("2 ** 3 ** 2");
        let ExprKind::Binary {
            op: BinaryOp::Pow,
            lhs,
            rhs,
        } = e.kind
        else {
            panic!("expected `**`: {e:?}");
        };
        assert!(matches!(lhs.kind, ExprKind::Int(_)));
        assert!(matches!(
            rhs.kind,
            ExprKind::Binary {
                op: BinaryOp::Pow,
                ..
            }
        ));
    }

    #[test]
    fn power_binds_tighter_than_unary_minus() {
        // `-2 ** 2` is `-(2 ** 2)`, PHP's own rule.
        let e = parse_ok("-2 ** 2");
        let ExprKind::Unary {
            op: UnaryOp::Neg,
            expr,
        } = e.kind
        else {
            panic!("expected unary `-`: {e:?}");
        };
        assert!(matches!(
            expr.kind,
            ExprKind::Binary {
                op: BinaryOp::Pow,
                ..
            }
        ));
    }

    #[test]
    fn as_conversion_binds_tighter_than_any_binary_operator() {
        // ADR 0007 § 2: `$a as int + 1` is `($a as int) + 1`.
        let e = parse_ok("$a as int + 1");
        let ExprKind::Binary {
            op: BinaryOp::Add,
            lhs,
            rhs,
        } = e.kind
        else {
            panic!("expected a top-level `+`: {e:?}");
        };
        assert!(matches!(lhs.kind, ExprKind::Conversion { .. }));
        assert!(matches!(rhs.kind, ExprKind::Int(_)));
    }

    #[test]
    fn assignment_is_right_associative() {
        let e = parse_ok("$a = $b = 1");
        let ExprKind::Assign {
            op: AssignOp::Assign,
            target,
            value,
            ..
        } = e.kind
        else {
            panic!("expected `=`: {e:?}");
        };
        assert!(matches!(target.kind, ExprKind::Variable(_)));
        assert!(matches!(
            value.kind,
            ExprKind::Assign {
                op: AssignOp::Assign,
                ..
            }
        ));
    }

    #[test]
    fn invalid_assignment_target_is_diagnosed_but_still_parses() {
        let (e, diags) = parse_with_diags("1 = 2");
        assert!(diags.has_errors());
        assert!(matches!(e.kind, ExprKind::Assign { .. }));
    }

    #[test]
    fn reference_assignment_sets_the_by_ref_flag() {
        let e = parse_ok("$a = &$b");
        let ExprKind::Assign {
            op: AssignOp::Assign,
            by_ref,
            ..
        } = e.kind
        else {
            panic!("expected `=`: {e:?}");
        };
        assert!(by_ref);
    }

    #[test]
    fn compound_assignment_has_no_reference_form() {
        // `+=&` is not PHP syntax; a plain `+=` never sets `by_ref` even
        // though the RHS could, on its own, start with a legal expression.
        let e = parse_ok("$a += $b");
        let ExprKind::Assign {
            op: AssignOp::AddAssign,
            by_ref,
            ..
        } = e.kind
        else {
            panic!("expected `+=`: {e:?}");
        };
        assert!(!by_ref);
    }

    #[test]
    fn not_nests_inside_a_cast_and_other_unary_operators() {
        // `!` sits at a looser precedence tier than a cast or unary op in the
        // grammar ([`Parser::parse_not`]), but a cast/unary op's operand
        // recurses straight into [`Parser::parse_unary`], skipping that
        // tier — so without `parse_unary`'s own `Bang` arm, these would fail
        // to parse at all rather than nesting the way PHP accepts.
        let e = parse_ok("(int) !$x");
        let ExprKind::Cast { expr, .. } = e.kind else {
            panic!("expected a cast: {e:?}");
        };
        assert!(matches!(
            expr.kind,
            ExprKind::Unary {
                op: UnaryOp::Not,
                ..
            }
        ));

        let e = parse_ok("-!$x");
        let ExprKind::Unary {
            op: UnaryOp::Neg,
            expr,
        } = e.kind
        else {
            panic!("expected unary `-`: {e:?}");
        };
        assert!(matches!(
            expr.kind,
            ExprKind::Unary {
                op: UnaryOp::Not,
                ..
            }
        ));
    }

    #[test]
    fn ternary_and_elvis() {
        let e = parse_ok("$a ? $b : $c");
        let ExprKind::Ternary { then, .. } = e.kind else {
            panic!("expected ternary: {e:?}");
        };
        assert!(then.is_some());

        let e = parse_ok("$a ?: $c");
        let ExprKind::Ternary { then, .. } = e.kind else {
            panic!("expected elvis ternary: {e:?}");
        };
        assert!(then.is_none());
    }

    #[test]
    fn coalesce_is_right_associative() {
        let e = parse_ok("$a ?? $b ?? $c");
        let ExprKind::Binary {
            op: BinaryOp::Coalesce,
            rhs,
            ..
        } = e.kind
        else {
            panic!("expected `??`: {e:?}");
        };
        assert!(matches!(
            rhs.kind,
            ExprKind::Binary {
                op: BinaryOp::Coalesce,
                ..
            }
        ));
    }

    #[test]
    fn method_call_index_and_property_chain() {
        // $obj->foo()->bar[0]
        let e = parse_ok("$obj->foo()->bar[0]");
        let ExprKind::Index { base, index } = e.kind else {
            panic!("expected an index: {e:?}");
        };
        assert!(index.is_some());
        let ExprKind::PropertyAccess {
            object,
            property: MemberName::Ident(_),
            ..
        } = base.kind
        else {
            panic!("expected a property access: {base:?}");
        };
        assert!(matches!(object.kind, ExprKind::MethodCall { .. }));
    }

    #[test]
    fn nullsafe_method_call() {
        let e = parse_ok("$obj?->foo()");
        assert!(matches!(
            e.kind,
            ExprKind::MethodCall { nullsafe: true, .. }
        ));
    }

    #[test]
    fn static_access_forms() {
        assert!(matches!(
            parse_ok("Foo::BAR").kind,
            ExprKind::ClassConstAccess { .. }
        ));
        assert!(matches!(
            parse_ok("Foo::bar()").kind,
            ExprKind::StaticCall { .. }
        ));
        assert!(matches!(
            parse_ok("Foo::class").kind,
            ExprKind::ClassNameConst { .. }
        ));
        assert!(matches!(
            parse_ok("Foo::$prop").kind,
            ExprKind::StaticPropertyAccess { .. }
        ));
    }

    #[test]
    fn new_with_args_and_dynamic_class() {
        let e = parse_ok("new Foo(1, 2)");
        let ExprKind::New {
            target: NewTarget::Name(_),
            args: CallArgs::List(args),
        } = e.kind
        else {
            panic!("expected `new Foo(1, 2)`: {e:?}");
        };
        assert_eq!(args.len(), 2);

        // The trailing `()` must belong to `new`, not to `$cls`.
        let e = parse_ok("new $cls()");
        assert!(matches!(
            e.kind,
            ExprKind::New {
                target: NewTarget::Expr(_),
                ..
            }
        ));
    }

    #[test]
    fn nested_array_generic_closes_through_a_split_shift_token() {
        // `>>` must split into two `>` closes, one per nesting level.
        let e = parse_ok("$m as array<array<uint>>");
        let ExprKind::Conversion { ty, .. } = e.kind else {
            panic!("expected a conversion: {e:?}");
        };
        let TypeKind::Atom(TypeAtom::Array(Some(inner))) = ty.kind else {
            panic!("expected `array<...>`: {ty:?}");
        };
        assert!(matches!(
            inner.kind,
            TypeKind::Atom(TypeAtom::Array(Some(_)))
        ));
    }

    #[test]
    fn union_and_intersection_types() {
        let e = parse_ok("$m as int|string");
        let ExprKind::Conversion { ty, .. } = e.kind else {
            panic!("expected a conversion: {e:?}");
        };
        let TypeKind::Union(members) = ty.kind else {
            panic!("expected a union: {ty:?}");
        };
        assert_eq!(members.len(), 2);

        let e = parse_ok("$m as A&B");
        let ExprKind::Conversion { ty, .. } = e.kind else {
            panic!("expected a conversion: {e:?}");
        };
        assert!(matches!(ty.kind, TypeKind::Intersection(_)));
    }

    #[test]
    fn legacy_cast_carries_as_semantics() {
        let e = parse_ok("(int)$x");
        assert!(matches!(
            e.kind,
            ExprKind::Cast {
                ty: CastType::Int,
                ..
            }
        ));
    }

    #[test]
    fn match_expression() {
        let e = parse_ok("match ($x) { 1, 2 => 'a', default => 'b' }");
        let ExprKind::Match { arms, .. } = e.kind else {
            panic!("expected match: {e:?}");
        };
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0].conditions.as_ref().map(Vec::len), Some(2));
        assert!(arms[1].conditions.is_none());
    }

    #[test]
    fn closure_with_use_and_return_type() {
        let e = parse_ok("function (int $x) use (&$y): int { return $x + $y; }");
        let ExprKind::Closure(c) = e.kind else {
            panic!("expected a closure: {e:?}");
        };
        assert_eq!(c.params.len(), 1);
        assert!(c.params[0].ty.is_some());
        assert_eq!(c.uses.len(), 1);
        assert!(c.uses[0].by_ref);
        assert!(c.return_type.is_some());
        assert_eq!(c.body.stmts.len(), 1);
    }

    #[test]
    fn arrow_fn_captures_by_expression() {
        let e = parse_ok("fn (int $x): int => $x + $y");
        let ExprKind::ArrowFn(f) = e.kind else {
            panic!("expected an arrow function: {e:?}");
        };
        assert_eq!(f.params.len(), 1);
        assert!(matches!(
            f.body.kind,
            ExprKind::Binary {
                op: BinaryOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn missing_parameter_type_is_diagnosed() {
        let (e, diags) = parse_with_diags("fn ($x) => $x");
        assert!(diags.has_errors());
        let ExprKind::ArrowFn(f) = e.kind else {
            panic!("expected an arrow function: {e:?}");
        };
        assert!(f.params[0].ty.is_none());
    }

    #[test]
    fn first_class_callable_syntax() {
        let e = parse_ok("strlen(...)");
        assert!(matches!(
            e.kind,
            ExprKind::Call {
                args: CallArgs::FirstClassCallable,
                ..
            }
        ));
    }

    #[test]
    fn named_and_spread_arguments() {
        let e = parse_ok("foo(x: 1, ...$rest)");
        let ExprKind::Call {
            args: CallArgs::List(args),
            ..
        } = e.kind
        else {
            panic!("expected a call: {e:?}");
        };
        assert_eq!(args.len(), 2);
        assert!(args[0].name.is_some());
        assert!(args[1].spread);
    }

    #[test]
    fn array_literal_with_key_spread_and_by_ref() {
        let e = parse_ok("['a' => 1, &$x, ...$rest]");
        let ExprKind::ArrayLiteral(items) = e.kind else {
            panic!("expected an array literal: {e:?}");
        };
        assert_eq!(items.len(), 3);
        assert!(items[0].key.is_some());
        assert!(items[1].by_ref);
        assert!(items[2].spread);
    }

    #[test]
    fn double_quoted_string_without_interpolation_collapses_to_a_plain_literal() {
        assert!(matches!(parse_ok(r#""plain text""#).kind, ExprKind::Str(_)));
    }

    #[test]
    fn double_quoted_string_with_interpolation() {
        let e = parse_ok(r#""a $name->prop b""#);
        let ExprKind::Interpolated(parts) = e.kind else {
            panic!("expected an interpolated string: {e:?}");
        };
        assert_eq!(parts.len(), 3);
        assert!(matches!(parts[0], StringPart::Text(_)));
        assert!(matches!(parts[1], StringPart::Expr(_)));
        assert!(matches!(parts[2], StringPart::Text(_)));
        let StringPart::Expr(inner) = &parts[1] else {
            unreachable!()
        };
        assert!(matches!(inner.kind, ExprKind::PropertyAccess { .. }));
    }

    #[test]
    fn spawn_script_with_options() {
        let e = parse_ok("spawn script 'jobs/report.mwl' with(args: $a, grants: $g)");
        let ExprKind::SpawnScript { options, .. } = e.kind else {
            panic!("expected spawn script: {e:?}");
        };
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].key, SpawnOptionKey::Args);
        assert_eq!(options[1].key, SpawnOptionKey::Grants);
    }

    #[test]
    fn spawn_script_without_with_clause() {
        let e = parse_ok("spawn script 'jobs/report.mwl'");
        let ExprKind::SpawnScript { options, .. } = e.kind else {
            panic!("expected spawn script: {e:?}");
        };
        assert!(options.is_empty());
    }

    #[test]
    fn spawn_and_script_are_still_plain_identifiers_elsewhere() {
        // `spawn` alone (no following `script`) must stay an ordinary
        // constant-fetch name, not misfire the `spawn script` production.
        assert!(matches!(parse_ok("spawn").kind, ExprKind::ConstFetch(_)));
    }

    #[test]
    fn alt_colon_syntax_end_words_are_plain_identifiers() {
        // PHP's alternative colon syntax (`if (...): ... endif;`) is deliberately
        // out of scope; `endif`/`endfor`/`endforeach`/`endswitch`/`endwhile`/
        // `enddeclare` must not be reserved words.
        for name in [
            "endif",
            "endfor",
            "endforeach",
            "endswitch",
            "endwhile",
            "enddeclare",
        ] {
            assert!(matches!(parse_ok(name).kind, ExprKind::ConstFetch(_)));
        }
    }

    #[test]
    fn yield_forms() {
        assert!(matches!(
            parse_ok("fn () => yield").kind,
            ExprKind::ArrowFn(_)
        ));

        let e = parse_ok("yield $x");
        assert!(matches!(
            e.kind,
            ExprKind::Yield {
                key: None,
                value: Some(_)
            }
        ));

        let e = parse_ok("yield $k => $v");
        assert!(matches!(
            e.kind,
            ExprKind::Yield {
                key: Some(_),
                value: Some(_)
            }
        ));

        let e = parse_ok("yield from $gen");
        assert!(matches!(e.kind, ExprKind::YieldFrom(_)));
    }

    #[test]
    fn isset_and_empty() {
        let e = parse_ok("isset($a, $b)");
        let ExprKind::Isset(args) = e.kind else {
            panic!("expected isset: {e:?}");
        };
        assert_eq!(args.len(), 2);

        assert!(matches!(parse_ok("empty($a)").kind, ExprKind::Empty(_)));
    }

    #[test]
    fn name_span_covers_the_qualified_name() {
        let mut map = SourceMap::new();
        let id = map.add("t.mwl", "<?mwl Core\\Bytes::fromHex('ab')");
        let mut diags = Diagnostics::new();
        let mut p = Parser::new(map.file(id), &mut diags);
        p.bump();
        let e = p.parse_expr();
        assert!(!diags.has_errors());
        let ExprKind::StaticCall { class, .. } = e.kind else {
            panic!("expected a static call: {e:?}");
        };
        let ExprKind::ConstFetch(name) = class.kind else {
            panic!("expected a name: {class:?}");
        };
        assert_eq!(text(&map, id, name.span), "Core\\Bytes");
    }

    #[test]
    fn static_closure_modifier_is_diagnosed_but_still_parses() {
        let (e, diags) = parse_with_diags("static function () { return 1; }");
        assert!(diags.has_errors());
        let ExprKind::Closure(c) = e.kind else {
            panic!("expected a closure: {e:?}");
        };
        assert!(c.is_static);
    }

    #[test]
    fn static_arrow_fn_modifier_is_diagnosed_but_still_parses() {
        let (e, diags) = parse_with_diags("static fn ($x) => $x");
        assert!(diags.has_errors());
        let ExprKind::ArrowFn(f) = e.kind else {
            panic!("expected an arrow function: {e:?}");
        };
        assert!(f.is_static);
    }

    #[test]
    fn ordinary_closure_is_not_diagnosed() {
        parse_ok("function () { return 1; }");
        parse_ok("fn (int $x) => $x");
    }

    #[test]
    fn superglobals_are_diagnosed_but_still_parse_as_variables() {
        for name in [
            "$GLOBALS",
            "$_REQUEST",
            "$_GET",
            "$_POST",
            "$_COOKIE",
            "$_FILES",
            "$_SERVER",
            "$_SESSION",
            "$_ENV",
            "$argv",
            "$argc",
            "$_ARGS",
        ] {
            let (e, diags) = parse_with_diags(name);
            assert!(diags.has_errors(), "expected a diagnostic for {name}");
            assert!(matches!(e.kind, ExprKind::Variable(_)), "for {name}");
        }
    }

    #[test]
    fn an_ordinary_variable_is_not_mistaken_for_a_superglobal() {
        parse_ok("$_getter");
        parse_ok("$server");
        parse_ok("$globals");
    }

    #[test]
    fn dollar_dollar_name_is_variable_variable() {
        let (e, diags) = parse_with_diags("$$name");
        assert!(diags.has_errors());
        assert!(matches!(e.kind, ExprKind::Error));
    }

    #[test]
    fn dollar_brace_expr_is_variable_variable() {
        let (e, diags) = parse_with_diags("${$name}");
        assert!(diags.has_errors());
        assert!(matches!(e.kind, ExprKind::Error));
    }

    // ========================================================================
    // Chunk 2: statements
    // ========================================================================

    /// Parses `src` as one statement (wrapped in `<?mwl `) and asserts no
    /// diagnostics were reported.
    fn parse_stmt_ok(src: &str) -> Stmt {
        let mut map = SourceMap::new();
        let id = map.add("t.mwl", format!("<?mwl {src}"));
        let mut diags = Diagnostics::new();
        let mut p = Parser::new(map.file(id), &mut diags);
        p.bump(); // OpenTagMwl
        let s = p.parse_statement();
        assert!(
            !diags.has_errors(),
            "unexpected diagnostics for {src:?}: {diags:?}"
        );
        s
    }

    /// Parses `src` as one statement and returns it along with whatever
    /// diagnostics were reported, for tests that expect a reported error.
    fn parse_stmt_with_diags(src: &str) -> (Stmt, Diagnostics) {
        let mut map = SourceMap::new();
        let id = map.add("t.mwl", format!("<?mwl {src}"));
        let mut diags = Diagnostics::new();
        let mut p = Parser::new(map.file(id), &mut diags);
        p.bump();
        let s = p.parse_statement();
        (s, diags)
    }

    #[test]
    fn if_elseif_else_chain() {
        let s = parse_stmt_ok("if ($a) { 1; } elseif ($b) { 2; } else { 3; }");
        let StmtKind::If { then, else_, .. } = s.kind else {
            panic!("expected an if: {s:?}");
        };
        assert!(matches!(then.kind, StmtKind::Block(_)));
        let else_ = else_.expect("elseif chain");
        let StmtKind::If {
            else_: inner_else, ..
        } = else_.kind
        else {
            panic!("expected `elseif` to produce a nested if: {else_:?}");
        };
        assert!(matches!(inner_else.unwrap().kind, StmtKind::Block(_)));
    }

    #[test]
    fn else_if_two_words_matches_elseif() {
        // `else if (...)` recurses through the ordinary statement dispatch
        // rather than a dedicated `elseif` production, but produces the same
        // nested-`If` shape.
        let s = parse_stmt_ok("if ($a) { 1; } else if ($b) { 2; }");
        let StmtKind::If { else_, .. } = s.kind else {
            panic!("expected an if: {s:?}");
        };
        assert!(matches!(else_.unwrap().kind, StmtKind::If { .. }));
    }

    #[test]
    fn while_do_while_and_for() {
        let s = parse_stmt_ok("while ($i < 10) { $i++; }");
        assert!(matches!(s.kind, StmtKind::While { .. }));

        let s = parse_stmt_ok("do { $i++; } while ($i < 10);");
        assert!(matches!(s.kind, StmtKind::DoWhile { .. }));

        let s = parse_stmt_ok("for ($i = 0; $i < 10; $i++) { }");
        let StmtKind::For {
            init, cond, step, ..
        } = s.kind
        else {
            panic!("expected a for loop: {s:?}");
        };
        assert_eq!(init.len(), 1);
        assert_eq!(cond.len(), 1);
        assert_eq!(step.len(), 1);
    }

    #[test]
    fn empty_statement_and_empty_for_body() {
        let s = parse_stmt_ok(";");
        assert!(matches!(s.kind, StmtKind::Empty));
        let s = parse_stmt_ok("for (;;) ;");
        let StmtKind::For { body, .. } = s.kind else {
            panic!("expected a for loop: {s:?}");
        };
        assert!(matches!(body.kind, StmtKind::Empty));
    }

    #[test]
    fn foreach_with_typed_key_and_value() {
        let s = parse_stmt_ok("foreach ($rows as string $k => array<int> $row) { }");
        let StmtKind::Foreach {
            key,
            value,
            value_by_ref,
            ..
        } = s.kind
        else {
            panic!("expected a foreach: {s:?}");
        };
        assert!(key.is_some());
        assert!(value.ty.is_some());
        assert!(!value_by_ref);
    }

    #[test]
    fn foreach_value_only_by_reference() {
        let s = parse_stmt_ok("foreach ($items as int &$v) { }");
        let StmtKind::Foreach {
            key, value_by_ref, ..
        } = s.kind
        else {
            panic!("expected a foreach: {s:?}");
        };
        assert!(key.is_none());
        assert!(value_by_ref);
    }

    #[test]
    fn foreach_key_and_by_reference_value() {
        let s = parse_stmt_ok("foreach ($items as string $k => int &$v) { }");
        let StmtKind::Foreach {
            key, value_by_ref, ..
        } = s.kind
        else {
            panic!("expected a foreach: {s:?}");
        };
        assert!(key.is_some());
        assert!(value_by_ref);
    }

    #[test]
    fn foreach_header_as_belongs_to_foreach_not_conversion() {
        // ADR 0007 § 2 / docs/spec/00-overview.md § 3.2: converting the
        // *subject* inside a `foreach` header needs parens, since a bare
        // `as` right after the subject is `foreach`'s own separator.
        let s = parse_stmt_ok("foreach (($m as array<int>) as int $v) { }");
        insta::assert_debug_snapshot!(s);
    }

    #[test]
    fn switch_with_fallthrough_and_default() {
        let s = parse_stmt_ok("switch ($x) { case 1: case 2: echo $x; break; default: echo 0; }");
        let StmtKind::Switch { cases, .. } = s.kind else {
            panic!("expected a switch: {s:?}");
        };
        assert_eq!(cases.len(), 3);
        assert!(cases[0].cond.is_some());
        assert!(cases[0].body.is_empty(), "fallthrough case has no body");
        assert!(cases[2].cond.is_none(), "the last arm is `default`");
    }

    #[test]
    fn break_and_continue_with_level() {
        let s = parse_stmt_ok("break;");
        assert!(matches!(s.kind, StmtKind::Break(None)));
        let s = parse_stmt_ok("continue 2;");
        assert!(matches!(s.kind, StmtKind::Continue(Some(_))));
    }

    #[test]
    fn try_multi_catch_and_finally() {
        let s = parse_stmt_ok(
            "try { risky(); } catch (TypeError|ValueError $e) { } finally { cleanup(); }",
        );
        let StmtKind::Try {
            catches, finally, ..
        } = s.kind
        else {
            panic!("expected a try: {s:?}");
        };
        assert_eq!(catches.len(), 1);
        assert!(matches!(catches[0].ty.kind, TypeKind::Union(_)));
        assert!(catches[0].var.is_some());
        assert!(finally.is_some());
    }

    #[test]
    fn catch_without_a_variable() {
        let s = parse_stmt_ok("try { } catch (Throwable) { }");
        let StmtKind::Try { catches, .. } = s.kind else {
            panic!("expected a try: {s:?}");
        };
        assert!(catches[0].var.is_none());
    }

    #[test]
    fn echo_and_unset() {
        let s = parse_stmt_ok("echo 1, 2, 3;");
        let StmtKind::Echo(exprs) = s.kind else {
            panic!("expected echo: {s:?}");
        };
        assert_eq!(exprs.len(), 3);

        let s = parse_stmt_ok("unset($a, $b);");
        let StmtKind::Unset(exprs) = s.kind else {
            panic!("expected unset: {s:?}");
        };
        assert_eq!(exprs.len(), 2);
    }

    #[test]
    fn typed_local_declaration_with_and_without_initializer() {
        let s = parse_stmt_ok("int $n = 0;");
        let StmtKind::LocalDecl { ty, value, .. } = s.kind else {
            panic!("expected a local decl: {s:?}");
        };
        assert!(matches!(ty.kind, TypeKind::Atom(TypeAtom::Int)));
        assert!(value.is_some());

        let s = parse_stmt_ok("array<uint> $ids;");
        let StmtKind::LocalDecl { value, .. } = s.kind else {
            panic!("expected a local decl: {s:?}");
        };
        assert!(value.is_none());
    }

    #[test]
    fn a_parenthesized_expression_statement_is_not_confused_with_a_type() {
        // `(` also starts a type (a parenthesized union/intersection), so a
        // statement beginning with `(non-type-expr)` trial-parses as a local
        // decl first. The trial must fail cleanly here: `parse_type_atom`'s
        // error recovery doesn't consume the offending token, so without
        // checking whether the trial itself reported anything, the cursor
        // landing on a `$variable` by coincidence (as it does right after
        // `$a` here) reads as "yes, a type was followed by a variable."
        let s = parse_stmt_ok("($a > 0 || $b > 0) ? f($a) : g($a);");
        let StmtKind::Expr(e) = s.kind else {
            panic!("expected an expression statement: {s:?}");
        };
        assert!(matches!(e.kind, ExprKind::Ternary { .. }));
    }

    #[test]
    fn a_name_used_as_a_type_is_not_confused_with_a_static_call() {
        let s = parse_stmt_ok("User $owner = User::find($id);");
        assert!(matches!(s.kind, StmtKind::LocalDecl { .. }));

        let s = parse_stmt_ok("Foo::bar();");
        let StmtKind::Expr(e) = s.kind else {
            panic!("expected an expression statement: {s:?}");
        };
        assert!(matches!(e.kind, ExprKind::StaticCall { .. }));
    }

    #[test]
    fn a_bitwise_or_of_two_constants_is_not_confused_with_a_union_type() {
        let s = parse_stmt_ok("Foo | Bar;");
        let StmtKind::Expr(e) = s.kind else {
            panic!("expected an expression statement: {s:?}");
        };
        assert!(matches!(
            e.kind,
            ExprKind::Binary {
                op: BinaryOp::BitOr,
                ..
            }
        ));
    }

    #[test]
    fn destructure_brackets_typed_nested_and_skipped() {
        let s = parse_stmt_ok("[int $a, , string $b] = $triple;");
        let StmtKind::Destructure { target, .. } = s.kind else {
            panic!("expected a destructure: {s:?}");
        };
        assert_eq!(target.elements.len(), 3);
        assert!(matches!(
            target.elements[0],
            DestructureElement::Leaf { .. }
        ));
        assert!(matches!(target.elements[1], DestructureElement::Skip));
        assert!(matches!(
            target.elements[2],
            DestructureElement::Leaf { .. }
        ));

        let s = parse_stmt_ok("[[int $x, int $y], string $label] = $point;");
        let StmtKind::Destructure { target, .. } = s.kind else {
            panic!("expected a destructure: {s:?}");
        };
        assert!(matches!(
            target.elements[0],
            DestructureElement::Nested { .. }
        ));
    }

    #[test]
    fn destructure_with_string_keys() {
        let s = parse_stmt_ok("['id' => uint $id, 'name' => string $name] = $row;");
        let StmtKind::Destructure { target, .. } = s.kind else {
            panic!("expected a destructure: {s:?}");
        };
        let DestructureElement::Leaf { key, .. } = &target.elements[0] else {
            panic!("expected a leaf: {:?}", target.elements[0]);
        };
        assert!(key.is_some());
    }

    #[test]
    fn list_is_a_second_spelling_of_bracket_destructuring() {
        let s = parse_stmt_ok("list(int $a, string $b) = $pair;");
        assert!(matches!(s.kind, StmtKind::Destructure { .. }));
    }

    #[test]
    fn list_elements_still_require_a_type_like_brackets_do() {
        // Unlike plain PHP, neither spelling has an untyped form.
        let (_, diags) = parse_stmt_with_diags("list($a, $b) = $pair;");
        assert!(diags.has_errors());
    }

    #[test]
    fn a_plain_array_literal_statement_is_not_confused_with_destructuring() {
        let s = parse_stmt_ok("[1, 2, 3];");
        let StmtKind::Expr(e) = s.kind else {
            panic!("expected an expression statement: {s:?}");
        };
        assert!(matches!(e.kind, ExprKind::ArrayLiteral(_)));
    }

    #[test]
    fn global_is_diagnosed_but_still_parses() {
        let (s, diags) = parse_stmt_with_diags("global $a, $b;");
        assert!(diags.has_errors());
        let StmtKind::Global(vars) = s.kind else {
            panic!("expected global: {s:?}");
        };
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn goto_is_diagnosed_but_still_parses() {
        let (s, diags) = parse_stmt_with_diags("goto done;");
        assert!(diags.has_errors());
        assert!(matches!(s.kind, StmtKind::Goto(_)));
    }

    #[test]
    fn function_scope_static_is_diagnosed_but_still_parses() {
        let (s, diags) = parse_stmt_with_diags("static $calls = 0;");
        assert!(diags.has_errors());
        let StmtKind::StaticLocal { ty, vars } = s.kind else {
            panic!("expected a static local: {s:?}");
        };
        assert!(ty.is_none());
        assert_eq!(vars.len(), 1);
    }

    #[test]
    fn typed_function_scope_static_is_also_diagnosed() {
        let (s, diags) = parse_stmt_with_diags("static int $calls = 0;");
        assert!(diags.has_errors());
        let StmtKind::StaticLocal { ty, .. } = s.kind else {
            panic!("expected a static local: {s:?}");
        };
        assert!(ty.is_some());
    }

    #[test]
    fn static_closure_and_static_property_are_not_confused_with_function_static() {
        // `static fn`/`static function` are already diagnosed as unsupported
        // closure modifiers (chunk 1) — the point here is that they must
        // NOT also be routed into the function-scope-`static` rejection.
        let (s, diags) = parse_stmt_with_diags("static fn (int $x) => $x;");
        assert!(
            !diags
                .iter()
                .any(|d| d.code == Some(code::E_STATIC_LOCAL_UNSUPPORTED))
        );
        let StmtKind::Expr(e) = s.kind else {
            panic!("expected an expression statement: {s:?}");
        };
        assert!(matches!(e.kind, ExprKind::ArrowFn(_)));

        parse_stmt_ok("static::method();");
        parse_stmt_ok("$x = static::$prop;");
    }

    #[test]
    fn eval_extract_settype_are_diagnosed() {
        for (src, code) in [
            ("eval($src);", code::E_EVAL_UNSUPPORTED),
            ("extract($arr);", code::E_EXTRACT_UNSUPPORTED),
            ("settype($x, 'int');", code::E_SETTYPE_UNSUPPORTED),
        ] {
            let (s, diags) = parse_stmt_with_diags(src);
            assert!(diags.has_errors(), "expected a diagnostic for {src:?}");
            assert!(
                diags.iter().any(|d| d.code == Some(code)),
                "expected {code:?} for {src:?}, got {diags:?}"
            );
            let StmtKind::Expr(e) = s.kind else {
                panic!("expected an expression statement: {s:?}");
            };
            assert!(matches!(e.kind, ExprKind::Error));
        }
    }

    #[test]
    fn include_and_require_are_expressions() {
        let s = parse_stmt_ok("$x = include 'a.mwl';");
        let StmtKind::Expr(e) = s.kind else {
            panic!("expected an expression statement: {s:?}");
        };
        let ExprKind::Assign { value, .. } = e.kind else {
            panic!("expected an assignment: {e:?}");
        };
        assert!(matches!(
            value.kind,
            ExprKind::Include {
                kind: IncludeKind::Include,
                ..
            }
        ));

        parse_stmt_ok("require_once 'b.mwl';");
        parse_stmt_ok("include_once 'c.mwl';");
        parse_stmt_ok("require 'd.mwl';");
    }

    // ========================================================================
    // Chunk 3: declarations
    // ========================================================================

    #[test]
    fn class_with_extends_implements_and_members() {
        let s = parse_stmt_ok(
            "class Account extends Base implements Comparable, Countable { \
             public readonly uint $id; \
             public const int MAX = 10; \
             public function __construct(public readonly string $name) {} \
             }",
        );
        let StmtKind::ClassDecl(class) = s.kind else {
            panic!("expected a class decl: {s:?}");
        };
        assert!(class.extends.is_some());
        assert_eq!(class.implements.len(), 2);
        assert_eq!(class.members.len(), 3);
        let ClassMemberKind::Property(prop) = &class.members[0].kind else {
            panic!("expected a property: {:?}", class.members[0]);
        };
        assert!(prop.modifiers.contains(&Modifier::Readonly));
        let ClassMemberKind::Const(c) = &class.members[1].kind else {
            panic!("expected a const: {:?}", class.members[1]);
        };
        assert!(c.ty.is_some());
        let ClassMemberKind::Method(m) = &class.members[2].kind else {
            panic!("expected a method: {:?}", class.members[2]);
        };
        assert_eq!(m.params.len(), 1);
        assert!(m.params[0].modifiers.contains(&Modifier::Public));
        assert!(m.params[0].modifiers.contains(&Modifier::Readonly));
        assert!(m.body.is_some());
    }

    #[test]
    fn abstract_and_final_class_modifiers() {
        let s = parse_stmt_ok("abstract class Shape { public abstract function area(): float; }");
        let StmtKind::ClassDecl(class) = s.kind else {
            panic!("expected a class decl: {s:?}");
        };
        assert!(class.modifiers.contains(&Modifier::Abstract));
        let ClassMemberKind::Method(m) = &class.members[0].kind else {
            panic!("expected a method: {:?}", class.members[0]);
        };
        assert!(m.modifiers.contains(&Modifier::Abstract));
        assert!(m.body.is_none());

        let s = parse_stmt_ok("final class Sealed {}");
        let StmtKind::ClassDecl(class) = s.kind else {
            panic!("expected a class decl: {s:?}");
        };
        assert!(class.modifiers.contains(&Modifier::Final));
    }

    #[test]
    fn interface_with_multiple_extends() {
        let s = parse_stmt_ok(
            "interface Shape extends Comparable, Countable { public function area(): float; }",
        );
        let StmtKind::InterfaceDecl(iface) = s.kind else {
            panic!("expected an interface decl: {s:?}");
        };
        assert_eq!(iface.extends.len(), 2);
        assert_eq!(iface.members.len(), 1);
    }

    #[test]
    fn trait_use_insteadof_is_kept() {
        let s = parse_stmt_ok(
            "class Greeter { use Greets, Announces { Greets::hello insteadof Announces; } }",
        );
        let StmtKind::ClassDecl(class) = s.kind else {
            panic!("expected a class decl: {s:?}");
        };
        let ClassMemberKind::UseTrait(u) = &class.members[0].kind else {
            panic!("expected a trait use: {:?}", class.members[0]);
        };
        assert_eq!(u.traits.len(), 2);
        assert_eq!(u.adaptations.len(), 1);
        assert!(matches!(
            u.adaptations[0].kind,
            TraitAdaptationKind::InsteadOf { .. }
        ));
    }

    #[test]
    fn trait_use_as_rename_and_visibility_are_rejected() {
        let (_, diags) =
            parse_stmt_with_diags("class Greeter { use Greets { Greets::hello as sayHello; } }");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TRAIT_METHOD_RENAME_UNSUPPORTED))
        );

        let (_, diags) =
            parse_stmt_with_diags("class Greeter { use Greets { Greets::hello as protected; } }");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TRAIT_METHOD_VISIBILITY_UNSUPPORTED))
        );
    }

    #[test]
    fn enum_cases_and_explicit_backing_type() {
        let s = parse_stmt_ok("enum Status { Active, Banned, }");
        let StmtKind::EnumDecl(e) = s.kind else {
            panic!("expected an enum decl: {s:?}");
        };
        assert!(e.backing.is_none());
        assert_eq!(e.cases.len(), 2);
        assert!(e.cases[0].value.is_none());

        let s =
            parse_stmt_ok("enum Permission: uint { Read = 0b001, Write = 0b010, Admin = 0b100 }");
        let StmtKind::EnumDecl(e) = s.kind else {
            panic!("expected an enum decl: {s:?}");
        };
        assert!(matches!(
            e.backing.unwrap().kind,
            TypeKind::Atom(TypeAtom::Uint)
        ));
        assert_eq!(e.cases.len(), 3);
        assert!(e.cases[0].value.is_some());
    }

    #[test]
    fn enum_implements_method_and_string_backing_are_rejected() {
        let (_, diags) = parse_stmt_with_diags("enum Status implements Comparable { Active }");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_ENUM_IMPLEMENTS_UNSUPPORTED))
        );

        let (_, diags) = parse_stmt_with_diags("enum Status: string { Active }");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_ENUM_STRING_BACKING_UNSUPPORTED))
        );

        let (s, diags) =
            parse_stmt_with_diags("enum Status { Active, public function foo(): void {} }");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_ENUM_MEMBER_UNSUPPORTED))
        );
        let StmtKind::EnumDecl(e) = s.kind else {
            panic!("expected an enum decl: {s:?}");
        };
        assert_eq!(e.cases.len(), 1);
        assert_eq!(e.members.len(), 1);
    }

    #[test]
    fn property_hooks_get_and_set() {
        let s = parse_stmt_ok(
            "class Temperature { \
             public float $celsius; \
             public float $fahrenheit { \
                 get => $this->celsius * 9 / 5 + 32; \
                 set(float $f) { $this->celsius = ($f - 32) * 5 / 9; } \
             } \
             }",
        );
        let StmtKind::ClassDecl(class) = s.kind else {
            panic!("expected a class decl: {s:?}");
        };
        let ClassMemberKind::Property(prop) = &class.members[1].kind else {
            panic!("expected a property: {:?}", class.members[1]);
        };
        let hooks = prop.hooks.as_ref().expect("hooked property");
        assert_eq!(hooks.len(), 2);
        assert_eq!(hooks[0].kind, PropertyHookKind::Get);
        assert!(matches!(hooks[0].body, Some(PropertyHookBody::Expr(_))));
        assert_eq!(hooks[1].kind, PropertyHookKind::Set);
        assert!(hooks[1].param.is_some());
        assert!(matches!(hooks[1].body, Some(PropertyHookBody::Block(_))));
    }

    #[test]
    fn asymmetric_visibility_modifier() {
        let s = parse_stmt_ok("class Point { public private(set) int $x; }");
        let StmtKind::ClassDecl(class) = s.kind else {
            panic!("expected a class decl: {s:?}");
        };
        let ClassMemberKind::Property(prop) = &class.members[0].kind else {
            panic!("expected a property: {:?}", class.members[0]);
        };
        assert!(prop.modifiers.contains(&Modifier::Public));
        assert!(
            prop.modifiers
                .contains(&Modifier::SetVisibility(Visibility::Private))
        );
    }

    #[test]
    fn namespace_statement_and_block_forms() {
        let s = parse_stmt_ok("namespace App\\Models;");
        let StmtKind::NamespaceDecl(ns) = s.kind else {
            panic!("expected a namespace decl: {s:?}");
        };
        assert!(ns.name.is_some());
        assert!(ns.body.is_none());

        let s = parse_stmt_ok("namespace App { class Foo {} }");
        let StmtKind::NamespaceDecl(ns) = s.kind else {
            panic!("expected a namespace decl: {s:?}");
        };
        assert!(ns.body.is_some());
    }

    #[test]
    fn namespace_core_is_reserved() {
        let (_, diags) = parse_stmt_with_diags("namespace Core;");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_RESERVED_CORE_NAMESPACE))
        );

        let (_, diags) = parse_stmt_with_diags("namespace Core\\Sub;");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_RESERVED_CORE_NAMESPACE))
        );
    }

    #[test]
    fn use_import_plain_and_rejected_alias() {
        let s = parse_stmt_ok("use App\\Models\\User;");
        let StmtKind::UseDecl(u) = s.kind else {
            panic!("expected a use decl: {s:?}");
        };
        assert!(u.alias.is_none());

        let (s, diags) = parse_stmt_with_diags("use App\\Models\\User as Model;");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_IMPORT_ALIAS_UNSUPPORTED))
        );
        let StmtKind::UseDecl(u) = s.kind else {
            panic!("expected a use decl: {s:?}");
        };
        assert!(u.alias.is_some());
    }

    #[test]
    fn type_alias_declaration() {
        let s = parse_stmt_ok("type UserId = uint;");
        let StmtKind::TypeAliasDecl(t) = s.kind else {
            panic!("expected a type alias decl: {s:?}");
        };
        assert!(matches!(t.ty.kind, TypeKind::Atom(TypeAtom::Uint)));

        // A single bare class atom parses fine — the restriction is M2's.
        parse_stmt_ok("type Id = SomeClass;");
    }

    #[test]
    fn anonymous_class_as_new_target() {
        let s = parse_stmt_ok("$x = new class (1) implements Comparable { public int $n = 1; };");
        let StmtKind::Expr(e) = s.kind else {
            panic!("expected an expression statement: {s:?}");
        };
        let ExprKind::Assign { value, .. } = e.kind else {
            panic!("expected an assignment: {e:?}");
        };
        let ExprKind::New { target, args } = value.kind else {
            panic!("expected a `new`: {value:?}");
        };
        let NewTarget::AnonClass(decl) = target else {
            panic!("expected an anonymous class target: {target:?}");
        };
        assert_eq!(decl.implements.len(), 1);
        assert_eq!(decl.members.len(), 1);
        assert!(matches!(args, CallArgs::List(list) if list.len() == 1));
    }

    #[test]
    fn toplevel_function_and_const_are_rejected() {
        let (s, diags) = parse_stmt_with_diags("function greet(): void {}");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TOPLEVEL_FUNCTION_UNSUPPORTED))
        );
        assert!(matches!(s.kind, StmtKind::TopLevelFunction(_)));

        let (s, diags) = parse_stmt_with_diags("const FOO = 1;");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_TOPLEVEL_CONST_UNSUPPORTED))
        );
        let StmtKind::TopLevelConst(consts) = s.kind else {
            panic!("expected a rejected top-level const: {s:?}");
        };
        assert_eq!(consts.len(), 1);

        // An anonymous closure statement is unaffected.
        parse_stmt_ok("function () {};");
    }

    // ========================================================================
    // `parse_file`: the whole-file HTML/code round trip
    // ========================================================================

    fn parse_file_ok(src: &str) -> Vec<Stmt> {
        let mut map = SourceMap::new();
        let id = map.add("t.mwl", src.to_string());
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(id), &mut diags);
        assert!(
            !diags.has_errors(),
            "unexpected diagnostics for {src:?}: {diags:?}"
        );
        stmts
    }

    #[test]
    fn a_pure_code_file_has_no_inline_html() {
        let stmts = parse_file_ok("<?mwl echo 1;");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0].kind, StmtKind::Echo(_)));
    }

    #[test]
    fn leading_html_before_the_open_tag_is_kept_verbatim() {
        let mut map = SourceMap::new();
        let id = map.add("t.mwl", "hello <?mwl echo 1;");
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(id), &mut diags);
        assert!(!diags.has_errors());

        let StmtKind::InlineHtml(span) = stmts[0].kind else {
            panic!("expected leading inline HTML: {:?}", stmts[0]);
        };
        assert_eq!(map.file(id).span_text(span), Some("hello "));
        assert!(matches!(stmts[1].kind, StmtKind::Echo(_)));
    }

    /// `cargo fuzz run parse` found this exact byte sequence — minimized to
    /// a `switch` keyword followed by nothing resembling `case`/`default`/
    /// `}` — spinning forever and growing `cases`/`body` without bound
    /// instead of terminating, because neither `parse_switch`'s case loop
    /// nor a case body's own statement loop had the force-progress guard
    /// [`Parser::parse_block`]'s copy has (see the module docs' "Error
    /// recovery" section). If this regresses, the test hangs rather than
    /// fails cleanly — same as the bug itself did.
    #[test]
    fn a_malformed_switch_does_not_hang_or_grow_without_bound() {
        let mut map = SourceMap::new();
        let id = map.add("t.mwl", "<?=\n\0\0switch]]\0\0w]]]]\n".to_string());
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(id), &mut diags);
        assert!(diags.has_errors());
        assert!(
            stmts.len() < 1000,
            "runaway recovery: {} statements",
            stmts.len()
        );
    }

    /// `cargo fuzz run parse` also found deeply nested parens overflowing
    /// the native call stack outright — not a hang, an immediate crash,
    /// since unlike the `switch` case above this recursion is entirely
    /// well-formed at every level (`parse_type_operand`/`parse_low_or`
    /// legitimately calling back into themselves), so no force-progress
    /// guard applies. [`Parser::enter_recursive`] bounds it instead. This
    /// input is two full orders of magnitude past the limit; if the guard
    /// regresses, this crashes the test process rather than failing it
    /// cleanly.
    #[test]
    fn extremely_deep_nesting_does_not_overflow_the_stack() {
        for opener in ['(', '['] {
            let closer = if opener == '(' { ')' } else { ']' };
            let mut src = String::from("<?mwl $x = ");
            src.extend(std::iter::repeat_n(opener, 10_000));
            src.push('1');
            src.extend(std::iter::repeat_n(closer, 10_000));
            src.push(';');
            let mut map = SourceMap::new();
            let id = map.add("t.mwl", src);
            let mut diags = Diagnostics::new();
            let stmts = parse_file(map.file(id), &mut diags);
            assert!(diags.has_errors());
            assert!(!stmts.is_empty());
        }
    }

    /// A long postfix chain (`$x[0][0][0]...`) builds an equally long
    /// `Box`-nested `Expr` through a *loop*, not recursion, so it survives
    /// parsing regardless — but the resulting structure used to grow
    /// without bound, and a fuzz-run investigation found that overflowing
    /// the stack in the very first ordinary recursive walk over it
    /// afterwards (originally `mwl ast`'s pretty-printer). Confirms the
    /// chain itself gets folded back to a bounded depth: this walks the
    /// `Index`/`base` links by hand (not `{:#?}`, to keep the test's own
    /// assertion from being exactly the kind of unbounded recursive walk
    /// this is guarding against) and checks it stops within a small
    /// multiple of the guard's limit.
    #[test]
    fn a_long_postfix_chain_is_folded_back_to_a_bounded_depth() {
        let mut src = String::from("<?mwl $x");
        for i in 0..10_000 {
            src.push_str(&format!("[{i}]"));
        }
        src.push(';');
        let mut map = SourceMap::new();
        let id = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(id), &mut diags);
        assert!(diags.has_errors());

        let StmtKind::Expr(mut e) = stmts[0].kind.clone() else {
            panic!("expected an expression statement: {:?}", stmts[0]);
        };
        let mut depth = 0u32;
        while let ExprKind::Index { base, .. } = e.kind {
            e = *base;
            depth += 1;
        }
        assert!(depth < 1000, "chain was not folded back: depth {depth}");
    }

    #[test]
    fn a_file_that_never_opens_a_tag_is_all_inline_html() {
        let stmts = parse_file_ok("just some text, no code at all");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0].kind, StmtKind::InlineHtml(_)));
    }

    #[test]
    fn closing_and_reopening_a_tag_mid_block_is_legal() {
        // `if ($x) { ?>html<?php }` — PHP allows leaving code mode inside a
        // block; the `}` that closes the `if` is itself back in code mode.
        let stmts = parse_file_ok("<?mwl if ($x) { ?>html<?php } ?>tail");
        let StmtKind::If { then, .. } = &stmts[0].kind else {
            panic!("expected an if: {:?}", stmts[0]);
        };
        let StmtKind::Block(block) = &then.kind else {
            panic!("expected a block body: {then:?}");
        };
        assert!(
            block
                .stmts
                .iter()
                .any(|s| matches!(s.kind, StmtKind::InlineHtml(_))),
            "expected inline HTML inside the block: {block:?}"
        );
        let StmtKind::InlineHtml(_) = stmts.last().unwrap().kind else {
            panic!("expected trailing inline HTML: {:?}", stmts.last());
        };
    }

    #[test]
    fn short_echo_tag_is_sugar_for_echo() {
        let stmts = parse_file_ok("<?= $name ?>");
        let StmtKind::Echo(exprs) = &stmts[0].kind else {
            panic!("expected an echo: {:?}", stmts[0]);
        };
        assert_eq!(exprs.len(), 1);
    }

    #[test]
    fn short_echo_tag_semicolon_before_close_tag_is_optional_but_allowed() {
        parse_file_ok("<?= $name ?>");
        parse_file_ok("<?= $name; ?>");
    }
}
