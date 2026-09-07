//! `nvs ast --json`, driven the way the AST panel drives it — through the
//! built binary, because what
//! `rule:ide/the-ast-panel-shells-out-to-the-cli` promises is what the
//! *command* prints, the same reason `meta.rs` goes through it.

use std::path::PathBuf;
use std::process::Command;

/// `nvs ast <args...> tests/fixtures/ast/<fixture>`, as `(stdout, success)`.
fn ast(args: &[&str], fixture: &str) -> (String, bool) {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "ast",
        fixture,
    ]
    .iter()
    .collect();
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("ast")
        .args(args)
        .arg(path)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the document is UTF-8"),
        out.status.success(),
    )
}

/// `rule:ide/ast-json-schema-is-frozen`'s default, from both sides: the
/// fixture ends in `$u->`, so the parser reports an error on it and recovers,
/// and the question is which half of the pair prints a tree for it.
#[test]
fn ast_resilient_is_the_default_and_strict_is_opt_in() {
    let (default, ok) = ast(&["--json"], "recovered.nvs");
    assert!(!ok, "source with an error still exits non-zero");
    let tree: serde_json::Value =
        serde_json::from_str(&default).expect("the default printed a document");
    assert_eq!(tree["kind"], "File", "the root is the file: {default}");
    assert!(
        tree["children"]
            .as_array()
            .expect("`children` is an array")
            .iter()
            .any(|node| node["kind"] == "ClassDecl"),
        "the class before the error is in the tree: {default}"
    );

    let (explicit, _) = ast(&["--json", "--resilient"], "recovered.nvs");
    assert_eq!(
        explicit, default,
        "`--resilient` names the default rather than changing it"
    );

    let (strict, ok) = ast(&["--json", "--strict"], "recovered.nvs");
    assert!(!ok, "strict over source with an error exits non-zero");
    assert!(
        strict.is_empty(),
        "`--strict` printed a tree for source it refused: {strict}"
    );

    // The two are a pair rather than a precedence: asking for both is an
    // argument error, so no caller has to know which one wins.
    let (both, ok) = ast(&["--json", "--resilient", "--strict"], "recovered.nvs");
    assert!(!ok, "`--resilient --strict` is refused");
    assert!(both.is_empty(), "an argument error prints no tree: {both}");
}
