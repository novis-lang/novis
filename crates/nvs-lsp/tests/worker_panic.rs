//! A panic on an editor worker stays on the worker.
//!
//! The test switch `nvs_lsp::panic_on_worker` makes every worker job of one
//! file panic. The index logs that entry once, skips it, and never analyses it
//! on its own thread. The other entry is indexed, and the index answers. A
//! refresh of the file sends it again.

use std::fs;
use std::path::PathBuf;

use nvs_lsp::{CheckScope, Documents, SymbolIndex};
use nvs_test::temp::TempDir;

/// Acceptance `worker-panic`: a part that panics on a worker is logged, its
/// entry is skipped, and the index answers the next request.
#[test]
fn a_part_that_panics_on_a_worker_is_logged_and_its_entry_is_skipped() {
    let dir = TempDir::new(nvs_repo::scratch("lsp-worker-panic"));
    fs::write(
        dir.at("calm.nvs"),
        "<?nvs\nclass Calm {\n    public function a(): int { return 1; }\n}\n",
    )
    .expect("a fixture file");
    fs::write(
        dir.at("boom.nvs"),
        "<?nvs\nclass Boom {\n    public function b(): int { return 2; }\n}\n",
    )
    .expect("a fixture file");

    let mut documents = Documents::new();
    documents.survey(CheckScope::Workspace, Some(dir.root()));
    let mut index = SymbolIndex::planned(&documents, CheckScope::Workspace, Some(dir.root()));
    index.use_workers(2);
    nvs_lsp::panic_on_worker(Some(PathBuf::from("boom.nvs")));
    index.settle(&documents);
    nvs_lsp::panic_on_worker(None);

    assert!(index.pending().is_empty(), "the index settled");
    let logged = index.take_panicked();
    assert_eq!(
        logged.len(),
        1,
        "one entry panicked, logged once: {logged:?}"
    );
    assert!(
        logged[0].ends_with("boom.nvs"),
        "the log names the file: {logged:?}"
    );
    assert!(index.take_panicked().is_empty(), "an entry is logged once");
    assert!(
        index.declaration("Boom").is_none(),
        "the entry was never analysed on the server's own thread"
    );
    assert!(
        index.declaration("Calm").is_some(),
        "the other entry is indexed, and the index answers"
    );

    // Its file changes: the entry is sent again, and this time it is indexed.
    let boom = logged[0].clone();
    documents.resurvey(&boom);
    index.refresh(&documents, &boom);
    index.settle(&documents);
    assert!(
        index.declaration("Boom").is_some(),
        "a refresh of the file ends the skip"
    );
}
