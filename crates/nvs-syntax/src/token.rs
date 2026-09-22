//! The token vocabulary: what the lexer produces, one span-tagged [`Token`] at a
//! time, and the [`Trivia`] it skips between them.
//!
//! A token carries no owned text. Its [`Span`] already points at the exact bytes
//! it came from, and [`nvs_diagnostics::SourceFile::span_text`] recovers them
//! whenever a later stage needs to interpret a literal (parse the digits,
//! unescape a string). This keeps a token 16 bytes and `Copy`, which matters
//! because every later stage — parser lookahead, incremental reparse for the
//! LSP, snapshot tests — holds many of them at once.

use nvs_diagnostics::Span;

/// One lexical token: its kind, plus the exact source range it covers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Token {
    /// What kind of token this is.
    pub kind: TokenKind,
    /// The exact bytes this token covers, including any delimiters (a string's
    /// quotes, a tag's `<?nvs`).
    pub span: Span,
}

impl Token {
    /// Pairs a kind with its span.
    #[must_use]
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// One run of source text the grammar never sees: whitespace, or a comment.
///
/// A trivium owns no text either, for the same reason a [`Token`] does not —
/// the [`Span`] is the text. Retaining these is what lets the stream be put
/// back together: every token and every trivium, concatenated in offset order,
/// are the file (`rule:ide/tokens-plus-trivia-reproduce-the-file`). A
/// [`Lexer`](crate::Lexer) collects the three ignorable kinds only when it was
/// built to ([`Lexer::with_trivia`](crate::Lexer::with_trivia)), so a compile
/// path pays nothing for a layer only a formatter and an editor read; a
/// [`TriviaKind::DocComment`] it keeps either way, because the grammar attaches
/// one to the declaration below it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Trivia {
    /// What kind of run this is.
    pub kind: TriviaKind,
    /// The exact bytes it covers, a comment's own delimiters included.
    pub span: Span,
}

impl Trivia {
    /// Pairs a kind with its span.
    #[must_use]
    pub const fn new(kind: TriviaKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// The four kinds of run a [`Trivia`] covers.
///
/// Deliberately not `#[non_exhaustive]`, unlike [`TokenKind`]:
/// `rule:ide/one-grammar-one-tree` closes this set at four, so a fifth kind
/// should fail to compile everywhere one is read rather than fall into a `_`
/// arm that quietly treats it as an ordinary comment.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TriviaKind {
    /// A run of whitespace, however long and across however many lines.
    Whitespace,
    /// A comment nothing but a formatter reads: `//`, `#`, or a run of four or
    /// more slashes.
    LineComment,
    /// `/* … */`, including an unterminated one — that is a diagnostic, and it
    /// is still trivia covering the bytes it ran over.
    BlockComment,
    /// Exactly three slashes: documentation
    /// (`rule:tooling/doc-comment-is-three-slashes`), and the one variant
    /// anything past the lexer reads.
    DocComment,
}

/// Every kind of token the lexer can produce.
///
/// Reserved words that PHP already fully reserves — `int`, `string`, `static`,
/// `self`, and so on — are their own [`Keyword`] variant, not an [`Ident`]. Three
/// spellings the spec introduces are deliberately kept *contextual* instead:
/// `spawn`, `script` and `with` (the `spawn script … with(…)` grammar,
/// [`docs/spec/00-overview.md` § 2](/docs/spec/00-overview.md)) and `type`
/// (the alias declaration, `rule:statements/nothing-gets-a-second-name`)
/// lex as plain [`Ident`]s; the parser recognises them by text only at the one
/// grammar position each is meaningful in. Reserving common English words
/// globally when only a handful of ADR-introduced constructs need them would
/// cost every existing PHP program that happens to use `type` or `with` as a
/// method or variable name, for no benefit the grammar requires.
///
/// [`Ident`]: TokenKind::Ident
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum TokenKind {
    /// End of input. The lexer yields this forever once reached, so a parser
    /// never has to special-case "ran off the end of the token stream."
    Eof,

    /// A run of literal HTML/text outside any `<?…` tag, emitted verbatim as
    /// output — PHP's inline-HTML mode.
    InlineHtml,
    /// `<?nvs`
    OpenTagNvs,
    /// `<?php` — lexed like `<?nvs` so the parser can diagnose it by name
    /// rather than misreading it as inline HTML; rejected at parse time,
    /// `<?nvs` is the only code-mode open tag Novis keeps (`rule:statements/nvs-is-the-only-open-tag`).
    OpenTagPhp,
    /// `<?=` — short-echo, exactly `<?nvs echo`.
    OpenTagEcho,
    /// `?>`, leaving code mode.
    CloseTag,

    /// A bare identifier that is not a reserved keyword.
    Ident,
    /// `$name` — the span includes the leading `$`.
    Variable,
    /// A reserved word.
    Keyword(Keyword),

    /// An integer literal: decimal, `0x`/`0X` hex, `0o`/`0O` octal, `0b`/`0B`
    /// binary, with PHP's `_` digit separators. The lexer does not evaluate it
    /// or decide `int` versus `uint` — that is a checker question
    /// ([ADR 0007 § 4](/docs/decisions/0007.md)).
    IntLiteral,
    /// A floating-point literal, including an exponent (`1e10`, `1.5e-3`).
    FloatLiteral,
    /// A duration literal — `30s`, `1h30m`, `500ms`
    /// (`rule:types/duration-literal`).
    ///
    /// **One token, maximal munch**: `1h30m` is this, not three tokens. The
    /// lexer has already checked the whole grammar
    /// ([`crate::duration`](../duration/index.html)) and reported anything
    /// wrong, so a token of this kind always cooks to a nanosecond count.
    DurationLiteral,

    /// `'…'`, a single-quoted string. Recognises only PHP's two escapes
    /// (`\\` and `\'`); everything else is literal, so the whole token is
    /// unescaped in one pass whenever its value is needed.
    SingleQuotedString,

    /// Opens a double-quoted string: the `"` itself.
    DoubleQuoteOpen,
    /// Closes a double-quoted string: the `"` itself.
    DoubleQuoteClose,
    /// Opens a heredoc: `<<<LABEL` or `<<<"LABEL"` (interpolated body) up to and
    /// including the newline that starts the body.
    HeredocOpen,
    /// Opens a nowdoc: `<<<'LABEL'` up to and including the newline that starts
    /// the body. A nowdoc's body is a single [`StringPart`](Self::StringPart)
    /// with no interpolation, exactly like a single-quoted string's content.
    NowdocOpen,
    /// The closing label line of a heredoc/nowdoc (leading whitespace plus the
    /// label). The lexer records only where this token is — stripping that
    /// leading whitespace from the body's content lines (PHP 7.3+ "flexible
    /// heredoc") happens later, once the parser has assembled the whole
    /// literal's span: see `nvs_types::string_lit::heredoc_shape`/
    /// `dedent_heredoc_run`, not tokenization.
    HeredocClose,
    /// A run of literal text inside a double-quoted string, heredoc or nowdoc,
    /// between its delimiters and/or interpolation sites. May contain escape
    /// sequences (double-quoted/heredoc) or be entirely literal (nowdoc).
    StringPart,
    /// Opens PHP's "complex syntax" interpolation, `{$`. A single expression
    /// follows, lexed as ordinary code tokens, then
    /// [`ComplexInterpClose`](Self::ComplexInterpClose). The legacy `${name}`
    /// form is deliberately not supported — it is PHP-deprecated since 8.2, and
    /// `{$name}` already covers the same case.
    ComplexInterpOpen,
    /// Closes a `{$ … }` interpolation: the matching `}`.
    ComplexInterpClose,

    /// Opens a markup literal: the prefix and the delimiter together,
    /// `` html` `` (`rule:core-classes/html-literal`). Between this and
    /// [`MarkupClose`](Self::MarkupClose) the lexer is in a double-quoted
    /// string's body mode with the delimiter swapped — the same
    /// [`StringPart`](Self::StringPart) runs and the same
    /// [`ComplexInterpOpen`](Self::ComplexInterpOpen) holes — so it scans for
    /// nothing but those and the closing backtick, and learns no HTML.
    MarkupOpen,
    /// Closes a markup literal: the `` ` `` itself. A backtick in the body is
    /// written `` \` ``, which the body scanner takes as an escape like any
    /// other and leaves for the cooking pass.
    MarkupClose,
    /// Opens the second kind of hole a markup literal has, `<?=` — the output
    /// tag a page already uses, so a literal is a piece of a page in a value.
    /// One expression follows, lexed as ordinary code tokens, then
    /// [`MarkupEchoClose`](Self::MarkupEchoClose). Unlike
    /// [`ComplexInterpOpen`](Self::ComplexInterpOpen) the expression may begin
    /// with anything, which is what puts a constant or a static call in a page
    /// without a local (`rule:core-classes/html-literal`).
    MarkupEchoOpen,
    /// Closes a `<?= … ?>` hole: the `?>`. Inside the hole `}` is a brace like
    /// any other, and nothing but this token ends it.
    MarkupEchoClose,

    // --- punctuation and operators ------------------------------------------
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `,`
    Comma,
    /// `;`
    Semicolon,
    /// `:`
    Colon,
    /// `::`
    DoubleColon,
    /// `->`
    Arrow,
    /// `?->`
    NullsafeArrow,
    /// `=>`
    FatArrow,
    /// `?`
    Question,
    /// `??`
    QuestionQuestion,
    /// `??=`
    QuestionQuestionEquals,
    /// `.`
    Dot,
    /// `.=`
    DotEquals,
    /// `...` — spread/variadic.
    Ellipsis,
    /// `+`
    Plus,
    /// `+=`
    PlusEquals,
    /// `++`
    PlusPlus,
    /// `-`
    Minus,
    /// `-=`
    MinusEquals,
    /// `--`
    MinusMinus,
    /// `*`
    Star,
    /// `*=`
    StarEquals,
    /// `**`
    StarStar,
    /// `**=`
    StarStarEquals,
    /// `/`
    Slash,
    /// `/=`
    SlashEquals,
    /// `%`
    Percent,
    /// `%=`
    PercentEquals,
    /// `&`
    Amp,
    /// `&=`
    AmpEquals,
    /// `&&`
    AmpAmp,
    /// `|`
    Pipe,
    /// `|=`
    PipeEquals,
    /// `||`
    PipePipe,
    /// `|>` — the pipeline operator
    /// (`rule:expressions/pipeline-substitution`). One token rather than
    /// `Pipe` followed by `Gt`, so that the parser never has to decide whether
    /// two adjacent tokens were written adjacently.
    PipeGreater,
    /// `^`
    Caret,
    /// `^=`
    CaretEquals,
    /// `~`
    Tilde,
    /// `!`
    Bang,
    /// `!=` / `<>`
    BangEquals,
    /// `=`
    Equals,
    /// `==`
    EqualsEquals,
    /// `<`
    Lt,
    /// `<=`
    LtEquals,
    /// `<<`
    LtLt,
    /// `<<=`
    LtLtEquals,
    /// `<=>`
    Spaceship,
    /// `>`
    Gt,
    /// `>=`
    GtEquals,
    /// `>>`
    GtGt,
    /// `>>=`
    GtGtEquals,
    /// `@`
    At,
    /// A bare `$` not immediately followed by an identifier character — the
    /// only shape left once [`Variable`](Self::Variable) claims `$name`. Reached
    /// by `$$name` and `${expr}` ("variable variables"), which the lexer still
    /// tokenizes; rejecting them with
    /// [`code::E_VARIABLE_VARIABLE`](nvs_diagnostics::code::E_VARIABLE_VARIABLE)
    /// is the parser's job, per the pragmatic-superset exclusions.
    Dollar,
    /// `\` — namespace separator.
    Backslash,
    /// `#[` — opens an attribute.
    AttributeOpen,

    /// A character (sequence) the lexer could not classify. Recovery continues
    /// past it; [`code::E_UNEXPECTED_CHAR`](nvs_diagnostics::code::E_UNEXPECTED_CHAR)
    /// is reported alongside it.
    Unknown,
}

/// Declares the reserved-word table once, and derives from it everything that
/// reads it: the [`Keyword`] enum, the lexer's spelling lookup, the spelling a
/// variant prints back, and the list of every word there is.
///
/// One row per word is what makes those four agree. A variant with no
/// spelling does not compile, a spelling with no variant does not compile, and
/// anything enumerating the words — the parser's own coverage over
/// `rule:core-api/identifier-casing`'s name segments — walks a list that grows
/// with the table rather than a copy of it that does not.
macro_rules! keywords {
    ($($variant:ident => $spelling:literal,)*) => {
        /// A reserved word.
        ///
        /// Spelling is matched **exactly**, in lower case only
        /// (`rule:classes/reserved-spellings-are-lower-case`) — unlike PHP, which matches its own keywords case-insensitively.
        /// `IF` and `If` are therefore ordinary [`TokenKind::Ident`]s, not this
        /// token; nothing diagnoses them, because `rule:core-api/identifier-casing` makes both legal class
        /// names. Every variant's [`name()`](Keyword::name) gives that one spelling.
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        #[non_exhaustive]
        #[expect(
            missing_docs,
            reason = "each variant's meaning is exactly its lower-case spelling in the table below; a doc comment on every one would just restate the name"
        )]
        pub enum Keyword {
            $($variant,)*
        }

        impl Keyword {
            /// Every reserved word, in the order the table declares them.
            pub const ALL: &'static [Self] = &[$(Self::$variant,)*];

            /// Looks up a reserved word by its (already lower-cased) spelling.
            ///
            /// Returns `None` for anything that is not one of Novis's reserved
            /// words, including the contextual spellings — the caller then
            /// lexes it as an [`Ident`](TokenKind::Ident).
            #[must_use]
            pub fn from_lowercase(s: &str) -> Option<Self> {
                Some(match s {
                    $($spelling => Self::$variant,)*
                    _ => return None,
                })
            }

            /// This word's one spelling, exactly as source must write it.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $spelling,)*
                }
            }
        }
    };
}

keywords! {
    Abstract => "abstract",
    And => "and",
    Array => "array",
    As => "as",
    Autoload => "autoload",
    Bool => "bool",
    Break => "break",
    Bytes => "bytes",
    Callable => "callable",
    Case => "case",
    Catch => "catch",
    Class => "class",
    Clone => "clone",
    Const => "const",
    Continue => "continue",
    Decimal => "decimal",
    Declare => "declare",
    Default => "default",
    Die => "die",
    Do => "do",
    Echo => "echo",
    Else => "else",
    Elseif => "elseif",
    Empty => "empty",
    Enum => "enum",
    Eval => "eval",
    Exit => "exit",
    Extends => "extends",
    Extract => "extract",
    False => "false",
    Final => "final",
    Finally => "finally",
    Float => "float",
    Fn => "fn",
    For => "for",
    Foreach => "foreach",
    Function => "function",
    Global => "global",
    Goto => "goto",
    If => "if",
    Implements => "implements",
    Include => "include",
    IncludeOnce => "include_once",
    Inout => "inout",
    InstanceOf => "instanceof",
    Insteadof => "insteadof",
    Int => "int",
    Interface => "interface",
    Is => "is",
    Isset => "isset",
    Iterable => "iterable",
    Lateinit => "lateinit",
    Let => "let",
    List => "list",
    Match => "match",
    Mixed => "mixed",
    Namespace => "namespace",
    Never => "never",
    New => "new",
    Null => "null",
    Object => "object",
    Or => "or",
    Parent => "parent",
    Print => "print",
    Private => "private",
    Protected => "protected",
    Public => "public",
    Readonly => "readonly",
    Require => "require",
    RequireOnce => "require_once",
    Return => "return",
    Secret => "secret",
    SelfKw => "self",
    Settype => "settype",
    Static => "static",
    String => "string",
    Switch => "switch",
    Tainted => "tainted",
    Throw => "throw",
    Trait => "trait",
    True => "true",
    Try => "try",
    Uint => "uint",
    Unset => "unset",
    Use => "use",
    Var => "var",
    Void => "void",
    While => "while",
    Xor => "xor",
    Yield => "yield",
}
