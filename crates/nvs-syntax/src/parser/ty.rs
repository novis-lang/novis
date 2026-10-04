//! The type grammar (`rule:types/grammar`) — and the qualified name every other layer
//! parses through.
//!
//! The whole of it: nested `array<T>`, DNF unions and intersections, the
//! `?T` sugar, `tainted`/`secret` qualifiers (`rule:security/tainted-qualifier`, `rule:security/secret-qualifier`), ADR
//! 0036 § 3's inline `{name: T}` shape, `rule:types/single-value-types`'s literal and enum-case
//! atoms, and `decimal` (`rule:types/decimal`). A `>>` closing two nested generics is
//! split back into two `>` closes here rather than in the lexer — see
//! [`Parser::expect_type_close_angle`].
//!
//! What a type *means* is `nvs-types`'s; this module only says which spellings
//! parse. The one exception is a spelling refused outright at parse time —
//! a `float` literal in type position, an interpolated string — where the
//! diagnostic belongs to the grammar that rejects it.
//!
//! Part of [`super`]'s one `impl Parser`, split across this directory so a
//! session editing one layer of the grammar does not carry the rest in
//! context. Every item moved here unchanged; the methods are `pub(super)` so
//! they reach across these modules and no further, which is the reach they had
//! when `parser` was a single file.

use super::*;

impl<'src, 'd> Parser<'src, 'd> {
    /// The one class a `catch` names, in either of its two forms.
    ///
    /// A union parses — the type grammar has no reason to refuse `A|B` here
    /// and refusing it in the grammar would cost a worse diagnostic — and is
    /// then rejected, because `rule:types/declaration` gives the binding one static type
    /// and a clause naming two classes has no type to give it. The clause is
    /// still built, from the first class alone, so the block's or the arm's
    /// own body is checked rather than abandoned and the file reports the rest
    /// of its problems in the same run.
    ///
    /// `help` is the caller's: the block form's fix is another clause and
    /// `rule:expressions/catch-expression`'s is another arm. The message above it is shared, since § 1 makes
    /// an arm a clause of one guard and "a `catch` clause names one class" is
    /// true of both.
    pub(super) fn parse_caught_type(&mut self, help: &str) -> Type {
        match self.parse_type() {
            Type {
                kind: TypeKind::Union(items),
                span,
            } => {
                self.diags.report(
                    Diagnostic::error(
                        code::E_CATCH_UNION_TYPE_UNSUPPORTED,
                        "a `catch` clause names one class",
                    )
                    .with_primary(span, "this names two")
                    .with_help(help),
                );
                items
                    .into_iter()
                    .next()
                    .expect("a union holds at least two members")
            }
            ty => ty,
        }
    }

    /// Whether the current token could begin a type expression — used to
    /// decide, without committing, whether a mandatory type was actually
    /// omitted (e.g. a parameter written without one).
    pub(super) fn can_start_type(&mut self) -> bool {
        Self::token_starts_type(self.peek().kind)
            || self.at_negative_int_literal()
            || self.at_class_reference()
    }

    /// `class<` — `rule:types/class-reference`'s class reference, the second type atom that
    /// takes two tokens to recognise and so is asked here rather than in
    /// [`Self::token_starts_type`]. The lookahead is not an optimisation: a
    /// bare `class` is the *declaration* keyword, and answering `true` for it
    /// on its own would route every `class Foo {}` in the program through
    /// [`Self::parse_stmt_maybe_local_decl`]'s trial parse.
    pub(super) fn at_class_reference(&mut self) -> bool {
        self.at_keyword(Keyword::Class) && matches!(self.peek_at(1).kind, TokenKind::Lt)
    }

    /// `property<` — `rule:types/property-key`'s property key, recognised by spelling
    /// because `property` is deliberately not a reserved word: the ADR makes it
    /// a keyword in this one position and nowhere else, so a program keeps
    /// `$property`, `->property()` and a function called `property`.
    ///
    /// The `<` is what separates it from an ordinary class name, and it costs
    /// nothing that a class *could* be called `property`: `rule:core-api/identifier-casing`'s casing
    /// check already refuses a lower-case class name, so no declaration this
    /// program can write competes for the spelling.
    pub(super) fn at_property_key(&mut self) -> bool {
        self.at_contextual("property") && matches!(self.peek_at(1).kind, TokenKind::Lt)
    }

    /// `-1` — `rule:types/single-value-types`'s one type atom that needs two tokens to
    /// recognise, which is why it is asked here rather than in
    /// [`Self::token_starts_type`]. A bare `-` never starts a type on its
    /// own: routing every statement-initial `-$x;` through
    /// [`Self::parse_stmt_maybe_local_decl`]'s trial parse would cost a
    /// backtrack on an extremely common shape for nothing.
    pub(super) fn at_negative_int_literal(&mut self) -> bool {
        self.at(TokenKind::Minus) && matches!(self.peek_at(1).kind, TokenKind::IntLiteral)
    }

    /// The token-kind half of [`Self::can_start_type`], factored out so a
    /// statement-position lookahead ([`Self::at_function_scope_static`]) can
    /// ask the same question one token further ahead without a second,
    /// drifting copy of this list.
    pub(super) fn token_starts_type(kind: TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Keyword(
                Keyword::Null
                    | Keyword::Bool
                    | Keyword::Int
                    | Keyword::Uint
                    | Keyword::Float
                    | Keyword::Decimal
                    | Keyword::String
                    | Keyword::Bytes
                    | Keyword::Tainted
                    | Keyword::Secret
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
                | TokenKind::LBrace
                // `rule:types/single-value-types`'s two atoms, a string and an int. A statement that merely
                // *starts* with one (`1 + 2;`, `"x" . $y;`) is no longer a
                // free ride to the expression path, but it still gets there:
                // `parse_stmt_maybe_local_decl` trial-parses the type and
                // backtracks the moment no `$name` follows.
                | TokenKind::IntLiteral
                | TokenKind::SingleQuotedString
                | TokenKind::DoubleQuoteOpen
                // Not a type — but `rule:types/single-value-types`'s diagnostic is worth more
                // than the "expected a type" this would otherwise get.
                | TokenKind::FloatLiteral
        )
    }

    /// Whether `static` at the current position starts the rejected
    /// function-scope storage declaration (`rule:statements/no-function-static-and-no-global`) rather than an
    /// ordinary `static::`/`static function`/`static fn`/bare-`static`
    /// expression — decided by one token of lookahead past `static` itself:
    /// a `$name` (the ordinary untyped PHP spelling) or a type-start token
    /// (the ADR's own illustrative typed spelling) both mean this is the
    /// rejected construct.
    pub(super) fn at_function_scope_static(&mut self) -> bool {
        matches!(self.peek_at(1).kind, TokenKind::Variable)
            || Self::token_starts_type(self.peek_at(1).kind)
    }

    /// Parses one type expression: `union`.
    pub(crate) fn parse_type(&mut self) -> Type {
        self.parse_type_union()
    }

    /// The right-hand side of `is` (`rule:types/type-test` § *The value arm*).
    ///
    /// **The `$` decides, and nothing else does.** A `$variable` opens the
    /// value arm — the dynamic class test, whose operand is parsed at the `|>`
    /// level exactly as any other value in an operator position, so
    /// `$x is $this->cls` reads as one operand. Every other token starts a
    /// type. Deciding on the sigil rather than on what happens to parse is what
    /// keeps a DNF type's opening `(` a type, and it leaves the arms with no
    /// token in common for a later reader to have to disambiguate.
    pub(super) fn parse_test_operand(&mut self) -> TestOperand {
        if self.at(TokenKind::Variable) {
            return TestOperand::Value(Box::new(self.parse_pipe()));
        }
        TestOperand::Type(self.parse_type())
    }

    pub(super) fn parse_type_union(&mut self) -> Type {
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

    pub(super) fn parse_type_intersection(&mut self) -> Type {
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
    /// than being PHP's retired by-reference marker, which follows a type
    /// with nothing between them (`int &$x` — a parameter, a `foreach` value
    /// binding, a destructuring leaf). Both shapes start identically; the
    /// deciding token is one further ahead: an intersection member is always
    /// another type atom, never a bare `$name`.
    ///
    /// `rule:statements/ampersand-is-not-a-by-reference-marker` retires that marker but keeps it *recognizable*, so this
    /// survives the removal rather than being deleted with it: the site one
    /// level up names the exact fix (E0237, `Parser::report_by_reference_marker`),
    /// which it can only do if the type parser leaves the `&` alone instead
    /// of consuming it and then failing on `$x` as a malformed intersection
    /// member. The ADR's own § 3 is the home of that correction.
    pub(super) fn at_intersection_amp(&mut self) -> bool {
        self.at(TokenKind::Amp) && Self::token_starts_type(self.peek_at(1).kind)
    }

    /// Self-recursive both for a nested `?` (`??int`, absurd but legal
    /// grammar) and via [`Self::parse_type_union`] for a parenthesized type
    /// (`((((int))))`) — needs the same recursion guard as
    /// [`Self::parse_assignment`], and guarding it alone is enough to bound
    /// the paren cycle too, since `parse_type_union`/`_intersection` always
    /// lead straight back here with no branching in between.
    pub(super) fn parse_type_operand(&mut self) -> Type {
        self.guarded(Self::error_type_here, Self::parse_type_operand_inner)
    }

    pub(super) fn error_type_here(&mut self) -> Type {
        let span = self.peek().span.shrink_to_start();
        Type {
            kind: TypeKind::Atom(TypeAtom::Mixed),
            span,
        }
    }

    pub(super) fn parse_type_operand_inner(&mut self) -> Type {
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
    /// this is the one place that ambiguity has to be resolved (`rule:types/grammar`).
    pub(super) fn expect_type_close_angle(&mut self) -> Span {
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

    /// An optional `<T, U>` suffix on a name, in type position or after an
    /// `implements` entry. Returns what was written (empty when there was no
    /// `<` at all) and the span covering `name_span` through the closing `>`.
    ///
    /// The parser deliberately accepts this after *any* name and imposes no
    /// arity: `rule:iteration/concrete-generic-implements`'s one narrow door is `Iterable`/`Iterator`, but which names are
    /// generic is `nvs_hir::interfaces`' roster and only the checker reads
    /// it. Parsing `Foo<int>` and refusing it later gets the reader a
    /// diagnostic that names the rule, instead of a cascade off a `<` that
    /// was read as a comparison.
    ///
    /// Nothing here needs the lookahead the expression side would: a type
    /// position never admits a `<` operator, so a `<` after a name in one is
    /// unambiguously a type-argument list.
    pub(super) fn parse_type_args(&mut self, name_span: Span) -> (Vec<Type>, Span) {
        if self.eat(TokenKind::Lt).is_none() {
            return (Vec::new(), name_span);
        }
        let mut args = vec![self.parse_type_union()];
        while self.eat(TokenKind::Comma).is_some() {
            args.push(self.parse_type_union());
        }
        let close = self.expect_type_close_angle();
        (args, name_span.to(close))
    }

    /// `self`, `static` and `parent` are whole atoms in type position, so a
    /// `::` after one is `E0135` rather than the `expected ';'` a bare atom
    /// followed by a stray token would otherwise produce
    /// (`rule:types/grammar`). The refused pair is consumed so the rest of the
    /// declaration still parses, and what it recovers as is `mixed` — the
    /// wildcard reports nothing further, where recovering as the keyword's own
    /// atom would have the checker complain about a type the program never
    /// wrote. A statement-initial declaration is the one position this does not
    /// reach the reader: `parse_stmt_maybe_local_decl`'s trial parse treats any
    /// diagnostic as proof the tokens were an expression, which is equally true
    /// of every other refusal in this file.
    fn parse_class_keyword_type(&mut self, atom: TypeAtom, keyword: &str) -> Type {
        let start = self.bump().span;
        if !(self.at(TokenKind::DoubleColon) && Self::is_name_segment(self.peek_at(1).kind)) {
            return Type {
                kind: TypeKind::Atom(atom),
                span: start,
            };
        }
        self.bump();
        let member = self.bump().span;
        let span = start.to(member);
        self.diags.report(
            Diagnostic::error(
                code::E_CLASS_KEYWORD_MEMBER_IN_TYPE,
                format!("`{keyword}::` is not a type"),
            )
            .with_primary(span, "a class keyword is a whole type on its own")
            .with_help(
                "name the owner class — `Owner::MEMBER` — or, for a `type` alias inside the body \
                 that declares it, write the alias's bare name (`rule:types/grammar`)",
            ),
        );
        Type {
            kind: TypeKind::Atom(TypeAtom::Mixed),
            span,
        }
    }

    pub(super) fn parse_type_atom(&mut self) -> Type {
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
            TokenKind::Keyword(Keyword::Decimal) => atom!(Decimal),
            TokenKind::Keyword(Keyword::String) => atom!(String),
            TokenKind::Keyword(Keyword::Bytes) => atom!(Bytes),
            TokenKind::Keyword(Keyword::Tainted) => {
                self.bump();
                let inner = self.parse_type_atom();
                let span = start.to(inner.span);
                let atom = match inner.kind {
                    TypeKind::Atom(TypeAtom::String) => TypeAtom::TaintedString,
                    TypeKind::Atom(TypeAtom::Bytes) => TypeAtom::TaintedBytes,
                    // `rule:security/tainted-qualifier`: over a shape the
                    // qualifier distributes to every text-carrying field and is
                    // then gone, so what reaches the checker is the
                    // field-by-field spelling this saves writing. A shape with
                    // no text anywhere is diagnosed rather than accepted as a
                    // no-op — a qualifier that promises nothing still reads as
                    // a promise.
                    TypeKind::Atom(TypeAtom::Shape(fields)) => {
                        let mut carries_text = false;
                        let fields = fields
                            .into_iter()
                            .map(|field| ShapeField {
                                ty: Self::taint_type(field.ty, &mut carries_text),
                                ..field
                            })
                            .collect();
                        if !carries_text {
                            self.diags.report(
                                Diagnostic::error(
                                    code::E_TAINTED_SHAPE_HAS_NO_TEXT,
                                    "`tainted` qualifies nothing in this shape",
                                )
                                .with_primary(span, "no `string` or `bytes` field anywhere in it")
                                .with_help(
                                    "drop the qualifier, or write the field this shape was \
                                     meant to carry (`rule:security/tainted-qualifier`)",
                                ),
                            );
                        }
                        TypeAtom::Shape(fields)
                    }
                    other @ TypeKind::Atom(
                        TypeAtom::SecretString
                        | TypeAtom::SecretBytes
                        | TypeAtom::SecretTaintedString
                        | TypeAtom::SecretTaintedBytes,
                    ) => {
                        // `rule:security/secret-qualifier`: `secret` and `tainted` compose, but
                        // only `secret` first — `inner` already built a
                        // `Secret*` atom, meaning the source spelled `secret`
                        // before this `tainted`, i.e. wrote the rejected
                        // order (`tainted secret string`).
                        self.diags.report(
                            Diagnostic::error(
                                code::E_SECRET_TAINTED_ORDER,
                                "`secret` must come before `tainted`",
                            )
                            .with_primary(span, "wrong qualifier order")
                            .with_help(
                                "write `secret tainted string`/`secret tainted bytes` \
                                 (`rule:security/secret-qualifier`)",
                            ),
                        );
                        return Type { kind: other, span };
                    }
                    other => {
                        self.diags.report(
                            Diagnostic::error(
                                code::E_TAINTED_NON_SCALAR,
                                "`tainted` only qualifies `string`, `bytes` and a shape of them",
                            )
                            .with_primary(span, "not something `tainted` can qualify")
                            .with_help("write `tainted string`, `tainted bytes` or `tainted {…}` (`rule:security/tainted-qualifier`)"),
                        );
                        return Type { kind: other, span };
                    }
                };
                Type {
                    kind: TypeKind::Atom(atom),
                    span,
                }
            }
            TokenKind::Keyword(Keyword::Secret) => {
                self.bump();
                let inner = self.parse_type_atom();
                let span = start.to(inner.span);
                let atom = match inner.kind {
                    TypeKind::Atom(TypeAtom::String) => TypeAtom::SecretString,
                    TypeKind::Atom(TypeAtom::Bytes) => TypeAtom::SecretBytes,
                    TypeKind::Atom(TypeAtom::TaintedString) => TypeAtom::SecretTaintedString,
                    TypeKind::Atom(TypeAtom::TaintedBytes) => TypeAtom::SecretTaintedBytes,
                    _ => {
                        self.diags.report(
                            Diagnostic::error(
                                code::E_SECRET_NON_SCALAR,
                                "`secret` only qualifies `string`/`bytes`",
                            )
                            .with_primary(span, "not a scalar `secret` can qualify")
                            .with_help("write `secret string` or `secret bytes` (`rule:security/secret-qualifier`)"),
                        );
                        return Type {
                            kind: inner.kind,
                            span,
                        };
                    }
                };
                Type {
                    kind: TypeKind::Atom(atom),
                    span,
                }
            }
            TokenKind::Keyword(Keyword::Object) => atom!(Object),
            TokenKind::LBrace => self.parse_shape_type(),
            TokenKind::Keyword(Keyword::Mixed) => atom!(Mixed),
            TokenKind::Keyword(Keyword::Void) => atom!(Void),
            TokenKind::Keyword(Keyword::Never) => atom!(Never),
            TokenKind::Keyword(Keyword::True) => atom!(True),
            TokenKind::Keyword(Keyword::False) => atom!(False),
            TokenKind::Keyword(Keyword::Iterable) => atom!(Iterable),
            TokenKind::Keyword(Keyword::Callable) => self.parse_callable_type(),
            TokenKind::Keyword(Keyword::SelfKw) => {
                self.parse_class_keyword_type(TypeAtom::SelfTy, "self")
            }
            TokenKind::Keyword(Keyword::Static) => {
                self.parse_class_keyword_type(TypeAtom::StaticTy, "static")
            }
            TokenKind::Keyword(Keyword::Parent) => {
                self.parse_class_keyword_type(TypeAtom::Parent, "parent")
            }
            // `rule:types/class-reference`'s class reference. `class` is already the
            // declaration keyword, so this arm is only reached through
            // `at_class_reference` (a `<` immediately after it) and the two
            // spellings never compete: a declaration is `class Name`.
            TokenKind::Keyword(Keyword::Class) if self.at_class_reference() => {
                self.bump();
                self.bump();
                let inner = self.parse_type_union();
                let close = self.expect_type_close_angle();
                Type {
                    kind: TypeKind::Atom(TypeAtom::ClassRef(Box::new(inner))),
                    span: start.to(close),
                }
            }
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
            // `rule:types/property-key`'s property key. This sits in front of the ordinary
            // name arm because `property` reaches the parser as a plain
            // identifier; the `<` in `at_property_key` is the whole of what
            // tells the two apart, and a name that merely *looks* generic
            // (`Iterator<User>`) still lands below.
            TokenKind::Ident if self.at_property_key() => {
                self.bump();
                self.bump();
                let inner = self.parse_type_union();
                let close = self.expect_type_close_angle();
                Type {
                    kind: TypeKind::Atom(TypeAtom::PropertyKey(Box::new(inner))),
                    span: start.to(close),
                }
            }
            TokenKind::Ident | TokenKind::Backslash => {
                let name = self.parse_name();
                // `rule:types/constant-in-type-position` and `rule:types/enum-case-type`: `Foo::BAR` in type position. Which of the
                // two meanings it has depends on what `Foo` resolves to, so
                // the parser records the pair and stops there — exactly what
                // it already does for a bare name. A type-argument list is
                // not offered after `::`: neither a class constant nor an
                // enum case is generic.
                if self.at(TokenKind::DoubleColon) && Self::is_name_segment(self.peek_at(1).kind) {
                    self.bump();
                    let member = self.bump().span;
                    let span = name.span.to(member);
                    return Type {
                        kind: TypeKind::Atom(TypeAtom::Member(name, member)),
                        span,
                    };
                }
                let (args, span) = self.parse_type_args(name.span);
                Type {
                    kind: TypeKind::Atom(TypeAtom::Name(name, args)),
                    span,
                }
            }
            // `rule:types/single-value-types`'s atoms, the generalisation of the `true`
            // and `false` atoms just above from `bool`'s two values to every
            // `string` and `int`.
            TokenKind::SingleQuotedString => {
                self.bump();
                Type {
                    kind: TypeKind::Atom(TypeAtom::SingleValueString(start)),
                    span: start,
                }
            }
            TokenKind::DoubleQuoteOpen => self.parse_string_single_value_type(),
            TokenKind::IntLiteral => {
                self.bump();
                Type {
                    kind: TypeKind::Atom(TypeAtom::SingleValueInt(start)),
                    span: start,
                }
            }
            TokenKind::Minus if self.at_negative_int_literal() => {
                self.bump();
                let lit = self.bump().span;
                let span = start.to(lit);
                Type {
                    kind: TypeKind::Atom(TypeAtom::SingleValueInt(span)),
                    span,
                }
            }
            TokenKind::FloatLiteral => {
                self.bump();
                self.reject_float_single_value_type(start)
            }
            TokenKind::Minus if matches!(self.peek_at(1).kind, TokenKind::FloatLiteral) => {
                self.bump();
                let lit = self.bump().span;
                self.reject_float_single_value_type(start.to(lit))
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

    /// A double-quoted `"a"` in type position — `rule:types/single-value-types`'s string literal
    /// atom, spelled the way the ADR spells it. The body is read with the
    /// ordinary [`Self::parse_string_body`] so escapes lex identically to a
    /// value position's, and an interpolated one is refused: a type has no
    /// scope to interpolate a variable from. The refused case still yields
    /// the atom, so the rest of the declaration parses on.
    pub(super) fn parse_string_single_value_type(&mut self) -> Type {
        let open = self.bump().span; // DoubleQuoteOpen
        let (parts, close) = self.parse_string_body(TokenKind::DoubleQuoteClose);
        let span = open.to(close);
        if !parts.iter().all(|p| matches!(p, StringPart::Text(_))) {
            self.diags.report(
                Diagnostic::error(
                    code::E_INTERPOLATION_IN_TYPE,
                    "a single-value string type cannot contain variables",
                )
                .with_primary(span, "this type names one exact string")
                .with_help(
                    "write the string out — a type is resolved at compile time, so there is \
                     nothing to interpolate from (`rule:types/single-value-types`)",
                ),
            );
        }
        Type {
            kind: TypeKind::Atom(TypeAtom::SingleValueString(span)),
            span,
        }
    }

    /// `rule:types/single-value-types`: there is no single-value `float` type, deferred until
    /// floating-point equality has a real answer. Diagnosed by name rather
    /// than left to `error_expected("a type")`, since the reason a reader
    /// needs is "not this type, on purpose" and not "unparseable here".
    pub(super) fn reject_float_single_value_type(&mut self, span: Span) -> Type {
        self.diags.report(
            Diagnostic::error(
                code::E_FLOAT_SINGLE_VALUE_TYPE,
                "a `float` value cannot be used as a type",
            )
            .with_primary(
                span,
                "only a string or a whole number can be a single-value type",
            )
            .with_help(
                "use `float` and check the value, or list the allowed values as whole \
                 numbers (`rule:types/single-value-types`)",
            ),
        );
        Type {
            kind: TypeKind::Atom(TypeAtom::Float),
            span,
        }
    }

    /// `callable`, and the signature `rule:types/callable-signature` lets it
    /// carry: `'callable' '(' (type (',' type)*)? ')' ':' type`.
    ///
    /// One token of lookahead decides between the two spellings, with no
    /// checkpointed trial parse: type position never admits a call, so a `(`
    /// after `callable` can only open a parameter list. Bare `callable` stays
    /// [`TypeAtom::Callable`] — the top of the lattice, not a signature whose
    /// list happens to be empty.
    ///
    /// The return type is parsed as a whole union, so `callable(): int|string`
    /// returns the union. A union *over* a callable is written
    /// `(callable(): int)|string`, which [`Self::parse_type_operand_inner`]'s
    /// paren operand already parses.
    ///
    /// Both refusals recover as bare `callable`, dropping the list that was
    /// written. That is the type every callable value satisfies, so a file
    /// that misspells one signature reports that mistake and not a second one
    /// about every use of the value, which is what recovering into an invented
    /// signature would cost.
    pub(super) fn parse_callable_type(&mut self) -> Type {
        let start = self.bump().span; // 'callable'
        if !self.at(TokenKind::LParen) {
            return Type {
                kind: TypeKind::Atom(TypeAtom::Callable),
                span: start,
            };
        }
        self.bump(); // '('
        let mut params = Vec::new();
        let mut named_at: Option<Span> = None;
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            let ty = self.parse_type();
            // `callable(int $x): string`. The type itself is well formed and
            // only the name is refused, so every name is consumed — the `eat`
            // runs before the `&&` is asked — and the parameter kept, with one
            // report below however many of them carried one.
            if let Some(name) = self.eat(TokenKind::Variable)
                && named_at.is_none()
            {
                named_at = Some(name);
            }
            params.push(ty);
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let close = self.expect(TokenKind::RParen, "`)`");
        if let Some(name) = named_at {
            self.diags.report(
                Diagnostic::error(
                    code::E_CALLABLE_TYPE_NAMES_A_PARAMETER,
                    "a `callable` type does not name its parameters",
                )
                .with_primary(name, "a parameter name")
                .with_help(
                    "write the types alone, as `callable(int): string` — a name here would \
                     imply calling through the value by name (`rule:types/callable-signature`)",
                ),
            );
        }
        if self.eat(TokenKind::Colon).is_none() {
            let span = start.to(close);
            self.diags.report(
                Diagnostic::error(
                    code::E_CALLABLE_TYPE_WITHOUT_RETURN,
                    "a `callable` type must declare its return type",
                )
                .with_primary(span, "no `:` and return type after the parameter list")
                .with_help(
                    "write `callable(int): void` if it returns nothing — a parameter list \
                     without a return type says less than bare `callable` \
                     (`rule:types/callable-signature`)",
                ),
            );
            return Type {
                kind: TypeKind::Atom(TypeAtom::Callable),
                span,
            };
        }
        let ret = self.parse_type();
        let span = start.to(ret.span);
        Type {
            kind: TypeKind::Atom(TypeAtom::CallableSig {
                params,
                ret: Box::new(ret),
            }),
            span,
        }
    }

    /// `{name: T, name?: T, ...}` in type position — `rule:types/shape-type`,
    /// Novis's one structurally-checked type. No ambiguity to resolve here the
    /// way the value literal has (see [`Self::parse_anon_object_expr`]): type
    /// position never dispatches `{` to a block, so an empty `{}` is simply
    /// an empty shape rather than needing the literal's "at least one field"
    /// rule.
    ///
    /// The `?` sits before the `:` and marks the *key* optional, which is why
    /// it cannot be confused with the `?` of a nullable type after it: `{a?:
    /// ?int}` says both, and `{a?: int}` and `{a: ?int}` say different things.
    pub(super) fn parse_shape_type(&mut self) -> Type {
        let start = self.bump().span; // '{'
        let mut fields = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let field_start = self.peek().span;
            // A keyword is a field name here for the reason the value literal
            // gives in `Self::parse_anon_object_expr`: `{class: string}`
            // is the type of `{class: …}`, and a value the literal can build
            // needs a type a declaration can write.
            let name = if matches!(self.peek().kind, TokenKind::Keyword(_)) {
                self.bump().span
            } else {
                self.expect(TokenKind::Ident, "a field name")
            };
            let required = self.eat(TokenKind::Question).is_none();
            self.expect(TokenKind::Colon, "`:`");
            let ty = self.parse_type();
            let span = field_start.to(ty.span);
            fields.push(ShapeField {
                name,
                ty,
                required,
                span,
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let close = self.expect(TokenKind::RBrace, "`}`");
        Type {
            kind: TypeKind::Atom(TypeAtom::Shape(fields)),
            span: start.to(close),
        }
    }

    /// `tainted {…}`'s distribution — `rule:security/tainted-qualifier`. Every
    /// text-carrying leaf under `ty` becomes its tainted form, transitively:
    /// through a nullable, a grouping, a union or intersection member, an
    /// `array<T>` element and a nested shape's fields. `carries_text` is set by
    /// any leaf that is text at all, already-tainted ones included, because the
    /// question the caller asks is whether the qualifier promises anything —
    /// not whether this rewrite changed something.
    fn taint_type(ty: Type, carries_text: &mut bool) -> Type {
        let kind = match ty.kind {
            TypeKind::Nullable(inner) => {
                TypeKind::Nullable(Box::new(Self::taint_type(*inner, carries_text)))
            }
            TypeKind::Paren(inner) => {
                TypeKind::Paren(Box::new(Self::taint_type(*inner, carries_text)))
            }
            TypeKind::Union(members) => TypeKind::Union(
                members
                    .into_iter()
                    .map(|member| Self::taint_type(member, carries_text))
                    .collect(),
            ),
            TypeKind::Intersection(members) => TypeKind::Intersection(
                members
                    .into_iter()
                    .map(|member| Self::taint_type(member, carries_text))
                    .collect(),
            ),
            TypeKind::Atom(atom) => TypeKind::Atom(Self::taint_atom(atom, carries_text)),
        };
        Type {
            kind,
            span: ty.span,
        }
    }

    /// [`Self::taint_type`] at a leaf. A `secret` scalar becomes the composed
    /// `secret tainted` atom rather than a diagnostic: the rejected order is
    /// `tainted secret` *written* that way (`rule:security/secret-qualifier`),
    /// and a field the shape's qualifier reaches was written `secret` first.
    fn taint_atom(atom: TypeAtom, carries_text: &mut bool) -> TypeAtom {
        match atom {
            TypeAtom::String => {
                *carries_text = true;
                TypeAtom::TaintedString
            }
            TypeAtom::Bytes => {
                *carries_text = true;
                TypeAtom::TaintedBytes
            }
            TypeAtom::SecretString => {
                *carries_text = true;
                TypeAtom::SecretTaintedString
            }
            TypeAtom::SecretBytes => {
                *carries_text = true;
                TypeAtom::SecretTaintedBytes
            }
            already @ (TypeAtom::TaintedString
            | TypeAtom::TaintedBytes
            | TypeAtom::SecretTaintedString
            | TypeAtom::SecretTaintedBytes) => {
                *carries_text = true;
                already
            }
            TypeAtom::Array(Some(elem)) => {
                TypeAtom::Array(Some(Box::new(Self::taint_type(*elem, carries_text))))
            }
            TypeAtom::Shape(fields) => TypeAtom::Shape(
                fields
                    .into_iter()
                    .map(|field| ShapeField {
                        ty: Self::taint_type(field.ty, carries_text),
                        ..field
                    })
                    .collect(),
            ),
            other => other,
        }
    }

    /// A possibly-namespace-qualified name: `Foo`, `Core\Bytes`,
    /// `App\Models\User`.
    ///
    /// A **leading** separator is refused here rather than folded into the
    /// span, per
    /// `rule:statements/a-leading-separator-does-not-parse`:
    /// a name with a separator in it is already read from the root, so the
    /// prefix has no work left to do. The token is still consumed after the
    /// report, so the rest of the name parses and one mistake yields one
    /// diagnostic — the same recovery [`Self::parse_use_decl`] gives a
    /// rename and a group import.
    ///
    /// A segment may be a reserved word's spelling: a `camelCase` segment
    /// such as `list` lexes as a keyword, and `Foo\list` still has to parse
    /// per [`docs/spec/00-overview.md` § 5](/docs/spec/00-overview.md).
    /// `Core\Bytes` no longer needs this tolerance —
    /// `rule:classes/reserved-spellings-are-lower-case` made keyword matching exact, so `Bytes` is an ordinary
    /// [`TokenKind::Ident`] instead of colliding with the `bytes` type
    /// keyword.
    /// Once a name is expected at all (this is only ever called after an
    /// `Ident` or `Backslash` was already seen), a keyword spelling here is
    /// unambiguous — the same reasoning already applied to a member name
    /// after `->`/`::` in [`Self::parse_member_name`].
    pub(super) fn parse_name(&mut self) -> Name {
        // Taken *after* the separator below, so a `Name`'s span never covers
        // one. Excluding it is what lets every downstream reader take the
        // span text as the name itself: nothing strips a leading separator
        // anywhere, because on the one path that writes one the span already
        // starts past it, and on every other path there is none to strip.
        if let Some(slash) = self.eat(TokenKind::Backslash) {
            self.diags.report(
                Diagnostic::error(
                    code::E_LEADING_BACKSLASH_UNSUPPORTED,
                    "a name is already absolute",
                )
                .with_primary(slash, "remove the leading `\\`")
                .with_help(
                    "a name with a `\\` in it is read from the root, and one without resolves \
                     through the imports then the enclosing namespace (`rule:statements/a-qualified-name-is-absolute` and `rule:statements/a-leading-separator-does-not-parse`)",
                ),
            );
        }
        let start = self.peek().span;
        let mut last = self.expect_name_segment();
        while self.at(TokenKind::Backslash) && Self::is_name_segment(self.peek_at(1).kind) {
            self.bump();
            last = self.bump().span;
        }
        Name {
            span: start.to(last),
        }
    }

    pub(super) fn is_name_segment(kind: TokenKind) -> bool {
        matches!(kind, TokenKind::Ident | TokenKind::Keyword(_))
    }

    pub(super) fn expect_name_segment(&mut self) -> Span {
        if Self::is_name_segment(self.peek().kind) {
            self.bump().span
        } else {
            self.error_expected("a name")
        }
    }
}
