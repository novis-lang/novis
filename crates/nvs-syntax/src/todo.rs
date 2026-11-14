//! `rule:tooling/a-todo-is-a-comment-the-tools-list`: which comments are todos,
//! and what each one says.
//!
//! A todo is read from the trivia a [`Lexer::with_trivia`] keeps, never from the
//! tree, so the grammar does not know the form exists. A
//! [`TriviaKind::LineComment`] that opens with `//` can be one, and so can a
//! [`TriviaKind::DocComment`] line: a todo inside a `///` run is the author's
//! note beside the documentation, and the parser keeps that line out of the
//! prose ([`crate::ast::DocComment::is_todo`]). A `#` comment is a line comment
//! too and is never one. What follows the slashes and the spaces after them
//! must be `TODO:` in capitals, and the rest of the line, trimmed, is the
//! todo's text.
//!
//! [`todos`] lexes the file a second time to get the trivia. It runs only where
//! a tool lists todos (the editor and `nvs check --todos`), so the compile path
//! pays nothing, and the second lex is O(file) beside a parse of the same file.

use nvs_diagnostics::{Diagnostics, SourceFile, Span};

use crate::{Lexer, TokenKind, Trivia, TriviaKind};

/// What marks a comment as a todo, after its slashes and the spaces after them.
const MARKER: &str = "TODO:";

/// One `// TODO:` or `/// TODO:` comment line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Todo {
    /// The whole comment line, its slashes included.
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
        .filter(|t| matches!(t.kind, TriviaKind::LineComment | TriviaKind::DocComment))
        .filter_map(|t| {
            let text = todo_text(file.span_text(t.span)?)?;
            Some(Todo {
                span: t.span,
                text: text.trim().to_string(),
            })
        })
        .collect()
}

/// What follows `TODO:` in `comment`, untrimmed, when the comment is a todo.
/// `comment` is one comment line as written, its slashes first; a line that
/// does not open with `//` is never a todo.
pub(crate) fn todo_text(comment: &str) -> Option<&str> {
    let body = comment.strip_prefix("//")?.trim_start_matches('/');
    body.trim_start().strip_prefix(MARKER)
}
