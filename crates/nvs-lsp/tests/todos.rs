//! What an editor shows for a `// TODO:` comment
//! (`rule:tooling/a-todo-is-a-comment-the-tools-list`): an `Information` entry
//! with source `todo` and no code, published beside the file's diagnostics and
//! never held back by the phase gate.

use lsp_types::{DiagnosticSeverity, Position};
use nvs_diagnostics::PositionEncoding;
use nvs_lsp::completion_files::CompletionFiles;
use nvs_lsp::{Documents, Phases, SOURCE, TODO_SOURCE, analyse, for_document, uri_of};

/// What an editor is sent for `source`, opened alone.
fn published(source: &str) -> Vec<lsp_types::Diagnostic> {
    let dir = nvs_repo::scratch("lsp-todo-comment");
    let uri = uri_of(&dir.join("case.nvs")).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    let analysed = analyse(&documents, &uri).expect("an open document analyses");
    for_document(
        &analysed,
        &CompletionFiles::default(),
        Phases::Gated,
        PositionEncoding::Utf16,
    )
}

#[test]
fn a_todo_comment_is_published_at_information_level() {
    // The parse error opens the phase gate, and the todo is published under it.
    let entries = published("<?nvs\n// TODO: page through the results.\n$x = ;\n");
    assert!(
        entries
            .iter()
            .any(|entry| entry.source.as_deref() == Some(SOURCE)),
        "the parse error is published too: {entries:?}"
    );
    let todos: Vec<_> = entries
        .iter()
        .filter(|entry| entry.source.as_deref() == Some(TODO_SOURCE))
        .collect();
    assert_eq!(todos.len(), 1, "{entries:?}");
    let todo = todos[0];
    assert_eq!(todo.severity, Some(DiagnosticSeverity::INFORMATION));
    assert_eq!(todo.code, None);
    assert_eq!(todo.message, "page through the results.");
    assert_eq!(todo.range.start, Position::new(1, 0));
    assert_eq!(todo.range.end, Position::new(1, 34));
}
