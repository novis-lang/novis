//! The run between two tokens on one line: nothing or one space, decided by
//! what the two tokens are (`rule:tooling/fmt-one-run-between-two-tokens`).
//!
//! `nvs_syntax::parse` hands back the trivia and no tokens, so this stage lexes
//! the file once more with [`nvs_syntax::tokenize`] and walks each pair of
//! adjacent tokens. A pair is this stage's only where its gap is on one line
//! and holds nothing but spaces and tabs: a line break is the author's
//! (`rule:tooling/fmt-never-reflows`), and a comment between two tokens leaves
//! the runs on either side of it to its author. The one comment this stage
//! does space is the one that ends a line after code.
//!
//! # A token's role
//!
//! Most tokens mean one thing wherever they are written: a `,`, a `->`, a
//! `=>`, a `&&`. A few mean two, and the tree tells them apart. The innermost
//! node holding the token's offset is the production the token belongs to, so
//! a `-` whose node is `Binary` is a subtraction, a `?` whose node is `Ternary`
//! is a ternary's, and a `(` whose node is a `Call` opens its arguments. A
//! written type is no node of its own, so this stage collects the span of every
//! type the file writes ([`nvs_syntax::written::written_types_in`]), and a `?`,
//! `|`, `&`, `<`, `>`, `(` or `)` with both of its pair inside one span is type
//! punctuation: nothing on either side of it. A unary operator's node is
//! `Unary`, `PreIncDec` or `PostIncDec`, and nothing separates it from its
//! operand.
//!
//! A keyword written where a name goes is a name: `$row->list`, `Foo::print`
//! and `function match()` space as a word does.
//!
//! # What it leaves alone
//!
//! A pair this table does not name keeps its author's run, and so does every
//! pair inside text a program prints (a string, a heredoc, an html template's
//! body) and every pair that touches a kind on [`AUTHORS`]: `<?nvs`, `<?=`,
//! `?>` and inline HTML among them. A run is never emptied where its two
//! tokens, written together, would lex as something else.
//!
//! `crates/nvs-fmt/tests/token_gaps.rs` reads every `TokenKind` the lexer has
//! and fails naming one this file mentions nowhere, so a new token kind is
//! given a row or put on [`AUTHORS`] before it is ever spaced.
//!
//! # What it spends
//!
//! One more lex of the file, one token vector and one vector of the file's
//! written types, all dropped once the runs are pushed. This runs only in
//! `nvs fmt` and the editor's formatting request.

use nvs_diagnostics::{BytePos, Diagnostics, SourceFile, SourceMap, Span};
use nvs_syntax::{Keyword, Parsed, SyntaxIndex, Token, TokenKind, Trivia, TriviaKind};

/// One space, the only run this stage writes that is not empty.
const SPACE: &str = " ";

/// Nothing at all.
const NOTHING: &str = "";

/// The control keywords whose `(` follows one space.
const CONTROL: &[Keyword] = &[
    Keyword::If,
    Keyword::Elseif,
    Keyword::While,
    Keyword::For,
    Keyword::Foreach,
    Keyword::Switch,
    Keyword::Match,
    Keyword::Catch,
];

/// The keywords a `(` follows with nothing between, where the parser reads the
/// pair as a call or as a declaration's parameters.
const CALLEE_WORDS: &[Keyword] = &[
    Keyword::Fn,
    Keyword::Isset,
    Keyword::Empty,
    Keyword::Unset,
    Keyword::Exit,
    Keyword::Die,
    Keyword::Static,
    Keyword::SelfKw,
    Keyword::Parent,
    Keyword::Class,
];

/// The keywords a `(` follows with one space, as a value follows them.
const VALUE_WORDS: &[Keyword] = &[
    Keyword::Return,
    Keyword::Echo,
    Keyword::Print,
    Keyword::Throw,
    Keyword::Yield,
    Keyword::Clone,
    Keyword::Function,
    Keyword::Require,
    Keyword::RequireOnce,
    Keyword::Include,
    Keyword::IncludeOnce,
];

/// The keywords that join two operands, as a binary operator does.
const OPERATOR_WORDS: &[Keyword] = &[
    Keyword::And,
    Keyword::Or,
    Keyword::Xor,
    Keyword::Is,
    Keyword::As,
    Keyword::InstanceOf,
];

/// The node kinds whose own `(` opens arguments or parameters, spelled as
/// `crates/nvs-syntax/src/walk.rs` spells them.
const CALLS: &[&str] = &[
    "Call",
    "MethodCall",
    "StaticCall",
    "New",
    "Function",
    "Method",
    "Fn",
    "Isset",
    "Empty",
    "Unset",
    "Exit",
    "Attribute",
    "ClassDecl",
    "PropertyHook",
];

/// The node kinds whose own `:` has nothing before it and one space after it:
/// a return type, a named argument, an anonymous object's or a shape's field,
/// and an enum's backing type.
const COLONS: &[&str] = &[
    "Function",
    "Method",
    "Fn",
    "Call",
    "MethodCall",
    "StaticCall",
    "New",
    "Attribute",
    "AnonObject",
    "ShapeType",
    "EnumDecl",
];

/// The node kinds whose own `:` ends a label: nothing before it, and the run
/// after it is the author's.
const LABELS: &[&str] = &["Case", "Switch"];

/// The node kind an operator token that also writes a type, a reference or a
/// sign is a binary operator in. `$a = &$b` is an `Assign`, and its `&` is a
/// reference.
const OPERATION: &str = "Binary";

/// The token kinds whose every run is the author's, whatever the other token
/// of the pair is. An open or a close tag and inline HTML sit beside markup
/// the program prints. A string's text, a `$` and a `\` the table names no
/// row for, and the end of the file and a byte the lexer does not know are
/// never spaced by a guess.
const AUTHORS: &[TokenKind] = &[
    TokenKind::InlineHtml,
    TokenKind::OpenTagNvs,
    TokenKind::OpenTagPhp,
    TokenKind::OpenTagEcho,
    TokenKind::CloseTag,
    TokenKind::StringPart,
    TokenKind::Dollar,
    TokenKind::Backslash,
    TokenKind::Unknown,
    TokenKind::Eof,
];

/// The node kinds of a prefix operator, which nothing separates from the
/// operand after it.
const PREFIXES: &[&str] = &["Unary", "PreIncDec"];

/// Every run this table decides in `parsed`'s file, whose text is `text`: the
/// run that has to precede an offset, by that offset.
///
/// A run already as the table wants it is not pushed, so the printer leaves
/// it as it is.
pub(crate) fn runs(parsed: &Parsed, text: &str) -> Vec<(usize, String)> {
    let mut map = SourceMap::new();
    let id = map.add("gap.nvs", text);
    let mut scratch = Diagnostics::new();
    let tokens = nvs_syntax::tokenize(map.file(id), &mut scratch);
    let mut types: Vec<Span> =
        nvs_syntax::written::written_types_in(&parsed.stmts, 0, BytePos::MAX)
            .into_iter()
            .map(|ty| ty.span)
            .collect();
    types.sort_unstable_by_key(|span| span.start);
    let trivia = &parsed.trivia;
    let pairs = Pairs::new(&parsed.index, &tokens, types);
    let mut wanted = Vec::new();
    for at in 0..tokens.len().saturating_sub(1) {
        let (left, right) = (tokens[at], tokens[at + 1]);
        let gap = &text[left.span.end as usize..right.span.start as usize];
        if !pairs.in_code[at] || !is_one_line(gap) {
            continue;
        }
        let Some(run) = pairs.run(at) else {
            continue;
        };
        if gap == run {
            continue;
        }
        if run.is_empty() && fuses(text, left, right) {
            continue;
        }
        wanted.push((right.span.start as usize, run.to_owned()));
    }
    trailing_comments(&mut wanted, text, trivia, &tokens, &pairs.in_code);
    wanted
}

/// Whether `gap` is a run on one line: spaces and tabs, or nothing.
fn is_one_line(gap: &str) -> bool {
    gap.bytes().all(|byte| byte == b' ' || byte == b'\t')
}

/// The file's tokens, and what the tree and the token stream say about each.
struct Pairs<'a> {
    index: &'a SyntaxIndex,
    tokens: &'a [Token],
    /// Whether each token sits in code rather than in text a program prints,
    /// as the token after it reads it: the pair `at`, `at + 1` is code when
    /// this holds for `at`.
    in_code: Vec<bool>,
    /// The `;` tokens inside a `for` header, as indices into `tokens`.
    for_semicolons: Vec<usize>,
    /// The span of every type the file writes, by where it starts. None of
    /// them holds another.
    types: Vec<Span>,
}

impl<'a> Pairs<'a> {
    fn new(index: &'a SyntaxIndex, tokens: &'a [Token], types: Vec<Span>) -> Self {
        let in_code = code_after_each(tokens);
        let mut for_semicolons = Vec::new();
        for at in 0..tokens.len() {
            if is_keyword(tokens[at], Keyword::For)
                && !is_named(tokens, at)
                && tokens
                    .get(at + 1)
                    .is_some_and(|next| next.kind == TokenKind::LParen)
            {
                let mut depth = 0_usize;
                for (inner, token) in tokens.iter().enumerate().skip(at + 1) {
                    match token.kind {
                        TokenKind::LParen => depth += 1,
                        TokenKind::RParen => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        TokenKind::Semicolon if depth == 1 => for_semicolons.push(inner),
                        _ => {}
                    }
                }
            }
        }
        Self {
            index,
            tokens,
            in_code,
            for_semicolons,
            types,
        }
    }

    /// The run the pair `at`, `at + 1` takes, or [`None`] where the table
    /// names no row for it.
    fn run(&self, at: usize) -> Option<&'static str> {
        let left = self.tokens[at];
        let right = self.tokens[at + 1];
        let (l, r) = (left.kind, right.kind);
        if is_aside(l) || is_aside(r) {
            return None;
        }
        // `,` and `;`.
        if l == TokenKind::Comma {
            return match r {
                TokenKind::Comma => None,
                TokenKind::RParen | TokenKind::RBracket => Some(NOTHING),
                _ => Some(SPACE),
            };
        }
        if r == TokenKind::Comma || r == TokenKind::Semicolon {
            return Some(NOTHING);
        }
        if l == TokenKind::Semicolon {
            if self.for_semicolons.binary_search(&at).is_err() {
                return None;
            }
            return Some(match r {
                TokenKind::RParen => NOTHING,
                _ => SPACE,
            });
        }
        // Brackets, `#[`, `->`, `?->` and `::`.
        if matches!(
            l,
            TokenKind::LParen | TokenKind::LBracket | TokenKind::AttributeOpen
        ) || matches!(r, TokenKind::RParen | TokenKind::RBracket)
        {
            return Some(NOTHING);
        }
        if is_member_access(l) || is_member_access(r) {
            return Some(NOTHING);
        }
        // A `:` inside a type is an object type's field or a `callable`'s
        // return type, and keeps its own row against a `?` after it.
        if l == TokenKind::Colon && self.in_one_type(at) {
            return Some(SPACE);
        }
        if (is_type_mark(l) || is_type_mark(r)) && self.in_one_type(at) {
            return Some(NOTHING);
        }
        if self.is_unary(at) {
            return Some(NOTHING);
        }
        if l == TokenKind::FatArrow || r == TokenKind::FatArrow {
            return Some(SPACE);
        }
        // A ternary's `?` and `:`.
        let left_ternary = self.is_ternary(at);
        let right_ternary = self.is_ternary(at + 1);
        if left_ternary && right_ternary && l == TokenKind::Question && r == TokenKind::Colon {
            return Some(NOTHING);
        }
        if left_ternary || right_ternary {
            return Some(SPACE);
        }
        // Every other `:`.
        if r == TokenKind::Colon {
            let kind = self.node_at(at + 1)?;
            return (COLONS.contains(&kind) || LABELS.contains(&kind)).then_some(NOTHING);
        }
        if l == TokenKind::Colon {
            return COLONS.contains(&self.node_at(at)?).then_some(SPACE);
        }
        if self.is_operator(at) || self.is_operator(at + 1) {
            return Some(SPACE);
        }
        if r == TokenKind::LParen {
            return self.before_paren(at);
        }
        if r == TokenKind::LBracket {
            return (self.node_at(at + 1) == Some("Index") && ends_a_value(self.tokens, at))
                .then_some(NOTHING);
        }
        self.words(at)
    }

    /// The run before a `(` that no earlier row named: a control keyword's, a
    /// callee's, or a keyword's that a value follows.
    fn before_paren(&self, at: usize) -> Option<&'static str> {
        let left = self.tokens[at];
        let word = self.keyword(at);
        if word.is_some_and(|word| CONTROL.contains(&word)) {
            return Some(SPACE);
        }
        let calls = self
            .node_at(at + 1)
            .is_some_and(|kind| CALLS.contains(&kind));
        let callee = match left.kind {
            TokenKind::Ident | TokenKind::Variable | TokenKind::RParen | TokenKind::RBracket => {
                true
            }
            TokenKind::Keyword(_) => {
                is_named(self.tokens, at) || word.is_some_and(|word| CALLEE_WORDS.contains(&word))
            }
            _ => false,
        };
        if callee {
            return calls.then_some(NOTHING);
        }
        word.is_some_and(|word| VALUE_WORDS.contains(&word))
            .then_some(SPACE)
    }

    /// The keyword and two-word rows: one space between a keyword and the
    /// word, variable, literal or `{` after it, and between a type or a name
    /// and the word after it.
    fn words(&self, at: usize) -> Option<&'static str> {
        let right = self.tokens[at + 1];
        let right_is_keyword = self.keyword(at + 1).is_some();
        if self.keyword(at).is_some() {
            let follows = matches!(
                right.kind,
                TokenKind::Ident
                    | TokenKind::Variable
                    | TokenKind::Keyword(_)
                    | TokenKind::IntLiteral
                    | TokenKind::FloatLiteral
                    | TokenKind::Duration
                    | TokenKind::SingleQuotedString
                    | TokenKind::DoubleQuoteOpen
                    | TokenKind::HeredocOpen
                    | TokenKind::NowdocOpen
                    | TokenKind::LBrace
            );
            return follows.then_some(SPACE);
        }
        let left_is_name = self.tokens[at].kind == TokenKind::Ident || is_named(self.tokens, at);
        if left_is_name && (right.kind == TokenKind::Variable || right_is_keyword) {
            return Some(SPACE);
        }
        (self.tokens[at].kind == TokenKind::Variable && right_is_keyword).then_some(SPACE)
    }

    /// The keyword token `at` is, unless it is written where a name goes.
    fn keyword(&self, at: usize) -> Option<Keyword> {
        match self.tokens[at].kind {
            TokenKind::Keyword(word) if !is_named(self.tokens, at) => Some(word),
            _ => None,
        }
    }

    /// Whether token `at` is a binary or an assignment operator.
    fn is_operator(&self, at: usize) -> bool {
        let token = self.tokens[at];
        match token.kind {
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Amp
            | TokenKind::Pipe
            | TokenKind::Lt
            | TokenKind::Gt
            | TokenKind::GtGt
            | TokenKind::Star => self.node_at(at) == Some(OPERATION),
            TokenKind::Keyword(_) => self
                .keyword(at)
                .is_some_and(|word| OPERATOR_WORDS.contains(&word)),
            kind => is_operator_only(kind),
        }
    }

    /// Whether the pair `at`, `at + 1` is a unary operator and its operand,
    /// in either order.
    fn is_unary(&self, at: usize) -> bool {
        let (l, r) = (self.tokens[at].kind, self.tokens[at + 1].kind);
        match l {
            TokenKind::Bang | TokenKind::Tilde | TokenKind::At | TokenKind::Ellipsis => {
                return true;
            }
            TokenKind::Minus | TokenKind::Plus | TokenKind::PlusPlus | TokenKind::MinusMinus
                if self
                    .node_at(at)
                    .is_some_and(|kind| PREFIXES.contains(&kind)) =>
            {
                return true;
            }
            _ => {}
        }
        matches!(r, TokenKind::PlusPlus | TokenKind::MinusMinus)
            && self.node_at(at + 1) == Some("PostIncDec")
    }

    /// Whether both tokens of the pair `at`, `at + 1` sit inside one written
    /// type.
    fn in_one_type(&self, at: usize) -> bool {
        let (left, right) = (self.tokens[at].span, self.tokens[at + 1].span);
        let before = self.types.partition_point(|ty| ty.start <= left.start);
        before
            .checked_sub(1)
            .is_some_and(|last| right.end <= self.types[last].end)
    }

    /// Whether token `at` is a `?` or a `:` that a ternary owns.
    fn is_ternary(&self, at: usize) -> bool {
        matches!(self.tokens[at].kind, TokenKind::Question | TokenKind::Colon)
            && self.node_at(at) == Some("Ternary")
    }

    /// The kind of the innermost node holding token `at`'s first byte.
    fn node_at(&self, at: usize) -> Option<&'static str> {
        let offset = BytePos::try_from(self.tokens[at].span.start as usize).ok()?;
        self.index.at(offset).innermost().map(|node| node.kind)
    }
}

/// Whether a pair touching `kind` is left to its author, whatever the other
/// token is: whether `kind` is on [`AUTHORS`].
fn is_aside(kind: TokenKind) -> bool {
    AUTHORS.contains(&kind)
}

/// Whether `kind` is punctuation a written type uses: `?`, `|`, `&`, `<`,
/// `>`, `>>` closing two argument lists, `(` and `)`.
fn is_type_mark(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Question
            | TokenKind::Pipe
            | TokenKind::Amp
            | TokenKind::Lt
            | TokenKind::Gt
            | TokenKind::GtGt
            | TokenKind::LParen
            | TokenKind::RParen
    )
}

/// Whether `kind` is `->`, `?->` or `::`.
fn is_member_access(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Arrow | TokenKind::NullsafeArrow | TokenKind::DoubleColon
    )
}

/// Whether `kind` is an operator that is never anything but a binary or an
/// assignment operator.
fn is_operator_only(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Equals
            | TokenKind::PlusEquals
            | TokenKind::MinusEquals
            | TokenKind::StarEquals
            | TokenKind::StarStarEquals
            | TokenKind::SlashEquals
            | TokenKind::PercentEquals
            | TokenKind::DotEquals
            | TokenKind::AmpEquals
            | TokenKind::PipeEquals
            | TokenKind::CaretEquals
            | TokenKind::LtLtEquals
            | TokenKind::GtGtEquals
            | TokenKind::QuestionQuestionEquals
            | TokenKind::QuestionQuestionPlusEquals
            | TokenKind::QuestionQuestionMinusEquals
            | TokenKind::QuestionQuestionDotEquals
            | TokenKind::StarStar
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Dot
            | TokenKind::Caret
            | TokenKind::LtLt
            | TokenKind::AmpAmp
            | TokenKind::PipePipe
            | TokenKind::QuestionQuestion
            | TokenKind::PipeGreater
            | TokenKind::EqualsEquals
            | TokenKind::BangEquals
            | TokenKind::LtEquals
            | TokenKind::GtEquals
            | TokenKind::Spaceship
    )
}

/// Whether `token` is the keyword `word`.
fn is_keyword(token: Token, word: Keyword) -> bool {
    token.kind == TokenKind::Keyword(word)
}

/// Whether the keyword at `at` is written where a name goes: after `->`,
/// `?->`, `::`, `\` or `function`.
fn is_named(tokens: &[Token], at: usize) -> bool {
    matches!(tokens[at].kind, TokenKind::Keyword(_))
        && at > 0
        && matches!(
            tokens[at - 1].kind,
            TokenKind::Arrow
                | TokenKind::NullsafeArrow
                | TokenKind::DoubleColon
                | TokenKind::Backslash
                | TokenKind::Keyword(Keyword::Function)
        )
}

/// Whether token `at` can end the value an index `[` reads.
fn ends_a_value(tokens: &[Token], at: usize) -> bool {
    matches!(
        tokens[at].kind,
        TokenKind::Variable
            | TokenKind::Ident
            | TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::RBrace
            | TokenKind::SingleQuotedString
            | TokenKind::DoubleQuoteClose
    ) || is_named(tokens, at)
}

/// For each token, whether what follows it is code.
///
/// A string, a heredoc and an html template open text, and an interpolation
/// or a template's echo opens code inside it again, so a stack of the two is
/// the whole state.
fn code_after_each(tokens: &[Token]) -> Vec<bool> {
    let mut stack = vec![true];
    tokens
        .iter()
        .map(|token| {
            match token.kind {
                TokenKind::DoubleQuoteOpen
                | TokenKind::HeredocOpen
                | TokenKind::NowdocOpen
                | TokenKind::HtmlTemplateOpen => stack.push(false),
                TokenKind::ComplexInterpOpen | TokenKind::HtmlTemplateEchoOpen => stack.push(true),
                TokenKind::DoubleQuoteClose
                | TokenKind::HeredocClose
                | TokenKind::HtmlTemplateClose
                | TokenKind::ComplexInterpClose
                | TokenKind::HtmlTemplateEchoClose
                    if stack.len() > 1 =>
                {
                    stack.pop();
                }
                _ => {}
            }
            stack.last().copied().unwrap_or(true)
        })
        .collect()
}

/// Whether `left` and `right`, written with nothing between them, lex as
/// anything but the same two tokens.
///
/// `- -$x` keeps its space because `--` is a decrement, and `> >` closing two
/// type arguments keeps its space because `>>` is a shift.
fn fuses(text: &str, left: Token, right: Token) -> bool {
    let joined = format!(
        "<?nvs {}{}",
        &text[left.span.start as usize..left.span.end as usize],
        &text[right.span.start as usize..right.span.end as usize],
    );
    let mut map = SourceMap::new();
    let id = map.add("joined.nvs", joined);
    let file: &SourceFile = map.file(id);
    let mut scratch = Diagnostics::new();
    let kinds: Vec<TokenKind> = nvs_syntax::tokenize(file, &mut scratch)
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| !matches!(kind, TokenKind::OpenTagNvs | TokenKind::Eof))
        .collect();
    kinds != [left.kind, right.kind]
}

/// Records the one space between code and a comment that ends its line.
///
/// A `//` or `#` comment always ends its line. A block comment does when
/// nothing but whitespace follows it there; one with code after it is inside
/// the line, and the runs on either side of it are its author's.
fn trailing_comments(
    wanted: &mut Vec<(usize, String)>,
    text: &str,
    trivia: &[Trivia],
    tokens: &[Token],
    in_code: &[bool],
) {
    for trivium in trivia {
        let start = trivium.span.start as usize;
        let end = trivium.span.end as usize;
        let ends_line = match trivium.kind {
            TriviaKind::LineComment => true,
            TriviaKind::BlockComment | TriviaKind::DocComment => {
                let rest = &text[end..];
                let line = rest.find('\n').map_or(rest, |at| &rest[..at]);
                line.trim().is_empty()
            }
            TriviaKind::Whitespace => false,
        };
        if !ends_line {
            continue;
        }
        let before = tokens.partition_point(|token| (token.span.end as usize) <= start);
        let Some(at) = before.checked_sub(1) else {
            continue;
        };
        let token = tokens[at];
        let gap = &text[token.span.end as usize..start];
        if !in_code[at] || is_aside(token.kind) || !is_one_line(gap) || gap == SPACE {
            continue;
        }
        wanted.push((start, SPACE.to_owned()));
    }
}
