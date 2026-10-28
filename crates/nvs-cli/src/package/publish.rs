//! `nvs pkg publish` — a release sent to its registry as a repository URL, a commit, a version and
//! the package's folder (`rule:packaging/the-registry-builds-the-archive-from-git`). No file is
//! uploaded: the registry clones the commit and builds the archive itself with
//! `nvs registry build` ([`super::build`]).
//!
//! **What is sent is read from git** ([`Release::find`]): the commit is `HEAD` of the repository
//! that holds the nearest `package.toml`, the folder is that file's folder relative to the
//! repository's root, and the URL is the `origin` remote's, or `--repository`. The repository is
//! opened with gix's isolated options, so no configuration file, hook or helper program runs. A
//! URL that is not `https:` is refused here, as the registry refuses it.
//!
//! **Both forms first pack the commit locally**, with the build's own code: [`git::clone_within`]
//! over a `file:` URL of the same repository, and [`build::checked`]. So `--dry-run`'s digest is
//! the registry's, and what is not committed is not in it. Both download every `[[download]]`
//! entry from each of its URLs and refuse one whose size or SHA-256 hash differs
//! ([`super::download::Store::check_every_url`]), because the registry never downloads one.
//!
//! **`--dry-run` prints every file, its size, the total and the digest, and sends nothing.** The
//! other form sends `POST /api/publish` with the token `nvs pkg login` stored ([`login::stored`])
//! and prints the registry's answer: the release's digest and whether it is pending or released.
//! A digest that differs from the local one is reported, since the same commit must pack to the
//! same bytes. After the answer it prints the account's pending releases, as `nvs pkg whoami`
//! does ([`login::print_pending`]), and the registry's `warning` on standard error when no owner
//! device or webhook took the message about the release
//! (`rule:packaging/an-owner-is-notified-without-email`). The warning leaves the exit status
//! alone, because the release is published. A reply without either field, from an older
//! registry, prints neither.
//!
//! Cost: one archive in memory, at most [`super::pack::MAX_ARCHIVE_BYTES`], its files in a scratch
//! folder while it packs, each download once in the download cache, and one request. Nothing here
//! runs on the request path.

use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::{Map, Value};

use super::download::Store;
use super::fetch::{self, Failed};
use super::git::{self, Outbound};
use super::index::{Fault, fault};
use super::manifest::{self, Version};
use super::{build, login};

/// `nvs pkg publish`: the release packed and checked locally, then sent unless `dry_run`.
pub(crate) fn run(registry: Option<&str>, repository: Option<&str>, dry_run: bool) -> ExitCode {
    fetch::exit(publishing(registry, repository, dry_run))
}

/// What one publish sends.
struct Release {
    /// The repository's `https:` URL.
    url: String,
    /// The repository's root on this computer.
    root: PathBuf,
    /// `HEAD`'s full hash.
    commit: String,
    /// The package's folder relative to `root`, `/`-separated, empty for the root.
    dir: String,
}

impl Release {
    /// The release of the package whose `package.toml` is `manifest`, sent as `repository` or as
    /// the `origin` remote's URL.
    ///
    /// # Errors
    ///
    /// No git repository above the package, a repository with no commit, no URL, and a URL that
    /// is not `https:`.
    fn find(manifest: &Path, repository: Option<&str>) -> Result<Self, Fault> {
        let package = manifest
            .parent()
            .ok_or_else(|| fault("`package.toml` has no folder"))?;
        let Some(root) = package.ancestors().find(|dir| dir.join(".git").exists()) else {
            return Err(fault(format!(
                "`{}` is not in a git repository. The registry builds a package from a commit of \
                 its public repository.",
                package.display()
            )));
        };
        let repo = gix::open_opts(root, gix::open::Options::isolated()).map_err(|err| {
            fault(format!(
                "cannot open the repository `{}`: {err}",
                root.display()
            ))
        })?;
        let commit = repo
            .head_id()
            .map_err(|err| fault(format!("the repository has no commit to publish: {err}")))?
            .to_hex()
            .to_string();
        let url = match repository {
            Some(url) => url.to_owned(),
            None => {
                let remote = repo.find_remote("origin").map_err(|_| {
                    fault(
                        "the repository has no remote named `origin`. Name the repository's \
                         URL with `--repository https://...`.",
                    )
                })?;
                remote
                    .url(gix::remote::Direction::Fetch)
                    .map(|url| url.to_bstring().to_string())
                    .ok_or_else(|| fault("the remote `origin` has no URL"))?
            }
        };
        if !url.starts_with("https://") {
            return Err(fault(format!(
                "`{url}` is not an `https://` URL. The registry clones a repository over HTTPS \
                 only. Name the repository's HTTPS URL with `--repository https://...`."
            )));
        }
        let dir = package
            .strip_prefix(root)
            .map_err(|_| fault("the package's folder is not inside its repository"))?
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        Ok(Self {
            url,
            root: root.to_path_buf(),
            commit,
            dir,
        })
    }
}

/// [`run`], returning what stopped it.
fn publishing(
    registry: Option<&str>,
    repository: Option<&str>,
    dry_run: bool,
) -> Result<(), Failed> {
    let origin = login::origin(registry)?;
    let cwd = std::env::current_dir()
        .map_err(|err| fault(format!("cannot read the current folder: {err}")))?;
    let Some(path) = manifest::find(&cwd) else {
        return Err(fault(format!(
            "There is no `package.toml` in `{}` or a folder above it.",
            cwd.display()
        ))
        .into());
    };
    let release = Release::find(&path, repository)?;
    let scratch = nvs_config::data::current()
        .map(|folder| folder.tmp())
        .ok_or_else(|| fault("There is no data folder to pack in. Use `--data`."))?;
    std::fs::create_dir_all(&scratch)
        .map_err(|err| fault(format!("cannot make `{}`: {err}", scratch.display())))?;
    let local = format!("file:{}", release.root.display());
    let cloned = git::clone_within(
        &local,
        &release.commit,
        &release.dir,
        &release.root,
        &scratch,
        git::LIMITS,
        &Outbound::default(),
    )?;
    let version = build::checked(&cloned.manifest, None)?;
    let name = cloned
        .manifest
        .name
        .clone()
        .expect("`build::checked` refuses a manifest with no name");
    if !cloned.manifest.downloads.is_empty() {
        let outbound =
            Outbound::new(crate::config::package_network(None).map_err(Failed::Reported)?);
        let store = Store::current()?;
        for entry in &cloned.manifest.downloads {
            store.check_every_url(&name, entry, &outbound)?;
        }
    }
    let digest = format!("blake3:{}", cloned.digest.to_hex());
    let mut out = io::stdout().lock();
    // Standard output that cannot be written to has nobody to tell.
    let _ = writeln!(out, "{} {version}", name.slug());
    let _ = writeln!(out, "Repository: {}", release.url);
    let _ = writeln!(out, "Commit: {}", release.commit);
    if !release.dir.is_empty() {
        let _ = writeln!(out, "Folder: {}", release.dir);
    }
    if dry_run {
        let mut total = 0u64;
        let mut count = 0usize;
        for entry in tar::Archive::new(&cloned.archive[..])
            .entries()
            .map_err(|err| fault(format!("the archive does not read: {err}")))?
        {
            let entry = entry.map_err(|err| fault(format!("the archive does not read: {err}")))?;
            let path = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
            let _ = writeln!(out, "{:>10}  {path}", entry.size());
            total += entry.size();
            count += 1;
        }
        let _ = writeln!(out, "{total:>10}  total, {count} files");
        let _ = writeln!(out, "Digest: {digest}");
        let _ = writeln!(out, "This was a dry run. Nothing was sent to `{origin}`.");
        return Ok(());
    }
    let token = login::stored(&origin)?;
    let sent = send(&origin, &token, &release, version)?;
    if sent.digest != digest {
        let _ = writeln!(
            out,
            "Warning: the registry built the digest {}, and this computer packs {digest}. The \
             same commit should give the same digest.",
            sent.digest
        );
    }
    let _ = writeln!(out, "Digest: {}", sent.digest);
    let _ = writeln!(out, "{}", sent.state);
    login::print_pending(&mut out, &sent.pending);
    let _ = out.flush();
    if let Some(warning) = &sent.warning {
        eprintln!("Warning: {}", login::printable(warning));
    }
    Ok(())
}

/// What the registry answered a publish.
struct Sent {
    /// The digest of the archive the registry built.
    digest: String,
    /// What happens to the release next, as the registry says it.
    state: String,
    /// The account's releases that are not selectable yet.
    pending: Vec<login::Pending>,
    /// The registry's sentence when no owner device or webhook took the message.
    warning: Option<String>,
}

/// `release` at `version` sent to the registry at `origin` with `token`.
///
/// # Errors
///
/// A request that fails, a token the registry does not accept, a refusal, which the fault quotes,
/// and a reply that is not what a registry returns.
fn send(origin: &str, token: &str, release: &Release, version: Version) -> Result<Sent, Fault> {
    let mut body = Map::new();
    body.insert("repository".into(), Value::from(release.url.as_str()));
    body.insert("commit".into(), Value::from(release.commit.as_str()));
    body.insert("version".into(), Value::from(version.to_string()));
    body.insert("directory".into(), Value::from(release.dir.as_str()));
    let body = serde_json::to_vec(&Value::Object(body)).expect("a tree of strings serialises");
    let (status, reply) = login::send(origin, token, "/api/publish", Some(body))?;
    let json: Option<Value> = serde_json::from_slice(&reply).ok();
    let text = |key: &str| {
        json.as_ref()
            .and_then(|json| json.get(key))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    match status {
        200..=299 => {}
        401 => {
            return Err(fault(format!(
                "The registry `{origin}` does not accept your stored token. The token is expired \
                 or revoked. Run `nvs pkg login` again with a new token."
            )));
        }
        status => {
            let why = text("error").unwrap_or_else(|| format!("it returned status {status}"));
            return Err(fault(format!(
                "The registry `{origin}` did not publish the release: {why}"
            )));
        }
    }
    let bad = || {
        fault(format!(
            "The reply of `{origin}/api/publish` is not what a registry returns."
        ))
    };
    Ok(Sent {
        digest: text("digest").ok_or_else(bad)?,
        state: text("message").ok_or_else(bad)?,
        pending: json.as_ref().and_then(login::pending_of).ok_or_else(bad)?,
        warning: text("warning").filter(|warning| !warning.trim().is_empty()),
    })
}
