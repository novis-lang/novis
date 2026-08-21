//! The token vocabulary: what the lexer produces, one span-tagged [`Token`] at a
//! time.
//!
//! A token carries no owned text. Its [`Span`] already points at the exact bytes
//! it came from, and [`mwl_diagnostics::SourceFile::span_text`] recovers them
//! whenever a later stage needs to interpret a literal (parse the digits,
//! unescape a string). This keeps a token 16 bytes and `Copy`, which matters
//! because every later stage — parser lookahead, incremental reparse for the
//! LSP, snapshot tests — holds many of them at once.

use mwl_diagnostics::Span;

/// One lexical token: its kind, plus the exact source range it covers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Token {
    /// What kind of token this is.
    pub kind: TokenKind,
    /// The exact bytes this token covers, including any delimiters (a string's
    /// quotes, a tag's `<?mwl`).
    pub span: Span,
}

impl Token {
    /// Pairs a kind with its span.
    #[must_use]
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Every kind of token the lexer can produce.
///
/// Reserved words that PHP already fully reserves — `int`, `string`, `static`,
/// `self`, and so on — are their own [`Keyword`] variant, not an [`Ident`]. Three
/// spellings the spec introduces are deliberately kept *contextual* instead:
/// `spawn`, `script` and `with` (the `spawn script … with(…)` grammar,
/// [`docs/spec/00-overview.md` § 2](../../../docs/spec/00-overview.md)) and `type`
/// (the alias declaration, [ADR 0015](../../../docs/adr/0015-no-name-aliasing.md))
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
    /// `<?mwl`
    OpenTagMwl,
    /// `<?php` — accepted as a second spelling of `<?mwl`, identical parse
    /// ([`docs/spec/00-overview.md` § 1](../../../docs/spec/00-overview.md)).
    OpenTagPhp,
    /// `<?=` — short-echo, exactly `<?mwl echo`.
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
    /// ([ADR 0007 § 4](../../../docs/adr/0007-explicit-type-system.md)).
    IntLiteral,
    /// A floating-point literal, including an exponent (`1e10`, `1.5e-3`).
    FloatLiteral,

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
    /// label). The lexer records only where this token is — whether to strip
    /// that leading whitespace from the body's content lines (PHP 7.3+
    /// "flexible heredoc") is left to the stage that turns tokens into a final
    /// string value, not decided during tokenization.
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
    /// `!==`
    BangEqualsEquals,
    /// `=`
    Equals,
    /// `==`
    EqualsEquals,
    /// `===`
    EqualsEqualsEquals,
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
    /// [`code::E_VARIABLE_VARIABLE`](mwl_diagnostics::code::E_VARIABLE_VARIABLE)
    /// is the parser's job, per the pragmatic-superset exclusions.
    Dollar,
    /// `\` — namespace separator.
    Backslash,
    /// `#[` — opens an attribute.
    AttributeOpen,

    /// A character (sequence) the lexer could not classify. Recovery continues
    /// past it; [`code::E_UNEXPECTED_CHAR`](mwl_diagnostics::code::E_UNEXPECTED_CHAR)
    /// is reported alongside it.
    Unknown,
}

/// A reserved word.
///
/// Spelling is matched case-insensitively, as PHP already does for its own
/// keywords — `IF`, `If` and `if` are the same token. Every variant's `name()`
/// gives the canonical lower-case spelling.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
#[expect(
    missing_docs,
    reason = "each variant's meaning is exactly its lower-case spelling below in `from_lowercase`; a doc comment on every one would just restate the name"
)]
pub enum Keyword {
    Abstract,
    And,
    Array,
    As,
    Bool,
    Break,
    Bytes,
    Callable,
    Case,
    Catch,
    Class,
    Clone,
    Const,
    Continue,
    Declare,
    Default,
    Die,
    Do,
    Echo,
    Else,
    Elseif,
    Empty,
    Enum,
    Eval,
    Exit,
    Extends,
    Extract,
    False,
    Final,
    Finally,
    Float,
    Fn,
    For,
    Foreach,
    Function,
    Global,
    Goto,
    If,
    Implements,
    Include,
    IncludeOnce,
    InstanceOf,
    Insteadof,
    Int,
    Interface,
    Isset,
    Iterable,
    List,
    Match,
    Mixed,
    Namespace,
    Never,
    New,
    Null,
    Object,
    Or,
    Parent,
    Print,
    Private,
    Protected,
    Public,
    Readonly,
    Require,
    RequireOnce,
    Return,
    SelfKw,
    Settype,
    Static,
    String,
    Switch,
    Tainted,
    Throw,
    Trait,
    True,
    Try,
    Uint,
    Unset,
    Use,
    Var,
    Void,
    While,
    Xor,
    Yield,
}

impl Keyword {
    /// Looks up a reserved word by its (already lower-cased) spelling.
    ///
    /// Returns `None` for anything that is not one of MWL's reserved words,
    /// including the contextual spellings — the caller then lexes it as an
    /// [`Ident`](TokenKind::Ident).
    #[must_use]
    pub fn from_lowercase(s: &str) -> Option<Self> {
        Some(match s {
            "abstract" => Self::Abstract,
            "and" => Self::And,
            "array" => Self::Array,
            "as" => Self::As,
            "bool" => Self::Bool,
            "break" => Self::Break,
            "bytes" => Self::Bytes,
            "callable" => Self::Callable,
            "case" => Self::Case,
            "catch" => Self::Catch,
            "class" => Self::Class,
            "clone" => Self::Clone,
            "const" => Self::Const,
            "continue" => Self::Continue,
            "declare" => Self::Declare,
            "default" => Self::Default,
            "die" => Self::Die,
            "do" => Self::Do,
            "echo" => Self::Echo,
            "else" => Self::Else,
            "elseif" => Self::Elseif,
            "empty" => Self::Empty,
            "enum" => Self::Enum,
            "eval" => Self::Eval,
            "exit" => Self::Exit,
            "extends" => Self::Extends,
            "extract" => Self::Extract,
            "false" => Self::False,
            "final" => Self::Final,
            "finally" => Self::Finally,
            "float" => Self::Float,
            "fn" => Self::Fn,
            "for" => Self::For,
            "foreach" => Self::Foreach,
            "function" => Self::Function,
            "global" => Self::Global,
            "goto" => Self::Goto,
            "if" => Self::If,
            "implements" => Self::Implements,
            "include" => Self::Include,
            "include_once" => Self::IncludeOnce,
            "instanceof" => Self::InstanceOf,
            "insteadof" => Self::Insteadof,
            "int" => Self::Int,
            "interface" => Self::Interface,
            "isset" => Self::Isset,
            "iterable" => Self::Iterable,
            "list" => Self::List,
            "match" => Self::Match,
            "mixed" => Self::Mixed,
            "namespace" => Self::Namespace,
            "never" => Self::Never,
            "new" => Self::New,
            "null" => Self::Null,
            "object" => Self::Object,
            "or" => Self::Or,
            "parent" => Self::Parent,
            "print" => Self::Print,
            "private" => Self::Private,
            "protected" => Self::Protected,
            "public" => Self::Public,
            "readonly" => Self::Readonly,
            "require" => Self::Require,
            "require_once" => Self::RequireOnce,
            "return" => Self::Return,
            "self" => Self::SelfKw,
            "settype" => Self::Settype,
            "static" => Self::Static,
            "string" => Self::String,
            "switch" => Self::Switch,
            "tainted" => Self::Tainted,
            "throw" => Self::Throw,
            "trait" => Self::Trait,
            "true" => Self::True,
            "try" => Self::Try,
            "uint" => Self::Uint,
            "unset" => Self::Unset,
            "use" => Self::Use,
            "var" => Self::Var,
            "void" => Self::Void,
            "while" => Self::While,
            "xor" => Self::Xor,
            "yield" => Self::Yield,
            _ => return None,
        })
    }
}
