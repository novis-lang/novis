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
//! (`rule:packaging/the-registry-keeps-an-append-only-log`), [`entry`] is the closed list of
//! events it records, and [`note`] signs and opens the C2SP signed notes its checkpoints are.
//! [`proof`] is the client's side of the log: the checkpoint it keeps per registry, the fork
//! check, and the inclusion proof of every archive it fetches. The registry's own commands over the log are `crate::registry`'s, outside `nvs pkg`.
//! [`login`] is `nvs pkg login` and `nvs pkg whoami`: the publish token stored in the data folder
//! and checked with the registry (`rule:packaging/a-publish-token-is-scoped-and-expires`).
//! [`publish`] is `nvs pkg publish`, which sends a commit for the registry to build, and
//! [`build`] is that build, `nvs registry build`
//! (`rule:packaging/the-registry-builds-the-archive-from-git`). [`retract`] is `nvs pkg retract`
//! (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`) and [`unlist`] is
//! `nvs pkg unlist` (`rule:packaging/an-owner-may-unlist`), each one request to the registry.
//! [`advisory`] is the registry's signed advisory feed, every retraction and withhold in the log,
//! and its check against the log, and [`audit`] is `nvs pkg audit`, which reports each one a
//! locked version is under.

use std::path::PathBuf;
use std::process::ExitCode;

pub(crate) mod advisory;
mod api;
mod audit;
pub(crate) mod build;
mod diff;
mod download;
pub(crate) mod entry;
mod fetch;
#[cfg(test)]
mod fixture;
mod git;
pub(crate) mod index;
mod lock;
pub(crate) mod log;
mod login;
mod manifest;
mod minimum;
pub(crate) mod note;
mod pack;
mod proof;
mod publish;
mod report;
mod resolve;
mod retract;
mod unlist;

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
    /// List each security advisory for the versions in `package.lock`.
    ///
    /// A registry publishes an advisory when it retracts or withholds a
    /// version. For each locked version with an advisory, the command
    /// prints the reason and the smallest version that fixes it. The command
    /// checks that the registry's list of advisories matches its public log.
    ///
    /// A registry can name a second copy of its log's checkpoint on another
    /// host. The command checks that both copies show the same log.
    ///
    /// The command also warns about code in a namespace that no line of
    /// `[grants]` matches. That code has every permission of your application.
    ///
    /// The command exits with an error while a locked version has an
    /// advisory, so you can run it in CI.
    Audit {
        /// Print the report as JSON.
        #[arg(long)]
        json: bool,
    },
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
    /// Store your publish token for a registry.
    ///
    /// Create the token on the registry's website. Then run this command
    /// and paste the token. You can also pipe it in, such as
    /// `nvs pkg login < token.txt`.
    ///
    /// The command reads the token from standard input. A token on the
    /// command line is visible to other programs and in your shell history.
    ///
    /// The registry checks the token first. Then the command stores it in
    /// the data folder, where only your account can read it. It prints the
    /// account and the scope, which is the vendor or the package the token
    /// can publish.
    Login {
        /// The registry's URL. The default is `https://registry.novis-lang.org`.
        #[arg(long, value_name = "URL")]
        registry: Option<String>,
    },
    /// Show the account and the scope of your stored publish token.
    ///
    /// The command sends the stored token to the registry. It prints the
    /// account, the scope and the date the token expires. Run
    /// `nvs pkg login` first to store a token.
    Whoami {
        /// The registry's URL. The default is `https://registry.novis-lang.org`.
        #[arg(long, value_name = "URL")]
        registry: Option<String>,
    },
    /// Publish the package at the last commit of its git repository.
    ///
    /// The command sends the repository's URL, the commit, the version and
    /// the package's folder in the repository. It sends no file. The
    /// registry downloads the commit and builds the package from it, so
    /// push the commit first. Changes that are not committed are not
    /// published.
    ///
    /// Before it sends anything, the command builds the package from the
    /// commit on this computer. It also downloads each `[[download]]` file
    /// from every URL and checks its size and hash. Run `nvs pkg login`
    /// first to store a publish token.
    Publish {
        /// The registry's URL. The default is `https://registry.novis-lang.org`.
        #[arg(long, value_name = "URL")]
        registry: Option<String>,
        /// The repository's public URL, starting with `https://`. The
        /// default is the URL of the remote `origin`.
        #[arg(long, value_name = "URL")]
        repository: Option<String>,
        /// Build the package and print every file, its size, the total and
        /// the digest, and send nothing. The digest is the one the registry
        /// builds from the same commit.
        #[arg(long)]
        dry_run: bool,
    },
    /// Retract a version of your package that is broken or unsafe.
    ///
    /// Run this command in the folder of your package. A retracted version
    /// can still be downloaded, so a `package.lock` that names it still
    /// works. `nvs pkg add` and `nvs pkg update` do not choose it.
    ///
    /// With a publish token, the retraction starts seven days later. An
    /// owner can make it start at once with a passkey on the registry's
    /// website. Run `nvs pkg login` first to store a publish token.
    Retract {
        /// The version to retract, such as `1.4.0`.
        version: manifest::Version,
        /// Why the version is retracted. The package index shows this text
        /// with the version. It can have at most 500 characters.
        #[arg(long, value_name = "TEXT")]
        reason: String,
        /// The version that fixes the problem, such as `1.4.1`. It must be
        /// a published version of the package.
        #[arg(long, value_name = "VERSION")]
        fixed_in: Option<manifest::Version>,
        /// The registry's URL. The default is `https://registry.novis-lang.org`.
        #[arg(long, value_name = "URL")]
        registry: Option<String>,
    },
    /// Hide your package or one version of it from search and listings.
    ///
    /// Run this command in the folder of your package. Without VERSION,
    /// every version is hidden. An unlisted version can still be
    /// downloaded, so a `package.lock` that names it still works.
    ///
    /// Only an owner who signs in with a passkey can unlist. With a publish
    /// token, the command prints the page on the registry's website where
    /// an owner confirms it.
    Unlist {
        /// The version to hide, such as `1.4.0`. Without it, every version
        /// is hidden.
        version: Option<manifest::Version>,
        /// Show the package or the version in search and listings again.
        #[arg(long)]
        undo: bool,
        /// The registry's URL. The default is `https://registry.novis-lang.org`.
        #[arg(long, value_name = "URL")]
        registry: Option<String>,
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
            Command::Audit { json } => audit::run(json),
            Command::Fetch { locked } => fetch::run(fetch::Mode::Fetch, locked),
            Command::Vendor { locked } => fetch::run(fetch::Mode::Vendor, locked),
            Command::Pack { dir, out } => pack::run(&dir, out.as_deref()),
            Command::Download { url, to } => download::run_download(&url, &to),
            Command::Provide { file } => download::run_provide(&file),
            Command::Login { registry } => login::login(registry.as_deref()),
            Command::Whoami { registry } => login::whoami(registry.as_deref()),
            Command::Publish {
                registry,
                repository,
                dry_run,
            } => publish::run(registry.as_deref(), repository.as_deref(), dry_run),
            Command::Retract {
                version,
                reason,
                fixed_in,
                registry,
            } => retract::run(registry.as_deref(), version, &reason, fixed_in),
            Command::Unlist {
                version,
                undo,
                registry,
            } => unlist::run(registry.as_deref(), version, undo),
        }
    }
}
