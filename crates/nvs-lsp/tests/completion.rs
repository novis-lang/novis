//! What a receiver offers when the token after its arrow is not a member name.
//!
//! Here rather than beside `nvs_lsp::completion`'s unit tests because the
//! answer is what a whole analysis resolved: the receiver's class comes from
//! the checker's own table, so a fixture would be asserting that a type this
//! file wrote survives being copied.
//!
//! The claim is one the `.lspt` corpus cannot make in a single case. Each case
//! under `tests/lsp/completion/` freezes one rendering at one cursor
//! (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`); what is asserted here
//! is that every one of those renderings is the *same* one — a receiver at the
//! end of a block, at the end of the file, before a `)` and before a `]`
//! answers exactly what the identical receiver above another statement answers.
//! The parser stops the access at the arrow in all four, so the closing
//! delimiter is the only thing that varies and none of it may reach the answer.

use nvs_lsp::{Analysed, Documents, Response, analyse, completion, uri_of};

/// The class every document below resolves its receiver to.
const CLASS: &str =
    r#"class User { public string $name; public function greet(): string { return "hi"; } }"#;

/// The receiver whose member half is empty in every document.
const RECEIVER: &str = "$b->";

/// `source` analysed as its own entry point, out of an open buffer alone —
/// nothing here `require`s anything, so no directory is needed
/// (`nvs_diagnostics::SourceMap::load`).
fn analysed(source: &str) -> Analysed {
    let uri = uri_of(&std::env::temp_dir().join("nvs-completion-case.nvs"))
        .expect("a temp path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    analyse(&documents, &uri).expect("an open document analyses")
}

/// What is offered immediately after the one trailing arrow in `source`,
/// rendered as a `.lspt` case freezes it.
fn offered(source: &str) -> String {
    let at = source
        .find(RECEIVER)
        .expect("the document writes the receiver")
        + RECEIVER.len();
    let at = u32::try_from(at).expect("a test document is short");
    Response::Completion(completion::at(&analysed(source), at)).render()
}

/// A document whose receiver is followed by `tail`.
fn document(tail: &str) -> String {
    format!("<?nvs\n{CLASS}\nvar $b = new User();\n{tail}")
}

/// The instance half of `User`, which is the whole answer in every shape below.
///
/// Its spelling is `nvs_lsp::render`'s and not this file's
/// (`rule:ide/the-rendering-has-one-home`), so the corpus and this test freeze
/// one text.
const MEMBERS: &str = "greet   method    (): string\nname    property  string\n";

/// A member list is what a receiver offers whatever closes the construct it
/// was written in.
///
/// The first row is the shape that has always worked, and it is in the list as
/// the control: `$b->` followed by another statement parses as a method call
/// whose member name is `if`, so the access runs past the cursor and the
/// cursor is inside it. In the other four nothing follows the arrow that the
/// grammar can take for a member name, so the access ends exactly at the
/// cursor — and a cursor at a node's end is outside it, which is
/// `nvs_syntax::SyntaxIndex::at`'s half-open containment and stays that way
/// because `selectionRange` is frozen on it.
#[test]
fn a_member_list_survives_a_receiver_at_the_end_of_a_block() {
    let shapes = [
        ("another statement", "$b->\nif (true) {\n"),
        ("the end of a block", "if (true) {\n    $b->\n}\n"),
        ("the end of the file", "$b->\n"),
        ("a closing parenthesis", "echo ($b->);\nif (true) {\n"),
        ("a closing bracket", "var $a = [$b->];\nif (true) {\n"),
    ];
    for (delimiter, tail) in shapes {
        assert_eq!(
            offered(&document(tail)),
            MEMBERS,
            "a receiver before {delimiter} is offered something other than its class's members"
        );
    }
}
