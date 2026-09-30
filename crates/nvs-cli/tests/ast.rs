//! `nvs ast --json`, driven the way the AST panel drives it — through the
//! built binary, because what
//! `rule:ide/the-ast-panel-shells-out-to-the-cli` promises is what the
//! *command* prints, the same reason `meta.rs` goes through it.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// `nvs ast <args...> <path>`, as `(stdout, stderr, success)`.
///
/// Both streams, because half of what the panel is promised is that they stay
/// apart: the document is standard output and the diagnostics are standard
/// error, on a file that does not compile as much as on one that does.
fn run_ast(args: &[&str], path: &Path) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("ast")
        .args(args)
        .arg(path)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the document is UTF-8"),
        String::from_utf8(out.stderr).expect("the diagnostics are UTF-8"),
        out.status.success(),
    )
}

/// `tests/fixtures/ast/<name>`.
fn fixture(name: &str) -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "ast", name]
        .iter()
        .collect()
}

/// `nvs ast <args...> tests/fixtures/ast/<fixture>`, as `(stdout, success)`.
fn ast(args: &[&str], name: &str) -> (String, bool) {
    let (out, _, ok) = run_ast(args, &fixture(name));
    (out, ok)
}

/// `rule:ide/ast-json-schema-is-frozen`'s default, from both sides: the
/// fixture ends in `$u->`, so the parser reports an error on it and recovers,
/// and the question is which half of the pair prints a tree for it.
// covers: tools:cli/nvs-ast
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

/// The two layers the grammar drops, which are the two
/// `rule:ide/ast-json-schema-is-frozen` puts back: the comments a formatter
/// reads, and the node the parser invented where the source stopped.
// covers: tools:cli/nvs-ast
#[test]
fn ast_json_includes_trivia_and_recovery_nodes() {
    let text = std::fs::read_to_string(fixture("recovered.nvs")).expect("the fixture is on disk");
    let (out, _) = ast(&["--json"], "recovered.nvs");
    let tree: Value = serde_json::from_str(&out).expect("the resilient default printed a document");

    // A trivium is a node of the same shape as any other, and its span is its
    // own bytes — the `//` included, which is what lets a panel print it.
    let comments: Vec<&Value> = subtree(&tree)
        .into_iter()
        .filter(|node| node["kind"] == "LineComment")
        .collect();
    assert_eq!(
        comments.len(),
        4,
        "the fixture's four `//` lines are in the tree: {out}"
    );
    for comment in comments {
        let [start, end] = span_of(comment);
        let source = &text[start..end];
        assert!(
            source.starts_with("//"),
            "a comment's span is the comment: {source:?}"
        );
        assert!(
            comment["children"]
                .as_array()
                .expect("a trivium has a `children`")
                .is_empty(),
            "a trivium contains nothing"
        );
    }

    // Placed under the innermost node that contains it, not flattened onto
    // the file: the whitespace inside the class body is the class's.
    let class = subtree(&tree)
        .into_iter()
        .find(|node| node["kind"] == "ClassDecl")
        .expect("the class is in the tree");
    assert!(
        subtree(class)
            .iter()
            .any(|node| node["kind"] == "Whitespace"),
        "the class body's whitespace is under the class: {class}"
    );

    // `rule:ide/recovery-is-explicit`, as a consumer sees it: the access the
    // parser recovered at says which form its member name took, so nothing
    // has to guess from an empty span.
    let recovered = subtree(&tree)
        .into_iter()
        .find(|node| node["kind"] == "PropertyAccess")
        .expect("the trailing `$u->` is in the tree");
    assert_eq!(
        recovered["member"], "missing",
        "the invented member name says so: {recovered}"
    );
}

/// The panel's whole reason for existing: the file that does not compile
/// still gets a document, and the diagnostics do not land in it
/// (`rule:ide/ast-json-schema-is-frozen`, `rule:ide/the-ast-panel-shells-out-to-the-cli`).
// covers: tools:cli/nvs-ast
#[test]
fn ast_json_renders_a_file_that_does_not_compile() {
    let text = std::fs::read_to_string(fixture("recovered.nvs")).expect("the fixture is on disk");
    let (out, err, ok) = run_ast(&["--json"], &fixture("recovered.nvs"));
    assert!(!ok, "the source has an error, so the command fails");
    assert!(
        err.contains("error"),
        "the diagnostic goes to standard error: {err}"
    );

    let tree: Value = serde_json::from_str(&out).expect("standard output is one JSON document");
    assert_eq!(
        span_of(&tree),
        [0, text.len()],
        "the root covers the whole file, error and all: {out}"
    );

    // Every statement of the file, not the prefix before the error: a panel
    // given only what parsed cleanly would be emptiest on the file it was
    // opened for.
    for kind in ["ClassDecl", "Echo", "PropertyAccess"] {
        assert!(
            subtree(&tree).iter().any(|node| node["kind"] == kind),
            "`{kind}` survived the error: {out}"
        );
    }
}

/// The schema, frozen over the corpus `rule:ide/ast-json-schema-is-frozen`
/// names: every node of every example is `kind`, `span`, `children` and
/// nothing but the scalars in [`SCALARS`], and one example's whole document
/// is frozen byte for byte so a reshaping that keeps the key set still fails.
// covers: tools:cli/nvs-ast
#[test]
fn ast_json_matches_its_frozen_schema() {
    let examples = examples();
    assert!(
        examples.len() > 20,
        "the examples corpus is on disk: {} file(s)",
        examples.len()
    );
    for path in &examples {
        let text = std::fs::read_to_string(path).expect("an example is UTF-8");
        let (out, _, _) = run_ast(&["--json"], path);
        let tree: Value = serde_json::from_str(&out)
            .unwrap_or_else(|err| panic!("{}: stdout is one document: {err}", path.display()));
        assert_eq!(
            tree["kind"],
            "File",
            "{}: the root is the file",
            path.display()
        );
        let root = [0, text.len()];
        assert_eq!(
            span_of(&tree),
            root,
            "{}: the root covers the file",
            path.display()
        );
        check_schema(&tree, root, path, !text.contains("|>"));
    }

    // The snapshot half: the corpus's smallest program, whole. Regenerate it
    // with `nvs ast --json examples/hello.nvs` the day that file is edited —
    // and read what changed before pasting it, since a diff here is either
    // that edit or a schema change.
    let (hello, _, ok) = run_ast(&["--json"], &example("hello.nvs"));
    assert!(ok, "the corpus's hello world compiles");
    assert_eq!(
        hello.trim_end(),
        r#"{"children":[{"children":[],"kind":"Whitespace","span":[5,6]},{"children":[{"children":[],"kind":"Whitespace","span":[10,11]},{"children":[],"kind":"Str","span":[11,26]}],"kind":"Echo","span":[6,27]},{"children":[],"kind":"Whitespace","span":[27,28]}],"kind":"File","span":[0,28]}"#,
        "the frozen document for `examples/hello.nvs`"
    );
}

/// Both notations over the attack written against this command: a sixty-link
/// chain, comments inside one expression, text that is not ASCII. The debug
/// notation writes a span as `file-index:start..end`, the document's root
/// covers every byte and the schema holds under it, and `--resilient` with
/// `--strict` is refused before the file is read.
// covers: tools:cli/nvs-ast
#[test]
fn ast_prints_both_notations_for_a_file_that_is_hard_to_print() {
    let path = nvs_repo::path("tests")
        .join("hostile")
        .join("tools")
        .join("cli")
        .join("nvs-ast")
        .join("01-a-file-that-is-hard-to-print.nvs");
    let text = std::fs::read_to_string(&path).expect("the attack is on disk");

    let (debug, err, ok) = run_ast(&[], &path);
    assert!(ok, "the attack parses without an error: {err}");
    assert!(
        debug.contains("span: 0:"),
        "a span is `file-index:start..end`: {debug}"
    );

    let (out, err, ok) = run_ast(&["--json"], &path);
    assert!(ok, "the attack parses without an error: {err}");
    let tree: Value = serde_json::from_str(&out).expect("standard output is one document");
    let root = [0, text.len()];
    assert_eq!(span_of(&tree), root, "the root covers every byte: {out}");
    check_schema(&tree, root, &path, true);
    for kind in ["BlockComment", "LineComment", "DocComment", "Paren"] {
        assert!(
            subtree(&tree).iter().any(|node| node["kind"] == kind),
            "the attack's `{kind}` is in the tree: {out}"
        );
    }

    let (out, err, ok) = run_ast(&["--resilient", "--strict"], &path);
    assert!(
        !ok && out.is_empty() && err.contains("cannot be used with"),
        "the two modes are refused together: {out}{err}"
    );
}

/// The scalar field names a node may carry beside `kind`, `span` and
/// `children` — `nvs_syntax::walk`'s `Field`s, and the whole of them.
///
/// A seventh arriving here is a schema change rather than a detail, so it is
/// added in this list deliberately, by the slice that adds the field, rather
/// than appearing in a consumer's output unannounced.
const SCALARS: [&str; 6] = ["byRef", "member", "nullsafe", "op", "target", "value"];

/// Asserts the schema over one subtree, `at` naming the file for the failure.
///
/// `in_source_order` is false for a file that writes `|>`. The parser
/// substitutes a pipeline's left side into the hole on its right
/// (`rule:expressions/pipeline-substitution`), so a subtree written earlier in
/// the file ends up under a call written after it: the emitted tree is the
/// nested spelling's, while every span still points at the characters the
/// author actually wrote. Sibling order and containment are properties of a
/// parse that follows the source, and that operator is the one construct that
/// deliberately does not — everything else the schema says is asserted over
/// such a file exactly as over any other.
fn check_schema(node: &Value, parent: [usize; 2], file: &Path, in_source_order: bool) {
    let at = file.display();
    assert!(
        node["kind"].as_str().is_some_and(|kind| !kind.is_empty()),
        "{at}: `kind` is a non-empty string: {node}"
    );
    for name in node
        .as_object()
        .unwrap_or_else(|| panic!("{at}: a node is an object: {node}"))
        .keys()
    {
        assert!(
            matches!(name.as_str(), "kind" | "span" | "children")
                || SCALARS.contains(&name.as_str()),
            "{at}: `{name}` is not in the frozen schema: {node}"
        );
    }

    let span = span_of(node);
    assert!(span[0] <= span[1], "{at}: a span runs forwards: {node}");
    assert!(
        !in_source_order || (parent[0] <= span[0] && span[1] <= parent[1]),
        "{at}: a node is inside its parent {parent:?}: {node}"
    );

    let mut previous = span[0];
    for child in node["children"]
        .as_array()
        .unwrap_or_else(|| panic!("{at}: `children` is an array: {node}"))
    {
        let start = span_of(child)[0];
        assert!(
            !in_source_order || start >= previous,
            "{at}: children are in offset order: {node}"
        );
        previous = start;
        check_schema(child, span, file, in_source_order);
    }
}

/// A node's span as `[start, end]`, asserting that it is one.
fn span_of(node: &Value) -> [usize; 2] {
    let span = node["span"]
        .as_array()
        .unwrap_or_else(|| panic!("`span` is an array: {node}"));
    assert_eq!(span.len(), 2, "`span` is `[start, end]`: {node}");
    let offset = |value: &Value| {
        let offset = value
            .as_u64()
            .unwrap_or_else(|| panic!("an offset is a number: {node}"));
        usize::try_from(offset).expect("an offset into a file this host read fits a `usize`")
    };
    [offset(&span[0]), offset(&span[1])]
}

/// Every node under `node`, itself excluded, in no particular order.
fn subtree(node: &Value) -> Vec<&Value> {
    fn descend<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
        for child in node["children"]
            .as_array()
            .expect("every node has a `children`")
        {
            out.push(child);
            descend(child, out);
        }
    }

    let mut out = Vec::new();
    descend(node, &mut out);
    out
}

/// `examples/<name>` under the repository root, whatever the shell's directory is.
fn example(name: &str) -> PathBuf {
    nvs_repo::path("examples").join(name)
}

/// Every `*.nvs` under `examples/`, sorted, subdirectories included.
fn examples() -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_examples(&nvs_repo::path("examples"), &mut out);
    out.sort();
    out
}

fn collect_examples(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("the examples tree is in the repository") {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            collect_examples(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "nvs") {
            out.push(path);
        }
    }
}
