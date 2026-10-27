//! A package's own `package.toml`, read for the two things the compiler needs of it: the
//! package's name and the capabilities it requests.
//!
//! ```toml
//! [package]
//! name = 'Acme\Http'
//!
//! [capabilities]
//! required = ["net.connect"]
//! optional = ["fs.write"]
//! ```
//!
//! **This module reads `[package] name` and `[capabilities]`, and nothing else of the file**
//! (`rule:packaging/package-toml-is-a-tool-input`). A version, the dependencies and every other
//! table are the `nvs pkg` commands', and an unknown key there is their refusal to make.
//!
//! **A name is the package's root namespace**, exactly two segments, `Vendor\Package`
//! (`rule:packaging/a-package-name-is-vendor-slash-name`). A leading `\` is allowed and means
//! nothing, and a first segment `Core` is refused in any case, as a `[grants]` key's is. A file
//! with no `[package] name` is not a package's: it is an application's, and [`read`] answers
//! `None` for it.
//!
//! **A capability is required or optional, never both**
//! (`rule:security/optional-capability-degrades`), and each is a name on the roster
//! (`rule:security/capability-roster-is-closed`). Everything this module refuses is
//! [`code::E_BAD_PACKAGE_MANIFEST`].
//!
//! Cost: one read of each package's file, once per compiler run. Memory: a name and two short
//! lists per package, held for that run and never per request.

use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostic, SourceId, SourceMap, Span, code};
use serde::Deserialize;
use toml::Spanned;

use crate::capability::Cap;
use crate::grants::{FILE, is_segment};

/// The namespace no package may take, compared without case.
const RESERVED: &str = "Core";

/// One package: its name, the directory its `package.toml` is in, and what it requests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    /// `Vendor\Package`, without a leading `\`.
    name: String,
    /// The directory that holds the package's `package.toml`. Every file of the package is in it.
    dir: PathBuf,
    /// The capabilities the package cannot work without, in [`Cap::ALL`]'s order.
    required: Vec<Cap>,
    /// The capabilities the package can work without, in [`Cap::ALL`]'s order.
    optional: Vec<Cap>,
}

impl Package {
    /// The package's name and root namespace, `Vendor\Package`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The directory that holds the package's `package.toml`.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The capabilities `[capabilities] required` lists.
    #[must_use]
    pub fn required(&self) -> &[Cap] {
        &self.required
    }

    /// The capabilities `[capabilities] optional` lists.
    #[must_use]
    pub fn optional(&self) -> &[Cap] {
        &self.optional
    }

    /// Whether `namespace` is the package's own namespace or one below it. Compared exactly
    /// (`rule:classes/names-resolve-case-sensitively`), and a leading `\` means nothing.
    #[must_use]
    pub fn owns(&self, namespace: &str) -> bool {
        let namespace = namespace.strip_prefix('\\').unwrap_or(namespace);
        match namespace.strip_prefix(self.name.as_str()) {
            Some(rest) => rest.is_empty() || rest.starts_with('\\'),
            None => false,
        }
    }

    /// The package `text` declares, a `package.toml` in `dir` registered in `sources` as `name`.
    ///
    /// # Errors
    ///
    /// [`code::E_BAD_PACKAGE_MANIFEST`], as [`read`] refuses it.
    pub fn parse(
        sources: &mut SourceMap,
        name: &str,
        text: &str,
        dir: &Path,
    ) -> Result<Option<Self>, Diagnostic> {
        let id = sources.add(name, text);
        parse_in(id, text, dir)
    }
}

/// The package whose `package.toml` is in `dir`, or `None` when the file has no
/// `[package] name` and so is an application's.
///
/// The file is loaded into `sources` by its path, so a server that holds a unit sees an edit to
/// it.
///
/// # Errors
///
/// [`code::E_BAD_PACKAGE_MANIFEST`]: a file that cannot be read or is not TOML, a name that is
/// not two namespace segments or is under `Core`, a capability not on the roster, or one listed
/// as both required and optional.
pub fn read(dir: &Path, sources: &mut SourceMap) -> Result<Option<Package>, Diagnostic> {
    let path = dir.join(FILE);
    let id = sources.load(&path).map_err(|err| {
        Diagnostic::error(
            code::E_BAD_PACKAGE_MANIFEST,
            format!("cannot read `{}`: {err}", path.display()),
        )
        .with_note(
            "the compiler reads the package's name and capabilities from this file".to_owned(),
        )
    })?;
    parse_in(id, sources.file(id).text(), dir)
}

/// The part of `package.toml` this module reads. Every other table is left to `nvs pkg`.
#[derive(Deserialize)]
struct File {
    #[serde(default)]
    package: Option<Header>,
    #[serde(default)]
    capabilities: Option<Requests>,
}

/// `[package]`, of which this module reads the name alone.
#[derive(Deserialize)]
struct Header {
    #[serde(default)]
    name: Option<Spanned<String>>,
}

/// `[capabilities]`.
#[derive(Deserialize)]
struct Requests {
    #[serde(default)]
    required: Vec<Spanned<String>>,
    #[serde(default)]
    optional: Vec<Spanned<String>>,
}

/// [`Package::parse`] over `text`, already in the caller's map as `id`.
fn parse_in(id: SourceId, text: &str, dir: &Path) -> Result<Option<Package>, Diagnostic> {
    let (file, _) = crate::file::parse_at::<File>(id, text)
        .map_err(|refused| refused.with_code(code::E_BAD_PACKAGE_MANIFEST))?;
    let Some(written) = file.package.and_then(|header| header.name) else {
        return Ok(None);
    };
    let name = package_name(id, &written)?;
    let requests = file.capabilities.unwrap_or(Requests {
        required: Vec::new(),
        optional: Vec::new(),
    });
    let required = caps(id, &requests.required)?;
    let optional = caps(id, &requests.optional)?;
    if let Some(both) = requests
        .optional
        .iter()
        .find(|written| Cap::parse(written.get_ref()).is_some_and(|cap| required.contains(&cap)))
    {
        return Err(bad(
            id,
            both,
            format!("`{}` is both required and optional", both.get_ref()),
            "list it in `required` or in `optional`, not in both",
        ));
    }
    Ok(Some(Package {
        name,
        dir: dir.to_path_buf(),
        required,
        optional,
    }))
}

/// The name `written`, checked: two namespace segments, the first not `Core`.
fn package_name(id: SourceId, written: &Spanned<String>) -> Result<String, Diagnostic> {
    let text = written.get_ref();
    let bare = text.strip_prefix('\\').unwrap_or(text);
    let segments: Vec<&str> = bare.split('\\').collect();
    if segments.len() != 2 || !segments.iter().all(|segment| is_segment(segment)) {
        return Err(bad(
            id,
            written,
            format!("`{text}` is not a package name"),
            "a package name is a vendor and a name, such as `Acme\\Http`",
        ));
    }
    if segments[0].eq_ignore_ascii_case(RESERVED) {
        return Err(bad(
            id,
            written,
            format!("a package cannot be named `{text}`"),
            "`Core` is the standard library, and its namespace is reserved",
        ));
    }
    Ok(bare.to_owned())
}

/// Each name of one `[capabilities]` list, checked against the roster.
fn caps(id: SourceId, names: &[Spanned<String>]) -> Result<Vec<Cap>, Diagnostic> {
    let mut found = Vec::new();
    for name in names {
        let Some(cap) = Cap::parse(name.get_ref()) else {
            let roster: Vec<&str> = Cap::ALL.iter().map(|cap| cap.name()).collect();
            return Err(bad(
                id,
                name,
                format!("`{}` is not a capability", name.get_ref()),
                &format!("the capabilities are {}", roster.join(", ")),
            ));
        };
        found.push(cap);
    }
    found.sort_by_key(|cap| Cap::ALL.iter().position(|each| each == cap));
    found.dedup();
    Ok(found)
}

/// [`code::E_BAD_PACKAGE_MANIFEST`] at `at`.
fn bad(id: SourceId, at: &Spanned<String>, message: String, help: &str) -> Diagnostic {
    let mut diagnostic =
        Diagnostic::error(code::E_BAD_PACKAGE_MANIFEST, message).with_help(help.to_owned());
    let range = at.span();
    if let (Ok(start), Ok(end)) = (u32::try_from(range.start), u32::try_from(range.end)) {
        diagnostic = diagnostic.with_primary(Span::new(id, start, end), "here");
    }
    diagnostic
}
