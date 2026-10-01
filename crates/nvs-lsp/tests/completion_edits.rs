//! What a completion item does to a buffer, and when a list is offered at all.
//!
//! Here rather than under `tests/lsp/completion/` because `nvs_lsp::render`
//! freezes the row a client shows — the label, what is written after it, the
//! kind, the text at the right — and everything asserted below is a field a
//! client *acts* on or asks for later: the text an item replaces, the `use`
//! line it adds elsewhere, the order a tie is broken in, whether a request a
//! trigger character raised is answered, and the card `completionItem/resolve`
//! fills in. A case may not invent a rendering for those
//! (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`), so a Rust test holds
//! them, the way `tests/completion.rs` holds a PHP name's `insert_text`.
//!
//! The region half is here too, because it is the same decision seen from the
//! other request: the bytes `nvs/regions` stops reporting as HTML are exactly
//! the ones completion offers the open tags at.

use lsp_types::{
    CompletionItem, CompletionTextEdit, Documentation, InsertTextFormat, Position, Range, TextEdit,
};
use nvs_diagnostics::{PositionEncoding, SourceMap};
use nvs_lsp::{
    CheckScope, Client, Documents, PhpNames, SymbolIndex, analyse, card, completion, regions,
    uri_of,
};
use nvs_stdlib::registry;

/// What is offered at the end of `source`, which is where a developer types.
///
/// No document below ends in a newline, on purpose: the cursor at the last
/// byte of a file is outside every half-open span in it, and a list that is
/// only right one line above the end of the file is wrong in every new file.
fn offered(source: &str) -> Vec<CompletionItem> {
    offered_to(Client::default(), source)
}

/// A client that places the cursor and runs both of the editor's commands.
const EDITOR: Client = Client {
    snippets: true,
    suggest: true,
    parameter_hints: true,
};

/// The mark a document below writes where its cursor is.
const CURSOR: &str = "<|>";

/// What `client` is offered at the [`CURSOR`] in `source`, or at its end
/// where it writes none.
fn offered_to(client: Client, source: &str) -> Vec<CompletionItem> {
    offered_in(
        &std::env::temp_dir().join("nvs-completion-edit.nvs"),
        client,
        source,
    )
}

/// [`offered_to`], with the buffer opened under `path`. Nothing is written
/// there: the path only says which directory the document sits in.
fn offered_in(path: &std::path::Path, client: Client, source: &str) -> Vec<CompletionItem> {
    let cursor = source.find(CURSOR).unwrap_or(source.len());
    let source = source.replacen(CURSOR, "", 1);
    let uri = uri_of(path).expect("a test path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source);
    let analysis = analyse(&documents, &uri).expect("an open document analyses");
    let index = SymbolIndex::build(&documents, CheckScope::Open, None);
    let at = u32::try_from(cursor).expect("a test document is short");
    completion::at(
        &analysis,
        &index,
        at,
        PhpNames::Off,
        client,
        PositionEncoding::Utf8,
    )
}

/// What accepting the item labelled `label` writes, and the command it runs.
fn accepting(items: &[CompletionItem], label: &str) -> (String, Option<String>) {
    let item = named(items, label);
    (
        item.insert_text.clone().unwrap_or_else(|| label.to_owned()),
        item.command.as_ref().map(|command| command.command.clone()),
    )
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

/// A class-reference literal's item is a class's whole name, and it replaces
/// all the text between the opening quote and the cursor, separators included.
#[test]
fn a_class_name_item_replaces_all_the_text_written() {
    let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("nvs-class-case.nvs");
    let source = "<?nvs\nnamespace App;\nclass User { }\n$c = 'App\\U<|>' as class<User>;";
    let items = offered_in(&here, EDITOR, source);
    assert_eq!(
        named(&items, "App\\User").text_edit,
        Some(CompletionTextEdit::Edit(TextEdit {
            range: Range {
                start: Position {
                    line: 3,
                    character: 6,
                },
                end: Position {
                    line: 3,
                    character: 11,
                },
            },
            new_text: "App\\User".to_owned(),
        }))
    );
}

/// A directory in a path literal replaces only the segment being written,
/// writes a `/` after its name, and opens the list again for the next segment.
#[test]
fn a_path_item_replaces_the_segment_being_written() {
    let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("nvs-path-case.nvs");
    let items = offered_in(&here, EDITOR, "<?nvs\nrequire '../nvs-l<|>';");
    let directory = named(&items, "nvs-lsp/");
    let segment = Range {
        start: Position {
            line: 1,
            character: 12,
        },
        end: Position {
            line: 1,
            character: 17,
        },
    };
    assert_eq!(
        directory.text_edit,
        Some(CompletionTextEdit::Edit(TextEdit {
            range: segment,
            new_text: "nvs-lsp/".to_owned(),
        }))
    );
    assert_eq!(
        directory
            .command
            .as_ref()
            .map(|command| command.command.as_str()),
        Some(Client::SUGGEST)
    );

    // The crate's directory holds `Cargo.toml` beside `src` and `tests`, and a
    // root names a directory, so the file is not offered.
    let items = offered_in(&here, EDITOR, "<?nvs\nautoload 'App' from 'sr<|>';");
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(
        labels,
        ["src/", "tests/"],
        "a root offers directories alone"
    );
}

/// A literal at a path parameter is offered every file as well as each
/// directory, and an absolute one lists the directory it names whatever
/// directory the document sits in.
#[test]
fn a_path_argument_offers_every_file_and_an_absolute_one_completes_from_itself() {
    let crate_dir = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    let elsewhere = std::env::temp_dir().join("nvs-path-argument-case.nvs");
    let source = format!("<?nvs\necho Core\\IO::read('{crate_dir}/<|>');");
    let items = offered_in(&elsewhere, EDITOR, &source);
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
        labels.contains(&"Cargo.toml") && labels.contains(&"src/"),
        "an absolute literal lists the directory it names, files included: {labels:?}"
    );

    // The same text at a parameter that is not a path is a string like any
    // other, and no entry of the directory is offered.
    let source = format!("<?nvs\necho Core\\Str::length('{crate_dir}/<|>');");
    let items = offered_in(&elsewhere, EDITOR, &source);
    assert!(
        items.iter().all(|item| item.label != "Cargo.toml"),
        "a plain parameter offers no file"
    );
}

/// `'`, `"` and `/` raise a request inside a literal at a path or class-name
/// parameter, and inside one at a plain parameter they do not.
#[test]
fn a_quote_or_slash_at_a_marked_parameter_is_answered() {
    for (source, answered) in [
        ("<?nvs\necho Core\\IO::read('<|>');", true),
        ("<?nvs\necho Core\\IO::read(\"<|>\");", true),
        ("<?nvs\necho Core\\IO::read('data/<|>');", true),
        ("<?nvs\n$info = Core\\Reflect::forClass('<|>');", true),
        ("<?nvs\necho Core\\Str::length('<|>');", false),
        ("<?nvs\necho Core\\Str::length('data/<|>');", false),
    ] {
        let cursor = source.find(CURSOR).expect("a cursor is written");
        let text = source.replacen(CURSOR, "", 1);
        let uri = uri_of(&std::env::temp_dir().join("nvs-completion-trigger-argument.nvs"))
            .expect("a temp path is UTF-8");
        let mut documents = Documents::new();
        documents.open(uri.clone(), 1, text);
        let analysis = analyse(&documents, &uri).expect("an open document analyses");
        let at = u32::try_from(cursor).expect("a test document is short");
        assert_eq!(
            completion::continues_a_trigger(&analysis, at),
            answered,
            "`{source}`"
        );
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

/// Two classes, an interface and an enum, which is every kind `new` is asked
/// about.
const TYPES: &str = "<?nvs
interface Shape {}
enum Colour { case Red; }
abstract class Base { public function constructor(int $sides) {} }
class Square extends Base {}
class Point { public function constructor() {} }
class Origin extends Point {}
";

#[test]
fn a_type_in_an_expression_writes_its_scope_and_opens_the_list_again() {
    for written in [
        "var $a = ",
        "echo ",
        "return ",
        "foo(",
        "foo(1, ",
        "var $a = [",
        "var $a = $b ?? ",
        "var $a = $b ? ",
        "var $a = 1 | ",
        "if (",
    ] {
        let items = offered_to(EDITOR, &format!("{TYPES}{written}"));
        for label in ["Square", "Shape", "Colour", "Str"] {
            assert_eq!(
                accepting(&items, label),
                (format!("{label}::"), Some(Client::SUGGEST.to_owned())),
                "`{written}` is followed by a value, and `{label}` there is a receiver"
            );
        }
    }
}

#[test]
fn a_type_is_written_alone_where_a_type_or_a_statement_may_start() {
    for written in [
        "",
        "?",
        "tainted ",
        "function f(",
        "function f(int $a, ",
        "function f(?",
        "function f(int|",
        "function f(tainted ",
        "function f(): ",
        "function f(int $a): ?",
        "function f<T>(",
        "var $f = fn(",
        "try {} catch (",
        "class A extends ",
        "class A implements Shape, ",
        "var $a = $b is ",
        "var $a = $b as ?",
        "#[",
        "var $a = 1 < ",
        "class ",
    ] {
        let items = offered_to(EDITOR, &format!("{TYPES}{written}"));
        assert_eq!(
            accepting(&items, "Square"),
            ("Square".to_owned(), None),
            "`{written}` may be followed by a type, so the name is the whole of it"
        );
    }
}

#[test]
fn a_written_type_is_offered_types_and_no_word_or_variable() {
    let items = offered_to(
        EDITOR,
        &format!("{TYPES}var $total = 1;\nfunction f(int $a, "),
    );
    assert!(items.iter().any(|item| item.label == "Square"));
    assert!(
        items
            .iter()
            .all(|item| item.label != "match" && item.label != "$total"),
        "a parameter's type is neither an expression nor a variable"
    );
}

#[test]
fn new_offers_what_it_compiles_on_and_writes_the_call() {
    let items = offered_to(EDITOR, &format!("{TYPES}var $a = new "));
    for refused in ["Shape", "Colour", "Base", "Str"] {
        assert!(
            items.iter().all(|item| item.label != refused),
            "`new {refused}` does not compile"
        );
    }
    for (label, written) in [
        ("Square", "Square($0)"),
        ("Point", "Point()"),
        ("Origin", "Origin()"),
    ] {
        assert_eq!(
            named(&items, label).insert_text.as_deref(),
            Some(written),
            "the cursor is inside the parentheses only where an argument goes"
        );
    }
    let square = named(&items, "Square");
    assert_eq!(square.insert_text_format, Some(InsertTextFormat::SNIPPET));
    assert_eq!(
        square
            .command
            .as_ref()
            .map(|command| command.command.as_str()),
        Some(Client::PARAMETER_HINTS)
    );
    let point = named(&items, "Point");
    assert_eq!(point.insert_text_format, None);
    assert_eq!(point.command, None);
    let (constructed, _) = nvs_stdlib::registry::CONSTRUCTORS[0];
    let short = constructed
        .rsplit('\\')
        .next()
        .expect("a name has a last segment");
    assert_eq!(
        named(&items, short)
            .additional_text_edits
            .as_ref()
            .map(Vec::len),
        Some(1),
        "a `Core` class with a constructor is offered, and still imports itself"
    );
}

#[test]
fn a_qualified_name_in_a_snippet_keeps_its_separators() {
    let source = "<?nvs\nnamespace App;\nclass Str { public function constructor(int $a) {} }\n";
    let items = offered_to(EDITOR, &format!("{source}var $a = new App\\"));
    assert_eq!(
        named(&items, "Str").insert_text.as_deref(),
        Some("Str($0)"),
        "after a separator the label is the rest of the name"
    );
    let items = offered_to(
        EDITOR,
        &format!("{source}namespace Other;\nclass Str {{}}\nvar $a = new S"),
    );
    assert_eq!(
        named(&items, r"App\Str").insert_text.as_deref(),
        Some(r"App\\Str($0)"),
        "a snippet reads a lone `\\` as an escape"
    );
}

#[test]
fn a_class_word_is_a_receiver_inside_an_expression_and_a_call_after_new() {
    let inside = |written: &str| {
        offered_to(
            EDITOR,
            &format!(
                "{TYPES}class Last extends Point {{ public function f() {{ {written}{CURSOR} }} }}"
            ),
        )
    };
    let items = inside("return ");
    for word in ["self", "static", "parent"] {
        assert_eq!(
            accepting(&items, word),
            (format!("{word}::"), Some(Client::SUGGEST.to_owned()))
        );
    }
    let items = inside("");
    assert_eq!(accepting(&items, "static"), ("static".to_owned(), None));
    assert_eq!(accepting(&items, "self"), ("self".to_owned(), None));
    assert_eq!(
        accepting(&items, "parent"),
        ("parent::".to_owned(), Some(Client::SUGGEST.to_owned())),
        "`parent` opens no statement of its own"
    );
    let items = inside("return new ");
    assert_eq!(
        named(&items, "static").insert_text.as_deref(),
        Some("static($0)")
    );
}

#[test]
fn what_already_follows_the_cursor_is_not_written_twice() {
    let items = offered_to(EDITOR, &format!("{TYPES}var $a = new Squ{CURSOR} (4);"));
    assert_eq!(accepting(&items, "Square"), ("Square".to_owned(), None));
    let items = offered_to(EDITOR, &format!("{TYPES}var $a = new Squ{CURSOR}are(4);"));
    assert_eq!(
        accepting(&items, "Square"),
        ("Square".to_owned(), None),
        "the rest of the name the cursor is inside is read past"
    );
}

#[test]
fn a_client_that_named_nothing_gets_plain_text_and_no_command() {
    let items = offered(&format!("{TYPES}var $a = "));
    assert_eq!(accepting(&items, "Square"), ("Square::".to_owned(), None));
    let items = offered(&format!("{TYPES}var $a = new "));
    let square = named(&items, "Square");
    assert_eq!(square.insert_text.as_deref(), Some("Square()"));
    assert_eq!(square.insert_text_format, None);
    assert_eq!(square.command, None);
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

/// A class with a `///` run above it and above one of its members, for the
/// rows and the cards below.
const DOCUMENTED: &str = "<?nvs\n/// A person who can be greeted.\nclass User {\n    public const int LIMIT = 10;\n    public string $name;\n    /// Greets `$to`.\n    public function greet(string $to): string { return $to; }\n    public function ping(): void {}\n}\nvar $u = new User();\n";

#[test]
fn a_method_is_accepted_as_a_call() {
    let items = offered_to(EDITOR, &format!("{DOCUMENTED}$u->"));
    assert_eq!(
        accepting(&items, "greet"),
        (
            "greet($0)".to_owned(),
            Some(Client::PARAMETER_HINTS.to_owned())
        ),
        "a method with a parameter leaves the cursor inside, with signature help open"
    );
    assert_eq!(
        named(&items, "greet").insert_text_format,
        Some(InsertTextFormat::SNIPPET)
    );
    assert_eq!(
        accepting(&items, "ping"),
        ("ping()".to_owned(), None),
        "a method with no parameter is written whole"
    );
    assert_eq!(
        accepting(&items, "name"),
        ("name".to_owned(), None),
        "a property is not a call"
    );
    let plain = offered(&format!("{DOCUMENTED}$u->"));
    assert_eq!(
        accepting(&plain, "greet"),
        ("greet()".to_owned(), None),
        "a client without snippets gets the parentheses as text"
    );
    let written = offered_to(EDITOR, &format!("{DOCUMENTED}$u->gre{CURSOR}()"));
    assert_eq!(
        accepting(&written, "greet"),
        ("greet".to_owned(), None),
        "a `(` already written is not written twice"
    );
    let core = offered_to(EDITOR, "<?nvs\nCore\\Str::");
    assert_eq!(accepting(&core, "length").0, "length($0)");
}

#[test]
fn a_row_writes_the_signature_after_the_name_and_the_type_at_the_right() {
    let items = offered(&format!("{DOCUMENTED}$u->"));
    let greet = named(&items, "greet");
    let details = greet
        .label_details
        .as_ref()
        .expect("a method carries label details");
    assert_eq!(details.detail.as_deref(), Some("(string $to)"));
    assert_eq!(details.description.as_deref(), Some("string"));
    assert_eq!(
        greet.detail.as_deref(),
        Some("User::greet(string $to): string"),
        "the detail is the qualified signature a client heads the card with"
    );
    let name = named(&items, "name")
        .label_details
        .as_ref()
        .expect("a property carries label details");
    assert_eq!(
        (name.detail.as_deref(), name.description.as_deref()),
        (None, Some("string"))
    );
    let statics = offered(&format!("{DOCUMENTED}User::"));
    let limit = named(&statics, "LIMIT")
        .label_details
        .as_ref()
        .expect("a constant carries label details");
    assert_eq!(
        (limit.detail.as_deref(), limit.description.as_deref()),
        (Some(" = 10"), Some("int"))
    );
    let types = offered("<?nvs\nSt");
    let str = named(&types, "Str");
    assert_eq!(
        str.label_details
            .as_ref()
            .and_then(|details| details.detail.as_deref()),
        Some(" [Core]"),
        "a type offered by a short name says where it is declared"
    );
    assert_eq!(str.detail.as_deref(), Some(r"Core\Str"));
}

/// The card `completionItem/resolve` fills in for the item labelled `label`
/// at the [`CURSOR`] in `source`, or at its end.
fn card_of(source: &str, label: &str) -> Option<String> {
    let cursor = source.find(CURSOR).unwrap_or(source.len());
    let source = source.replacen(CURSOR, "", 1);
    let uri = uri_of(&std::env::temp_dir().join("nvs-completion-card.nvs"))
        .expect("a temp path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source);
    let analysis = analyse(&documents, &uri).expect("an open document analyses");
    let index = SymbolIndex::build(&documents, CheckScope::Open, None);
    let at = u32::try_from(cursor).expect("a test document is short");
    let items = card::keyed_to(
        completion::at(
            &analysis,
            &index,
            at,
            PhpNames::Off,
            Client::default(),
            PositionEncoding::Utf8,
        ),
        &uri,
    );
    let item = card::resolve(&documents, named(&items, label).clone());
    match item.documentation {
        Some(Documentation::MarkupContent(content)) => Some(content.value),
        Some(Documentation::String(text)) => Some(text),
        None => None,
    }
}

#[test]
fn a_list_carries_keys_and_resolving_an_item_reads_its_card() {
    let list = offered(&format!("{DOCUMENTED}$u->"));
    assert!(
        list.iter().all(|item| item.documentation.is_none()),
        "no card travels with the list"
    );
    assert_eq!(
        card_of(&format!("{DOCUMENTED}$u->"), "greet").as_deref(),
        Some("Greets `$to`."),
        "a member's `///` run is its card"
    );
    assert_eq!(
        card_of(&format!("{DOCUMENTED}$u->"), "ping"),
        None,
        "a member nobody documented resolves to no card"
    );
    assert_eq!(
        card_of(&format!("{DOCUMENTED}Us"), "User").as_deref(),
        Some("A person who can be greeted."),
        "a type's `///` run is its card"
    );
    let length = registry::class(r"Core\Str")
        .and_then(|class| class.members().find(|row| row.name == "length"))
        .and_then(|row| row.doc)
        .expect("Core\\Str::length carries a card");
    let card = card_of("<?nvs\nCore\\Str::", "length")
        .expect("a `Core` member resolves to its reference card");
    assert!(
        card.starts_with(length.short),
        "the card opens with the row's `short`"
    );
    let order = registry::core_enum(r"Core\Order")
        .and_then(|core| core.doc)
        .expect("Core\\Order carries a card");
    let asc = order
        .cases
        .iter()
        .find(|case| case.name == "Asc")
        .map_or(order.short, |case| case.desc);
    assert_eq!(
        card_of("<?nvs\nCore\\Order::", "Asc").as_deref(),
        Some(asc),
        "a case resolves to its own line, or its enum's"
    );
}
