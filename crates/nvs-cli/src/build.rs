//! `nvs build` — a build artifact from a checked program.
//!
//! The two artifacts are made elsewhere: the OpenAPI document by
//! [`crate::run_build`], and the single-file executable by
//! [`crate::bundle::build`]. This module owns the command's arguments and
//! which of the two a command line names.

use std::path::PathBuf;
use std::process::ExitCode;

/// Produce a build artifact from a checked program.
///
/// Naming an artifact is required rather than defaulted: `nvs build` with
/// nothing named would be a subcommand that succeeds having done nothing,
/// and the group is how the next artifact joins without changing what this
/// invocation means.
// `rule:routing/api-document-is-a-deterministic-build-artifact` spells the
// OpenAPI command, and
// `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`
// the bundle.
#[derive(clap::Args)]
#[command(group = clap::ArgGroup::new("artifact").required(true).args(["openapi", "compile"]))]
pub(crate) struct Args {
    /// The entry point of the program to build.
    file: PathBuf,
    /// Write the OpenAPI 3.1 document generated from this program's route
    /// table to standard output.
    // `rule:routing/api-document-is-generated-from-the-route-table`.
    #[arg(long)]
    openapi: bool,
    /// Write a portable single-file executable: this program's `require`
    /// graph as source, appended to a copy of the `nvs` host binary.
    // `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`.
    #[arg(long)]
    compile: bool,
    /// Where to write the executable (default: the entry file's stem).
    #[arg(short = 'o', long, value_name = "PATH", requires = "compile")]
    output: Option<PathBuf>,
}

impl Args {
    /// Runs `nvs build` with the arguments clap parsed, and the global
    /// `--config` files a bundle carries.
    pub(crate) fn run(self, config: &[PathBuf]) -> ExitCode {
        if self.compile {
            crate::bundle::build(&self.file, self.output.as_deref(), config)
        } else {
            crate::run_build(&self.file, self.openapi)
        }
    }
}
