//! The recursive-descent parser: the machinery every production shares, and
//! the four modules the grammar itself lives in.
//!
//! | module | grammar |
//! |---|---|
//! | [`ty`] | the type grammar (ADR 0007 § 3), and a qualified name |
//! | [`expr`] | every operator at its PHP precedence, down to a primary |
//! | [`stmt`] | control flow, `echo`, `unset`, a typed local, destructuring |
//! | [`decl`] | classes, interfaces, enums, attributes, `namespace`/`use`/`autoload`/`type` |
//!
//! One `impl Parser` split across those four, which Rust allows for an
//! inherent impl inside one crate; this file holds the state, the token
//! buffer, the diagnostics helpers and the two entry points
//! ([`parse_file`]/[`parse_expression`]). A production reaches its neighbours
//! as `pub(super)`, which is the reach it had when `parser` was a single file,
//! and no further.
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
//! case's own statement list, and similar) — those additionally force a token
//! of progress if one iteration consumed none at all, mirroring
//! [`Parser::parse_block`]'s own copy of the guard, so a malformed body cannot
//! hang the parser or grow its result vector without bound. A comma-separated
//! list (call arguments, array items, parameters, ...) never needs this: it
//! already breaks out the moment no comma follows.
//!
//! # Backtracking, and how deep anything may nest
//!
//! Every production is a single, committed pass — except two statement forms
//! whose grammar is ambiguous on a token prefix alone, which trial-parse the
//! more specific production and [`Parser::restore`] a [`Checkpoint`] when the
//! deciding token does not show up. Both are in [`stmt`], and that module's
//! header names them.
//!
//! A separate concern from either of the above: how *deep* one construct can
//! nest inside another. [`Parser::enter_recursive`] bounds every kind of
//! nesting (an expression in an expression, a type in a type, a statement in a
//! statement, a destructuring target in one, a postfix chain link) with one
//! shared counter and a sticky give-up flag, so a fuzzer-shaped input —
//! thousands of nested `(`/`[`, or a long `->`/`[...]` chain — can neither
//! overflow the native call stack nor, once it gives up once, cost more than
//! one more token of work per token remaining in the file. See that method's
//! docs for the full reasoning.

use std::collections::VecDeque;

use mwl_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};

use crate::ast::{
    AnonClassDecl, Arg, ArrayItem, AssignOp, Attribute, AttributeGroup, AutoloadDecl, AutoloadKind,
    BinaryOp, Block, CallArgs, CatchClause, ClassDecl, ClassMember, ClassMemberKind, ConstMember,
    DestructureElement, DestructureTarget, EnumCase, EnumDecl, Expr, ExprKind, FnBody, FnExpr,
    ForeachBinding, ImplementsClause, IncDecOp, InterfaceDecl, MatchArm, MemberName, MethodMember,
    Modifier, Name, NamespaceDecl, NewTarget, ObjectLiteralField, Param, PropertyHook,
    PropertyHookBody, PropertyHookKind, PropertyMember, ShapeField, SpawnOption, SpawnOptionKey,
    StaticVar, Stmt, StmtKind, StringPart, SwitchCase, Type, TypeAliasDecl, TypeAtom, TypeKind,
    UnaryOp, UseDecl, Visibility,
};
use crate::lexer::Lexer;
use crate::token::{Keyword, Token, TokenKind};

// The grammar, one module per layer — see the table above and each module's
// own header. They add methods to the one `impl Parser` below and export
// nothing, so there is nothing to import from them.
mod decl;
mod expr;
mod stmt;
mod ty;

#[cfg(test)]
mod tests;

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

    /// The text of an identifier-shaped span, for matching a contextual
    /// keyword (`spawn`, `script`, `with`) by spelling rather than token kind
    /// — see [`crate::token`]'s module docs for why these three stay plain
    /// [`TokenKind::Ident`]s instead of reserved words.
    ///
    /// The comparison is exact: a contextual keyword is a reserved spelling
    /// like any other, so it is lower case only
    /// ([ADR 0062](../../../docs/adr/0062-case-sensitivity-is-a-compiler-property.md)
    /// § 2). `SPAWN` is just an identifier.
    fn ident_text(&self, span: Span) -> &str {
        self.file.span_text(span).unwrap_or_default()
    }

    fn at_contextual(&mut self, word: &str) -> bool {
        if !matches!(self.peek().kind, TokenKind::Ident) {
            return false;
        }
        let span = self.peek().span;
        self.ident_text(span) == word
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

    /// ADR 0107 § 3: `&` is no longer a by-reference marker. The marker is
    /// still *recognized* wherever PHP would put one — that is the whole
    /// reason [`Self::at_intersection_amp`] survives — so the site can name
    /// the exact fix instead of failing as a malformed type. `fix` is that
    /// fix, in the site's own words, because the two binding positions are
    /// replaced by `inout` and the two returning ones are replaced by
    /// nothing at all.
    fn report_by_reference_marker(&mut self, amp: Span, fix: &str) {
        self.diags.report(
            Diagnostic::error(
                code::E_BY_REFERENCE_MARKER_RETIRED,
                "`&` is not a by-reference marker in MWL",
            )
            .with_primary(amp, "`&` is bitwise AND, and an intersection type")
            .with_help(fix.to_string()),
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
