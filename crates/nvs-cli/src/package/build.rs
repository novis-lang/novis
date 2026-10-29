//! `nvs registry build` — the archive of one release, built by the registry from a git commit
//! (`rule:packaging/the-registry-builds-the-archive-from-git`).
//!
//! **The clone is [`git::clone_within`]'s**, over `https:` alone: a `file:` or `http:` URL is
//! refused before anything connects, because a publisher wrote it. It fetches the one commit with
//! no history, inside [`git::LIMITS`], and connects only where the outbound address table allows,
//! with the working directory's `[capabilities.net] internal` key as the operator's exception.
//! `--ca-file` adds one trust anchor to the TLS client for a git host behind the operator's own
//! certificate authority ([`crate::config::package_network`]). It is the operator's flag, which
//! the service passes from its own configuration and never from a publisher.
//!
//! **The archive is [`super::pack`]'s**, of the package's folder within the repository, so
//! `nvs pkg publish --dry-run` packs the same bytes from the same commit. Every refusal of the pack
//! is the build's: the limits, the license file, a path another platform cannot hold, an
//! `nvs.toml`, a malformed `[[download]]` entry. The registry never downloads one.
//!
//! **Then the build checks what only a release needs** ([`checked`]): a `[package] name` and a
//! `version`, the version the publisher sent, no git dependency, every declaration inside the
//! package's namespace, and, given the previous release's API document, no break the version step
//! does not allow ([`super::diff::report`]). The API is read from the archive's own files,
//! unpacked into a scratch folder, so what is checked is what is served.
//!
//! **What it prints is one JSON document** on standard output, for the service: `"ok": true` with
//! the name, the version, the digest, every file, the dependencies with their minimums, the
//! requested capabilities in the index file's shape, the API report, and what the registry's
//! search reads: `doc_coverage`, how many public declarations carry a doc comment, of how many;
//! `docs`, their text; and `summary`, the readme's first paragraph ([`api::Docs`], [`summary`]).
//! The package page reads three more: `readme`, the readme's text ([`readme`]); `downloads`, each
//! `[[download]]` entry as written; and `supersedes`, the package this one replaces, or `null`.
//! Or it prints `"ok": false` with the
//! refusal as a sentence the service shows the publisher, and exit `1`. A compile error's
//! diagnostics go to standard error, as `nvs pkg api` prints them. `--archive` and `--api` write
//! the archive and its API document for the service to store, each through a temporary file.
//!
//! Cost: the archive in memory, at most [`super::pack::MAX_ARCHIVE_BYTES`], the clone and the
//! unpacked files in scratch folders removed on every way out, and one compile of the package.
//! The build is a child process of the service, so all of it ends with the process. Nothing here
//! runs on the request path.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rand::RngExt as _;
use serde_json::{Map, Value};

use super::git::{self, Outbound};
use super::index::{Fault, fault};
use super::manifest::{Manifest, Source, Version};
use super::{api, diff, fetch};

/// What the JSON document's `schema` is. A change to its keys raises it: `2` added
/// `doc_coverage`, `docs` and `summary`, and `3` added `readme`, `downloads` and `supersedes`.
pub(crate) const SCHEMA: u64 = 3;

/// The most bytes [`summary`] keeps. A search result shows it on one or two lines.
const MAX_SUMMARY: usize = 280;

/// The most bytes of the readme [`readme`] keeps. The registry stores it with each release, so it
/// is the most a release adds to the database for its page.
const MAX_README: usize = 64 * 1024;

/// One build's inputs, as `nvs registry build` takes them.
#[derive(clap::Args, Debug)]
pub(crate) struct Build {
    /// The repository's URL. It must start with `https://`.
    url: String,
    /// The commit to build, as its full hash of 40 characters.
    commit: String,
    /// The package's folder in the repository, such as `packages/http`.
    /// The default is the repository's root.
    #[arg(long, value_name = "DIR")]
    dir: Option<String>,
    /// The version the publisher sent. The build fails if `package.toml`
    /// has another version.
    #[arg(long, value_name = "VERSION")]
    version: Option<Version>,
    /// The API file of the previous release, which `--api` wrote. The build
    /// fails if the new version breaks that API and is not a new major
    /// version.
    #[arg(long, value_name = "FILE", requires = "previous_version")]
    previous: Option<PathBuf>,
    /// The version of the previous release.
    #[arg(long, value_name = "VERSION")]
    previous_version: Option<Version>,
    /// Write the archive to this file.
    #[arg(long, value_name = "FILE")]
    archive: Option<PathBuf>,
    /// Write the package's API to this file.
    #[arg(long, value_name = "FILE")]
    api: Option<PathBuf>,
    /// A PEM file with one more certificate authority to trust when the
    /// build connects to the git server.
    #[arg(long, value_name = "FILE")]
    ca_file: Option<PathBuf>,
}

/// Why a build was refused, and the API report when a break is the reason.
struct Refused {
    refusal: String,
    api: Option<Value>,
}

impl From<Fault> for Refused {
    fn from(fault: Fault) -> Self {
        Self {
            refusal: fault.0,
            api: None,
        }
    }
}

/// `nvs registry build`: the document printed, and success; or the refusal printed, and failure.
pub(crate) fn run(build: &Build) -> ExitCode {
    let outbound = match crate::config::package_network(build.ca_file.as_deref()) {
        Ok(written) => Outbound::new(written),
        Err(code) => return code,
    };
    let (document, code) = match built(build, &outbound) {
        Ok(document) => (document, ExitCode::SUCCESS),
        Err(refused) => {
            let mut out = Map::new();
            out.insert("schema".into(), Value::from(SCHEMA));
            out.insert("ok".into(), Value::from(false));
            out.insert("refusal".into(), Value::from(refused.refusal));
            out.insert("api".into(), refused.api.unwrap_or(Value::Null));
            (Value::Object(out), ExitCode::FAILURE)
        }
    };
    let text = serde_json::to_string_pretty(&document).expect("a tree of strings serialises");
    println!("{text}");
    code
}

/// [`run`]'s document, or why the build was refused.
fn built(build: &Build, outbound: &Outbound) -> Result<Value, Refused> {
    let url = build.url.as_str();
    if !url.starts_with("https://") {
        return Err(fault(format!(
            "`{url}` is not an `https://` URL. The registry clones a repository over HTTPS only."
        ))
        .into());
    }
    let commit = build.commit.as_str();
    if commit.len() != 40
        || !commit
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(fault(format!(
            "`{commit}` is not a full commit hash. A commit hash is 40 characters from 0 to 9 \
             and a to f."
        ))
        .into());
    }
    let scratch = nvs_config::data::current()
        .map(|folder| folder.tmp())
        .ok_or_else(|| fault("There is no data folder to clone into. Use `--data`."))?;
    fs::create_dir_all(&scratch)
        .map_err(|err| fault(format!("cannot make `{}`: {err}", scratch.display())))?;
    let dir = build
        .dir
        .as_deref()
        .unwrap_or_default()
        .trim_end_matches('/');
    let cloned = git::clone_within(url, commit, dir, &scratch, &scratch, git::LIMITS, outbound)?;
    let version = checked(&cloned.manifest, build.version)?;
    let name = cloned
        .manifest
        .name
        .as_ref()
        .expect("`checked` refuses a manifest with no name");
    let unpacked = Unpacked::new(&scratch, &cloned.archive)?;
    let (document, docs) = api::extract_counted(&unpacked.0).map_err(|_| {
        fault(format!(
            "`{name}` does not compile at the commit `{commit}`. The build's standard error has \
             the errors."
        ))
    })?;
    inside(&document, name.namespace())?;
    let report = match &build.previous {
        None => Value::Null,
        Some(previous) => {
            let old = diff::side(previous).map_err(|_| {
                fault(format!(
                    "the previous release's API document `{}` cannot be read. The build's \
                     standard error says why.",
                    previous.display()
                ))
            })?;
            let (report, allowed) =
                diff::report(&old, &document, build.previous_version, Some(version));
            if !allowed {
                let from = build
                    .previous_version
                    .map_or_else(|| "the previous release".to_owned(), |v| v.to_string());
                return Err(Refused {
                    refusal: format!(
                        "{version} breaks the API of {from}. Only a new major version may \
                         break it. `api.changes` lists every change."
                    ),
                    api: Some(report),
                });
            }
            report
        }
    };
    if let Some(path) = &build.archive {
        write(path, &cloned.archive)?;
    }
    if let Some(path) = &build.api {
        let text = serde_json::to_string_pretty(&document).expect("a tree of strings serialises");
        write(path, text.as_bytes())?;
    }
    let mut out = Map::new();
    out.insert("schema".into(), Value::from(SCHEMA));
    out.insert("ok".into(), Value::from(true));
    out.insert("name".into(), Value::from(name.to_string()));
    out.insert("version".into(), Value::from(version.to_string()));
    out.insert("repository".into(), Value::from(url));
    out.insert("commit".into(), Value::from(commit));
    out.insert("directory".into(), Value::from(dir));
    out.insert(
        "digest".into(),
        Value::from(format!("blake3:{}", cloned.digest.to_hex())),
    );
    out.insert("size".into(), Value::from(cloned.archive.len()));
    out.insert("files".into(), Value::Array(files(&cloned.archive)?));
    out.insert(
        "dependencies".into(),
        Value::Object(dependencies(&cloned.manifest)),
    );
    out.insert(
        "capabilities".into(),
        capabilities(cloned.manifest.package.as_ref()),
    );
    out.insert("api".into(), report);
    out.insert(
        "doc_coverage".into(),
        serde_json::json!({
            "documented": docs.documented,
            "public": docs.public,
        }),
    );
    out.insert("docs".into(), Value::from(docs.text));
    let readme = readme(&cloned.archive)?;
    out.insert("summary".into(), Value::from(summary(&readme)));
    out.insert("readme".into(), Value::from(readme));
    out.insert(
        "downloads".into(),
        Value::Array(
            cloned
                .manifest
                .downloads
                .iter()
                .map(|download| {
                    serde_json::json!({
                        "path": download.path,
                        "urls": download.urls,
                        "sha256": download.sha256,
                        "size": download.size,
                        "license": download.license,
                    })
                })
                .collect(),
        ),
    );
    out.insert(
        "supersedes".into(),
        cloned
            .manifest
            .supersedes
            .as_ref()
            .map_or(Value::Null, |name| Value::from(name.to_string())),
    );
    Ok(Value::Object(out))
}

/// The archive's readme, the root file named `README` with any extension, as UTF-8 with each
/// invalid sequence replaced, and cut to [`MAX_README`] bytes at a character's end. Empty when
/// there is no readme. The package page shows it as text a publisher wrote.
///
/// # Errors
///
/// An archive that does not read, which the pack never writes.
fn readme(archive: &[u8]) -> Result<String, Fault> {
    let bad = |err: io::Error| fault(format!("the archive does not read: {err}"));
    for entry in tar::Archive::new(archive).entries().map_err(bad)? {
        let entry = entry.map_err(bad)?;
        let path = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
        let stem = path.split('.').next().unwrap_or_default();
        if path.contains('/') || !stem.eq_ignore_ascii_case("readme") {
            continue;
        }
        let mut bytes = Vec::new();
        io::Read::read_to_end(&mut io::Read::take(entry, MAX_README as u64), &mut bytes)
            .map_err(bad)?;
        return Ok(cut(
            String::from_utf8_lossy(&bytes).into_owned(),
            MAX_README,
        ));
    }
    Ok(String::new())
}

/// The first paragraph of `readme`: its first run of lines that are not empty and not a `#`
/// heading, joined by spaces, and cut to [`MAX_SUMMARY`] bytes at a character's end.
fn summary(readme: &str) -> String {
    let mut words = String::new();
    for line in readme.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            if words.is_empty() {
                continue;
            }
            break;
        }
        if !words.is_empty() {
            words.push(' ');
        }
        words.push_str(line);
    }
    cut(words, MAX_SUMMARY)
}

/// `text` cut to at most `max` bytes, at a character's end.
fn cut(mut text: String, max: usize) -> String {
    if text.len() > max {
        let mut end = max;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    text
}

/// The version of the package `manifest` names, once it is one a registry can publish.
///
/// # Errors
///
/// No `[package] name` or `version`, a version other than `sent`, and a git dependency.
pub(crate) fn checked(manifest: &Manifest, sent: Option<Version>) -> Result<Version, Fault> {
    if manifest.name.is_none() {
        return Err(fault(
            "`package.toml` has no `[package] name`, so the folder is an application. Only a \
             package can be published.",
        ));
    }
    let Some(version) = manifest.version else {
        return Err(fault(
            "`package.toml` has no `[package] version`. A published package needs one.",
        ));
    };
    if let Some(sent) = sent.filter(|sent| *sent != version) {
        return Err(fault(format!(
            "The publish is for version {sent}, and `package.toml` at this commit has version \
             {version}."
        )));
    }
    if let Some(dependency) = manifest
        .dependencies
        .iter()
        .find(|dependency| matches!(dependency.source, Source::Git { .. }))
    {
        return Err(fault(format!(
            "`package.toml` names `{}` as a git dependency. A published package may depend only \
             on published packages.",
            dependency.name
        )));
    }
    Ok(version)
}

/// Whether every declaration `document` lists is inside `namespace`, compared without case.
///
/// # Errors
///
/// The first declaration outside it.
fn inside(document: &Value, namespace: &str) -> Result<(), Fault> {
    let prefix = format!("{}\\", namespace.to_lowercase());
    let declarations = document
        .get("declarations")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    for declaration in declarations {
        let name = declaration
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !name.to_lowercase().starts_with(&prefix) {
            return Err(fault(format!(
                "`{name}` is outside the package's namespace `{namespace}`. Every class, \
                 interface, enum and function of a package must be in its namespace."
            )));
        }
    }
    Ok(())
}

/// Each registry dependency of `manifest` and its minimum, in name order. [`checked`] has refused
/// a git dependency before this runs.
fn dependencies(manifest: &Manifest) -> Map<String, Value> {
    manifest
        .dependencies
        .iter()
        .filter_map(|dependency| match &dependency.source {
            Source::Registry { minimum } => Some((
                dependency.name.to_string(),
                Value::from(minimum.to_string()),
            )),
            Source::Git { .. } => None,
        })
        .collect()
}

/// The capabilities `package` requests, as `{"required": [...], "optional": [...]}`.
fn capabilities(package: Option<&nvs_config::package::Package>) -> Value {
    let names = |caps: &[nvs_config::Cap]| caps.iter().map(|cap| cap.name()).collect::<Vec<_>>();
    serde_json::json!({
        "required": package.map(|package| names(package.required())).unwrap_or_default(),
        "optional": package.map(|package| names(package.optional())).unwrap_or_default(),
    })
}

/// Each file of `archive`, as its path and its size, in the archive's order.
///
/// # Errors
///
/// An archive that does not read, which the pack never writes.
fn files(archive: &[u8]) -> Result<Vec<Value>, Fault> {
    let bad = |err: io::Error| fault(format!("the archive does not read: {err}"));
    let mut list = Vec::new();
    for entry in tar::Archive::new(archive).entries().map_err(bad)? {
        let entry = entry.map_err(bad)?;
        let path = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
        let mut file = Map::new();
        file.insert("path".into(), Value::from(path));
        file.insert("size".into(), Value::from(entry.size()));
        list.push(Value::Object(file));
    }
    Ok(list)
}

/// `bytes` written to `path` through a temporary file beside it, so a failed write leaves
/// nothing under the name.
fn write(path: &Path, bytes: &[u8]) -> Result<(), Fault> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".partial");
    let temporary = PathBuf::from(temporary);
    let written = fs::write(&temporary, bytes).and_then(|()| fs::rename(&temporary, path));
    if written.is_err() {
        // The write's error is the one to report; a temporary that will not go is a stray file.
        let _ = fs::remove_file(&temporary);
    }
    written.map_err(|err| fault(format!("cannot write `{}`: {err}", path.display())))
}

/// An archive's files in a scratch folder of their own, removed when it is dropped.
struct Unpacked(PathBuf);

impl Unpacked {
    /// `archive` unpacked into a new folder under `scratch`.
    fn new(scratch: &Path, archive: &[u8]) -> Result<Self, Fault> {
        let dir = scratch.join(format!(
            "nvs-build-{}-{:032x}",
            std::process::id(),
            rand::rng().random::<u128>()
        ));
        let unpacked = Self(dir);
        fetch::unpack(archive, &unpacked.0)?;
        Ok(unpacked)
    }
}

impl Drop for Unpacked {
    fn drop(&mut self) {
        // A folder that cannot be removed is left for the user's temporary-file cleanup.
        let _ = fs::remove_dir_all(&self.0);
    }
}
