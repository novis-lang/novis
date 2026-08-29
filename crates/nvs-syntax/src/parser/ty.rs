//! The type grammar (ADR 0007 § 3) — and the qualified name every other layer
//! parses through.
//!
//! The whole of it: nested `array<T>`, DNF unions and intersections, the
//! `?T` sugar, `tainted`/`secret` qualifiers (ADR 0024 § 1, ADR 0033 § 1), ADR
//! 0036 § 3's inline `{name: T}` shape, ADR 0047's literal and enum-case
//! atoms, and `decimal` (ADR 0054). A `>>` closing two nested generics is
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
    /// Whether the current token could begin a type expression — used to
    /// decide, without committing, whether a mandatory type was actually
    /// omitted (e.g. a parameter written without one).
    pub(super) fn can_start_type(&mut self) -> bool {
        Self::token_starts_type(self.peek().kind) || self.at_negative_int_literal()
    }

    /// `-1` — ADR 0047 § 1's one type atom that needs two tokens to
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
                // ADR 0047 § 1's two literal atoms. A statement that merely
                // *starts* with one (`1 + 2;`, `"x" . $y;`) is no longer a
                // free ride to the expression path, but it still gets there:
                // `parse_stmt_maybe_local_decl` trial-parses the type and
                // backtracks the moment no `$name` follows.
                | TokenKind::IntLiteral
                | TokenKind::SingleQuotedString
                | TokenKind::DoubleQuoteOpen
                // Not a type — but ADR 0047 § 7's diagnostic is worth more
                // than the "expected a type" this would otherwise get.
                | TokenKind::FloatLiteral
        )
    }

    /// Whether `static` at the current position starts the rejected
    /// function-scope storage declaration (ADR 0008 § 5) rather than an
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
    /// ADR 0107 § 3 retires that marker but keeps it *recognizable*, so this
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
    /// this is the one place that ambiguity has to be resolved (ADR 0007 § 3).
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
    /// arity: [ADR 0053](../../../docs/adr/0053-iteration-and-generators.md)
    /// § 2's one narrow door is `Iterable`/`Iterator`, but which names are
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
                    TypeKind::Atom(
                        TypeAtom::SecretString
                        | TypeAtom::SecretBytes
                        | TypeAtom::SecretTaintedString
                        | TypeAtom::SecretTaintedBytes,
                    ) => {
                        // ADR 0033 § 1: `secret` and `tainted` compose, but
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
                                 (ADR 0033 § 1)",
                            ),
                        );
                        return Type {
                            kind: inner.kind,
                            span,
                        };
                    }
                    _ => {
                        self.diags.report(
                            Diagnostic::error(
                                code::E_TAINTED_NON_SCALAR,
                                "`tainted` only qualifies `string`/`bytes`",
                            )
                            .with_primary(span, "not a scalar `tainted` can qualify")
                            .with_help("write `tainted string` or `tainted bytes` (ADR 0024 § 1)"),
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
                            .with_help("write `secret string` or `secret bytes` (ADR 0033 § 1)"),
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
                // ADR 0047 §§ 2-3: `Foo::BAR` in type position. Which of the
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
            // ADR 0047 § 1's literal atoms, the generalisation of the `true`
            // and `false` atoms just above from `bool`'s two values to every
            // `string` and `int`.
            TokenKind::SingleQuotedString => {
                self.bump();
                Type {
                    kind: TypeKind::Atom(TypeAtom::StringLiteral(start)),
                    span: start,
                }
            }
            TokenKind::DoubleQuoteOpen => self.parse_string_literal_type(),
            TokenKind::IntLiteral => {
                self.bump();
                Type {
                    kind: TypeKind::Atom(TypeAtom::IntLiteral(start)),
                    span: start,
                }
            }
            TokenKind::Minus if self.at_negative_int_literal() => {
                self.bump();
                let lit = self.bump().span;
                let span = start.to(lit);
                Type {
                    kind: TypeKind::Atom(TypeAtom::IntLiteral(span)),
                    span,
                }
            }
            TokenKind::FloatLiteral => {
                self.bump();
                self.reject_float_literal_type(start)
            }
            TokenKind::Minus if matches!(self.peek_at(1).kind, TokenKind::FloatLiteral) => {
                self.bump();
                let lit = self.bump().span;
                self.reject_float_literal_type(start.to(lit))
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

    /// A double-quoted `"a"` in type position — ADR 0047 § 1's string literal
    /// atom, spelled the way the ADR spells it. The body is read with the
    /// ordinary [`Self::parse_string_body`] so escapes lex identically to a
    /// value position's, and an interpolated one is refused: a type has no
    /// scope to interpolate a variable from. The refused case still yields
    /// the atom, so the rest of the declaration parses on.
    pub(super) fn parse_string_literal_type(&mut self) -> Type {
        let open = self.bump().span; // DoubleQuoteOpen
        let (parts, close) = self.parse_string_body(TokenKind::DoubleQuoteClose);
        let span = open.to(close);
        if !parts.iter().all(|p| matches!(p, StringPart::Text(_))) {
            self.diags.report(
                Diagnostic::error(
                    code::E_INTERPOLATION_IN_TYPE,
                    "a string literal type cannot interpolate",
                )
                .with_primary(span, "this type names one exact string")
                .with_help(
                    "write the string out — a type is resolved at compile time, so there is \
                     nothing to interpolate from (ADR 0047 § 1)",
                ),
            );
        }
        Type {
            kind: TypeKind::Atom(TypeAtom::StringLiteral(span)),
            span,
        }
    }

    /// ADR 0047 § 7: there is no `float` literal type, deferred until
    /// floating-point equality has a real answer. Diagnosed by name rather
    /// than left to `error_expected("a type")`, since the reason a reader
    /// needs is "not this type, on purpose" and not "unparseable here".
    pub(super) fn reject_float_literal_type(&mut self, span: Span) -> Type {
        self.diags.report(
            Diagnostic::error(
                code::E_FLOAT_LITERAL_TYPE,
                "a `float` literal is not a type",
            )
            .with_primary(span, "only `string` and `int` literals name a type")
            .with_help(
                "use `float` and guard the value, or name the accepted set with `int` \
                 literals (ADR 0047 § 7)",
            ),
        );
        Type {
            kind: TypeKind::Atom(TypeAtom::Float),
            span,
        }
    }

    /// `{name: T, ...}` in type position — ADR 0036 § 3, Novis's one
    /// structurally-checked type. No ambiguity to resolve here the way the
    /// value literal has (see [`Self::parse_object_literal_expr`]): type
    /// position never dispatches `{` to a block, so an empty `{}` is simply
    /// an empty shape rather than needing the literal's "at least one field"
    /// rule.
    pub(super) fn parse_shape_type(&mut self) -> Type {
        let start = self.bump().span; // '{'
        let mut fields = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            let field_start = self.peek().span;
            let name = self.expect(TokenKind::Ident, "a field name");
            self.expect(TokenKind::Colon, "`:`");
            let ty = self.parse_type();
            let span = field_start.to(ty.span);
            fields.push(ShapeField { name, ty, span });
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

    /// A possibly-namespace-qualified name: `Foo`, `Core\Bytes`,
    /// `App\Models\User`.
    ///
    /// A **leading** separator is refused here rather than folded into the
    /// span, per
    /// [ADR 0113](../../../docs/adr/0113-a-qualified-name-is-absolute.md) § 3:
    /// a name with a separator in it is already read from the root, so the
    /// prefix has no work left to do. The token is still consumed after the
    /// report, so the rest of the name parses and one mistake yields one
    /// diagnostic — the same recovery [`Self::parse_use_decl`] gives a
    /// rename and a group import.
    ///
    /// A segment may be a reserved word's spelling: a `camelCase` segment
    /// such as `list` lexes as a keyword, and `Foo\list` still has to parse
    /// per [`docs/spec/00-overview.md` § 5](../../../docs/spec/00-overview.md).
    /// `Core\Bytes` no longer needs this tolerance —
    /// [ADR 0062](../../../docs/adr/0062-case-sensitivity-is-a-compiler-property.md)
    /// § 2 made keyword matching exact, so `Bytes` is an ordinary
    /// [`TokenKind::Ident`] instead of colliding with the `bytes` type
    /// keyword.
    /// Once a name is expected at all (this is only ever called after an
    /// `Ident` or `Backslash` was already seen), a keyword spelling here is
    /// unambiguous — the same reasoning already applied to a member name
    /// after `->`/`::` in [`Self::parse_member_name`].
    pub(super) fn parse_name(&mut self) -> Name {
        let start = self.peek().span;
        if let Some(slash) = self.eat(TokenKind::Backslash) {
            self.diags.report(
                Diagnostic::error(
                    code::E_LEADING_BACKSLASH_UNSUPPORTED,
                    "a name is already absolute",
                )
                .with_primary(slash, "remove the leading `\\`")
                .with_help(
                    "a name with a `\\` in it is read from the root, and one without resolves \
                     through the imports then the enclosing namespace (ADR 0113 §§ 1, 3)",
                ),
            );
        }
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
