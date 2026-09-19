//! What a completion item does to a buffer, and when a list is offered at all.
//!
//! Here rather than under `tests/lsp/completion/` because `nvs_lsp::render`
//! freezes a label, a kind and a detail — the three fields a client shows — and
//! everything asserted below is a field a client *acts* on: the text an item
//! replaces, the `use` line it adds elsewhere, the order a tie is broken in, and
//! whether a request a trigger character raised is answered. A case may not
//! invent a rendering for those
//! (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`), so a Rust test holds
//! them, the way `tests/completion.rs` holds a PHP name's `insert_text`.
//!
//! The region half is here too, because it is the same decision seen from the
//! other request: the bytes `nvs/regions` stops reporting as HTML are exactly
//! the ones completion offers the open tags at.

use lsp_types::{CompletionItem, CompletionTextEdit, Position, Range, TextEdit};
use nvs_diagnostics::{PositionEncoding, SourceMap};
use nvs_lsp::{CheckScope, Documents, PhpNames, SymbolIndex, analyse, completion, regions, uri_of};

/// What is offered at the end of `source`, which is where a developer types.
///
/// No document below ends in a newline, on purpose: the cursor at the last
/// byte of a file is outside every half-open span in it, and a list that is
/// only right one line above the end of the file is wrong in every new file.
fn offered(source: &str) -> Vec<CompletionItem> {
    let uri = uri_of(&std::env::temp_dir().join("nvs-completion-edit.nvs"))
        .expect("a temp path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    let analysis = analyse(&documents, &uri).expect("an open document analyses");
    let index = SymbolIndex::build(&documents, CheckScope::Open, None);
    let at = u32::try_from(source.len()).expect("a test document is short");
    completion::at(&analysis, &index, at, PhpNames::Off, PositionEncoding::Utf8)
}

/// Whether a trigger character typed at the end of `source` is answered.
fn triggered(source: &str) -> bool {
    let uri = uri_of(&std::env::temp_dir().join("nvs-completion-trigger.nvs"))
        .expect("a temp path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    let analysis = analyse(&documents, &uri).expect("an open document analyses");
    let at = u32::try_from(source.len()).expect("a test document is short");
    completion::continues_a_trigger(&analysis, at)
}

/// The one item labelled `label`.
fn named<'a>(items: &'a [CompletionItem], label: &str) -> &'a CompletionItem {
    let mut found = items.iter().filter(|item| item.label == label);
    let item = found
        .next()
        .unwrap_or_else(|| panic!("`{label}` is offered"));
    assert!(found.next().is_none(), "`{label}` is offered once");
    item
}

/// An empty range at `line`:`character`, which is what an insertion is.
fn at(line: u32, character: u32) -> Range {
    let position = Position { line, character };
    Range {
        start: position,
        end: position,
    }
}

#[test]
fn a_core_class_is_offered_by_its_short_name_and_accepting_it_writes_the_use_line() {
    let items = offered("<?nvs\nStr");
    let item = named(&items, "Str");
    assert_eq!(item.detail.as_deref(), Some(r"Core\Str"));
    assert_eq!(
        item.filter_text.as_deref(),
        Some(r"Str Core\Str"),
        "both spellings find it, and so do the initials across the separator"
    );
    assert_eq!(
        item.additional_text_edits,
        Some(vec![TextEdit {
            range: at(0, 5),
            new_text: "\nuse Core\\Str;".to_owned(),
        }]),
        "with no `use` and no namespace, the line goes after the open tag"
    );
    assert_eq!(item.insert_text, None, "the label is what is inserted");
}

#[test]
fn the_use_line_joins_the_imports_the_file_already_has() {
    let items = offered("<?nvs\nnamespace App;\n\nuse Core\\Arr;\n\nStr");
    assert_eq!(
        named(&items, "Str").additional_text_edits,
        Some(vec![TextEdit {
            range: at(3, 13),
            new_text: "\nuse Core\\Str;".to_owned(),
        }])
    );
    let imported = named(&items, "Arr");
    assert_eq!(
        imported.additional_text_edits, None,
        "a name already imported is written and nothing else is edited"
    );
    let after_namespace = offered("<?nvs\nnamespace App;\nStr");
    assert_eq!(
        named(&after_namespace, "Str").additional_text_edits,
        Some(vec![TextEdit {
            range: at(1, 14),
            new_text: "\n\nuse Core\\Str;".to_owned(),
        }]),
        "with no `use` yet, the line starts a group of its own under the namespace"
    );
}

#[test]
fn a_short_name_this_file_already_uses_is_offered_qualified_instead() {
    let items = offered("<?nvs\nnamespace App;\nclass Str {}\nS");
    assert_eq!(named(&items, "Str").detail.as_deref(), Some(r"App\Str"));
    let core = named(&items, r"Core\Str");
    assert_eq!(
        core.additional_text_edits, None,
        "importing `Core\\Str` here would take the name `App\\Str` answers to"
    );
}

#[test]
fn after_use_every_type_is_its_qualified_name_and_no_word_is_offered() {
    let items = offered("<?nvs\nuse C");
    let item = named(&items, r"Core\Str");
    assert_eq!(item.additional_text_edits, None);
    assert!(
        items.iter().all(|item| item.label != "class"),
        "a `use` declaration takes a name and no reserved word"
    );
}

#[test]
fn a_tie_is_broken_by_what_this_file_already_reaches() {
    let items = offered("<?nvs\nnamespace App;\nuse Core\\Arr;\nclass Cart {}\nvar $total = 1;\n");
    let tier = |label: &str| {
        named(&items, label)
            .sort_text
            .as_deref()
            .and_then(|text| text.chars().next())
            .expect("a bare position ranks every item")
    };
    let order = [
        tier("$total"),
        tier("Arr"),
        tier("Cart"),
        tier("Str"),
        tier("class"),
    ];
    let mut sorted = order;
    sorted.sort_unstable();
    assert_eq!(
        order, sorted,
        "a local, an import, this namespace's own type, the rest of `Core`, then a word"
    );
    assert!(order.windows(2).all(|pair| pair[0] != pair[1]));
}

#[test]
fn a_variable_replaces_the_dollar_already_typed() {
    for (source, start) in [
        ("<?nvs\nvar $foo = 1;\n$", 0),
        ("<?nvs\nvar $foo = 1;\n$f", 0),
        ("<?nvs\nvar $foo = 1;\nif ($", 4),
    ] {
        let items = offered(source);
        assert_eq!(items.len(), 1, "after `$` only a variable is offered");
        let end =
            u32::try_from(source.lines().last().expect("a last line").len()).expect("a short line");
        assert_eq!(
            items[0].text_edit,
            Some(CompletionTextEdit::Edit(TextEdit {
                range: Range {
                    start: Position {
                        line: 2,
                        character: start,
                    },
                    end: Position {
                        line: 2,
                        character: end,
                    },
                },
                new_text: "$foo".to_owned(),
            })),
            "`{source}`: the `$` is replaced, so accepting never writes `$$foo`"
        );
    }
}

#[test]
fn a_trigger_character_is_answered_only_where_it_finished_a_spelling() {
    for source in [
        "<?nvs\n$u->",
        "<?nvs\nCore\\Str::",
        "<?nvs\nCore\\",
        "<?nvs\n$",
        "<p>\n<?",
    ] {
        assert!(
            triggered(source),
            "`{source}` ends a spelling a list follows"
        );
    }
    for source in [
        "<?nvs\nCore\\Str:",
        "<?nvs\nvar $b = $a >",
        "<?nvs\nvar $b = $a ?",
        "<?nvs\nvar $b = $a ? 1 :",
    ] {
        assert!(!triggered(source), "`{source}` ends in an operator");
    }
}

#[test]
fn a_half_written_open_tag_is_not_reported_as_html() {
    let source = "<p>one</p>\n<?\n<p>two</p>\n";
    let mut map = SourceMap::new();
    let id = map.add("regions.nvs", source);
    let found = regions::for_source(map.file(id), PositionEncoding::Utf8);
    let ranges: Vec<(u32, u32, u32, u32)> = found
        .iter()
        .map(|region| {
            let range = region.range;
            (
                range.start.line,
                range.start.character,
                range.end.line,
                range.end.character,
            )
        })
        .collect();
    assert_eq!(
        ranges,
        [(0, 0, 1, 0), (2, 0, 3, 0)],
        "the run is cut around `<?` and the character after it, so a cursor \
         still typing the tag is in no region a client forwards"
    );
    let xml = "<?xml version=\"1.0\"?>\n<p>one</p>\n";
    let id = map.add("xml.nvs", xml);
    assert_eq!(
        regions::for_source(map.file(id), PositionEncoding::Utf8).len(),
        1,
        "a processing instruction is markup's own and is not cut"
    );
}
