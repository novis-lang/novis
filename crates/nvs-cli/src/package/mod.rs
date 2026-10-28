//! `nvs pkg` — every package command a user types
//! (`rule:packaging/package-toml-is-a-tool-input`).
//!
//! One module per subcommand under this one. [`api`] is
//! `rule:packaging/only-a-major-may-break-the-api`'s extraction: a package's public API, read
//! from the checked program and never by running any of it, and [`diff`] is its comparison: every
//! change between two versions, breaking or additive, and whether the version step allows it.
//! [`manifest`] reads `package.toml`, a package's or an application's, and refuses an unknown key.
//! [`pack`] builds the deterministic archive whose digest is a package's identity
//! (`rule:packaging/a-package-is-its-digest`). [`index`] reads a registry's root file, index files
//! and archives, [`resolve`] selects the highest minimum of every package in the graph
//! (`rule:packaging/resolution-takes-the-highest-minimum`), and [`lock`] reads, writes and checks
//! `package.lock` (`rule:packaging/the-lockfile-holds-every-digest`).

use std::path::PathBuf;
use std::process::ExitCode;

mod api;
mod diff;
#[cfg(test)]
mod fixture;
#[allow(
    dead_code,
    reason = "the commands that resolve and lock are not written yet"
)]
mod git;
// `nvs pkg fetch` and `add` read a registry, resolve and lock; they are not written yet.
#[allow(
    dead_code,
    reason = "the commands that resolve and lock are not written yet"
)]
mod index;
#[allow(
    dead_code,
    reason = "the commands that resolve and lock are not written yet"
)]
mod lock;
// `nvs pkg diff` reads the version, and `nvs pkg pack` the name and `files`. The dependencies,
// the registries and the downloads are read by `nvs pkg fetch` and `add`, which are not written yet.
#[allow(
    dead_code,
    reason = "the commands that read most of a manifest are not written yet"
)]
mod manifest;
mod pack;
#[allow(
    dead_code,
    reason = "the commands that resolve and lock are not written yet"
)]
mod resolve;

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
    /// Build the archive that is published for a package.
    ///
    /// The archive contains every `.nvs` file under `src`, `package.toml`,
    /// the license and readme files next to it, and each file that the
    /// `files` list in `package.toml` names. Nothing else is packed.
    ///
    /// The command prints each file with its size, the total, and the
    /// package's digest. The same files always give the same archive and the
    /// same digest, on every system.
    ///
    /// A package may have at most 5,000 files of at most 16 MiB together.
    /// Every path must also work on Windows, macOS and Linux.
    Pack {
        /// The package's directory, the one that contains `package.toml`.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Where to write the archive. The default is
        /// `<vendor>-<name>-<version>.tar` in the current directory.
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
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
            Command::Pack { dir, out } => pack::run(&dir, out.as_deref()),
        }
    }
}
