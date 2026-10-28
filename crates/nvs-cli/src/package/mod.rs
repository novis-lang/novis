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
//! `package.lock` (`rule:packaging/the-lockfile-holds-every-digest`). [`fetch`] is `nvs pkg fetch`
//! and `nvs pkg vendor`: the graph written under `vendor/`, with `vendor/packages.nvs` and one grant
//! line per package (`rule:packaging/a-fetched-package-is-an-autoload-line`). [`minimum`] is
//! `nvs pkg add`, `nvs pkg update` and `nvs pkg outdated`, which choose a minimum among the
//! released versions, and [`report`] is what an `add` or an `update` prints before it writes.
//! [`download`] fetches, checks and places a package's `[[download]]` files
//! (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`). [`log`] is the
//! registry's transparency log in C2SP tlog-tiles, with its inclusion and consistency proofs
//! (`rule:packaging/the-registry-keeps-an-append-only-log`), and [`note`] signs and opens the C2SP
//! signed notes its checkpoints are.

use std::path::PathBuf;
use std::process::ExitCode;

mod api;
mod diff;
mod download;
mod fetch;
#[cfg(test)]
mod fixture;
mod git;
mod index;
mod lock;
mod log;
mod manifest;
mod minimum;
mod note;
mod pack;
mod report;
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
    /// Download the packages that `package.toml` needs into the `vendor` folder.
    ///
    /// Each package goes into `vendor/<vendor>/<name>`. When two packages need
    /// different versions of a third one, the higher version is used. The
    /// versions and the digest of every package are written to
    /// `package.lock`.
    ///
    /// The file `vendor/packages.nvs` has one `autoload` line for each
    /// package. Your program needs `require './vendor/packages.nvs';` once.
    ///
    /// For each new package, the command adds one line to the `[grants]`
    /// table in `package.toml`. The line gives the package no capabilities.
    /// Add a capability to the line to allow it. The command never changes a
    /// line that is already there.
    ///
    /// No code of a package runs.
    Fetch {
        /// Use only the versions in `package.lock`, and never change it. The
        /// command stops with an error if `package.lock` does not match.
        #[arg(long)]
        locked: bool,
    },
    /// Add a package to `package.toml` and download it.
    ///
    /// PACKAGE is the package's name, such as `acme/http`. Without a version,
    /// the newest released version is added. Write `acme/http@1.4.0` to add
    /// one exact version.
    ///
    /// A new version is released seven days after it is published, or when an
    /// owner confirms it. Before that, you can only add it with its exact
    /// version.
    ///
    /// The command prints the capabilities that the package and every package
    /// it needs request. Then it does what `nvs pkg fetch` does.
    Add {
        /// The package, such as `acme/http` or `acme/http@1.4.0`.
        package: String,
    },
    /// Raise the minimum version of packages to their newest released version.
    ///
    /// Without PACKAGES, every package in `[dependencies]` is raised. A
    /// minimum is only raised, never lowered.
    ///
    /// The command prints what changed since `package.lock`: the versions,
    /// the capabilities each package requests, the owners and the
    /// repositories. Then it does what `nvs pkg fetch` does.
    Update {
        /// The packages to raise, such as `acme/http`.
        packages: Vec<String>,
    },
    /// List the packages that have a newer released version.
    ///
    /// The command changes no file. Use `nvs pkg update` to raise a minimum.
    Outdated,
    /// Download a file and add it to the `[[download]]` list of your package.
    ///
    /// Use this for a file that is too large for the package, such as a font
    /// or a data set. The command downloads the file from URL, saves it at
    /// PATH in the package, and writes its URL, size and SHA-256 hash to
    /// `package.toml`. Run it in the folder of your package.
    ///
    /// When somebody fetches your package, `nvs pkg fetch` downloads the file
    /// and checks its size and hash. A file that does not match is not used.
    /// The URL must start with `https://`.
    Download {
        /// The file's URL, such as `https://example.com/fonts/Big.otf`.
        url: String,
        /// Where the file goes in the package, such as `fonts/Big.otf`.
        #[arg(long, value_name = "PATH")]
        to: String,
    },
    /// Add a file you downloaded yourself to the download cache.
    ///
    /// Use this when a package lists a file and none of its URLs work. Get
    /// the file in another way, then run this command. The file is only
    /// added when its size and SHA-256 hash match a file that a package in
    /// `package.lock` lists. Then run `nvs pkg fetch`.
    Provide {
        /// The file to add.
        file: PathBuf,
    },
    /// Write every package into the `vendor` folder again.
    ///
    /// This command does what `nvs pkg fetch` does, with one difference.
    /// `nvs pkg fetch` keeps a package folder whose files match
    /// `package.lock`. This command downloads every package again and
    /// replaces its folder. Use it when a file in `vendor` was changed.
    Vendor {
        /// Use only the versions in `package.lock`, and never change it. The
        /// command stops with an error if `package.lock` does not match.
        #[arg(long)]
        locked: bool,
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
            Command::Add { package } => minimum::add(&package),
            Command::Update { packages } => minimum::update(&packages),
            Command::Outdated => minimum::outdated(),
            Command::Fetch { locked } => fetch::run(fetch::Mode::Fetch, locked),
            Command::Vendor { locked } => fetch::run(fetch::Mode::Vendor, locked),
            Command::Pack { dir, out } => pack::run(&dir, out.as_deref()),
            Command::Download { url, to } => download::run_download(&url, &to),
            Command::Provide { file } => download::run_provide(&file),
        }
    }
}
