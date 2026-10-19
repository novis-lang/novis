//! `nvs lsp-test` — a tree of `.lspt` cases, run against the language server.
//!
//! The suite itself is `nvs_lsp::suite`'s. This module owns only the
//! command's arguments, the runner directory and the exit code.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::tmp;

/// Run a tree of `.lspt` cases against the language server.
///
/// The editor-behaviour suite: each case is a document, a cursor, a request
/// and the answer frozen as text. It answers a different question from `nvs
/// test` and shares no summary with it — two suites, two counts, because a
/// number that meant both would mean neither.
///
/// Prints `N passed, M failed` and exits non-zero if any case failed.
// `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.
#[derive(clap::Args)]
pub(crate) struct Args {
    /// The case files and directories to run.
    #[arg(required = true)]
    paths: Vec<PathBuf>,
    /// Print the request × construct matrix instead of the summary line.
    ///
    /// Coverage is inferred from the node each case's question landed on
    /// and never declared by the case, so the matrix is a reading of the
    /// corpus rather than a list anyone maintains.
    // `rule:ide/lspt-coverage-is-inferred`.
    #[arg(long)]
    coverage: bool,
}

impl Args {
    /// Runs `nvs lsp-test` with the arguments clap parsed, and the global
    /// `--config` files that place its runner directory.
    pub(crate) fn run(self, config: &[PathBuf]) -> ExitCode {
        // The report is this terminal program's own output, which is where
        // a `.lspt` summary belongs: `rule:ide/stdout-belongs-to-the-protocol` gives stdout to
        // the protocol only in the process `nvs lsp` runs, and this is not
        // that process — `nvs_lsp::suite` writes to the sink it is handed
        // and names none.
        let mut out = std::io::stdout().lock();
        let report = if self.coverage {
            nvs_lsp::suite::Report::Coverage
        } else {
            nvs_lsp::suite::Report::Summary
        };
        let root = match tmp::runner_dir(config) {
            Ok(root) => root,
            Err(code) => return code,
        };
        match nvs_lsp::suite::run(&self.paths, &root, &mut out, report) {
            Ok(outcome) if outcome.summary.is_success() => ExitCode::SUCCESS,
            Ok(_) => ExitCode::FAILURE,
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        }
    }
}
