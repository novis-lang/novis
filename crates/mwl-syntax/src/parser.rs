//! The recursive-descent parser: types and expressions so far.
//!
//! # What is here so far
//!
//! The full type grammar (ADR 0007 § 3, including nested `array<T>` and DNF
//! unions/intersections) and the full expression grammar M1's plan names:
//! every operator at its PHP precedence, `as` conversion (binding tighter
//! than any binary operator per ADR 0007 § 2), `match`, closures and arrow
//! functions, generators (`yield`/`yield from`), named arguments, spread,
//! nullsafe, first-class callable syntax, and `spawn script … with(…)`
//! ([`docs/spec/00-overview.md` § 2](../../../docs/spec/00-overview.md)).
//! [`Parser::parse_block`]/[`Parser::parse_statement`] exist only so a closure
//! literal has a body to parse — see [`crate::ast`]'s module docs for what is
//! deliberately not here yet.
//!
//! Attributes (`#[...]`) are not consumed here: every place PHP allows one is
//! a declaration site (a class, a method, a parameter), so attribute parsing
//! arrives with the declarations chunk rather than being bolted onto
//! expressions now.
//!
//! # Error recovery
//!
//! Every `parse_*` method always returns *something* — never a `Result`, so a
//! caller never has to decide whether to keep going. A missing token is
//! [`code::E_EXPECTED_TOKEN`] reported at the empty span where it should have
//! been, without consuming whatever actually follows; a missing expression is
//! [`code::E_EXPECTED_EXPR`] and an [`ExprKind::Error`] node. The one place
//! this is not enough on its own is [`Parser::parse_block`]'s statement loop,
//! which additionally forces a token of progress if a statement consumed none
//! at all, so a malformed body cannot hang the parser.

use std::collections::VecDeque;

use mwl_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};

use crate::ast::{
    Arg, ArrayItem, ArrowFnExpr, AssignOp, BinaryOp, Block, CallArgs, CastType, ClosureExpr,
    ClosureUse, Expr, ExprKind, IncDecOp, MatchArm, MemberName, Modifier, Name, NewTarget, Param,
    SpawnOption, SpawnOptionKey, Stmt, StmtKind, StringPart, Type, TypeAtom, TypeKind, UnaryOp,
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

    // ========================================================================
    // Types (ADR 0007 § 3)
    // ========================================================================

    /// Whether the current token could begin a type expression — used to
    /// decide, without committing, whether a mandatory type was actually
    /// omitted (e.g. a parameter written without one).
    fn can_start_type(&mut self) -> bool {
        matches!(
            self.peek().kind,
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
        if !self.at(TokenKind::Amp) {
            return first;
        }
        let mut items = vec![first];
        while self.eat(TokenKind::Amp).is_some() {
            items.push(self.parse_type_operand());
        }
        let span = items[0].span.to(items[items.len() - 1].span);
        Type {
            kind: TypeKind::Intersection(items),
            span,
        }
    }

    fn parse_type_operand(&mut self) -> Type {
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
        self.parse_low_or()
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

    fn parse_low_or(&mut self) -> Expr {
        self.parse_left_assoc(
            Self::parse_low_xor,
            &[(TokenKind::Keyword(Keyword::Or), BinaryOp::LowOr)],
        )
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

    fn parse_assignment(&mut self) -> Expr {
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
        let value = self.parse_assignment();
        let span = target.span.to(value.span);
        Expr {
            span,
            kind: ExprKind::Assign {
                op,
                target: Box::new(target),
                value: Box::new(value),
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

    fn parse_coalesce(&mut self) -> Expr {
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
    /// `instanceof`, which binds tighter.
    fn parse_not(&mut self) -> Expr {
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

    fn parse_unary(&mut self) -> Expr {
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

    fn parse_postfix(&mut self) -> Expr {
        let mut e = self.parse_primary();
        loop {
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
                TokenKind::Keyword(Keyword::As) => {
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
                _ => break,
            }
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
                Expr {
                    span: start,
                    kind: ExprKind::Variable(start),
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
        let mut modifiers = Vec::new();
        loop {
            let m = match self.peek().kind {
                TokenKind::Keyword(Keyword::Public) => Modifier::Public,
                TokenKind::Keyword(Keyword::Protected) => Modifier::Protected,
                TokenKind::Keyword(Keyword::Private) => Modifier::Private,
                TokenKind::Keyword(Keyword::Readonly) => Modifier::Readonly,
                _ => break,
            };
            self.bump();
            modifiers.push(m);
        }
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
    // Minimal statements — just enough for a closure/arrow-function body
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

    fn parse_statement(&mut self) -> Stmt {
        let start = self.peek().span;
        match self.peek().kind {
            TokenKind::LBrace => {
                let block = self.parse_block();
                Stmt {
                    span: block.span,
                    kind: StmtKind::Block(block),
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
            _ => {
                let expr = self.parse_expr();
                self.expect(TokenKind::Semicolon, "`;`");
                let span = start.to(self.last_span);
                Stmt {
                    span,
                    kind: StmtKind::Expr(expr),
                }
            }
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
}
