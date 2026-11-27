//! What an editor shows for test-only code
//! (`rule:testing/a-test-only-declaration-is-test-code`): completion offers a
//! `#[TestOnly]` member or type only inside test code, and a hover on one says
//! that `nvs run` and `nvs build` leave it out.
//!
//! The checker's own tests pin which uses compile. These pin that the editor
//! offers nothing whose use would not compile at the cursor.

use lsp_types::{CompletionItem, HoverContents};
use nvs_diagnostics::PositionEncoding;
use nvs_lsp::completion_files::CompletionFiles;
use nvs_lsp::{
    Analysed, CheckScope, Client, Documents, SymbolIndex, analyse, completion, hover, uri_of,
};

/// A production cache with a test-only `clearAll`, and a test-only fake clock.
const DECLARED: &str = r#"<?nvs
use Core\Test;
use Core\TestOnly;

class Cache {
  public function get(string $key): int { return 1; }
  #[TestOnly]
  public function clearAll(): void {}
}

#[TestOnly]
class FakeClock {
  public function now(): int { return 7; }
}

class SystemClock {
  public function now(): int { return 1; }
}
"#;

/// `before` then `after` opened alone and analysed, with the items completion
/// offers between the two.
fn offered(before: &str, after: &str) -> Vec<CompletionItem> {
    let source = format!("{DECLARED}{before}{after}");
    let (documents, analysed) = opened(&source);
    let index = SymbolIndex::build(&documents, CheckScope::Open, None);
    completion::at(
        &analysed,
        &documents,
        &index,
        &CompletionFiles::default(),
        u32::try_from(DECLARED.len() + before.len()).expect("the document is short"),
        Client::default(),
        PositionEncoding::Utf8,
    )
}

/// `source` opened alone and analysed.
fn opened(source: &str) -> (Documents, Analysed) {
    let dir = nvs_repo::scratch("lsp-test-only");
    let uri = uri_of(&dir.join("case.nvs")).expect("a scratch path is UTF-8");
    let mut documents = Documents::new();
    documents.open(uri.clone(), 1, source.to_owned());
    let analysed = analyse(&documents, &uri).expect("an open document analyses");
    (documents, analysed)
}

/// Whether an item labelled `label` is among `items`.
fn has(items: &[CompletionItem], label: &str) -> bool {
    items.iter().any(|item| item.label == label)
}

/// The text of a test method's body up to the cursor, and the rest of the
/// class after it.
const TEST_OPEN: &str = "class CacheTest {\n  #[Test]\n  public function clears(): void {\n    ";
const TEST_CLOSE: &str = "\n  }\n}\n";

/// After `$cache->` in code that is not a test, `get` is offered and the
/// test-only `clearAll` is not.
// covers: lang:testing/testonly-code-that-only-tests-can-use, tools:editor/nvs-lsp
#[test]
fn completion_outside_test_code_offers_no_test_only_member() {
    let items = offered("var $cache = new Cache();\n$cache->", "");
    assert!(has(&items, "get"), "{items:#?}");
    assert!(!has(&items, "clearAll"), "{items:#?}");
}

/// After `new ` in code that is not a test, `SystemClock` is offered and the
/// test-only `FakeClock` is not.
#[test]
fn completion_outside_test_code_offers_no_test_only_class() {
    let items = offered("var $clock = new Sys", "");
    assert!(has(&items, "SystemClock"), "{items:#?}");
    assert!(!has(&items, "FakeClock"), "{items:#?}");
}

/// Inside a `#[Test]` method, both the test-only member and the test-only
/// class are offered.
#[test]
fn completion_inside_test_code_offers_test_only_code() {
    let members = offered(
        &format!("{TEST_OPEN}var $cache = new Cache();\n    $cache->"),
        TEST_CLOSE,
    );
    assert!(has(&members, "get"), "{members:#?}");
    assert!(has(&members, "clearAll"), "{members:#?}");
    let types = offered(&format!("{TEST_OPEN}var $clock = new Sys"), TEST_CLOSE);
    assert!(has(&types, "SystemClock"), "{types:#?}");
    assert!(has(&types, "FakeClock"), "{types:#?}");
}

/// A hover on a call of the test-only method, and on the test-only class,
/// says it is only for tests. A hover on a production method does not.
#[test]
fn a_hover_on_test_only_code_says_it_is_for_tests() {
    let source = format!(
        "{DECLARED}{TEST_OPEN}var $cache = new Cache();\n    $cache->clearAll();\n    \
         var $clock = new FakeClock();\n    Test::assertSame(1, $cache->get('a'));{TEST_CLOSE}"
    );
    let (_, analysed) = opened(&source);
    let hovered = |needle: &str| {
        let at = source.rfind(needle).expect("the needle is in the document");
        hover::at(
            &analysed,
            &CompletionFiles::default(),
            u32::try_from(at).expect("the document is short"),
            PositionEncoding::Utf8,
        )
        .map(|hovered| match hovered.contents {
            HoverContents::Markup(markup) => markup.value,
            other => panic!("a hover is Markdown: {other:?}"),
        })
        .unwrap_or_default()
    };
    let line = "**Only for tests.** `nvs run` and `nvs build` leave it out.";
    assert!(
        hovered("clearAll();").contains(line),
        "{}",
        hovered("clearAll();")
    );
    assert!(
        hovered("FakeClock();").contains(line),
        "{}",
        hovered("FakeClock();")
    );
    assert!(
        !hovered("get('a')").contains(line),
        "{}",
        hovered("get('a')")
    );
}
