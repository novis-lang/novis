//! The gate `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case` asks for: the
//! coverage matrix over `tests/lsp/` has no empty cell, and a red run names the
//! cells nobody wrote a case for.
//!
//! Here rather than in a unit test because the subject is the **corpus**, not
//! the module: `crate::coverage`'s own tests build a matrix by hand out of two
//! recorded rows, which says the bookkeeping is right and nothing at all about
//! what the cases on disk reached. This one answers every `.lspt` file the way
//! `nvs lsp-test` does and reads the result — the same pass, since
//! `nvs_lsp::suite::Outcome` carries the matrix beside the count precisely so a
//! caller wanting both does not run the corpus twice.
//!
//! What "no empty cell" means is `crate::coverage`'s module doc, § *Decision:
//! what "no empty cell" can mean, and what it cannot*, and this file asserts
//! that reading rather than restating it: the whole cross product is not
//! achievable by any corpus, so the claim is split into a strong half over the
//! cursor rows and a weaker one over the document-wide rows, and
//! `Request::takes_cursor` is the only thing that decides which a request is in.

use std::path::PathBuf;

use nvs_lsp::suite::{self, Report};
use nvs_lsp::{Request, coverage::Matrix};

/// The corpus: `tests/lsp` under the repository root.
fn corpus() -> PathBuf {
    nvs_repo::path("tests/lsp")
}

/// Every construct any case reached, and the matrix it was read off.
///
/// A failing case is fatal here rather than merely reported: coverage is
/// credited only for a case whose answer equalled its `--EXPECT--`, so a red
/// corpus produces a matrix with holes that describe the failure rather than
/// the gap, and reporting those cells as missing cases would send the next
/// session to write cases that already exist.
fn matrix() -> Matrix {
    let mut report = Vec::new();
    let scratch = nvs_repo::scratch("lspt-corpus");
    let outcome = suite::run(
        &[corpus()],
        &scratch.join("cases"),
        &mut report,
        Report::Summary,
    )
    .expect("the corpus is readable and the report is written to memory");
    assert!(
        outcome.summary.is_success(),
        "the coverage matrix is only readable over a corpus that passes:\n{}",
        String::from_utf8_lossy(&report),
    );
    outcome.matrix
}

/// The matrix has no empty cell, in the two senses that can be asked for.
///
/// One test rather than two because there is one claim: the corpus answers
/// every request at every construct it knows about. Splitting it would let a
/// run report half of a gap and pass the other half, and the message below is
/// the whole point of the gate — it names the cells, so the work it asks for is
/// a list rather than a search.
#[test]
fn every_request_answers_every_construct() {
    let matrix = matrix();
    let constructs = matrix.constructs();
    assert!(
        !constructs.is_empty(),
        "a corpus that reached no construct at all is a broken run, not a full matrix",
    );

    let mut missing = Vec::new();

    // A cursor goes anywhere, so each of these owes the whole vocabulary.
    for request in Request::ALL.iter().filter(|it| it.takes_cursor()) {
        for construct in &constructs {
            if matrix.count(*request, construct) == 0 {
                missing.push(format!("  {} at {construct}", request.name()));
            }
        }
    }

    // The document-wide requests owe the vocabulary between them, each row
    // being bounded by what its own answer can name.
    let wide: Vec<Request> = Request::ALL
        .iter()
        .filter(|it| !it.takes_cursor())
        .copied()
        .collect();
    for construct in &constructs {
        if wide
            .iter()
            .all(|request| matrix.count(*request, construct) == 0)
        {
            missing.push(format!(
                "  {construct} is reached by no whole-document request"
            ));
        }
    }
    for request in &wide {
        if constructs
            .iter()
            .all(|construct| matrix.count(*request, construct) == 0)
        {
            missing.push(format!("  {} reaches no construct at all", request.name()));
        }
    }

    assert!(
        missing.is_empty(),
        "the coverage matrix has {} empty cell(s), each a `.lspt` case nobody wrote:\n{}\n\n{}",
        missing.len(),
        missing.join("\n"),
        matrix.render(),
    );
}
