//! The printer is the identity, and a file that does not parse is refused.
//!
//! Goal stage 2's two claims, over the corpus rather than over a fixture:
//! `crates/nvs-syntax/tests/lossless.rs` proves the tokens and the trivia tile
//! every file in it, and these are that proof turned into a program — the
//! bytes `nvs fmt` answers today are the bytes it was given, and every layout
//! rule that lands later is a departure from this baseline it has to state.

use std::path::{Path, PathBuf};

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

/// The repository root, from this crate's manifest.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.nvs` file under `dir`, sorted, so a failure names the same file on
/// every machine.
fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("nvs") {
            out.push(path);
        }
    }
}

#[test]
fn the_identity_printer_reproduces_every_corpus_file() {
    let root = repo_root();
    let mut corpus = Vec::new();
    collect(&root.join("examples"), &mut corpus);
    collect(&root.join("tests"), &mut corpus);
    corpus.sort();

    let mut changed = Vec::new();
    let mut formatted = 0_usize;
    for path in &corpus {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let mut map = SourceMap::new();
        let id = map.add(path.display().to_string(), &text);
        // A corpus file that does not parse is one of the deliberately broken
        // ones under `tests/`, and the refusal is the other test's claim.
        let Ok(output) = format(map.file(id)) else {
            continue;
        };
        formatted += 1;
        if output != text {
            changed.push(path.display().to_string());
        }
    }

    assert!(
        formatted > 50,
        "only {formatted} corpus file(s) were formatted — this test is measuring nothing"
    );
    assert!(
        changed.is_empty(),
        "{} corpus file(s) came back changed:\n{}",
        changed.len(),
        changed.join("\n")
    );
}

#[test]
fn a_file_with_a_syntax_error_is_refused_and_left_unchanged() {
    let source = "<?nvs\nfunction broken( {\n";
    let mut map = SourceMap::new();
    let id = map.add("broken.nvs".to_string(), source);
    let file = map.file(id);

    let refusal = format(file).expect_err("a file that does not parse is never formatted");
    assert_eq!(refusal.name(), "broken.nvs");
    assert!(
        refusal.diagnostics().has_errors(),
        "a refusal carries the errors that caused it"
    );
    assert_eq!(
        file.text(),
        source,
        "the file a refusal names is handed back untouched"
    );
}
