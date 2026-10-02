//! A string argument at a parameter a completion file names offers the file's values, from the
//! attachments whose `when` the call meets and one segment at a time where a list has a
//! separator, and each item changes only that string. A literal equal to a value hovers as that
//! value, and one that is no value at a `strict` parameter is warned on
//! (`rule:ide/completion-files-offer-values-at-named-parameters`). A parameter typed as a union of
//! string literals offers its members with no file, and merged with a file's values
//! (`rule:types/literal-types`).
//!
//! Each test writes a workspace of its own under cargo's scratch folder in `target/` and deletes it
//! when it ends, as `tests/completion_files.rs` does.

use std::fs;
use std::path::{Path, PathBuf};

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemTag, CompletionTextEdit, DiagnosticSeverity,
    DiagnosticTag, Documentation, HoverContents, NumberOrString, Position, Range,
};
use nvs_diagnostics::{PositionEncoding, code};
use nvs_lsp::completion_files::CompletionFiles;
use nvs_lsp::{
    Analysed, CheckScope, Client, Documents, Phases, PhpNames, SymbolIndex, actions, analyse,
    completion, hover, uri_of,
};

/// A scratch workspace that deletes itself.
struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new(name: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("completion-values-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("a scratch directory");
        Self { root }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::create_dir_all(path.parent().expect("a file sits in a folder")).expect("a folder");
        fs::write(&path, text).expect("a writable scratch file");
        path
    }

    fn load(&self) -> CompletionFiles {
        CompletionFiles::load(std::slice::from_ref(&self.root))
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// The classes every document below calls.
const CLASSES: &str = "<?nvs\nnamespace App\\Ui;\n\nclass Icon {\n    \
     public static function render(string $name, int $size, string $style = 'solid'): string {\n        \
     return $name . $style . $size;\n    }\n}\n\nclass Label {\n    \
     public static function text(string $key): string {\n        \
     return $key;\n    }\n}\n\n";

/// Values for `$name`, and nothing for `$style`.
const ICONS: &str = r#"{
  "sets": {
    "icons": [
      "home",
      {
        "value": "arrow-left",
        "label": "Arrow left",
        "labelDetail": " (outline)",
        "labelDescription": "navigation",
        "kind": "constant",
        "title": "Left arrow",
        "documentation": "Points back. ![preview](completion.rs) [Guide](https://example.com/icons)",
        "deprecated": true,
        "sortText": "0"
      }
    ]
  },
  "parameters": [
    { "method": "App\\Ui\\Icon::render", "parameter": "name", "set": "icons", "values": ["it's", "cost$"] }
  ]
}"#;

/// The mark a document below writes where its cursor is.
const CURSOR: &str = "<|>";

/// The list offered at the [`CURSOR`] in `call`, written after [`CLASSES`] in a document in
/// `workspace`, with the byte offset of the cursor in the whole text.
fn offered(
    workspace: &Workspace,
    files: &CompletionFiles,
    call: &str,
) -> (Vec<CompletionItem>, bool) {
    let (documents, analysis, at) = opened(workspace, call);
    let index = SymbolIndex::build(&documents, CheckScope::Open, None);
    let items = completion::at(
        &analysis,
        &index,
        files,
        at,
        PhpNames::Off,
        Client::default(),
        PositionEncoding::Utf8,
    );
    (items, completion::continues_a_trigger(&analysis, files, at))
}

/// `call`, written after [`CLASSES`] in a document in `workspace` with its [`CURSOR`] taken out,
/// opened and analysed, and the byte offset of the cursor in the whole text.
fn opened(workspace: &Workspace, call: &str) -> (Documents, Analysed, u32) {
    let source = format!("{CLASSES}{call}");
    let cursor = source.find(CURSOR).expect("the call marks its cursor");
    let source = source.replacen(CURSOR, "", 1);
    let path = workspace.write("src/page.nvs", &source);
    let uri = uri_of(&path).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source);
    let analysis = analyse(&documents, &uri).expect("an open document analyses");
    let at = u32::try_from(cursor).expect("a test document is short");
    (documents, analysis, at)
}

/// The Markdown a hover at the [`CURSOR`] in `call` answers with, and the range it underlines.
fn hovered(workspace: &Workspace, files: &CompletionFiles, call: &str) -> Option<(String, Range)> {
    let (_, analysis, at) = opened(workspace, call);
    hover::at(&analysis, files, at, PositionEncoding::Utf8).map(|hover| {
        let HoverContents::Markup(markup) = hover.contents else {
            panic!("a hover is Markdown");
        };
        (
            markup.value,
            hover.range.expect("a hover underlines its node"),
        )
    })
}

/// The one item labelled `label`.
fn named<'a>(items: &'a [CompletionItem], label: &str) -> &'a CompletionItem {
    items
        .iter()
        .find(|item| item.label == label)
        .unwrap_or_else(|| panic!("no item `{label}` among {items:?}"))
}

/// The text the item `label` writes, and the range it replaces.
fn edit(items: &[CompletionItem], label: &str) -> (String, Range) {
    match &named(items, label).text_edit {
        Some(CompletionTextEdit::Edit(edit)) => (edit.new_text.clone(), edit.range),
        other => panic!("`{label}` replaces no range: {other:?}"),
    }
}

/// The range of the text between the quotes that open at byte column `open` on the call's line,
/// `len` bytes long.
fn between(open: u32, len: u32) -> Range {
    let line = u32::try_from(CLASSES.lines().count()).expect("short classes");
    Range::new(
        Position::new(line, open + 1),
        Position::new(line, open + 1 + len),
    )
}

/// A workspace named `name` with [`ICONS`] as its one completion file, the file's path, and the
/// files loaded.
fn icons(name: &str) -> (Workspace, PathBuf, CompletionFiles) {
    let workspace = Workspace::new(name);
    let file = workspace.write(".novis/completion/icons.json", ICONS);
    let files = workspace.load();
    (workspace, file, files)
}

/// The byte column the first argument's quote opens at in `Icon::render(…)`.
fn render_open() -> u32 {
    u32::try_from("Icon::render(".len()).expect("short")
}

#[test]
fn a_string_argument_at_a_named_parameter_offers_the_files_values() {
    let (workspace, _, files) = icons("offers");

    // Every value, the set's first and the attachment's own after it.
    let (items, _) = offered(&workspace, &files, "Icon::render('ho<|>me', 16, 'solid');");
    assert_eq!(labels(&items), ["Arrow left", "cost$", "home", "it's"]);

    // A parameter no file names is ordinary text.
    let (items, _) = offered(&workspace, &files, "Icon::render('home', 16, '<|>');");
    assert!(
        !items.iter().any(|item| item.label == "home"),
        "no value at `$style`"
    );

    // Without the file, the same argument offers no value either.
    let (items, _) = offered(
        &workspace,
        &CompletionFiles::default(),
        "Icon::render('<|>', 16, 'solid');",
    );
    assert!(
        !items.iter().any(|item| item.label == "home"),
        "no value without the file"
    );
}

/// Three classes that share one method: `Child` inherits it from `Base`, and `Own` overrides it.
const FAMILY: &str = "class Base {\n    \
     public static function pick(string $name): string {\n        return $name;\n    }\n}\n\n\
     class Child extends Base {}\n\n\
     class Own extends Base {\n    \
     public static function pick(string $name): string {\n        return $name;\n    }\n}\n\n";

/// One value for each of the three classes' `pick`.
const PICKS: &str = r#"{
  "parameters": [
    { "method": "App\\Ui\\Base::pick", "parameter": "name", "values": ["base"] },
    { "method": "App\\Ui\\Child::pick", "parameter": "name", "values": ["child"] },
    { "method": "App\\Ui\\Own::pick", "parameter": "name", "values": ["own"] }
  ]
}"#;

#[test]
fn an_inherited_method_is_reached_through_its_declaring_class() {
    let workspace = Workspace::new("inherited");
    workspace.write(".novis/completion/picks.json", PICKS);
    let files = workspace.load();
    for (call, want) in [
        ("Base::pick('<|>');", ["base"]),
        ("Child::pick('<|>');", ["base"]),
        ("Own::pick('<|>');", ["own"]),
    ] {
        let (items, _) = offered(&workspace, &files, &format!("{FAMILY}{call}"));
        assert_eq!(labels(&items), want, "{call}");
    }
}

#[test]
fn an_item_carries_every_field_the_file_gives() {
    let (workspace, _, files) = icons("fields");
    let (items, _) = offered(&workspace, &files, "Icon::render('<|>', 16, 'solid');");
    let arrow = named(&items, "Arrow left");
    assert_eq!(arrow.kind, Some(CompletionItemKind::CONSTANT));
    assert_eq!(arrow.detail.as_deref(), Some("Left arrow"));
    let details = arrow.label_details.as_ref().expect("label details");
    assert_eq!(details.detail.as_deref(), Some(" (outline)"));
    assert_eq!(details.description.as_deref(), Some("navigation"));
    assert_eq!(arrow.tags, Some(vec![CompletionItemTag::DEPRECATED]));
    assert_eq!(arrow.sort_text.as_deref(), Some("0"));
    assert_eq!(arrow.filter_text.as_deref(), Some("arrow-left"));
    assert!(matches!(
        arrow.documentation,
        Some(Documentation::MarkupContent(_))
    ));
    assert_eq!(named(&items, "home").kind, Some(CompletionItemKind::VALUE));
}

#[test]
fn an_item_replaces_the_string_and_escapes_for_its_quote() {
    let (workspace, _, files) = icons("escapes");
    let open = render_open();

    // Single-quoted: each item replaces the whole text between the quotes, and only that.
    let (items, _) = offered(&workspace, &files, "Icon::render('ho<|>me', 16, 'solid');");
    for (label, written) in [
        ("home", "home"),
        ("Arrow left", "arrow-left"),
        ("it's", "it\\'s"),
        ("cost$", "cost$"),
    ] {
        assert_eq!(edit(&items, label), (written.to_owned(), between(open, 4)));
    }

    // Double-quoted: the escapes are that quote's, and an empty literal is replaced too.
    let (items, _) = offered(&workspace, &files, "Icon::render(\"<|>\", 16, 'solid');");
    assert_eq!(edit(&items, "it's"), ("it's".to_owned(), between(open, 0)));
    assert_eq!(
        edit(&items, "cost$"),
        ("cost\\$".to_owned(), between(open, 0))
    );
}

#[test]
fn an_item_carries_no_command_and_no_other_edit() {
    let (workspace, _, files) = icons("no-command");
    let (items, _) = offered(&workspace, &files, "Icon::render('<|>', 16, 'solid');");
    assert!(!items.is_empty());
    for item in &items {
        let label = &item.label;
        assert!(item.command.is_none(), "`{label}` runs nothing");
        assert!(
            item.additional_text_edits.is_none(),
            "`{label}` writes nothing else"
        );
        assert!(
            item.insert_text.is_none() && item.insert_text_format.is_none(),
            "`{label}` is no snippet"
        );
    }
}

#[test]
fn a_relative_link_in_documentation_resolves_against_the_completion_file() {
    let (workspace, file, files) = icons("links");
    let (items, _) = offered(&workspace, &files, "Icon::render('<|>', 16, 'solid');");
    let arrow = named(&items, "Arrow left");
    let Some(Documentation::MarkupContent(markup)) = &arrow.documentation else {
        panic!("the documentation is Markdown: {:?}", arrow.documentation);
    };
    let image = uri_of(&file.parent().expect("a folder").join("completion.rs"))
        .expect("a scratch path is UTF-8");
    assert_eq!(
        markup.value,
        format!(
            "Points back. ![preview]({}) [Guide](https://example.com/icons)",
            image.as_str()
        )
    );
}

#[test]
fn a_quote_opens_the_list_at_a_named_parameter() {
    let (workspace, _, files) = icons("quote");
    for quoted in [
        "Icon::render('<|>', 16, 'solid');",
        "Icon::render(\"<|>\", 16, 'solid');",
    ] {
        let (_, triggered) = offered(&workspace, &files, quoted);
        assert!(
            triggered,
            "a quote at a named parameter opens the list: {quoted}"
        );
    }

    // A parameter no file names is ordinary text, and a quote opens nothing there.
    let (_, triggered) = offered(&workspace, &files, "Icon::render('home', 16, '<|>');");
    assert!(!triggered);

    // Without the file, the quote opens nothing at `$name` either.
    let (_, triggered) = offered(
        &workspace,
        &CompletionFiles::default(),
        "Icon::render('<|>', 16, 'solid');",
    );
    assert!(!triggered);
}

/// A method whose parameter is a path.
const LOADER: &str = "class Loader {\n    \
     public static function load(#[Core\\Path] string $file): string {\n        return $file;\n    }\n}\n\n";

#[test]
fn a_path_parameter_with_a_completion_file_offers_both_lists() {
    let workspace = Workspace::new("path");
    workspace.write(
        ".novis/completion/loader.json",
        r#"{ "parameters": [ { "method": "App\\Ui\\Loader::load", "parameter": "file", "values": ["@theme/page.html"] } ] }"#,
    );
    let files = workspace.load();
    let (items, triggered) = offered(&workspace, &files, &format!("{LOADER}Loader::load('<|>');"));
    assert!(triggered, "a quote at a path parameter opens the list");
    assert_eq!(labels(&items), ["@theme/page.html", "page.nvs"]);
    assert_eq!(
        named(&items, "@theme/page.html").kind,
        Some(CompletionItemKind::VALUE)
    );
    assert_eq!(
        named(&items, "page.nvs").kind,
        Some(CompletionItemKind::FILE)
    );
}

#[test]
fn hover_on_a_literal_equal_to_a_value_shows_its_title_and_documentation() {
    let (workspace, file, files) = icons("hover");
    let image = uri_of(&file.parent().expect("a folder").join("completion.rs"))
        .expect("a scratch path is UTF-8");
    let open = render_open();
    let literal = "'arrow-left'";
    let call = "Icon::render('arrow-<|>left', 16, 'solid');";
    let line = between(open, 0).start.line;
    assert_eq!(
        hovered(&workspace, &files, call),
        Some((
            format!(
                "Left arrow\n\nPoints back. ![preview]({}) [Guide](https://example.com/icons)",
                image.as_str()
            ),
            Range::new(
                Position::new(line, open),
                Position::new(line, open + u32::try_from(literal.len()).expect("short")),
            ),
        ))
    );

    // A literal that is no value, and the same literal with no file loaded, show neither field.
    for (files, call) in [
        (&files, "Icon::render('arrow-<|>right', 16, 'solid');"),
        (&CompletionFiles::default(), call),
    ] {
        let shown = hovered(&workspace, files, call).map(|(markdown, _)| markdown);
        assert!(
            !shown
                .as_deref()
                .is_some_and(|markdown| markdown.contains("Left arrow")),
            "{call}: {shown:?}"
        );
    }
}

/// Two values for `$name`: one names the line that defines it, and one names nothing.
const LOCATED: &str = r#"{
  "parameters": [
    {
      "method": "App\\Ui\\Icon::render",
      "parameter": "name",
      "values": [{ "value": "home", "location": { "file": "icons/home.svg", "line": 3 } }, "star"]
    }
  ]
}"#;

/// Where go-to-definition at the [`CURSOR`] in `call` goes, with [`LOCATED`] loaded.
fn defined(name: &str, call: &str) -> (PathBuf, Option<nvs_lsp::definition::Declared>) {
    let workspace = Workspace::new(name);
    let file = workspace.write(".novis/completion/located.json", LOCATED);
    let svg = workspace.write(
        ".novis/completion/icons/home.svg",
        "<svg>\n\n<path/>\n</svg>\n",
    );
    assert_eq!(svg, file.parent().expect("a folder").join("icons/home.svg"));
    let files = workspace.load();
    let (_, analysis, at) = opened(&workspace, call);
    let declared = nvs_lsp::definition::at(&analysis, &files, at, PositionEncoding::Utf8);
    (svg, declared)
}

#[test]
fn definition_on_a_literal_equal_to_a_value_goes_to_its_location() {
    let (svg, declared) = defined("definition", "Icon::render('ho<|>me', 16, 'solid');");
    let declared = declared.expect("`home` names its location");
    assert_eq!(declared.path, svg);
    assert_eq!(
        declared.range,
        Range::new(Position::new(2, 0), Position::new(2, 0))
    );
}

#[test]
fn a_value_without_a_location_has_no_definition() {
    let (_, declared) = defined("no-location", "Icon::render('st<|>ar', 16, 'solid');");
    assert_eq!(declared, None);
}

/// The ranges of the unknown-value warnings in the document `call` makes, with `file` as the one
/// completion file of a workspace named `name`.
fn warned(name: &str, file: &str, call: &str) -> Vec<Range> {
    let workspace = Workspace::new(name);
    workspace.write(".novis/completion/strict.json", file);
    let files = workspace.load();
    let (_, analysis, _) = opened(&workspace, &format!("{CURSOR}{call}"));
    nvs_lsp::for_document(&analysis, &files, Phases::Gated, PositionEncoding::Utf8)
        .into_iter()
        .filter(|diagnostic| {
            diagnostic.code
                == Some(NumberOrString::String(
                    code::W_COMPLETION_VALUE_UNKNOWN.as_str().to_owned(),
                ))
        })
        .inspect(|diagnostic| {
            assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::WARNING));
        })
        .map(|diagnostic| diagnostic.range)
        .collect()
}

/// `home` and `star` for `$name`, checked.
const STRICT: &str = r#"{
  "parameters": [
    { "method": "App\\Ui\\Icon::render", "parameter": "name", "values": ["home", "star"], "strict": true }
  ]
}"#;

/// The range of the literal `literal`, written as the first argument of `Icon::render(…)`.
fn first_argument(literal: &str) -> Range {
    let open = render_open();
    let line = between(open, 0).start.line;
    Range::new(
        Position::new(line, open),
        Position::new(line, open + u32::try_from(literal.len()).expect("short")),
    )
}

#[test]
fn a_strict_parameter_warns_on_a_literal_that_is_not_one_of_its_values() {
    for (call, want) in [
        ("Icon::render('moon', 16);", vec![first_argument("'moon'")]),
        (
            "Icon::render(\"Home\", 16);",
            vec![first_argument("\"Home\"")],
        ),
        ("Icon::render('home', 16);", Vec::new()),
        ("Icon::render(\"star\", 16, 'line');", Vec::new()),
        ("Label::text('moon');", Vec::new()),
    ] {
        assert_eq!(warned("strict", STRICT, call), want, "{call}");
    }
}

#[test]
fn a_strict_parameter_does_not_check_a_value_built_at_run_time() {
    for call in [
        "$name = 'moon';\nIcon::render($name, 16);",
        "Icon::render('mo' . 'on', 16);",
        "$name = 'moon';\nIcon::render(\"{$name}\", 16);",
    ] {
        assert_eq!(warned("run-time", STRICT, call), Vec::new(), "{call}");
    }
}

/// `home` for `$name` at every call, and `moon` only where `$style` is `line`, checked.
const STRICT_WHEN: &str = r#"{
  "parameters": [
    { "method": "App\\Ui\\Icon::render", "parameter": "name", "values": ["home"], "strict": true },
    {
      "method": "App\\Ui\\Icon::render",
      "parameter": "name",
      "values": ["moon"],
      "when": { "parameter": "style", "equals": "line" }
    }
  ]
}"#;

#[test]
fn a_strict_check_is_skipped_where_a_when_cannot_be_decided() {
    for (call, want) in [
        (
            "$style = 'line';\nIcon::render('moon', 16, $style);",
            Vec::new(),
        ),
        ("Icon::render('moon', 16);", Vec::new()),
        ("Icon::render('moon', 16, 'line');", Vec::new()),
        (
            "Icon::render('moon', 16, 'solid');",
            vec![first_argument("'moon'")],
        ),
    ] {
        assert_eq!(warned("strict-when", STRICT_WHEN, call), want, "{call}");
    }
}

#[test]
fn a_parameter_without_strict_reports_nothing() {
    for call in ["Icon::render('moon', 16);", "Icon::render('home', 16);"] {
        assert_eq!(warned("not-strict", ICONS, call), Vec::new(), "{call}");
    }
}

/// `arrow-left` deprecated for `arrow-back`, `star` deprecated with no replacement, `old`
/// deprecated for a replacement with a quote and a `$` in it, and `moon` deprecated only where
/// `$style` is `line`.
const DEPRECATED: &str = r#"{
  "parameters": [
    {
      "method": "App\\Ui\\Icon::render",
      "parameter": "name",
      "values": [
        "home",
        { "value": "arrow-left", "deprecated": true, "replacement": "arrow-back" },
        { "value": "star", "deprecated": true },
        { "value": "old", "deprecated": true, "replacement": "it's $new" }
      ]
    },
    {
      "method": "App\\Ui\\Icon::render",
      "parameter": "name",
      "values": [{ "value": "moon", "deprecated": true }],
      "when": { "parameter": "style", "equals": "line" }
    }
  ]
}"#;

#[test]
fn a_literal_equal_to_a_deprecated_value_gets_a_deprecated_hint() {
    let workspace = Workspace::new("deprecated");
    workspace.write(".novis/completion/deprecated.json", DEPRECATED);
    let files = workspace.load();
    for (call, want) in [
        (
            "Icon::render('arrow-left', 16);",
            vec![first_argument("'arrow-left'")],
        ),
        (
            "Icon::render(\"star\", 16);",
            vec![first_argument("\"star\"")],
        ),
        ("Icon::render('home', 16);", Vec::new()),
        ("Icon::render('moon', 16, 'solid');", Vec::new()),
        (
            "Icon::render('moon', 16, 'line');",
            vec![first_argument("'moon'")],
        ),
        ("Label::text('arrow-left');", Vec::new()),
    ] {
        let (_, analysis, _) = opened(&workspace, &format!("{CURSOR}{call}"));
        let hinted: Vec<Range> =
            nvs_lsp::for_document(&analysis, &files, Phases::Gated, PositionEncoding::Utf8)
                .into_iter()
                .filter(|diagnostic| {
                    diagnostic.code
                        == Some(NumberOrString::String(
                            code::W_COMPLETION_VALUE_DEPRECATED.as_str().to_owned(),
                        ))
                })
                .inspect(|diagnostic| {
                    assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::HINT));
                    assert_eq!(diagnostic.tags, Some(vec![DiagnosticTag::DEPRECATED]));
                })
                .map(|diagnostic| diagnostic.range)
                .collect();
        assert_eq!(hinted, want, "{call}");
    }
}

#[test]
fn the_replacement_quick_fix_rewrites_only_the_string_and_escapes_for_its_quote() {
    let workspace = Workspace::new("replacement");
    workspace.write(".novis/completion/deprecated.json", DEPRECATED);
    let files = workspace.load();
    let open = render_open();
    for (call, want) in [
        (
            "Icon::render('arrow-<|>left', 16);",
            vec![("replace with `arrow-back`", "arrow-back", between(open, 10))],
        ),
        (
            "Icon::render('o<|>ld', 16);",
            vec![("replace with `it's $new`", "it\\'s $new", between(open, 3))],
        ),
        (
            "Icon::render(\"o<|>ld\", 16);",
            vec![("replace with `it's $new`", "it's \\$new", between(open, 3))],
        ),
        ("Icon::render('st<|>ar', 16);", Vec::new()),
        ("Icon::render('ho<|>me', 16);", Vec::new()),
    ] {
        let (_, analysis, at) = opened(&workspace, call);
        let offered: Vec<(String, String, Range)> = actions::at(
            &analysis,
            &files,
            at,
            at,
            actions::Kind::QuickFix,
            PositionEncoding::Utf8,
        )
        .into_iter()
        .map(|action| (action.title, action.replacement, action.range))
        .collect();
        let want: Vec<(String, String, Range)> = want
            .into_iter()
            .map(|(title, text, range)| (title.to_owned(), text.to_owned(), range))
            .collect();
        assert_eq!(offered, want, "{call}");
    }

    // Without the file, nothing is offered.
    let (_, analysis, at) = opened(&workspace, "Icon::render('arrow-<|>left', 16);");
    let offered = actions::at(
        &analysis,
        &CompletionFiles::default(),
        at,
        at,
        actions::Kind::QuickFix,
        PositionEncoding::Utf8,
    );
    assert!(offered.is_empty(), "{offered:?}");
}

/// Values for `$name`, two of them only for some values of `$style`.
const STYLED: &str = r#"{
  "parameters": [
    { "method": "App\\Ui\\Icon::render", "parameter": "name", "values": ["home"] },
    {
      "method": "App\\Ui\\Icon::render",
      "parameter": "name",
      "values": ["star"],
      "when": { "parameter": "style", "equals": ["solid", "duo"] }
    },
    {
      "method": "App\\Ui\\Icon::render",
      "parameter": "name",
      "values": ["moon"],
      "when": { "parameter": "style", "equals": "line" }
    }
  ]
}"#;

/// The labels of `items`, in the order offered.
fn labels(items: &[CompletionItem]) -> Vec<&str> {
    items.iter().map(|item| item.label.as_str()).collect()
}

/// The labels offered at the cursor in each call, with [`STYLED`] loaded, against the labels each
/// one should get.
fn assert_styled(name: &str, calls: &[(&str, &[&str])]) {
    let workspace = Workspace::new(name);
    workspace.write(".novis/completion/styled.json", STYLED);
    let files = workspace.load();
    for (call, want) in calls {
        let (items, _) = offered(&workspace, &files, call);
        assert_eq!(labels(&items), *want, "{call}");
    }
}

#[test]
fn an_attachment_with_when_applies_only_when_the_other_argument_equals_one_of_its_strings() {
    assert_styled(
        "when",
        &[
            ("Icon::render('<|>', 16, 'solid');", &["home", "star"]),
            ("Icon::render('<|>', 16, \"duo\");", &["home", "star"]),
            ("Icon::render('<|>', 16, 'line');", &["home", "moon"]),
            ("Icon::render('<|>', 16, 'outline');", &["home"]),
        ],
    );
}

#[test]
fn an_attachment_with_when_does_not_apply_when_the_other_argument_is_not_a_literal() {
    assert_styled(
        "when-not-literal",
        &[
            (
                "$style = 'solid';\nIcon::render('<|>', 16, $style);",
                &["home"],
            ),
            ("Icon::render('<|>', 16, 'so' . 'lid');", &["home"]),
            // Left out, `$style` is `'solid'` at run time, and still no literal says so.
            ("Icon::render('<|>', 16);", &["home"]),
        ],
    );
}

#[test]
fn a_named_argument_is_read_for_when_like_a_positional_one() {
    assert_styled(
        "when-named",
        &[
            ("Icon::render('<|>', 16, style: 'line');", &["home", "moon"]),
            (
                "Icon::render(style: 'solid', size: 16, name: '<|>');",
                &["home", "star"],
            ),
        ],
    );
}

/// Keys for `$key`, split on `.`.
const KEYS: &str = r#"{
  "sets": {
    "keys": {
      "separator": ".",
      "values": [
        "shop.cart.title",
        { "value": "shop.cart.empty", "title": "Empty cart" },
        "shop.checkout.pay",
        "blog.title",
        { "value": "shop", "title": "The shop" }
      ]
    }
  },
  "parameters": [
    { "method": "App\\Ui\\Label::text", "parameter": "key", "set": "keys" }
  ]
}"#;

/// A workspace with [`KEYS`] and [`ICONS`] in it, and the files loaded from it.
fn keys(name: &str) -> (Workspace, CompletionFiles) {
    let workspace = Workspace::new(name);
    workspace.write(".novis/completion/keys.json", KEYS);
    workspace.write(".novis/completion/icons.json", ICONS);
    let files = workspace.load();
    (workspace, files)
}

#[test]
fn a_list_with_a_separator_offers_the_next_segment() {
    let (workspace, files) = keys("segments");

    // The first segments. `shop` ends a value and leads to longer ones, and is offered once, as
    // that value.
    let (items, _) = offered(&workspace, &files, "Label::text('<|>');");
    assert_eq!(labels(&items), ["blog", "shop"]);
    assert_eq!(named(&items, "shop").detail.as_deref(), Some("The shop"));

    // A segment that only leads to longer values is its text and nothing else.
    let blog = named(&items, "blog");
    assert_eq!(
        (&blog.kind, &blog.detail, &blog.documentation, &blog.tags),
        (&None, &None, &None, &None)
    );

    // After a separator, the segments that follow what was typed, typed text included.
    for call in ["Label::text('shop.<|>');", "Label::text('shop.ca<|>');"] {
        let (items, _) = offered(&workspace, &files, call);
        assert_eq!(labels(&items), ["cart", "checkout"], "{call}");
    }

    // The last segment of a value is that value's item.
    let (items, _) = offered(&workspace, &files, "Label::text('shop.cart.<|>');");
    assert_eq!(labels(&items), ["empty", "title"]);
    let empty = named(&items, "empty");
    assert_eq!(empty.detail.as_deref(), Some("Empty cart"));
    assert_eq!(empty.kind, Some(CompletionItemKind::VALUE));

    // Text that starts no value offers nothing.
    let (items, _) = offered(&workspace, &files, "Label::text('news.<|>');");
    assert!(items.is_empty(), "{items:?}");
}

#[test]
fn a_segment_item_replaces_only_the_segment_being_typed() {
    let (workspace, files) = keys("segment-edits");
    let open = u32::try_from("Label::text(".len()).expect("short");
    for (call, label, from, len) in [
        // From the last separator before the cursor to the end of the text.
        ("Label::text('shop.ca<|>');", "cart", 5, 2),
        // To the next separator after the cursor, so the segments after it are kept.
        ("Label::text('shop.ca<|>rt.title');", "cart", 5, 4),
        ("Label::text(\"shop.cart.<|>\");", "empty", 10, 0),
        ("Label::text('<|>');", "shop", 0, 0),
    ] {
        let (items, _) = offered(&workspace, &files, call);
        assert_eq!(
            edit(&items, label),
            (label.to_owned(), between(open + from, len)),
            "{call}"
        );
    }
}

#[test]
fn a_dot_opens_the_list_inside_a_string_whose_list_has_a_dot_separator() {
    let (workspace, files) = keys("dot-opens");
    for call in [
        "Label::text('shop.<|>');",
        "Label::text(\"shop.cart.<|>\");",
        "Label::text('<|>');",
    ] {
        let (_, triggered) = offered(&workspace, &files, call);
        assert!(triggered, "{call}");
    }
}

#[test]
fn a_dot_outside_such_a_string_opens_nothing() {
    let (workspace, files) = keys("dot-closed");
    for call in [
        // A list without a separator.
        "Icon::render('home.<|>', 16, 'solid');",
        // A separator the list does not split on.
        "Label::text('shop/<|>');",
        "Label::text('shop:<|>');",
        // Code, and a string at a parameter no file names.
        "$text = 'shop' .<|> 'cart';",
        "Icon::render('home', 16, 'line.<|>');",
    ] {
        let (_, triggered) = offered(&workspace, &files, call);
        assert!(!triggered, "{call}");
    }
    let (_, triggered) = offered(
        &workspace,
        &CompletionFiles::default(),
        "Label::text('shop.<|>');",
    );
    assert!(!triggered, "no file names the parameter");
}

/// A method whose `$mode` is a union of string literals, one whose `$size` adds `null` to its
/// strings, one whose `$count` adds an `int` literal, and one whose `$text` is a plain `string`.
const MODES: &str = "class Mode {\n    \
     public static function set(\"light\"|\"dark\"|\"it's\" $mode): string {\n        \
     return $mode;\n    }\n\n    \
     public static function size(\"small\"|\"large\"|null $size): string {\n        \
     return 'size';\n    }\n\n    \
     public static function count(\"none\"|\"many\"|3 $count): string {\n        \
     return 'count';\n    }\n\n    \
     public static function plain(string $text): string {\n        return $text;\n    }\n}\n\n";

/// The list offered at the cursor in `call`, written after [`MODES`], with no completion file, and
/// whether a trigger character there opens it.
fn moded(name: &str, call: &str) -> (Vec<CompletionItem>, bool) {
    let workspace = Workspace::new(name);
    offered(
        &workspace,
        &CompletionFiles::default(),
        &format!("{MODES}{call}"),
    )
}

#[test]
fn a_string_argument_at_a_literal_union_parameter_offers_its_members() {
    let (items, _) = moded("union", "Mode::set('<|>');");
    assert_eq!(labels(&items), ["dark", "it's", "light"]);
    for item in &items {
        assert_eq!(item.kind, Some(CompletionItemKind::VALUE), "{}", item.label);
    }

    // Each item replaces the text between the quotes, escaped for the quote.
    let line = u32::try_from(format!("{CLASSES}{MODES}").lines().count()).expect("short");
    let text = Range::new(Position::new(line, 11), Position::new(line, 16));
    let (items, _) = moded("union-edits", "Mode::set('li<|>ght');");
    assert_eq!(edit(&items, "dark"), ("dark".to_owned(), text));
    assert_eq!(edit(&items, "it's"), ("it\\'s".to_owned(), text));
}

#[test]
fn a_nullable_literal_union_offers_its_string_members() {
    let (items, _) = moded("nullable", "Mode::size('<|>');");
    assert_eq!(labels(&items), ["large", "small"]);
}

#[test]
fn a_quote_opens_the_list_at_a_literal_union_parameter() {
    for quoted in ["Mode::set('<|>');", "Mode::set(\"<|>\");"] {
        let (_, triggered) = moded("union-quote", quoted);
        assert!(triggered, "{quoted}");
    }
    let (_, triggered) = moded("union-plain-quote", "Mode::plain('<|>');");
    assert!(!triggered, "a plain `string` opens nothing");
}

#[test]
fn a_literal_union_parameter_with_a_completion_file_offers_both_lists_merged() {
    let workspace = Workspace::new("union-merged");
    workspace.write(
        ".novis/completion/modes.json",
        r#"{ "parameters": [ { "method": "App\\Ui\\Mode::set", "parameter": "mode", "values": [
            { "value": "dark", "label": "Dark", "title": "Dark colours" }, "system"
        ] } ] }"#,
    );
    let files = workspace.load();
    let (items, triggered) = offered(&workspace, &files, &format!("{MODES}Mode::set('<|>');"));
    assert!(triggered);

    // `dark` is offered once, as the file's value with its label and title.
    assert_eq!(labels(&items), ["Dark", "it's", "light", "system"]);
    assert_eq!(
        named(&items, "Dark").detail.as_deref(),
        Some("Dark colours")
    );
    assert_eq!(edit(&items, "Dark").0, "dark");
}

#[test]
fn a_parameter_typed_string_offers_no_literal_members() {
    // A plain `string`, and a union with an `int` literal among its strings.
    for (name, call) in [
        ("plain", "Mode::plain('<|>');"),
        ("mixed", "Mode::count('<|>');"),
    ] {
        let (items, triggered) = moded(name, call);
        assert!(
            !items
                .iter()
                .any(|item| item.kind == Some(CompletionItemKind::VALUE)),
            "{call}: {items:?}"
        );
        assert!(!triggered, "{call}");
    }
}
