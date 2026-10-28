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

use serde::Deserialize;

use super::manifest::{Name, Version};
use super::pack;

/// The one file a registry's URL serves.
pub(crate) const ROOT_FILE: &str = "registry.json";

/// The schema version of the root file and the index files this client reads.
pub(crate) const SCHEMA: u64 = 1;

/// The most bytes a root file may have.
const MAX_ROOT_BYTES: u64 = 64 * 1024;

/// The most bytes an index file may have.
const MAX_INDEX_BYTES: u64 = 1024 * 1024;

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
        selectable_at: u64,
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
    /// Published, and selectable by `nvs pkg add` from this Unix time on.
    Pending { selectable_at: u64 },
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
}

impl Registry {
    /// The registry at `url`, its root file read from the URL, or from the copy kept in `keep`
    /// when the URL does not answer. A root file read from the URL replaces the kept copy.
    /// A relative `file:` URL resolves against `base`.
    ///
    /// # Errors
    ///
    /// A URL that does not answer with no kept copy, a root file that is missing, is not valid,
    /// or has a newer schema, and a file host that is not a URL.
    pub(crate) fn open(url: &str, base: &Path, keep: Option<&Path>) -> Result<Self, Fault> {
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
        let root: RootFile = serde_json::from_slice(&bytes)
            .map_err(|err| fault(format!("the `{ROOT_FILE}` of `{url}` is not valid: {err}")))?;
        schema(root.schema, &format!("the `{ROOT_FILE}` of `{url}`"))?;
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

    /// The index of `name`, from the first file host that answers, or `None` when that host has
    /// no such package.
    ///
    /// # Errors
    ///
    /// No file host answered, or the index file is not valid.
    pub(crate) fn index(&self, name: &Name) -> Result<Option<Index>, Fault> {
        let path = format!("index/{}.json", name.slug());
        let mut failures = Vec::new();
        for host in &self.hosts {
            match host.get(&path, MAX_INDEX_BYTES) {
                Ok(Some(bytes)) => return parse_index(&bytes, name, host).map(Some),
                Ok(None) => return Ok(None),
                Err(failure) => failures.push(format!("{host}: {failure}")),
            }
        }
        Err(unanswered(&format!("the index of `{name}`"), &failures))
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
        Err(unanswered(
            &format!("the archive of `{name}` {version}"),
            &failures,
        ))
    }
}

/// The fault when no file host gave `what`, with each host's failure on its own line.
fn unanswered(what: &str, failures: &[String]) -> Fault {
    let mut message = format!("cannot read {what} from any file host of the registry:");
    for failure in failures {
        message.push_str("\n  ");
        message.push_str(failure);
    }
    fault(message)
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

/// The index file `bytes` read from `host`, checked to be the index of `name`.
fn parse_index(bytes: &[u8], name: &Name, host: &Place) -> Result<Index, Fault> {
    let what = format!("the index of `{name}` on {host}");
    let bad = |detail: String| fault(format!("{what} is not valid: {detail}"));
    let file: IndexFile = serde_json::from_slice(bytes).map_err(|err| bad(err.to_string()))?;
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

    /// The fixture's index and archive read back, and the archive's digest is the one packed.
    #[test]
    fn a_fixture_registry_reads_back() {
        let fixture = Fixture::new("index-reads-back");
        let digest = fixture.publish("Acme\\Http", "1.4.0", &[], "\"released\"");
        let registry = Registry::open(&fixture.url(), Path::new("."), None).expect("it opens");
        let index = registry
            .index(&http())
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
        assert_eq!(registry.index(&missing), Ok(None));
    }

    /// A first file host that does not exist is skipped, and the second one answers.
    #[test]
    fn a_missing_first_file_host_reads_from_the_second() {
        let fixture = Fixture::new("index-second-host");
        fixture.publish("Acme\\Http", "1.0.0", &[], "\"released\"");
        let missing = fixture.root().join("no-such-host");
        fixture.hosts(&[url_of(&missing), url_of(&fixture.files())]);
        let registry = Registry::open(&fixture.url(), Path::new("."), None).expect("it opens");
        let index = registry.index(&http()).expect("the second host answers");
        assert!(index.is_some());
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
        let registry = Registry::open(&fixture.url(), Path::new("."), None).expect("it opens");
        let version = "1.0.0".parse().expect("a version");
        let failed = registry
            .archive(&http(), version, &digest)
            .expect_err("no host has it");
        assert!(
            failed.0.contains("the archive of `Acme\\Http` 1.0.0"),
            "{failed}"
        );
        fixture.hosts(&[url_of(&bad), url_of(&fixture.files())]);
        let registry = Registry::open(&fixture.url(), Path::new("."), None).expect("it opens");
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
        let fresh = Registry::open(&fixture.url(), Path::new("."), Some(&keep)).expect("it opens");
        assert!(!fresh.from_kept);
        fs::remove_file(fixture.registry().join(ROOT_FILE)).expect("the test removes the file");
        fs::remove_dir(fixture.registry()).expect("the test removes the registry's directory");
        let kept = Registry::open(&fixture.url(), Path::new("."), Some(&keep)).expect("kept");
        assert!(kept.from_kept);
        assert!(
            kept.index(&http())
                .expect("the file host answers")
                .is_some()
        );
        let none = Registry::open(&fixture.url(), Path::new("."), None).expect_err("no copy");
        assert!(none.0.starts_with("cannot read the registry"), "{none}");
    }

    /// A root file with a newer schema is refused with a message that says what to do.
    #[test]
    fn a_newer_schema_is_refused() {
        let fixture = Fixture::new("index-schema");
        fs::write(fixture.registry().join(ROOT_FILE), "{\"schema\": 2}")
            .expect("the test writes the root file");
        let failed = Registry::open(&fixture.url(), Path::new("."), None).expect_err("refused");
        assert!(failed.0.contains("schema 2"), "{failed}");
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
