//! `rule:tooling/a-todo-is-a-comment-the-tools-list`: which comments are todos,
//! and what each one says.
//!
//! A todo is read from the trivia a [`Lexer::with_trivia`] keeps, never from the
//! tree, so the grammar does not know the form exists. Only a
//! [`TriviaKind::LineComment`] that opens with `//` can be one: a `#` comment is
//! the same kind of trivium and is not, and a `///` is a
//! [`TriviaKind::DocComment`] the filter never sees. What follows the slashes
//! and the spaces after them must be `TODO:` in capitals, and the rest of the
//! line, trimmed, is the todo's text.
//!
//! [`todos`] lexes the file a second time to get the trivia. It runs only where
//! a tool lists todos (the editor and `nvs check --todos`), so the compile path
//! pays nothing, and the second lex is O(file) beside a parse of the same file.

use nvs_diagnostics::{Diagnostics, SourceFile, Span};

use crate::{Lexer, TokenKind, Trivia, TriviaKind};

/// What marks a comment as a todo, after its slashes and the spaces after them.
const MARKER: &str = "TODO:";

/// One `// TODO:` comment.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Todo {
    /// The whole comment, its slashes included.
    pub span: Span,
    /// What follows `TODO:`, with the spaces around it trimmed. It may be empty.
    pub text: String,
}

/// Every todo in `file`, in source order.
///
/// A lexical error is ignored here: the parse of the same file reports it, and
/// the comments before and after it are still trivia.
#[must_use]
pub fn todos(file: &SourceFile) -> Vec<Todo> {
    let mut diags = Diagnostics::new();
    let mut lexer = Lexer::with_trivia(file);
    while lexer.next_token(&mut diags).kind != TokenKind::Eof {}
    todos_in(file, lexer.trivia())
}

/// The todos among `trivia`, which must come from `file`.
#[must_use]
pub fn todos_in(file: &SourceFile, trivia: &[Trivia]) -> Vec<Todo> {
    trivia
        .iter()
        .filter(|t| t.kind == TriviaKind::LineComment)
        .filter_map(|t| {
            let comment = file.span_text(t.span)?;
            let body = comment.strip_prefix("//")?.trim_start_matches('/');
            let text = body.trim_start().strip_prefix(MARKER)?;
            Some(Todo {
                span: t.span,
                text: text.trim().to_string(),
            })
        })
        .collect()
}
