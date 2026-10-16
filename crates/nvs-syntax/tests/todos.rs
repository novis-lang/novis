//! `rule:tooling/a-todo-is-a-comment-the-tools-list`: which comments are todos,
//! and the text each one carries.

use nvs_diagnostics::SourceMap;
use nvs_syntax::todos;

/// Every todo in `src`, as the line it is on (0-based) and its text.
fn todos_of(src: &str) -> Vec<(usize, String)> {
    let mut map = SourceMap::new();
    let id = map.add("t.nvs", src);
    let file = map.file(id);
    todos(file)
        .into_iter()
        .map(|t| (file.line_index(t.span.start), t.text))
        .collect()
}

#[test]
fn a_todo_line_comment_is_collected_with_its_text() {
    let src = "<?nvs\n\
               // TODO: page through the results once `find` takes an offset.\n\
               $x = 1; //TODO:   trim the name  \n\
               //// TODO: a divider can carry one too\n\
               // TODO:\n";
    assert_eq!(
        todos_of(src),
        vec![
            (
                1,
                "page through the results once `find` takes an offset.".to_string()
            ),
            (2, "trim the name".to_string()),
            (3, "a divider can carry one too".to_string()),
            (4, String::new()),
        ]
    );
}

#[test]
fn a_doc_comment_a_block_comment_and_a_lowercase_todo_are_not_collected() {
    let src = "<?nvs\n\
               /// TODO: this is documentation\n\
               function f(): void {}\n\
               /* TODO: a block comment */\n\
               // todo: lower case\n\
               // TODO without the colon\n\
               // a TODO: in the middle\n\
               # TODO: a hash comment\n\
               $s = \"// TODO: inside a string\";\n";
    assert_eq!(todos_of(src), Vec::<(usize, String)>::new());
}
