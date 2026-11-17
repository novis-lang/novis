//! `rule:packaging/a-restriction-limits-where-a-declaration-is-used`: the `[[restrict]]` tables of
//! the application's `package.toml` and of each package's own, read into one type both readers
//! share.
//!
//! ```toml
//! [[restrict]]
//! use     = 'Core\Router::urlSigned'
//! allow   = ['Framework\Core\Signing\*']
//! note    = "Use `Framework\Core\Url::signed`. It adds the framework's key rotation."
//! replace = 'Framework\Core\Url::signed($route, $params)'
//! ```
//!
//! **This module checks what the file alone can check.** A `use` or an `allow` entry is a
//! [`Target`]: a name (`'Ns\Name'`, which may be a namespace, a class or a function), a subtree
//! (`'Ns\*'`), or a member (`'Ns\Name::member'`). A segment that is not a name, a `\*` anywhere
//! but at the end, an empty `use`, a `replace` on a subtree, a field this module does not know, and
//! in a package's file an `allow` entry outside the package's own name are each
//! [`code::E_BAD_RESTRICTION`] at the place they are written. Whether a target names a declaration
//! of the program needs the program, and the check after type checking asks it. A `Core` target is
//! the one case this crate can be told about: [`Restrictions::check_core`] takes the standard
//! library's lookup from its caller.
//!
//! **A leading `\` means nothing**, as in a `[grants]` key, and names compare exactly
//! (`rule:classes/names-resolve-case-sensitively`).
//!
//! **[`Restrictions::digest`] is `None` for a program with no table**, so `crate::cache` adds
//! nothing to the key and that program keeps the key it had. Otherwise it covers every entry of
//! every table, in the order the tables were read, with each field length-prefixed. The file and
//! the line are not in it: two checkouts with the same entries compile the same units.
//!
//! Cost: one more TOML pass over each `package.toml` the compiler reads, once per compiler run.
//! Memory: a few short strings per entry, held once per compiled program and never per request.

use std::fmt;

use nvs_diagnostics::{Diagnostic, SourceId, Span, code};
use serde::Deserialize;
use toml::Spanned;

use crate::grants::is_segment;

/// What a `use` or an `allow` entry names.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Target {
    /// `'Ns\Name'`: a namespace's own declarations, a class, interface or enum, or a function.
    /// Which one is a question about the program, not about the file.
    Name(String),
    /// `'Ns\*'`: the namespace `Ns` and every namespace below it.
    Subtree(String),
    /// `'Ns\Name::member'`: a method, property or class constant of a class. A property may be
    /// written with or without its `$`, and is held without it.
    Member {
        /// The class, without a leading `\`.
        class: String,
        /// The member's name, without a `$`.
        member: String,
    },
}

impl Target {
    /// The namespace or class the target is under: the name, the subtree's root or the member's
    /// class.
    #[must_use]
    pub fn root(&self) -> &str {
        match self {
            Self::Name(name) | Self::Subtree(name) => name,
            Self::Member { class, .. } => class,
        }
    }

    /// Whether the target lies under the standard library, compared without case as
    /// `rule:core-api/reserved-namespace` reserves it.
    #[must_use]
    pub fn is_core(&self) -> bool {
        self.root()
            .split('\\')
            .next()
            .is_some_and(|first| first.eq_ignore_ascii_case("Core"))
    }

    /// Whether the target lies inside the namespace `name`: `name` itself or one below it.
    #[must_use]
    pub fn is_under(&self, name: &str) -> bool {
        match self.root().strip_prefix(name) {
            Some(rest) => rest.is_empty() || rest.starts_with('\\'),
            None => false,
        }
    }

    /// The target `written` names, or `None` when it is not a target's shape.
    fn parse(written: &str) -> Option<Self> {
        let bare = written.strip_prefix('\\').unwrap_or(written);
        if let Some((class, member)) = bare.split_once("::") {
            let member = member.strip_prefix('$').unwrap_or(member);
            return (path(class) && is_segment(member)).then(|| Self::Member {
                class: class.to_owned(),
                member: member.to_owned(),
            });
        }
        if let Some(root) = bare.strip_suffix("\\*") {
            return path(root).then(|| Self::Subtree(root.to_owned()));
        }
        path(bare).then(|| Self::Name(bare.to_owned()))
    }

    /// Feeds the target to the digest, with a tag byte for its form.
    fn feed(&self, hasher: &mut blake3::Hasher) {
        match self {
            Self::Name(name) => {
                hasher.update(&[0]);
                text(hasher, name);
            }
            Self::Subtree(name) => {
                hasher.update(&[1]);
                text(hasher, name);
            }
            Self::Member { class, member } => {
                hasher.update(&[2]);
                text(hasher, class);
                text(hasher, member);
            }
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(name) => f.write_str(name),
            Self::Subtree(name) => write!(f, "{name}\\*"),
            Self::Member { class, member } => write!(f, "{class}::{member}"),
        }
    }
}

/// One `[[restrict]]` entry, checked, and where it was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Restriction {
    target: Target,
    allow: Vec<Target>,
    note: Option<String>,
    replace: Option<String>,
    /// The package whose file this is, or `None` for the application's.
    package: Option<String>,
    /// The file's name as the caller registered it.
    file: String,
    /// The line of the entry's `use`, from 1.
    line: usize,
    /// The span of the entry's `use` value.
    span: Option<Span>,
    /// The span of the `replace` value, its quotes included.
    replace_span: Option<Span>,
}

impl Restriction {
    /// What the entry restricts.
    #[must_use]
    pub fn target(&self) -> &Target {
        &self.target
    }

    /// The places that may use the target. Empty when no place may.
    #[must_use]
    pub fn allow(&self) -> &[Target] {
        &self.allow
    }

    /// The Markdown shown in the error and on hover.
    #[must_use]
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    /// The expression that replaces a use, as written.
    #[must_use]
    pub fn replace(&self) -> Option<&str> {
        self.replace.as_deref()
    }

    /// The package whose `package.toml` holds the entry, or `None` for the application's.
    #[must_use]
    pub fn package(&self) -> Option<&str> {
        self.package.as_deref()
    }

    /// The file the entry is in, as its reader registered it.
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }

    /// The line of the entry's `use`, counted from 1.
    #[must_use]
    pub fn line(&self) -> usize {
        self.line
    }

    /// The span of the entry's `use` value.
    #[must_use]
    pub fn span(&self) -> Option<Span> {
        self.span
    }

    /// Where the replacement's code starts in the file and the span of its whole value, quotes
    /// included. `None` when there is no `replace`, or when its value is not the text written
    /// between its quotes, as with an escape sequence: the code must be the written text, so an
    /// error inside it points at the bytes in the file.
    #[must_use]
    pub fn replace_at(&self) -> Option<(u32, Span)> {
        let (text, span) = (self.replace.as_deref()?, self.replace_span?);
        let written = usize::try_from(span.end.checked_sub(span.start)?).ok()?;
        let quote = match written.checked_sub(text.len())? {
            2 => 1,
            6 => 3,
            _ => return None,
        };
        Some((span.start + quote, span))
    }

    /// The span of the `replace` value, quotes included.
    #[must_use]
    pub fn replace_span(&self) -> Option<Span> {
        self.replace_span
    }

    /// Feeds the entry to the digest. The file and the line are left out.
    fn feed(&self, hasher: &mut blake3::Hasher) {
        self.target.feed(hasher);
        hasher.update(&len(self.allow.len()));
        for place in &self.allow {
            place.feed(hasher);
        }
        for optional in [&self.note, &self.replace, &self.package] {
            match optional {
                Some(value) => {
                    hasher.update(&[1]);
                    text(hasher, value);
                }
                None => {
                    hasher.update(&[0]);
                }
            }
        }
    }
}

/// Every `[[restrict]]` entry the program read, from the application's file and each package's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Restrictions {
    entries: Vec<Restriction>,
}

impl Restrictions {
    /// No entry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Every entry, in the order the tables were read.
    #[must_use]
    pub fn entries(&self) -> &[Restriction] {
        &self.entries
    }

    /// Whether no table has an entry.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Replaces every package's entries with `more`, keeping the application's own ahead of
    /// them. A compiler that walks the program again calls it again with what it found.
    pub fn set_packages(&mut self, more: impl IntoIterator<Item = Restriction>) {
        self.entries.retain(|entry| entry.package.is_none());
        self.entries.extend(more);
    }

    /// `BLAKE3` over every entry, or `None` when there is none, so a program with no table adds
    /// nothing to its key.
    #[must_use]
    pub fn digest(&self) -> Option<[u8; 32]> {
        if self.entries.is_empty() {
            return None;
        }
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"restrict");
        hasher.update(&len(self.entries.len()));
        for entry in &self.entries {
            entry.feed(&mut hasher);
        }
        Some(*hasher.finalize().as_bytes())
    }

    /// Checks each target under `Core` against `known`, the standard library's lookup: `true`
    /// when the target names a `Core` declaration or namespace.
    ///
    /// # Errors
    ///
    /// [`code::E_BAD_RESTRICTION`] at the first `Core` target that names nothing.
    pub fn check_core(&self, known: impl Fn(&Target) -> bool) -> Result<(), Diagnostic> {
        let Some(entry) = self
            .entries
            .iter()
            .find(|entry| entry.target.is_core() && !known(&entry.target))
        else {
            return Ok(());
        };
        let mut diagnostic = Diagnostic::error(
            code::E_BAD_RESTRICTION,
            format!("`{}` is not part of `Core`", entry.target),
        )
        .with_help("check the name for a typo".to_owned());
        if let Some(span) = entry.span {
            diagnostic = diagnostic.with_primary(span, "this name");
        }
        Err(diagnostic)
    }
}

/// The `[[restrict]]` table of `text`, a `package.toml` registered as `id` under the name `file`.
/// `package` is the package's name, or `None` for the application's file.
///
/// # Errors
///
/// [`code::E_BAD_RESTRICTION`], as the module says.
pub(crate) fn parse(
    id: SourceId,
    text: &str,
    file: &str,
    package: Option<&str>,
) -> Result<Vec<Restriction>, Diagnostic> {
    let (tables, _) = crate::file::parse_at::<Tables>(id, text)
        .map_err(|refused| refused.with_code(code::E_BAD_RESTRICTION))?;
    let mut entries = Vec::with_capacity(tables.restrict.len());
    for entry in tables.restrict {
        let line = text
            .get(..entry.target.span().start)
            .map_or(1, |before| before.matches('\n').count() + 1);
        let target = read_target(id, &entry.target)?;
        let mut allow = Vec::with_capacity(entry.allow.get_ref().len());
        for written in entry.allow.get_ref() {
            let place = read_target(id, written)?;
            if let Some(owner) = package
                && !place.is_under(owner)
            {
                return Err(bad(
                    id,
                    written,
                    format!(
                        "`{}` is not inside the package `{owner}`",
                        written.get_ref()
                    ),
                    &format!(
                        "a package's restriction can allow only its own code, under `{owner}`"
                    ),
                ));
            }
            allow.push(place);
        }
        if let (Some(written), Target::Subtree(_)) = (&entry.replace, &target) {
            return Err(bad(
                id,
                written,
                format!(
                    "`{}` restricts a namespace, so it has no replacement",
                    entry.target.get_ref()
                ),
                "a `replace` belongs to a method or a function. Delete this line",
            ));
        }
        entries.push(Restriction {
            span: span(id, &entry.target),
            replace_span: entry.replace.as_ref().and_then(|written| span(id, written)),
            target,
            allow,
            note: entry.note.map(Spanned::into_inner),
            replace: entry.replace.map(Spanned::into_inner),
            package: package.map(str::to_owned),
            file: file.to_owned(),
            line,
        });
    }
    Ok(entries)
}

/// The part of `package.toml` this module reads. Every other table is another reader's.
#[derive(Deserialize)]
struct Tables {
    #[serde(default)]
    restrict: Vec<Entry>,
}

/// One `[[restrict]]` entry as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    #[serde(rename = "use")]
    target: Spanned<String>,
    allow: Spanned<Vec<Spanned<String>>>,
    #[serde(default)]
    note: Option<Spanned<String>>,
    #[serde(default)]
    replace: Option<Spanned<String>>,
}

/// The target `written` names, checked.
fn read_target(id: SourceId, written: &Spanned<String>) -> Result<Target, Diagnostic> {
    Target::parse(written.get_ref()).ok_or_else(|| {
        bad(
            id,
            written,
            format!("`{}` is not the name of a declaration", written.get_ref()),
            "write a class such as `Acme\\Http\\Client`, a method such as \
             `Acme\\Http\\Client::send`, a namespace such as `Acme\\Http`, or `Acme\\Http\\*` \
             for a namespace and every namespace below it",
        )
    })
}

/// Whether `name` is one or more namespace segments joined with `\`.
fn path(name: &str) -> bool {
    !name.is_empty() && name.split('\\').all(is_segment)
}

/// [`code::E_BAD_RESTRICTION`] at `at`.
fn bad(id: SourceId, at: &Spanned<String>, message: String, help: &str) -> Diagnostic {
    let mut diagnostic =
        Diagnostic::error(code::E_BAD_RESTRICTION, message).with_help(help.to_owned());
    if let Some(span) = span(id, at) {
        diagnostic = diagnostic.with_primary(span, "here");
    }
    diagnostic
}

/// The span of a value TOML read, in the file `id` names.
fn span<T>(id: SourceId, at: &Spanned<T>) -> Option<Span> {
    let range = at.span();
    let start = u32::try_from(range.start).ok()?;
    let end = u32::try_from(range.end).ok()?;
    Some(Span::new(id, start, end))
}

/// Feeds `value`, length-prefixed.
fn text(hasher: &mut blake3::Hasher, value: &str) {
    hasher.update(&len(value.len()));
    hasher.update(value.as_bytes());
}

/// A length as the eight bytes the digest feeds.
fn len(n: usize) -> [u8; 8] {
    u64::try_from(n).unwrap_or(u64::MAX).to_le_bytes()
}
