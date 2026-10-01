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

/// One class carrying two of each tag, which
/// `rule:tooling/doc-comment-tags-are-see-and-example` allows and a card
/// therefore carries as lists. `Core\Str::length` is the target with no page in
/// this run, so the rendered line has one of each kind in it.
const REPEATED_TAGS: &str = r#"<?nvs
/// A till, and the money it has taken.
///
/// @see Till::charge
/// @see Core\Str::length
/// @example examples/charge.nvs
/// @example examples/refund.nvs
class Till {
    /// Everything this till has taken, in cents.
    public int $taken;

    /// Builds a till that has taken nothing yet.
    public function constructor() {
        $this->taken = 0;
    }

    /// Takes `$cents`, and answers the running total.
    public function charge(int $cents): int {
        $this->taken = $this->taken + $cents;
        return $this->taken;
    }
}

var $till = new Till();
echo $till->charge(150) . "\n";
"#;

/// A class with a parent class, an interface and a static method. `Shop\Item`
/// and `Shop\Printable` are named through `use` imports, so the page has to
/// name them fully qualified to link to their pages.
const LINEAGE: &str = r#"<?nvs
namespace Shop;

/// Something a reader can see.
interface Printable {
    /// The text a reader sees.
    public function render(): string;
}

/// A thing a shop sells.
abstract class Item {
    /// What the item costs, in cents.
    public function price(): int {
        return 100;
    }
}

namespace Blog;

use Shop\Item;
use Shop\Printable;

/// A post, sold like any other item.
///
/// @see parent::price
class Post extends Item implements Printable {
    /// The post as a reader sees it.
    public function render(): string {
        return "a post";
    }

    /// A post with nothing in it yet.
    public static function blank(): Post {
        return new Post();
    }
}

echo Post::blank()->render(), "\n";
"#;

/// A fresh directory holding `case.nvs`, and the `pages` directory `nvs doc`
/// will be pointed at.
fn fixture(name: &str) -> PathBuf {
    fixture_of(name, TWO_CLASSES)
}

/// The same, over a program the caller chose.
fn fixture_of(name: &str, program: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nvs-doc-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a private directory under the temp dir");
    std::fs::write(dir.join("case.nvs"), program).expect("the program is written");
    dir
}

/// Writes the files an `@example` names, under the `examples/` the tag's own
/// check requires them to sit in.
fn examples(dir: &Path, names: &[&str]) {
    let examples = dir.join("examples");
    std::fs::create_dir_all(&examples).expect("the examples directory is created");
    for name in names {
        std::fs::write(examples.join(name), "<?nvs\necho \"an example\\n\";\n")
            .expect("the example is written");
    }
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

#[test]
fn nvs_doc_renders_every_see_and_every_example() {
    // `rule:tooling/doc-comment-tags-are-see-and-example`: both tags repeat, so
    // a card carries lists and the page renders each list on one line, in the
    // order the tags were written. A renderer that read the first of each would
    // drop the rest silently, which is the one failure a single-tag page cannot
    // show.
    let dir = fixture_of("repeated", REPEATED_TAGS);
    examples(&dir, &["charge.nvs", "refund.nvs"]);
    let out = doc(&dir);
    assert!(
        out.status.success(),
        "`nvs doc` failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let page = page(&dir, "Till.md");
    assert!(
        page.contains(r"See also: [Till::charge](Till.md#charge), `Core\Str::length`"),
        "both `@see` targets were not rendered on one line: {page}"
    );
    assert!(
        page.contains("Example: `examples/charge.nvs`, `examples/refund.nvs`"),
        "both `@example` paths were not rendered on one line: {page}"
    );
}

#[test]
fn nvs_doc_writes_static_in_front_of_a_static_methods_signature() {
    // The document carries `static` as the member's `kind` and leaves it out
    // of `signature`, so the page has to write it back. Without it a reader
    // calls `blank()` on an instance.
    let dir = fixture_of("static", LINEAGE);
    let out = doc(&dir);
    assert!(
        out.status.success(),
        "`nvs doc` failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let page = page(&dir, "Blog.Post.md");
    assert!(
        page.contains("```nvs\nstatic blank(): Post\n```"),
        "the static method's signature has no `static`: {page}"
    );
    assert!(
        page.contains("```nvs\nrender(): string\n```"),
        "an instance method's signature gained a prefix: {page}"
    );
}

#[test]
fn nvs_doc_names_the_parent_class_and_the_interfaces() {
    // `extends` and `implements` come from the document fully qualified, and
    // each becomes a link because both declarations got a page in this run.
    // `parent::price` in a `@see` links to the parent's page for the same
    // reason.
    let dir = fixture_of("lineage", LINEAGE);
    let out = doc(&dir);
    assert!(
        out.status.success(),
        "`nvs doc` failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let page = page(&dir, "Blog.Post.md");
    assert!(
        page.contains(r"Extends: [Shop\Item](Shop.Item.md)"),
        "the parent class is not named: {page}"
    );
    assert!(
        page.contains(r"Implements: [Shop\Printable](Shop.Printable.md)"),
        "the interface is not named: {page}"
    );
    assert!(
        page.contains("[parent::price](Shop.Item.md#price)"),
        "`parent::` in a `@see` did not link to the parent's page: {page}"
    );
}
