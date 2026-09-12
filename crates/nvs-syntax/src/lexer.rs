//! The dual-mode lexer: HTML/inline-text on one side, Novis code on the other,
//! with a small stack of nested modes for double-quoted strings, heredocs and
//! their interpolation sites.
//!
//! # Mode model
//!
//! `modes[0]` is always the *outer* state — [`Mode::Html`] or [`Mode::Code`]
//! with `interpolation: false` — and a `<?nvs`/`<?=` tag (or the rejected-
//! but-still-lexed `<?php`) or a `?>`
//! toggles it **in place**, never by pushing: there is exactly one outer state
//! at a time, so `modes.len() == 1` is the precise condition for "an unclosed
//! tag is legal to run to end of file"
//! ([`docs/spec/00-overview.md` § 1](/docs/spec/00-overview.md)).
//! Everything nested inside that outer state — a double-quoted string, a
//! markup literal, a heredoc/nowdoc, a `{$…}` interpolation site — is a
//! genuine push/pop frame, and reaching end of input with any of those still
//! open is [`code::E_UNTERMINATED`].
//!
//! A markup literal, ``html`…` ``, is [`Mode::DoubleQuoted`] with the
//! delimiter swapped: the same segments, the same `{$` holes counting brace
//! depth, the same escapes. The lexer scans for the closing backtick and for
//! `{$`, and for nothing else — it learns no HTML
//! (`rule:core-classes/html-literal`).
//!
//! # What the lexer does not do
//!
//! It does not evaluate literals — an integer's value, a string's unescaped
//! text, a heredoc's indentation strip. That "cooking" happens once a value is
//! actually needed, so the lexer's contract is a token stream whose spans are
//! exactly right, nothing more.
//!
//! It does, when asked, keep what it skips: [`Lexer::with_trivia`] retains
//! every whitespace run and every comment as a [`Trivia`], which is what a
//! formatter and an editor need and what a compile path does not
//! (`rule:ide/one-grammar-one-tree`).

use std::collections::VecDeque;

use nvs_diagnostics::{BytePos, Diagnostic, Diagnostics, SourceFile, Span, code};

use crate::duration;
use crate::token::{Keyword, Token, TokenKind, Trivia, TriviaKind};

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
    /// Inside ``html`…` ``, between [`TokenKind::MarkupOpen`] and
    /// [`TokenKind::MarkupClose`]. This is [`Mode::DoubleQuoted`] with the
    /// delimiter swapped and nothing else changed: the segments, the `{$`
    /// holes and the escapes are a double-quoted string's
    /// (`rule:core-classes/html-literal`).
    Markup { start: BytePos },
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
/// see [`crate::parser::Parser::checkpoint`]. A clone carries the trivia
/// collected so far, so restoring a checkpoint rewinds the trivia along with
/// the position rather than leaving a speculative parse's comments behind it.
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    file: &'a SourceFile,
    text: &'a str,
    pos: BytePos,
    modes: Vec<Mode>,
    pending: VecDeque<Token>,
    /// True while this file opened with `#!` and no `?>` has left code mode
    /// yet — the exact window in which an `<?nvs` is [`code::E_TAG_IN_SHEBANG_FILE`]
    /// rather than a tag. See [`Self::new`].
    shebang_open: bool,
    /// True when this lexer keeps the runs *nothing but a formatter reads* —
    /// whitespace, and a comment that is not documentation. Off for every
    /// compile path, on for [`Self::with_trivia`]'s callers. A
    /// [`TriviaKind::DocComment`] is kept either way; see [`Self::push_trivia`].
    collect_trivia: bool,
    /// What has been skipped so far, in source order. Holds only the doc
    /// comments unless `collect_trivia` is set.
    trivia: Vec<Trivia>,
}

impl<'a> Lexer<'a> {
    /// Starts lexing `file` from its first byte, in HTML mode — or in code
    /// mode when the file's first two bytes are `#!`
    /// (`rule:tooling/shebang-opens-code-mode`
    /// ).
    ///
    /// **The shebang line is not skipped and is not a token: it is lexed as
    /// the `#` comment it already is.** Starting the outer mode as
    /// [`Mode::Code`] with `pos` still at 0 is the whole implementation — code
    /// mode's own trivia rule then consumes line 1 to its `\n`, emits nothing,
    /// and bidi-checks it exactly as it checks any other `#` comment
    /// (`rule:security/bidi-boundaries`
    /// ). Skipping the bytes before lexing would have been shorter and
    /// would have opened a hole at the one place in a file where an unbalanced
    /// override reorders everything after it.
    ///
    /// The trigger is exactly the bytes `#!` at offset 0: no lookahead, no
    /// BOM tolerance, and `#!` anywhere else is ordinary text or an ordinary
    /// comment.
    #[must_use]
    pub fn new(file: &'a SourceFile) -> Self {
        let text = file.text();
        let shebang_open = text.starts_with("#!");
        Self {
            file,
            text,
            pos: 0,
            modes: vec![if shebang_open {
                Mode::Code {
                    interpolation: false,
                    brace_depth: 0,
                }
            } else {
                Mode::Html
            }],
            pending: VecDeque::new(),
            shebang_open,
            collect_trivia: false,
            trivia: Vec::new(),
        }
    }

    /// Starts lexing `file` exactly as [`Self::new`] does, and keeps every run
    /// it would otherwise only advance past, as a [`Trivia`]
    /// (`rule:ide/one-grammar-one-tree`).
    ///
    /// The token stream is identical either way — collecting is a side channel,
    /// not a mode — so a caller that wants both reads tokens as usual and asks
    /// for [`Self::take_trivia`] at the end.
    #[must_use]
    pub fn with_trivia(file: &'a SourceFile) -> Self {
        Self {
            collect_trivia: true,
            ..Self::new(file)
        }
    }

    /// Every trivium skipped so far, in source order. A lexer that did not come
    /// from [`Self::with_trivia`] holds only the [`TriviaKind::DocComment`]s.
    #[must_use]
    pub fn trivia(&self) -> &[Trivia] {
        &self.trivia
    }

    /// Takes what has been collected, leaving this lexer's own vector empty and
    /// still collecting — for a caller that owns the result rather than reading
    /// it in place.
    pub fn take_trivia(&mut self) -> Vec<Trivia> {
        std::mem::take(&mut self.trivia)
    }

    /// Puts a taken vector back, for [`crate::parser::Parser`]'s checkpoint,
    /// which moves the trivia aside so cloning a lexer does not copy it. The
    /// vector must be this lexer's own: any other would leave the trivia out of
    /// source order, which is the one thing a consumer relies on.
    pub(crate) fn put_trivia(&mut self, trivia: Vec<Trivia>) {
        self.trivia = trivia;
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
            Some(Mode::DoubleQuoted { .. } | Mode::Markup { .. } | Mode::Heredoc { .. }) => {
                self.lex_quoted_body(diags)
            }
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
            Mode::Markup { start } => {
                ("unterminated markup literal", start, TokenKind::MarkupClose)
            }
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

    /// Records the run from `start` to the current position as one trivium.
    /// Called once per run and after it has been consumed, so the span is
    /// exactly the bytes skipped.
    ///
    /// A [`TriviaKind::DocComment`] is recorded whether or not this lexer is
    /// collecting, and the flag governs only the three ignorable kinds. A `///`
    /// is content the language itself reads — it attaches to the declaration
    /// below it and an unattached one is a diagnostic
    /// (`rule:tooling/doc-comment-attaches-to-the-next-declaration`) — so the
    /// compile path has to see one, while it still pays nothing for the
    /// whitespace and ordinary comments only a formatter needs.
    fn push_trivia(&mut self, kind: TriviaKind, start: BytePos) {
        if self.collect_trivia || kind == TriviaKind::DocComment {
            self.trivia
                .push(Trivia::new(kind, self.mk_span(start, self.pos)));
        }
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

    /// `rule:security/bidi-boundaries`
    /// at the lexer: a directional control that opens a scope must close it
    /// inside the span that opened it, and the span is **one line** of one
    /// token — so a heredoc, a block comment or an inline-HTML run cannot open
    /// a scope on one line and close it on the next. A failure is a hard error
    /// with no suppression, which is `rule:core-api/casing-checks-the-leading-character`'s standing position for every
    /// other spelling rule.
    ///
    /// Called on the four spans that carry free text: a comment, a
    /// single-quoted literal, one text piece of a double-quoted/heredoc body,
    /// and an inline-HTML run. Identifiers need nothing — they are ASCII-only
    /// by construction, which is what closes the homoglyph half of Trojan
    /// Source without a lint.
    fn check_bidi(&self, span: Span, diags: &mut Diagnostics) {
        let text = &self.text[span.start as usize..span.end as usize];
        let mut line_start = span.start;
        for line in text.split_inclusive('\n') {
            if let Some((offset, control)) = nvs_render::bidi::first_unterminated(line) {
                let at = line_start + u32::try_from(offset).expect("offset within one token");
                let end = at + u32::try_from(control.len_utf8()).expect("a control is 3 bytes");
                diags.report(
                    Diagnostic::error(
                        code::E_UNBALANCED_BIDI,
                        format!(
                            "unterminated bidirectional control U+{:04X} {}",
                            control as u32,
                            nvs_render::bidi::control_name(control)
                        ),
                    )
                    .with_primary(self.mk_span(at, end), "opens a directional scope")
                    .with_note(format!(
                        "the scope is still open where this line ends; close it with {}",
                        nvs_render::bidi::terminator_of(control)
                    )),
                );
            }
            line_start += u32::try_from(line.len()).expect("a line is shorter than its file");
        }
    }

    fn is_ident_start(c: char) -> bool {
        c == '_' || c.is_ascii_alphabetic()
    }

    fn is_ident_continue(c: char) -> bool {
        c == '_' || c.is_ascii_alphanumeric()
    }

    // --- HTML mode ------------------------------------------------------------

    fn lex_html(&mut self, diags: &mut Diagnostics) {
        let start = self.pos;
        loop {
            if self.eof() {
                break;
            }
            if self.peek() == Some('<')
                && let Some((kind, len)) = self.match_open_tag()
            {
                if self.pos > start {
                    let span = self.mk_span(start, self.pos);
                    self.check_bidi(span, diags);
                    self.push(TokenKind::InlineHtml, span);
                }
                let tag_start = self.pos;
                self.pos += u32::try_from(len).expect("tag length is at most 5 bytes");
                let tag_span = self.mk_span(tag_start, self.pos);
                // `rule:classes/reserved-spellings-are-lower-case`: `<?nvs` has exactly one spelling. `<?PHP` is
                // left alone — it is rejected outright by
                // `E_PHP_OPEN_TAG_UNSUPPORTED` (`rule:statements/nvs-is-the-only-open-tag`) whatever case
                // it was typed in, and two diagnostics for one tag would
                // point at two different fixes.
                if kind == TokenKind::OpenTagNvs
                    && &self.text[tag_start as usize..self.pos as usize] != "<?nvs"
                {
                    diags.report(
                        Diagnostic::error(
                            code::E_RESERVED_SPELLING_CASE,
                            "`<?nvs` must be written in lower case",
                        )
                        .with_primary(tag_span, "write `<?nvs`"),
                    );
                }
                self.push(kind, tag_span);
                *self.modes.last_mut().expect("mode stack never empty") = Mode::Code {
                    interpolation: false,
                    brace_depth: 0,
                };
                return;
            }
            self.bump();
        }
        if self.pos > start {
            let span = self.mk_span(start, self.pos);
            self.check_bidi(span, diags);
            self.push(TokenKind::InlineHtml, span);
        }
    }

    /// Checks (without consuming) whether one of the three open-tag spellings
    /// begins at the current position. `<?php`/`<?nvs` must be followed by
    /// whitespace, `?` or end of input, so `<?phpx` is not mistaken for a tag.
    ///
    /// Both are still *recognised* case-insensitively, the way `rule:statements/nvs-is-the-only-open-tag`
    /// already recognises `<?php` purely so the diagnostic can name the fix:
    /// a file opening `<?NVS` must keep lexing as code, or every later line
    /// collapses into one useless `InlineHtml` token. [`Self::lex_html`]
    /// reports the casing.
    fn match_open_tag(&self) -> Option<(TokenKind, usize)> {
        for (spelling, kind) in [
            ("<?php", TokenKind::OpenTagPhp),
            ("<?nvs", TokenKind::OpenTagNvs),
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

    /// Consumes whitespace and comments, recording each run as one [`Trivia`]
    /// when this lexer is collecting them.
    ///
    /// A line comment's kind is read off its opening run of slashes: exactly
    /// three is a [`TriviaKind::DocComment`] and every other length is ordinary
    /// (`rule:tooling/doc-comment-is-three-slashes`), which is Rust's own rule
    /// and is what keeps a `////` divider a divider. A `#` comment is ordinary
    /// at any length, and a run of slashes is counted before the rest of the
    /// line is consumed because that is the only part of a comment its kind
    /// depends on.
    fn skip_trivia(&mut self, diags: &mut Diagnostics) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    let start = self.pos;
                    while self.peek().is_some_and(char::is_whitespace) {
                        self.bump();
                    }
                    self.push_trivia(TriviaKind::Whitespace, start);
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    let start = self.pos;
                    let mut slashes = 0_u32;
                    while self.peek() == Some('/') {
                        slashes += 1;
                        self.bump();
                    }
                    while !self.eof() && self.peek() != Some('\n') {
                        self.bump();
                    }
                    self.check_bidi(self.mk_span(start, self.pos), diags);
                    let kind = if slashes == 3 {
                        TriviaKind::DocComment
                    } else {
                        TriviaKind::LineComment
                    };
                    self.push_trivia(kind, start);
                }
                // `#[` opens an attribute, not a comment.
                Some('#') if self.peek_at(1) == Some('[') => break,
                Some('#') => {
                    let start = self.pos;
                    while !self.eof() && self.peek() != Some('\n') {
                        self.bump();
                    }
                    self.check_bidi(self.mk_span(start, self.pos), diags);
                    self.push_trivia(TriviaKind::LineComment, start);
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
                    self.check_bidi(self.mk_span(start, self.pos), diags);
                    self.push_trivia(TriviaKind::BlockComment, start);
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
            // Past the first `?>` a shebang file is an ordinary template: the
            // text after it is output and a later `<?nvs` reopens code mode
            // (`rule:tooling/shebang-opens-code-mode`).
            self.shebang_open = false;
            // One immediately following newline is swallowed, so a template
            // line ending in `?>` does not emit a blank line (spec § 1). It is
            // the only byte outside `skip_trivia` that no token covers, so it
            // is recorded as trivia too — otherwise every `?>` at the end of a
            // line would be a hole in
            // `rule:ide/tokens-plus-trivia-reproduce-the-file`.
            let swallowed = self.pos;
            if self.starts_with("\r\n") {
                self.pos += 2;
            } else if self.peek() == Some('\n') {
                self.bump();
            }
            if self.pos != swallowed {
                self.push_trivia(TriviaKind::Whitespace, swallowed);
            }
            return;
        }

        if self.shebang_open
            && self.peek() == Some('<')
            && let Some((TokenKind::OpenTagNvs, len)) = self.match_open_tag()
        {
            // `rule:tooling/shebang-opens-code-mode`: this file is already in code mode, so the tag is
            // named rather than lexed as `<` `?` `nvs` and reported three
            // tokens later as something the author did not write. Consuming it
            // and carrying on is the recovery that matches the intent.
            let start = self.pos;
            self.pos += u32::try_from(len).expect("tag length is at most 5 bytes");
            diags.report(
                Diagnostic::error(
                    code::E_TAG_IN_SHEBANG_FILE,
                    "this file opens with `#!` and is already in code mode",
                )
                .with_primary(self.mk_span(start, self.pos), "remove the `<?nvs`"),
            );
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

        // The prefix and the delimiter are one token, so the backtick has to
        // follow `html` immediately; anywhere else a backtick is an
        // unexpected character, Novis having no shell-execution form
        // (`rule:core-classes/process-is-argv-only`).
        if self.starts_with("html`") {
            let start = self.pos;
            self.pos += 5;
            self.push(TokenKind::MarkupOpen, self.mk_span(start, self.pos));
            self.modes.push(Mode::Markup { start });
            return;
        }

        match self.peek() {
            Some('$') => {
                self.lex_dollar();
                return;
            }
            Some(c) if c.is_ascii_digit() => {
                self.lex_number(diags);
                return;
            }
            Some('.') if self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) => {
                self.lex_number(diags);
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

    /// A reserved word is matched **exactly**, in lower case only
    /// (`rule:classes/reserved-spellings-are-lower-case`). `IF` is therefore an ordinary [`TokenKind::Ident`], not a
    /// mis-cased `if`, and gets no diagnostic here: `rule:core-api/identifier-casing` makes `IF` a
    /// perfectly legal class name, so nothing lexical distinguishes the two.
    /// It also means `Core\Bytes` needs no special handling — `Bytes` is an
    /// `Ident`, where PHP-style case-insensitive matching made it collide
    /// with the `bytes` type keyword.
    fn lex_ident_or_keyword(&mut self) {
        let start = self.pos;
        while self.peek().is_some_and(Self::is_ident_continue) {
            self.bump();
        }
        let span = self.mk_span(start, self.pos);
        let text = &self.text[start as usize..self.pos as usize];
        let kind = Keyword::from_lowercase(text).map_or(TokenKind::Ident, TokenKind::Keyword);
        self.push(kind, span);
    }

    /// A numeric literal, and — `rule:types/duration-literal` — the duration literal that shares its opening digits.
    ///
    /// A duration is reached only from a **plain decimal** integer: the
    /// `0x`/`0o`/`0b` forms return before this point, so `0x1d` stays one hex
    /// literal, and a float that is followed by a unit letter is § 1's
    /// fractional refusal rather than a rounded duration.
    ///
    /// The trigger is deliberately narrow. A candidate is scanned with
    /// [`duration::is_duration_char`] — the digits plus the unit letters in
    /// **both** cases — so `30foo` is still an integer followed by an
    /// identifier and gets no duration diagnostic, while `30S` reaches the
    /// grammar and is told which ADR makes it wrong.
    fn lex_number(&mut self, diags: &mut Diagnostics) {
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

        if self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            self.lex_duration_suffix(start, is_float, diags);
            return;
        }

        let kind = if is_float {
            TokenKind::FloatLiteral
        } else {
            TokenKind::IntLiteral
        };
        self.push(kind, self.mk_span(start, self.pos));
    }

    /// The half of [`Self::lex_number`] that runs once a letter follows the
    /// digits, with `start` the number's first byte and `is_float` whether a
    /// `.` or an exponent was consumed on the way here.
    ///
    /// Leaves the position where it found it — pushing an ordinary
    /// [`TokenKind::IntLiteral`] and letting the letters lex as an identifier
    /// — for anything outside the duration alphabet, which is what keeps this
    /// production from claiming `30foo`.
    fn lex_duration_suffix(&mut self, start: BytePos, is_float: bool, diags: &mut Diagnostics) {
        let mut end = self.pos;
        while (end as usize) < self.text.len()
            && self.text[end as usize..]
                .chars()
                .next()
                .is_some_and(duration::is_duration_char)
        {
            end += 1;
        }
        let candidate = &self.text[start as usize..end as usize];
        // Anything past the alphabet means this was never a duration: leave
        // the digits as the integer they are and let the letters lex on their
        // own, exactly as before this production existed.
        let followed_by_more = self.text[end as usize..]
            .chars()
            .next()
            .is_some_and(Self::is_ident_continue);
        if followed_by_more {
            self.push(
                if is_float {
                    TokenKind::FloatLiteral
                } else {
                    TokenKind::IntLiteral
                },
                self.mk_span(start, self.pos),
            );
            return;
        }

        // A fractional count needs no rule of its own: the candidate is sliced
        // from the number's first byte, so `1.5s` reaches `duration::parse`
        // with its `.` intact and `rule:types/duration-literal`'s refusal is the grammar's.
        let span = self.mk_span(start, end);
        self.pos = end;
        match duration::parse(candidate) {
            Ok(_) => self.push(TokenKind::DurationLiteral, span),
            // A unit in the wrong case is the one refusal here that leaves the
            // literal's shape intact: `rule:classes/reserved-spellings-are-lower-case` gives the
            // spelling one form and the bytes are a duration otherwise, so the
            // token is the literal that was written and the diagnostic's own
            // primary span is the text to lower-case —
            // `rule:tooling/fmt-normalizes-only-reserved-spellings` is what writes it. No compile
            // path continues past the error, so nothing downstream reads a
            // literal that parses only once it is respelled.
            Err(duration::DurationError::MisCasedUnit(_)) => {
                diags.report(
                    Diagnostic::error(
                        code::E_RESERVED_SPELLING_CASE,
                        format!("`{candidate}` must be written in lower case"),
                    )
                    .with_primary(span, format!("write `{}`", candidate.to_ascii_lowercase())),
                );
                self.push(TokenKind::DurationLiteral, span);
            }
            Err(err) => {
                diags.report(
                    Diagnostic::error(
                        code::E_BAD_DURATION_LITERAL,
                        format!("`{candidate}` is not a duration literal"),
                    )
                    .with_primary(span, err.message()),
                );
                self.push(TokenKind::Unknown, span);
            }
        }
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
        let span = self.mk_span(start, self.pos);
        self.check_bidi(span, diags);
        self.push(TokenKind::SingleQuotedString, span);
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

        // `rule:expressions/one-equality-operator`: `===` and `!==` are not spellings Novis has. They are
        // still *recognised* here, for the reason `rule:statements/nvs-is-the-only-open-tag` recognises
        // `<?php` — a rejected spelling nobody names reappears as two
        // confusing tokens — and then reported and lexed as the two-character
        // operator, so one file reports every one of its own problems in one
        // run rather than only the first.
        macro_rules! rejected_equality {
            ($kind:expr, $wrong:literal, $right:literal) => {{
                self.pos += 3;
                let span = self.mk_span(start, self.pos);
                diags.report(
                    Diagnostic::error(
                        code::E_IDENTITY_OPERATOR_UNSUPPORTED,
                        concat!("`", $wrong, "` is not supported"),
                    )
                    .with_primary(span, "Novis keeps exactly one equality operator")
                    .with_help(concat!(
                        "use `",
                        $right,
                        "` — it never converts either operand, so there is nothing for `",
                        $wrong,
                        "` to distinguish"
                    )),
                );
                self.push($kind, span);
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
                // `|>` is tried ahead of `|`: a `|` is never followed by `>`
                // in any expression Novis accepts, and the type grammar's
                // unions are the only other place the character appears.
                if self.starts_with("|>") {
                    op!(2, TokenKind::PipeGreater)
                }
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
                    rejected_equality!(TokenKind::BangEquals, "!==", "!=")
                }
                if self.starts_with("!=") {
                    op!(2, TokenKind::BangEquals)
                }
                op!(1, TokenKind::Bang)
            }
            '=' => {
                if self.starts_with("===") {
                    rejected_equality!(TokenKind::EqualsEquals, "===", "==")
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
                // `rule:expressions/one-equality-operator`: `!=` is the inequality operator, and `<>` is
                // PHP's inherited second spelling of it. Recognised and then
                // rejected for the reason `===` and `!==` are above — a
                // spelling nobody names comes back as `Lt` followed by `Gt`,
                // which is an error about `>` on a line that has no `>` in the
                // sense the author meant. The token pushed is the one `<>`
                // means, so the rest of the file reports its own problems in
                // this run rather than in the next one.
                if self.starts_with("<>") {
                    self.pos += 2;
                    let span = self.mk_span(start, self.pos);
                    diags.report(
                        Diagnostic::error(
                            code::E_ANGLE_NOT_EQUAL_UNSUPPORTED,
                            "`<>` is not supported",
                        )
                        .with_primary(span, "Novis spells inequality one way")
                        .with_help(
                            "use `!=` — it is the same comparison, and `<>` is the second \
                             spelling PHP inherited from its own predecessors",
                        ),
                    );
                    self.push(TokenKind::BangEquals, span);
                    return;
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
    /// literal's span -- see `nvs_types::string_lit::heredoc_shape`.
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
        // What ends the body is all that separates the three: one character
        // for `"…"` and ``html`…` ``, a label line for a heredoc/nowdoc. The
        // scan itself -- segments, `{$` holes, simple `$name`, escapes -- is
        // one scan, which is the whole of `rule:core-classes/html-literal`'s
        // lexing.
        let (delimiter, interpolation, label) = match self.modes.last() {
            Some(Mode::DoubleQuoted { .. }) => (
                Some(('"', TokenKind::DoubleQuoteClose)),
                true,
                String::new(),
            ),
            Some(Mode::Markup { .. }) => (Some(('`', TokenKind::MarkupClose)), true, String::new()),
            Some(Mode::Heredoc {
                label,
                interpolation,
                ..
            }) => (None, *interpolation, label.clone()),
            _ => unreachable!("lex_quoted_body called outside a string/heredoc/markup mode"),
        };
        let is_heredoc = delimiter.is_none();

        if is_heredoc && self.heredoc_terminator_here(&label) {
            let span = self.consume_heredoc_terminator(&label);
            self.push(TokenKind::HeredocClose, span);
            self.modes.pop();
            return;
        }
        if let Some((closer, close_kind)) = delimiter
            && self.peek() == Some(closer)
        {
            let start = self.pos;
            self.bump();
            self.push(close_kind, self.mk_span(start, self.pos));
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
            if let Some((closer, _)) = delimiter
                && self.peek() == Some(closer)
            {
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
            let span = self.mk_span(start, self.pos);
            self.check_bidi(span, diags);
            self.push(TokenKind::StringPart, span);
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
    use nvs_diagnostics::SourceMap;

    use super::*;

    /// Tokenizes `src` and returns just the kinds, plus the diagnostics sink,
    /// so a test can assert on shape without spelling out every span.
    fn kinds(src: &str) -> (Vec<TokenKind>, Diagnostics) {
        let mut map = SourceMap::new();
        let id = map.add("t.nvs", src);
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
            kinds_ok("hello <?nvs echo 1; ?>world"),
            vec![
                InlineHtml,
                OpenTagNvs,
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
    fn a_shebang_first_line_opens_code_mode_and_is_not_a_token() {
        // `rule:tooling/shebang-opens-code-mode`: the bytes `#!` at offset 0 put the outer mode in code,
        // and line 1 is trivia — the token stream is what the same file
        // without it would produce behind a `<?nvs`.
        assert_eq!(
            kinds_ok("#!/usr/bin/env nvs\necho 1;"),
            vec![Keyword(super::Keyword::Echo), IntLiteral, Semicolon, Eof]
        );
        // `?>` leaves code mode as it always does, and the text after it is
        // output rather than a second shebang line.
        assert_eq!(
            kinds_ok("#!/usr/bin/env nvs\n?>tail"),
            vec![CloseTag, InlineHtml, Eof]
        );
        // The trigger is offset 0 and nothing else: one leading space and the
        // file is an ordinary template whose first line is HTML.
        assert_eq!(kinds_ok(" #!/usr/bin/env nvs\n"), vec![InlineHtml, Eof]);
    }

    #[test]
    fn a_shebang_line_is_bidi_checked_like_the_comment_it_is() {
        // The one line in a file whose unbalanced override would reorder
        // everything after it (`rule:security/bidi-boundaries`) — which is why the shebang is
        // lexed as trivia rather than skipped by `Lexer::new`.
        assert_eq!(bidi_errors("#!/usr/bin/env nvs \u{202E}x\necho 1;"), 1);
    }

    #[test]
    fn an_open_tag_in_a_shebang_file_is_named_until_a_close_tag() {
        // `rule:tooling/shebang-opens-code-mode`: `<?nvs` before any `?>` is E0009 and the tag is
        // consumed, so the code after it still lexes as code…
        let (kinds, diags) = kinds("#!/usr/bin/env nvs\n<?nvs echo 1;");
        assert_eq!(
            kinds,
            vec![Keyword(super::Keyword::Echo), IntLiteral, Semicolon, Eof]
        );
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.code == Some(code::E_TAG_IN_SHEBANG_FILE))
                .count(),
            1
        );
        // …and after a `?>` the file is an ordinary template, where the tag is
        // the reopen it looks like.
        assert_eq!(
            kinds_ok("#!/usr/bin/env nvs\n?>text<?nvs echo 1;"),
            vec![
                CloseTag,
                InlineHtml,
                OpenTagNvs,
                Keyword(super::Keyword::Echo),
                IntLiteral,
                Semicolon,
                Eof,
            ]
        );
    }

    #[test]
    fn php_tag_still_lexes_as_its_own_token() {
        // The lexer keeps recognizing `<?php` and switches to code mode on
        // it, same as `<?nvs` — purely so `nvs-syntax`'s parser can produce a
        // diagnostic naming `<?nvs` (`rule:statements/nvs-is-the-only-open-tag`) instead of misreading it
        // as inline HTML. This is a lex-only test; the rejection itself is a
        // parser-level diagnostic, asserted in `parser.rs`.
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
        let (kinds, diags) = kinds("<?nvs echo 1;");
        assert!(!diags.has_errors());
        assert_eq!(
            kinds,
            vec![
                OpenTagNvs,
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
            kinds_ok("<?nvs // a\n # b\n /* c */ echo 1;"),
            vec![
                OpenTagNvs,
                Keyword(super::Keyword::Echo),
                IntLiteral,
                Semicolon,
                Eof
            ]
        );
    }

    #[test]
    fn unterminated_block_comment_is_reported() {
        let (_, diags) = kinds("<?nvs /* never closes");
        assert!(diags.has_errors());
    }

    #[test]
    fn keywords_are_lower_case_only() {
        // `rule:classes/reserved-spellings-are-lower-case`. `ECHO` is an ordinary identifier, with no diagnostic
        // of its own: `rule:core-api/identifier-casing` makes it a legal class name, so nothing
        // here can tell a mis-typed `echo` from a deliberate `ECHO`.
        assert_eq!(
            kinds_ok("<?nvs ECHO myVar"),
            vec![OpenTagNvs, Ident, Ident, Eof]
        );
        assert_eq!(
            kinds_ok("<?nvs echo myVar"),
            vec![OpenTagNvs, Keyword(super::Keyword::Echo), Ident, Eof]
        );
    }

    #[test]
    fn mixed_case_bytes_is_an_ident_not_the_type_keyword() {
        // The concrete payoff of exact keyword matching: `Core\Bytes` is
        // three ordinary name tokens, where case-insensitive matching made
        // `Bytes` collide with the `bytes` type atom.
        assert_eq!(
            kinds_ok(r"<?nvs Core\Bytes"),
            vec![OpenTagNvs, Ident, Backslash, Ident, Eof]
        );
    }

    #[test]
    fn mis_cased_open_tag_is_reported_but_still_opens_code_mode() {
        // `rule:classes/reserved-spellings-are-lower-case`: recognised so the rest of the file keeps lexing as
        // code and the diagnostic can name the fix — `rule:statements/nvs-is-the-only-open-tag`'s treatment
        // of `<?php`, applied to casing.
        let (kinds, diags) = kinds("<?NVS echo 1;");
        assert_eq!(
            kinds,
            vec![
                OpenTagNvs,
                Keyword(super::Keyword::Echo),
                IntLiteral,
                Semicolon,
                Eof
            ]
        );
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(nvs_diagnostics::code::E_RESERVED_SPELLING_CASE)),
            "expected E_RESERVED_SPELLING_CASE, got {diags:?}"
        );
    }

    #[test]
    fn new_type_atoms_are_keywords() {
        assert_eq!(
            kinds_ok("<?nvs uint bytes"),
            vec![
                OpenTagNvs,
                Keyword(super::Keyword::Uint),
                Keyword(super::Keyword::Bytes),
                Eof
            ]
        );
    }

    #[test]
    fn decimal_is_a_keyword() {
        // `rule:types/decimal`: `decimal` is a scalar type, so it reserves a word the
        // same way `uint` and `bytes` above do.
        assert_eq!(
            kinds_ok("<?nvs decimal"),
            vec![OpenTagNvs, Keyword(super::Keyword::Decimal), Eof]
        );
    }

    #[test]
    fn a_trailing_m_is_not_a_decimal_literal_suffix() {
        // `rule:types/numeric-literal-placement` and its *Alternatives rejected*: Novis has no literal
        // suffix at all, so `19.99m` is not one decimal token. `rule:types/duration-literal`
        // decides which *kind* of refusal it gets: a duration is recognised
        // only after a plain decimal integer, and a fractional count is that
        // ADR's own named lexer error (`1.5s`, its § 4), so `19.99m` is
        // rejected by the duration grammar rather than lexing as two tokens.
        // A suffix outside the unit alphabet still splits in two -- see
        // `a_non_unit_suffix_is_still_an_integer_and_an_identifier`.
        let (_, diags) = kinds("<?nvs 19.99m");
        assert!(diags.has_errors());
        assert_eq!(
            kinds_ok("<?nvs 19.99x"),
            vec![OpenTagNvs, FloatLiteral, Ident, Eof]
        );
    }

    #[test]
    fn tainted_is_a_keyword() {
        // `rule:security/tainted-qualifier`: `tainted` needs a new reserved keyword, landing in
        // M1's grammar alongside `uint`'s own addition above.
        assert_eq!(
            kinds_ok("<?nvs tainted string"),
            vec![
                OpenTagNvs,
                Keyword(super::Keyword::Tainted),
                Keyword(super::Keyword::String),
                Eof
            ]
        );
    }

    #[test]
    fn secret_is_a_keyword() {
        // `rule:security/secret-qualifier`: `secret` needs its own new reserved keyword,
        // independent of `tainted`'s.
        assert_eq!(
            kinds_ok("<?nvs secret string"),
            vec![
                OpenTagNvs,
                Keyword(super::Keyword::Secret),
                Keyword(super::Keyword::String),
                Eof
            ]
        );
    }

    #[test]
    fn lateinit_is_a_keyword() {
        // `rule:classes/lateinit-restrictions`: `lateinit` is a new reserved property modifier,
        // alongside `readonly`'s own keyword.
        assert_eq!(
            kinds_ok("<?nvs lateinit"),
            vec![OpenTagNvs, Keyword(super::Keyword::Lateinit), Eof]
        );
    }

    #[test]
    fn spawn_script_with_are_contextual_not_reserved() {
        // These must lex as plain identifiers -- the grammar recognises them
        // by text only at the one position each is meaningful, per
        // docs/spec/00-overview.md § 2 and token.rs's module docs.
        assert_eq!(
            kinds_ok("<?nvs spawn script with type"),
            vec![OpenTagNvs, Ident, Ident, Ident, Ident, Eof]
        );
    }

    #[test]
    fn integer_literal_bases() {
        assert_eq!(
            kinds_ok("<?nvs 0x1F 0o17 0b101 0755 1_000_000"),
            vec![
                OpenTagNvs, IntLiteral, IntLiteral, IntLiteral, IntLiteral, IntLiteral, Eof
            ]
        );
    }

    /// `rule:types/duration-literal`: one token per literal, maximal munch, and the units in
    /// descending order.
    #[test]
    fn duration_literal_shapes() {
        assert_eq!(
            kinds_ok("<?nvs 30s 1h30m 500ms 1w 1w2d3h4m5s6ms7us8ns"),
            vec![
                OpenTagNvs,
                DurationLiteral,
                DurationLiteral,
                DurationLiteral,
                DurationLiteral,
                DurationLiteral,
                Eof
            ]
        );
    }

    /// The two shapes `rule:types/duration-literal` protects: `0x1d` stays one hex literal
    /// because the `0x` form returns before the duration production is
    /// reached, and `3 d` is two tokens because whitespace ends the candidate.
    #[test]
    fn a_duration_literal_does_not_swallow_a_hex_literal_or_a_spaced_identifier() {
        assert_eq!(kinds_ok("<?nvs 0x1d"), vec![OpenTagNvs, IntLiteral, Eof]);
        assert_eq!(
            kinds_ok("<?nvs 3 d"),
            vec![OpenTagNvs, IntLiteral, Ident, Eof]
        );
    }

    /// A suffix outside the unit alphabet is not a duration attempt at all, so
    /// it keeps the integer-then-identifier lexing it always had rather than
    /// collecting a duration diagnostic.
    #[test]
    fn a_non_unit_suffix_is_still_an_integer_and_an_identifier() {
        assert_eq!(
            kinds_ok("<?nvs 30foo 1e 30Something"),
            vec![
                OpenTagNvs, IntLiteral, Ident, IntLiteral, Ident, IntLiteral, Ident, Eof
            ]
        );
    }

    /// Every refusal `rule:types/duration-literal` makes about a literal's
    /// *shape* reaches the lexer, and each produces one error rather than a
    /// cascade — the grammar itself is tested in [`crate::duration`], so what
    /// this holds is that the lexer *reaches* it. There is nothing left to lex,
    /// so the token is [`TokenKind::Unknown`].
    #[test]
    fn a_malformed_duration_literal_is_one_lexer_error() {
        for src in ["<?nvs 30m1h", "<?nvs 1h1h", "<?nvs 1.5s", "<?nvs 100000w"] {
            let (kinds, diags) = kinds(src);
            assert!(diags.has_errors(), "{src} should be refused");
            assert_eq!(kinds, vec![OpenTagNvs, Unknown, Eof], "for {src}");
        }
    }

    /// The case of a unit is the one thing `rule:types/duration-literal` refuses that leaves a
    /// literal behind: the shape is a duration and only its spelling is wrong,
    /// so the token is the one that was written and the error names the
    /// lower-case form for `rule:tooling/fmt-normalizes-only-reserved-spellings` to write.
    #[test]
    fn a_mis_cased_duration_unit_keeps_its_token_and_names_its_spelling() {
        let (kinds, diags) = kinds("<?nvs 1H30M");
        assert_eq!(kinds, vec![OpenTagNvs, DurationLiteral, Eof]);
        assert_eq!(diags.error_count(), 1);
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(nvs_diagnostics::code::E_RESERVED_SPELLING_CASE)),
            "expected E_RESERVED_SPELLING_CASE, got {diags:?}"
        );
    }

    /// `rule:types/duration-literal` keeps the sign out of the literal, so the parser never has
    /// to decide whether the `-` in `$a -7d` is binary — it is always its own
    /// token, and `nvs_types` refuses the arithmetic that results.
    #[test]
    fn a_duration_literal_never_carries_a_sign() {
        assert_eq!(
            kinds_ok("<?nvs -7d"),
            vec![OpenTagNvs, Minus, DurationLiteral, Eof]
        );
    }

    #[test]
    fn float_literal_shapes() {
        assert_eq!(
            kinds_ok("<?nvs 1.5 .5 1. 1e10 1.5e-3"),
            vec![
                OpenTagNvs,
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
        let (kinds, diags) = kinds(r"<?nvs 'a\'b\\c\nd'");
        assert!(!diags.has_errors());
        assert_eq!(kinds, vec![OpenTagNvs, SingleQuotedString, Eof]);
    }

    #[test]
    fn unterminated_single_quoted_string_is_reported() {
        let (kinds, diags) = kinds("<?nvs 'abc");
        assert!(diags.has_errors());
        assert_eq!(kinds, vec![OpenTagNvs, SingleQuotedString, Eof]);
    }

    /// `rule:security/bidi-boundaries`: the four source spans that carry free text, each rejected
    /// when a directional scope it opens outlives the line that opened it.
    fn bidi_errors(src: &str) -> usize {
        let (_, diags) = kinds(src);
        diags
            .iter()
            .filter(|d| d.code == Some(code::E_UNBALANCED_BIDI))
            .count()
    }

    #[test]
    fn an_unterminated_bidi_control_is_rejected_in_every_source_span() {
        // A comment -- both spellings -- a single-quoted literal, a
        // double-quoted one, and an inline-HTML run.
        assert_eq!(bidi_errors("<?nvs // owner\u{202E} check\n"), 1);
        assert_eq!(bidi_errors("<?nvs # owner\u{202E} check\n"), 1);
        assert_eq!(bidi_errors("<?nvs /* owner\u{202E} check */\n"), 1);
        assert_eq!(bidi_errors("<?nvs echo 'owner\u{202E}';"), 1);
        assert_eq!(bidi_errors("<?nvs echo \"owner\u{2066}\";"), 1);
        assert_eq!(bidi_errors("plain\u{202B}html<?nvs echo 1;"), 1);
    }

    #[test]
    fn a_balanced_bidi_control_lexes_cleanly() {
        // The case that fails if the rule is ever widened to a blanket ban:
        // legitimate mixed-direction text isolates the Latin run and closes it.
        let (kinds, diags) = kinds("<?nvs echo \"خطأ \u{2066}user_id\u{2069} !\";");
        assert!(!diags.has_errors(), "{:?}", diags.iter().next());
        assert_eq!(
            kinds,
            vec![
                OpenTagNvs,
                Keyword(super::Keyword::Echo),
                DoubleQuoteOpen,
                StringPart,
                DoubleQuoteClose,
                Semicolon,
                Eof,
            ]
        );
    }

    #[test]
    fn a_bidi_scope_may_not_cross_a_line_inside_one_token() {
        // Balanced across the heredoc as a whole, unbalanced per line -- which
        // is the span `rule:security/bidi-boundaries` gives the lexer, so this is rejected.
        let src = "<?nvs $s = <<<TXT\n\u{202E}first\nsecond\u{202C}\nTXT;\n";
        assert_eq!(bidi_errors(src), 1);
        // The same two lines, each closing its own scope, are fine.
        let ok = "<?nvs $s = <<<TXT\n\u{202E}first\u{202C}\n\u{202E}second\u{202C}\nTXT;\n";
        assert_eq!(bidi_errors(ok), 0);
    }

    #[test]
    fn a_stray_terminator_is_not_an_error() {
        // It closes nothing and opens nothing -- `rule:security/bidi-predicate`.
        assert_eq!(bidi_errors("<?nvs echo 'a\u{202C}b\u{2069}';"), 0);
    }

    #[test]
    fn double_quoted_string_with_no_interpolation() {
        assert_eq!(
            kinds_ok(r#"<?nvs "plain text""#),
            vec![
                OpenTagNvs,
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
            kinds_ok(r#"<?nvs "a $name b""#),
            vec![
                OpenTagNvs,
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
            kinds_ok(r#"<?nvs "$obj->prop->more""#),
            vec![
                OpenTagNvs,
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
            kinds_ok(r#"<?nvs "$arr[key] $arr[-1] $arr[$i]""#),
            vec![
                OpenTagNvs,
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
            kinds_ok(r#"<?nvs "sum: {$a + $b}""#),
            vec![
                OpenTagNvs,
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
            kinds_ok(r#"<?nvs "{$f(function () { return 1; })}""#),
            vec![
                OpenTagNvs,
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
    fn a_markup_literal_lexes_as_parts_the_way_a_double_quoted_string_does() {
        // The same shape a `"…"` produces with the delimiter swapped: segments
        // and `{$` holes, and nothing in the lexer that knows what a tag is
        // (`rule:core-classes/html-literal`).
        assert_eq!(
            kinds_ok("<?nvs echo html`<span>posted by </span>{$name}`;"),
            vec![
                OpenTagNvs,
                Keyword(super::Keyword::Echo),
                MarkupOpen,
                StringPart,
                ComplexInterpOpen,
                Variable,
                ComplexInterpClose,
                MarkupClose,
                Semicolon,
                Eof,
            ]
        );
    }

    #[test]
    fn a_markup_hole_tracks_nested_braces_so_a_closure_does_not_close_it() {
        // The hole opens the same `Mode::Code` frame a string's does, so the
        // closure's braces are counted rather than mistaken for the closer.
        assert_eq!(
            kinds_ok("<?nvs html`<p>{$f(function () { return 1; })}</p>`;"),
            vec![
                OpenTagNvs,
                MarkupOpen,
                StringPart,
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
                StringPart,
                MarkupClose,
                Semicolon,
                Eof,
            ]
        );
    }

    #[test]
    fn a_brace_before_a_non_dollar_stays_a_segment_byte() {
        // A `<style>` block needs no escape and `{Money::format(…)}` is text,
        // because only `{$` opens a hole (ADR 0169 § 3). The `$c` inside that
        // text still interpolates in the simple syntax, which is the same
        // answer a double-quoted string gives.
        assert_eq!(
            kinds_ok("<?nvs html`<style>.a{color:red}</style>{Money::format($c)}`;"),
            vec![
                OpenTagNvs,
                MarkupOpen,
                StringPart,
                Variable,
                StringPart,
                MarkupClose,
                Semicolon,
                Eof,
            ]
        );
    }

    #[test]
    fn a_bare_dollar_name_interpolates_in_a_markup_literal() {
        assert_eq!(
            kinds_ok("<?nvs html`<b>$name</b>`;"),
            vec![
                OpenTagNvs,
                MarkupOpen,
                StringPart,
                Variable,
                StringPart,
                MarkupClose,
                Semicolon,
                Eof,
            ]
        );
    }

    #[test]
    fn a_backslash_backtick_is_a_literal_backtick_in_a_markup_segment() {
        // The escape is consumed with the segment, so the delimiter it names
        // never reaches the closer check and the body stays one part.
        assert_eq!(
            kinds_ok(r"<?nvs html`<code>\`</code>`;"),
            vec![
                OpenTagNvs,
                MarkupOpen,
                StringPart,
                MarkupClose,
                Semicolon,
                Eof,
            ]
        );
    }

    #[test]
    fn a_backslash_brace_is_a_literal_brace_before_a_dollar() {
        // `\{` is what writes a literal brace where `{$` would otherwise open
        // a hole; what follows is then an ordinary simple interpolation.
        assert_eq!(
            kinds_ok(r"<?nvs html`\{$name}`;"),
            vec![
                OpenTagNvs,
                MarkupOpen,
                StringPart,
                Variable,
                StringPart,
                MarkupClose,
                Semicolon,
                Eof,
            ]
        );
    }

    #[test]
    fn an_unterminated_markup_literal_reports_e0002_at_the_delimiter_that_opened_it() {
        // The code an unterminated string, heredoc and interpolation already
        // carry -- the literal adds no diagnostic of its own
        // (`rule:core-classes/html-literal`).
        let src = "<?nvs echo html`<p>hello";
        let (kinds, diags) = kinds(src);
        assert_eq!(
            kinds,
            vec![
                OpenTagNvs,
                Keyword(super::Keyword::Echo),
                MarkupOpen,
                StringPart,
                MarkupClose,
                Eof,
            ]
        );
        let reported = diags
            .iter()
            .find(|d| d.code == Some(code::E_UNTERMINATED))
            .expect("an open literal at end of file is reported");
        let opener = u32::try_from(src.find("html`").expect("the opener is in the source"))
            .expect("test sources are short");
        assert_eq!(reported.labels[0].span.start, opener);
    }

    #[test]
    fn dollar_dollar_is_two_tokens_not_a_variable() {
        assert_eq!(
            kinds_ok("<?nvs $$name"),
            vec![OpenTagNvs, Dollar, Variable, Eof]
        );
    }

    #[test]
    fn heredoc_with_interpolation() {
        let src = "<?nvs $s = <<<EOT\nhello $name\nEOT;\n";
        assert_eq!(
            kinds_ok(src),
            vec![
                OpenTagNvs,
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
        let src = "<?nvs $s = <<<'EOT'\nraw $name text\nEOT;\n";
        assert_eq!(
            kinds_ok(src),
            vec![
                OpenTagNvs,
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
        let src = "<?nvs <<<EOT\nEOTX\nEOT;\n";
        let (kinds, diags) = kinds(src);
        assert!(!diags.has_errors());
        assert_eq!(
            kinds,
            vec![
                OpenTagNvs,
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
        let (kinds, diags) = kinds("<?nvs <<<EOT\nhello\n");
        assert!(diags.has_errors());
        assert_eq!(
            kinds,
            vec![OpenTagNvs, HeredocOpen, StringPart, HeredocClose, Eof]
        );
    }

    #[test]
    fn operators_longest_match_wins() {
        assert_eq!(
            // `<>` was here until it became a rejected spelling; it is the one
            // operator whose longest match reports, so it is asserted in
            // `the_angle_inequality_spelling_is_a_compile_error` instead.
            kinds_ok("<?nvs <=> ??= ?-> **= <<= >>= ->"),
            vec![
                OpenTagNvs,
                Spaceship,
                QuestionQuestionEquals,
                NullsafeArrow,
                StarStarEquals,
                LtLtEquals,
                GtGtEquals,
                Arrow,
                Eof,
            ]
        );
    }

    /// `rule:expressions/one-equality-operator`: neither rejected spelling reaches the parser, and each
    /// is still consumed whole — three characters, one diagnostic — so the
    /// tokens either side of it are the ones the author wrote.
    #[test]
    fn a_rejected_equality_spelling_is_a_compile_error() {
        for (src, spelling) in [
            ("<?nvs $a === $b;", "`===` is not supported"),
            ("<?nvs $a !== $b;", "`!==` is not supported"),
        ] {
            let (kinds, diags) = kinds(src);
            assert!(
                diags.iter().any(|d| {
                    d.code == Some(nvs_diagnostics::code::E_IDENTITY_OPERATOR_UNSUPPORTED)
                        && d.message == spelling
                }),
                "expected E_IDENTITY_OPERATOR_UNSUPPORTED for {src:?}, got {diags:?}"
            );
            assert_eq!(
                kinds,
                vec![
                    OpenTagNvs,
                    Variable,
                    if spelling.starts_with("`!") {
                        BangEquals
                    } else {
                        EqualsEquals
                    },
                    Variable,
                    Semicolon,
                    Eof,
                ],
                "recovery for {src:?}"
            );
        }
    }

    /// `rule:expressions/one-equality-operator`: `<>` is the third spelling a *lexical* rule refuses, and
    /// like the other two it is consumed whole — two characters, one
    /// diagnostic — so the tokens either side are the ones the author wrote
    /// and no `Lt`/`Gt` pair reaches the parser to be reported about instead.
    #[test]
    fn the_angle_inequality_spelling_is_a_compile_error() {
        let (kinds, diags) = kinds("<?nvs $a <> $b;");
        assert!(
            diags.iter().any(|d| {
                d.code == Some(nvs_diagnostics::code::E_ANGLE_NOT_EQUAL_UNSUPPORTED)
                    && d.message == "`<>` is not supported"
            }),
            "expected E_ANGLE_NOT_EQUAL_UNSUPPORTED, got {diags:?}"
        );
        assert_eq!(
            kinds,
            vec![OpenTagNvs, Variable, BangEquals, Variable, Semicolon, Eof,],
            "recovery"
        );
    }

    /// The prefixes `<>` shares its first character with keep their meanings:
    /// a rejected spelling is recognised, never given a character it does not
    /// own.
    #[test]
    fn the_other_angle_operators_are_untouched() {
        assert_eq!(
            kinds_ok("<?nvs $a <=> $b; $a << $b; $a <= $b; $a < $b;"),
            vec![
                OpenTagNvs, Variable, Spaceship, Variable, Semicolon, Variable, LtLt, Variable,
                Semicolon, Variable, LtEquals, Variable, Semicolon, Variable, Lt, Variable,
                Semicolon, Eof,
            ]
        );
    }

    #[test]
    fn attribute_open_is_distinct_from_a_comment() {
        assert_eq!(
            kinds_ok("<?nvs #[Attr] # comment\n1"),
            vec![OpenTagNvs, AttributeOpen, Ident, RBracket, IntLiteral, Eof]
        );
    }

    #[test]
    fn unexpected_character_recovers() {
        let (kinds, diags) = kinds("<?nvs 1 ` 2");
        assert!(diags.has_errors());
        assert_eq!(
            kinds,
            vec![OpenTagNvs, IntLiteral, Unknown, IntLiteral, Eof]
        );
    }

    #[test]
    fn eof_yields_forever() {
        let mut map = SourceMap::new();
        let id = map.add("t.nvs", "<?nvs 1");
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
