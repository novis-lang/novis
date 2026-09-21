//! The compile-time autoload map: which file declares a name
//! (`rule:programs/autoload` and `rule:programs/one-declaration-per-autoloaded-file`).
//!
//! [`crate::requires`] owns the graph walk; this module owns the *map* it
//! consults. The two halves stay apart because they answer different
//! questions: `requires` asks "what does this file pull in", and everything
//! here asks "given a name nothing has declared, which file would declare
//! it".
//!
//! A [`Site`] is one `autoload` declaration, already cooked out of its
//! spans and paired with the directory of the file that wrote it — `rule:programs/autoload`'s "paths are relative to THIS file, never to the entry point".
//! [`AutoloadMap::build`] turns a program's sites into prefix → roots, and
//! [`AutoloadMap::resolve`] turns a [`QName`] into the file that declares it,
//! probing each root in declaration order and returning the whole ordered
//! trace — misses included — because `rule:packaging/autoload-probes-fold-into-the-cache-key` keys the artifact cache on
//! it. [`AutoloadMap::enumerate`] is that map read the other way, listing
//! every name the roots declare: § 3's scan, which only a program calling
//! `Core\Program::implementing<T>()` or writing a `#[Route]` ever pays for.
//!
//! These rules of § 1 live here rather than in the resolver:
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
//!   filesystem happily opens `mailer.nvs` for `Mailer`; that is a *miss*
//!   here rather than a diagnostic, because nothing in the source spelled a
//!   path to blame — the file is simply not the one the name asks for, and on
//!   Linux it would not have been found at all
//!   ([ADR 0062](/docs/decisions/0062.md)
//!   § 3).
//!
//! `nvs check --autoload-map` prints the result, which is why what a
//! `discover` glob does *quietly* — passing over a directory that cannot
//! name a namespace, and producing a prefix an explicit declaration already
//! owns — is kept on the map rather than dropped where it happens.
//! [`AutoloadMap::render`] is that printer. Its shape is a counted section
//! per kind, one line per prefix with its roots in probe order:
//!
//! ```text
//! prefixes (2)
//!   App       explicit  override
//!   Plugin    discover  Plugin/src
//! shadowed (1)
//!   App       discover  App/src
//! skipped (1)
//!   vendor    not a PascalCase namespace segment
//! ```
//!
//! A count sits on every header, so an empty section still says so — the
//! answer someone reaching for the flag is usually after, since a glob that
//! discovers nothing is § 1's worst outcome and looks exactly like a glob
//! nobody wrote. Paths are printed relative to the directory `render` is
//! given and with `/` separators, so both legs of the test suite render one
//! string.
//!
//! A bundled executable has no directories to probe, so every path this module
//! touches goes through [`crate::requires::canonicalize`] or [`listing`], which
//! answer out of `nvs_diagnostics::embedded`'s payload when there is one and out
//! of the filesystem otherwise. The payload carries every file the roots
//! declared at build time, which is what makes a bundled program resolve the
//! names its source tree resolved.
//!
//! Path traversal is structurally impossible with no sanitizer, per § 1: a
//! probed suffix is built only out of namespace segments, and
//! `rule:core-api/identifier-casing` leaves
//! no way to spell `.`, `..` or a separator in one.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, Span, code};
use nvs_syntax::ast::{NamespaceDecl, Stmt, StmtKind};
use rustc_hash::FxHashSet;

use crate::qname::QName;

/// The file extension every autoloaded declaration lives in.
const SOURCE_EXTENSION: &str = "nvs";

/// One `autoload` declaration, cooked out of its
/// [`nvs_syntax::ast::AutoloadDecl`] spans by [`crate::requires`].
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

/// The forms a [`Site`] takes, with every literal already decoded.
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
    /// Whether another program's declaration introduced it
    /// ([`AutoloadMap::build_borrowing`]). Resolution reads a borrowed entry
    /// like any other; [`AutoloadMap::claims`] answers for this program's own.
    borrowed: bool,
}

/// Every `autoload` declaration in a program, resolved into prefix → roots.
#[derive(Clone, Debug, Default)]
pub struct AutoloadMap {
    entries: Vec<Entry>,
    /// The declarations this program wrote, as [`Self::build`] was handed
    /// them, kept so an editor can lend them to the analysis of a file this
    /// program autoloads ([`Self::sites`]). One [`Site`] per `autoload`
    /// statement in the bootstrap chain, released with the map.
    sites: Vec<Site>,
    /// Every directory a `discover` glob matched and did not turn into a
    /// prefix, with the reason, kept only so [`Self::render`] can print it:
    /// resolution never consults this.
    skipped: Vec<(PathBuf, &'static str)>,
    /// Every prefix a `discover` glob produced that an explicit declaration
    /// already owned, with the root the glob would have given it — the
    /// vendor-override rule, made visible.
    shadowed: Vec<(String, PathBuf)>,
    /// What *this program's* resolution probed, as opposed to what the
    /// declarations above say it could: written only by
    /// [`Self::resolve_recording`].
    trace: ProbeTrace,
}

/// Every path a program's `autoload` resolution probed, in the order it probed
/// them and with the misses kept — `rule:packaging/autoload-probes-fold-into-the-cache-key`'s datum.
///
/// **The misses are the point.** A file nobody references changes nothing, and
/// when a reference is finally written it is the *referencing* file whose
/// content hash moves, so an artifact cache keyed on the files it compiled
/// already notices that on its own. What it cannot notice is shadowing:
/// writing `src/Thing.nvs` where `App\Thing` currently resolves to
/// `vendor/compat/Thing.nvs` changes the answer without touching a byte of
/// anything the first compile hashed. The probed-and-missed path is the only
/// record that the answer was ever a question.
///
/// What it holds: one [`PathBuf`] per root probed per autoloaded name —
/// O(names × roots) for the length of one resolution, released with the map
/// (`rule:programs/memory-priority`).
#[derive(Clone, Debug, Default)]
pub struct ProbeTrace {
    probed: Vec<PathBuf>,
}

impl ProbeTrace {
    /// The trace, in probe order, misses included.
    ///
    /// A name appears at most once: `crate::requires`' walk refuses a second
    /// probe of a name it has already asked about, which is what bounds the
    /// length and what makes the order a function of the program rather than of
    /// how many times something asked.
    #[must_use]
    pub fn probed(&self) -> &[PathBuf] {
        &self.probed
    }
}

/// What one [`AutoloadMap::resolve`] call did: the file it landed on, if
/// any, and the ordered list of paths it probed to get there.
#[derive(Clone, Debug, Default)]
pub struct Probe {
    /// The file that declares the name, canonicalized.
    pub hit: Option<PathBuf>,
    /// Every path probed, in order, *including* the misses — `rule:packaging/autoload-probes-fold-into-the-cache-key`'s
    /// shadowing edge: adding `src/Thing.nvs` where `App\Thing` currently
    /// resolves to `vendor/compat/Thing.nvs` changes the answer without
    /// touching a file anything already hashed.
    pub tried: Vec<PathBuf>,
}

impl AutoloadMap {
    /// Builds the map out of every declaration in the program's `require`
    /// chain — the union `rule:programs/autoload` describes, with its duplicate rule
    /// applied. Explicit prefixes are taken first so a `discover` glob knows
    /// which names to skip, which is what makes the result independent of the
    /// order the files were walked in.
    #[must_use]
    pub fn build(sites: &[Site], diags: &mut Diagnostics) -> Self {
        Self::build_borrowing(sites, &[], diags)
    }

    /// [`Self::build`], with another program's declarations behind this one's.
    ///
    /// `borrowed` is what an editor passes when the file it analyses is one
    /// another program autoloads. Such a file writes no `autoload` of its own
    /// — `rule:programs/autoload` forbids it one — so the names it uses
    /// resolve through the map of the program that reaches it.
    ///
    /// Every site in `own` is taken before any in `borrowed`, in both passes,
    /// so a prefix both declare is `own`'s and no diagnostic ever names a
    /// borrowed site as the earlier of two. A borrowed site reports nothing at
    /// all, a duplicate and a malformed glob included: its span is a position
    /// in a file this program never loaded, and the program that wrote it is
    /// where those are already reported.
    #[must_use]
    pub fn build_borrowing(own: &[Site], borrowed: &[Site], diags: &mut Diagnostics) -> Self {
        let mut map = Self {
            sites: own.to_vec(),
            ..Self::default()
        };
        let mut unreported = Diagnostics::new();
        let ordered: Vec<(&Site, bool)> = own
            .iter()
            .map(|site| (site, false))
            .chain(borrowed.iter().map(|site| (site, true)))
            .collect();

        for &(site, is_borrowed) in &ordered {
            let SiteKind::Prefix { prefix, roots } = &site.kind else {
                continue;
            };
            let segments = QName::parse(prefix).segments().to_vec();
            if let Some(previous) = map.entry(&segments) {
                if !is_borrowed {
                    report_duplicate(prefix, previous.span, site.span, diags);
                }
                continue;
            }
            map.entries.push(Entry {
                segments,
                roots: roots.iter().map(|r| canonical(&site.base_dir, r)).collect(),
                span: site.span,
                explicit: true,
                borrowed: is_borrowed,
            });
        }

        for &(site, is_borrowed) in &ordered {
            let SiteKind::Discover { glob } = &site.kind else {
                continue;
            };
            let sink = if is_borrowed {
                &mut unreported
            } else {
                &mut *diags
            };
            let expanded = discover(&site.base_dir, glob, site.span, sink);
            if !is_borrowed {
                map.skipped.extend(expanded.skipped);
            }
            for (name, root) in expanded.found {
                let segments = vec![name.clone()];
                match map.entry(&segments).map(|e| (e.span, e.explicit)) {
                    // A borrowed glob that repeats a prefix already taken adds
                    // nothing and says nothing.
                    Some(_) if is_borrowed => {}
                    // § 1: an explicit prefix beats a glob that would produce
                    // the same one, and the glob skips the name rather than
                    // colliding with it — the rule a vendor override rides on.
                    Some((_, true)) => map.shadowed.push((name, root)),
                    Some((first, false)) => {
                        report_duplicate(&name, first, site.span, diags);
                    }
                    None => map.entries.push(Entry {
                        segments,
                        roots: vec![root],
                        span: site.span,
                        explicit: false,
                        borrowed: is_borrowed,
                    }),
                }
            }
        }

        map
    }

    /// The declarations this program wrote, in the order the walk met them —
    /// what another analysis hands [`Self::build_borrowing`] as `borrowed`.
    #[must_use]
    pub fn sites(&self) -> &[Site] {
        &self.sites
    }

    /// Whether `path` is under a root one of this program's own declarations
    /// names, which makes it a file this program autoloads.
    ///
    /// A borrowed entry does not count: it says which program a *name*
    /// resolves through, and this asks which program a *file* belongs to.
    #[must_use]
    pub fn claims(&self, path: &Path) -> bool {
        let path = crate::requires::canonicalize(path).unwrap_or_else(|| path.to_path_buf());
        self.entries
            .iter()
            .filter(|entry| !entry.borrowed)
            .flat_map(|entry| &entry.roots)
            .any(|root| path.starts_with(root))
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
    /// declaration order and the first hit wins (`rule:programs/autoload`'s Composer
    /// rule). The remaining segments are directories and the last is the file
    /// name plus `.nvs`, compared to the on-disk entry exactly.
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

            let Some(canonical) = crate::requires::canonicalize(&candidate) else {
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

    /// [`Self::resolve`], keeping what it probed in this map's [`ProbeTrace`].
    ///
    /// The graph walk resolves through this one; every other caller through the
    /// plain one. `rule:packaging/autoload-probes-fold-into-the-cache-key` keys a unit on what the
    /// *program's own* resolution probed, so a caller merely asking the map a
    /// question — `nvs check --autoload-map`, an editor resolving a name under
    /// the cursor — must not be able to lengthen a trace a cache key is
    /// computed from. Splitting the two is what makes that structural rather
    /// than a note telling each caller to be careful.
    pub fn resolve_recording(&mut self, name: &QName) -> Probe {
        let probe = self.resolve(name);
        self.trace.probed.extend(probe.tried.iter().cloned());
        probe
    }

    /// What this program's resolution probed, handed back with the map rather
    /// than dropped — the trace `rule:packaging/autoload-probes-fold-into-the-cache-key` folds into the
    /// unit's cache key.
    #[must_use]
    pub fn probe_trace(&self) -> &ProbeTrace {
        &self.trace
    }

    /// Every name the roots declare, sorted by fully-qualified name —
    /// [`Self::resolve`] run in the other direction, and the one place
    /// resolution is not lazy (`rule:programs/implementing`).
    ///
    /// `Core\Program::implementing<T>()` and `rule:routing/routes-are-compiled-not-registered`'s compile-time route
    /// table are its only callers, under § 3's opt-in rule: a program
    /// writing neither never calls this and never pays the directory
    /// listing. What comes back is the *file* set. Which of those files
    /// declares a class satisfying anything is
    /// [`crate::hierarchy::implementors`]' question, over the graph built
    /// once `crate::requires` has loaded them — this half knows nothing
    /// about what a file contains, only that a file at this path is where a
    /// declaration of this name would live, which is the same rule
    /// [`Self::resolve`] applies from the other end.
    ///
    /// A name declared under two roots of one prefix is listed once, at the
    /// root [`Self::resolve`] would have probed first, so the two directions
    /// cannot disagree about which file declares a name. A directory or file
    /// stem that cannot name a namespace segment is passed over in silence,
    /// for the reason § 1 gives for a `discover` glob: a root inevitably
    /// holds a `.git`, a `README.md` and a `vendor`.
    #[must_use]
    pub fn enumerate(&self) -> Vec<(QName, PathBuf)> {
        let mut found: Vec<(QName, PathBuf)> = Vec::new();
        let mut walked: FxHashSet<PathBuf> = FxHashSet::default();
        for entry in &self.entries {
            for root in &entry.roots {
                collect_declared(root, &entry.segments, &mut found, &mut walked);
            }
        }
        // Stable, so a name found under two roots keeps the first probe's
        // file — `resolve`'s own first-hit-wins rule, arrived at from the
        // other side. `dedup_by` drops the later of each run, which is the
        // same choice.
        found.sort_by(|a, b| a.0.segments().cmp(b.0.segments()));
        found.dedup_by(|a, b| a.0 == b.0);
        found
    }

    /// Renders the resolved map for `nvs check --autoload-map` — `rule:programs/autoload`'s last sentence, which asks for what was *skipped* and what was
    /// *shadowed* beside the prefixes that resolve.
    ///
    /// Paths are shown relative to `base` where they sit under it. The module
    /// docs hold the shape, and why every header carries a count.
    #[must_use]
    pub fn render(&self, base: &Path) -> String {
        let base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());

        let mut entries: Vec<&Entry> = self.entries.iter().collect();
        entries.sort_by(|a, b| a.segments.cmp(&b.segments));
        let mut shadowed = self.shadowed.clone();
        shadowed.sort_by(|a, b| a.0.cmp(&b.0));
        let skipped: Vec<(String, &str)> = self
            .skipped
            .iter()
            .map(|(path, reason)| (show(path, &base), *reason))
            .collect();

        let width = entries
            .iter()
            .map(|entry| entry.segments.join("\\").chars().count())
            .chain(shadowed.iter().map(|(prefix, _)| prefix.chars().count()))
            .max()
            .unwrap_or(0);

        let mut out = String::new();
        let _ = writeln!(out, "prefixes ({})", entries.len());
        for entry in &entries {
            let roots: Vec<String> = entry.roots.iter().map(|root| show(root, &base)).collect();
            let _ = writeln!(
                out,
                "  {:width$}  {}  {}",
                entry.segments.join("\\"),
                if entry.explicit {
                    "explicit"
                } else {
                    "discover"
                },
                roots.join(", "),
            );
        }

        let _ = writeln!(out, "shadowed ({})", shadowed.len());
        for (prefix, root) in &shadowed {
            let _ = writeln!(out, "  {prefix:width$}  discover  {}", show(root, &base));
        }

        let _ = writeln!(out, "skipped ({})", skipped.len());
        let skip_width = skipped
            .iter()
            .map(|(path, _)| path.chars().count())
            .max()
            .unwrap_or(0);
        for (path, reason) in &skipped {
            let _ = writeln!(out, "  {path:skip_width$}  {reason}");
        }

        out
    }

    fn entry(&self, segments: &[String]) -> Option<&Entry> {
        self.entries.iter().find(|e| e.segments == segments)
    }
}

/// Appends every `.nvs` file under `dir` to `out`, named under `prefix`,
/// recursing into each subdirectory that can name a namespace segment —
/// [`AutoloadMap::enumerate`]'s whole mechanism, and the mirror of the
/// suffix `AutoloadMap::resolve` builds a candidate path out of.
///
/// A path comes back canonicalized, matching [`Probe::hit`], because the
/// caller loading these files compares them against the ones the `require`
/// walk already canonicalized and a file loaded twice would declare
/// everything in it twice.
///
/// `walked` holds the directories already visited, so a symlink pointing
/// back up its own tree costs one skipped directory rather than an unbounded
/// walk. The root is a path the program wrote; everything under it is
/// whatever the deployment put there, which is not the same guarantee.
fn collect_declared(
    dir: &Path,
    prefix: &[String],
    out: &mut Vec<(QName, PathBuf)>,
    walked: &mut FxHashSet<PathBuf>,
) {
    if !walked.insert(crate::requires::canonicalize(dir).unwrap_or_else(|| dir.to_path_buf())) {
        return;
    }
    // A root that does not exist reads as a root declaring nothing, the same
    // answer `canonical` leaves a probe under it with.
    let Some(entries) = listing(dir) else {
        return;
    };

    for (path, is_directory) in entries {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if is_directory {
            if is_namespace_segment(&name) {
                let mut nested = prefix.to_vec();
                nested.push(name);
                collect_declared(&path, &nested, out, walked);
            }
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some(SOURCE_EXTENSION) {
            continue;
        }
        // The on-disk spelling is the name here, so there is nothing for
        // `spelled_exactly` to check: this direction reads the name off the
        // filesystem instead of asking the filesystem for one.
        let Some(stem) = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_owned)
        else {
            continue;
        };
        if !is_namespace_segment(&stem) {
            continue;
        }
        let canonical = crate::requires::canonicalize(&path).unwrap_or(path);
        out.push((QName::join(prefix, &stem), canonical));
    }
}

/// One directory's entries as this module reads them — `(path, whether it is a
/// directory)` — or `None` for a directory there is nothing to list.
///
/// Inside a bundle the payload *is* the world
/// (`rule:packaging/a-bundle-is-found-by-its-footer-before-argv-is-read`): it
/// carries every file the roots declared when it was built, so the walk over it
/// reaches the names the source tree reached and a real `read_dir` would reach
/// whatever the machine running the bundle happens to have. That is the same
/// split [`crate::requires::canonicalize`] makes for a single path, and the
/// reason both are one function rather than a call site each.
fn listing(dir: &Path) -> Option<Vec<(PathBuf, bool)>> {
    if nvs_diagnostics::embedded::is_active() {
        let entries = nvs_diagnostics::embedded::read_dir(dir)?;
        return Some(
            entries
                .iter()
                .map(|(name, is_directory)| (dir.join(name), *is_directory))
                .collect(),
        );
    }
    let entries = std::fs::read_dir(dir).ok()?;
    Some(
        entries
            .flatten()
            .map(|entry| {
                let is_directory = entry.file_type().is_ok_and(|kind| kind.is_dir());
                (entry.path(), is_directory)
            })
            .collect(),
    )
}

/// [`Path::is_dir`], answered out of the payload inside a bundle.
fn is_dir(path: &Path) -> bool {
    if nvs_diagnostics::embedded::is_active() {
        return nvs_diagnostics::embedded::is_dir(path);
    }
    path.is_dir()
}

/// One path the way [`AutoloadMap::render`] prints it: relative to `base`
/// where it sits under it, whole where it does not, and always `/`-separated
/// so the Windows and WSL legs render one string.
fn show(path: &Path, base: &Path) -> String {
    let shown = path.strip_prefix(base).unwrap_or(path);
    let text = shown.to_string_lossy().replace('\\', "/");
    if text.is_empty() {
        ".".to_owned()
    } else {
        text
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
        .with_note("one prefix has one home (`rule:programs/autoload`)"),
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
    crate::requires::canonicalize(&joined).unwrap_or(joined)
}

/// What one `autoload discover '<glob>'` expanded to.
///
/// The skipped half is not resolution's business — nothing probes it — but
/// § 1 promises `nvs check --autoload-map` prints it, and this is the only
/// place that knows why a matched directory produced no prefix.
#[derive(Debug, Default)]
struct Discovered {
    /// The `(prefix, root)` pairs the glob produced, sorted by prefix.
    found: Vec<(String, PathBuf)>,
    /// Each directory passed over in silence, with the reason, sorted by
    /// path.
    skipped: Vec<(PathBuf, &'static str)>,
}

/// Expands `autoload discover '<glob>'` into its `(prefix, root)` pairs.
fn discover(base_dir: &Path, glob: &str, span: Span, diags: &mut Diagnostics) -> Discovered {
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
        return Discovered::default();
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
        return Discovered::default();
    }

    let mut scanned = base_dir.to_path_buf();
    for part in &parts[..star] {
        scanned.push(part);
    }
    let Some(entries) = listing(&scanned) else {
        diags.report(
            Diagnostic::error(
                code::E_AUTOLOAD_GLOB_SHAPE,
                format!("`{}` cannot be scanned", scanned.display()),
            )
            .with_primary(span, "no directory to discover modules in"),
        );
        return Discovered::default();
    };

    let mut out = Discovered::default();
    for (path, is_directory) in entries {
        // The `*` matches a *directory*; a plain file sitting beside them can
        // never become a root — the `is_dir` test below already refused it —
        // and reporting one as skipped would fill `--autoload-map` with every
        // source file in the tree.
        if !is_directory {
            continue;
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        // Canonical from here down. `--autoload-map` prints every path
        // against a canonical base, and the tail pushed on below may not
        // exist to canonicalize on its own.
        let matched = crate::requires::canonicalize(&path).unwrap_or(path);
        // § 1: a directory whose name is not a legal namespace segment is
        // skipped in silence. `.git` and `vendor` are always there.
        if !is_namespace_segment(&name) {
            out.skipped
                .push((matched, "not a PascalCase namespace segment"));
            continue;
        }
        let mut root = matched;
        for part in &parts[star + 1..] {
            root.push(part);
        }
        if !is_dir(&root) {
            out.skipped
                .push((root, "the glob's remaining segments name no directory"));
            continue;
        }
        let root = crate::requires::canonicalize(&root).unwrap_or(root);
        out.found.push((name, root));
    }
    // `read_dir` yields in whatever order the filesystem hands back, and the
    // duplicate diagnostic below has to name the same two sites on every
    // machine — as does anything `--autoload-map` prints.
    out.found.sort_by(|a, b| a.0.cmp(&b.0));
    out.skipped.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// `rule:core-api/identifier-casing`'s namespace-segment shape: `PascalCase`, ASCII alphanumeric, and
/// never a leading `_` (`rule:classes/no-leading-underscore-identifiers`).
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

/// Applies `rule:programs/one-declaration-per-autoloaded-file` to a file that was reached through the autoload map:
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
                "`rule:programs/one-declaration-per-autoloaded-file`: otherwise whether a name exists depends on what was resolved first",
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
/// through the map — `rule:programs/autoload`, which honors a declaration only where the
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
            .with_note("`rule:programs/autoload`: write it in a file the entry point `require`s"),
        );
    }
}
