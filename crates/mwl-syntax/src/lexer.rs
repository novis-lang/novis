//! The dual-mode lexer: HTML/inline-text on one side, MWL code on the other,
//! with a small stack of nested modes for double-quoted strings, heredocs and
//! their interpolation sites.
//!
//! # Mode model
//!
//! `modes[0]` is always the *outer* state — [`Mode::Html`] or [`Mode::Code`]
//! with `interpolation: false` — and a `<?mwl`/`<?php`/`<?=` tag or a `?>`
//! toggles it **in place**, never by pushing: there is exactly one outer state
//! at a time, so `modes.len() == 1` is the precise condition for "an unclosed
//! tag is legal to run to end of file"
//! ([`docs/spec/00-overview.md` § 1](../../../docs/spec/00-overview.md)).
//! Everything nested inside that outer state — a double-quoted string, a
//! heredoc/nowdoc, a `{$…}` interpolation site — is a genuine push/pop frame,
//! and reaching end of input with any of those still open is
//! [`code::E_UNTERMINATED`].
//!
//! # What the lexer does not do
//!
//! It does not evaluate literals (an integer's value, a string's unescaped
//! text, a heredoc's indentation strip) and it does not preserve comments or
//! whitespace as trivia. Both are later-stage concerns: numeric/string
//! "cooking" happens once a value is actually needed, and trivia-preserving
//! reparse for `mwl fmt`/`mwl lsp` is M10's job, not M1's — the lexer's
//! contract is a token stream whose spans are exactly right, nothing more.

use std::collections::VecDeque;

use mwl_diagnostics::{BytePos, Diagnostic, Diagnostics, SourceFile, Span, code};

use crate::token::{Keyword, Token, TokenKind};

/// One entry in the lexer's mode stack. See the module docs for how `modes[0]`
/// differs from everything above it.
#[derive(Debug, Clone)]
enum Mode {
    /// Inline HTML/text, emitted verbatim as output.
    Html,
    /// Ordinary code. `interpolation` is true only for a frame opened by a
    /// `{$` inside a string — that is what makes `}` sometimes close the
    /// frame instead of lexing as [`TokenKind::RBrace`], and what makes `?>`
    /// meaningless here (§ *What closes this frame* below).
    Code {
        interpolation: bool,
        /// Nested `{`/`}` seen since this frame opened, for the interpolation
        /// case: the frame closes on a `}` seen at depth 0, not on the first
        /// `}` at all — a closure literal inside `{$…}` has its own braces.
        brace_depth: u32,
    },
    /// Inside `"…"`, between [`TokenKind::DoubleQuoteOpen`] and
    /// [`TokenKind::DoubleQuoteClose`].
    DoubleQuoted { start: BytePos },
    /// Inside a heredoc/nowdoc body, between its open and close delimiters.
    /// `interpolation` is false for a nowdoc (`<<<'LABEL'`).
    Heredoc {
        label: String,
        interpolation: bool,
        start: BytePos,
    },
}

/// Produces one [`Token`] at a time from a source file.
///
/// Reports into a caller-supplied [`Diagnostics`] sink and keeps going after
/// an error — a malformed heredoc header, an unterminated string — rather
/// than aborting, so one lexer run surfaces every lexical problem in a file.
///
/// `Clone` so the parser can checkpoint and restore a lexer position wholesale
/// when a statement's grammar is genuinely ambiguous on a token prefix alone —
/// see [`crate::parser::Parser::checkpoint`].
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    file: &'a SourceFile,
    text: &'a str,
    pos: BytePos,
    modes: Vec<Mode>,
    pending: VecDeque<Token>,
}

impl<'a> Lexer<'a> {
    /// Starts lexing `file` from its first byte, in HTML mode.
    #[must_use]
    pub fn new(file: &'a SourceFile) -> Self {
        Self {
            file,
            text: file.text(),
            pos: 0,
            modes: vec![Mode::Html],
            pending: VecDeque::new(),
        }
    }

    /// Produces the next token, reporting any lexical errors along the way.
    ///
    /// Yields [`TokenKind::Eof`] forever once the input is exhausted and every
    /// nested mode has been unwound, so a caller never needs to special-case
    /// "ran off the end."
    pub fn next_token(&mut self, diags: &mut Diagnostics) -> Token {
        while self.pending.is_empty() {
            self.produce(diags);
        }
        self.pending.pop_front().expect("just checked non-empty")
    }

    fn produce(&mut self, diags: &mut Diagnostics) {
        if self.eof() {
            self.handle_eof(diags);
            return;
        }
        match self.modes.last() {
            Some(Mode::Html) => self.lex_html(diags),
            Some(Mode::Code { .. }) => self.lex_code(diags),
            Some(Mode::DoubleQuoted { .. } | Mode::Heredoc { .. }) => self.lex_quoted_body(diags),
            None => unreachable!("mode stack is never empty"),
        }
    }

    fn handle_eof(&mut self, diags: &mut Diagnostics) {
        if self.modes.len() == 1 {
            // The single outer state, open or not, is always legal to run to
            // EOF -- see the module docs.
            self.push(TokenKind::Eof, Span::at(self.file.id(), self.pos));
            return;
        }
        let mode = self.modes.pop().expect("checked len() > 1 above");
        let (what, start, close_kind) = match mode {
            Mode::DoubleQuoted { start } => (
                "unterminated string literal",
                start,
                TokenKind::DoubleQuoteClose,
            ),
            Mode::Heredoc { start, .. } => (
                "unterminated heredoc/nowdoc",
                start,
                TokenKind::HeredocClose,
            ),
            Mode::Code { .. } => (
                "unterminated `{$…}` interpolation",
                self.pos,
                TokenKind::ComplexInterpClose,
            ),
            Mode::Html => unreachable!("HTML mode is only ever modes[0]"),
        };
        diags.report(
            Diagnostic::error(code::E_UNTERMINATED, what)
                .with_primary(self.mk_span(start, self.pos), "runs to end of file"),
        );
        self.push(close_kind, Span::at(self.file.id(), self.pos));
    }

    fn push(&mut self, kind: TokenKind, span: Span) {
        self.pending.push_back(Token::new(kind, span));
    }

    // --- cursor -------------------------------------------------------------

    fn eof(&self) -> bool {
        self.pos as usize >= self.text.len()
    }

    fn rest(&self) -> &'a str {
        &self.text[self.pos as usize..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += u32::try_from(c.len_utf8()).expect("a char is at most 4 UTF-8 bytes");
        Some(c)
    }

    fn starts_with(&self, s: &str) -> bool {
        self.rest().starts_with(s)
    }

    fn at_line_start(&self) -> bool {
        self.pos == 0 || self.text.as_bytes()[self.pos as usize - 1] == b'\n'
    }

    fn mk_span(&self, start: BytePos, end: BytePos) -> Span {
        Span::new(self.file.id(), start, end)
    }

    fn is_ident_start(c: char) -> bool {
        c == '_' || c.is_ascii_alphabetic()
    }

    fn is_ident_continue(c: char) -> bool {
        c == '_' || c.is_ascii_alphanumeric()
    }

    // --- HTML mode ------------------------------------------------------------

    fn lex_html(&mut self, _diags: &mut Diagnostics) {
        let start = self.pos;
        loop {
            if self.eof() {
                break;
            }
            if self.peek() == Some('<')
                && let Some((kind, len)) = self.match_open_tag()
            {
                if self.pos > start {
                    self.push(TokenKind::InlineHtml, self.mk_span(start, self.pos));
                }
                let tag_start = self.pos;
                self.pos += u32::try_from(len).expect("tag length is at most 5 bytes");
                self.push(kind, self.mk_span(tag_start, self.pos));
                *self.modes.last_mut().expect("mode stack never empty") = Mode::Code {
                    interpolation: false,
                    brace_depth: 0,
                };
                return;
            }
            self.bump();
        }
        if self.pos > start {
            self.push(TokenKind::InlineHtml, self.mk_span(start, self.pos));
        }
    }

    /// Checks (without consuming) whether one of the three open-tag spellings
    /// begins at the current position. `<?php`/`<?mwl` must be followed by
    /// whitespace, `?` or end of input, so `<?phpx` is not mistaken for a tag.
    fn match_open_tag(&self) -> Option<(TokenKind, usize)> {
        for (spelling, kind) in [
            ("<?php", TokenKind::OpenTagPhp),
            ("<?mwl", TokenKind::OpenTagMwl),
        ] {
            if let Some(head) = self.rest().get(..spelling.len())
                && head.eq_ignore_ascii_case(spelling)
            {
                let after = self
                    .rest()
                    .get(spelling.len()..)
                    .and_then(|s| s.chars().next());
                if after.is_none_or(|c| c.is_whitespace() || c == '?') {
                    return Some((kind, spelling.len()));
                }
            }
        }
        if self.starts_with("<?=") {
            return Some((TokenKind::OpenTagEcho, 3));
        }
        None
    }

    // --- code mode ------------------------------------------------------------

    fn skip_trivia(&mut self, diags: &mut Diagnostics) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while !self.eof() && self.peek() != Some('\n') {
                        self.bump();
                    }
                }
                // `#[` opens an attribute, not a comment.
                Some('#') if self.peek_at(1) == Some('[') => break,
                Some('#') => {
                    while !self.eof() && self.peek() != Some('\n') {
                        self.bump();
                    }
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    let start = self.pos;
                    self.bump();
                    self.bump();
                    let mut closed = false;
                    while !self.eof() {
                        if self.peek() == Some('*') && self.peek_at(1) == Some('/') {
                            self.bump();
                            self.bump();
                            closed = true;
                            break;
                        }
                        self.bump();
                    }
                    if !closed {
                        diags.report(
                            Diagnostic::error(code::E_UNTERMINATED, "unterminated block comment")
                                .with_primary(self.mk_span(start, self.pos), "runs to end of file"),
                        );
                    }
                }
                _ => break,
            }
        }
    }

    fn lex_code(&mut self, diags: &mut Diagnostics) {
        self.skip_trivia(diags);
        if self.eof() {
            return;
        }

        let interp_frame = matches!(
            self.modes.last(),
            Some(Mode::Code {
                interpolation: true,
                ..
            })
        );

        if !interp_frame && self.starts_with("?>") {
            let start = self.pos;
            self.pos += 2;
            self.push(TokenKind::CloseTag, self.mk_span(start, self.pos));
            *self.modes.last_mut().expect("mode stack never empty") = Mode::Html;
            // One immediately following newline is swallowed, so a template
            // line ending in `?>` does not emit a blank line (spec § 1).
            if self.starts_with("\r\n") {
                self.pos += 2;
            } else if self.peek() == Some('\n') {
                self.bump();
            }
            return;
        }

        if self.starts_with("#[") {
            let start = self.pos;
            self.pos += 2;
            self.push(TokenKind::AttributeOpen, self.mk_span(start, self.pos));
            return;
        }

        if self.starts_with("<<<") {
            self.lex_heredoc_open(diags);
            return;
        }

        match self.peek() {
            Some('$') => {
                self.lex_dollar();
                return;
            }
            Some(c) if c.is_ascii_digit() => {
                self.lex_number();
                return;
            }
            Some('.') if self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) => {
                self.lex_number();
                return;
            }
            Some('\'') => {
                self.lex_single_quoted(diags);
                return;
            }
            Some('"') => {
                let start = self.pos;
                self.bump();
                self.push(TokenKind::DoubleQuoteOpen, self.mk_span(start, self.pos));
                self.modes.push(Mode::DoubleQuoted { start });
                return;
            }
            Some(c) if Self::is_ident_start(c) => {
                self.lex_ident_or_keyword();
                return;
            }
            _ => {}
        }

        self.lex_operator(diags);
    }

    fn lex_dollar(&mut self) {
        if let Some(span) = self.try_lex_variable() {
            self.push(TokenKind::Variable, span);
        } else {
            let start = self.pos;
            self.bump(); // the '$' itself
            self.push(TokenKind::Dollar, self.mk_span(start, self.pos));
        }
    }

    /// Consumes a `$name` at the current position and returns its span.
    /// Consumes nothing and returns `None` if `$` is not followed immediately
    /// by an identifier-start character (`$$x`, `${...}`).
    fn try_lex_variable(&mut self) -> Option<Span> {
        debug_assert_eq!(self.peek(), Some('$'));
        if !self.peek_at(1).is_some_and(Self::is_ident_start) {
            return None;
        }
        let start = self.pos;
        self.bump(); // '$'
        while self.peek().is_some_and(Self::is_ident_continue) {
            self.bump();
        }
        Some(self.mk_span(start, self.pos))
    }

    fn lex_ident_or_keyword(&mut self) {
        let start = self.pos;
        while self.peek().is_some_and(Self::is_ident_continue) {
            self.bump();
        }
        let span = self.mk_span(start, self.pos);
        let text = &self.text[start as usize..self.pos as usize];
        let lower = text.to_ascii_lowercase();
        let kind = Keyword::from_lowercase(&lower).map_or(TokenKind::Ident, TokenKind::Keyword);
        self.push(kind, span);
    }

    fn lex_number(&mut self) {
        let start = self.pos;
        if self.peek() == Some('0') {
            match self.peek_at(1) {
                Some('x' | 'X') => {
                    self.bump();
                    self.bump();
                    self.consume_digit_run(|c| c.is_ascii_hexdigit());
                    self.push(TokenKind::IntLiteral, self.mk_span(start, self.pos));
                    return;
                }
                Some('o' | 'O') => {
                    self.bump();
                    self.bump();
                    self.consume_digit_run(|c| matches!(c, '0'..='7'));
                    self.push(TokenKind::IntLiteral, self.mk_span(start, self.pos));
                    return;
                }
                Some('b' | 'B') => {
                    self.bump();
                    self.bump();
                    self.consume_digit_run(|c| c == '0' || c == '1');
                    self.push(TokenKind::IntLiteral, self.mk_span(start, self.pos));
                    return;
                }
                _ => {}
            }
        }

        self.consume_digit_run(|c| c.is_ascii_digit());
        let mut is_float = false;

        // A trailing `.` always starts a float, with zero or more digits
        // after it -- PHP's own DNUM rule, including the "1." edge case.
        if self.peek() == Some('.') {
            is_float = true;
            self.bump();
            self.consume_digit_run(|c| c.is_ascii_digit());
        }

        if matches!(self.peek(), Some('e' | 'E')) {
            let sign_len: usize = if matches!(self.peek_at(1), Some('+' | '-')) {
                2
            } else {
                1
            };
            if self.peek_at(sign_len).is_some_and(|c| c.is_ascii_digit()) {
                is_float = true;
                self.bump();
                if matches!(self.peek(), Some('+' | '-')) {
                    self.bump();
                }
                self.consume_digit_run(|c| c.is_ascii_digit());
            }
        }

        let kind = if is_float {
            TokenKind::FloatLiteral
        } else {
            TokenKind::IntLiteral
        };
        self.push(kind, self.mk_span(start, self.pos));
    }

    fn consume_digit_run(&mut self, is_digit: impl Fn(char) -> bool) {
        while let Some(c) = self.peek() {
            if is_digit(c) || c == '_' {
                self.bump();
            } else {
                break;
            }
        }
    }

    fn lex_single_quoted(&mut self, diags: &mut Diagnostics) {
        let start = self.pos;
        self.bump(); // opening '
        loop {
            match self.peek() {
                None => {
                    diags.report(
                        Diagnostic::error(code::E_UNTERMINATED, "unterminated string literal")
                            .with_primary(self.mk_span(start, self.pos), "runs to end of file"),
                    );
                    break;
                }
                Some('\\') => {
                    self.bump();
                    // Only `\\` and `\'` are real escapes; any other
                    // backslash is literal, exactly as PHP's single-quoted
                    // strings already work.
                    if matches!(self.peek(), Some('\\' | '\'')) {
                        self.bump();
                    }
                }
                Some('\'') => {
                    self.bump();
                    break;
                }
                Some(_) => {
                    self.bump();
                }
            }
        }
        self.push(TokenKind::SingleQuotedString, self.mk_span(start, self.pos));
    }

    fn lex_operator(&mut self, diags: &mut Diagnostics) {
        let start = self.pos;
        let c = self.peek().expect("lex_code already checked eof");

        macro_rules! op {
            ($len:expr, $kind:expr) => {{
                self.pos += $len;
                self.push($kind, self.mk_span(start, self.pos));
                return;
            }};
        }

        match c {
            '(' => op!(1, TokenKind::LParen),
            ')' => op!(1, TokenKind::RParen),
            '[' => op!(1, TokenKind::LBracket),
            ']' => op!(1, TokenKind::RBracket),
            ',' => op!(1, TokenKind::Comma),
            ';' => op!(1, TokenKind::Semicolon),
            '@' => op!(1, TokenKind::At),
            '\\' => op!(1, TokenKind::Backslash),
            '~' => op!(1, TokenKind::Tilde),
            ':' => {
                if self.starts_with("::") {
                    op!(2, TokenKind::DoubleColon)
                }
                op!(1, TokenKind::Colon)
            }
            '?' => {
                if self.starts_with("?->") {
                    op!(3, TokenKind::NullsafeArrow)
                }
                if self.starts_with("??=") {
                    op!(3, TokenKind::QuestionQuestionEquals)
                }
                if self.starts_with("??") {
                    op!(2, TokenKind::QuestionQuestion)
                }
                op!(1, TokenKind::Question)
            }
            '.' => {
                if self.starts_with("...") {
                    op!(3, TokenKind::Ellipsis)
                }
                if self.starts_with(".=") {
                    op!(2, TokenKind::DotEquals)
                }
                op!(1, TokenKind::Dot)
            }
            '+' => {
                if self.starts_with("++") {
                    op!(2, TokenKind::PlusPlus)
                }
                if self.starts_with("+=") {
                    op!(2, TokenKind::PlusEquals)
                }
                op!(1, TokenKind::Plus)
            }
            '-' => {
                if self.starts_with("->") {
                    op!(2, TokenKind::Arrow)
                }
                if self.starts_with("--") {
                    op!(2, TokenKind::MinusMinus)
                }
                if self.starts_with("-=") {
                    op!(2, TokenKind::MinusEquals)
                }
                op!(1, TokenKind::Minus)
            }
            '*' => {
                if self.starts_with("**=") {
                    op!(3, TokenKind::StarStarEquals)
                }
                if self.starts_with("**") {
                    op!(2, TokenKind::StarStar)
                }
                if self.starts_with("*=") {
                    op!(2, TokenKind::StarEquals)
                }
                op!(1, TokenKind::Star)
            }
            '/' => {
                if self.starts_with("/=") {
                    op!(2, TokenKind::SlashEquals)
                }
                op!(1, TokenKind::Slash)
            }
            '%' => {
                if self.starts_with("%=") {
                    op!(2, TokenKind::PercentEquals)
                }
                op!(1, TokenKind::Percent)
            }
            '&' => {
                if self.starts_with("&&") {
                    op!(2, TokenKind::AmpAmp)
                }
                if self.starts_with("&=") {
                    op!(2, TokenKind::AmpEquals)
                }
                op!(1, TokenKind::Amp)
            }
            '|' => {
                if self.starts_with("||") {
                    op!(2, TokenKind::PipePipe)
                }
                if self.starts_with("|=") {
                    op!(2, TokenKind::PipeEquals)
                }
                op!(1, TokenKind::Pipe)
            }
            '^' => {
                if self.starts_with("^=") {
                    op!(2, TokenKind::CaretEquals)
                }
                op!(1, TokenKind::Caret)
            }
            '!' => {
                if self.starts_with("!==") {
                    op!(3, TokenKind::BangEqualsEquals)
                }
                if self.starts_with("!=") {
                    op!(2, TokenKind::BangEquals)
                }
                op!(1, TokenKind::Bang)
            }
            '=' => {
                if self.starts_with("===") {
                    op!(3, TokenKind::EqualsEqualsEquals)
                }
                if self.starts_with("==") {
                    op!(2, TokenKind::EqualsEquals)
                }
                if self.starts_with("=>") {
                    op!(2, TokenKind::FatArrow)
                }
                op!(1, TokenKind::Equals)
            }
            '<' => {
                if self.starts_with("<=>") {
                    op!(3, TokenKind::Spaceship)
                }
                if self.starts_with("<<=") {
                    op!(3, TokenKind::LtLtEquals)
                }
                if self.starts_with("<>") {
                    op!(2, TokenKind::BangEquals)
                }
                if self.starts_with("<<") {
                    op!(2, TokenKind::LtLt)
                }
                if self.starts_with("<=") {
                    op!(2, TokenKind::LtEquals)
                }
                op!(1, TokenKind::Lt)
            }
            '>' => {
                if self.starts_with(">>=") {
                    op!(3, TokenKind::GtGtEquals)
                }
                if self.starts_with(">>") {
                    op!(2, TokenKind::GtGt)
                }
                if self.starts_with(">=") {
                    op!(2, TokenKind::GtEquals)
                }
                op!(1, TokenKind::Gt)
            }
            '{' => {
                self.bump();
                if let Some(Mode::Code {
                    interpolation: true,
                    brace_depth,
                }) = self.modes.last_mut()
                {
                    *brace_depth += 1;
                }
                self.push(TokenKind::LBrace, self.mk_span(start, self.pos));
            }
            '}' => {
                if let Some(Mode::Code {
                    interpolation: true,
                    brace_depth,
                }) = self.modes.last_mut()
                {
                    if *brace_depth == 0 {
                        self.bump();
                        self.push(TokenKind::ComplexInterpClose, self.mk_span(start, self.pos));
                        self.modes.pop();
                        return;
                    }
                    *brace_depth -= 1;
                }
                self.bump();
                self.push(TokenKind::RBrace, self.mk_span(start, self.pos));
            }
            _ => {
                self.bump();
                diags.report(
                    Diagnostic::error(
                        code::E_UNEXPECTED_CHAR,
                        format!("unexpected character `{c}`"),
                    )
                    .with_primary(self.mk_span(start, self.pos), "not valid here"),
                );
                self.push(TokenKind::Unknown, self.mk_span(start, self.pos));
            }
        }
    }

    // --- heredoc / nowdoc header ------------------------------------------

    /// Called with the cursor at `<<<`. Consumes the header line
    /// (`<<<LABEL`, `<<<"LABEL"` or `<<<'LABEL'`, up to and including the
    /// newline that starts the body) and pushes a [`Mode::Heredoc`] frame, or
    /// reports [`code::E_BAD_HEREDOC`] and pushes a single
    /// [`TokenKind::Unknown`] if the header is malformed.
    fn lex_heredoc_open(&mut self, diags: &mut Diagnostics) {
        let start = self.pos;
        self.pos += 3; // "<<<"
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }

        let quote = match self.peek() {
            Some('\'') => Some('\''),
            Some('"') => Some('"'),
            _ => None,
        };
        if quote.is_some() {
            self.bump();
        }

        if !self.peek().is_some_and(Self::is_ident_start) {
            // "<<<" has no other meaning in the language, so this is simply
            // malformed input rather than something else to fall back to.
            self.pos = start + 3;
            diags.report(
                Diagnostic::error(
                    code::E_BAD_HEREDOC,
                    "expected a heredoc/nowdoc label after `<<<`",
                )
                .with_primary(self.mk_span(start, self.pos), "no identifier follows"),
            );
            self.push(TokenKind::Unknown, self.mk_span(start, self.pos));
            return;
        }

        let label_start = self.pos;
        while self.peek().is_some_and(Self::is_ident_continue) {
            self.bump();
        }
        let label = self.text[label_start as usize..self.pos as usize].to_string();

        if let Some(q) = quote {
            if self.peek() == Some(q) {
                self.bump();
            } else {
                diags.report(
                    Diagnostic::error(
                        code::E_BAD_HEREDOC,
                        "unterminated heredoc/nowdoc label quote",
                    )
                    .with_primary(
                        self.mk_span(start, self.pos),
                        format!("expected a closing `{q}`"),
                    ),
                );
            }
        }

        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
        match self.peek() {
            Some('\r') => {
                self.bump();
                if self.peek() == Some('\n') {
                    self.bump();
                }
            }
            Some('\n') => {
                self.bump();
            }
            _ => {
                diags.report(
                    Diagnostic::error(
                        code::E_BAD_HEREDOC,
                        "expected a newline after the heredoc/nowdoc label",
                    )
                    .with_primary(
                        self.mk_span(start, self.pos),
                        "the label must be alone on its line",
                    ),
                );
            }
        }

        let interpolation = quote != Some('\'');
        let open_kind = if interpolation {
            TokenKind::HeredocOpen
        } else {
            TokenKind::NowdocOpen
        };
        self.push(open_kind, self.mk_span(start, self.pos));
        self.modes.push(Mode::Heredoc {
            label,
            interpolation,
            start,
        });
    }

    /// Whether a heredoc/nowdoc terminator for `label` begins at the current
    /// position (leading horizontal whitespace, then the label, not
    /// immediately followed by another identifier character). Does not
    /// consume anything.
    fn heredoc_terminator_here(&self, label: &str) -> bool {
        let after_indent = self.rest().trim_start_matches([' ', '\t']);
        after_indent
            .strip_prefix(label)
            .is_some_and(|rest| !rest.chars().next().is_some_and(Self::is_ident_continue))
    }

    /// Consumes a heredoc/nowdoc terminator already confirmed by
    /// [`Self::heredoc_terminator_here`] and returns its span. Stripping its
    /// leading whitespace from the body's content lines (PHP 7.3+ "flexible
    /// heredoc") happens later, once the parser has assembled the whole
    /// literal's span -- see `mwl_types::string_lit::heredoc_shape`.
    fn consume_heredoc_terminator(&mut self, label: &str) -> Span {
        let start = self.pos;
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
        self.pos += u32::try_from(label.len()).expect("heredoc labels are short identifiers");
        self.mk_span(start, self.pos)
    }

    // --- double-quoted / heredoc / nowdoc body -----------------------------

    fn lex_quoted_body(&mut self, diags: &mut Diagnostics) {
        let (is_heredoc, interpolation, label) = match self.modes.last() {
            Some(Mode::DoubleQuoted { .. }) => (false, true, String::new()),
            Some(Mode::Heredoc {
                label,
                interpolation,
                ..
            }) => (true, *interpolation, label.clone()),
            _ => unreachable!("lex_quoted_body called outside a string/heredoc mode"),
        };

        if is_heredoc && self.heredoc_terminator_here(&label) {
            let span = self.consume_heredoc_terminator(&label);
            self.push(TokenKind::HeredocClose, span);
            self.modes.pop();
            return;
        }
        if !is_heredoc && self.peek() == Some('"') {
            let start = self.pos;
            self.bump();
            self.push(TokenKind::DoubleQuoteClose, self.mk_span(start, self.pos));
            self.modes.pop();
            return;
        }

        if interpolation {
            if self.starts_with("{$") {
                let start = self.pos;
                self.pos += 1; // consume only '{'; '$' becomes the next code token
                self.push(TokenKind::ComplexInterpOpen, self.mk_span(start, self.pos));
                self.modes.push(Mode::Code {
                    interpolation: true,
                    brace_depth: 0,
                });
                return;
            }
            if self.peek() == Some('$') && self.peek_at(1).is_some_and(Self::is_ident_start) {
                self.lex_simple_interpolation();
                return;
            }
        }

        let start = self.pos;
        loop {
            if self.eof() {
                break;
            }
            if is_heredoc && self.at_line_start() && self.heredoc_terminator_here(&label) {
                break;
            }
            if !is_heredoc && self.peek() == Some('"') {
                break;
            }
            if interpolation {
                if self.starts_with("{$") {
                    break;
                }
                if self.peek() == Some('$') && self.peek_at(1).is_some_and(Self::is_ident_start) {
                    break;
                }
                if self.peek() == Some('\\') {
                    self.lex_escape_in_place(diags);
                    continue;
                }
            }
            self.bump();
        }
        if self.pos > start {
            self.push(TokenKind::StringPart, self.mk_span(start, self.pos));
        }
    }

    /// Consumes one escape sequence starting at the current `\`. Every
    /// spelling is left literal except `\u{...}`, PHP's one escape that can
    /// actually be malformed -- see [`code::E_INVALID_ESCAPE`].
    fn lex_escape_in_place(&mut self, diags: &mut Diagnostics) {
        let start = self.pos;
        self.bump(); // '\\'
        if self.peek() == Some('u') && self.peek_at(1) == Some('{') {
            self.bump();
            self.bump();
            let hex_start = self.pos;
            while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                self.bump();
            }
            let has_digits = self.pos > hex_start;
            if has_digits && self.peek() == Some('}') {
                self.bump();
            } else {
                diags.report(
                    Diagnostic::error(
                        code::E_INVALID_ESCAPE,
                        "invalid `\\u{...}` escape: expected one or more hex digits followed by `}`",
                    )
                    .with_primary(self.mk_span(start, self.pos), "malformed unicode escape"),
                );
            }
        } else if !self.eof() {
            self.bump();
        }
    }

    /// PHP's "simple syntax" interpolation: `$name`, `$name->prop` (one level
    /// only) or `$name[offset]` (a bareword, digits, or another variable --
    /// never a quoted string or a nested expression, which both need complex
    /// syntax `{$…}` instead). Called with the cursor confirmed at a `$`
    /// followed by an identifier-start character.
    fn lex_simple_interpolation(&mut self) {
        let var_span = self
            .try_lex_variable()
            .expect("caller confirmed a variable follows");
        self.push(TokenKind::Variable, var_span);

        if self.starts_with("->") && self.peek_at(2).is_some_and(Self::is_ident_start) {
            let arrow_start = self.pos;
            self.pos += 2;
            self.push(TokenKind::Arrow, self.mk_span(arrow_start, self.pos));
            let prop_start = self.pos;
            while self.peek().is_some_and(Self::is_ident_continue) {
                self.bump();
            }
            self.push(TokenKind::Ident, self.mk_span(prop_start, self.pos));
            return;
        }

        if self.peek() == Some('[') {
            let lb_start = self.pos;
            self.bump();
            self.push(TokenKind::LBracket, self.mk_span(lb_start, self.pos));

            let off_start = self.pos;
            if self.peek() == Some('-') {
                self.bump();
            }
            if self.peek() == Some('$') {
                if let Some(span) = self.try_lex_variable() {
                    self.push(TokenKind::Variable, span);
                }
            } else {
                while self.peek().is_some_and(Self::is_ident_continue) {
                    self.bump();
                }
                if self.pos > off_start {
                    self.push(TokenKind::Ident, self.mk_span(off_start, self.pos));
                }
            }

            if self.peek() == Some(']') {
                let rb_start = self.pos;
                self.bump();
                self.push(TokenKind::RBracket, self.mk_span(rb_start, self.pos));
            }
            // A malformed offset (no digits/bareword, or a missing `]`) is
            // left for the parser to diagnose from the token shape -- simple
            // syntax does not backtrack once committed, matching PHP.
        }
    }
}

/// Tokenizes an entire file in one call, for tests and for anything that
/// wants the whole stream rather than pulling it lazily.
///
/// The returned vector always ends with exactly one [`TokenKind::Eof`].
pub fn tokenize(file: &SourceFile, diags: &mut Diagnostics) -> Vec<Token> {
    let mut lexer = Lexer::new(file);
    let mut out = Vec::new();
    loop {
        let tok = lexer.next_token(diags);
        let is_eof = tok.kind == TokenKind::Eof;
        out.push(tok);
        if is_eof {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use mwl_diagnostics::SourceMap;

    use super::*;

    /// Tokenizes `src` and returns just the kinds, plus the diagnostics sink,
    /// so a test can assert on shape without spelling out every span.
    fn kinds(src: &str) -> (Vec<TokenKind>, Diagnostics) {
        let mut map = SourceMap::new();
        let id = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let tokens = tokenize(map.file(id), &mut diags);
        (tokens.into_iter().map(|t| t.kind).collect(), diags)
    }

    fn kinds_ok(src: &str) -> Vec<TokenKind> {
        let (kinds, diags) = kinds(src);
        assert!(
            !diags.has_errors(),
            "unexpected diagnostics for {src:?}: {diags:?}"
        );
        kinds
    }

    use TokenKind::*;

    #[test]
    fn inline_html_before_a_tag_is_one_token() {
        assert_eq!(
            kinds_ok("hello <?mwl echo 1; ?>world"),
            vec![
                InlineHtml,
                OpenTagMwl,
                Keyword(super::Keyword::Echo),
                IntLiteral,
                Semicolon,
                CloseTag,
                InlineHtml,
                Eof,
            ]
        );
    }

    #[test]
    fn php_tag_is_accepted_like_mwl_tag() {
        assert_eq!(
            kinds_ok("<?php echo 1; ?>"),
            vec![
                OpenTagPhp,
                Keyword(super::Keyword::Echo),
                IntLiteral,
                Semicolon,
                CloseTag,
                Eof
            ]
        );
    }

    #[test]
    fn short_echo_tag() {
        assert_eq!(
            kinds_ok("<?= $x ?>"),
            vec![OpenTagEcho, Variable, CloseTag, Eof]
        );
    }

    #[test]
    fn php_tag_followed_by_identifier_char_is_not_a_tag() {
        // "<?phpx" is not "<?php" + "x": there is no valid tag here at all,
        // so it is just more inline HTML.
        let (kinds, _) = kinds("<?phpx");
        assert_eq!(kinds, vec![InlineHtml, Eof]);
    }

    #[test]
    fn unclosed_code_block_is_legal_at_eof() {
        let (kinds, diags) = kinds("<?mwl echo 1;");
        assert!(!diags.has_errors());
        assert_eq!(
            kinds,
            vec![
                OpenTagMwl,
                Keyword(super::Keyword::Echo),
                IntLiteral,
                Semicolon,
                Eof
            ]
        );
    }

    #[test]
    fn line_and_block_and_hash_comments_are_skipped() {
        assert_eq!(
            kinds_ok("<?mwl // a\n # b\n /* c */ echo 1;"),
            vec![
                OpenTagMwl,
                Keyword(super::Keyword::Echo),
                IntLiteral,
                Semicolon,
                Eof
            ]
        );
    }

    #[test]
    fn unterminated_block_comment_is_reported() {
        let (_, diags) = kinds("<?mwl /* never closes");
        assert!(diags.has_errors());
    }

    #[test]
    fn keywords_are_case_insensitive_but_idents_are_not() {
        assert_eq!(
            kinds_ok("<?mwl ECHO myVar"),
            vec![OpenTagMwl, Keyword(super::Keyword::Echo), Ident, Eof]
        );
    }

    #[test]
    fn new_type_atoms_are_keywords() {
        assert_eq!(
            kinds_ok("<?mwl uint bytes"),
            vec![
                OpenTagMwl,
                Keyword(super::Keyword::Uint),
                Keyword(super::Keyword::Bytes),
                Eof
            ]
        );
    }

    #[test]
    fn tainted_is_a_keyword() {
        // ADR 0024 § 1: `tainted` needs a new reserved keyword, landing in
        // M1's grammar alongside `uint`'s own addition above.
        assert_eq!(
            kinds_ok("<?mwl tainted string"),
            vec![
                OpenTagMwl,
                Keyword(super::Keyword::Tainted),
                Keyword(super::Keyword::String),
                Eof
            ]
        );
    }

    #[test]
    fn secret_is_a_keyword() {
        // ADR 0033 § 1: `secret` needs its own new reserved keyword,
        // independent of `tainted`'s.
        assert_eq!(
            kinds_ok("<?mwl secret string"),
            vec![
                OpenTagMwl,
                Keyword(super::Keyword::Secret),
                Keyword(super::Keyword::String),
                Eof
            ]
        );
    }

    #[test]
    fn lateinit_is_a_keyword() {
        // ADR 0038 § 1: `lateinit` is a new reserved property modifier,
        // alongside `readonly`'s own keyword.
        assert_eq!(
            kinds_ok("<?mwl lateinit"),
            vec![OpenTagMwl, Keyword(super::Keyword::Lateinit), Eof]
        );
    }

    #[test]
    fn spawn_script_with_are_contextual_not_reserved() {
        // These must lex as plain identifiers -- the grammar recognises them
        // by text only at the one position each is meaningful, per
        // docs/spec/00-overview.md § 2 and token.rs's module docs.
        assert_eq!(
            kinds_ok("<?mwl spawn script with type"),
            vec![OpenTagMwl, Ident, Ident, Ident, Ident, Eof]
        );
    }

    #[test]
    fn integer_literal_bases() {
        assert_eq!(
            kinds_ok("<?mwl 0x1F 0o17 0b101 0755 1_000_000"),
            vec![
                OpenTagMwl, IntLiteral, IntLiteral, IntLiteral, IntLiteral, IntLiteral, Eof
            ]
        );
    }

    #[test]
    fn float_literal_shapes() {
        assert_eq!(
            kinds_ok("<?mwl 1.5 .5 1. 1e10 1.5e-3"),
            vec![
                OpenTagMwl,
                FloatLiteral,
                FloatLiteral,
                FloatLiteral,
                FloatLiteral,
                FloatLiteral,
                Eof
            ]
        );
    }

    #[test]
    fn single_quoted_string_only_escapes_backslash_and_quote() {
        let (kinds, diags) = kinds(r"<?mwl 'a\'b\\c\nd'");
        assert!(!diags.has_errors());
        assert_eq!(kinds, vec![OpenTagMwl, SingleQuotedString, Eof]);
    }

    #[test]
    fn unterminated_single_quoted_string_is_reported() {
        let (kinds, diags) = kinds("<?mwl 'abc");
        assert!(diags.has_errors());
        assert_eq!(kinds, vec![OpenTagMwl, SingleQuotedString, Eof]);
    }

    #[test]
    fn double_quoted_string_with_no_interpolation() {
        assert_eq!(
            kinds_ok(r#"<?mwl "plain text""#),
            vec![
                OpenTagMwl,
                DoubleQuoteOpen,
                StringPart,
                DoubleQuoteClose,
                Eof
            ]
        );
    }

    #[test]
    fn simple_variable_interpolation() {
        assert_eq!(
            kinds_ok(r#"<?mwl "a $name b""#),
            vec![
                OpenTagMwl,
                DoubleQuoteOpen,
                StringPart,
                Variable,
                StringPart,
                DoubleQuoteClose,
                Eof,
            ]
        );
    }

    #[test]
    fn simple_property_interpolation_is_one_level_only() {
        assert_eq!(
            kinds_ok(r#"<?mwl "$obj->prop->more""#),
            vec![
                OpenTagMwl,
                DoubleQuoteOpen,
                Variable,
                Arrow,
                Ident,
                StringPart, // "->more" is literal: simple syntax is one level
                DoubleQuoteClose,
                Eof,
            ]
        );
    }

    #[test]
    fn simple_array_offset_interpolation() {
        assert_eq!(
            kinds_ok(r#"<?mwl "$arr[key] $arr[-1] $arr[$i]""#),
            vec![
                OpenTagMwl,
                DoubleQuoteOpen,
                Variable,
                LBracket,
                Ident,
                RBracket,
                StringPart,
                Variable,
                LBracket,
                Ident,
                RBracket,
                StringPart,
                Variable,
                LBracket,
                Variable,
                RBracket,
                DoubleQuoteClose,
                Eof,
            ]
        );
    }

    #[test]
    fn complex_interpolation_lexes_as_nested_code() {
        assert_eq!(
            kinds_ok(r#"<?mwl "sum: {$a + $b}""#),
            vec![
                OpenTagMwl,
                DoubleQuoteOpen,
                StringPart,
                ComplexInterpOpen,
                Variable,
                Plus,
                Variable,
                ComplexInterpClose,
                DoubleQuoteClose,
                Eof,
            ]
        );
    }

    #[test]
    fn complex_interpolation_tracks_nested_braces() {
        // The closure's own braces must not be mistaken for the closing `}`
        // of the interpolation site.
        assert_eq!(
            kinds_ok(r#"<?mwl "{$f(function () { return 1; })}""#),
            vec![
                OpenTagMwl,
                DoubleQuoteOpen,
                ComplexInterpOpen,
                Variable,
                LParen,
                Keyword(super::Keyword::Function),
                LParen,
                RParen,
                LBrace,
                Keyword(super::Keyword::Return),
                IntLiteral,
                Semicolon,
                RBrace,
                RParen,
                ComplexInterpClose,
                DoubleQuoteClose,
                Eof,
            ]
        );
    }

    #[test]
    fn dollar_dollar_is_two_tokens_not_a_variable() {
        assert_eq!(
            kinds_ok("<?mwl $$name"),
            vec![OpenTagMwl, Dollar, Variable, Eof]
        );
    }

    #[test]
    fn heredoc_with_interpolation() {
        let src = "<?mwl $s = <<<EOT\nhello $name\nEOT;\n";
        assert_eq!(
            kinds_ok(src),
            vec![
                OpenTagMwl,
                Variable,
                Equals,
                HeredocOpen,
                StringPart,
                Variable,
                StringPart,
                HeredocClose,
                Semicolon,
                Eof,
            ]
        );
    }

    #[test]
    fn nowdoc_has_no_interpolation() {
        let src = "<?mwl $s = <<<'EOT'\nraw $name text\nEOT;\n";
        assert_eq!(
            kinds_ok(src),
            vec![
                OpenTagMwl,
                Variable,
                Equals,
                NowdocOpen,
                StringPart,
                HeredocClose,
                Semicolon,
                Eof,
            ]
        );
    }

    #[test]
    fn heredoc_terminator_must_not_be_a_prefix_of_a_longer_identifier() {
        // "EOTX" on its own line must not be mistaken for the "EOT" terminator.
        let src = "<?mwl <<<EOT\nEOTX\nEOT;\n";
        let (kinds, diags) = kinds(src);
        assert!(!diags.has_errors());
        assert_eq!(
            kinds,
            vec![
                OpenTagMwl,
                HeredocOpen,
                StringPart,
                HeredocClose,
                Semicolon,
                Eof
            ]
        );
    }

    #[test]
    fn unterminated_heredoc_is_reported() {
        let (kinds, diags) = kinds("<?mwl <<<EOT\nhello\n");
        assert!(diags.has_errors());
        assert_eq!(
            kinds,
            vec![OpenTagMwl, HeredocOpen, StringPart, HeredocClose, Eof]
        );
    }

    #[test]
    fn operators_longest_match_wins() {
        assert_eq!(
            kinds_ok("<?mwl <=> ??= ?-> **= <<= >>= === !== <> ->"),
            vec![
                OpenTagMwl,
                Spaceship,
                QuestionQuestionEquals,
                NullsafeArrow,
                StarStarEquals,
                LtLtEquals,
                GtGtEquals,
                EqualsEqualsEquals,
                BangEqualsEquals,
                BangEquals,
                Arrow,
                Eof,
            ]
        );
    }

    #[test]
    fn attribute_open_is_distinct_from_a_comment() {
        assert_eq!(
            kinds_ok("<?mwl #[Attr] # comment\n1"),
            vec![OpenTagMwl, AttributeOpen, Ident, RBracket, IntLiteral, Eof]
        );
    }

    #[test]
    fn unexpected_character_recovers() {
        let (kinds, diags) = kinds("<?mwl 1 ` 2");
        assert!(diags.has_errors());
        assert_eq!(
            kinds,
            vec![OpenTagMwl, IntLiteral, Unknown, IntLiteral, Eof]
        );
    }

    #[test]
    fn eof_yields_forever() {
        let mut map = SourceMap::new();
        let id = map.add("t.mwl", "<?mwl 1");
        let mut diags = Diagnostics::new();
        let mut lexer = Lexer::new(map.file(id));
        loop {
            if lexer.next_token(&mut diags).kind == TokenKind::Eof {
                break;
            }
        }
        assert_eq!(lexer.next_token(&mut diags).kind, TokenKind::Eof);
        assert_eq!(lexer.next_token(&mut diags).kind, TokenKind::Eof);
    }
}
