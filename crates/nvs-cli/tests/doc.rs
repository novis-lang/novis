//! `nvs doc <entry>` — `rule:tooling/nvs-doc-renders-and-decides-nothing`'s
//! renderer, driven as a project drives it.
//!
//! Through the built binary rather than by calling the renderer, for the reason
//! `check_grants` writes down: `nvs-cli` is a binary crate with no library
//! target. What is under test is what lands on disk, which is a property of a
//! run rather than of a function.
//!
//! Each test owns a private directory, because the tests run concurrently.

use std::path::{Path, PathBuf};
use std::process::Output;

/// Two documented classes, one of them naming a member of the other. Both get a
/// page, and the `@see` on `Page::line` has a target this run rendered.
const TWO_CLASSES: &str = r#"<?nvs
/// A line of text, with the text it carries.
class Line {
    /// The text this line carries.
    public string $text;

    /// Builds a line over its own text.
    public function constructor(string $text) {
        $this->text = $text;
    }

    /// The line as a reader sees it.
    public function render(): string {
        return $this->text;
    }
}

/// A page, which is lines in an order.
class Page {
    /// The one line this page has room for.
    ///
    /// @see Line::render
    public function line(): Line {
        return new Line("only");
    }
}

var $page = new Page();
echo $page->line()->render() . "\n";
"#;

/// A fresh directory holding `case.nvs`, and the `pages` directory `nvs doc`
/// will be pointed at.
fn fixture(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nvs-doc-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a private directory under the temp dir");
    std::fs::write(dir.join("case.nvs"), TWO_CLASSES).expect("the program is written");
    dir
}

/// `nvs doc case.nvs --out pages`, run *in* `dir`.
fn doc(dir: &Path) -> Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["doc", "case.nvs", "--out", "pages"])
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// One rendered page's text.
fn page(dir: &Path, name: &str) -> String {
    let path = dir.join("pages").join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("{} was written: {err}", path.display()))
}

#[test]
fn nvs_doc_writes_one_page_per_class() {
    // `rule:tooling/nvs-doc-renders-and-decides-nothing`: a page per class, and
    // its content is the card `nvs meta --json` already carries — the prose,
    // and the signature read off the source.
    let dir = fixture("pages");
    let out = doc(&dir);
    let shown = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "`nvs doc` failed: {shown}");

    let line = page(&dir, "Line.md");
    assert!(line.starts_with("# Line"), "the page is not Line's: {line}");
    assert!(
        line.contains("The text this line carries."),
        "a member's prose is missing: {line}"
    );
    assert!(
        line.contains("render(): string"),
        "a member's signature is missing: {line}"
    );

    let page = page(&dir, "Page.md");
    assert!(
        page.contains("A page, which is lines in an order."),
        "the class's own prose is missing: {page}"
    );
}

#[test]
fn nvs_doc_renders_a_see_target_as_a_link() {
    // The one thing the renderer does that is more than a copy: a `@see` whose
    // target has a page in this run becomes a link into it. A target with no
    // page — every `Core` member — stays code, because a link to a file nobody
    // wrote is worse than the name.
    let dir = fixture("links");
    let out = doc(&dir);
    assert!(
        out.status.success(),
        "`nvs doc` failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let page = page(&dir, "Page.md");
    assert!(
        page.contains("[Line::render](Line.md#render)"),
        "the `@see` target was not rendered as a link: {page}"
    );
}
