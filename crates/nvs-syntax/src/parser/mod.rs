//! The recursive-descent parser: the machinery every production shares, and
//! the four modules the grammar itself lives in.
//!
//! | module | grammar |
//! |---|---|
//! | [`ty`] | the type grammar (`rule:types/grammar`), and a qualified name |
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
//! Every production is a single, committed pass — except three forms whose
//! grammar is ambiguous on a token prefix alone, which trial-parse the more
//! specific production and [`Parser::restore`] a [`Checkpoint`] when the
//! deciding token does not show up. All three are in [`stmt`], and that
//! module's header names them.
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

use nvs_diagnostics::{BytePos, Diagnostic, Diagnostics, SourceFile, Span, code};

use crate::ast::{
    AnonClassDecl, AnonObjectField, Arg, ArrayItem, AssignOp, Attribute, AttributeGroup,
    AutoloadDecl, AutoloadKind, BinaryOp, Block, CallArgs, CatchArm, CatchClause, ClassDecl,
    ClassMember, ClassMemberKind, ConstMember, DestructureElement, DestructureTarget, DocComment,
    DocTag, DocTagKind, EnumCase, EnumDecl, Expr, ExprKind, FnBody, FnExpr, ForInit,
    ForeachBinding, ForeachBindingTy, IfArm, ImplementsClause, IncDecOp, InterfaceDecl, MatchArm,
    MemberName, MethodMember, Modifier, Name, NamespaceDecl, NewTarget, Param, PropertyHook,
    PropertyHookBody, PropertyHookKind, PropertyMember, ShapeField, SpawnOption, SpawnOptionKey,
    StaticVar, Stmt, StmtKind, StringPart, SwitchCase, TestOperand, Type, TypeAliasDecl, TypeAtom,
    TypeKind, UnaryOp, UseDecl, Visibility, WrittenModifier,
};
use crate::index::SyntaxIndex;
use crate::lexer::Lexer;
use crate::token::{Keyword, Token, TokenKind, Trivia, TriviaKind};

// The grammar, one module per layer — see the table above and each module's
// own header. They add methods to the one `impl Parser` below and export
// nothing, so there is nothing to import from them.
mod decl;
mod expr;
mod grouping;
mod stmt;
mod ty;

#[cfg(test)]
mod tests;

/// Produces an AST from a source file, one construct at a time.
///
/// Holds its own lookahead buffer over the [`Lexer`]. Three tokens answer all
/// but one of the grammar's classifications, the widest being the `(`
/// *cast-keyword* `)` run that tells a cast apart from a parenthesized
/// expression. The exception is the shape-typed-local tell in `parser::stmt`,
/// which asks what follows a *matched* `{...}` and so buffers that run's
/// tokens, up to the limit that module fixes, until the statement under it
/// consumes them again.
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
    /// Where every `///` run that found a declaration begins — the offset of
    /// its first slash. [`Self::report_unattached_docs`] subtracts these from
    /// the doc comments the lexer kept, and what is left documented nothing
    /// (`rule:tooling/doc-comment-attaches-to-the-next-declaration`).
    docs_attached: Vec<BytePos>,
    /// The start of every `/** … */` block already warned about as sitting
    /// directly above a declaration, so a production that takes one
    /// declaration's run more than once reports the block once.
    docblocks_reported: Vec<BytePos>,
    /// The left side of the `|>` whose right side is being parsed, waiting for
    /// the `$_` that will become it — see [`Self::parse_pipe`], which parks it
    /// here and restores the enclosing one afterwards, and [`Self::parse_hole`],
    /// which takes it. `None` inside a right side means an earlier `$_` already
    /// took it, which is the whole of `rule:expressions/pipeline-hole-once`'s
    /// upper bound.
    pipe_hole: Option<Expr>,
    /// How many `|>` right sides enclose the cursor. Only the *lower* bound
    /// needs this: with the slot alone, "no hole is waiting" cannot tell a
    /// second `$_` on one right side from a `$_` written nowhere near a `|>`.
    pipe_rhs_depth: u32,
    /// Set while the cursor is inside the body of a method named
    /// `constructor`, which is what
    /// `rule:php-migration/a-constructor-return-carries-no-value` refuses a
    /// returned value in. Every nested body parks it — see
    /// [`Self::in_callable_body`] — because a `return` written in an anonymous function,
    /// a property hook or a method declared inside a constructor leaves that
    /// body and not the constructor.
    in_constructor: bool,
    /// Set while the cursor is inside a `finally` block, which
    /// `rule:php-migration/no-return-leaves-a-finally` refuses a `return` in.
    /// [`Self::in_callable_body`] parks it exactly as it parks
    /// [`Self::in_constructor`], because a `return` written in an anonymous function or a
    /// method declared inside the block leaves that body and not the `finally`.
    in_finally: bool,
    /// How many loops and `switch`es have been opened since the innermost
    /// enclosing `finally` block began. A `break`/`continue` level above this
    /// names a target outside the block, which is what
    /// `rule:php-migration/no-return-leaves-a-finally` refuses; the count means
    /// nothing unless [`Self::in_finally`] is set.
    finally_breakables: u32,
    /// Every modifier list parsed so far, in source order and each in the
    /// order it was written — what [`Parsed`]'s own `modifiers` ends up
    /// holding. [`None`] on a compile path, which reads
    /// [`Modifier`](crate::ast::Modifier) alone and would otherwise pay an
    /// allocation per declaration for positions it never asks about.
    modifiers: Option<Vec<Vec<WrittenModifier>>>,
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
/// stack-depth probe. **This bounds the parse, not the stack a build runs it
/// on.** A release binary refuses every depth cleanly here, and so does
/// `cargo fuzz`'s ASan-instrumented release build, which carries larger frames
/// than either. A *debug* binary does not: its frames overflow a 1 MiB
/// main-thread stack a level or two past where this guard first reports, so a
/// nested array literal aborts the process at twenty `[` where the guard
/// reports `E0108` at nineteen. Probe a nesting limit with a release binary;
/// raising this constant widens that debug-only window rather than closing it.
const MAX_RECURSION_DEPTH: u32 = 96;

/// A saved parser position, for the three places this parser backtracks:
/// grammar that is genuinely ambiguous on a token prefix alone (a
/// type-then-`$name` local declaration versus an ordinary expression
/// statement, at statement position and again in a `for` header's init
/// clause; a destructuring target versus a plain array literal). Trying
/// the more specific production and restoring on a mismatch is simpler and
/// far less error-prone than hand-writing a lookahead classifier that
/// duplicates the type grammar.
struct Checkpoint<'src> {
    lexer: Lexer<'src>,
    lookahead: VecDeque<Token>,
    last_span: Span,
    diags_len: usize,
    /// How much trivia had been collected when this was taken. The vector
    /// itself stays with the live lexer — see [`Parser::checkpoint`].
    trivia_len: usize,
    /// How many modifier lists had been collected when this was taken, so a
    /// speculative parse that reached a declaration — a `new class { ... }`
    /// inside an expression — leaves none of its members' modifiers behind
    /// when it is undone.
    modifiers_len: usize,
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
            docs_attached: Vec::new(),
            docblocks_reported: Vec::new(),
            pipe_hole: None,
            pipe_rhs_depth: 0,
            in_constructor: false,
            in_finally: false,
            finally_breakables: 0,
            modifiers: None,
        }
    }

    /// Starts parsing `file` exactly as [`Self::new`] does, and keeps the two
    /// things a caller putting the file back together needs and a compile path
    /// never asks for: every run the grammar skips over (see
    /// [`Lexer::with_trivia`]) and where each modifier was written. The
    /// statements are the same either way: this is the one grammar, with a side
    /// channel (`rule:ide/one-grammar-one-tree`).
    #[must_use]
    pub fn with_trivia(file: &'src SourceFile, diags: &'d mut Diagnostics) -> Self {
        Self {
            lexer: Lexer::with_trivia(file),
            modifiers: Some(Vec::new()),
            ..Self::new(file, diags)
        }
    }

    /// Takes the trivia collected so far, leaving this parser able to collect
    /// more. Empty unless this parser came from [`Self::with_trivia`].
    pub fn take_trivia(&mut self) -> Vec<Trivia> {
        self.lexer.take_trivia()
    }

    /// Takes the modifier lists collected so far, leaving this parser able to
    /// collect more. Empty unless this parser came from [`Self::with_trivia`].
    pub fn take_modifiers(&mut self) -> Vec<Vec<WrittenModifier>> {
        self.modifiers
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
    }

    /// The `///` run attached to a declaration that starts at `at`, if there is
    /// one, recorded as attached so it is not reported as documenting nothing.
    ///
    /// `at` is the declaration's own first token — before its attributes, since
    /// the run is written above those. Attachment is read off the *source text*
    /// between the run and `at` rather than off the whitespace trivia beside
    /// it, because a compile path collects no whitespace
    /// ([`Lexer::with_trivia`]) and both paths have to answer identically.
    /// Trivia reaching past `at` belongs to the lookahead buffer — the lexer
    /// runs ahead of the token a production is looking at — and is skipped
    /// rather than mistaken for the end of the run.
    ///
    /// The walk stops at the first trivium of any kind whose gap to the run
    /// does not join: the trivia are in source order, and a gap that fails to
    /// join still fails with more text in front of it, so no `///` further back
    /// can join either. That bounds the walk by the trivia beside the
    /// declaration, so the trivia path, which collects every space and
    /// comment, parses a file of n declarations in O(n).
    pub(super) fn take_doc_comment(&mut self, at: Span) -> Option<DocComment> {
        let text = self.file.text();
        let mut lines = Vec::new();
        let mut next = at.start;
        for trivium in self.lexer.trivia().iter().rev() {
            if trivium.span.end > at.start {
                continue;
            }
            if !doc_run_joins(text, trivium.span.end, next) {
                break;
            }
            if trivium.kind != TriviaKind::DocComment {
                continue;
            }
            lines.push(trivium.span);
            next = trivium.span.start;
        }
        self.warn_docblock_above(next);
        let (&first, &last) = (lines.last()?, lines.first()?);
        lines.reverse();
        self.docs_attached.push(first.start);
        let tags = self.doc_tags(&lines);
        Some(DocComment {
            span: first.to(last),
            lines,
            tags,
        })
    }

    /// Warns about a `/** … */` block sitting directly above `next` — a
    /// declaration's first token, or the `///` run above it — across no blank
    /// line. That is the PHPDoc habit, and here it is an ordinary comment that
    /// documents nothing (`rule:tooling/doc-comment-is-three-slashes`); the
    /// warning is what tells a reader whose `nvs check` says `no errors` that
    /// their documentation is being dropped.
    ///
    /// Read off the source text between the block and `next`, as attachment
    /// is, so the compile path — which records a `/**` block and no other
    /// ordinary comment — and the trivia path answer identically. Reported
    /// once per block: a production may take one declaration's run twice.
    ///
    /// The walk stops at the first trivium whose gap to `next` does not join,
    /// for the reason [`Self::take_doc_comment`] gives, so it costs the trivia
    /// beside `next` and never the whole file's.
    fn warn_docblock_above(&mut self, next: BytePos) {
        let text = self.file.text();
        let Some(span) = self
            .lexer
            .trivia()
            .iter()
            .rev()
            .filter(|trivium| trivium.span.end <= next)
            .take_while(|trivium| doc_run_joins(text, trivium.span.end, next))
            .find(|trivium| trivium.kind == TriviaKind::BlockComment)
            .map(|trivium| trivium.span)
        else {
            return;
        };
        let body = text
            .get(span.start as usize..span.end as usize)
            .unwrap_or("");
        let docblock = body.starts_with("/**") && !body.starts_with("/**/");
        if !docblock || self.docblocks_reported.contains(&span.start) {
            return;
        }
        self.docblocks_reported.push(span.start);
        self.diags.report(
            Diagnostic::warning(
                code::W_DOC_BLOCK_BEFORE_A_DECLARATION,
                "this `/** … */` block documents nothing",
            )
            .with_primary(span, "an ordinary comment")
            .with_help("`/** … */` is an ordinary comment; write `///` for a doc comment"),
        );
    }

    /// The `@see` and `@example` lines of a run, reporting every other `@tag`
    /// written at the start of one
    /// (`rule:tooling/doc-comment-tags-are-see-and-example`).
    ///
    /// A tag is recognised only at the start of a line, past the marker and its
    /// indentation, so an `@` anywhere in the prose is a character. That is why
    /// this reads the lines rather than the run's text: the distinction lives
    /// nowhere else, and a run is otherwise just Markdown.
    fn doc_tags(&mut self, lines: &[Span]) -> Vec<DocTag> {
        let text = self.file.text();
        let mut tags = Vec::new();
        for line in lines {
            let Some(body) = text
                .get(line.start as usize..line.end as usize)
                .and_then(|line| line.get(DOC_MARKER.len()..))
            else {
                continue;
            };
            let indent = body.len() - body.trim_start().len();
            let Some(rest) = body[indent..].strip_prefix('@') else {
                continue;
            };
            let name_len = rest.bytes().take_while(u8::is_ascii_alphabetic).count();
            let (name, after) = rest.split_at(name_len);
            let lead = after.len() - after.trim_start().len();
            let written = after.trim();
            let at = line.start + in_line(DOC_MARKER.len() + indent);
            let span = Span::new(line.file, at, line.end);
            let argument_start = at + in_line(1 + name_len + lead);
            let argument = Span::new(
                line.file,
                argument_start,
                argument_start + in_line(written.len()),
            );
            let kind = match name {
                "see" => DocTagKind::See,
                "example" => DocTagKind::Example,
                _ => {
                    self.report_unknown_doc_tag(span, name);
                    continue;
                }
            };
            tags.push(DocTag {
                kind,
                span,
                argument,
            });
        }
        tags
    }

    /// Refuses one tag, naming what to write instead.
    ///
    /// The three PHPDoc spellings get their own help because each has a
    /// specific answer and an author reaching for one is not guessing — they
    /// are writing what every other PHP project taught them. Everything else,
    /// invented or merely retired, gets the rule: the set is two.
    fn report_unknown_doc_tag(&mut self, span: Span, name: &str) {
        let help = match name {
            "param" => {
                "name the parameter in a sentence instead — its type is already in the signature"
            }
            "return" | "returns" => {
                "the return type is already in the signature; a sentence says what the value means"
            }
            "throws" => {
                "say what it throws in a sentence — a `Core` member's errors are its registry card \
                 (`rule:core-api/reference-card`)"
            }
            _ => {
                "a doc comment is Markdown plus `@see` and `@example`, and the set is closed \
                 (`rule:tooling/doc-comment-tags-are-see-and-example`)"
            }
        };
        self.diags.report(
            Diagnostic::error(
                code::E_DOC_COMMENT_UNKNOWN_TAG,
                format!("`@{name}` is not a documentation tag"),
            )
            .with_primary(span, "not `@see` or `@example`")
            .with_help(help),
        );
    }

    /// Reports every `///` run no declaration took
    /// (`rule:tooling/doc-comment-attaches-to-the-next-declaration`).
    ///
    /// The runs are grouped here, at the end, rather than as the lexer produces
    /// them: a run is a fact about source text, and the same grouping has to
    /// answer for a file whose declarations attached none of them — including
    /// one with no declarations at all, which is where the marker is most often
    /// a note written with one slash too many.
    fn report_unattached_docs(&mut self) {
        let text = self.file.text();
        let docs: Vec<Span> = self
            .lexer
            .trivia()
            .iter()
            .filter(|trivium| trivium.kind == TriviaKind::DocComment)
            .map(|trivium| trivium.span)
            .collect();
        let mut run = 0;
        while run < docs.len() {
            let mut end = run + 1;
            while end < docs.len() && doc_run_joins(text, docs[end - 1].end, docs[end].start) {
                end += 1;
            }
            if !self.docs_attached.contains(&docs[run].start) {
                self.diags.report(
                    Diagnostic::error(
                        code::E_DOC_COMMENT_UNATTACHED,
                        "this doc comment is attached to nothing",
                    )
                    .with_primary(docs[run].to(docs[end - 1]), "documents nothing")
                    .with_help(
                        "a doc comment documents the declaration it precedes, across no blank \
                         line; write `//` for a note to the reader",
                    ),
                );
            }
            run = end;
        }
    }

    /// Parses statements until end of input — the body both whole-file entry
    /// points share, so neither can drift from the other.
    fn parse_all(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while !self.at(TokenKind::Eof) {
            let before = self.peek().span;
            stmts.push(self.parse_statement());
            // Mirrors `parse_block`'s guard: if a statement consumed nothing (a
            // production bailed out on an error), force progress so a malformed
            // file can't hang the parser in an infinite loop.
            if self.peek().span == before && !self.at(TokenKind::Eof) {
                self.bump();
            }
        }
        self.report_unattached_docs();
        self.check_one_namespace(&stmts);
        stmts
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
            kind: ExprKind::Error(span),
        }
    }

    /// Saves the current position, so a speculative parse can be undone by
    /// [`Self::restore`] if it turns out to be the wrong production.
    fn checkpoint(&mut self) -> Checkpoint<'src> {
        // The trivia collected so far is moved aside rather than cloned into
        // the checkpoint: one is taken per ambiguous statement and the vector
        // grows with the file, so cloning here would make a trivia-collecting
        // parse quadratic in the comments before the cursor. `restore`
        // truncates the live vector back to `trivia_len` instead, which is the
        // same rewind the clone would have been.
        let trivia = self.lexer.take_trivia();
        let lexer = self.lexer.clone();
        let trivia_len = trivia.len();
        self.lexer.put_trivia(trivia);
        Checkpoint {
            lexer,
            lookahead: self.lookahead.clone(),
            last_span: self.last_span,
            diags_len: self.diags.len(),
            trivia_len,
            modifiers_len: self.modifiers.as_ref().map_or(0, Vec::len),
        }
    }

    /// Undoes every token consumed and every diagnostic reported since
    /// `cp` was taken.
    fn restore(&mut self, cp: Checkpoint<'src>) {
        let mut trivia = self.lexer.take_trivia();
        trivia.truncate(cp.trivia_len);
        self.lexer = cp.lexer;
        self.lexer.put_trivia(trivia);
        self.lookahead = cp.lookahead;
        self.last_span = cp.last_span;
        self.diags.truncate(cp.diags_len);
        if let Some(runs) = self.modifiers.as_mut() {
            runs.truncate(cp.modifiers_len);
        }
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
    /// (`rule:classes/reserved-spellings-are-lower-case`). `SPAWN` is just an identifier.
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

    /// Parses a nested body with [`Self::in_constructor`] set to `constructor`
    /// and the `finally` state cleared, restoring both afterwards. Every body a
    /// `return` can belong to goes through here, so the two flags answer "does
    /// a `return` here leave the constructor" and "does it leave the `finally`"
    /// rather than "is either one anywhere above this".
    fn in_callable_body<T>(&mut self, constructor: bool, f: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.in_constructor, constructor);
        let outer_finally = std::mem::replace(&mut self.in_finally, false);
        let outer_breakables = std::mem::replace(&mut self.finally_breakables, 0);
        let parsed = f(self);
        self.in_constructor = outer;
        self.in_finally = outer_finally;
        self.finally_breakables = outer_breakables;
        parsed
    }

    /// Parses a `finally` block with [`Self::in_finally`] set and the count of
    /// breakables started fresh, restoring both afterwards. Starting the count
    /// at zero is what makes a `break` inside the block measure against the
    /// loops the block itself opens rather than the ones around the `try`.
    fn in_finally_body<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.in_finally, true);
        let outer_breakables = std::mem::replace(&mut self.finally_breakables, 0);
        let parsed = f(self);
        self.in_finally = outer;
        self.finally_breakables = outer_breakables;
        parsed
    }

    /// Parses the body of a loop or a `switch` with [`Self::finally_breakables`]
    /// raised, which is the one thing that tells a `break` targeting a loop
    /// written inside a `finally` from one reaching past the block.
    fn in_breakable_body<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        self.finally_breakables += 1;
        let parsed = f(self);
        self.finally_breakables -= 1;
        parsed
    }

    /// `rule:php-migration/let-and-is-are-reserved`: neither word may name
    /// anything, and the two are refused for different reasons. `let` holds a
    /// spelling with no construct behind it; `is` is the type-test operator
    /// (`rule:types/type-test`), which is a construct that simply cannot stand
    /// where a name does. The label says which of the two this is, and the
    /// help names the living spelling the program should reach for.
    fn report_reserved_for_future_use(&mut self, word: Keyword, span: Span) {
        let (spelling, label, living) = if word == Keyword::Let {
            (
                "let",
                "reserved for a future construct — it names nothing here",
                "`var` declares an inferred local",
            )
        } else {
            (
                "is",
                "the type-test operator — it names nothing here",
                "`$x is T` tests a value against a type and `as` converts",
            )
        };
        self.diags.report(
            Diagnostic::error(
                code::E_RESERVED_FOR_FUTURE_USE,
                format!("`{spelling}` is a reserved spelling"),
            )
            .with_primary(span, label)
            .with_help(format!(
                "{living}; rename any `{spelling}` the program used as a name"
            )),
        );
    }

    /// `rule:statements/no-function-static-and-no-global`: `static` is not a modifier of an anonymous
    /// function. An anonymous function captures `$this` only if it uses it, so the keyword has nothing
    /// left to mean. Reports and keeps going; the caller still builds the anonymous function's node.
    fn report_static_anon_fn_modifier(&mut self, span: Span) {
        self.diags.report(
            Diagnostic::error(
                code::E_STATIC_ANON_FN_UNSUPPORTED,
                "`static` is not allowed before an anonymous function",
            )
            .with_primary(
                span,
                "an anonymous function captures `$this` only when it uses it",
            )
            .with_help("drop `static` (`rule:statements/no-function-static-and-no-global`)"),
        );
    }

    /// `rule:statements/ampersand-is-not-a-by-reference-marker`: `&` is no longer a by-reference marker. The marker is
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
                "`&` is not a by-reference marker in Novis",
            )
            .with_primary(amp, "`&` is bitwise AND, and an intersection type")
            .with_help(fix.to_string()),
        );
    }

    /// A declaration's initializer, after its `=`. `int $b = &$a;` is PHP's
    /// reference assignment written as a declaration, so the `&` is reported
    /// as [`by_reference_assignment`] and the value after it is kept, which
    /// lets the declaration's type still be checked against it.
    fn parse_decl_initializer(&mut self, name: Span) -> Expr {
        if self.eat(TokenKind::Amp).is_none() {
            return self.parse_expr();
        }
        let value = self.parse_expr();
        let value_text = self
            .file
            .span_text(value.span)
            .unwrap_or_default()
            .to_owned();
        self.diags
            .report(by_reference_assignment(name.to(value.span), &value_text));
        value
    }

    /// The help `rule:statements/no-host-populated-variables` gives for a PHP superglobal, or `None` if `name`
    /// (the raw `$…` text) is not one.
    ///
    /// **Each accessor arm names the `Core` member that replaces the variable**,
    /// as the rule's table maps it, because the rule has the refusal name its
    /// replacement. A name written here is a second copy of the registry's,
    /// and this crate cannot reach the registry, so `nvs-types`' test
    /// `every_core_member_a_refused_superglobal_names_is_registered` holds
    /// each one against `nvs_stdlib::registry`.
    fn superglobal_replacement(name: &str) -> Option<&'static str> {
        Some(match name {
            "$GLOBALS" => {
                "`$GLOBALS` does not exist; declare a `static` property, a constant, or pass the \
                 value as a parameter"
            }
            "$_REQUEST" => {
                "`$_REQUEST` does not exist, and neither does anything merging query, body and \
                 cookie input into one place — read each explicitly, so the source is visible at \
                 the call site (`rule:statements/no-host-populated-variables`)"
            }
            "$_GET" => {
                "use `Core\\Request::query` — it returns one query parameter of the current request"
            }
            "$_POST" => {
                "use `Core\\Request::post` — it returns one form field of the current request"
            }
            "$_COOKIE" => {
                "use `Core\\Request::cookie` — it returns one cookie of the current request"
            }
            "$_FILES" => {
                "use `Core\\Request::files` — it returns the files uploaded with the current \
                 request"
            }
            "$_SERVER" => {
                "use `Core\\Request` for the request — `Core\\Request::method`, \
                 `Core\\Request::path` and `Core\\Request::header` — and `Core\\Server` for the \
                 server"
            }
            "$_SESSION" => {
                "call `Core\\Session::start`, then use `Core\\Session::get` and \
                 `Core\\Session::set`"
            }
            "$_ENV" => "use `Core\\Env::get` — it returns one environment variable",
            "$argv" | "$argc" => {
                "use `Core\\Cli::arguments` — it returns the command-line arguments as an array"
            }
            "$_ARGS" => {
                "use `Core\\Script::args` — it returns the value the script was started with"
            }
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
            | ExprKind::Error(_)
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

/// What opens a doc comment (`rule:tooling/doc-comment-is-three-slashes`). The
/// lexer decides that a trivium is one; this is only how many bytes to step
/// over to reach the line's content.
///
/// Exported because a [`DocComment`] holds its lines as spans and leaves the
/// stripping to whoever reads the prose — `nvs_lsp::hover` is the first such
/// reader, and a second spelling of `///` in another crate is a place the two
/// could disagree about where a line's content starts.
pub const DOC_MARKER: &str = "///";

/// A byte count inside one doc-comment line, as an offset to add to that line's
/// own start. A source file is bounded by `MAX_SOURCE_LEN`, so a line inside one
/// always fits.
fn in_line(bytes: usize) -> BytePos {
    BytePos::try_from(bytes).expect("an offset within one line of a source file")
}

/// Whether `[from, to)` is nothing but whitespace crossing at most one line
/// break — the gap that keeps two `///` lines in one run, and the gap that
/// keeps a run attached to the declaration under it
/// (`rule:tooling/doc-comment-attaches-to-the-next-declaration`). A blank line
/// crosses two, which is what ends a run and detaches it; anything that is not
/// whitespace ends it as well, so an ordinary `//` between the two separates
/// them rather than being absorbed.
fn doc_run_joins(text: &str, from: BytePos, to: BytePos) -> bool {
    let Some(gap) = text.get(from as usize..to as usize) else {
        return false;
    };
    gap.chars().all(char::is_whitespace) && gap.bytes().filter(|byte| *byte == b'\n').count() <= 1
}

/// `E0701` — a reference assignment, refused rather than lowered. `span` is the
/// whole assignment and `value_text` the right-hand side after the `&`.
///
/// Novis has no references: `rule:types/implicit-capture` removed by-reference capture, so no
/// binding aliases another, and `rule:classes/two-copy-depths` fixes what a copy means, so the
/// right-hand side is a copy at the point the assignment runs. The `&` has no
/// owner in either rule — the same reasoning the `[&$x]` refusal (`E0483`)
/// already states, and the reason both are refusals rather than missing
/// lowerings.
///
/// Two places report it, so it is built here once: the parser for a
/// declaration's initializer (`int $b = &$a;`), whose AST keeps no `&`, and
/// `nvs_types` for an assignment expression (`$b = &$a;`), whose AST does.
/// `value_text` is quoted back because dropping one character is the whole fix.
#[must_use]
pub fn by_reference_assignment(span: Span, value_text: &str) -> Diagnostic {
    Diagnostic::error(
        code::E_ASSIGN_BY_REFERENCE,
        "a binding cannot be assigned by reference",
    )
    .with_primary(span, format!("this would share `{value_text}`'s own slot"))
    .with_help(
        "Novis has no references: `rule:types/implicit-capture` removed by-reference capture and `rule:classes/two-copy-depths` makes \
         this a copy, so drop the `&` — `inout` is a parameter and binding mode (`rule:statements/inout-is-the-by-reference-spelling`), \
         not a way to make two names one place, and to share one mutable cell you hold it in \
         an object and assign that",
    )
}

/// Parses a single expression from `file`, for tests and tools that want just
/// that much. Trailing tokens after the expression are left unconsumed.
#[must_use]
pub fn parse_expression(file: &SourceFile, diags: &mut Diagnostics) -> Expr {
    Parser::new(file, diags).parse_expr()
}

/// A whole file, parsed: its statements, the runs between them that the
/// grammar skipped, where each modifier of it was written, and where each node
/// of it is.
///
/// The fields `rule:ide/one-grammar-one-tree` names, and no second tree: the
/// index holds spans and kinds, every node it points at is a [`Stmt`] in
/// `stmts`, and `modifiers` points into the same text those spans do.
#[derive(Debug)]
pub struct Parsed {
    /// Every top-level statement, in source order — exactly what
    /// [`parse_file`] returns on its own.
    pub stmts: Vec<Stmt>,
    /// Every whitespace run and every comment, in source order
    /// (`rule:ide/tokens-plus-trivia-reproduce-the-file`).
    pub trivia: Vec<Trivia>,
    /// Every modifier list the file wrote, in source order, each list in the
    /// order its author wrote it and each modifier carrying the span it was
    /// written at. A [`Modifier`](crate::ast::Modifier) says what a
    /// declaration is; this says where the word is, which is what a formatter
    /// ordering a list needs and what a compile path never asks for.
    pub modifiers: Vec<Vec<WrittenModifier>>,
    /// Which node a byte offset is inside, and what that node is inside
    /// (`rule:ide/the-index-answers-the-cursor`). Built here rather than on
    /// demand because it is rebuilt per analysis either way, and a `Parsed`
    /// that answers about positions is what separates this entry point from
    /// [`parse_file`]; [`crate::index`] is what one walk over `stmts` costs.
    pub index: SyntaxIndex,
}

/// Parses a whole file top to bottom and keeps what it skipped, for a caller
/// that has to put the file back together — a formatter, an editor, or the
/// documentation a `///` carries.
///
/// Same grammar, same statements and same diagnostics as [`parse_file`]; the
/// trivia is a side channel, not a second parse (`rule:ide/one-grammar-one-tree`).
#[must_use]
pub fn parse(file: &SourceFile, diags: &mut Diagnostics) -> Parsed {
    let mut parser = Parser::with_trivia(file, diags);
    let stmts = parser.parse_all();
    let index = SyntaxIndex::of_stmts(&stmts);
    Parsed {
        stmts,
        trivia: parser.take_trivia(),
        modifiers: parser.take_modifiers(),
        index,
    }
}

/// Parses a whole file top to bottom: the `nvs ast`/corpus-parse entry point
/// M1's plan names, and the one place the file always starts in HTML mode
/// (spec `00-overview.md` § 1) rather than a test's manually-bumped open tag.
/// Errors are reported into `diags` rather than stopping the parse — callers
/// that only care whether it parsed cleanly should check
/// [`Diagnostics::has_errors`] afterwards.
///
/// This is the strict half of the pair: every compile path calls it, it keeps
/// no trivia, and it exists beside [`parse`] so that adding the lossless mode
/// changed no call site.
#[must_use]
pub fn parse_file(file: &SourceFile, diags: &mut Diagnostics) -> Vec<Stmt> {
    Parser::new(file, diags).parse_all()
}
