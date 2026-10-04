//! What `textDocument/inlayHint` answers, over documents the type phase reached
//! and documents it did not.
//!
//! Here rather than in a unit test for `tests/redactions.rs`' reason: the
//! answer is a *type* question, and only a full analysis — the graph `nvs
//! check` resolves, the check it runs — knows what a `var` declaration inferred
//! or which parameter an argument filled. An open buffer stands in for the file
//! under it (`nvs_diagnostics::SourceMap::load`), so a case here needs no
//! directory on disk.
//!
//! The expectations are the hint's own fields rather than a rendering of them:
//! `rule:ide/the-rendering-has-one-home` puts the text a case freezes in
//! `nvs_lsp::render`, and that spelling arrives with the request itself. What
//! is pinned here is *what is answered* — where, with what label, as which
//! kind — which is the half a `.lspt` case cannot show on its own.

use lsp_types::{InlayHint, InlayHintKind, InlayHintLabel};
use nvs_diagnostics::PositionEncoding;
use nvs_lsp::{Documents, analyse, hints, uri_of};

/// Every hint `source` carries, as `(line, character, label, kind)` on the
/// wire's own 0-based positions.
///
/// UTF-8 columns, which is what a case is read in: the encoding negotiation is
/// `rule:ide/positions-have-one-home`'s business and has nothing to say about
/// which hints are answered.
fn hints(source: &str) -> Vec<(u32, u32, String, InlayHintKind)> {
    let dir = nvs_repo::scratch("lsp-hints");
    let uri = uri_of(&dir.join("case.nvs")).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    let analysed = analyse(&documents, &uri).expect("an open document analyses");
    hints::for_document(&analysed, PositionEncoding::Utf8)
        .iter()
        .map(entry)
        .collect()
}

/// One hint, flattened to the four things a test has anything to say about.
fn entry(hint: &InlayHint) -> (u32, u32, String, InlayHintKind) {
    let InlayHintLabel::String(label) = &hint.label else {
        panic!("this server writes a hint's label as one string");
    };
    (
        hint.position.line,
        hint.position.character,
        label.clone(),
        hint.kind
            .expect("every hint this server answers has a kind"),
    )
}

/// `rule:types/var-inference`'s spelling is the one that hides a type, so it is
/// the one that gets it back — and a declaration that wrote its type out gets
/// nothing, because the word is already on the line.
#[test]
fn an_inferred_var_declaration_carries_its_type_as_a_hint() {
    assert_eq!(
        hints("<?nvs\nvar $count = 41;\nint $written = 1;\nvar $greeting = \"hi\";\n"),
        vec![
            (1, 10, ": int".to_owned(), InlayHintKind::TYPE),
            (3, 13, ": string".to_owned(), InlayHintKind::TYPE),
        ]
    );
}

/// A `foreach` binding written `var` hides its type the way a `var` local
/// does, so it gets the same hint after its name: `string` on an array's key,
/// the element type on its value. The loop that wrote its types gets nothing.
#[test]
fn a_var_foreach_binding_is_hinted_with_its_inferred_type() {
    let source = concat!(
        "<?nvs\n",
        "array<int> $a = [1, 2];\n",
        "foreach ($a as var $k => var $v) {\n",
        "    echo $v;\n",
        "}\n",
        "foreach ($a as string $key => int $value) {\n",
        "    echo $value;\n",
        "}\n",
    );
    assert_eq!(
        hints(source),
        vec![
            (2, 21, ": string".to_owned(), InlayHintKind::TYPE),
            (2, 31, ": int".to_owned(), InlayHintKind::TYPE),
        ]
    );
}

/// A `var` local over a one-type array literal hides an `array<T>` the way
/// `var $n = 1;` hides an `int`, so it gets the same hint — nested literals
/// included, since the hint is the binding's checked type.
#[test]
fn a_var_array_literal_is_hinted_with_its_inferred_type() {
    assert_eq!(
        hints("<?nvs\nvar $ids = [1, 2];\nvar $grid = [[1], [2, 3]];\n"),
        vec![
            (1, 8, ": array<int>".to_owned(), InlayHintKind::TYPE),
            (2, 9, ": array<array<int>>".to_owned(), InlayHintKind::TYPE),
        ]
    );
}

/// A literal argument is the one a reader cannot tell the meaning of, so it is
/// the one that gets the parameter's name — joined through
/// `nvs_types::ResolvedCall::arg_slots`, which is the checker's own answer to
/// which parameter each written argument filled.
///
/// An argument that is a variable already says what it is and gets nothing, and
/// so does one written `name:`, which would otherwise be annotated twice.
#[test]
fn a_literal_argument_carries_its_parameter_name() {
    let source = concat!(
        "<?nvs\n",
        "class Greeter {\n",
        "    public function greet(string $name, int $times): string { return $name; }\n",
        "}\n",
        "var $g = new Greeter();\n",
        "string $who = \"world\";\n",
        "$g->greet(\"world\", 2);\n",
        "$g->greet($who, 2);\n",
        "$g->greet(name: \"world\", times: 2);\n",
    );
    assert_eq!(
        hints(source),
        vec![
            (4, 6, ": Greeter".to_owned(), InlayHintKind::TYPE),
            (6, 10, "name:".to_owned(), InlayHintKind::PARAMETER),
            (6, 19, "times:".to_owned(), InlayHintKind::PARAMETER),
            (7, 16, "times:".to_owned(), InlayHintKind::PARAMETER),
        ]
    );
}

/// The module's whole failure mode, asserted from the other side: a hint is
/// text an editor draws as if it were in the file, so a site the type phase
/// recorded nothing for gets none rather than a guess.
///
/// Two sites that record nothing, in one document that does resolve: a receiver
/// nothing declared, and a method the class it resolved to does not have. The
/// declaration above them still carries its hint, which is what says the empty
/// answer is the table's and not the walk's.
#[test]
fn no_hint_is_produced_from_anything_the_type_phase_did_not_record() {
    assert_eq!(hints("<?nvs\n$unknown->greet(\"world\", 2);\n"), vec![]);
    assert_eq!(
        hints("<?nvs\nclass C {}\nvar $c = new C();\n$c->nope(1);\n"),
        vec![(2, 6, ": C".to_owned(), InlayHintKind::TYPE)]
    );
}
