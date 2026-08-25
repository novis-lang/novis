//! The compile-time autoload map: which file declares a name
//! ([ADR 0061](../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
//! §§ 1-2).
//!
//! [`crate::requires`] owns the graph walk; this module owns the *map* it
//! consults. The two halves stay apart because they answer different
//! questions: `requires` asks "what does this file pull in", and everything
//! here asks "given a name nothing has declared, which file would declare
//! it".
//!
//! A [`Site`] is one `autoload` declaration, already cooked out of its
//! spans and paired with the directory of the file that wrote it — ADR 0061
//! § 1's "paths are relative to THIS file, never to the entry point".
//! [`AutoloadMap::build`] turns a program's sites into prefix → roots, and
//! [`AutoloadMap::resolve`] turns a [`QName`] into the file that declares it,
//! probing each root in declaration order and returning the whole ordered
//! trace — misses included — because ADR 0061 § 5 keys the artifact cache on
//! it.
//!
//! Three rules of § 1 live here rather than in the resolver:
//!
//! - **An explicit prefix beats a `discover` glob** producing the same
//!   prefix; the glob skips that name. Every *other* duplicate is
//!   [`code::E_DUPLICATE_AUTOLOAD_PREFIX`]. So [`AutoloadMap::build`] takes
//!   every site at once rather than accepting them one at a time: the
//!   explicit set has to be complete before a glob can know what to skip,
//!   and that is exactly what makes the map order-independent.
//! - **A discovered directory whose name is not a legal namespace segment is
//!   skipped, not diagnosed.** A glob over a filesystem inevitably sweeps
//!   `.git`, `vendor` and `node_modules`; diagnosing them would make the form
//!   unusable. A *malformed glob* — no `*`, more than one, or one sharing a
//!   segment with other text — is [`code::E_AUTOLOAD_GLOB_SHAPE`], and so is
//!   one whose base directory does not exist, since a typo that silently
//!   autoloads nothing is the worst outcome available.
//! - **The on-disk entry's name is compared exactly.** A case-insensitive
//!   filesystem happily opens `mailer.mwl` for `Mailer`; that is a *miss*
//!   here rather than a diagnostic, because nothing in the source spelled a
//!   path to blame — the file is simply not the one the name asks for, and on
//!   Linux it would not have been found at all
//!   ([ADR 0062](../../../docs/adr/0062-case-sensitivity-is-a-compiler-property.md)
//!   § 3).
//!
//! Path traversal is structurally impossible with no sanitizer, per § 1: a
//! probed suffix is built only out of namespace segments, and
//! [ADR 0029](../../../docs/adr/0029-identifier-casing-is-checked.md) leaves
//! no way to spell `.`, `..` or a separator in one.

use std::path::{Path, PathBuf};

use mwl_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use mwl_syntax::ast::{NamespaceDecl, Stmt, StmtKind};

use crate::qname::QName;

/// The file extension every autoloaded declaration lives in.
const SOURCE_EXTENSION: &str = "mwl";

/// One `autoload` declaration, cooked out of its
/// [`mwl_syntax::ast::AutoloadDecl`] spans by [`crate::requires`].
#[derive(Clone, Debug)]
pub struct Site {
    /// The directory of the file that wrote the declaration; every path in
    /// it resolves against this.
    pub base_dir: PathBuf,
    /// Which form was written.
    pub kind: SiteKind,
    /// The whole declaration, for a diagnostic to point at.
    pub span: Span,
}

/// The two forms a [`Site`] takes, with every literal already decoded.
#[derive(Clone, Debug)]
pub enum SiteKind {
    /// `autoload 'Prefix' from 'root', ...;`
    Prefix {
        /// The namespace prefix, written without a trailing separator.
        prefix: String,
        /// The roots, in declaration order.
        roots: Vec<String>,
    },
    /// `autoload discover '<glob>';`
    Discover {
        /// The glob, containing exactly one `*` occupying a whole segment.
        glob: String,
    },
}

/// One prefix and the roots it is probed against, in declaration order.
#[derive(Clone, Debug)]
struct Entry {
    /// The prefix's segments — `Acme\Legacy` as `["Acme", "Legacy"]`.
    segments: Vec<String>,
    /// The roots, canonicalized where the filesystem allowed it.
    roots: Vec<PathBuf>,
    /// Where the declaration that introduced this prefix was written.
    span: Span,
    /// Whether it came from an explicit `'Prefix' from ...`, which is what
    /// decides whether a `discover` glob producing the same prefix skips the
    /// name or collides with it.
    explicit: bool,
}

/// Every `autoload` declaration in a program, resolved into prefix → roots.
#[derive(Clone, Debug, Default)]
pub struct AutoloadMap {
    entries: Vec<Entry>,
}

/// What one [`AutoloadMap::resolve`] call did: the file it landed on, if
/// any, and the ordered list of paths it probed to get there.
#[derive(Clone, Debug, Default)]
pub struct Probe {
    /// The file that declares the name, canonicalized.
    pub hit: Option<PathBuf>,
    /// Every path probed, in order, *including* the misses — ADR 0061 § 5's
    /// shadowing edge: adding `src/Thing.mwl` where `App\Thing` currently
    /// resolves to `vendor/compat/Thing.mwl` changes the answer without
    /// touching a file anything already hashed.
    pub tried: Vec<PathBuf>,
}

impl AutoloadMap {
    /// Builds the map out of every declaration in the program's `require`
    /// chain — the union ADR 0061 § 1 describes, with its duplicate rule
    /// applied. Explicit prefixes are taken first so a `discover` glob knows
    /// which names to skip, which is what makes the result independent of the
    /// order the files were walked in.
    #[must_use]
    pub fn build(sites: &[Site], diags: &mut Diagnostics) -> Self {
        let mut map = Self::default();

        for site in sites {
            let SiteKind::Prefix { prefix, roots } = &site.kind else {
                continue;
            };
            let segments = QName::parse(prefix).segments().to_vec();
            if let Some(previous) = map.entry(&segments) {
                report_duplicate(prefix, previous.span, site.span, diags);
                continue;
            }
            map.entries.push(Entry {
                segments,
                roots: roots.iter().map(|r| canonical(&site.base_dir, r)).collect(),
                span: site.span,
                explicit: true,
            });
        }

        for site in sites {
            let SiteKind::Discover { glob } = &site.kind else {
                continue;
            };
            for (name, root) in discover(&site.base_dir, glob, site.span, diags) {
                let segments = vec![name];
                match map.entry(&segments).map(|e| (e.span, e.explicit)) {
                    // § 1: an explicit prefix beats a glob that would produce
                    // the same one, and the glob skips the name rather than
                    // colliding with it — the rule a vendor override rides on.
                    Some((_, true)) => {}
                    Some((first, false)) => {
                        report_duplicate(&segments.join("\\"), first, site.span, diags);
                    }
                    None => map.entries.push(Entry {
                        segments,
                        roots: vec![root],
                        span: site.span,
                        explicit: false,
                    }),
                }
            }
        }

        map
    }

    /// Whether any declaration was collected at all — the fast exit for a
    /// program that never wrote one, which is every program until it needs a
    /// framework.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The file that declares `name`, and every path probed on the way to it.
    ///
    /// Longest matching prefix wins; within a prefix, roots are probed in
    /// declaration order and the first hit wins (ADR 0061 § 1's Composer
    /// rule). The remaining segments are directories and the last is the file
    /// name plus `.mwl`, compared to the on-disk entry exactly.
    #[must_use]
    pub fn resolve(&self, name: &QName) -> Probe {
        let mut probe = Probe::default();
        let segments = name.segments();

        let mut best: Option<&Entry> = None;
        for entry in &self.entries {
            if entry.segments.len() >= segments.len() {
                continue;
            }
            if !segments.starts_with(&entry.segments) {
                continue;
            }
            if best.is_none_or(|b| b.segments.len() < entry.segments.len()) {
                best = Some(entry);
            }
        }
        let Some(entry) = best else {
            return probe;
        };

        let suffix = &segments[entry.segments.len()..];
        for root in &entry.roots {
            let mut candidate = root.clone();
            for segment in &suffix[..suffix.len() - 1] {
                candidate.push(segment);
            }
            candidate.push(format!("{}.{SOURCE_EXTENSION}", suffix[suffix.len() - 1]));
            probe.tried.push(candidate.clone());

            let Ok(canonical) = candidate.canonicalize() else {
                continue;
            };
            if !spelled_exactly(&canonical, root, suffix) {
                continue;
            }
            probe.hit = Some(canonical);
            return probe;
        }

        probe
    }

    fn entry(&self, segments: &[String]) -> Option<&Entry> {
        self.entries.iter().find(|e| e.segments == segments)
    }
}

fn report_duplicate(prefix: &str, first: Span, second: Span, diags: &mut Diagnostics) {
    diags.report(
        Diagnostic::error(
            code::E_DUPLICATE_AUTOLOAD_PREFIX,
            format!("`{prefix}` is already autoloaded from somewhere else"),
        )
        .with_primary(second, "declared again here")
        .with_secondary(first, "first declared here")
        .with_note("one prefix has one home (ADR 0061 § 1)"),
    );
}

/// Resolves one written root against the declaring file's directory,
/// canonicalizing where the filesystem allows it so a probe's on-disk
/// spelling check has a canonical base to line up against. A root that does
/// not exist stays as-written: every probe under it then simply misses, which
/// is the same answer with no extra diagnostic for a deployment that ships
/// only some of its modules.
fn canonical(base_dir: &Path, root: &str) -> PathBuf {
    let joined = base_dir.join(root);
    joined.canonicalize().unwrap_or(joined)
}

/// Expands `autoload discover '<glob>'` into its `(prefix, root)` pairs.
fn discover(
    base_dir: &Path,
    glob: &str,
    span: Span,
    diags: &mut Diagnostics,
) -> Vec<(String, PathBuf)> {
    let parts: Vec<&str> = glob.split(['/', '\\']).collect();
    let stars = parts.iter().filter(|p| p.contains('*')).count();
    let Some(star) = parts.iter().position(|p| *p == "*") else {
        diags.report(
            Diagnostic::error(
                code::E_AUTOLOAD_GLOB_SHAPE,
                format!("`{glob}` is not a discovery glob"),
            )
            .with_primary(span, "no `*` occupying a whole path segment")
            .with_note("a `discover` glob holds exactly one `*`, and it is a whole segment"),
        );
        return Vec::new();
    };
    if stars != 1 {
        diags.report(
            Diagnostic::error(
                code::E_AUTOLOAD_GLOB_SHAPE,
                format!("`{glob}` holds more than one `*`"),
            )
            .with_primary(span, "only one segment may be matched")
            .with_note("a `discover` glob holds exactly one `*`, and it is a whole segment"),
        );
        return Vec::new();
    }

    let mut scanned = base_dir.to_path_buf();
    for part in &parts[..star] {
        scanned.push(part);
    }
    let Ok(listing) = std::fs::read_dir(&scanned) else {
        diags.report(
            Diagnostic::error(
                code::E_AUTOLOAD_GLOB_SHAPE,
                format!("`{}` cannot be scanned", scanned.display()),
            )
            .with_primary(span, "no directory to discover modules in"),
        );
        return Vec::new();
    };

    let mut found: Vec<(String, PathBuf)> = Vec::new();
    for entry in listing.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // § 1: a directory whose name is not a legal namespace segment is
        // skipped in silence. `.git` and `vendor` are always there.
        if !is_namespace_segment(&name) {
            continue;
        }
        let mut root = entry.path();
        for part in &parts[star + 1..] {
            root.push(part);
        }
        if !root.is_dir() {
            continue;
        }
        found.push((name, root.canonicalize().unwrap_or(root)));
    }
    // `read_dir` yields in whatever order the filesystem hands back, and the
    // duplicate diagnostic below has to name the same two sites on every
    // machine.
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

/// ADR 0029's namespace-segment shape: `PascalCase`, ASCII alphanumeric, and
/// never a leading `_` ([ADR 0030](../../../docs/adr/0030-no-leading-underscores-constructor-spelling.md)).
fn is_namespace_segment(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase()) && chars.all(|c| c.is_ascii_alphanumeric())
}

/// Whether the entry `canonicalize` landed on is spelled the way the name
/// asked for it, component by component.
///
/// `root` is canonical, so the trailing components of `canonical` line up
/// positionally against `suffix` — unless a symlink was crossed, which shows
/// up as a length mismatch and skips the check rather than guessing, exactly
/// as [`crate::requires::check_path_case`] does for a `require`.
fn spelled_exactly(canonical: &Path, root: &Path, suffix: &[String]) -> bool {
    let actual: Vec<String> = canonical
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let root_len = root.components().count();
    if actual.len() != root_len + suffix.len() {
        return true;
    }
    let last = suffix.len() - 1;
    for (i, written) in suffix.iter().enumerate() {
        let expected = if i == last {
            format!("{written}.{SOURCE_EXTENSION}")
        } else {
            written.clone()
        };
        if expected != actual[root_len + i] {
            return false;
        }
    }
    true
}

/// Applies ADR 0061 § 2 to a file that was reached through the autoload map:
/// exactly one top-level declaration, named after the file, and nothing else
/// but `namespace`/`use`.
///
/// `expected` is the file's base name. Files reached by `require` are
/// deliberately not subject to any of this and may declare anything.
pub fn check_file_shape(
    stmts: &[Stmt],
    src: &SourceFile,
    expected: &str,
    span: Span,
    diags: &mut Diagnostics,
) {
    let mut declared: Vec<(String, Span)> = Vec::new();
    let mut stray: Option<Span> = None;
    scan_shape(stmts, src, &mut declared, &mut stray);

    let complaint = if declared.is_empty() {
        Some((span, format!("nothing here declares `{expected}`")))
    } else if declared.len() > 1 {
        Some((declared[1].1, "a second declaration".to_owned()))
    } else if let Some(at) = stray {
        Some((
            at,
            "only declarations belong in an autoloaded file".to_owned(),
        ))
    } else if declared[0].0 == expected {
        None
    } else {
        Some((
            declared[0].1,
            format!(
                "this declares `{}`, but the file is named `{expected}`",
                declared[0].0
            ),
        ))
    };

    if let Some((at, message)) = complaint {
        diags.report(
            Diagnostic::error(
                code::E_AUTOLOAD_FILE_SHAPE,
                format!("an autoloaded file declares exactly one thing, named `{expected}`"),
            )
            .with_primary(at, message)
            .with_note(
                "ADR 0061 § 2: otherwise whether a name exists depends on what was resolved first",
            ),
        );
    }
}

fn scan_shape(
    stmts: &[Stmt],
    src: &SourceFile,
    declared: &mut Vec<(String, Span)>,
    stray: &mut Option<Span>,
) {
    for stmt in stmts {
        let span = match &stmt.kind {
            StmtKind::ClassDecl(d) => d.name.span,
            StmtKind::InterfaceDecl(d) => d.name.span,
            StmtKind::EnumDecl(d) => d.name.span,
            StmtKind::TypeAliasDecl(d) => d.name.span,
            StmtKind::NamespaceDecl(NamespaceDecl { body, .. }) => {
                if let Some(block) = body {
                    scan_shape(&block.stmts, src, declared, stray);
                }
                continue;
            }
            // `use` carries no declaration and an `autoload` here is already
            // `E_AUTOLOAD_IN_AUTOLOADED_FILE`, reported with its own span.
            StmtKind::UseDecl(_) | StmtKind::AutoloadDecl(_) | StmtKind::Error => continue,
            _ => {
                stray.get_or_insert(stmt.span);
                continue;
            }
        };
        let text = src.span_text(span).unwrap_or_default().to_owned();
        declared.push((text, span));
    }
}

/// Reports every `autoload` declaration in a file that was itself reached
/// through the map — ADR 0061 § 1, which honors a declaration only where the
/// entry point's `require` chain can see it. The rule is applied to the whole
/// autoloaded sub-graph, not only to the autoloaded file itself: a map that
/// grows as it is consulted is the self-dependence the rule exists to stop.
pub fn reject_declarations(sites: &[Site], diags: &mut Diagnostics) {
    for site in sites {
        diags.report(
            Diagnostic::error(
                code::E_AUTOLOAD_IN_AUTOLOADED_FILE,
                "`autoload` is not honored in a file reached by autoload",
            )
            .with_primary(
                site.span,
                "this declaration would extend the map that found it",
            )
            .with_note("ADR 0061 § 1: write it in a file the entry point `require`s"),
        );
    }
}
