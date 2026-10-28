//! `nvs pkg` — every package command a user types
//! (`rule:packaging/package-toml-is-a-tool-input`).
//!
//! One module per subcommand under this one. [`api`] is
//! `rule:packaging/only-a-major-may-break-the-api`'s extraction: a package's public API, read
//! from the checked program and never by running any of it, and [`diff`] is its comparison: every
//! change between two versions, breaking or additive, and whether the version step allows it.
//! [`manifest`] reads `package.toml`, a package's or an application's, and refuses an unknown key.

use std::path::PathBuf;
use std::process::ExitCode;

mod api;
mod diff;
// `nvs pkg diff` reads the version alone. The name, the dependencies, the registries and the
// downloads are read by `nvs pkg pack`, `fetch` and `add`, which are not written yet.
#[allow(
    dead_code,
    reason = "the commands that read most of a manifest are not written yet"
)]
mod manifest;

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
    /// additive.
    ///
    /// The version numbers decide which changes are allowed. Each version is
    /// the `version` in the directory's `package.toml`. `--from` and `--to`
    /// replace them. A major release may change anything. A minor or a patch
    /// release may only add. A `0.0.x` version is experimental, so nothing is
    /// checked. Without a version, every breaking change is an error.
    ///
    /// The command exits with an error if a change is not allowed.
    Diff {
        /// The older version: a package directory or an API JSON file.
        old: PathBuf,
        /// The newer version: a package directory or an API JSON file.
        new: PathBuf,
        /// The version number of OLD, such as `1.4.0`.
        #[arg(long, value_name = "VERSION")]
        from: Option<manifest::Version>,
        /// The version number of NEW, such as `1.5.0`.
        #[arg(long, value_name = "VERSION")]
        to: Option<manifest::Version>,
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
            Command::Diff {
                old,
                new,
                from,
                to,
                json,
            } => diff::run(&old, &new, from, to, json),
        }
    }
}
