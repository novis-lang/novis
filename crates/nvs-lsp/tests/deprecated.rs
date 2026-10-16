//! What an editor shows for a use of deprecated code
//! (`rule:attributes/a-deprecation-names-its-replacement-as-code`): the
//! `Deprecated` tag on `W1003`, and the rewrite it carries offered as a quick
//! fix and under a fix-all
//! (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`).
//!
//! The rewrite itself is the checker's, and `nvs-types`' own tests pin what it
//! writes. These pin that it reaches the editor whole: one action per use, with
//! the `use` line it needs as a second edit of that action. A deprecated
//! method's completion item is tagged too, and a hover on a use shows what the
//! attribute wrote.

use lsp_types::{CompletionItemTag, DiagnosticTag, HoverContents};
use nvs_diagnostics::PositionEncoding;
use nvs_lsp::completion_files::CompletionFiles;
use nvs_lsp::{
    Analysed, CheckScope, Client, Documents, Phases, Response, SymbolIndex, actions, analyse,
    completion, for_document, hover, uri_of,
};

/// A class whose `get` is deprecated in favour of `find`.
const API: &str = r#"<?nvs
class Api {
  #[Core\Deprecated(since: '2.0', note: 'It reads the cache twice.', replace: '$this->find($id)')]
  public function get(int $id): int { return $id; }
  public function find(int $id): int { return $id; }
}
var $api = new Api();
"#;

/// `source` opened alone and analysed, with the symbol index completion reads.
fn opened(source: &str) -> (Documents, Analysed) {
    let dir = nvs_repo::scratch("lsp-deprecated-member");
    let uri = uri_of(&dir.join("case.nvs")).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    let analysed = analyse(&documents, &uri).expect("an open document analyses");
    (documents, analysed)
}

/// A library whose `Api::cents` is deprecated, with a replacement that names
/// `Money`, a class a caller in another namespace has to import.
const SHOP: &str = r#"<?nvs
namespace Shop;
class Money { public static function of(int $c): int { return $c; } }
class Api {
  #[Core\Deprecated(replace: 'Money::of($cents)')]
  public static function cents(int $cents): int { return Money::of($cents); }
}
"#;

/// Two uses of `Api::cents` in one file that imports `Api` and not `Money`.
const PAGE: &str = r#"<?nvs
namespace Blog;
require './shop.nvs';
use Shop\Api;
class Page {
  public function total(): int { return Api::cents(5); }
  public function more(): int { return Api::cents(7); }
}
"#;

/// `PAGE` opened as its own entry point, with `SHOP` beside it on disk for its
/// `require` to read.
fn analysed() -> Analysed {
    let dir = nvs_repo::scratch("lsp-deprecated");
    std::fs::write(dir.join("shop.nvs"), SHOP).expect("the scratch directory is writable");
    let uri = uri_of(&dir.join("page.nvs")).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, PAGE.to_owned());
    analyse(&documents, &uri).expect("an open document analyses")
}

/// Every action offered over `[start, end)` of `PAGE` under `kind`, rendered as
/// a `.lspt` case freezes it.
fn offered(analysed: &Analysed, start: usize, end: usize, kind: actions::Kind) -> String {
    let at = |offset: usize| u32::try_from(offset).expect("the document is short");
    Response::CodeAction(actions::at(
        analysed,
        &CompletionFiles::default(),
        at(start),
        at(end),
        kind,
        PositionEncoding::Utf8,
    ))
    .render()
}

/// Each use is published with the `Deprecated` tag, which an editor draws as a
/// line through the name.
#[test]
fn a_deprecated_use_carries_the_deprecated_tag() {
    let published = for_document(
        &analysed(),
        &CompletionFiles::default(),
        Phases::Gated,
        PositionEncoding::Utf8,
    );
    let deprecated: Vec<_> = published
        .iter()
        .filter(|diagnostic| {
            diagnostic.code == Some(lsp_types::NumberOrString::String("W1003".to_owned()))
        })
        .collect();
    assert_eq!(deprecated.len(), 2, "{published:#?}");
    for diagnostic in deprecated {
        assert_eq!(diagnostic.tags, Some(vec![DiagnosticTag::DEPRECATED]));
    }
}

/// A light bulb on one use offers the rewrite and its import as one action. A
/// fix-all over the file offers both rewrites, and only the first carries the
/// import, so a save writes the `use` line once.
#[test]
fn a_deprecated_use_offers_its_fix_as_a_quick_fix_and_under_fix_all() {
    let analysed = analysed();
    let first = PAGE
        .find("Api::cents(5)")
        .expect("the first use is in the page");
    assert_eq!(
        offered(&analysed, first, first, actions::Kind::QuickFix),
        "6:41-6:54 quickfix replace with `Money::of(5)` -> \"Money::of(5)\" \
         + 4:14-4:14 -> \"\\nuse Shop\\Money;\"\n"
    );
    assert_eq!(
        offered(&analysed, 0, PAGE.len(), actions::Kind::FixAll),
        "6:41-6:54 source.fixAll.nvs replace with `Money::of(5)` -> \"Money::of(5)\" \
         + 4:14-4:14 -> \"\\nuse Shop\\Money;\"\n\
         7:40-7:53 source.fixAll.nvs replace with `Money::of(7)` -> \"Money::of(7)\"\n"
    );
}

/// After `$api->`, `get` is offered with the `Deprecated` tag, and `find`,
/// which is not deprecated, without one.
#[test]
fn a_deprecated_member_completion_item_is_tagged_deprecated() {
    let source = format!("{API}$api->");
    let (documents, analysed) = opened(&source);
    let index = SymbolIndex::build(&documents, CheckScope::Open, None);
    let items = completion::at(
        &analysed,
        &index,
        &CompletionFiles::default(),
        u32::try_from(source.len()).expect("the document is short"),
        Client::default(),
        PositionEncoding::Utf8,
    );
    let tags = |label: &str| {
        items
            .iter()
            .find(|item| item.label == label)
            .unwrap_or_else(|| panic!("`{label}` is offered: {items:#?}"))
            .tags
            .clone()
    };
    assert_eq!(tags("get"), Some(vec![CompletionItemTag::DEPRECATED]));
    assert_eq!(tags("find"), None);
}

/// A hover on a use of `get` shows the version, the note and the code to write
/// instead, as the attribute wrote them.
#[test]
fn hover_on_a_deprecated_member_shows_since_note_and_replacement() {
    let source = format!("{API}echo $api->get(1);\n");
    let (_, analysed) = opened(&source);
    let at = source.find("get(1)").expect("the use is in the document");
    let hovered = hover::at(
        &analysed,
        &CompletionFiles::default(),
        u32::try_from(at).expect("the document is short"),
        PositionEncoding::Utf8,
    )
    .expect("a deprecated method has something to show");
    let HoverContents::Markup(markup) = hovered.contents else {
        panic!("a hover is Markdown");
    };
    assert_eq!(
        markup.value,
        "**Deprecated** since 2.0. It reads the cache twice.\n\n\
         Write `$this->find($id)` instead."
    );
}
