//! The expression grammar: every operator at its PHP precedence, the postfix
//! chain, and every primary.
//!
//! The precedence chain runs lowest to highest as a function per level, each
//! calling the next — `parse_expr` down to [`Parser::parse_primary`] — so a
//! level's binding power is where it sits in that call graph and nowhere else.
//! `as` binds tighter than any binary operator (`rule:types/conversion`), and
//! `and`/`or`/`xor` are caught at the bottom of the chain rather than parsed,
//! since `&&`/`||` are the only logical connectives Novis keeps (`rule:expressions/no-keyword-logical-operators`).
//!
//! Beyond the operators: `match`, closures and arrow functions (`rule:types/anonymous-function`'s one
//! literal, `fn`, plus the `function` forms it refuses), generators
//! (`yield`/`yield from`), named arguments, spread, nullsafe, first-class
//! callable syntax, `require` (an expression, not a statement — `rule:statements/require-is-the-only-inclusion-construct`),
//! `spawn script … with(…)`, `rule:types/anonymous-object`'s `{a: 1}` object literal, and the
//! interpolated-string bodies the lexer hands back in parts.
//!
//! Several PHP spellings are parsed here only to be diagnosed, and they are
//! deliberately *parsed* rather than left to fail: a legacy `(int)$x` cast (ADR
//! 0034), `eval`/`extract`/`settype` (`rule:security/closed-doors`), `die`, and
//! `include`/`include_once`/`require_once` (`rule:statements/require-is-the-only-inclusion-construct`). Recovering the whole
//! construct is what lets the diagnostic name the replacement and the rest of
//! the file keep parsing.
//!
//! Part of [`super`]'s one `impl Parser`, split across this directory so a
//! session editing one layer of the grammar does not carry the rest in
//! context. Every item moved here unchanged; the methods are `pub(super)` so
//! they reach across these modules and no further, which is the reach they had
//! when `parser` was a single file.

use super::*;

impl<'src, 'd> Parser<'src, 'd> {
    /// Parses one expression: assignment, the ternary, and every binary and
    /// unary operator, down to a primary expression. Every other production
    /// that needs "an expression" calls this. A stray `and`/`or`/`xor`
    /// keyword is also caught here — see [`Self::parse_low_or`] — since
    /// `&&`/`||` are the only logical connectives Novis keeps
    /// (`rule:expressions/no-keyword-logical-operators`).
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
    /// ([`docs/spec/00-overview.md` § 3.2](/docs/spec/00-overview.md)).
    /// Converting the subject still works, just parenthesized:
    /// `foreach (($m as array<int>) as int $v)` — the parens start a fresh
    /// [`Self::parse_expr`] call, which lifts the suppression for its own
    /// duration.
    pub(super) fn parse_expr_no_top_as(&mut self) -> Expr {
        let prev = self.suppress_as;
        self.suppress_as = true;
        let e = self.parse_low_or();
        self.suppress_as = prev;
        e
    }

    /// The general form of the shape [`Self::parse_postfix`] documents: a
    /// left-associative tier consumes its chain in a loop, so a long one costs
    /// this function's own stack nothing, but each operator left-nests the tree
    /// one level deeper — and that depth is what the first ordinary recursive
    /// walk over the result has to survive. So each operator is charged one
    /// level of [`Self::enter_recursive`]'s budget, **held open until the chain
    /// ends**, which is what makes the counter track the tree being built
    /// rather than the descent that built it. Every binary tier funnels through
    /// here, so `$n + 1 + 1 …` and `$a . $b . $c …` draw on the same bounded
    /// budget as the nesting they are flat alternatives to, instead of a free
    /// one each. `links` is the count to hand back, since nothing else will.
    pub(super) fn parse_left_assoc(
        &mut self,
        next: fn(&mut Self) -> Expr,
        ops: &[(TokenKind, BinaryOp)],
    ) -> Expr {
        let mut lhs = next(self);
        let mut links: u32 = 0;
        loop {
            let kind = self.peek().kind;
            let Some(&(_, op)) = ops.iter().find(|(tk, _)| *tk == kind) else {
                break;
            };
            if self.enter_recursive() {
                lhs = Expr {
                    span: lhs.span,
                    kind: ExprKind::Error(lhs.span),
                };
                break;
            }
            links += 1;
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
        for _ in 0..links {
            self.exit_recursive();
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
    /// The guarded recursion entry above [`Self::parse_assignment`] — see
    /// [`Self::guarded`]. Also where a stray `and`/`or`/`xor` keyword is
    /// caught: Novis never gave them PHP's lower-precedence meaning distinct
    /// from `&&`/`||`, so each occurrence is diagnosed in place
    /// (`rule:expressions/no-keyword-logical-operators`)
    /// and folded into an `ExprKind::Error`, consuming its right-hand operand
    /// so parsing can continue past it rather than cascading into unrelated
    /// "expected token" errors.
    pub(super) fn parse_low_or(&mut self) -> Expr {
        self.guarded(Self::error_expr_here, Self::parse_rejected_logical_keyword)
    }

    pub(super) fn parse_rejected_logical_keyword(&mut self) -> Expr {
        let mut lhs = self.parse_assignment();
        loop {
            let (spelling, replacement) = match self.peek().kind {
                TokenKind::Keyword(Keyword::Or) => ("or", Some("||")),
                TokenKind::Keyword(Keyword::And) => ("and", Some("&&")),
                TokenKind::Keyword(Keyword::Xor) => ("xor", None),
                _ => break,
            };
            let op_span = self.bump().span;
            let rhs = self.parse_assignment();
            let span = lhs.span.to(rhs.span);
            let help = match replacement {
                Some(r) => {
                    format!("use `{r}` instead — it is the only logical connective Novis keeps")
                }
                None => {
                    "there is no direct replacement — write `(a || b) && !(a && b)`, or `a != b` \
                          when both operands are already `bool`"
                        .to_string()
                }
            };
            self.diags.report(
                Diagnostic::error(
                    code::E_LOGICAL_KEYWORD_UNSUPPORTED,
                    format!("`{spelling}` is not supported"),
                )
                .with_primary(
                    op_span,
                    "Novis keeps exactly one spelling for each logical connective",
                )
                .with_help(help),
            );
            lhs = Expr {
                span,
                kind: ExprKind::Error(span),
            };
        }
        lhs
    }

    /// Reports [`code::E_INVALID_ASSIGN_TARGET`] unless `target` is a place —
    /// the one syntactic gate every write spelling passes through.
    ///
    /// An **increment** needs it for the same reason an assignment does, and
    /// used not to have it: `nvs_ir::lower` desugars `$x++` into the
    /// `$x = $x + 1` a compound assignment becomes, so a target it cannot
    /// write to is a target it cannot read-modify-write either, and
    /// `$h->rows()++` reached that rewrite's own assertion instead of a
    /// diagnostic. PHP refuses the identical shapes — *"Can't use method
    /// return value in write context"* — so this is where the four increment
    /// arms in [`Self::parse_unary`] and [`Self::parse_postfix`] join the two
    /// assignment ones.
    ///
    /// Parentheses are deliberately **not** peeled here: PHP refuses `($a) = 5`
    /// and `($a)++` outright, at parse time, and the only spelling that does
    /// write through them is a whole subscript chain (`($a)[0] = 2`, whose
    /// target is the `Index` this already admits) — which
    /// `nvs_types::expr::assign::check_write_target` then resolves with
    /// [`Expr::unparenthesized`], at the level where it means something.
    fn require_write_target(&mut self, target: &Expr) {
        if !is_assignable(target) {
            self.diags.report(
                Diagnostic::error(
                    code::E_INVALID_ASSIGN_TARGET,
                    "this expression cannot be assigned to",
                )
                .with_primary(target.span, "not a valid assignment target"),
            );
        }
    }

    /// Right-recursive on its own operand (`$a = $b = $c = ...`), so a long
    /// chain of `=` needs the same recursion guard as the array-literal
    /// nesting that originally motivated it — see [`Self::guarded`].
    pub(super) fn parse_assignment(&mut self) -> Expr {
        self.guarded(Self::error_expr_here, Self::parse_assignment_inner)
    }

    pub(super) fn parse_assignment_inner(&mut self) -> Expr {
        self.parse_assignment_over(Self::parse_catch)
    }

    /// The assignment level with `rule:expressions/catch-expression`'s `catch` cut out of its head — what
    /// a ternary's *else* branch parses at.
    ///
    /// That branch trails, so parsing it at the ordinary level would let it
    /// swallow a `catch` belonging to whatever guards the ternary. § 2's third
    /// row reads `f() catch (A) => $y ?: 1 catch (B) => 2` as two arms of one
    /// guard, and by the same rule `$c ? $a : $b catch (E) => 0` guards the
    /// whole ternary rather than only `$b`. The *then* branch is delimited by
    /// its own `:` and cannot trail, so a `catch` written there is ordinary
    /// and parses at the full level.
    fn parse_ternary_else(&mut self) -> Expr {
        self.guarded(Self::error_expr_here, |p| {
            p.parse_assignment_over(Self::parse_ternary)
        })
    }

    /// The body [`Self::parse_assignment_inner`] and [`Self::parse_ternary_else`]
    /// share; they differ only in the level the left-hand side is parsed at.
    fn parse_assignment_over(&mut self, head: fn(&mut Self) -> Expr) -> Expr {
        let target = head(self);
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
            TokenKind::QuestionQuestionPlusEquals => AssignOp::CoalesceAddAssign,
            TokenKind::QuestionQuestionMinusEquals => AssignOp::CoalesceSubAssign,
            TokenKind::QuestionQuestionDotEquals => AssignOp::CoalesceConcatAssign,
            _ => return target,
        };
        self.bump();
        // `$ok && $row = $next` is reported once its value is parsed, because
        // the parentheses its fix writes close after the value
        // (`super::grouping`).
        let misread = Self::misread_assignment_target(&target);
        if !misread {
            self.require_write_target(&target);
        }
        // `target = &value` binds by reference; PHP has no `&`-form of a
        // compound operator (`+=&` is not a thing), so this only applies to
        // plain `=`.
        let by_ref = op == AssignOp::Assign && self.eat(TokenKind::Amp).is_some();
        let value = self.parse_assignment();
        if misread {
            return self.regroup_misread_assignment(target, op, value, by_ref);
        }
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

    /// `rule:expressions/catch-expression-precedence`: `catch` sits between assignment and the ternary.
    ///
    /// The guarded expression is everything the ternary level parses, so one
    /// arm covers a whole `??` chain or a whole `?:`; the arm body is parsed
    /// at that same level, so a following `catch` starts the next arm of this
    /// guard rather than nesting under the previous arm's fallback. Both
    /// readings this rules out are ruled out by construction rather than by a
    /// check — `try` never appears in the expression form, and a `catch` that
    /// belongs to the block form follows a `}` the statement parser is already
    /// inside.
    pub(super) fn parse_catch(&mut self) -> Expr {
        let guarded = self.parse_ternary();
        if !self.at_keyword(Keyword::Catch) {
            return guarded;
        }
        let mut arms = Vec::new();
        while self.at_keyword(Keyword::Catch) {
            arms.push(self.parse_catch_arm());
        }
        let end = arms.last().expect("the loop ran at least once").span;
        Expr {
            span: guarded.span.to(end),
            kind: ExprKind::Catch {
                guarded: Box::new(guarded),
                arms,
            },
        }
    }

    /// One `catch (T $e) => expr` arm. Its `( Type $var? )` half is the block
    /// form's, through [`Self::parse_caught_type`]; only the body differs.
    fn parse_catch_arm(&mut self) -> CatchArm {
        let start = self.bump().span; // `catch`
        self.expect(TokenKind::LParen, "`(`");
        let ty = self.parse_caught_type(
            "write one `catch` arm per class, each with its own variable name, \
             or one arm naming a class they all extend",
        );
        let var = self.eat(TokenKind::Variable);
        self.expect(TokenKind::RParen, "`)`");
        self.expect(TokenKind::FatArrow, "`=>`");
        let body = self.parse_catch_arm_body();
        CatchArm {
            span: start.to(body.span),
            ty,
            var,
            body,
        }
    }

    /// `rule:expressions/catch-arm-is-an-expression`: the arm body is parsed as an expression and that is the
    /// entire rule — `throw` is admitted for free, since `throw expr` is
    /// already an expression, and `return`, `break` and `continue` are out
    /// because they are statements.
    ///
    /// The three are named rather than left to the generic expected-expression
    /// error: a reader who writes `return` here wants to be told the block
    /// form exists, not which token was expected. The keyword is consumed and
    /// whatever follows it is parsed as the arm's body, so a file reports the
    /// rest of its problems in the same run.
    fn parse_catch_arm_body(&mut self) -> Expr {
        let spelling = match self.peek().kind {
            TokenKind::Keyword(Keyword::Return) => "return",
            TokenKind::Keyword(Keyword::Break) => "break",
            TokenKind::Keyword(Keyword::Continue) => "continue",
            _ => return self.parse_ternary(),
        };
        let span = self.peek().span;
        self.diags.report(
            Diagnostic::error(
                code::E_CATCH_ARM_NOT_AN_EXPRESSION,
                format!("a `catch` arm is an expression, and `{spelling}` is a statement"),
            )
            .with_primary(span, format!("`{spelling}` cannot appear here"))
            .with_help(
                "`throw` is an expression, so `catch (E $e) => throw new …` is \
                 allowed here; for an early return write the block form, \
                 `try { … } catch (E $e) { return …; }`",
            ),
        );
        self.bump();
        // What may legitimately follow an arm: the statement's `;`, a
        // separator in whatever list this expression sits in, a closer, or the
        // next arm. Anything else is parsed as the body, so `return $x` still
        // yields `$x` and a bare `return` still yields one error rather than
        // two.
        if matches!(
            self.peek().kind,
            TokenKind::Semicolon
                | TokenKind::Comma
                | TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::RBrace
                | TokenKind::Eof
                | TokenKind::Keyword(Keyword::Catch)
        ) {
            self.error_expr_here()
        } else {
            self.parse_ternary()
        }
    }

    pub(super) fn parse_ternary(&mut self) -> Expr {
        let cond = self.parse_coalesce();
        if self.eat(TokenKind::Question).is_none() {
            return cond;
        }
        if self.eat(TokenKind::Colon).is_some() {
            let else_ = self.parse_ternary_else();
            self.warn_misread_ternary(&cond, else_.span, true);
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
        let else_ = self.parse_ternary_else();
        self.warn_misread_ternary(&cond, else_.span, false);
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
    pub(super) fn parse_coalesce(&mut self) -> Expr {
        self.guarded(Self::error_expr_here, Self::parse_coalesce_inner)
    }

    pub(super) fn parse_coalesce_inner(&mut self) -> Expr {
        let lhs = self.parse_logic_or();
        if self.eat(TokenKind::QuestionQuestion).is_some() {
            let rhs = self.parse_coalesce();
            self.warn_misread_coalesce(&lhs, &rhs);
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

    pub(super) fn parse_logic_or(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_logic_and,
            &[(TokenKind::PipePipe, BinaryOp::Or)],
        )
    }

    pub(super) fn parse_logic_and(&mut self) -> Expr {
        self.parse_left_assoc(Self::parse_bit_or, &[(TokenKind::AmpAmp, BinaryOp::And)])
    }

    pub(super) fn parse_bit_or(&mut self) -> Expr {
        self.parse_left_assoc(Self::parse_bit_xor, &[(TokenKind::Pipe, BinaryOp::BitOr)])
    }

    pub(super) fn parse_bit_xor(&mut self) -> Expr {
        self.parse_left_assoc(Self::parse_bit_and, &[(TokenKind::Caret, BinaryOp::BitXor)])
    }

    pub(super) fn parse_bit_and(&mut self) -> Expr {
        self.parse_left_assoc(Self::parse_equality, &[(TokenKind::Amp, BinaryOp::BitAnd)])
    }

    pub(super) fn parse_equality(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_relational,
            &[
                (TokenKind::EqualsEquals, BinaryOp::Eq),
                (TokenKind::BangEquals, BinaryOp::NotEq),
                (TokenKind::Spaceship, BinaryOp::Cmp),
            ],
        )
    }

    pub(super) fn parse_relational(&mut self) -> Expr {
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

    pub(super) fn parse_concat(&mut self) -> Expr {
        self.parse_left_assoc(Self::parse_shift, &[(TokenKind::Dot, BinaryOp::Concat)])
    }

    pub(super) fn parse_shift(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_additive,
            &[
                (TokenKind::LtLt, BinaryOp::Shl),
                (TokenKind::GtGt, BinaryOp::Shr),
            ],
        )
    }

    pub(super) fn parse_additive(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_multiplicative,
            &[
                (TokenKind::Plus, BinaryOp::Add),
                (TokenKind::Minus, BinaryOp::Sub),
            ],
        )
    }

    pub(super) fn parse_multiplicative(&mut self) -> Expr {
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
    /// `is`, which binds tighter. The stacking is unbounded self-
    /// recursion (`!!!!!!...`), so it needs the same guard as
    /// [`Self::parse_assignment`].
    pub(super) fn parse_not(&mut self) -> Expr {
        self.guarded(Self::error_expr_here, Self::parse_not_inner)
    }

    pub(super) fn parse_not_inner(&mut self) -> Expr {
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
        self.parse_type_test()
    }

    /// `is` — the one type test (`rule:types/type-test`), and the level where
    /// `instanceof` is refused, because that is where a reader of PHP writes
    /// it (`rule:php-migration/one-type-test`).
    ///
    /// **One token after the keyword decides which arm the right side is**, and
    /// [`Self::parse_test_operand`] is where that is written down: a
    /// `$variable` opens the value arm, every other token a type.
    ///
    /// Left-associative in a loop, so each test is charged one level of the
    /// recursion budget and held open to the end of the chain, for the
    /// reason `parse_left_assoc` gives: the tree nests even though the
    /// parser does not.
    pub(super) fn parse_type_test(&mut self) -> Expr {
        let mut lhs = self.parse_pipe();
        let mut links: u32 = 0;
        while self.at_keyword(Keyword::Is) || self.at_keyword(Keyword::InstanceOf) {
            if self.enter_recursive() {
                lhs = Expr {
                    span: lhs.span,
                    kind: ExprKind::Error(lhs.span),
                };
                break;
            }
            links += 1;
            let refused = self.at_keyword(Keyword::InstanceOf);
            let keyword = self.bump().span;
            if refused {
                self.reject_instanceof(keyword);
                // The right operand is consumed at the level the refused
                // spelling took one, so the site costs this one diagnostic
                // rather than a second about a token nothing expected.
                let right = self.parse_pipe();
                let span = lhs.span.to(right.span);
                lhs = Expr {
                    span,
                    kind: ExprKind::Error(span),
                };
                continue;
            }
            let against = self.parse_test_operand();
            let span = lhs.span.to(against.span());
            lhs = Expr {
                span,
                kind: ExprKind::TypeTest {
                    expr: Box::new(lhs),
                    against,
                },
            };
        }
        for _ in 0..links {
            self.exit_recursive();
        }
        lhs
    }

    /// `rule:php-migration/one-type-test`: `instanceof` names nothing in Novis
    /// but its own refusal.
    ///
    /// The keyword stays a keyword so the spelling can be named here — read as
    /// an ordinary identifier it would arrive at name resolution as a constant
    /// that resolves to nothing, and the reader would be told about a name
    /// instead of about the operator they wrote. The help carries both
    /// rewrites, because the class-value form has one too.
    fn reject_instanceof(&mut self, span: Span) {
        self.diags.report(
            Diagnostic::error(
                code::E_INSTANCEOF_IS_NOT_AN_OPERATOR,
                "`instanceof` is not a Novis operator; the type test is `is`",
            )
            .with_primary(span, "no operator is spelled this way")
            .with_help(
                "write `$x is Request`; a class reference on the right is `$x is $cls` \
                 (`rule:types/type-test`)",
            ),
        );
    }

    /// `|>` — the pipeline operator, one level between `is` and unary
    /// because `rule:expressions/pipeline-precedence` binds it tighter than
    /// every binary operator and looser than unary: `-$a |> Math::abs($_)` is
    /// `Math::abs(-$a)` and `"x=" . $a |> Str::upper($_)` is
    /// `"x=" . Str::upper($a)`. Its right side is parsed at the postfix level,
    /// so an operand that is not already a call or an access is written
    /// parenthesized.
    ///
    /// **The substitution happens here and leaves nothing behind.** The left
    /// side is parked in `pipe_hole` while the right side parses, and the `$_`
    /// [`Self::parse_hole`] meets takes it — so what this returns is the tree
    /// the nested spelling produces, and no later pass can tell the two apart
    /// (`rule:expressions/pipeline-substitution`). There is no `ExprKind` for
    /// the operator and none for the hole.
    ///
    /// What is left in the slot afterwards is the whole arity check
    /// (`rule:expressions/pipeline-hole-once`): still occupied means the right
    /// side had no hole, and a second `$_` finds it already empty.
    ///
    /// **Every stage is charged one level of [`Self::enter_recursive`]'s
    /// budget, held open across the whole chain**, the way
    /// [`Self::parse_postfix`] charges one per link. A chain is a loop, so it
    /// costs this function's own stack nothing — but each stage wraps the
    /// previous tree in one more node, so `n` stages build a tree `n` deep,
    /// and that is what the first ordinary recursive walk over it afterwards
    /// has to survive. Charging per iteration and giving it back immediately
    /// would leave the counter flat while the tree grew, so the operator
    /// could build a depth the substitution it stands for
    /// (`rule:expressions/pipeline-substitution`) is refused at. `stages`
    /// tracks the increments to hand back, exactly as `chain_len` does there.
    pub(super) fn parse_pipe(&mut self) -> Expr {
        let mut lhs = self.parse_unary();
        let mut stages: u32 = 0;
        while self.at(TokenKind::PipeGreater) {
            if self.enter_recursive() {
                lhs = Expr {
                    span: lhs.span,
                    kind: ExprKind::Error(lhs.span),
                };
                break;
            }
            stages += 1;
            self.bump();
            let outer = self.pipe_hole.replace(lhs);
            self.pipe_rhs_depth += 1;
            let rhs = self.parse_postfix();
            self.pipe_rhs_depth -= 1;
            if std::mem::replace(&mut self.pipe_hole, outer).is_some() {
                self.report_right_side_without_a_hole(&rhs);
            }
            lhs = rhs;
        }
        for _ in 0..stages {
            self.exit_recursive();
        }
        lhs
    }

    /// The right side of a `|>` never named `$_`, so there was nowhere to put
    /// the left side. Recovery keeps the right side as the expression, which
    /// is the tree the author would have got had they written the hole in the
    /// argument they meant.
    fn report_right_side_without_a_hole(&mut self, rhs: &Expr) {
        let applies_a_callable = matches!(
            &rhs.kind,
            ExprKind::Fn(_)
                | ExprKind::Call {
                    args: CallArgs::FirstClassCallable,
                    ..
                }
                | ExprKind::MethodCall {
                    args: CallArgs::FirstClassCallable,
                    ..
                }
                | ExprKind::StaticCall {
                    args: CallArgs::FirstClassCallable,
                    ..
                }
        );
        let mut diag = Diagnostic::error(
            code::E_PIPELINE_RIGHT_SIDE_HAS_NO_HOLE,
            "the right side of `|>` needs the hole `$_`",
        )
        .with_primary(
            rhs.span,
            "no `$_` here, so the piped value has nowhere to go",
        )
        .with_help("write the hole where the value goes — `Str::trim($_)`");
        if applies_a_callable {
            diag = diag.with_note(
                "PHP 8.5's `|>` applies a callable; Novis's substitutes `$_` at compile time",
            );
        }
        self.diags.report(diag);
    }

    /// `$_`, which is a hole and never a variable — so it is turned into the
    /// left side of its `|>` here, before [`crate::casing`] ever sees an
    /// identifier made of underscores (`rule:expressions/pipeline-hole-once`).
    /// The two ways it can fail to have a left side to become are the other
    /// two refusals that rule names.
    fn parse_hole(&mut self, span: Span) -> Expr {
        self.bump();
        if self.pipe_rhs_depth == 0 {
            self.diags.report(
                Diagnostic::error(
                    code::E_HOLE_OUTSIDE_A_PIPELINE,
                    "`$_` is the pipeline hole and has no meaning here",
                )
                .with_primary(span, "outside the right side of a `|>`")
                .with_help("a hole is only written on the right side of `|>`"),
            );
            return Expr {
                span,
                kind: ExprKind::Error(span),
            };
        }
        match self.pipe_hole.take() {
            Some(lhs) => lhs,
            None => {
                self.diags.report(
                    Diagnostic::error(
                        code::E_PIPELINE_RIGHT_SIDE_REPEATS_THE_HOLE,
                        "`$_` may appear exactly once on the right side of a `|>`",
                    )
                    .with_primary(span, "the hole was already taken by an earlier `$_`")
                    .with_help("bind the value to a local instead"),
                );
                Expr {
                    span,
                    kind: ExprKind::Error(span),
                }
            }
        }
    }

    /// Whether `(` at the current position opens a legacy `(T)expr` cast
    /// rather than a parenthesized expression — decided by a 3-token
    /// lookahead, since both start identically. The spelling itself, not just
    /// its shape, is returned so the diagnostic can name it.
    pub(super) fn peek_cast_keyword(&mut self) -> Option<&'static str> {
        let spelling = match self.peek_at(1).kind {
            TokenKind::Keyword(Keyword::Int) => "int",
            TokenKind::Keyword(Keyword::Uint) => "uint",
            TokenKind::Keyword(Keyword::Float) => "float",
            TokenKind::Keyword(Keyword::String) => "string",
            TokenKind::Keyword(Keyword::Bool) => "bool",
            TokenKind::Keyword(Keyword::Array) => "array",
            TokenKind::Keyword(Keyword::Object) => "object",
            _ => return None,
        };
        (self.peek_at(2).kind == TokenKind::RParen).then_some(spelling)
    }

    /// Self-recursive on its own operand for every prefix form it handles
    /// (a cast, `-`/`+`/`~`/`@`/`!`, `++`/`--`), and mutually recursive with
    /// [`Self::parse_power`] for a chain of `**` — needs the same guard as
    /// [`Self::parse_assignment`], and guarding it alone is enough to bound
    /// that `**` cycle too, since every trip around it passes back through
    /// here.
    pub(super) fn parse_unary(&mut self) -> Expr {
        self.guarded(Self::error_expr_here, Self::parse_unary_inner)
    }

    pub(super) fn parse_unary_inner(&mut self) -> Expr {
        if self.at(TokenKind::LParen)
            && let Some(spelling) = self.peek_cast_keyword()
        {
            let start = self.bump().span; // '('
            let ty = self.parse_type(); // the cast keyword, alone before the ')'
            self.bump(); // ')'
            let operand = self.parse_unary();
            let span = start.to(operand.span);
            let mut reported = Diagnostic::error(
                code::E_LEGACY_CAST_UNSUPPORTED,
                format!("`({spelling})expr` is not supported"),
            )
            .with_primary(span, "Novis keeps exactly one conversion spelling")
            .with_help(format!(
                "use `expr as {spelling}` — it throws instead of silently truncating"
            ));
            // The rewrite the help line describes, computed once here rather
            // than by whoever reads it: an editor offers it as a code action
            // (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`)
            // and `nvs convert` writes it with a `TODO` beside it. Unsafe
            // because it is exactly the behaviour change the help line names —
            // the cast truncated where `as` throws — so a converter must not
            // apply it silently. An operand that did not parse gets no fix: its
            // text is whatever the recovery consumed rather than an expression.
            if !matches!(operand.kind, ExprKind::Error(_))
                && let Some(text) = self.file.span_text(operand.span)
                && !text.is_empty()
            {
                reported = reported.with_unsafe_fix(
                    span,
                    format!("{text} as {spelling}"),
                    format!("rewrite as `{text} as {spelling}`"),
                );
            }
            self.diags.report(reported);
            // The cast comes back as the `as` its help line names, marked
            // `legacy`, so the expression around it is typed as written and this
            // one mistake gives this one diagnostic.
            return Expr {
                span,
                kind: ExprKind::Conversion {
                    expr: Box::new(operand),
                    ty,
                    legacy: true,
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
            // `@expr` is refused where it is written, exactly as the legacy
            // cast above is, so `UnaryOp::Suppress` is a variant the parser
            // never produces. `rule:errors/escalation-ladder`'s ladder makes a runtime failure a
            // `Throwable`, not a diagnostic printed next to a value, so there
            // is nothing an operand-shaped marker could suppress; `rule:core-api/removals`
            // already lists `@` among what that decision closes. The operand is
            // parsed *and handed back in place of the whole thing*, so `@$n * 2`
            // is typed as `$n * 2` and reports once rather than reporting again
            // at every binding a `mixed` no longer satisfies. That is where this
            // parts from the legacy cast above, which names a target type it
            // cannot honestly produce a value of and so yields
            // [`ExprKind::Error`]; `@` says nothing about its operand's type.
            TokenKind::At => {
                let start = self.bump().span;
                let operand = self.parse_unary();
                let span = start.to(operand.span);
                self.diags.report(
                    Diagnostic::error(
                        code::E_SUPPRESSION_UNSUPPORTED,
                        "`@expr` error suppression does not exist",
                    )
                    .with_primary(span, "there is nothing here to suppress")
                    .with_help(
                        "a failure is a `Throwable` propagated by checked return, not a \
                         diagnostic printed beside a value — catch it with `try`/`catch`",
                    ),
                );
                operand
            }
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
                self.require_write_target(&expr);
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
                self.require_write_target(&expr);
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
    pub(super) fn parse_power(&mut self) -> Expr {
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
    pub(super) fn parse_postfix(&mut self) -> Expr {
        let mut e = self.parse_primary();
        let mut chain_len: u32 = 0;
        loop {
            if self.enter_recursive() {
                e = Expr {
                    span: e.span,
                    kind: ExprKind::Error(e.span),
                };
                break;
            }
            chain_len += 1;
            match self.peek().kind {
                TokenKind::Arrow | TokenKind::NullsafeArrow => {
                    let nullsafe = matches!(self.peek().kind, TokenKind::NullsafeArrow);
                    self.bump();
                    let member = self.parse_member_name();
                    let type_args = self.parse_call_type_args();
                    e = if self.at(TokenKind::LParen) {
                        let args = self.parse_call_args();
                        let span = e.span.to(self.last_span);
                        Expr {
                            span,
                            kind: ExprKind::MethodCall {
                                object: Box::new(e),
                                nullsafe,
                                method: member,
                                type_args,
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
                    self.require_write_target(&e);
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
                    self.require_write_target(&e);
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
                            legacy: false,
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

    pub(super) fn parse_after_double_colon(&mut self, class: Expr) -> Expr {
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
                let type_args = self.parse_call_type_args();
                if self.at(TokenKind::LParen) {
                    let args = self.parse_call_args();
                    let span = class.span.to(self.last_span);
                    Expr {
                        span,
                        kind: ExprKind::StaticCall {
                            class: Box::new(class),
                            method: member,
                            type_args,
                            args,
                        },
                    }
                // A name that was never written is a class-constant access
                // whose name is missing, not a dynamic member: `Foo::` at the
                // caret is the `::` shape of `$u->`, and sending it to the
                // arm below would report an invented "expected `(`" on top of
                // the diagnostic `parse_member_name` already reported. What
                // `rule:ide/recovery-is-explicit` asks for is not answered on
                // this path, though, because `ClassConstAccess` carries a bare
                // `Span` and so has nowhere to say the name was invented.
                } else if let MemberName::Ident(name) | MemberName::Missing(name) = member {
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
                        kind: ExprKind::Error(span),
                    }
                }
            }
        }
    }

    /// An optional `<T, U>` written between a member name and the `(` of a
    /// call — `<User>` in `Core\Json::decodeAs<User>($body)`, which
    /// `rule:core-classes/derive-attribute` and
    /// [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
    /// § 6 both write. Returns what was written, empty when this is not a
    /// type-argument list at all.
    ///
    /// Unlike [`Self::parse_type_args`], which is only ever reached from a
    /// type position, this one is genuinely ambiguous: `Foo::BAR < X > ($y)`
    /// is also two comparisons. It is resolved by a **checkpointed trial
    /// parse** — the `<` opens a type-argument list only when everything up
    /// to a matching `>` parses as a type list with no diagnostic *and* the
    /// very next token is `(`. Anything else rewinds to the `<` and leaves it
    /// to the expression grammar, so `$a::$b < $c > ($d)` is still two
    /// comparisons (a `$name` does not parse as a type) and so is every
    /// `<`-then-no-call shape.
    ///
    /// The residue is that a comparison of a class constant against a
    /// *class-shaped name*, immediately followed by a parenthesized operand,
    /// now reads as a call with type arguments. That is the same trade C# and
    /// Rust's turbofish-free method position make, it needs a `SCREAMING_CASE`
    /// constant compared to an `UpperCamel` name to hit, and parentheses
    /// (`(Foo::BAR < X) > ($y)`) say the other thing.
    pub(super) fn parse_call_type_args(&mut self) -> Vec<Type> {
        if !self.at(TokenKind::Lt) {
            return Vec::new();
        }
        let cp = self.checkpoint();
        let before = self.diags.len();
        let (args, _) = self.parse_type_args(self.last_span);
        if self.diags.len() == before && self.at(TokenKind::LParen) {
            return args;
        }
        self.restore(cp);
        Vec::new()
    }

    /// The name on the right of `->`/`?->`/`::`: an ordinary identifier (any
    /// keyword spelling accepted too, matching PHP's own allowance of
    /// keyword-named methods), `$name` (a dynamic member name) or `{expr}`
    /// (a fully computed one).
    ///
    /// The last two parse and carry no refusal of their own, which is
    /// [`code::E_VARIABLE_VARIABLE`]'s rule stopping one construct short of
    /// them: `rule:types/property-key-access` admits `$obj->$key` when the operand's *type* is a
    /// `property<T>` the receiver satisfies, and a type is the one thing this
    /// parser cannot see. So the spelling is no longer what is rejected — the
    /// missing check is — and `E0235` is reported by `nvs_types`, at the same
    /// span, for every operand that is not a key. Both spellings reach it the
    /// same way: `->{$k}` is `->$k` written the other way round, and § 4's
    /// three neighbours that keep the refusal are distinguished by what they
    /// are (a call, an `unset`, an unsatisfiable receiver), never by which of
    /// the two brackets wrote them.
    ///
    /// The fourth outcome is the one nobody wrote: where none of the three is
    /// present, the name is [`MemberName::Missing`] at the insertion point,
    /// diagnosed there and left explicit rather than synthesized as an
    /// identifier spanning nothing (`rule:ide/recovery-is-explicit`). `$u->`
    /// with the caret after the arrow is that shape, and it is the one member
    /// completion is asked about.
    pub(super) fn parse_member_name(&mut self) -> MemberName {
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
            _ => MemberName::Missing(self.error_expected("a member name")),
        }
    }

    pub(super) fn parse_call_args(&mut self) -> CallArgs {
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

    /// `'...' expr | name ':' expr | 'inout'? expr` — `rule:statements/inout-is-written-at-the-call` writes the
    /// marker again at the call site, and it goes outside a named argument's
    /// `name:` for the same reason it goes outside the parameter's type: it
    /// marks the binding, not the value. Whether it is *required* here needs
    /// the callee's signature and so belongs to `nvs_types` (E0713/E0714).
    ///
    /// **A `name` here is any word, keyword or not.** `rule:core-api/shape-rules`
    /// R2 makes every parameter callable by the `$name` the spec writes, and
    /// seven of those names — `Core\Arr::map`'s `$fn` and its siblings — are
    /// spellings the lexer reserves. The `:` is the whole disambiguation: no
    /// expression in argument position starts with a keyword followed by one,
    /// so admitting a [`Keyword`](TokenKind::Keyword) token before it costs the
    /// grammar nothing and takes the same one-token contextual reading `spawn`
    /// and `type` already take ([`crate::token`]). Renaming the parameter is
    /// the alternative, and R2 makes the spec authoritative for the name.
    pub(super) fn parse_arg(&mut self) -> Arg {
        let start = self.peek().span;
        if self.eat(TokenKind::Ellipsis).is_some() {
            let value = self.parse_expr();
            let span = start.to(value.span);
            return Arg {
                name: None,
                spread: true,
                inout: false,
                value,
                span,
            };
        }
        let inout = self.eat_keyword(Keyword::Inout).is_some();
        if matches!(self.peek().kind, TokenKind::Ident | TokenKind::Keyword(_))
            && self.peek_at(1).kind == TokenKind::Colon
        {
            let name = self.bump().span;
            self.bump(); // ':'
            let value = self.parse_expr();
            let span = start.to(value.span);
            return Arg {
                name: Some(name),
                spread: false,
                inout,
                value,
                span,
            };
        }
        let value = self.parse_expr();
        let span = start.to(value.span);
        Arg {
            name: None,
            spread: false,
            inout,
            value,
            span,
        }
    }

    // ========================================================================
    // Primary expressions
    // ========================================================================

    pub(super) fn parse_primary(&mut self) -> Expr {
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
            // The lexer reported this token when it made it, so it stands for
            // one error expression and adds no diagnostic of its own.
            TokenKind::Unknown => {
                self.bump();
                Expr {
                    span: start,
                    kind: ExprKind::Error(start),
                }
            }
            TokenKind::DurationLiteral => {
                self.bump();
                Expr {
                    span: start,
                    kind: ExprKind::Duration(start),
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
            TokenKind::MarkupOpen => self.parse_markup_literal(),
            TokenKind::HeredocOpen | TokenKind::NowdocOpen => self.parse_heredoc_string(),
            TokenKind::Variable => {
                if self.file.span_text(start) == Some("$_") {
                    return self.parse_hole(start);
                }
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
                    .with_help("use an explicit `array<T>` instead — its keys are `string`, so one entry per name"),
                );
                Expr {
                    span,
                    kind: ExprKind::Error(span),
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
                TokenKind::Keyword(Keyword::Function) => self.parse_rejected_function_closure(true),
                TokenKind::Keyword(Keyword::Fn) => self.parse_fn_expr(true),
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
            TokenKind::LBrace => self.parse_object_literal_expr(),
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
            TokenKind::Keyword(Keyword::Function) => self.parse_rejected_function_closure(false),
            TokenKind::Keyword(Keyword::Fn) => self.parse_fn_expr(false),
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
            // `rule:php-migration/let-and-is-are-reserved`: either spelling
            // where an expression begins is a name, and the error node keeps
            // the word from reaching a pass that has no meaning for it.
            TokenKind::Keyword(word @ (Keyword::Let | Keyword::Is)) => {
                let span = self.bump().span;
                self.report_reserved_for_future_use(word, span);
                Expr {
                    span,
                    kind: ExprKind::Error(span),
                }
            }
            TokenKind::Keyword(Keyword::Exit | Keyword::Die) => self.parse_exit(),
            TokenKind::Keyword(Keyword::Eval) => self.parse_eval(),
            TokenKind::Keyword(Keyword::Extract) => self.parse_extract(),
            TokenKind::Keyword(Keyword::Settype) => self.parse_settype(),
            TokenKind::Keyword(Keyword::Require) => self.parse_require(),
            TokenKind::Keyword(
                kw @ (Keyword::Include | Keyword::IncludeOnce | Keyword::RequireOnce),
            ) => self.parse_rejected_include_family(kw),
            TokenKind::Ident if self.at_contextual("await") && self.at_await_operand() => {
                self.parse_await()
            }
            TokenKind::Ident
                if self.at_contextual("spawn") && self.peek_at(1).kind == TokenKind::Ident =>
            {
                let second = self.peek_at(1).span;
                if self.ident_text(second) == "script" {
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
                    kind: ExprKind::Error(span),
                }
            }
        }
    }

    pub(super) fn parse_name_expr(&mut self) -> Expr {
        let name = self.parse_name();
        Expr {
            span: name.span,
            kind: ExprKind::ConstFetch(name),
        }
    }

    // ========================================================================
    // Array literals
    // ========================================================================

    pub(super) fn parse_array_literal_brackets(&mut self) -> Expr {
        let start = self.bump().span; // '['
        let items = self.parse_array_items(TokenKind::RBracket);
        self.expect(TokenKind::RBracket, "`]`");
        let span = start.to(self.last_span);
        Expr {
            span,
            kind: ExprKind::ArrayLiteral(items),
        }
    }

    pub(super) fn parse_array_literal_legacy(&mut self) -> Expr {
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

    pub(super) fn parse_array_items(&mut self, closer: TokenKind) -> Vec<ArrayItem> {
        let mut items = Vec::new();
        while !self.at(closer) && !self.at(TokenKind::Eof) {
            items.push(self.parse_array_item());
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        items
    }

    pub(super) fn parse_array_item(&mut self) -> ArrayItem {
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
    // Object literals (`rule:types/anonymous-object`)
    // ========================================================================

    /// Whether `{` at the current position looks like the start of an
    /// object-literal field (`{ ident :`) rather than a genuine block — the
    /// one-token-past-`{` lookahead the ADR's two "already commits to a
    /// block" call sites ([`Self::parse_fn_expr`]'s arrow body,
    /// [`Self::parse_statement_inner`]'s statement-initial `{`) need before
    /// committing. An empty `{}` never matches, so it stays a block at both
    /// sites, unchanged from before this ADR.
    pub(super) fn at_object_literal_in_block_position(&mut self) -> bool {
        self.peek_at(1).kind == TokenKind::Ident && self.peek_at(2).kind == TokenKind::Colon
    }

    /// Parses the `{...}` at a position [`Self::at_object_literal_in_block_position`]
    /// already confirmed looks like an object literal, reports the
    /// parenthesize-to-disambiguate diagnostic, and discards the parsed
    /// value in favour of [`ExprKind::Error`] — the same "diagnose the
    /// closed, already-identified collision, recover with `Error`" shape
    /// [`Self::parse_unary_inner`]'s legacy-cast handling already uses,
    /// rather than smuggling a literal through a position that was
    /// unambiguously a block a moment ago.
    pub(super) fn parse_object_literal_needs_parens(&mut self) -> Expr {
        let literal = self.parse_object_literal_expr();
        let span = literal.span;
        self.diags.report(
            Diagnostic::error(
                code::E_OBJECT_LITERAL_NEEDS_PARENS,
                "an object literal here is ambiguous with a block",
            )
            .with_primary(span, "`{` already means a block in this position")
            .with_help("wrap it in parentheses: `({...})` (`rule:types/anonymous-object`)"),
        );
        Expr {
            span,
            kind: ExprKind::Error(span),
        }
    }

    /// `{name: value, ...}` as a primary expression — `rule:types/anonymous-object`. No
    /// shorthand (`{x}`) and no computed key (`{[expr]: value}`); either is
    /// diagnosed in place and the field is dropped rather than aborting the
    /// whole literal, so one bad field doesn't hide problems with the rest.
    pub(super) fn parse_object_literal_expr(&mut self) -> Expr {
        let start = self.bump().span; // '{'
        let fields = self.parse_object_literal_fields(TokenKind::RBrace);
        let close = self.expect(TokenKind::RBrace, "`}`");
        Expr {
            span: start.to(close),
            kind: ExprKind::ObjectLiteral(fields),
        }
    }

    /// The `name: value, ...` run between an already-consumed opener and the
    /// `close` its caller is about to expect. Shared with `rule:attributes/attach-sites-and-forms`'s
    /// attribute payload, whose named form writes the same run inside
    /// parentheses — one rule about what a field may be, in one place.
    pub(super) fn parse_object_literal_fields(
        &mut self,
        close: TokenKind,
    ) -> Vec<ObjectLiteralField> {
        let mut fields = Vec::new();
        while !self.at(close) && !self.at(TokenKind::Eof) {
            if self.at(TokenKind::LBracket) {
                let key_start = self.bump().span; // '['
                let _ = self.parse_expr();
                let key_end = self.expect(TokenKind::RBracket, "`]`");
                let key_span = key_start.to(key_end);
                self.diags.report(
                    Diagnostic::error(
                        code::E_OBJECT_LITERAL_COMPUTED_KEY,
                        "an object literal has no computed key",
                    )
                    .with_primary(key_span, "every field name is a static identifier")
                    .with_help("write the literal field name directly, e.g. `{name: value}`"),
                );
                if self.eat(TokenKind::Colon).is_some() {
                    let _ = self.parse_expr();
                }
            } else {
                let field_start = self.peek().span;
                // A field name is a name, not an expression, so a keyword is
                // one: `{default: "ada"}` is `Core\Cli::ask`'s own option
                // (`rule:tooling/a-prompt-is-a-core-member`) and `{match: …}`, `{class: …}` are the shapes
                // a JSON document or an HTML attribute set arrives as. Nothing
                // is ambiguous here — the token is followed by a `:` inside an
                // already-open literal, where no statement keyword can begin —
                // and refusing them would put a spelling in the spec that no
                // call site could write.
                let name = if matches!(self.peek().kind, TokenKind::Keyword(_)) {
                    self.bump().span
                } else {
                    self.expect(TokenKind::Ident, "a field name")
                };
                if self.eat(TokenKind::Colon).is_none() {
                    self.diags.report(
                        Diagnostic::error(
                            code::E_OBJECT_LITERAL_SHORTHAND,
                            "an object literal has no shorthand field",
                        )
                        .with_primary(name, "write the value explicitly")
                        .with_help("write `{name: value}` instead of `{name}`"),
                    );
                } else {
                    let value = self.parse_expr();
                    let span = field_start.to(value.span);
                    fields.push(ObjectLiteralField { name, value, span });
                }
            }
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        fields
    }

    // ========================================================================
    // `new`
    // ========================================================================

    pub(super) fn parse_new(&mut self) -> Expr {
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
                let span = start.to(self.error_expected_expr());
                return Expr {
                    span,
                    kind: ExprKind::Error(span),
                };
            }
        };
        // `new Foo` with no `(...)` is a complete expression, so a following
        // `<` is genuinely ambiguous with a comparison here exactly as it is
        // after a member name. Reusing the call site's own trial parse keeps
        // one answer for both: the list is a list only when it parses cleanly
        // and a `(` follows, and `new Foo < $x` stays a comparison.
        let type_args = self.parse_call_type_args();
        let args = if self.at(TokenKind::LParen) {
            self.parse_call_args()
        } else {
            CallArgs::List(Vec::new())
        };
        let span = start.to(self.last_span);
        Expr {
            span,
            kind: ExprKind::New {
                target,
                type_args,
                args,
            },
        }
    }

    /// `new class (args)? (extends Base)? (implements Iface, ...)? { ... }`
    /// — an anonymous class declaration used directly as a `new` target.
    /// Unlike an ordinary `new Name(args)`, the argument list sits right
    /// after `class`, before `extends`/`implements`/the body, so it cannot
    /// reuse [`Self::parse_new`]'s generic post-target `args` parsing.
    pub(super) fn parse_new_anon_class(&mut self, start: Span) -> Expr {
        let class = self.bump().span; // 'class'
        // Refused at the keyword and then parsed whole: the body's members are
        // still checked, and `nvs_ir` — which has no name to lower this under —
        // is never reached, because a parse error stops the pipeline. See
        // docs/adr/README.md § *Decisions taken at project start*, which
        // refuses a nested declaration in expression position for the reason it
        // refuses a conditional one.
        self.diags.report(
            Diagnostic::error(
                code::E_ANONYMOUS_CLASS_UNSUPPORTED,
                "a class declaration is not an expression",
            )
            .with_primary(start.to(class), "this class has no name to be known by")
            .with_help(
                "declare a named class in the same file and write `new That(…)`, or use a \
                 closure where the class is one method (`rule:types/anonymous-function`)",
            ),
        );
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
                type_args: Vec::new(),
                args,
            },
        }
    }

    /// The class-value expression of `new $expr(...)`/`new (expr)(...)`: a
    /// restricted postfix chain that stops before a call, because the
    /// trailing `(...)` belongs to `new` itself, not to this expression.
    pub(super) fn parse_new_target_expr(&mut self) -> Expr {
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

    pub(super) fn parse_match(&mut self) -> Expr {
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

    /// A declaration's parameter list, where `rule:types/declaration` holds
    /// with nothing to soften it: every parameter names its type.
    pub(super) fn parse_params(&mut self) -> Vec<Param> {
        self.parse_param_list(true)
    }

    /// A closure literal's parameter list, the one list where a parameter may
    /// leave its type out (`rule:types/callable-literal-inference`) and take it
    /// from the position the literal is written in. Nothing else in the
    /// language stands in such a position, so every other list keeps the
    /// refusal.
    pub(super) fn parse_closure_params(&mut self) -> Vec<Param> {
        self.parse_param_list(false)
    }

    /// Every list goes through here, so this is where a second parameter of
    /// one name is `E_BAD_PARAM_LIST`, for a method, a closure and an arrow
    /// function alike. The parameter is kept, so the list still has its
    /// arity.
    fn parse_param_list(&mut self, ty_required: bool) -> Vec<Param> {
        self.expect(TokenKind::LParen, "`(`");
        let mut params: Vec<Param> = Vec::new();
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            let param = self.parse_param(ty_required);
            let name = self.file.span_text(param.name).unwrap_or_default();
            if let Some(first) = params
                .iter()
                .find(|p| !name.is_empty() && self.file.span_text(p.name) == Some(name))
            {
                self.diags.report(
                    Diagnostic::error(
                        code::E_BAD_PARAM_LIST,
                        format!("two parameters are named `{name}`"),
                    )
                    .with_primary(param.name, "rename this parameter")
                    .with_secondary(first.name, "the first one is here"),
                );
            }
            params.push(param);
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::RParen, "`)`");
        params
    }

    /// `attrs? modifiers? 'inout'? Type? '...'? '$'name ('=' default)?` —
    /// `rule:statements/inout-is-the-by-reference-spelling` puts `inout` in the modifier slot `parse_modifiers`
    /// already runs, so it reads like the `public readonly int $x` beside it
    /// and costs the grammar nothing. PHP's `int &$x` is still recognized,
    /// one token past the type, purely so it can be named (E0237).
    ///
    /// `ty_required` is the whole of the difference between the two lists
    /// above, and it decides one thing: whether a missing type is reported
    /// here or left for the checker to fill in from the expected type.
    fn parse_param(&mut self, ty_required: bool) -> Param {
        let start = self.peek().span;
        let attributes = self.parse_attribute_groups();
        let modifiers = self.parse_modifiers(super::decl::ModifierSite::Parameter);
        let inout = self.eat_keyword(Keyword::Inout).is_some();
        let ty = if self.can_start_type() {
            Some(self.parse_type())
        } else if ty_required {
            let span = self.peek().span.shrink_to_start();
            self.diags.report(
                Diagnostic::error(code::E_EXPECTED_TOKEN, "expected a parameter type")
                    .with_primary(
                        span,
                        "every parameter declares a type (`rule:types/declaration`)",
                    ),
            );
            None
        } else {
            None
        };
        if let Some(amp) = self.eat(TokenKind::Amp) {
            self.report_by_reference_marker(amp, "write `inout` before the parameter's type");
        }
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
            inout,
            variadic,
            name,
            default,
        }
    }

    /// `function (...) { ... }` / `function (...) use (...) { ... }`: not a
    /// spelling Novis keeps at all (`rule:types/anonymous-function`) — `fn` covers both a block
    /// and an expression body, so there is nothing left for a second
    /// literal to do. Recovers by parsing the whole shape (params, an
    /// optional `use` clause, an optional return type, the block) so the
    /// parser can keep going, then discards it in favor of `ExprKind::Error`.
    pub(super) fn parse_rejected_function_closure(&mut self, is_static: bool) -> Expr {
        let start = self.peek().span;
        if is_static {
            self.bump();
        }
        let function_span = self.peek().span;
        self.bump(); // `function`
        let _ = self.eat(TokenKind::Amp); // by-ref return — dropped along with this literal
        let _ = self.parse_closure_params();
        let use_clause = self.parse_and_discard_closure_use_clause();
        let _ = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type())
        } else {
            None
        };
        let body = self.in_callable_body(false, Self::parse_block);
        let span = start.to(body.span);
        self.diags.report(
            Diagnostic::error(
                code::E_FUNCTION_CLOSURE_UNSUPPORTED,
                "anonymous `function` literals are not supported",
            )
            .with_primary(function_span, "Novis keeps exactly one closure literal")
            .with_help(
                "use `fn(...) => ...` (an expression body) or `fn(...) => { ... }` (a block body)",
            ),
        );
        if let Some(by_ref) = use_clause {
            self.report_closure_use_clause(span, by_ref);
        }
        Expr {
            span,
            kind: ExprKind::Error(span),
        }
    }

    /// Parses a `use (...)` capture clause if one is present, purely for
    /// error recovery — `fn` has no `use` clause of any kind (`rule:types/implicit-capture`).
    /// Returns `Some(saw_by_ref)` if a clause was present at all.
    pub(super) fn parse_and_discard_closure_use_clause(&mut self) -> Option<bool> {
        self.eat_keyword(Keyword::Use)?;
        self.expect(TokenKind::LParen, "`(`");
        let mut saw_by_ref = false;
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            if self.eat(TokenKind::Amp).is_some() {
                saw_by_ref = true;
            }
            let _ = self.expect(TokenKind::Variable, "a captured variable");
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::RParen, "`)`");
        Some(saw_by_ref)
    }

    /// `rule:types/implicit-capture`/§ 6: a closure has no `use` clause, ever; capture by
    /// reference specifically has no replacement syntax at all.
    pub(super) fn report_closure_use_clause(&mut self, span: Span, by_ref: bool) {
        if by_ref {
            self.diags.report(
                Diagnostic::error(
                    code::E_CLOSURE_USE_BY_REF_UNSUPPORTED,
                    "capture by reference is not supported",
                )
                .with_primary(span, "closures have no `use` clause")
                .with_help("share the value through an object property instead"),
            );
        } else {
            self.diags.report(
                Diagnostic::error(
                    code::E_CLOSURE_USE_UNSUPPORTED,
                    "closures have no `use` clause",
                )
                .with_primary(
                    span,
                    "every outer variable a closure's body reads is captured automatically, \
                     by value",
                ),
            );
        }
    }

    /// `fn [name] (...): T => expr` or `fn [name] (...): T => { ... }` — the
    /// one closure literal (`rule:types/anonymous-function`). `name` is an optional self-name
    /// for recursion (§ 3); a stray `use (...)` clause is still accepted
    /// for recovery and diagnosed the same way the rejected `function`
    /// literal is.
    pub(super) fn parse_fn_expr(&mut self, is_static: bool) -> Expr {
        let start = self.peek().span;
        if is_static {
            self.report_static_closure_modifier(start);
            self.bump();
        }
        self.expect(TokenKind::Keyword(Keyword::Fn), "`fn`");
        let name = self.eat(TokenKind::Ident);
        let params = self.parse_closure_params();
        let use_span = self.peek().span;
        let use_clause = self.parse_and_discard_closure_use_clause();
        if let Some(by_ref) = use_clause {
            self.report_closure_use_clause(use_span, by_ref);
        }
        let return_type = if self.eat(TokenKind::Colon).is_some() {
            Some(self.parse_type())
        } else {
            None
        };
        self.expect(TokenKind::FatArrow, "`=>`");
        let body = if self.at(TokenKind::LBrace) {
            if self.at_object_literal_in_block_position() {
                // `rule:types/anonymous-object`: `{` here already means a block body — an
                // object literal needs `fn() => ({...})` instead.
                FnBody::Expr(Box::new(self.parse_object_literal_needs_parens()))
            } else {
                FnBody::Block(self.in_callable_body(false, Self::parse_block))
            }
        } else {
            FnBody::Expr(Box::new(self.parse_expr()))
        };
        let end = match &body {
            FnBody::Expr(e) => e.span,
            FnBody::Block(b) => b.span,
        };
        let span = start.to(end);
        Expr {
            span,
            kind: ExprKind::Fn(FnExpr {
                is_static,
                name,
                params,
                return_type,
                body,
            }),
        }
    }

    // ========================================================================
    // `yield`, `print`/`throw` already inlined above, `isset`/`empty`/`exit`
    // ========================================================================

    pub(super) fn at_expr_terminator(&mut self) -> bool {
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

    pub(super) fn parse_yield(&mut self) -> Expr {
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

    pub(super) fn parse_isset(&mut self) -> Expr {
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

    pub(super) fn parse_empty(&mut self) -> Expr {
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

    /// `exit`, optionally with a status/message argument. `die` reaches here
    /// too (both keywords dispatch to this method) but is rejected — Novis
    /// keeps exactly one process-termination keyword. See `rule:statements/exit-is-the-only-termination-keyword`.
    pub(super) fn parse_exit(&mut self) -> Expr {
        let is_die = self.at_keyword(Keyword::Die);
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
        if is_die {
            self.diags.report(
                Diagnostic::error(code::E_DIE_UNSUPPORTED, "`die` is not supported")
                    .with_primary(
                        span,
                        "use `exit` instead — it is the only process-termination keyword Novis keeps",
                    )
                    .with_help(
                        "`exit` accepts the same optional status/message argument `die` did",
                    ),
            );
            return Expr {
                span,
                kind: ExprKind::Error(span),
            };
        }
        Expr {
            span,
            kind: ExprKind::Exit(arg),
        }
    }

    // ========================================================================
    // Rejected calls that need their own argument-list parse, so their
    // diagnostic covers the whole call rather than "expected an expression"
    // at a keyword that shouldn't be one
    // ========================================================================

    /// Consumes `'(' expr (',' expr)* ')'` without keeping any of it — for a
    /// rejected pseudo-call whose arguments never reach the AST.
    pub(super) fn skip_call_args(&mut self) {
        self.expect(TokenKind::LParen, "`(`");
        if !self.at(TokenKind::RParen) {
            let _ = self.parse_expr();
            while self.eat(TokenKind::Comma).is_some() && !self.at(TokenKind::RParen) {
                let _ = self.parse_expr();
            }
        }
        self.expect(TokenKind::RParen, "`)`");
    }

    /// `eval(...)` — `rule:types/conversion`: there is no such construct, since a string
    /// has no stable identity to compile ahead of time.
    pub(super) fn parse_eval(&mut self) -> Expr {
        let start = self.bump().span;
        self.skip_call_args();
        let span = start.to(self.last_span);
        self.diags.report(
            Diagnostic::error(code::E_EVAL_UNSUPPORTED, "`eval` is not supported")
                .with_primary(span, "Novis compiles ahead of execution")
                .with_help(
                    "give the code a path: `require` it to share this frame, or `spawn script` \
                     it to isolate it",
                ),
        );
        Expr {
            span,
            kind: ExprKind::Error(span),
        }
    }

    /// `extract(...)` — `rule:types/conversion`: introduces bindings whose names are not
    /// known statically, which every later stage assumes it can enumerate.
    pub(super) fn parse_extract(&mut self) -> Expr {
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
            kind: ExprKind::Error(span),
        }
    }

    /// `settype(...)` — `rule:types/conversion`: no assignment, operator or call may
    /// change what a binding's declared type is; `as` converts into a new
    /// binding instead.
    pub(super) fn parse_settype(&mut self) -> Expr {
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
            kind: ExprKind::Error(span),
        }
    }

    /// `require` — the sole surviving same-frame inclusion keyword
    /// (`rule:statements/require-is-the-only-inclusion-construct`):
    /// an expression, not a statement, per
    /// [`docs/spec/00-overview.md` § 2](/docs/spec/00-overview.md):
    /// same frame, same globals, same statics as the caller. Precedence
    /// mirrors `print`/`throw` above: it consumes a full expression, not
    /// just a primary.
    pub(super) fn parse_require(&mut self) -> Expr {
        let start = self.bump().span;
        let path = self.parse_expr();
        let span = start.to(path.span);
        Expr {
            span,
            kind: ExprKind::Require {
                path: Box::new(path),
            },
        }
    }

    /// `include`/`include_once`/`require_once` — parsed the same shape as
    /// `require` so the diagnostic can cover the whole construct, then
    /// discarded: `rule:statements/require-is-the-only-inclusion-construct`
    /// keeps exactly one same-frame inclusion keyword.
    pub(super) fn parse_rejected_include_family(&mut self, kw: Keyword) -> Expr {
        let start = self.bump().span;
        let _ = self.parse_expr();
        let span = start.to(self.last_span);
        let spelling = match kw {
            Keyword::Include => "include",
            Keyword::IncludeOnce => "include_once",
            Keyword::RequireOnce => "require_once",
            _ => unreachable!("only dispatched for the include/require family"),
        };
        self.diags.report(
            Diagnostic::error(
                code::E_INCLUDE_FAMILY_UNSUPPORTED,
                format!("`{spelling}` is not supported"),
            )
            .with_primary(
                span,
                "Novis keeps exactly one same-frame inclusion construct",
            )
            .with_help(
                "use `require` — it already throws on a missing file and runs every time it is \
                 reached",
            ),
        );
        Expr {
            span,
            kind: ExprKind::Error(span),
        }
    }

    // ========================================================================
    // `spawn script … with(…)`
    // ========================================================================

    pub(super) fn parse_spawn_script(&mut self) -> Expr {
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

    // ========================================================================
    // `await expr`
    // ========================================================================

    /// Whether the token after a contextual `await` begins an operand.
    ///
    /// `await` is contextual for [`crate::token`]'s reason and disambiguated
    /// the way `spawn` is — by what follows it, not by reserving the spelling
    /// — so `await` written alone stays an ordinary name and only a
    /// juxtaposition is the operator. The four kinds listed are every shape an
    /// awaitable can be written as: the variable holding a handle, a
    /// parenthesised expression, a name (`await someHandle`, and also
    /// `await spawn script '…'`, whose `spawn` is itself an [`TokenKind::Ident`]),
    /// and a qualified name's leading `\`.
    ///
    /// The one spelling this claims from a program that predates it is
    /// `await($x)`, which PHP would read as a call to a function named
    /// `await`. That is the intended reading here and the trade is deliberate:
    /// the alternative — excluding `(` — would make `await ($handle)` a
    /// mysterious unknown-function error at the one place a developer is most
    /// likely to reach for parentheses.
    fn at_await_operand(&mut self) -> bool {
        matches!(
            self.peek_at(1).kind,
            TokenKind::Variable | TokenKind::Ident | TokenKind::Backslash | TokenKind::LParen
        )
    }

    /// `await $handle` — `rule:security/isolate-shares-nothing`'s other half.
    ///
    /// The operand is a [`Self::parse_unary`], exactly as `clone`'s is, so
    /// `await $h->result` awaits the property and `await $h + 1` adds to what
    /// the await produced. Nothing binds tighter than the postfix chain here,
    /// which is what makes `await` read as one word in front of the handle
    /// rather than as a call.
    pub(super) fn parse_await(&mut self) -> Expr {
        let start = self.bump().span; // 'await'
        let operand = self.parse_unary();
        let span = start.to(operand.span);
        Expr {
            span,
            kind: ExprKind::Await(Box::new(operand)),
        }
    }

    pub(super) fn parse_spawn_option(&mut self) -> SpawnOption {
        let start = self.peek().span;
        let key_span = self.expect(TokenKind::Ident, "a `with(...)` option name");
        let key_text = self.ident_text(key_span);
        let key = match key_text {
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

    pub(super) fn parse_double_quoted_string(&mut self) -> Expr {
        let open = self.bump().span; // DoubleQuoteOpen
        let (parts, close) = self.parse_string_body(TokenKind::DoubleQuoteClose);
        collapse_string_parts(open.to(close), parts)
    }

    pub(super) fn parse_heredoc_string(&mut self) -> Expr {
        let open = self.bump().span; // HeredocOpen or NowdocOpen
        let (parts, close) = self.parse_string_body(TokenKind::HeredocClose);
        collapse_string_parts(open.to(close), parts)
    }

    /// `` html`<span>{$name}</span>` `` — the string body with the backtick as
    /// its closer (`rule:core-classes/html-literal`).
    ///
    /// It does not collapse a hole-free body the way a quoted string does: the
    /// node is what says `Core\Html\Markup`, so ``html`<hr>` `` must stay an
    /// [`ExprKind::Markup`] holding one text part rather than becoming an
    /// [`ExprKind::Str`].
    pub(super) fn parse_markup_literal(&mut self) -> Expr {
        let open = self.bump().span; // MarkupOpen
        let (parts, close) = self.parse_string_body(TokenKind::MarkupClose);
        Expr {
            span: open.to(close),
            kind: ExprKind::Markup(parts),
        }
    }

    pub(super) fn parse_string_body(&mut self, closer: TokenKind) -> (Vec<StringPart>, Span) {
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
                // The same part as a brace hole: what differs is only which
                // expressions the lexer let through, and by here that is
                // decided (`rule:core-classes/html-literal`).
                TokenKind::MarkupEchoOpen => {
                    self.bump();
                    let e = self.parse_expr();
                    self.expect(TokenKind::MarkupEchoClose, "`?>`");
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
    pub(super) fn parse_simple_interp_variable(&mut self) -> Expr {
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
                            kind: ExprKind::Error(span),
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
}
