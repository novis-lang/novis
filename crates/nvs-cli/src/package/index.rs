//! A registry as the client reads it: the root file, the index files and the archives
//! (`rule:packaging/a-package-is-its-digest`, `rule:packaging/registry-hosting-is-configuration`).
//!
//! **The registry's URL serves one file, `registry.json`** ([`ROOT_FILE`]), which lists the file
//! hosts in order. Every other file is under each file host: `index/<vendor>/<name>.json` for a
//! package, `archive/<digest>.tar` for a version, and the registry's transparency log under
//! `log/`. Each version in an index file names, as `log`, the number of the log record that
//! publishes it, so the client proves that record is in the log ([`super::proof`]). A root file with no file host has the files
//! under the registry's URL itself, as a `file:` registry has them. The schema of both files is
//! the digest rule's fragment, and [`SCHEMA`] is the version this client reads. The files can move
//! to a CDN, or gain a second host, by a change to the root file and no `nvs` release.
//!
//! **A file host that does not answer is skipped**, and so is one whose archive's BLAKE3 digest is
//! not the one the index names. A host that answers that a package does not exist has answered, so
//! a missing package is never looked up on the next host. A `file:` host is a directory, and one
//! that does not exist has not answered.
//!
//! **The last root file read is kept** under the data folder's `registry/` ([`Registry::open`]), one
//! file per registry URL, so the client still finds the file hosts when the registry's URL does not
//! answer.
//!
//! **The root file and every index file are signed notes** whose text is the file's JSON on one
//! line, as `nvs registry index sign` writes them ([`sign`]). The JSON carries `signed_at`, a Unix
//! time, and `checkpoint`, the size and base64 root hash of the log the file was signed with. A
//! file is used once it opens with the registry's pinned verifier key, is signed at most
//! [`AHEAD`] past this computer's clock, and names a checkpoint the log the client checked
//! continues ([`super::proof::Seen::covers`]). An index file that fails any of these is skipped and
//! the next file host is tried, except a validly signed checkpoint the log does not continue,
//! which is the fork error. A command that selects a version passes an oldest time, and an index
//! file signed before it is skipped the same way ([`MAX_AGE`]). The root file has no next host:
//! one that fails is an error, and it is never refused for its age, so the kept copy still finds
//! the file hosts through an outage.
//!
//! An `https:` URL is read through [`nvs_stdlib::tool_stream`], one TLS stack with
//! `Core\Http\Client`, and the read stops one byte past the file's limit, so an archive is read up
//! to [`pack::MAX_ARCHIVE_BYTES`] and no further.
//! A relative `file:` URL resolves against the directory of the `package.toml` that names it.
//!
//! Cost: the client holds each index file it read while it resolves, and one archive at a time.
//! Nothing here runs on the request path.

use std::fmt;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Deserialize;

use super::log::Hash;
use super::manifest::{Name, Version};
use super::note::{self, Checkpoint, Signer, Verifier};
use super::pack;
use super::proof::{Seen, Skip};

/// The one file a registry's URL serves.
pub(crate) const ROOT_FILE: &str = "registry.json";

/// The schema version of the root file and the index files this client reads.
pub(crate) const SCHEMA: u64 = 1;

/// The most bytes a root file may have.
const MAX_ROOT_BYTES: u64 = 64 * 1024;

/// The most bytes an index file may have.
const MAX_INDEX_BYTES: u64 = 1024 * 1024;

/// How far past this computer's clock a file may be signed, in seconds: one hour.
pub(crate) const AHEAD: u64 = 60 * 60;

/// How long ago an index file may be signed for a command that selects a version, in seconds:
/// seven days. The registry signs every index file again each day.
pub(crate) const MAX_AGE: u64 = 7 * 24 * 60 * 60;

/// How long one `https:` read may take before the size of the file is counted.
const WITHIN: Duration = Duration::from_secs(30);

/// The slowest rate an `https:` read is given time for, in bytes per second: 256 KiB/s.
const SLOWEST_RATE: u64 = 256 * 1024;

/// The name a refusal or a failed read names the client by.
const MEMBER: &str = "nvs pkg";

/// Why a registry could not be read. The text is the message the user reads.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Fault(pub(crate) String);

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A [`Fault`] with the text `message`.
pub(crate) fn fault(message: impl Into<String>) -> Fault {
    Fault(message.into())
}

/// One place files are read from: a registry's URL or a file host.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Place {
    /// A directory on this computer, from a `file:` URL.
    Dir(PathBuf),
    /// An `https:` URL, without a trailing `/`.
    Https(String),
}

impl Place {
    /// The place `url` names. A relative `file:` path resolves against `base`.
    fn new(url: &str, base: &Path) -> Result<Self, Fault> {
        let url = url.trim_end_matches('/');
        if url.starts_with("https://") {
            return Ok(Self::Https(url.to_owned()));
        }
        let Some(path) = url.strip_prefix("file:") else {
            return Err(fault(format!(
                "`{url}` is not a registry URL. A URL starts with `https://` or `file:`."
            )));
        };
        let path = path.strip_prefix("//").unwrap_or(path);
        // `file:///C:/x` names `C:/x` on Windows, and `/x` everywhere else.
        let path = if cfg!(windows) && path.as_bytes().get(2) == Some(&b':') {
            &path[1..]
        } else {
            path
        };
        Ok(Self::Dir(base.join(path)))
    }

    /// The file at `path` under this place, at most `limit` bytes, or `None` when the place
    /// answered that there is no such file.
    ///
    /// # Errors
    ///
    /// A place that did not answer, a failed read and a file over `limit`.
    fn get(&self, path: &str, limit: u64) -> Result<Option<Vec<u8>>, Fault> {
        match self {
            Self::Dir(dir) => {
                if !dir.is_dir() {
                    return Err(fault(format!(
                        "the directory `{}` does not exist",
                        dir.display()
                    )));
                }
                let file = dir.join(path);
                let opened = match fs::File::open(&file) {
                    Ok(opened) => opened,
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                    Err(err) => {
                        return Err(fault(format!("cannot read `{}`: {err}", file.display())));
                    }
                };
                let mut bytes = Vec::new();
                opened
                    .take(limit + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|err| fault(format!("cannot read `{}`: {err}", file.display())))?;
                over(&bytes, limit, &file.display().to_string())?;
                Ok(Some(bytes))
            }
            Self::Https(url) => {
                let url = format!("{url}/{path}");
                let headers = vec![(
                    "User-Agent".to_owned(),
                    format!("nvs/{}", env!("CARGO_PKG_VERSION")),
                )];
                let bounds = nvs_stdlib::ToolBounds {
                    within: WITHIN + Duration::from_secs(limit / SLOWEST_RATE),
                    bytes: limit + 1,
                };
                let anywhere = |_| None;
                let mut reply =
                    nvs_stdlib::tool_stream(MEMBER, &url, headers, None, bounds, &anywhere)
                        .map_err(Fault)?;
                match reply.status {
                    200 => {
                        let mut bytes = Vec::new();
                        reply
                            .read_to_end(&mut bytes)
                            .map_err(|err| fault(format!("cannot read `{url}`: {err}")))?;
                        over(&bytes, limit, &url)?;
                        Ok(Some(bytes))
                    }
                    404 | 410 => Ok(None),
                    status => Err(fault(format!("`{url}` returned status {status}"))),
                }
            }
        }
    }
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dir(dir) => write!(f, "file:{}", dir.display()),
            Self::Https(url) => f.write_str(url),
        }
    }
}

/// A fault when `bytes` is longer than `limit`.
fn over(bytes: &[u8], limit: u64, what: &str) -> Result<(), Fault> {
    if bytes.len() as u64 > limit {
        return Err(fault(format!("`{what}` is larger than {limit} bytes")));
    }
    Ok(())
}

/// `registry.json`, as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RootFile {
    schema: u64,
    #[serde(default)]
    hosts: Vec<String>,
    signed_at: u64,
    checkpoint: CheckpointLine,
}

/// One package's index file, as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexFile {
    schema: u64,
    name: String,
    #[serde(default)]
    owners: Vec<String>,
    versions: Vec<ReleaseLine>,
    signed_at: u64,
    checkpoint: CheckpointLine,
}

/// The checkpoint a signed file names, as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointLine {
    size: u64,
    root: String,
}

/// When a root file or an index file was signed, and the log it was signed with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Stamp {
    /// The Unix time it was signed at, in seconds.
    pub(crate) signed_at: u64,
    /// The size of the log's checkpoint.
    pub(crate) size: u64,
    /// The root hash of the log's checkpoint.
    pub(crate) root: Hash,
}

impl Stamp {
    /// The stamp `signed_at` and `checkpoint` write, once it is signed at most [`AHEAD`] past
    /// `now`.
    fn read(signed_at: u64, checkpoint: &CheckpointLine, now: u64) -> Result<Self, String> {
        let root = BASE64
            .decode(&checkpoint.root)
            .ok()
            .and_then(|root| Hash::try_from(root).ok())
            .ok_or("its checkpoint's root is not a base64 hash of 32 bytes")?;
        if signed_at > now.saturating_add(AHEAD) {
            return Err(format!(
                "it is signed at {}, more than an hour after this computer's clock",
                time(signed_at)
            ));
        }
        Ok(Self {
            signed_at,
            size: checkpoint.size,
            root,
        })
    }
}

/// The Unix time `seconds` as a UTC date and time, such as `2026-10-28 14:05 UTC`.
pub(crate) fn time(seconds: u64) -> String {
    let seconds = i64::try_from(seconds).unwrap_or(i64::MAX);
    let (year, month, day) = crate::telemetry::civil(seconds.div_euclid(86_400));
    let minutes = seconds.rem_euclid(86_400) / 60;
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        minutes / 60,
        minutes % 60
    )
}

/// `file`, a root file or an index file as JSON or as a note `signer` signed before, signed
/// again by `signer` at `at` with `checkpoint`: its JSON on one line, with `signed_at` and
/// `checkpoint` set, as a signed note.
///
/// # Errors
///
/// The file is neither a JSON object nor a note `signer` signed.
pub(crate) fn sign(
    file: &[u8],
    signer: &Signer,
    checkpoint: &Checkpoint,
    at: u64,
) -> Result<String, String> {
    let json = match serde_json::from_slice::<serde_json::Value>(file) {
        Ok(json) => json,
        Err(_) => {
            let opened = note::open(file, &[signer.verifier()])
                .map_err(|err| format!("it is not JSON, and not a note this key signed: {err}"))?;
            serde_json::from_str(opened.text)
                .map_err(|err| format!("its text is not JSON: {err}"))?
        }
    };
    let serde_json::Value::Object(mut object) = json else {
        return Err("it is not a JSON object".to_owned());
    };
    object.insert("signed_at".to_owned(), at.into());
    object.insert(
        "checkpoint".to_owned(),
        serde_json::json!({
            "size": checkpoint.size(),
            "root": BASE64.encode(checkpoint.root()),
        }),
    );
    let text = format!("{}\n", serde_json::Value::Object(object));
    note::sign(&text, signer).map_err(|err| err.to_string())
}

/// The JSON text of the note `bytes`, once it opens with `verifier`.
fn opened<'a>(bytes: &'a [u8], verifier: &Verifier) -> Result<&'a str, String> {
    note::open(bytes, std::slice::from_ref(verifier))
        .map(|opened| opened.text)
        .map_err(|err| format!("its signature does not check with the registry's key: {err}"))
}

/// One version in an index file, as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseLine {
    version: String,
    digest: String,
    state: StateLine,
    #[serde(default)]
    dependencies: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    capabilities: Capabilities,
    repository: String,
    commit: String,
    log: u64,
}

/// A version's state, as written: a bare string, or an object with the state's fields.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum StateLine {
    Pending {
        #[serde(default)]
        selectable_at: Option<u64>,
    },
    Released,
    Unlisted,
    Retracted {
        reason: String,
        fixed_in: Option<String>,
    },
    Withheld {
        reason: String,
    },
}

/// The capabilities a version requests, as its `package.toml` splits them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Capabilities {
    #[serde(default)]
    pub(crate) required: Vec<String>,
    #[serde(default)]
    pub(crate) optional: Vec<String>,
}

/// Where a version stands (`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`,
/// `rule:packaging/an-owner-may-unlist`, `rule:packaging/a-known-bad-version-is-retracted-not-deleted`,
/// `rule:packaging/the-operator-only-withholds`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum State {
    /// Published, and selectable by `nvs pkg add` from this Unix time on. With no time, it waits
    /// for an owner's passkey: the first stable release, or one behind such a dependency.
    Pending { selectable_at: Option<u64> },
    /// Selectable.
    Released,
    /// Hidden from search, and still selected when a minimum names it.
    Unlisted,
    /// Known bad. Never selected, except where the lock already names it.
    Retracted {
        reason: String,
        fixed_in: Option<Version>,
    },
    /// Its bytes are no longer served, and it is never selected.
    Withheld { reason: String },
}

/// One version of a package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Release {
    pub(crate) version: Version,
    pub(crate) digest: blake3::Hash,
    pub(crate) state: State,
    /// Each dependency and its minimum, in name order.
    pub(crate) dependencies: Vec<(Name, Version)>,
    pub(crate) capabilities: Capabilities,
    /// The repository the registry built the archive from.
    pub(crate) repository: String,
    /// The commit the registry built the archive from.
    pub(crate) commit: String,
    /// The number of the record in the registry's log that publishes this version. A git
    /// dependency has none.
    pub(crate) log: Option<u64>,
}

/// One package's index: its owners and every version, in version order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Index {
    pub(crate) name: Name,
    pub(crate) owners: Vec<String>,
    pub(crate) releases: Vec<Release>,
}

impl Index {
    /// The release `version`, if the index lists it.
    pub(crate) fn release(&self, version: Version) -> Option<&Release> {
        self.releases
            .iter()
            .find(|release| release.version == version)
    }
}

/// A registry whose root file has been read.
#[derive(Debug)]
pub(crate) struct Registry {
    /// The registry's URL, as `package.toml` names it.
    pub(crate) url: String,
    /// The file hosts, in the order they are tried.
    hosts: Vec<Place>,
    /// The name of the files kept for this registry under the data folder's `registry/`.
    pub(crate) kept: String,
    /// Whether the root file is the kept copy, because the registry's URL did not answer.
    pub(crate) from_kept: bool,
    /// The root file's stamp, which the caller checks against the log.
    pub(crate) stamp: Stamp,
    /// The pinned key every file of this registry is signed with.
    verifier: Verifier,
    /// This computer's clock when the registry was opened, as a Unix time.
    now: u64,
}

impl Registry {
    /// The registry at `url`, its root file read from the URL, or from the copy kept in `keep`
    /// when the URL does not answer, and opened with `verifier` at the time `now`. A root file
    /// read from the URL replaces the kept copy. A relative `file:` URL resolves against `base`.
    ///
    /// # Errors
    ///
    /// A URL that does not answer with no kept copy, a root file that is missing, does not open
    /// with `verifier`, is signed in the future, is not valid, or has a newer schema, and a file
    /// host that is not a URL.
    pub(crate) fn open(
        url: &str,
        base: &Path,
        keep: Option<&Path>,
        verifier: &Verifier,
        now: u64,
    ) -> Result<Self, Fault> {
        let place = Place::new(url, base)?;
        let name = kept_name(&place);
        let kept = keep.map(|dir| dir.join(format!("{name}.json")));
        let (bytes, from_kept) = match place.get(ROOT_FILE, MAX_ROOT_BYTES) {
            Ok(Some(bytes)) => (bytes, false),
            Ok(None) => {
                return Err(fault(format!(
                    "`{url}` is not a registry: it has no `{ROOT_FILE}`"
                )));
            }
            Err(unanswered) => match kept.as_deref().map(fs::read) {
                Some(Ok(bytes)) => (bytes, true),
                _ => {
                    return Err(fault(format!(
                        "cannot read the registry `{url}`: {unanswered}"
                    )));
                }
            },
        };
        let refused =
            |why: String| fault(format!("the `{ROOT_FILE}` of `{url}` is not used: {why}"));
        let text = opened(&bytes, verifier).map_err(refused)?;
        let root: RootFile = serde_json::from_str(text)
            .map_err(|err| fault(format!("the `{ROOT_FILE}` of `{url}` is not valid: {err}")))?;
        schema(root.schema, &format!("the `{ROOT_FILE}` of `{url}`"))?;
        let stamp = Stamp::read(root.signed_at, &root.checkpoint, now).map_err(refused)?;
        let hosts = if root.hosts.is_empty() {
            vec![place]
        } else {
            root.hosts
                .iter()
                .map(|host| Place::new(host, base))
                .collect::<Result<_, _>>()?
        };
        if let (false, Some(kept)) = (from_kept, kept) {
            keep_root(&kept, &bytes)?;
        }
        Ok(Self {
            url: url.to_owned(),
            hosts,
            kept: name,
            from_kept,
            stamp,
            verifier: verifier.clone(),
            now,
        })
    }

    /// The file hosts, in the order they are tried.
    pub(crate) fn hosts(&self) -> std::ops::Range<usize> {
        0..self.hosts.len()
    }

    /// The name of file host `host`, as a message names it.
    pub(crate) fn host_name(&self, host: usize) -> String {
        self.hosts[host].to_string()
    }

    /// The file at `path` under file host `host`, at most `limit` bytes, or `None` when the host
    /// answered that there is no such file.
    ///
    /// # Errors
    ///
    /// The host did not answer, the read failed, or the file is over `limit`.
    pub(crate) fn get(
        &self,
        host: usize,
        path: &str,
        limit: u64,
    ) -> Result<Option<Vec<u8>>, Fault> {
        self.hosts[host].get(path, limit)
    }

    /// The index of `name`, from the first file host whose index file opens with the pinned key,
    /// names a checkpoint `seen` continues, and is signed no earlier than `oldest`, or `None` when
    /// a host answers that it has no such package.
    ///
    /// # Errors
    ///
    /// No file host gave a file that checks, a signed index file that is not valid, and the fork
    /// error.
    pub(crate) fn index(
        &self,
        name: &Name,
        seen: &mut Seen,
        oldest: Option<u64>,
    ) -> Result<Option<Index>, Fault> {
        let path = format!("index/{}.json", name.slug());
        let mut failures = Vec::new();
        for host in &self.hosts {
            let bytes = match host.get(&path, MAX_INDEX_BYTES) {
                Ok(Some(bytes)) => bytes,
                Ok(None) => return Ok(None),
                Err(failure) => {
                    failures.push(format!("{host}: {failure}"));
                    continue;
                }
            };
            match self.checked(&bytes, name, host, seen, oldest) {
                Ok(index) => return Ok(Some(index)),
                Err(Skip::Host(why)) => failures.push(format!("{host}: {why}")),
                Err(Skip::Stop(fork)) => return Err(fork),
            }
        }
        Err(self.unanswered(&format!("the index of `{name}`"), &failures))
    }

    /// The index file `bytes` from `host`, once its signature, time and checkpoint check.
    fn checked(
        &self,
        bytes: &[u8],
        name: &Name,
        host: &Place,
        seen: &mut Seen,
        oldest: Option<u64>,
    ) -> Result<Index, Skip> {
        let text = opened(bytes, &self.verifier).map_err(Skip::Host)?;
        let what = format!("the index of `{name}` on {host}");
        let file: IndexFile = serde_json::from_str(text)
            .map_err(|err| Skip::Stop(fault(format!("{what} is not valid: {err}"))))?;
        let stamp = Stamp::read(file.signed_at, &file.checkpoint, self.now).map_err(Skip::Host)?;
        seen.covers(self, &stamp)?;
        if oldest.is_some_and(|oldest| stamp.signed_at < oldest) {
            return Err(Skip::Host(format!(
                "it is signed at {}, more than seven days ago, so no version is selected from it",
                time(stamp.signed_at)
            )));
        }
        parse_index(file, name, host).map_err(Skip::Stop)
    }

    /// The archive of `name` at `version`, whose BLAKE3 digest is `digest`, from the first file
    /// host that has it with that digest.
    ///
    /// # Errors
    ///
    /// No file host has the archive with that digest. The message names the package.
    pub(crate) fn archive(
        &self,
        name: &Name,
        version: Version,
        digest: &blake3::Hash,
    ) -> Result<Vec<u8>, Fault> {
        let path = format!("archive/{}.tar", digest.to_hex());
        let mut failures = Vec::new();
        for host in &self.hosts {
            match host.get(&path, pack::MAX_ARCHIVE_BYTES) {
                Ok(Some(bytes)) => {
                    let got = blake3::hash(&bytes);
                    if got == *digest {
                        return Ok(bytes);
                    }
                    failures.push(format!(
                        "{host}: the archive's digest is {}, not {}",
                        got.to_hex(),
                        digest.to_hex()
                    ));
                }
                Ok(None) => failures.push(format!("{host}: there is no such archive")),
                Err(failure) => failures.push(format!("{host}: {failure}")),
            }
        }
        Err(self.unanswered(&format!("the archive of `{name}` {version}"), &failures))
    }

    /// The fault when no file host gave `what`, with each host's failure on its own line.
    fn unanswered(&self, what: &str, failures: &[String]) -> Fault {
        let mut message = format!(
            "cannot read {what} from any file host of the registry `{}`:",
            self.url
        );
        for failure in failures {
            message.push_str("\n  ");
            message.push_str(failure);
        }
        fault(message)
    }
}

/// The kept root file's name for `place`: a hash, because a URL is not a file name. A directory
/// is named by its canonical path, so two applications that reach the same registry through
/// different relative `file:` URLs keep one root file and one checkpoint for it.
fn kept_name(place: &Place) -> String {
    let named = match place {
        Place::Dir(dir) => nvs_config::trust::canonical(dir)
            .map_or_else(|_| place.to_string(), |dir| Place::Dir(dir).to_string()),
        Place::Https(_) => place.to_string(),
    };
    let hex = blake3::hash(named.as_bytes()).to_hex();
    hex[..32].to_owned()
}

/// `bytes` written to `kept` through a temporary file and a rename, so a reader never sees half.
fn keep_root(kept: &Path, bytes: &[u8]) -> Result<(), Fault> {
    let failed = |err: std::io::Error| {
        fault(format!(
            "cannot keep the root file at `{}`: {err}",
            kept.display()
        ))
    };
    if let Some(dir) = kept.parent() {
        fs::create_dir_all(dir).map_err(failed)?;
    }
    let partial = kept.with_extension("partial");
    fs::write(&partial, bytes).map_err(failed)?;
    fs::rename(&partial, kept).map_err(failed)
}

/// A fault when `found` is a schema this client does not read.
fn schema(found: u64, what: &str) -> Result<(), Fault> {
    if found != SCHEMA {
        return Err(fault(format!(
            "{what} has schema {found}, and this version of `nvs` reads schema {SCHEMA}. \
             Install the newest `nvs`."
        )));
    }
    Ok(())
}

/// The index file `file` read from `host`, checked to be the index of `name`.
fn parse_index(file: IndexFile, name: &Name, host: &Place) -> Result<Index, Fault> {
    let what = format!("the index of `{name}` on {host}");
    let bad = |detail: String| fault(format!("{what} is not valid: {detail}"));
    schema(file.schema, &what)?;
    let named = Name::checked(&file.name).map_err(|(message, _)| bad(message))?;
    if named.slug() != name.slug() {
        return Err(bad(format!("it names the package `{named}`")));
    }
    let version = |text: &str| text.parse::<Version>().map_err(bad);
    let mut releases = Vec::with_capacity(file.versions.len());
    for line in file.versions {
        let digest = digest(&line.digest).ok_or_else(|| {
            bad(format!(
                "the digest of {} is not 64 lowercase hex digits",
                line.version
            ))
        })?;
        let state = match line.state {
            StateLine::Pending { selectable_at } => State::Pending { selectable_at },
            StateLine::Released => State::Released,
            StateLine::Unlisted => State::Unlisted,
            StateLine::Retracted { reason, fixed_in } => State::Retracted {
                reason,
                fixed_in: fixed_in.as_deref().map(version).transpose()?,
            },
            StateLine::Withheld { reason } => State::Withheld { reason },
        };
        let mut dependencies = Vec::with_capacity(line.dependencies.len());
        for (dependency, minimum) in &line.dependencies {
            let dependency = Name::checked(dependency).map_err(|(message, _)| bad(message))?;
            dependencies.push((dependency, version(minimum)?));
        }
        releases.push(Release {
            version: version(&line.version)?,
            digest,
            state,
            dependencies,
            capabilities: line.capabilities,
            repository: line.repository,
            commit: line.commit,
            log: Some(line.log),
        });
    }
    releases.sort_by_key(|release| release.version);
    if let Some(pair) = releases
        .windows(2)
        .find(|pair| pair[0].version == pair[1].version)
    {
        return Err(bad(format!("it lists {} twice", pair[0].version)));
    }
    Ok(Index {
        name: named,
        owners: file.owners,
        releases,
    })
}

/// The BLAKE3 digest written as `text`: exactly 64 lowercase hex digits.
pub(crate) fn digest(text: &str) -> Option<blake3::Hash> {
    if text.len() != 64 || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    blake3::Hash::from_hex(text).ok()
}

#[cfg(test)]
mod tests {
    use super::super::fixture::Fixture;
    use super::*;

    fn http() -> Name {
        Name::checked("Acme\\Http").expect("a valid name")
    }

    /// This computer's clock, as a Unix time.
    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock is past 1970")
            .as_secs()
    }

    /// The fixture's registry, opened with its key, the kept root file in `keep`.
    fn open(fixture: &Fixture, keep: Option<&Path>) -> Result<Registry, Fault> {
        Registry::open(
            &fixture.url(),
            Path::new("."),
            keep,
            &fixture.verifier(),
            now(),
        )
    }

    /// The log of `registry`, opened with the fixture's key and no kept checkpoint.
    fn seen(fixture: &Fixture, registry: &Registry) -> Seen {
        Seen::open(registry, &fixture.verifier(), None).expect("the log opens")
    }

    /// The index of `Acme\Http` in the fixture's registry, read with no oldest time.
    fn read(fixture: &Fixture) -> Result<Option<Index>, Fault> {
        let registry = open(fixture, None).expect("it opens");
        let mut seen = seen(fixture, &registry);
        registry.index(&http(), &mut seen, None)
    }

    /// The fixture's index and archive read back, and the archive's digest is the one packed.
    #[test]
    fn a_fixture_registry_reads_back() {
        let fixture = Fixture::new("index-reads-back");
        let digest = fixture.publish("Acme\\Http", "1.4.0", &[], "\"released\"");
        let registry = open(&fixture, None).expect("it opens");
        let mut seen = seen(&fixture, &registry);
        let index = registry
            .index(&http(), &mut seen, None)
            .expect("it answers")
            .expect("it exists");
        let release = index
            .release("1.4.0".parse().expect("a version"))
            .expect("listed");
        assert_eq!(release.digest, digest);
        assert_eq!(release.state, State::Released);
        let bytes = registry
            .archive(&http(), release.version, &digest)
            .expect("the archive reads");
        assert_eq!(blake3::hash(&bytes), digest);
        let missing = Name::checked("Acme\\Gone").expect("a valid name");
        assert_eq!(registry.index(&missing, &mut seen, None), Ok(None));
    }

    /// The index file the registry service writes, `apps/registry/fixtures/index.json`, which
    /// the service's `IndexTest.nvs` pins, reads back once `nvs registry index sign`'s signing
    /// adds the time and the checkpoint.
    #[test]
    fn the_registry_services_index_file_reads_back() {
        let fixture = Fixture::new("index-service");
        let written = nvs_repo::path("apps/registry/fixtures/index.json");
        let text = fs::read_to_string(&written).expect("the service's fixture reads");
        let json: serde_json::Value = serde_json::from_str(&text).expect("the fixture is JSON");
        fixture.append(&super::super::entry::Entry::Publish {
            package: "acme/http".to_owned(),
            version: "1.0.0".to_owned(),
            digest: "a".repeat(64),
            repository: "https://git.example.com/acme/http.git".to_owned(),
            commit: "1".repeat(40),
        });
        let path = fixture.files().join("index/acme/http.json");
        fs::create_dir_all(path.parent().expect("an index file has a folder"))
            .expect("the test makes the index folder");
        fixture.write_signed(&path, &json);

        let index = read(&fixture).expect("it answers").expect("it exists");
        assert_eq!(index.owners, ["0123456789abcdef0123456789abcdef"]);
        let first = index
            .release("1.0.0".parse().expect("a version"))
            .expect("listed");
        assert_eq!(first.state, State::Released);
        assert_eq!(first.log, Some(0));
        assert!(first.dependencies.is_empty());
        assert_eq!(first.capabilities, Capabilities::default());
        let second = index
            .release("1.1.0".parse().expect("a version"))
            .expect("listed");
        assert_eq!(
            second.state,
            State::Pending {
                selectable_at: Some(1_800_000_000)
            }
        );
        assert_eq!(second.digest.to_hex().as_str(), "b".repeat(64));
        assert_eq!(second.log, Some(3));
        assert_eq!(second.repository, "https://git.example.com/acme/http.git");
        assert_eq!(second.commit, "2".repeat(40));
        let log = Name::checked("Acme\\Log").expect("a valid name");
        assert_eq!(
            second.dependencies,
            [(log, "1.2.0".parse().expect("a version"))]
        );
        assert_eq!(second.capabilities.required, ["net.connect"]);
        assert_eq!(second.capabilities.optional, ["fs.read"]);
    }

    /// A first file host that does not exist is skipped, and the second one answers.
    #[test]
    fn a_missing_first_file_host_reads_from_the_second() {
        let fixture = Fixture::new("index-second-host");
        fixture.publish("Acme\\Http", "1.0.0", &[], "\"released\"");
        let missing = fixture.root().join("no-such-host");
        fixture.hosts(&[url_of(&missing), url_of(&fixture.files())]);
        let index = read(&fixture).expect("the second host answers");
        assert!(index.is_some());
    }

    /// The first file host's index file with a bad signature, signed by another key, or with no
    /// signature is skipped, and the second host's signed file is read.
    #[test]
    fn a_badly_signed_index_file_reads_the_next_host() {
        let fixture = Fixture::new("index-bad-signature");
        fixture.publish("Acme\\Http", "1.0.0", &[], "\"released\"");
        let good = fs::read(fixture.files().join("index/acme/http.json")).expect("it is signed");
        let good = String::from_utf8(good).expect("a note is text");
        let (other, _) = Signer::generate("registry.example.com").expect("a key is made");
        let checkpoint = Checkpoint::empty("registry.example.com").expect("a valid origin");
        let text = good.split("\n\n").next().expect("a note has text");
        let unsigned = text.to_owned();
        let changed = good.replace("\"released\"", "{\"withheld\":{\"reason\":\"x\"}}");
        let foreign = sign(text.as_bytes(), &other, &checkpoint, now()).expect("it signs");
        let bad = fixture.root().join("bad-host");
        fs::create_dir_all(bad.join("index/acme")).expect("the test makes the host");
        fixture.hosts(&[url_of(&bad), url_of(&fixture.files())]);
        for (file, why) in [
            (unsigned, "no blank line"),
            (changed, "does not check"),
            (foreign, "does not check"),
        ] {
            fs::write(bad.join("index/acme/http.json"), &file).expect("the test writes it");
            let index = read(&fixture).expect("the second host answers");
            let release = &index.expect("it exists").releases[0];
            assert_eq!(release.state, State::Released, "{why}");
        }
    }

    /// An index file signed two hours past this computer's clock is not used.
    #[test]
    fn an_index_file_signed_in_the_future_is_refused() {
        let fixture = Fixture::new("index-future");
        fixture.signing_at(now() + 2 * 60 * 60);
        fixture.publish("Acme\\Http", "1.0.0", &[], "\"released\"");
        let failed = read(&fixture).expect_err("it is refused");
        assert!(
            failed
                .0
                .contains("more than an hour after this computer's clock"),
            "{failed}"
        );
    }

    /// An index file signed eight days ago is not used to select a version, and is read when
    /// nothing is selected from it.
    #[test]
    fn an_old_index_file_is_refused_only_for_selecting() {
        let fixture = Fixture::new("index-old");
        fixture.signing_at(now() - 8 * 24 * 60 * 60);
        fixture.publish("Acme\\Http", "1.0.0", &[], "\"released\"");
        let registry = open(&fixture, None).expect("it opens");
        let mut seen = seen(&fixture, &registry);
        let oldest = Some(now() - MAX_AGE);
        let failed = registry
            .index(&http(), &mut seen, oldest)
            .expect_err("it is too old to select from");
        assert!(failed.0.contains("more than seven days ago"), "{failed}");
        assert!(failed.0.contains(&fixture.url()), "{failed}");
        assert!(registry.index(&http(), &mut seen, None).is_ok());
    }

    /// An index file signed with a checkpoint the log does not continue is the fork error, and
    /// one signed with a checkpoint past the log's end reads the next host.
    #[test]
    fn an_index_file_names_a_checkpoint_the_log_continues() {
        let fixture = Fixture::new("index-checkpoint");
        fixture.publish("Acme\\Http", "1.0.0", &[], "\"released\"");
        fixture.publish("Acme\\Http", "1.1.0", &[], "\"released\"");
        let path = fixture.files().join("index/acme/http.json");
        let text = fs::read_to_string(&path).expect("it is signed");
        let json = text.split("\n\n").next().expect("a note has text");
        let signer = fixture.signer();
        let forked = Checkpoint::new("registry.example.com", 1, [7; 32]).expect("valid");
        let note = sign(json.as_bytes(), signer, &forked, now()).expect("it signs");
        fs::write(&path, note).expect("the test writes it");
        let failed = read(&fixture).expect_err("forked");
        assert!(failed.0.contains("was forked"), "{failed}");

        let ahead = Checkpoint::new("registry.example.com", 9, [7; 32]).expect("valid");
        let note = sign(json.as_bytes(), signer, &ahead, now()).expect("it signs");
        fs::write(&path, note).expect("the test writes it");
        let failed = read(&fixture).expect_err("past the log's end");
        assert!(failed.0.contains("a log of 9 records"), "{failed}");
    }

    /// An archive whose bytes do not give the index's digest is read from the next host, and with
    /// no next host the fault names the package.
    #[test]
    fn a_digest_mismatch_reads_the_next_host() {
        let fixture = Fixture::new("index-mismatch");
        let digest = fixture.publish("Acme\\Http", "1.0.0", &[], "\"released\"");
        let bad = fixture.root().join("bad-host");
        let archive = format!("archive/{}.tar", digest.to_hex());
        fs::create_dir_all(bad.join("archive")).expect("the test makes the directory");
        fs::write(bad.join(&archive), b"not the archive").expect("the test writes the archive");
        fixture.hosts(&[url_of(&bad)]);
        let registry = open(&fixture, None).expect("it opens");
        let version = "1.0.0".parse().expect("a version");
        let failed = registry
            .archive(&http(), version, &digest)
            .expect_err("no host has it");
        assert!(
            failed.0.contains("the archive of `Acme\\Http` 1.0.0"),
            "{failed}"
        );
        fixture.hosts(&[url_of(&bad), url_of(&fixture.files())]);
        let registry = open(&fixture, None).expect("it opens");
        let bytes = registry
            .archive(&http(), version, &digest)
            .expect("the second host has it");
        assert_eq!(blake3::hash(&bytes), digest);
    }

    /// A registry URL that does not answer is read through the root file kept from the last
    /// read, and with no kept file it is a fault.
    #[test]
    fn an_unanswered_registry_reads_the_kept_root_file() {
        let fixture = Fixture::new("index-kept");
        fixture.publish("Acme\\Http", "1.0.0", &[], "\"released\"");
        fixture.hosts(&[url_of(&fixture.files())]);
        let keep = fixture.root().join("data-registry");
        let fresh = open(&fixture, Some(&keep)).expect("it opens");
        assert!(!fresh.from_kept);
        fs::remove_file(fixture.registry().join(ROOT_FILE)).expect("the test removes the file");
        fs::remove_dir(fixture.registry()).expect("the test removes the registry's directory");
        let kept = open(&fixture, Some(&keep)).expect("kept");
        assert!(kept.from_kept);
        let mut seen = seen(&fixture, &kept);
        assert!(
            kept.index(&http(), &mut seen, None)
                .expect("the file host answers")
                .is_some()
        );
        let none = open(&fixture, None).expect_err("no copy");
        assert!(none.0.starts_with("cannot read the registry"), "{none}");
    }

    /// A root file with a newer schema is refused with a message that says what to do, and an
    /// unsigned one is refused.
    #[test]
    fn a_newer_schema_is_refused() {
        let fixture = Fixture::new("index-schema");
        let root = fixture.registry().join(ROOT_FILE);
        fixture.write_signed(&root, &serde_json::json!({ "schema": 2 }));
        let failed = open(&fixture, None).expect_err("refused");
        assert!(failed.0.contains("schema 2"), "{failed}");
        fs::write(&root, "{\"schema\": 1}").expect("the test writes the root file");
        let failed = open(&fixture, None).expect_err("refused");
        assert!(failed.0.contains("signature"), "{failed}");
    }

    /// A relative `file:` URL resolves against the base directory.
    #[test]
    fn a_relative_file_url_resolves_against_its_base() {
        let base = Path::new("app");
        assert_eq!(
            Place::new("file:registry/", base),
            Ok(Place::Dir(base.join("registry")))
        );
        assert_eq!(
            Place::new("https://example.com/", base),
            Ok(Place::Https("https://example.com".to_owned()))
        );
    }

    /// The `file:` URL of `path`.
    fn url_of(path: &Path) -> String {
        format!("file:{}", path.display())
    }
}
