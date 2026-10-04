//! `nvs check --strict-docs` — `rule:tooling/strict-docs`'s lint, and the
//! silence it is off by default.
//!
//! Through the built binary rather than by calling the front end, for the reason
//! `check_grants` already writes down: `nvs-cli` is a binary crate with no
//! library target. It is also the only way to ask the question this file asks,
//! because what is under test is *what the flag does to a command's verdict* —
//! the same program checks clean without it and fails with it, and an exit
//! status is a property of a process.
//!
//! Each test owns a private directory, because the tests run concurrently and
//! the fixture is the child's `cwd`.

use std::path::Path;
use std::process::{Command, Output};

/// A class whose one public method carries no `///`. Everything else in it is
/// documented, so the only thing `--strict-docs` can have to say is about
/// `total`.
const AN_UNDOCUMENTED_METHOD: &str = r"<?nvs
/// A shop's money, in the smallest unit its currency has.
class Money {
    /// How many of that unit this amount is.
    public int $cents = 0;

    public function total(): int { return $this->cents; }
}
";

/// The same class with `total` documented and a `private` helper that is not.
/// A private member is never reported, at any setting, so this checks clean
/// under the flag.
const A_PRIVATE_HELPER: &str = r"<?nvs
/// A shop's money, in the smallest unit its currency has.
class Money {
    /// How many of that unit this amount is.
    public int $cents = 0;

    /// The amount, as the reader of this package says it.
    public function total(): int { return $this->doubled(); }

    private function doubled(): int { return $this->cents + $this->cents; }
}
";

/// Three undocumented public methods, each of which overrides or implements a
/// documented one: `render` from an interface that `Named` extends, `name`
/// from `Named` itself and `price` from the parent class. Each inherits its
/// ancestor's doc comment, so this checks clean under the flag.
const INHERITED_DOCS: &str = r#"<?nvs
/// Something a reader can see.
interface Printable {
    /// The text a reader sees.
    public function render(): string;
}

/// Something a reader can see, under a name.
interface Named extends Printable {
    /// The name a reader sees.
    public function name(): string;
}

/// A thing a shop sells.
class Item {
    /// What the item costs, in cents.
    public function price(): int { return 100; }
}

/// A post, sold like any other item.
class Post extends Item implements Named {
    public function name(): string { return "First post"; }

    public function render(): string { return $this->name(); }

    public function price(): int { return 250; }
}
"#;

/// The same parent class, and a subclass method that overrides nothing.
/// `summary` has no ancestor to inherit a doc comment from, so it is reported.
const A_NEW_METHOD_ON_A_SUBCLASS: &str = r#"<?nvs
/// A thing a shop sells.
class Item {
    /// What the item costs, in cents.
    public function price(): int { return 100; }
}

/// A post, sold like any other item.
class Post extends Item {
    public function price(): int { return 250; }

    public function summary(): string { return "A post"; }
}
"#;

/// A fresh directory holding `case.nvs`. The name is the test's, so two tests
/// never share a working directory.
fn fixture(name: &str, program: &str) -> nvs_repo::Scratch {
    let dir = nvs_repo::scratch(&format!("strict-docs-{name}"));
    std::fs::write(dir.join("case.nvs"), program).expect("the program is written");
    dir
}

/// `nvs check <args...> case.nvs`, run *in* `dir`.
fn check(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("check")
        .args(args)
        .arg("case.nvs")
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// Both streams together: a diagnostic is rendered to standard error and
/// `no errors` to standard output, and a test asserting on one should not have
/// to know which.
fn shown(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn strict_docs_reports_an_undocumented_public_member() {
    // `rule:tooling/strict-docs`: a public member with no attached doc comment
    // is what the flag reports, and it names the member rather than the file so
    // the repair is one line above one declaration.
    let dir = fixture("undocumented", AN_UNDOCUMENTED_METHOD);
    let out = check(&dir, &["--strict-docs"]);
    let shown = shown(&out);
    assert!(
        !out.status.success(),
        "an undocumented public member checked clean under --strict-docs: {shown}"
    );
    assert!(
        shown.contains("E0326"),
        "the report is not `E_DOC_MISSING`: {shown}"
    );
    assert!(
        shown.contains("total"),
        "the report does not name the member: {shown}"
    );
}

#[test]
fn strict_docs_is_silent_about_a_private_member() {
    // The rule's other half: a private helper is never reported. Nothing about
    // it is reachable by a reader of the package, so demanding a sentence about
    // it would buy the noise the rule exists to refuse.
    let dir = fixture("private", A_PRIVATE_HELPER);
    let out = check(&dir, &["--strict-docs"]);
    let shown = shown(&out);
    assert!(
        out.status.success(),
        "a documented surface with a private helper failed --strict-docs: {shown}"
    );
    assert!(
        !shown.contains("E0326"),
        "a private member was reported: {shown}"
    );
}

#[test]
fn strict_docs_accepts_an_override_of_a_documented_method() {
    // A method that overrides a parent's method or implements an interface's
    // inherits that method's doc comment, through any number of `extends`
    // steps. Repeating the sentence above every implementation would be the
    // generated noise the rule exists to refuse.
    let dir = fixture("inherited", INHERITED_DOCS);
    let out = check(&dir, &["--strict-docs"]);
    let shown = shown(&out);
    assert!(
        out.status.success(),
        "an override of a documented method failed --strict-docs: {shown}"
    );
    assert!(
        !shown.contains("E0326"),
        "an override of a documented method was reported: {shown}"
    );
}

#[test]
fn strict_docs_reports_a_subclass_method_that_overrides_nothing() {
    // Inheriting is by name, from an ancestor that documents that name. A new
    // method on a subclass has no such ancestor, so it is reported even though
    // the parent class is fully documented and the override beside it is not.
    let dir = fixture("new-on-subclass", A_NEW_METHOD_ON_A_SUBCLASS);
    let out = check(&dir, &["--strict-docs"]);
    let shown = shown(&out);
    assert!(
        !out.status.success(),
        "a new undocumented subclass method checked clean under --strict-docs: {shown}"
    );
    assert!(
        shown.contains("Post::summary"),
        "the new method is not the one reported: {shown}"
    );
    assert!(
        !shown.contains("Post::price"),
        "the override of a documented method was reported: {shown}"
    );
}

#[test]
fn nvs_check_is_silent_about_documentation_without_the_flag() {
    // `nvs check` says nothing about documentation in any project that has not
    // asked, which is what makes the flag opt-in rather than a diagnostic
    // everyone answers with a generated `///`.
    let dir = fixture("silent", AN_UNDOCUMENTED_METHOD);
    let out = check(&dir, &[]);
    let shown = shown(&out);
    assert!(
        out.status.success(),
        "the same program failed a plain `nvs check`: {shown}"
    );
    assert!(
        !shown.contains("E0326"),
        "documentation was reported without the flag: {shown}"
    );
}
