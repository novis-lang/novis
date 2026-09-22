//! What `nvs/imports` carries off a copied range, and what `nvs/importEdits`
//! writes where it is pasted.
//!
//! Here rather than as `.lspt` cases because `nvs_lsp::render` has no spelling
//! for either answer — a list of name pairs, and a text edit — and a case may
//! not invent one (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`). The
//! two are held on the wire shapes the server answers, over open buffers alone
//! (`nvs_diagnostics::SourceMap::load`), so nothing here needs a directory.

use nvs_diagnostics::PositionEncoding;
use nvs_lsp::imports::{self, Import};
use nvs_lsp::{CheckScope, Documents, SymbolIndex, analyse, uri_of};

/// The file a selection is copied out of: a namespace, one import, a class
/// of its own, and every place a type name is written — a `::` receiver, a
/// `new`, a typed local, a parameter, and a qualified name that needs no
/// import.
const SOURCE: &str = "<?nvs\nnamespace Shop;\n\nuse Core\\Str;\n\nclass Cart {\n    public function total(Cart $other): int { return 1; }\n}\nvar $n = Str::length(\"a\");\nvar $c = new Cart();\nCart $again = $c;\nCore\\Debug::dump($n);\n";

/// `source`, analysed as an open buffer at `name`.
fn open(documents: &mut Documents, name: &str, source: &str) -> nvs_lsp::Analysed {
    let uri = uri_of(&std::env::temp_dir().join(name)).expect("a temp path is UTF-8");
    documents.open(uri.clone(), 1, source.to_owned());
    analyse(documents, &uri).expect("an open document analyses")
}

/// What `nvs/imports` answers for the bytes of `source` between the first
/// `from` and the end of the first `to`.
fn carried(source: &str, from: &str, to: &str) -> Vec<(String, String)> {
    let mut documents = Documents::new();
    let analysed = open(&mut documents, "nvs-imports-source.nvs", source);
    let start = source.find(from).expect("the document writes it");
    let end = source.find(to).expect("the document writes it") + to.len();
    imports::in_range(
        &analysed,
        u32::try_from(start).expect("short"),
        u32::try_from(end).expect("short"),
    )
    .into_iter()
    .map(|import| (import.symbol, import.written))
    .collect()
}

/// What `nvs/importEdits` writes into `destination` at its end for `carried`,
/// as `(line, character, text)` per edit.
fn written(destination: &str, carried: &[(&str, &str)]) -> Vec<(u32, u32, String)> {
    let mut documents = Documents::new();
    let analysed = open(&mut documents, "nvs-imports-destination.nvs", destination);
    let index = SymbolIndex::build(&documents, CheckScope::Open, None);
    let imports: Vec<Import> = carried
        .iter()
        .map(|(symbol, written)| Import {
            symbol: (*symbol).to_owned(),
            written: (*written).to_owned(),
        })
        .collect();
    imports::edits(
        &analysed,
        &index,
        u32::try_from(destination.len()).expect("short"),
        &imports,
        PositionEncoding::Utf8,
    )
    .into_iter()
    .map(|edit| {
        (
            edit.range.start.line,
            edit.range.start.character,
            edit.new_text,
        )
    })
    .collect()
}

/// Every place a name is written is carried once, with what it resolved to
/// where it was written: the import's target for `Str`, the namespace's own
/// class for `Cart`. The qualified `Core\Debug` is absolute and is not.
#[test]
fn a_range_carries_every_type_name_it_writes_resolved_at_the_source() {
    assert_eq!(
        carried(SOURCE, "var $n", "dump($n);"),
        vec![
            ("Core\\Str".to_owned(), "Str".to_owned()),
            ("Shop\\Cart".to_owned(), "Cart".to_owned()),
        ]
    );
}

/// A parameter's annotation is a written name like any other, and a range
/// that covers only the class body carries it and nothing outside the range.
#[test]
fn an_annotation_inside_the_range_is_carried_and_a_name_outside_it_is_not() {
    assert_eq!(
        carried(SOURCE, "public function", "return 1; }"),
        vec![("Shop\\Cart".to_owned(), "Cart".to_owned())]
    );
}

/// A file with no `use` and no namespace gets both lines after its open tag,
/// in one edit, sorted by the qualified name.
#[test]
fn a_bare_destination_gets_every_missing_use_line_after_its_open_tag() {
    assert_eq!(
        written("<?nvs\n", &[("Shop\\Cart", "Cart"), ("Core\\Str", "Str")]),
        vec![(0, 5, "\nuse Core\\Str;\nuse Shop\\Cart;".to_owned())]
    );
}

/// A destination where every name already resolves as it was written — the
/// namespace in force reaches one and an existing `use` reaches the other —
/// needs nothing, and the answer is no edit rather than an empty one.
#[test]
fn a_destination_that_already_resolves_every_name_gets_no_edit() {
    assert_eq!(
        written(
            "<?nvs\nnamespace Shop;\n\nuse Core\\Str;\n",
            &[("Shop\\Cart", "Cart"), ("Core\\Str", "Str")]
        ),
        Vec::<(u32, u32, String)>::new()
    );
}

/// A short name the destination already answers to — a class of its own by
/// that name — is not imported over, and the rest still are, after the last
/// `use` the destination has.
#[test]
fn a_taken_short_name_is_skipped_and_the_rest_follow_the_last_use() {
    assert_eq!(
        written(
            "<?nvs\nuse Core\\Arr;\nclass Cart {}\n",
            &[("Shop\\Cart", "Cart"), ("Core\\Str", "Str")]
        ),
        vec![(1, 13, "\nuse Core\\Str;".to_owned())]
    );
}

/// A name the range wrote qualified is carried as written and imported
/// nowhere: it means the same thing in every file.
#[test]
fn a_qualified_name_is_never_imported() {
    assert_eq!(
        written("<?nvs\n", &[("Core\\Debug", "Core\\Debug")]),
        Vec::<(u32, u32, String)>::new()
    );
}
