//! A string argument at a parameter a completion file names offers the file's values, and each
//! item changes only that string (`rule:ide/completion-files-offer-values-at-named-parameters`).
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

/// The class every document below calls.
const ICON_CLASS: &str = "<?nvs\nnamespace App\\Ui;\n\nclass Icon {\n    \
     public static function render(string $name, int $size, string $style): string {\n        \
     return $name . $style . $size;\n    }\n}\n\n";

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

/// The list offered at the [`CURSOR`] in `call`, written after [`ICON_CLASS`] in a document in
/// `workspace`, with the byte offset of the cursor in the whole text.
fn offered(
    workspace: &Workspace,
    files: &CompletionFiles,
    call: &str,
) -> (Vec<CompletionItem>, bool) {
    let source = format!("{ICON_CLASS}{call}");
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
    let line = u32::try_from(ICON_CLASS.lines().count()).expect("a short class");
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
