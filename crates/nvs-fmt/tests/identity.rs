//! Every corpus file is already formatted, and a file that does not parse is
//! refused.
//!
//! Over the corpus rather than over a fixture: every `.nvs` file under a tree
//! of `tools/data/nvs-fmt-corpus.txt`, its skips aside, is the formatter's own
//! output. `crates/nvs-syntax/tests/lossless.rs` proves the tokens and the
//! trivia tile every file, and this proves the layout rules hold over them.
//!
//! A change to a layout rule is therefore a corpus edit in the same commit,
//! and `bun nv nvs-fmt` makes it: it formats every corpus file and names each
//! expectation file beside a rewritten one that can quote a line the reformat
//! moved, with the command that writes it again. Reading the diff before
//! committing it is the whole review.

mod common;

use nvs_diagnostics::SourceMap;
use nvs_fmt::format;

#[test]
fn the_identity_printer_reproduces_every_corpus_file() {
    let corpus = common::files(true);

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
        "{} corpus file(s) came back changed — a layout rule that lands is a corpus edit in \
         the same session, and `bun nv nvs-fmt` makes it:\n{}",
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
