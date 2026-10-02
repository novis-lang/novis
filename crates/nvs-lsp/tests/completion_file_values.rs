//! A string argument at a parameter a completion file names offers the file's values, from the
//! attachments whose `when` the call meets and one segment at a time where a list has a
//! separator, and each item changes only that string
//! (`rule:ide/completion-files-offer-values-at-named-parameters`).
//!
//! Each test writes a workspace of its own under cargo's scratch folder in `target/` and deletes it
//! when it ends, as `tests/completion_files.rs` does.

use std::fs;
use std::path::{Path, PathBuf};

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemTag, CompletionTextEdit, Documentation,
    Position, Range,
};
use nvs_diagnostics::PositionEncoding;
use nvs_lsp::completion_files::CompletionFiles;
use nvs_lsp::{CheckScope, Client, Documents, PhpNames, SymbolIndex, analyse, completion, uri_of};

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
    let source = format!("{CLASSES}{call}");
    let cursor = source.find(CURSOR).expect("the call marks its cursor");
    let source = source.replacen(CURSOR, "", 1);
    let path = workspace.write("src/page.nvs", &source);
    let uri = uri_of(&path).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source);
    let analysis = analyse(&documents, &uri).expect("an open document analyses");
    let index = SymbolIndex::build(&documents, CheckScope::Open, None);
    let at = u32::try_from(cursor).expect("a test document is short");
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

#[test]
fn a_string_argument_at_a_named_parameter_offers_the_files_values() {
    let workspace = Workspace::new("offers");
    let file = workspace.write(".novis/completion/icons.json", ICONS);
    let files = workspace.load();

    // Single-quoted: every value, the set's first and the attachment's own after it.
    let (items, _) = offered(&workspace, &files, "Icon::render('ho<|>me', 16, 'solid');");
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(labels, ["Arrow left", "cost$", "home", "it's"]);

    // Each item replaces the whole text between the quotes, and only that.
    let open = u32::try_from("Icon::render(".len()).expect("short");
    for (label, written) in [
        ("home", "home"),
        ("Arrow left", "arrow-left"),
        ("it's", "it\\'s"),
        ("cost$", "cost$"),
    ] {
        assert_eq!(edit(&items, label), (written.to_owned(), between(open, 4)));
        let item = named(&items, label);
        assert!(item.command.is_none(), "`{label}` runs nothing");
        assert!(
            item.additional_text_edits.is_none(),
            "`{label}` writes nothing else"
        );
        assert!(item.insert_text.is_none() && item.insert_text_format.is_none());
    }

    // Every field the file gives is the item's.
    let arrow = named(&items, "Arrow left");
    assert_eq!(arrow.kind, Some(CompletionItemKind::CONSTANT));
    assert_eq!(arrow.detail.as_deref(), Some("Left arrow"));
    let details = arrow.label_details.as_ref().expect("label details");
    assert_eq!(details.detail.as_deref(), Some(" (outline)"));
    assert_eq!(details.description.as_deref(), Some("navigation"));
    assert_eq!(arrow.tags, Some(vec![CompletionItemTag::DEPRECATED]));
    assert_eq!(arrow.sort_text.as_deref(), Some("0"));
    assert_eq!(arrow.filter_text.as_deref(), Some("arrow-left"));
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
    assert_eq!(named(&items, "home").kind, Some(CompletionItemKind::VALUE));

    // Double-quoted: the escapes are that quote's, and an empty literal is replaced too.
    let (items, triggered) = offered(&workspace, &files, "Icon::render(\"<|>\", 16, 'solid');");
    assert!(triggered, "a quote at a named parameter opens the list");
    assert_eq!(edit(&items, "it's"), ("it's".to_owned(), between(open, 0)));
    assert_eq!(
        edit(&items, "cost$"),
        ("cost\\$".to_owned(), between(open, 0))
    );

    // A parameter no file names is ordinary text: no value is offered and a quote opens nothing.
    let (items, triggered) = offered(&workspace, &files, "Icon::render('home', 16, '<|>');");
    assert!(
        !items.iter().any(|item| item.label == "home"),
        "no value at `$style`"
    );
    assert!(!triggered);

    // Without the file, the same argument offers no value either.
    let (items, triggered) = offered(
        &workspace,
        &CompletionFiles::default(),
        "Icon::render('<|>', 16, 'solid');",
    );
    assert!(
        !items.iter().any(|item| item.label == "home"),
        "no value without the file"
    );
    assert!(!triggered);
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
