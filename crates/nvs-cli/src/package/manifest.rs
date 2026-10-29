//! `package.toml`, as the `nvs pkg` commands read it (`rule:packaging/package-toml-is-a-tool-input`).
//!
//! ```toml
//! [package]
//! name = 'Acme\Http'
//! version = "1.4.0"
//! files = ["templates/error.html"]
//!
//! [dependencies]
//! 'Acme\Log' = "2.1.0"
//! 'Acme\Fork' = { git = "https://git.example.com/acme/fork.git", rev = "<40 hex digits>" }
//!
//! [registries]
//! Acme = { url = "https://packages.example.com", key = "<verifier key>" }
//!
//! [capabilities]
//! required = ["net.connect"]
//!
//! [grants]
//! 'Acme\Log' = []
//!
//! [[download]]
//! path = "fonts/Big.otf"
//! urls = ["https://example.com/Big.otf"]
//! sha256 = "<64 hex digits>"
//! size = 16384000
//! ```
//!
//! **Every key is known, and an unknown one is refused**
//! (`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`). The file is found by
//! walking up from the working directory ([`find`]), never by the runtime. Everything refused is
//! [`code::E_BAD_PACKAGE_MANIFEST`], except a `[grants]` line, which keeps the compiler's code.
//!
//! **The compiler's parsers read their own parts, and this reader does not repeat them.**
//! `[package] name` and `[capabilities]` go through `nvs_config::package`, and `[grants]` through
//! `nvs_config::grants`, so `nvs check` and `nvs pkg` cannot disagree about either. This module adds
//! what only the client checks: a name is no built-in component's, a version is
//! `MAJOR.MINOR.PATCH`, and the dependencies and registries are well formed.
//!
//! **A file with a `[package] name` is a package's, and one without is an application's.** Only a
//! package has a `version`, `files`, `[capabilities]` and `[[download]]`.
//!
//! **A package's name is its namespace** (`rule:packaging/a-package-name-is-vendor-slash-name`),
//! and [`Name::slug`] derives `acme/http` from `Acme\Http`. Two dependencies whose slugs are equal
//! are refused, because a registry holds one name per case-folded namespace. `Core` is refused
//! everywhere, and so is the namespace of each built-in component, such as `Novis\Image`. Any other
//! `Novis\` name is the project's own account's, which the registry checks at publish, so the
//! client reads it.
//!
//! **A dependency is a minimum version or a git commit.** A git dependency names its URL and an
//! exact `rev`, never a branch or a tag (`rule:packaging/a-git-dependency-is-root-only`). Each
//! value has one form, so a dependency is never a table with a `version` key. A dependency's key
//! is the package's namespace, as `[grants]` keys it, so the file names a package one way.
//!
//! **Each vendor resolves from one registry** (`rule:packaging/a-vendor-resolves-from-one-registry`):
//! `[registries]` maps a vendor to its URL, and [`Manifest::registry`] gives every other vendor
//! [`DEFAULT_REGISTRY`]. A URL is `https:` or `file:`. The verifier `key` is read here and checked
//! by the transparency log's reader.
//!
//! The form of a `files` path and of a `[[download]]` entry is checked by the pack, which every
//! archive goes through, and the download checks its entries again before it fetches.
//!
//! Cost: one read of one file per command, and memory for its lists while the command runs.
//! Nothing here runs on the request path.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

use nvs_config::grants::Grants;
use nvs_config::package::Package;
use nvs_diagnostics::{Diagnostic, Renderer, SourceId, SourceMap, Span, code};
use serde::Deserialize;
use toml::Spanned;

/// The file a package or an application names itself in.
const FILE: &str = nvs_config::grants::FILE;

/// The registry every vendor that `[registries]` does not name resolves from.
pub(crate) const DEFAULT_REGISTRY: &str = "https://registry.novis-lang.org";

/// The verifier key of [`DEFAULT_REGISTRY`]'s log. It is `None` until the registry service makes
/// its key, and until then no archive is read from the default registry.
const DEFAULT_KEY: Option<&str> = None;

/// The namespace no package may take, compared without case (`rule:core-api/reserved-namespace`).
const RESERVED: &str = "Core";

/// The classes of the built-in components. Each one's first two segments are a namespace no
/// package may take (`rule:packaging/the-first-party-components-are-built-in`). The list is
/// `nvs_ext::builtin`'s own constants, so a new component is refused with no second list.
const BUILT_IN: [&str; 4] = [
    nvs_ext::builtin::IMAGE_CLASS,
    nvs_ext::builtin::INTL_CLASS,
    nvs_ext::builtin::PDF_CLASS,
    nvs_ext::builtin::SPREADSHEET_CLASS,
];

/// A package version, `MAJOR.MINOR.PATCH`. Each part is a plain decimal number with no leading
/// zero, and nothing may follow the third.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct Version {
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) patch: u64,
}

impl FromStr for Version {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let part = |piece: Option<&str>| {
            piece
                .filter(|p| {
                    !p.is_empty()
                        && p.bytes().all(|b| b.is_ascii_digit())
                        && (p.len() == 1 || !p.starts_with('0'))
                })
                .and_then(|p| p.parse::<u64>().ok())
        };
        let mut pieces = text.split('.');
        match (
            part(pieces.next()),
            part(pieces.next()),
            part(pieces.next()),
            pieces.next(),
        ) {
            (Some(major), Some(minor), Some(patch), None) => Ok(Self {
                major,
                minor,
                patch,
            }),
            _ => Err(format!(
                "`{text}` is not a version. A version is three numbers, such as `1.4.0`."
            )),
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A package's name: its root namespace, `Vendor\Package`, without a leading `\`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) struct Name(String);

impl Name {
    /// The namespace, `Acme\Http`.
    pub(crate) fn namespace(&self) -> &str {
        &self.0
    }

    /// The vendor, the namespace's first segment.
    pub(crate) fn vendor(&self) -> &str {
        self.0.split('\\').next().unwrap_or_default()
    }

    /// The name on the command line and in a URL: lowercase, with `\` replaced by `/`, so
    /// `Acme\Http` is `acme/http`.
    pub(crate) fn slug(&self) -> String {
        self.0.to_lowercase().replace('\\', "/")
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where a dependency comes from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Source {
    /// The vendor's registry, at this version or a later one
    /// (`rule:packaging/resolution-takes-the-highest-minimum`).
    Registry { minimum: Version },
    /// A git repository at one exact commit.
    Git { url: String, rev: String },
}

/// One line of `[dependencies]`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Dependency {
    pub(crate) name: Name,
    pub(crate) source: Source,
}

/// One line of `[registries]`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Registry {
    /// The vendor, as written.
    pub(crate) vendor: String,
    /// The registry's URL, without a trailing `/`.
    pub(crate) url: String,
    /// The registry's verifier key, as written.
    pub(crate) key: Option<String>,
}

/// One `[[download]]` entry, as written
/// (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`).
#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Download {
    pub(crate) path: String,
    pub(crate) urls: Vec<String>,
    pub(crate) sha256: String,
    pub(crate) size: u64,
    #[serde(default)]
    pub(crate) license: Option<String>,
}

/// A `package.toml`, read and checked.
#[derive(Debug)]
pub(crate) struct Manifest {
    /// The file's path.
    pub(crate) path: PathBuf,
    /// The package this file declares, or `None` for an application's file.
    pub(crate) package: Option<Package>,
    /// The package's name. `Some` exactly when [`Self::package`] is.
    pub(crate) name: Option<Name>,
    /// `[package] version`.
    pub(crate) version: Option<Version>,
    /// `[package] files`, as written.
    pub(crate) files: Vec<String>,
    /// `[package] supersedes`: the package this one replaces, such as the old major a maintainer
    /// keeps alive under its first name (`rule:packaging/only-a-major-may-break-the-api`).
    pub(crate) supersedes: Option<Name>,
    /// `[dependencies]`, in name order.
    pub(crate) dependencies: Vec<Dependency>,
    /// `[registries]`, in vendor order.
    pub(crate) registries: Vec<Registry>,
    /// `[[download]]`, in file order.
    pub(crate) downloads: Vec<Download>,
    /// `[grants]`.
    pub(crate) grants: Grants,
}

impl Manifest {
    /// The URL `vendor` resolves from: its `[registries]` line, compared without case, or
    /// [`DEFAULT_REGISTRY`].
    pub(crate) fn registry(&self, vendor: &str) -> &str {
        self.registries
            .iter()
            .find(|line| line.vendor.to_lowercase() == vendor.to_lowercase())
            .map_or(DEFAULT_REGISTRY, |line| line.url.as_str())
    }

    /// The verifier key of the registry `vendor` resolves from: its `[registries]` line's `key`,
    /// or [`DEFAULT_KEY`].
    pub(crate) fn registry_key(&self, vendor: &str) -> Option<&str> {
        match self
            .registries
            .iter()
            .find(|line| line.vendor.to_lowercase() == vendor.to_lowercase())
        {
            Some(line) => line.key.as_deref(),
            None => DEFAULT_KEY,
        }
    }
}

/// The nearest `package.toml` in `start` or a directory above it, the way `git` finds `.git`.
pub(crate) fn find(start: &Path) -> Option<PathBuf> {
    nvs_config::grants::find(start)
}

/// The `package.toml` at `path`.
///
/// # Errors
///
/// A file that cannot be read, or one [`parse`] refuses.
pub(crate) fn read(path: &Path, sources: &mut SourceMap) -> Result<Manifest, Diagnostic> {
    let id = sources.load(path).map_err(|err| {
        Diagnostic::error(
            code::E_BAD_PACKAGE_MANIFEST,
            format!("cannot read `{}`: {err}", path.display()),
        )
    })?;
    let text = sources.file(id).text().to_owned();
    parse_in(sources, id, &text, path)
}

/// The `package.toml` `text`, registered in `sources` under `path`.
///
/// # Errors
///
/// [`code::E_BAD_PACKAGE_MANIFEST`] for every refusal but a `[grants]` line's, which is the code
/// `nvs check` gives it.
pub(crate) fn parse(
    sources: &mut SourceMap,
    path: &Path,
    text: &str,
) -> Result<Manifest, Diagnostic> {
    let id = sources.add(path.display().to_string(), text);
    parse_in(sources, id, text, path)
}

/// The `version` in `<dir>/package.toml`, or `None` when there is no such file or no such key.
/// A file this module refuses has already been reported on standard error.
pub(crate) fn version(dir: &Path) -> Result<Option<Version>, ExitCode> {
    let path = dir.join(FILE);
    if !path.is_file() {
        return Ok(None);
    }
    let mut sources = SourceMap::new();
    match read(&path, &mut sources) {
        Ok(manifest) => Ok(manifest.version),
        Err(diagnostic) => {
            let mut err = std::io::stderr().lock();
            // Standard error that cannot be written to has nobody to tell.
            let _ = Renderer::new().render(&diagnostic, &sources, &mut err);
            Err(ExitCode::FAILURE)
        }
    }
}

/// The whole file, as `serde` reads it. Every table refuses a key it does not name.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    package: Option<Spanned<Header>>,
    #[serde(default)]
    dependencies: BTreeMap<Spanned<String>, Spanned<toml::Value>>,
    #[serde(default)]
    registries: BTreeMap<Spanned<String>, Spanned<RegistryLine>>,
    #[serde(default)]
    capabilities: Option<Spanned<Requests>>,
    // `nvs_config::grants` reads the lines.
    #[serde(default)]
    #[allow(
        dead_code,
        reason = "read so the key is known; `nvs_config::grants` parses it"
    )]
    grants: Option<toml::Table>,
    #[serde(default)]
    download: Option<Spanned<Vec<Download>>>,
}

/// `[package]`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    // `nvs_config::package` checks the name.
    #[serde(default)]
    name: Option<Spanned<String>>,
    #[serde(default)]
    version: Option<Spanned<String>>,
    #[serde(default)]
    files: Option<Spanned<Vec<String>>>,
    #[serde(default)]
    supersedes: Option<Spanned<String>>,
}

/// `[capabilities]`, whose names `nvs_config::package` checks.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "read so the keys are known; `nvs_config::package` parses them"
)]
struct Requests {
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    optional: Vec<String>,
}

/// One `[registries]` value.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryLine {
    url: Spanned<String>,
    #[serde(default)]
    key: Option<String>,
}

/// A `[dependencies]` value written as a table: a git dependency.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GitLine {
    git: String,
    rev: String,
}

/// [`parse`] over `text`, already in `sources` as `id`.
fn parse_in(
    sources: &mut SourceMap,
    id: SourceId,
    text: &str,
    path: &Path,
) -> Result<Manifest, Diagnostic> {
    // `toml` directly, not `nvs_config::file::parse_at`: that one only warns about an empty table
    // with an unknown name, and here every unknown key is refused.
    let file: File = toml::from_str(text).map_err(|err| {
        bad(
            id,
            err.span().unwrap_or(0..0),
            format!("`{}` is not valid: {}", path.display(), err.message()),
            "check the name of the key and the table it is in",
        )
    })?;
    let dir = path.parent().unwrap_or(Path::new(""));
    let label = path.display().to_string();
    let package = Package::parse(sources, &label, text, dir)?;
    let grants = Grants::parse(sources, &label, text)?;

    let name = match &package {
        Some(package) => Some(own_name(id, package, file.package.as_ref())?),
        None => {
            only_a_package(id, &file)?;
            None
        }
    };
    let header = file.package.map(Spanned::into_inner);
    let version = header
        .as_ref()
        .and_then(|header| header.version.as_ref())
        .map(|written| {
            written
                .get_ref()
                .parse::<Version>()
                .map_err(|err| bad(id, written.span(), err, "write the version as `1.4.0`"))
        })
        .transpose()?;
    let supersedes = header
        .as_ref()
        .and_then(|header| header.supersedes.as_ref())
        .map(|written| {
            let replaced = dependency_name(id, written)?;
            if name.as_ref() == Some(&replaced) {
                return Err(bad(
                    id,
                    written.span(),
                    format!("the package `{replaced}` cannot supersede itself"),
                    "name the package this one replaces",
                ));
            }
            Ok(replaced)
        })
        .transpose()?;
    let files = header
        .and_then(|header| header.files)
        .map(Spanned::into_inner)
        .unwrap_or_default();

    // Keys in file order, so a twin names the line written first.
    let mut lines: Vec<_> = file.dependencies.into_iter().collect();
    lines.sort_by_key(|(written, _)| written.span().start);
    let mut dependencies = Vec::new();
    let mut slugs: BTreeMap<String, Spanned<String>> = BTreeMap::new();
    for (written, value) in lines {
        let dependency_name = dependency_name(id, &written)?;
        if name.as_ref() == Some(&dependency_name) {
            return Err(bad(
                id,
                written.span(),
                format!("the package `{dependency_name}` cannot depend on itself"),
                "delete this line",
            ));
        }
        let slug = dependency_name.slug();
        if let Some(first) = slugs.get(&slug) {
            return Err(twin(id, first, &written, &slug));
        }
        slugs.insert(slug, written.clone());
        dependencies.push(Dependency {
            source: source(id, &dependency_name, value)?,
            name: dependency_name,
        });
    }
    dependencies.sort_by(|a, b| a.name.cmp(&b.name));

    let mut lines: Vec<_> = file.registries.into_iter().collect();
    lines.sort_by_key(|(written, _)| written.span().start);
    let mut registries = Vec::new();
    let mut vendors: BTreeMap<String, Spanned<String>> = BTreeMap::new();
    for (written, line) in lines {
        let vendor = written.get_ref();
        if !nvs_config::grants::is_segment(vendor) {
            return Err(bad(
                id,
                written.span(),
                format!("`{vendor}` is not a vendor"),
                "a vendor is the first part of a package name, such as `Acme`",
            ));
        }
        if vendor.eq_ignore_ascii_case(RESERVED) {
            return Err(bad(
                id,
                written.span(),
                format!("the vendor `{vendor}` cannot have a registry"),
                "`Core` is the standard library, and it is not a package",
            ));
        }
        let folded = vendor.to_lowercase();
        if let Some(first) = vendors.get(&folded) {
            return Err(twin(id, first, &written, &folded));
        }
        vendors.insert(folded, written.clone());
        let line = line.into_inner();
        registries.push(Registry {
            vendor: vendor.clone(),
            url: url(id, &line.url, "registry")?,
            key: line.key,
        });
    }
    registries.sort_by(|a, b| a.vendor.cmp(&b.vendor));

    Ok(Manifest {
        path: path.to_path_buf(),
        package,
        name,
        version,
        files,
        supersedes,
        dependencies,
        registries,
        downloads: file.download.map(Spanned::into_inner).unwrap_or_default(),
        grants,
    })
}

/// The name of the package this file declares, checked against the built-in components.
fn own_name(
    id: SourceId,
    package: &Package,
    header: Option<&Spanned<Header>>,
) -> Result<Name, Diagnostic> {
    let name = Name(package.name().to_owned());
    if let Some(class) = built_in(&name) {
        let span = header
            .and_then(|header| header.get_ref().name.as_ref())
            .map_or(0..0, Spanned::span);
        return Err(bad(
            id,
            span,
            format!("a package cannot be named `{name}`"),
            &format!("`{name}` is a built-in component, which provides `{class}`"),
        ));
    }
    Ok(name)
}

/// A key only a package has, in an application's file: a `version`, `files`, `[capabilities]` or
/// `[[download]]` without a `[package] name`.
fn only_a_package(id: SourceId, file: &File) -> Result<(), Diagnostic> {
    let header = file.package.as_ref();
    let found = header
        .and_then(|header| header.get_ref().version.as_ref())
        .map(|version| ("version", version.span()))
        .or_else(|| {
            header
                .and_then(|header| header.get_ref().files.as_ref())
                .map(|files| ("files", files.span()))
        })
        .or_else(|| {
            header
                .and_then(|header| header.get_ref().supersedes.as_ref())
                .map(|supersedes| ("supersedes", supersedes.span()))
        })
        .or_else(|| {
            file.capabilities
                .as_ref()
                .map(|caps| ("[capabilities]", caps.span()))
        })
        .or_else(|| {
            file.download
                .as_ref()
                .map(|download| ("[[download]]", download.span()))
        });
    match found {
        Some((key, span)) => Err(bad(
            id,
            span,
            format!("`{key}` needs a package name"),
            "only a package has this key: add `name = 'Vendor\\Package'` under `[package]`",
        )),
        None => Ok(()),
    }
}

/// A `[dependencies]` key, checked: two namespace segments, not under `Core`, and not a built-in
/// component.
fn dependency_name(id: SourceId, written: &Spanned<String>) -> Result<Name, Diagnostic> {
    Name::checked(written.get_ref())
        .map_err(|(message, help)| bad(id, written.span(), message, &help))
}

impl Name {
    /// `text` as a dependency's name: two namespace segments, not under `Core`, and not a
    /// built-in component. The index and the lock name packages through this too.
    ///
    /// # Errors
    ///
    /// The message and its help line.
    pub(crate) fn checked(text: &str) -> Result<Self, (String, String)> {
        let bare = text.strip_prefix('\\').unwrap_or(text);
        let segments: Vec<&str> = bare.split('\\').collect();
        if segments.len() != 2 || !segments.iter().all(|s| nvs_config::grants::is_segment(s)) {
            return Err((
                format!("`{text}` is not a package name"),
                "a package name is a vendor and a name, such as `Acme\\Http`".to_owned(),
            ));
        }
        let name = Self(bare.to_owned());
        if name.vendor().eq_ignore_ascii_case(RESERVED) {
            return Err((
                format!("`{text}` is not a package"),
                "`Core` is the standard library, and every program has it".to_owned(),
            ));
        }
        if let Some(class) = built_in(&name) {
            return Err((
                format!("`{text}` is not a package"),
                format!("`{text}` is a built-in component, and every program has `{class}`"),
            ));
        }
        Ok(name)
    }
}

/// The class of the built-in component whose namespace is `name`, compared without case.
fn built_in(name: &Name) -> Option<&'static str> {
    BUILT_IN.into_iter().find(|class| {
        let mut segments = class.split('\\');
        let namespace = [segments.next(), segments.next()];
        let [Some(vendor), Some(package)] = namespace else {
            return false;
        };
        name.slug() == format!("{vendor}/{package}").to_lowercase()
    })
}

/// A `[dependencies]` value: a version string, or a table with `git` and `rev`.
fn source(id: SourceId, name: &Name, value: Spanned<toml::Value>) -> Result<Source, Diagnostic> {
    let span = value.span();
    match value.into_inner() {
        toml::Value::String(text) => text
            .parse()
            .map(|minimum| Source::Registry { minimum })
            .map_err(|err| {
                bad(
                    id,
                    span,
                    err,
                    "write the lowest version you need, such as `1.4.0`",
                )
            }),
        toml::Value::Table(table) => {
            let line: GitLine =
                toml::Value::Table(table)
                    .try_into()
                    .map_err(|err: toml::de::Error| {
                        bad(
                            id,
                            span.clone(),
                            format!(
                                "the git dependency `{name}` is not valid: {}",
                                err.message()
                            ),
                            "a git dependency has `git`, the repository's URL, and `rev`, a commit",
                        )
                    })?;
            let rev = line.rev.as_str();
            let is_commit = matches!(rev.len(), 40 | 64)
                && rev.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
            if !is_commit {
                return Err(bad(
                    id,
                    span,
                    format!("`{rev}` is not a commit"),
                    "`rev` is a full commit hash in lowercase. A branch or a tag can change, so \
                     they are not allowed",
                ));
            }
            let url = url(id, &Spanned::new(span, line.git), "git")?;
            Ok(Source::Git { url, rev: line.rev })
        }
        _ => Err(bad(
            id,
            span,
            format!("the dependency `{name}` is not valid"),
            "write a version, such as `\"1.4.0\"`, or a table with `git` and `rev`",
        )),
    }
}

/// A registry or git URL, checked: `https:` with a host, or `file:`. A trailing `/` is removed.
fn url(id: SourceId, written: &Spanned<String>, what: &str) -> Result<String, Diagnostic> {
    let text = written.get_ref();
    let well_formed = !text.chars().any(char::is_whitespace)
        && (text
            .strip_prefix("https://")
            .is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
            || text
                .strip_prefix("file:")
                .is_some_and(|rest| !rest.is_empty()));
    if !well_formed {
        return Err(bad(
            id,
            written.span(),
            format!("`{text}` is not a {what} URL"),
            "a URL starts with `https://` or, for a directory on this computer, `file:`",
        ));
    }
    Ok(text.trim_end_matches('/').to_owned())
}

/// Two keys that are one name once case is ignored.
fn twin(id: SourceId, first: &Spanned<String>, second: &Spanned<String>, name: &str) -> Diagnostic {
    let mut diagnostic = bad(
        id,
        second.span(),
        format!(
            "`{}` and `{}` are the same name, `{name}`",
            first.get_ref(),
            second.get_ref()
        ),
        "a name is compared without case: delete one of the two lines",
    );
    if let Some(span) = span(id, first.span()) {
        diagnostic = diagnostic.with_secondary(span, "the first line");
    }
    diagnostic
}

/// [`code::E_BAD_PACKAGE_MANIFEST`] at `range`.
fn bad(id: SourceId, range: std::ops::Range<usize>, message: String, help: &str) -> Diagnostic {
    let mut diagnostic =
        Diagnostic::error(code::E_BAD_PACKAGE_MANIFEST, message).with_help(help.to_owned());
    if let Some(span) = span(id, range) {
        diagnostic = diagnostic.with_primary(span, "here");
    }
    diagnostic
}

/// `range` as a [`Span`] in `id`, or `None` past `u32`.
fn span(id: SourceId, range: std::ops::Range<usize>) -> Option<Span> {
    let start = u32::try_from(range.start).ok()?;
    let end = u32::try_from(range.end).ok()?;
    Some(Span::new(id, start, end))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nvs_diagnostics::SourceMap;

    use super::{DEFAULT_REGISTRY, Manifest, Source, Version, parse};

    /// `text` as a `package.toml` in a package directory.
    fn read(text: &str) -> Result<Manifest, String> {
        let mut sources = SourceMap::new();
        parse(&mut sources, Path::new("pkg/package.toml"), text).map_err(|d| d.message)
    }

    /// The message `text` is refused with.
    fn refused(text: &str) -> String {
        match read(text) {
            Ok(_) => panic!("accepted:\n{text}"),
            Err(message) => message,
        }
    }

    /// Three plain numbers parse, and anything else is not a version.
    #[test]
    fn a_version_is_three_plain_numbers() {
        let v: Version = "1.20.3".parse().expect("a version");
        assert_eq!((v.major, v.minor, v.patch), (1, 20, 3));
        assert_eq!(v.to_string(), "1.20.3");
        for bad in [
            "1.2",
            "1.2.3.4",
            "1.02.3",
            "+1.2.3",
            "1.2.3-beta",
            "",
            "a.b.c",
        ] {
            assert!(bad.parse::<Version>().is_err(), "{bad}");
        }
    }

    /// A whole package file reads, and its name gives the slug.
    #[test]
    fn a_package_file_reads_whole() {
        let rev = "a".repeat(40);
        let manifest = read(&format!(
            "[package]\nname = 'Acme\\HttpKit'\nversion = \"1.4.0\"\nfiles = [\"a.html\"]\nsupersedes = 'Acme\\Http'\n\
             [dependencies]\n'Acme\\Log' = \"2.1.0\"\n'Beta\\Fork' = {{ git = \"https://git.example.com/f.git\", rev = \"{rev}\" }}\n\
             [registries]\nAcme = {{ url = \"file:registry/\" }}\n\
             [capabilities]\nrequired = [\"net.connect\"]\n\
             [grants]\n'Acme\\Log' = []\n\
             [[download]]\npath = \"fonts/Big.otf\"\nurls = [\"https://example.com/Big.otf\"]\nsha256 = \"00\"\nsize = 3\n"
        ))
        .expect("a valid file");
        let name = manifest.name.as_ref().expect("a package");
        assert_eq!(name.slug(), "acme/httpkit");
        assert_eq!(name.vendor(), "Acme");
        assert_eq!(
            manifest.version.map(|v| v.to_string()).as_deref(),
            Some("1.4.0")
        );
        assert_eq!(manifest.files, ["a.html"]);
        assert_eq!(
            manifest
                .supersedes
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("Acme\\Http")
        );
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(
            manifest.dependencies[1].source,
            Source::Git {
                url: "https://git.example.com/f.git".to_owned(),
                rev
            }
        );
        assert_eq!(manifest.registry("acme"), "file:registry");
        assert_eq!(manifest.registry("Beta"), DEFAULT_REGISTRY);
        assert_eq!(manifest.downloads[0].size, 3);
        assert_eq!(manifest.grants.lines().len(), 1);
    }

    /// An application's file has no name, and every vendor it does not map uses the default.
    #[test]
    fn an_application_file_has_no_name() {
        let manifest = read("[dependencies]\n'Acme\\Log' = \"1.0.0\"\n").expect("valid");
        assert!(manifest.name.is_none() && manifest.package.is_none());
        assert_eq!(manifest.registry("Acme"), DEFAULT_REGISTRY);
    }

    /// Each refusal, by the words its message starts with.
    #[test]
    fn each_refusal_names_what_is_wrong() {
        let pkg = "[package]\nname = 'Acme\\Http'\n";
        let cases: &[(String, &str)] = &[
            (format!("{pkg}colour = 1\n"), "unknown field `colour`"),
            ("[publish]\n".to_owned(), "unknown"),
            (
                format!("{pkg}version = \"1.4\"\n"),
                "`1.4` is not a version",
            ),
            (
                "[package]\nname = 'Acme'\n".to_owned(),
                "`Acme` is not a package name",
            ),
            (
                "[package]\nname = 'Core\\Http'\n".to_owned(),
                "a package cannot be named",
            ),
            (
                "[package]\nname = 'novis\\image'\n".to_owned(),
                "a package cannot be named",
            ),
            (
                "[package]\nversion = \"1.0.0\"\n".to_owned(),
                "`version` needs a package name",
            ),
            (
                format!("{pkg}supersedes = 'Acme\\Http'\n"),
                "the package `Acme\\Http` cannot supersede itself",
            ),
            (
                format!("{pkg}supersedes = 'Acme'\n"),
                "`Acme` is not a package name",
            ),
            (
                "[capabilities]\nrequired = []\n".to_owned(),
                "`[capabilities]` needs a package name",
            ),
            (
                format!("{pkg}[capabilities]\nwanted = []\n"),
                "unknown field `wanted`",
            ),
            (
                format!("{pkg}[dependencies]\n'Acme\\Http' = \"1.0.0\"\n"),
                "the package `Acme\\Http` cannot depend on itself",
            ),
            (
                format!("{pkg}[dependencies]\n'Acme' = \"1.0.0\"\n"),
                "`Acme` is not a package name",
            ),
            (
                format!("{pkg}[dependencies]\n'Core\\Str' = \"1.0.0\"\n"),
                "`Core\\Str` is not a package",
            ),
            (
                format!("{pkg}[dependencies]\n'Novis\\Pdf' = \"1.0.0\"\n"),
                "`Novis\\Pdf` is not a package",
            ),
            (
                format!("{pkg}[dependencies]\n'Acme\\Log' = \"1\"\n"),
                "`1` is not a version",
            ),
            (
                format!("{pkg}[dependencies]\n'Acme\\Log' = \"1.0.0\"\n'acme\\log' = \"1.0.0\"\n"),
                "`Acme\\Log` and `acme\\log` are the same name, `acme/log`",
            ),
            (
                format!("{pkg}[dependencies]\n'Acme\\Log' = 1\n"),
                "the dependency `Acme\\Log` is not valid",
            ),
            (
                format!(
                    "{pkg}[dependencies]\n'Acme\\Log' = {{ git = \"https://x.example\", rev = \"main\" }}\n"
                ),
                "`main` is not a commit",
            ),
            (
                format!(
                    "{pkg}[dependencies]\n'Acme\\Log' = {{ git = \"https://x.example\", branch = \"main\" }}\n"
                ),
                "the git dependency `Acme\\Log` is not valid",
            ),
            (
                format!(
                    "{pkg}[dependencies]\n'Acme\\Log' = {{ git = \"http://x.example\", rev = \"{}\" }}\n",
                    "a".repeat(40)
                ),
                "`http://x.example` is not a git URL",
            ),
            (
                format!("{pkg}[registries]\nAcme = {{ url = \"http://r.example\" }}\n"),
                "`http://r.example` is not a registry URL",
            ),
            (
                format!(
                    "{pkg}[registries]\nAcme = {{ url = \"https://r.example\", mirror = 1 }}\n"
                ),
                "unknown field `mirror`",
            ),
            (
                format!("{pkg}[registries]\n'Acme\\Http' = {{ url = \"https://r.example\" }}\n"),
                "`Acme\\Http` is not a vendor",
            ),
            (
                format!("{pkg}[registries]\ncore = {{ url = \"https://r.example\" }}\n"),
                "the vendor `core` cannot have a registry",
            ),
            (
                format!(
                    "{pkg}[registries]\nAcme = {{ url = \"https://a.example\" }}\nACME = {{ url = \"https://b.example\" }}\n"
                ),
                "are the same name, `acme`",
            ),
            (
                format!(
                    "{pkg}[[download]]\npath = \"a\"\nurls = []\nsha256 = \"0\"\nsize = 1\nmirror = 2\n"
                ),
                "unknown field `mirror`",
            ),
        ];
        for (text, start) in cases {
            let message = refused(text);
            assert!(message.contains(start), "{message}\nfor:\n{text}");
        }
    }
}
