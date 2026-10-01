//! Statements: every control-flow form, `echo`, `unset`, `rule:types/grammar`.1's
//! typed local declaration and § 3.3's destructuring — and the three places
//! the parser has to guess and take it back.
//!
//! `if`/`elseif`/`else`, `while`, `do`/`while`, `for`, `foreach` with `rule:types/grammar`.2's mandatory typed bindings, `switch`, `break`/`continue`,
//! `try`/`catch`/`finally`, and the statement-shaped rejects (`global`,
//! `goto`, function-scope `static`) are all here, as is `rule:types/var-inference`'s `var $x =
//! e;`.
//!
//! # The three backtracking sites
//!
//! All three live here, because all three are ambiguous on a token prefix
//! alone: a typed local declaration versus an ordinary expression statement
//! that happens to start with a name (`Foo $x = ...;` versus `Foo::bar();`),
//! a destructuring target versus a plain array-literal expression statement
//! (`[int $a] = $p;` versus `[1, 2, 3];`), and `rule:iteration/for-init-clause`'s `for` init
//! clause, which is the same first ambiguity inside a header rather than at
//! statement position (`for (int $i = 0; …)` versus
//! `for ($i = Foo::bar(); …)`). Each trial-parses the more specific
//! production and [`Parser::restore`]s a [`Checkpoint`] if the deciding token
//! — a `Variable` after the type, a `=` after the pattern — doesn't show up,
//! rather than a hand-written lookahead classifier that would duplicate
//! [`Parser::parse_type`]'s grammar and drift from it. See
//! [`Parser::parse_stmt_maybe_local_decl`],
//! [`Parser::parse_stmt_maybe_destructure`] and [`Parser::parse_for_init`].
//!
//! Part of [`super`]'s one `impl Parser`, split across this directory so a
//! session editing one layer of the grammar does not carry the rest in
//! context. Every item moved here unchanged; the methods are `pub(super)` so
//! they reach across these modules and no further, which is the reach they had
//! when `parser` was a single file.

use super::*;

/// How far past a statement-initial `{` the shape-typed-local tell looks for
/// the matching `}`. A shape a local would be declared with is a handful of
/// tokens; past this the tell gives up and the `{` is read as the block or
/// object literal it otherwise looks like, which costs a worse message on
/// input no author writes. This is what keeps the scan — the one place the
/// parser looks further ahead than three tokens — buffering a bounded number
/// of tokens rather than however many the braced run holds.
const SHAPE_TYPED_LOCAL_SCAN_LIMIT: usize = 256;

impl<'src, 'd> Parser<'src, 'd> {
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
    pub(super) fn parse_statement(&mut self) -> Stmt {
        self.guarded(Self::error_stmt_here, Self::parse_statement_inner)
    }

    /// Whether the `{` at the current position opens a *shape type* written in
    /// front of a local's name (`{x: int} $point;`, `{x?: int} $point;`)
    /// rather than a block or an object literal, answering with how far the
    /// matching `}` is from here so the refusal can consume the run this
    /// identified.
    ///
    /// Both halves are load-bearing. The braced run has to open like a field
    /// list, which is what keeps a block a variable happens to follow
    /// (`{ echo 1; } $x = 1;`) out of it, and the token after the *matched*
    /// `}` has to be a variable, which is what keeps a discarded object
    /// literal (`{x: 1};`) out. An optional key (`{x?: int}`) opens a field
    /// list too: it is the same declaration written in the same place, and
    /// only [`Self::at_object_literal_in_block_position`], which answers for a
    /// *literal*, has no spelling for it.
    fn at_shape_typed_local(&mut self) -> Option<usize> {
        if self.peek().kind != TokenKind::LBrace || self.peek_at(1).kind != TokenKind::Ident {
            return None;
        }
        let opens_a_field = match self.peek_at(2).kind {
            TokenKind::Colon => true,
            TokenKind::Question => self.peek_at(3).kind == TokenKind::Colon,
            _ => false,
        };
        if !opens_a_field {
            return None;
        }
        let mut depth = 0usize;
        for n in 0..SHAPE_TYPED_LOCAL_SCAN_LIMIT {
            match self.peek_at(n).kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return (self.peek_at(n + 1).kind == TokenKind::Variable).then_some(n);
                    }
                }
                TokenKind::Eof => return None,
                _ => {}
            }
        }
        None
    }

    /// `{x: int} $point;` — the refusal `rule:types/shape-type` leaves this
    /// position with. Every other declaration slot (parameter, return type,
    /// property, class constant, `foreach` binding) takes a bare shape, and a
    /// local's cannot, because a statement-initial `{` is a block first. The
    /// shape is not the problem, so the message names the spelling that works.
    ///
    /// `close` is where [`Self::at_shape_typed_local`] found the matching `}`.
    /// That run, the variable behind it and whatever reaches the `;` are
    /// consumed and the statement becomes `Empty`: parsing `{x: int}` as a
    /// block would report the field list as broken statements, which is the
    /// cascade this error exists to replace.
    fn parse_shape_typed_local_refusal(&mut self, close: usize) -> Stmt {
        let start = self.peek().span;
        for _ in 0..=close {
            self.bump();
        }
        let shape = start.to(self.last_span);
        self.diags.report(
            Diagnostic::error(
                code::E_SHAPE_TYPED_LOCAL_NEEDS_AN_ALIAS,
                "a local declaration cannot be typed with a bare shape",
            )
            .with_primary(
                shape,
                "`{` at the start of a statement already opens a block",
            )
            .with_help(
                "name the shape first: `type Point = {x: int}; Point $point;` \
                 (`rule:types/shape-type`)",
            ),
        );
        self.bump(); // the variable the tell found behind the `}`
        while !self.at(TokenKind::Semicolon)
            && !self.at(TokenKind::Eof)
            && !self.at(TokenKind::RBrace)
        {
            self.bump();
        }
        if self.at(TokenKind::Semicolon) {
            self.bump();
        }
        Stmt {
            span: start.to(self.last_span),
            kind: StmtKind::Empty,
        }
    }

    pub(super) fn error_stmt_here(&mut self) -> Stmt {
        let span = self.peek().span.shrink_to_start();
        Stmt {
            span,
            kind: StmtKind::Empty,
        }
    }

    pub(super) fn parse_statement_inner(&mut self) -> Stmt {
        // --- HTML-mode round trip, spec 00-overview.md § 1 ---------------------
        // `?>`/`<?nvs` can reopen or reclose code mode anywhere a statement is
        // expected, not just at file scope — e.g. `if ($x) { ?>html<?nvs }` is
        // legal, exactly as in PHP. Skip every bare tag token here; stop at
        // the first token that is either real inline HTML (one `InlineHtml`
        // statement) or ordinary code. Nothing here recurses into
        // `self.parse_statement()`, so a tag run can't loop forever even when
        // the file ends right after `?>`.
        loop {
            match self.peek().kind {
                TokenKind::CloseTag | TokenKind::OpenTagNvs => {
                    self.bump();
                }
                TokenKind::OpenTagPhp => {
                    // `rule:statements/nvs-is-the-only-open-tag`: `<?nvs` is the only code-mode open tag —
                    // the lexer still recognizes `<?php` (same reason `eval`
                    // still lexes as a keyword) purely so this can name the
                    // fix instead of misreading it as inline HTML.
                    let span = self.bump().span;
                    self.diags.report(
                        Diagnostic::error(
                            code::E_PHP_OPEN_TAG_UNSUPPORTED,
                            "`<?php` is not supported",
                        )
                        .with_primary(
                            span,
                            "use `<?nvs` instead — it is the only code-mode open tag Novis keeps",
                        )
                        .with_help("`<?nvs` accepts exactly the same code that followed `<?php`"),
                    );
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
            // `}` or on EOF (`<?nvs if ($x) { ?><?php }`, or a file that ends
            // right after `?>`). The caller's own loop (`parse_block`,
            // `parse_file`) is what notices and stops, not this function.
            TokenKind::RBrace | TokenKind::Eof => Stmt {
                span: start,
                kind: StmtKind::Empty,
            },
            TokenKind::LBrace => {
                if let Some(close) = self.at_shape_typed_local() {
                    self.parse_shape_typed_local_refusal(close)
                } else if self.at_object_literal_in_block_position() {
                    // `rule:types/object-literal`: a statement-initial `{` already means a
                    // block — a discarded object-literal statement needs
                    // `({...});` instead.
                    let expr = self.parse_object_literal_needs_parens();
                    self.expect(TokenKind::Semicolon, "`;`");
                    let span = start.to(self.last_span);
                    Stmt {
                        span,
                        kind: StmtKind::Expr(expr),
                    }
                } else {
                    let block = self.parse_block();
                    Stmt {
                        span: block.span,
                        kind: StmtKind::Block(block),
                    }
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
                // `rule:php-migration/a-constructor-return-carries-no-value`:
                // the object under construction is the result, so a value here
                // has nowhere to go. A bare `return;` is untouched — it still
                // leaves early — and the node keeps the value it was written
                // with, so the rest of the body reports its own problems in
                // this run.
                if self.in_constructor && value.is_some() {
                    self.diags.report(
                        Diagnostic::error(
                            code::E_CONSTRUCTOR_RETURN_CARRIES_A_VALUE,
                            "a constructor's `return` carries no value",
                        )
                        .with_primary(span, "the object under construction is the result")
                        .with_help(
                            "drop the value — a bare `return;` still leaves the constructor \
                             early, provided every property is assigned on that path",
                        ),
                    );
                }
                // `rule:php-migration/no-return-leaves-a-finally`: this would
                // replace whatever the region was leaving with, including a
                // throw in flight — the one construct where an unhandled
                // exception vanishes with no handler anywhere in the program.
                // A `return` in a body nested inside the block leaves that
                // body instead, which is what parking the flag answers.
                if self.in_finally {
                    self.diags.report(
                        Diagnostic::error(
                            code::E_RETURN_LEAVES_A_FINALLY,
                            "a `return` never leaves a `finally` block",
                        )
                        .with_primary(span, "this discards what the region was leaving with")
                        .with_help(
                            "return from the `try` or a `catch` to change the result, or write \
                             it after the whole region",
                        ),
                    );
                }
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
            TokenKind::Keyword(Keyword::Var) => self.parse_var_local_decl(start),
            TokenKind::LBracket => self.parse_stmt_maybe_destructure(start),
            TokenKind::AttributeOpen => self.parse_attributed_decl_stmt(start),
            // `rule:types/class-reference`: `class<` at statement start is a typed local
            // holding a class reference, not a declaration — the one place the
            // two spellings meet, and one token of lookahead separates them
            // (`Parser::at_class_reference`). `abstract`/`final` never precede
            // a type, so only the bare-`class` half of this arm needs asking.
            TokenKind::Keyword(Keyword::Abstract | Keyword::Final | Keyword::Class)
                if !self.at_class_reference() =>
            {
                self.parse_class_decl(start)
            }
            TokenKind::Keyword(Keyword::Interface) => self.parse_interface_decl(start),
            TokenKind::Keyword(Keyword::Trait) => self.parse_trait_decl(start),
            TokenKind::Keyword(Keyword::Enum) => self.parse_enum_decl(start),
            TokenKind::Keyword(Keyword::Namespace) => self.parse_namespace_decl(start),
            TokenKind::Keyword(Keyword::Use) => self.parse_use_decl(start),
            TokenKind::Keyword(Keyword::Autoload) => self.parse_autoload_decl(start),
            TokenKind::Keyword(Keyword::Const) => {
                self.parse_toplevel_const_reject(start, Vec::new())
            }
            TokenKind::Keyword(Keyword::Function) if self.at_named_function_decl() => {
                self.parse_toplevel_function_reject(start, Vec::new())
            }
            _ if self.at_type_alias() => self.parse_type_alias_decl(start),
            // `(int)$x;` etc. at statement start is also a syntactically
            // valid (if pointless) redundantly-parenthesized local
            // declaration with no initializer — `(int)` parses fine as a
            // one-member parenthesized union, so `parse_stmt_maybe_local_decl`'s
            // trial parse would otherwise commit to that reading and never
            // report `rule:types/no-legacy-cast`'s diagnostic for this one, statement-start
            // spelling of the rejected cast. The seven legacy-cast keywords
            // never legitimately need redundant parens around a bare type,
            // so this shape is routed to the expression-statement path
            // instead, unconditionally, ahead of the general trial parse.
            _ if self.at(TokenKind::LParen) && self.peek_cast_keyword().is_some() => {
                self.parse_expr_statement(start)
            }
            _ if self.can_start_type() && !self.at_keyword(Keyword::Static) => {
                self.parse_stmt_maybe_local_decl(start)
            }
            _ => self.parse_expr_statement(start),
        }
    }

    pub(super) fn parse_expr_statement(&mut self, start: Span) -> Stmt {
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
    pub(super) fn finish_if(&mut self, start: Span) -> Stmt {
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

    pub(super) fn parse_while(&mut self, start: Span) -> Stmt {
        self.bump();
        self.expect(TokenKind::LParen, "`(`");
        let cond = self.parse_expr();
        self.expect(TokenKind::RParen, "`)`");
        let body = Box::new(self.in_breakable_body(Self::parse_statement));
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::While { cond, body },
        }
    }

    pub(super) fn parse_do_while(&mut self, start: Span) -> Stmt {
        self.bump();
        let body = Box::new(self.in_breakable_body(Self::parse_statement));
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
    pub(super) fn parse_expr_list_until(&mut self, closer: TokenKind) -> Vec<Expr> {
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

    /// `rule:iteration/for-init-clause`: a `for` header's init clause is either one `rule:types/grammar`.1 typed local declaration or an expression list, never both. Both
    /// shapes are a comma-separated run of items, so the clause is parsed as
    /// one — each item through [`Self::try_parse_for_decl`] — and a run that
    /// turns out to hold two declarations or one of each is § 3's diagnostic
    /// rather than a parse error about a semicolon. The condition and step
    /// clauses are unchanged.
    pub(super) fn parse_for_init(&mut self) -> ForInit {
        let mut decl: Option<Stmt> = None;
        let mut exprs = Vec::new();
        let mut reported = false;
        if self.at(TokenKind::Semicolon) {
            return ForInit::Exprs(exprs);
        }
        loop {
            let item = self.peek().span;
            match self.try_parse_for_decl() {
                // `rule:iteration/for-init-refusals`: one diagnostic per header, at the item that
                // first breaks § 1's rule. A second one would only describe
                // the same header again.
                Some(second) if decl.is_some() => {
                    if !reported {
                        reported = true;
                        self.diags.report(
                            Diagnostic::error(
                                code::E_FOR_INIT_TWO_DECLARATIONS,
                                "a `for` init clause declares at most one binding",
                            )
                            .with_primary(
                                second.span,
                                "this is the header's second declaration (`rule:iteration/for-init-clause`)",
                            )
                            .with_help(
                                "declare the second binding above the loop — the counter is \
                                 function-scoped either way (`rule:iteration/for-counter-scope`)",
                            ),
                        );
                    }
                }
                // The declaration is kept even when it arrived after an
                // expression, so a rejected header still declares its counter
                // and § 3's one diagnostic is not followed by an `E0301` for
                // every use of it — the cascade this ADR exists to delete.
                Some(first) => {
                    if !exprs.is_empty() {
                        self.report_for_init_mixed(item, &mut reported);
                    }
                    decl = Some(first);
                }
                None => {
                    let e = self.parse_expr();
                    if decl.is_some() {
                        self.report_for_init_mixed(item, &mut reported);
                    } else {
                        exprs.push(e);
                    }
                }
            }
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        match decl {
            Some(decl) => ForInit::Decl(Box::new(decl)),
            None => ForInit::Exprs(exprs),
        }
    }

    /// `rule:iteration/for-init-refusals`'s `E0124`, raised where the item that mixes the two
    /// forms begins — in either order, since neither is more wrong than the
    /// other.
    fn report_for_init_mixed(&mut self, item: Span, reported: &mut bool) {
        if *reported {
            return;
        }
        *reported = true;
        self.diags.report(
            Diagnostic::error(
                code::E_FOR_INIT_MIXES_DECL_AND_EXPR,
                "a `for` init clause holds one declaration or a list of expressions, not both",
            )
            .with_primary(
                item,
                "this item is the other kind (`rule:iteration/for-init-clause`)",
            )
            .with_help(
                "move the extra initialiser above the loop, or make every item an expression",
            ),
        );
    }

    /// One init-clause item, if it is a declaration. Which one it is cannot
    /// be decided from the first token — a `Name` starts both a class type
    /// and a constant expression, and only the `$` after it settles it — so
    /// the declaration is tried under a checkpoint and undone when no
    /// variable follows, the same backtracking
    /// [`Self::parse_stmt_maybe_local_decl`] does at statement position.
    fn try_parse_for_decl(&mut self) -> Option<Stmt> {
        let start = self.peek().span;
        if self.at_keyword(Keyword::Var) {
            self.bump();
            return Some(self.parse_for_decl_tail(start, None));
        }
        if self.can_start_type() {
            let cp = self.checkpoint();
            let ty = self.parse_type();
            // The same signal `parse_stmt_maybe_local_decl` trusts: a
            // malformed type can still land on a `$variable` by coincidence,
            // so a diagnostic reported during the trial is what says this was
            // never a type.
            if self.at(TokenKind::Variable) && self.diags.len() == cp.diags_len {
                return Some(self.parse_for_decl_tail(start, Some(ty)));
            }
            self.restore(cp);
        }
        None
    }

    /// The tail of a `for` init declaration, both spellings, minus the `;` —
    /// the header's own first semicolon terminates it and
    /// [`Self::parse_for`] is what expects that. `ty: None` is `rule:types/var-inference`'s
    /// `var`, whose initializer is mandatory for the same reason it is at
    /// statement position: there is nothing else to infer the type from.
    fn parse_for_decl_tail(&mut self, start: Span, ty: Option<Type>) -> Stmt {
        let name = self.expect(TokenKind::Variable, "a variable name");
        let value = if ty.is_none() {
            self.expect(
                TokenKind::Equals,
                "`=` — `var` infers its type from the initializer, so one is required",
            );
            Some(self.parse_decl_initializer(name))
        } else {
            self.eat(TokenKind::Equals)
                .map(|_| self.parse_decl_initializer(name))
        };
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::LocalDecl { ty, name, value },
        }
    }

    pub(super) fn parse_for(&mut self, start: Span) -> Stmt {
        self.bump();
        self.expect(TokenKind::LParen, "`(`");
        let init = self.parse_for_init();
        self.expect(TokenKind::Semicolon, "`;`");
        let cond = self.parse_expr_list_until(TokenKind::Semicolon);
        self.expect(TokenKind::Semicolon, "`;`");
        let step = self.parse_expr_list_until(TokenKind::RParen);
        self.expect(TokenKind::RParen, "`)`");
        let body = Box::new(self.in_breakable_body(Self::parse_statement));
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

    /// One `'inout'? (type | 'var') '$' identifier` binding — the shared tail
    /// of both `foreach`-target alternatives (`rule:types/grammar`.2,
    /// `rule:statements/inout-is-the-by-reference-spelling`,
    /// `rule:types/var-inference`). The marker is parsed here and reported back
    /// to the caller, since only the *value* position may carry one; the key
    /// position never calls this with a marker present without the caller
    /// first checking for one.
    pub(super) fn parse_foreach_binding(&mut self) -> (ForeachBinding, bool) {
        let start = self.peek().span;
        let inout = self.eat_keyword(Keyword::Inout).is_some();
        let ty = if let Some(var) = self.eat_keyword(Keyword::Var) {
            ForeachBindingTy::Var(var)
        } else if self.can_start_type() {
            ForeachBindingTy::Written(self.parse_type())
        } else {
            let span = self.peek().span.shrink_to_start();
            self.diags.report(
                Diagnostic::error(
                    code::E_EXPECTED_TOKEN,
                    "expected a `foreach` binding's type, or `var`",
                )
                .with_primary(
                    span,
                    "a `foreach` binding writes its type, or `var` to take the subject's \
                     element type (`rule:types/grammar`.2)",
                ),
            );
            ForeachBindingTy::Omitted
        };
        if let Some(amp) = self.eat(TokenKind::Amp) {
            self.report_by_reference_marker(amp, "write `inout` before the binding's type");
        }
        let name = self.expect(TokenKind::Variable, "a `foreach` binding name");
        let span = start.to(self.last_span);
        (ForeachBinding { ty, name, span }, inout)
    }

    /// `foreach (subject as key? value) body`, `rule:types/grammar`.2. The header's
    /// own `as` is looked for explicitly after a suppressed-`as` subject
    /// parse (see [`Self::parse_expr_no_top_as`]), and the first binding is
    /// re-read as the key only once a `=>` confirms it was one — an `inout`
    /// on the first binding can only mean the no-key form (`foreach ($x as
    /// inout int $v)`), since the two-binding form's marker sits on the
    /// *second* binding instead.
    pub(super) fn parse_foreach(&mut self, start: Span) -> Stmt {
        self.bump();
        self.expect(TokenKind::LParen, "`(`");
        let subject = self.parse_expr_no_top_as();
        self.expect_keyword(Keyword::As, "`as`");
        let (first, first_inout) = self.parse_foreach_binding();
        let (key, value, value_inout) = if !first_inout && self.eat(TokenKind::FatArrow).is_some() {
            let (value, value_inout) = self.parse_foreach_binding();
            (Some(first), value, value_inout)
        } else {
            (None, first, first_inout)
        };
        self.expect(TokenKind::RParen, "`)`");
        let body = Box::new(self.in_breakable_body(Self::parse_statement));
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::Foreach {
                subject,
                key,
                value,
                value_inout,
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
    pub(super) fn parse_switch(&mut self, start: Span) -> Stmt {
        self.bump();
        self.expect(TokenKind::LParen, "`(`");
        let subject = self.parse_expr();
        self.expect(TokenKind::RParen, "`)`");
        self.expect(TokenKind::LBrace, "`{`");
        let cases = self.in_breakable_body(|p| {
            let mut cases = Vec::new();
            while !p.at(TokenKind::RBrace) && !p.at(TokenKind::Eof) {
                let before = p.peek().span;
                cases.push(p.parse_switch_case());
                if p.peek().span == before && !p.at(TokenKind::RBrace) && !p.at(TokenKind::Eof) {
                    p.bump();
                }
            }
            cases
        });
        self.expect(TokenKind::RBrace, "`}`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::Switch { subject, cases },
        }
    }

    pub(super) fn parse_switch_case(&mut self) -> SwitchCase {
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

    /// The level a `break`/`continue` was written with: an absent one is 1, and
    /// an integer literal is its own digits. Anything else is a level this
    /// parser cannot read — a level is written, never computed — so it answers
    /// `None` and leaves the spelling to the pass that refuses it.
    fn written_break_level(&self, level: Option<&Expr>) -> Option<u32> {
        match level {
            None => Some(1),
            Some(Expr {
                kind: ExprKind::Int(span),
                ..
            }) => self.file.span_text(*span)?.replace('_', "").parse().ok(),
            Some(_) => None,
        }
    }

    pub(super) fn parse_break_continue(&mut self, start: Span, is_break: bool) -> Stmt {
        self.bump();
        let level = if self.at(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expr())
        };
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        // `rule:php-migration/no-return-leaves-a-finally`: a level reaching
        // past the loops and `switch`es the `finally` opened itself names a
        // target outside the block, and taking it discards what the region was
        // leaving with — the objection the `return` refusal makes. A loop
        // written wholly inside the block keeps both spellings.
        if self.in_finally
            && self
                .written_break_level(level.as_ref())
                .is_some_and(|written| written > self.finally_breakables)
        {
            let word = if is_break { "break" } else { "continue" };
            self.diags.report(
                Diagnostic::error(
                    code::E_BREAK_LEAVES_A_FINALLY,
                    format!("a `{word}` never leaves a `finally` block"),
                )
                .with_primary(span, "this target lies outside the `finally`")
                .with_help(format!(
                    "`{word}` a loop written inside the `finally` itself, or move what this \
                     was skipping to after the whole region"
                )),
            );
        }
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

    pub(super) fn parse_try(&mut self, start: Span) -> Stmt {
        self.bump();
        let body = self.parse_block();
        let mut catches = Vec::new();
        while self.at_keyword(Keyword::Catch) {
            catches.push(self.parse_catch_clause());
        }
        let finally = self
            .eat_keyword(Keyword::Finally)
            .map(|_| self.in_finally_body(Self::parse_block));
        let span = start.to(self.last_span);
        // A `try` with neither clause guards nothing: the block runs, nothing
        // is caught, and nothing runs on the way out. PHP refuses it as a
        // parse error and so does this, at the keyword rather than at the
        // block's end, because that is where the missing clause would be
        // written. The statement is still built with both halves empty, so a
        // file reports the rest of its problems in the same run.
        if catches.is_empty() && finally.is_none() {
            self.diags.report(
                Diagnostic::error(
                    code::E_TRY_WITHOUT_CLAUSE,
                    "a `try` needs a `catch` or a `finally`",
                )
                .with_primary(start, "this block is guarded by nothing")
                .with_help(
                    "add `catch (Throwable $e) { … }` to handle what it throws, or \
                     `finally { … }` to run on the way out either way",
                ),
            );
        }
        Stmt {
            span,
            kind: StmtKind::Try {
                body,
                catches,
                finally,
            },
        }
    }

    /// `catch (Type '$'? identifier?) { ... }`. The ordinary type grammar is
    /// reused, so PHP's `catch (A | B $e)` parses here — and is refused: the
    /// binding carries one static type, so a clause naming two classes has no
    /// type to give it. See [`Self::parse_caught_type`], which `rule:expressions/catch-expression`'s
    /// expression arm shares, and [`code::E_CATCH_UNION_TYPE_UNSUPPORTED`].
    pub(super) fn parse_catch_clause(&mut self) -> CatchClause {
        let start = self.bump().span; // 'catch'
        self.expect(TokenKind::LParen, "`(`");
        let ty = self.parse_caught_type(
            "write one `catch` clause per class, each with its own variable \
             name, or one clause naming a class they all extend",
        );
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

    pub(super) fn parse_echo(&mut self, start: Span) -> Stmt {
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

    /// `<?= expr (';')? ?>` — spec § 1: exactly `<?nvs echo expr; ?>`, one
    /// expression, and unlike every other statement form the `;` is optional
    /// right before the closing tag. `?>` is left for `parse_statement`'s next
    /// call to consume, matching how an ordinary `echo` leaves the following
    /// token for its caller.
    pub(super) fn parse_short_echo_tag(&mut self, start: Span) -> Stmt {
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

    pub(super) fn parse_unset_stmt(&mut self, start: Span) -> Stmt {
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

    pub(super) fn parse_global(&mut self, start: Span) -> Stmt {
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
                     (`rule:statements/no-function-static-and-no-global`)",
                ),
        );
        Stmt {
            span,
            kind: StmtKind::Global(vars),
        }
    }

    pub(super) fn parse_goto(&mut self, start: Span) -> Stmt {
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

    pub(super) fn parse_static_var(&mut self) -> StaticVar {
        let name = self.expect(TokenKind::Variable, "a variable name");
        let default = self.eat(TokenKind::Equals).map(|_| self.parse_expr());
        StaticVar { name, default }
    }

    /// Function-scope `static` — always rejected, whether written in PHP's
    /// ordinary untyped spelling (`static $calls = 0;`) or the typed
    /// spelling `rule:statements/no-function-static-and-no-global`'s own diagnostic wording illustrates
    /// (`static int $calls = 0;`); [`Self::at_function_scope_static`]
    /// already confirmed one of those two shapes follows `static`.
    pub(super) fn parse_static_local(&mut self, start: Span) -> Stmt {
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
                 parameter (`rule:statements/no-function-static-and-no-global`)",
            ),
        );
        Stmt {
            span,
            kind: StmtKind::StaticLocal { ty, vars },
        }
    }

    // ------------------------------------------------------------------------
    // `rule:types/grammar`.1: typed local declaration, vs. an ordinary expression
    // statement that happens to start with the same tokens (a class name
    // used as a type, versus the same name used as a constant fetch or a
    // static-call receiver). The one deciding signal is structural — "the
    // type comes first, in the same position PHP already uses for a
    // parameter" — so a genuine trial parse of the type, checked against
    // whatever token follows it, resolves every case correctly without a
    // second, hand-written classifier that would drift from `parse_type`.
    // ------------------------------------------------------------------------

    pub(super) fn parse_stmt_maybe_local_decl(&mut self, start: Span) -> Stmt {
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
        let value = self
            .eat(TokenKind::Equals)
            .map(|_| self.parse_decl_initializer(name));
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::LocalDecl {
                ty: Some(ty),
                name,
                value,
            },
        }
    }

    // ------------------------------------------------------------------------
    // `rule:types/var-inference`: `var $name = expr;` — the local's type is never written; it
    // is `value`'s own checked type, fixed forever exactly as if that type
    // had been spelled out. Unlike the typed spelling above, the initializer
    // is mandatory here — there is nothing to infer a type from otherwise —
    // so this is a plain, unambiguous parse with no trial/backtrack needed:
    // `var` never starts anything else at statement position.
    // ------------------------------------------------------------------------

    pub(super) fn parse_var_local_decl(&mut self, start: Span) -> Stmt {
        self.bump(); // `var`
        let name = self.expect(TokenKind::Variable, "a variable name");
        self.expect(
            TokenKind::Equals,
            "`=` — `var` infers its type from the initializer, so one is required",
        );
        let value = self.parse_decl_initializer(name);
        self.expect(TokenKind::Semicolon, "`;`");
        let span = start.to(self.last_span);
        Stmt {
            span,
            kind: StmtKind::LocalDecl {
                ty: None,
                name,
                value: Some(value),
            },
        }
    }

    // ------------------------------------------------------------------------
    // `rule:types/grammar`.3: destructuring statement
    // ------------------------------------------------------------------------

    /// `list(...)` is rejected in favour of `[...]` — `rule:expressions/bracket-destructuring`. It is still
    /// parsed in full (it never means anything but a destructuring target,
    /// so unlike `[...]` it collides with no expression grammar and needs no
    /// backtracking) purely so the diagnostic can span the whole construct
    /// and recovery can consume through the `;`, exactly the shape `rule:statements/nvs-is-the-only-open-tag`
    /// gave `die`. The parsed target is then discarded: a rejected construct
    /// never reaches the AST as a live node.
    pub(super) fn parse_destructure_from_list(&mut self, start: Span) -> Stmt {
        let open = self.bump().span; // 'list'
        self.expect(TokenKind::LParen, "`(`");
        let elements = self.parse_destructure_elements(TokenKind::RParen);
        let close = self.expect(TokenKind::RParen, "`)`");
        let target = DestructureTarget {
            elements,
            span: open.to(close),
        };
        let stmt = self.finish_destructure_stmt(start, target);
        self.diags.report(
            Diagnostic::error(
                code::E_LIST_DESTRUCTURING_UNSUPPORTED,
                "`list(...)` is not supported",
            )
            .with_primary(
                stmt.span,
                "use `[...]` instead — it is the only destructuring spelling Novis keeps",
            )
            .with_help("the element grammar is identical inside either bracket (`rule:expressions/bracket-destructuring`)"),
        );
        Stmt {
            span: stmt.span,
            kind: StmtKind::Error,
        }
    }

    /// `[...]` at statement start is ambiguous with a plain array-literal
    /// expression statement (`[1, 2, 3];`, legal if useless) — trial-parse
    /// the more specific destructuring-target grammar and only keep it if a
    /// `=` actually follows, exactly the same backtracking shape as
    /// [`Self::parse_stmt_maybe_local_decl`].
    pub(super) fn parse_stmt_maybe_destructure(&mut self, start: Span) -> Stmt {
        let cp = self.checkpoint();
        let target = self.parse_destructure_target();
        if !self.at(TokenKind::Equals) {
            self.restore(cp);
            return self.parse_expr_statement(start);
        }
        self.finish_destructure_stmt(start, target)
    }

    pub(super) fn finish_destructure_stmt(
        &mut self,
        start: Span,
        target: DestructureTarget,
    ) -> Stmt {
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
    pub(super) fn parse_destructure_target(&mut self) -> DestructureTarget {
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

    pub(super) fn parse_destructure_target_inner(&mut self) -> DestructureTarget {
        let open = self.expect(TokenKind::LBracket, "`[`");
        let elements = self.parse_destructure_elements(TokenKind::RBracket);
        let close = self.expect(TokenKind::RBracket, "`]`");
        DestructureTarget {
            elements,
            span: open.to(close),
        }
    }

    pub(super) fn parse_destructure_elements(
        &mut self,
        closer: TokenKind,
    ) -> Vec<DestructureElement> {
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
    pub(super) fn parse_destructure_key(&mut self) -> Option<Expr> {
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

    pub(super) fn parse_destructure_element(&mut self) -> DestructureElement {
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
        let inout = self.eat_keyword(Keyword::Inout).is_some();
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
                    "every destructuring leaf declares a type (`rule:types/grammar`.3)",
                ),
            );
            None
        };
        if let Some(amp) = self.eat(TokenKind::Amp) {
            self.report_by_reference_marker(amp, "write `inout` before the leaf's type");
        }
        let name = self.expect(TokenKind::Variable, "a destructuring leaf's name");
        let span = start.to(self.last_span);
        DestructureElement::Leaf {
            key,
            ty,
            inout,
            name,
            span,
        }
    }
}
