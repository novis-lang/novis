//! `rule:security/grants-are-keyed-on-a-namespace`'s grant table: the `[grants]` table of the
//! application's own `package.toml`, read into keys on namespace prefixes, and the lookup
//! `rule:security/longest-prefix-wins` describes.
//!
//! ```toml
//! [grants]
//! 'Acme\Http' = ["net.connect"]       # the namespace `Acme\Http` itself
//! 'Acme\Http\*' = ["net.connect"]     # `Acme\Http` and every namespace below it
//! 'Acme\Http\Cache' = []              # this one namespace, granted nothing
//! ```
//!
//! **This module reads `[grants]` and nothing else of the file**
//! (`rule:packaging/package-toml-is-a-tool-input`). Every other table is the `nvs pkg` commands',
//! and an unknown one there is their refusal to make, not this one's. The file is found by walking
//! up from a directory, the way `git` finds `.git` ([`find`]); a tree with no `package.toml` has
//! an empty table, and so does a file with no `[grants]`.
//!
//! **A key is a namespace, compared exactly** (`rule:classes/names-resolve-case-sensitively`). A
//! leading `\` is allowed and means nothing, so `'\Acme\Http'` and `'Acme\Http'` are the same key,
//! and writing both is the one way two keys can match identically: [`code::E_GRANT_KEYS_TIE`]
//! names both. A key that ends `\*` is the subtree form and matches its own namespace and every
//! depth below it; a key without it matches that namespace's own declarations alone. The key
//! with the most segments wins, and at equal length the exact key beats the subtree key.
//!
//! **Three refusals hold the table to its shape.** A capability that is not on the roster
//! (`rule:security/capability-roster-is-closed`), a key that is not a namespace, and a value that
//! is not a list of names are [`code::E_BAD_GRANT`]. A key whose first segment is `Core` in any
//! case is [`code::E_GRANT_UNDER_CORE`], because `Core` is reserved
//! (`rule:core-api/reserved-namespace`) and no grant may name it. A grant is a capability's name
//! and never a scope: the operator's `nvs.toml` holds the scope, and the grant a call site holds
//! is the intersection of the two.
//!
//! **[`Grants::digest`] is the part every unit's cache key carries**
//! (`rule:config/the-extension-set-is-in-every-unit-key`), so an edited grant line is an ordinary
//! cache miss. It is over the normalized keys and their capabilities, so a comment, a reordering
//! or a change anywhere else in the file keeps every key.
//!
//! Cost: one read of the file and one `metadata` per directory walked, once per compiler run, and
//! a table of a few dozen keys held for that run. Memory: the table is compile-time input, and
//! nothing of it is held per request.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostic, SourceId, SourceMap, Span, code};
use serde::Deserialize;
use toml::Spanned;

use crate::capability::Cap;

/// The file name [`find`] looks for.
pub const FILE: &str = "package.toml";

/// The namespace no key may name, compared without case.
const RESERVED: &str = "Core";

/// The application's grant table, read from its `package.toml`.
///
/// [`Default`] is the empty table: every namespace is unmatched, and so holds the application's
/// authority (`rule:security/an-unmatched-namespace-holds-the-application`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Grants {
    /// Every key, sorted by namespace and then exact before subtree.
    keys: Vec<Grant>,
    /// The file the table was read from, or `None` when no file was found.
    file: Option<PathBuf>,
}

/// One line of the table: a key and the capabilities it grants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    /// The namespace, without a leading `\` and without the `\*` of the subtree form.
    namespace: String,
    /// Whether the key ends `\*` and so matches every depth below [`Self::namespace`].
    subtree: bool,
    /// The capabilities granted, in [`Cap::ALL`]'s order, each once.
    caps: Vec<Cap>,
}

impl Grant {
    /// The namespace the key names, without a leading `\` or the `\*` of the subtree form.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Whether the key is the subtree form, `Vendor\Package\*`.
    #[must_use]
    pub fn is_subtree(&self) -> bool {
        self.subtree
    }

    /// The key as a person writes it in `[grants]`: `Acme\Http` or `Acme\Http\*`.
    #[must_use]
    pub fn key(&self) -> String {
        if self.subtree {
            format!("{}\\*", self.namespace)
        } else {
            self.namespace.clone()
        }
    }

    /// The capabilities this line grants, in [`Cap::ALL`]'s order.
    #[must_use]
    pub fn caps(&self) -> &[Cap] {
        &self.caps
    }

    /// Whether this line grants `cap`.
    #[must_use]
    pub fn grants(&self, cap: Cap) -> bool {
        self.caps.contains(&cap)
    }

    /// Whether the key matches a declaration in `namespace`, which has no leading `\`.
    fn matches(&self, namespace: &str) -> bool {
        match namespace.strip_prefix(self.namespace.as_str()) {
            Some("") => true,
            Some(rest) => self.subtree && rest.starts_with('\\'),
            None => false,
        }
    }

    /// The key's number of segments, which is what the longest match compares.
    fn depth(&self) -> usize {
        self.namespace.split('\\').count()
    }
}

impl Grants {
    /// The table of the `package.toml` that [`find`] finds from `start`, or the empty table when
    /// there is none.
    ///
    /// # Errors
    ///
    /// A file that cannot be read, or one whose `[grants]` table [`Self::parse`] refuses.
    pub fn read(start: &Path, sources: &mut SourceMap) -> Result<Self, Diagnostic> {
        let Some(path) = find(start) else {
            return Ok(Self::default());
        };
        let text = std::fs::read_to_string(&path).map_err(|err| {
            Diagnostic::error(
                code::E_BAD_GRANT,
                format!("cannot read `{}`: {err}", path.display()),
            )
            .with_note("the compiler reads the `[grants]` table of this file".to_owned())
        })?;
        let mut grants = Self::parse(sources, &path.display().to_string(), &text)?;
        grants.file = Some(path);
        Ok(grants)
    }

    /// The `[grants]` table of `text`, a `package.toml` registered in `sources` as `name`.
    ///
    /// # Errors
    ///
    /// A file that is not TOML, a `[grants]` that is not a table of lists, a capability not on
    /// the roster, or a key that is not a namespace: [`code::E_BAD_GRANT`]. A key under `Core`:
    /// [`code::E_GRANT_UNDER_CORE`]. Two keys that match identically:
    /// [`code::E_GRANT_KEYS_TIE`].
    pub fn parse(sources: &mut SourceMap, name: &str, text: &str) -> Result<Self, Diagnostic> {
        let (id, parsed) = crate::file::parse::<File>(sources, name, text);
        let file = parsed.map_err(|refused| refused.with_code(code::E_BAD_GRANT))?;
        let mut lines: Vec<_> = file.grants.into_iter().collect();
        lines.sort_by_key(|(written, _)| written.span().start);
        let mut keys: BTreeMap<(String, bool), (Grant, Spanned<String>)> = BTreeMap::new();
        for (written, caps) in lines {
            let grant = grant(id, &written, caps)?;
            let slot = (grant.namespace.clone(), grant.subtree);
            if let Some((_, first)) = keys.get(&slot) {
                return Err(tie(id, first, &written));
            }
            keys.insert(slot, (grant, written));
        }
        Ok(Self {
            keys: keys.into_values().map(|(grant, _)| grant).collect(),
            file: None,
        })
    }

    /// The line that holds for a declaration in `namespace`, or `None` when no key matches and
    /// the declaration holds the application's authority.
    ///
    /// `namespace` may start with `\`. The global namespace matches no key.
    #[must_use]
    pub fn lookup(&self, namespace: &str) -> Option<&Grant> {
        let namespace = namespace.strip_prefix('\\').unwrap_or(namespace);
        self.keys
            .iter()
            .filter(|grant| grant.matches(namespace))
            .max_by_key(|grant| (grant.depth(), !grant.subtree))
    }

    /// Every line, sorted by namespace and then exact before subtree.
    #[must_use]
    pub fn lines(&self) -> &[Grant] {
        &self.keys
    }

    /// Whether the table has no line.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// The `package.toml` the table was read from, or `None` when [`find`] found none.
    #[must_use]
    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    /// `BLAKE3` over every line, the digest each compiled unit's key carries.
    ///
    /// Each key and capability name is length-prefixed, so two tables never hash alike by
    /// moving a byte from one field to the next. The file's path is not in it: two checkouts
    /// that grant the same lines compile the same units.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&len(self.keys.len()));
        for grant in &self.keys {
            hasher.update(&len(grant.namespace.len()));
            hasher.update(grant.namespace.as_bytes());
            hasher.update(&[u8::from(grant.subtree)]);
            hasher.update(&len(grant.caps.len()));
            for cap in &grant.caps {
                hasher.update(&len(cap.name().len()));
                hasher.update(cap.name().as_bytes());
            }
        }
        *hasher.finalize().as_bytes()
    }
}

/// The nearest `package.toml` in `start` or a directory above it, the way `git` finds `.git`.
#[must_use]
pub fn find(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|dir| dir.join(FILE))
        .find(|candidate| candidate.is_file())
}

/// The part of `package.toml` this module reads. Every other table is left to `nvs pkg`.
#[derive(Deserialize)]
struct File {
    #[serde(default)]
    grants: BTreeMap<Spanned<String>, Spanned<Vec<Spanned<String>>>>,
}

/// One line, checked: the key is a namespace outside `Core`, and every name is a capability.
fn grant(
    id: SourceId,
    written: &Spanned<String>,
    caps: Spanned<Vec<Spanned<String>>>,
) -> Result<Grant, Diagnostic> {
    let key = written.get_ref();
    let bare = key.strip_prefix('\\').unwrap_or(key);
    let (namespace, subtree) = match bare.strip_suffix("\\*") {
        Some(namespace) => (namespace, true),
        None => (bare, false),
    };
    if !namespace.split('\\').all(is_segment) {
        return Err(bad(
            id,
            written,
            format!("`{key}` is not a namespace"),
            "write a namespace such as `Acme\\Http`, or `Acme\\Http\\*` for it and every \
             namespace below it",
        ));
    }
    if namespace
        .split('\\')
        .next()
        .is_some_and(|first| first.eq_ignore_ascii_case(RESERVED))
    {
        let mut diagnostic = Diagnostic::error(
            code::E_GRANT_UNDER_CORE,
            format!("the grant table cannot name `{key}`"),
        )
        .with_note("`Core` is the standard library, and its namespace is reserved".to_owned())
        .with_help("delete this line".to_owned());
        if let Some(span) = span(id, written) {
            diagnostic = diagnostic.with_primary(span, "this key");
        }
        return Err(diagnostic);
    }
    let mut granted = Vec::new();
    for name in caps.get_ref() {
        let Some(cap) = Cap::parse(name.get_ref()) else {
            let roster: Vec<&str> = Cap::ALL.iter().map(|cap| cap.name()).collect();
            return Err(bad(
                id,
                name,
                format!("`{}` is not a capability", name.get_ref()),
                &format!("the capabilities are {}", roster.join(", ")),
            ));
        };
        granted.push(cap);
    }
    granted.sort_by_key(|cap| Cap::ALL.iter().position(|each| each == cap));
    granted.dedup();
    Ok(Grant {
        namespace: namespace.to_owned(),
        subtree,
        caps: granted,
    })
}

/// Whether `segment` is one segment of a namespace: a letter or `_`, then letters, digits and `_`.
fn is_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    chars
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// [`code::E_BAD_GRANT`] at `at`.
fn bad(id: SourceId, at: &Spanned<String>, message: String, help: &str) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(code::E_BAD_GRANT, message).with_help(help.to_owned());
    if let Some(span) = span(id, at) {
        diagnostic = diagnostic.with_primary(span, "here");
    }
    diagnostic
}

/// [`code::E_GRANT_KEYS_TIE`] for two spellings of one key.
fn tie(id: SourceId, first: &Spanned<String>, second: &Spanned<String>) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(
        code::E_GRANT_KEYS_TIE,
        format!(
            "`{}` and `{}` are the same key",
            first.get_ref(),
            second.get_ref()
        ),
    )
    .with_note("a `\\` at the start of a key does not change the key".to_owned())
    .with_help("keep one of the two lines".to_owned());
    if let Some(span) = span(id, second) {
        diagnostic = diagnostic.with_primary(span, "this key");
    }
    if let Some(span) = span(id, first) {
        diagnostic = diagnostic.with_secondary(span, "the same key");
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

/// A length as the eight bytes the digest feeds.
fn len(n: usize) -> [u8; 8] {
    u64::try_from(n).unwrap_or(u64::MAX).to_le_bytes()
}
