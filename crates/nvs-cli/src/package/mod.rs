//! `nvs pkg` — every package command a user types
//! (`rule:packaging/package-toml-is-a-tool-input`).
//!
//! One module per subcommand under this one. [`api`] is
//! `rule:packaging/only-a-major-may-break-the-api`'s extraction: a package's public API, read
//! from the checked program and never by running any of it, and [`diff`] is its comparison: every
//! change between two versions, breaking or additive.

use std::path::PathBuf;
use std::process::ExitCode;

mod api;
mod diff;

/// Work with Novis packages.
#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(subcommand)]
    command: Command,
}

/// `nvs pkg`'s own subcommands.
#[derive(clap::Subcommand)]
enum Command {
    /// Print the public API of a package as JSON.
    ///
    /// The package is compiled and checked. None of its code runs. The JSON
    /// lists every class, interface, enum and type in the package's `src`
    /// directory, with every public and protected method, property and
    /// constant.
    Api {
        /// The package's directory, the one that contains `src`.
        dir: PathBuf,
    },
    /// Compare two versions of a package's public API.
    ///
    /// OLD and NEW are each a package directory or a JSON file that
    /// `nvs pkg api` printed. Each change prints on one line, as breaking or
    /// additive. The command exits with an error if a change is breaking.
    Diff {
        /// The older version: a package directory or an API JSON file.
        old: PathBuf,
        /// The newer version: a package directory or an API JSON file.
        new: PathBuf,
        /// Print the changes as JSON.
        #[arg(long)]
        json: bool,
    },
}

impl Args {
    /// Runs the `nvs pkg` subcommand clap parsed.
    pub(crate) fn run(self) -> ExitCode {
        match self.command {
            Command::Api { dir } => api::run(&dir),
            Command::Diff { old, new, json } => diff::run(&old, &new, json),
        }
    }
}
