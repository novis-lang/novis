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
//! owns — is kept on the map rather than dropped where it happens. So is a
//! root that does not exist: it is allowed, because a deployment leaves a
//! module out by not shipping its directory, and it is listed, because a
//! typo in a root looks the same. [`AutoloadMap::render`] is that printer.
//! Its shape is a counted section per kind, one line per prefix with its
//! roots in probe order:
//!
//! ```text
//! prefixes (3)
//!   App       explicit  override
//!   Billing   explicit  billing/src
//!   Plugin    discover  Plugin/src
//! shadowed (1)
//!   App       discover  App/src
//! skipped (1)
//!   vendor    not a PascalCase namespace segment
//! missing (1)
//!   Billing   billing/src
//! ```
//!
//! A prefix is checked where it is built. Each segment is a namespace
//! segment or one `{..}`, which is replaced by the name of the directory its
//! `.` and `..` steps reach from the declaring file; anything else is
//! [`code::E_AUTOLOAD_PREFIX_SHAPE`], since such a prefix can never match.
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

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, SourceMap, Span, code};
use nvs_syntax::ast::{NamespaceDecl, Stmt, StmtKind};
use rustc_hash::FxHashSet;

use crate::hierarchy::{ClassLinks, CoreRoster, HierarchyResolver};
use crate::qname::QName;
use crate::symbol::{SymbolKind, SymbolTable};

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
        /// The prefix's literal, quotes included.
        literal: Span,
        /// The roots, in declaration order, each with the span of the literal
        /// that wrote it, quotes included.
        roots: Vec<(String, Span)>,
    },
    /// `autoload discover '<glob>';`
    Discover {
        /// The glob, containing exactly one `*` occupying a whole segment.
        glob: String,
        /// The glob's literal, quotes included.
        literal: Span,
    },
}

/// One root of a prefix declaration, as [`Site::roots`] resolves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Root {
    /// The root's literal, quotes included.
    pub literal: Span,
    /// The directory a name under the prefix is probed in.
    pub dir: PathBuf,
    /// [`dir`](Self::dir) the way `nvs check --autoload-map` prints it:
    /// relative to the declaring file's directory, `/`-separated.
    pub shown: String,
    /// Whether [`dir`](Self::dir) exists.
    pub exists: bool,
}

impl Site {
    /// Each literal this declaration wrote, paired with the directory it
    /// names: a root, the directory a `discover` glob lists, which is the part
    /// before its `*` segment, and for the prefix the first of its roots that
    /// exists, which is the first directory a name under it is probed in.
    ///
    /// The paths come from the functions [`AutoloadMap::build`] resolves with,
    /// [`canonical`] and [`glob_base`], so an editor's link and the map can
    /// never name two directories for one literal. A literal that names no
    /// directory is left out. A missing root is allowed, and `nvs check
    /// --autoload-map` lists it under `missing`. A glob of the wrong shape is
    /// already [`code::E_AUTOLOAD_GLOB_SHAPE`].
    ///
    /// Nothing calls this while compiling: it costs one existence test per
    /// literal, paid by the caller that asks.
    #[must_use]
    pub fn directories(&self) -> Vec<(Span, PathBuf)> {
        match &self.kind {
            SiteKind::Prefix { literal, .. } => {
                let roots: Vec<(Span, PathBuf)> = self
                    .roots()
                    .into_iter()
                    .filter_map(|root| root.exists.then_some((root.literal, root.dir)))
                    .collect();
                let first = roots.first().map(|(_, dir)| (*literal, dir.clone()));
                first.into_iter().chain(roots).collect()
            }
            SiteKind::Discover { glob, literal } => {
                let parts = glob_parts(glob);
                star_segment(&parts)
                    .ok()
                    .map(|star| glob_base(&self.base_dir, &parts, star))
                    .and_then(|base| crate::requires::canonicalize(&base))
                    .filter(|base| is_dir(base))
                    .map(|base| vec![(*literal, base)])
                    .unwrap_or_default()
            }
        }
    }

    /// Each root of a prefix declaration, in probe order, as
    /// [`AutoloadMap::build`] resolves it. Empty for a `discover` glob.
    ///
    /// A missing root is kept, because an editor shows it beside the roots
    /// that exist, the way `nvs check --autoload-map` lists it under
    /// `missing`. Like [`Self::directories`], nothing calls this while
    /// compiling.
    #[must_use]
    pub fn roots(&self) -> Vec<Root> {
        let SiteKind::Prefix { roots, .. } = &self.kind else {
            return Vec::new();
        };
        nvs_footprint::exists(&self.base_dir);
        let base = self
            .base_dir
            .canonicalize()
            .unwrap_or_else(|_| self.base_dir.clone());
        roots
            .iter()
            .map(|(root, literal)| {
                let dir = canonical(&self.base_dir, root);
                Root {
                    literal: *literal,
                    shown: show(&dir, &base),
                    exists: is_dir(&dir),
                    dir,
                }
            })
            .collect()
    }

    /// The namespace a prefix declaration maps, one segment per segment it
    /// wrote, with a `{..}` segment replaced by the name of the directory it
    /// reaches. `None` for a `discover` glob and for a prefix of the wrong
    /// shape, which is already [`code::E_AUTOLOAD_PREFIX_SHAPE`].
    ///
    /// This is the function [`AutoloadMap::build`] reads the prefix with, so an
    /// editor shows the namespace the map holds.
    #[must_use]
    pub fn namespace(&self) -> Option<Vec<String>> {
        let SiteKind::Prefix { prefix, .. } = &self.kind else {
            return None;
        };
        prefix_segments(&self.base_dir, prefix, self.span).ok()
    }
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
/// A discovery scan is the other way a directory's contents become part of the
/// answer: a `discover` glob lists its base directory, and
/// [`AutoloadMap::enumerate_recording`] lists every directory under every root.
/// Adding a module there changes the answer with no probed path to notice it,
/// so each directory listed is kept too, with the names it held.
///
/// What it holds: one [`PathBuf`] per root probed per autoloaded name, and one
/// [`Listing`] per directory a scan listed — O(names × roots + directories)
/// for the length of one resolution, released with the map
/// (`rule:programs/memory-priority`).
#[derive(Clone, Debug, Default)]
pub struct ProbeTrace {
    probed: Vec<PathBuf>,
    listed: Vec<Listing>,
}

impl ProbeTrace {
    /// The trace, in probe order, misses included.
    ///
    /// A name appears at most once: `crate::requires`' walk refuses a second
    /// probe of a name it has already asked about, which is what bounds the
    /// length and what makes the order a function of the program rather than of
    /// how many times something asked.
    ///
    /// A `discover` glob adds the root it builds for each directory it matches,
    /// so a module whose root appears later is noticed.
    #[must_use]
    pub fn probed(&self) -> &[PathBuf] {
        &self.probed
    }

    /// Every directory a discovery scan listed, in the order it listed them.
    #[must_use]
    pub fn listed(&self) -> &[Listing] {
        &self.listed
    }
}

/// One directory a discovery scan listed, and what it found there
/// (`rule:packaging/autoload-probes-fold-into-the-cache-key`).
///
/// `names` is only the entries the scan can act on — see [`listed_names`] — so
/// a `README.md` written beside the modules changes the directory and not the
/// answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing {
    /// The directory, as the scan named it.
    pub dir: PathBuf,
    /// [`listed_names`] of `dir` when it was listed, and `None` for a directory
    /// there was nothing to list.
    pub names: Option<Vec<String>>,
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
            let SiteKind::Prefix { prefix, roots, .. } = &site.kind else {
                continue;
            };
            let segments = match prefix_segments(&site.base_dir, prefix, site.span) {
                Ok(segments) => segments,
                Err(diagnostic) => {
                    if !is_borrowed {
                        diags.report(diagnostic);
                    }
                    continue;
                }
            };
            if let Some(previous) = map.entry(&segments) {
                if !is_borrowed {
                    report_duplicate(prefix, previous.span, site.span, diags);
                }
                continue;
            }
            map.entries.push(Entry {
                segments,
                roots: roots
                    .iter()
                    .map(|(root, _)| canonical(&site.base_dir, root))
                    .collect(),
                span: site.span,
                explicit: true,
                borrowed: is_borrowed,
            });
        }

        for &(site, is_borrowed) in &ordered {
            let SiteKind::Discover { glob, .. } = &site.kind else {
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
                map.trace.probed.extend(expanded.candidates);
                map.trace.listed.extend(expanded.listed);
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
        self.enumerate_into(&mut Vec::new())
    }

    /// [`Self::enumerate`], keeping every directory it listed in this map's
    /// [`ProbeTrace`] — the graph walk's scan, for the reason
    /// [`Self::resolve_recording`] is the graph walk's resolve.
    pub fn enumerate_recording(&mut self) -> Vec<(QName, PathBuf)> {
        let mut listed = Vec::new();
        let found = self.enumerate_into(&mut listed);
        self.trace.listed.extend(listed);
        found
    }

    /// Every class and interface [`Self::enumerate`] lists that `loaded` does
    /// not declare, each with the `extends` and `implements` its file writes,
    /// as [`HierarchyResolver::written`] reads them.
    ///
    /// This is the editor's question about a program it has analysed: which
    /// classes the program could still load, and what each would be a subtype
    /// of. A string literal under `as class<T>` loads the class it names
    /// (`rule:types/class-reference`), so these are names it may be completed
    /// to. Each file is parsed the way the graph walk parses a file it loads,
    /// and only the
    /// declaration under the name the file is listed by is kept, the one
    /// `rule:programs/one-declaration-per-autoloaded-file` allows there. An
    /// enum is not kept, because it has no links. Nothing is added to a
    /// program and no diagnostic is kept, and nothing is cached: each call
    /// lists every root again and parses every file it lists that is not
    /// loaded.
    #[must_use]
    pub fn loadable_links(&self, loaded: &SymbolTable) -> Vec<(QName, SymbolKind, ClassLinks)> {
        let mut map = SourceMap::new();
        let mut diags = Diagnostics::new();
        self.enumerate()
            .into_iter()
            .filter(|(name, _)| !loaded.contains(name))
            .filter_map(|(name, path)| {
                let id = map.load(&path).ok()?;
                let stmts = nvs_syntax::parse_file(map.file(id), &mut diags);
                let mut hierarchy = HierarchyResolver::new(CoreRoster::Trusted);
                hierarchy.collect_links(&stmts, map.file(id));
                hierarchy
                    .written()
                    .into_iter()
                    .find(|(declared, _, _)| *declared == name)
            })
            .collect()
    }

    fn enumerate_into(&self, listed: &mut Vec<Listing>) -> Vec<(QName, PathBuf)> {
        let mut found: Vec<(QName, PathBuf)> = Vec::new();
        let mut walked: FxHashSet<PathBuf> = FxHashSet::default();
        for entry in &self.entries {
            for root in &entry.roots {
                collect_declared(root, &entry.segments, &mut found, &mut walked, listed);
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
        nvs_footprint::exists(base);
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

        let missing: Vec<(String, String)> = entries
            .iter()
            .flat_map(|entry| {
                entry
                    .roots
                    .iter()
                    .filter(|root| !is_dir(root))
                    .map(|root| (entry.segments.join("\\"), show(root, &base)))
            })
            .collect();
        let _ = writeln!(out, "missing ({})", missing.len());
        for (prefix, root) in &missing {
            let _ = writeln!(out, "  {prefix:width$}  {root}");
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
///
/// Every directory visited is added to `listed`, a root that does not exist
/// included, since creating it changes the answer too.
fn collect_declared(
    dir: &Path,
    prefix: &[String],
    out: &mut Vec<(QName, PathBuf)>,
    walked: &mut FxHashSet<PathBuf>,
    listed: &mut Vec<Listing>,
) {
    if !walked.insert(crate::requires::canonicalize(dir).unwrap_or_else(|| dir.to_path_buf())) {
        return;
    }
    // A root that does not exist reads as a root declaring nothing, the same
    // answer `canonical` leaves a probe under it with.
    let entries = listing(dir);
    listed.push(Listing {
        dir: dir.to_path_buf(),
        names: entries.as_deref().map(names_of),
    });
    let Some(entries) = entries else {
        return;
    };

    for (path, is_directory) in entries {
        if is_directory {
            let name = entry_name(&path);
            if is_namespace_segment(&name) {
                let mut nested = prefix.to_vec();
                nested.push(name);
                collect_declared(&path, &nested, out, walked, listed);
            }
            continue;
        }
        // The on-disk spelling is the name here, so there is nothing for
        // `spelled_exactly` to check: this direction reads the name off the
        // filesystem instead of asking the filesystem for one.
        let Some(stem) = declared_stem(&path) else {
            continue;
        };
        let canonical = crate::requires::canonicalize(&path).unwrap_or(path);
        out.push((QName::join(prefix, &stem), canonical));
    }
}

/// The last component of `path`, as text.
fn entry_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// The name a source file under a root declares, or `None` for a file that is
/// not a `.nvs` file or whose stem cannot name a namespace segment.
fn declared_stem(path: &Path) -> Option<String> {
    if path.extension().and_then(|ext| ext.to_str()) != Some(SOURCE_EXTENSION) {
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    is_namespace_segment(stem).then(|| stem.to_owned())
}

/// The names in `entries` a discovery scan can act on, sorted: every directory
/// that can name a namespace segment, and every `.nvs` file that can name a
/// declaration.
fn names_of(entries: &[(PathBuf, bool)]) -> Vec<String> {
    let mut names: Vec<String> = entries
        .iter()
        .filter(|(path, is_directory)| {
            if *is_directory {
                is_namespace_segment(&entry_name(path))
            } else {
                declared_stem(path).is_some()
            }
        })
        .map(|(path, _)| entry_name(path))
        .collect();
    names.sort();
    names
}

/// What a discovery scan would record for `dir` if it listed it now — the
/// [`Listing::names`] a later check compares against, read through the same
/// filter, so the two can only differ where the answer can.
#[must_use]
pub fn listed_names(dir: &Path) -> Option<Vec<String>> {
    listing(dir).as_deref().map(names_of)
}

/// Every entry of `dir`, each name as the disk spells it and whether it is a
/// directory, sorted by name, or `None` for a directory there is nothing to
/// list.
///
/// This is the listing a `discover` glob and § 3's scan read, unfiltered. An
/// editor offers names out of it while a `require` or `autoload` written path
/// is being written, so what it offers is what resolution would find.
#[must_use]
pub fn entries_of(dir: &Path) -> Option<Vec<(String, bool)>> {
    let mut entries: Vec<(String, bool)> = listing(dir)?
        .iter()
        .map(|(path, is_directory)| (entry_name(path), *is_directory))
        .collect();
    entries.sort();
    Some(entries)
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
    nvs_footprint::dir(dir);
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
    nvs_footprint::exists(path);
    path.is_dir()
}

/// One path the way [`AutoloadMap::render`] prints it: relative to `base`
/// where it sits under it, whole where it does not, and always `/`-separated
/// so the Windows and WSL legs render one string.
///
/// A whole path loses the `\\?\` Windows puts in front of a canonical one, so
/// it prints as `D:/srv/app` and a network path as `//host/share`.
fn show(path: &Path, base: &Path) -> String {
    let shown = path.strip_prefix(base).unwrap_or(path);
    let text = shown.to_string_lossy().replace('\\', "/");
    let text = match text.strip_prefix("//?/") {
        Some(rest) => rest
            .strip_prefix("UNC/")
            .map_or_else(|| rest.to_owned(), |share| format!("//{share}")),
        None => text,
    };
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
///
/// A missing root is still made canonical as far as the disk allows: its `.`
/// and `..` steps are folded by name, the deepest directory that exists is
/// canonicalized, and the rest is added back. `nvs check --autoload-map` then
/// prints it under `missing` in the same form as every other root.
fn canonical(base_dir: &Path, root: &str) -> PathBuf {
    let joined = base_dir.join(root);
    if let Some(found) = crate::requires::canonicalize(&joined) {
        return found;
    }
    let mut folded = PathBuf::new();
    for part in joined.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                folded.pop();
            }
            other => folded.push(other),
        }
    }
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    let mut existing = folded.clone();
    loop {
        if let Some(found) = crate::requires::canonicalize(&existing) {
            let mut whole = found;
            whole.extend(rest.iter().rev());
            return whole;
        }
        let Some(name) = existing.file_name().map(std::ffi::OsStr::to_os_string) else {
            return folded;
        };
        rest.push(name);
        existing.pop();
    }
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
    /// The base directory the glob listed, when its shape let it list one.
    listed: Vec<Listing>,
    /// The root built for each matched directory whose glob has segments after
    /// the `*`, found or not: creating `Shop/src` under `'*/src'` adds a module
    /// without changing what the base directory lists.
    candidates: Vec<PathBuf>,
}

/// Why a `discover` glob has no segment to match.
enum GlobShape {
    /// No `*` is a whole segment.
    NoStar,
    /// More than one segment holds a `*`.
    ManyStars,
}

/// A glob's segments, split at either separator.
fn glob_parts(glob: &str) -> Vec<&str> {
    glob.split(['/', '\\']).collect()
}

/// Which of `parts` is the glob's one `*` segment.
fn star_segment(parts: &[&str]) -> Result<usize, GlobShape> {
    let star = parts
        .iter()
        .position(|part| *part == "*")
        .ok_or(GlobShape::NoStar)?;
    if parts.iter().filter(|part| part.contains('*')).count() != 1 {
        return Err(GlobShape::ManyStars);
    }
    Ok(star)
}

/// The directory a glob lists: `base_dir` joined with the segments before its
/// `*` segment at `star`.
fn glob_base(base_dir: &Path, parts: &[&str], star: usize) -> PathBuf {
    let mut scanned = base_dir.to_path_buf();
    for part in &parts[..star] {
        scanned.push(part);
    }
    scanned
}

/// Expands `autoload discover '<glob>'` into its `(prefix, root)` pairs.
fn discover(base_dir: &Path, glob: &str, span: Span, diags: &mut Diagnostics) -> Discovered {
    let parts = glob_parts(glob);
    let star = match star_segment(&parts) {
        Ok(star) => star,
        Err(GlobShape::NoStar) => {
            diags.report(
                Diagnostic::error(
                    code::E_AUTOLOAD_GLOB_SHAPE,
                    format!("`{glob}` is not a discovery glob"),
                )
                .with_primary(span, "no `*` occupying a whole path segment")
                .with_note("a `discover` glob holds exactly one `*`, and it is a whole segment"),
            );
            return Discovered::default();
        }
        Err(GlobShape::ManyStars) => {
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
    };

    let scanned = glob_base(base_dir, &parts, star);
    let entries = listing(&scanned);
    let mut out = Discovered {
        listed: vec![Listing {
            dir: scanned.clone(),
            names: entries.as_deref().map(names_of),
        }],
        ..Discovered::default()
    };
    let Some(entries) = entries else {
        diags.report(
            Diagnostic::error(
                code::E_AUTOLOAD_GLOB_SHAPE,
                format!("`{}` cannot be scanned", scanned.display()),
            )
            .with_primary(span, "no directory to discover modules in"),
        );
        return out;
    };

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
        if star + 1 < parts.len() {
            out.candidates.push(root.clone());
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
    out.candidates.sort();
    out
}

/// Turns a written prefix into its segments, replacing a `{..}` segment by the
/// name of the directory it reaches — `rule:programs/autoload`'s directory
/// segment.
///
/// The braces hold a path of `.` and `..` steps only, resolved against the
/// declaring file's directory exactly as a root is, so `{.}` is that
/// directory's own name and `{..}` its parent's. The name is read from the
/// canonical path, which is the directory as the disk spells it. At most one
/// such segment is allowed: two would name the same module twice, or two
/// levels of one tree, and nothing needs that.
///
/// Every segment, written or read off the disk, has to be a namespace segment.
/// A prefix holding anything else can never match a name, so it is
/// [`code::E_AUTOLOAD_PREFIX_SHAPE`] here rather than an `E0303` on the first
/// name it was meant to find.
fn prefix_segments(base_dir: &Path, prefix: &str, span: Span) -> Result<Vec<String>, Diagnostic> {
    let refuse = |label: String| {
        Diagnostic::error(
            code::E_AUTOLOAD_PREFIX_SHAPE,
            format!("`{prefix}` is not a namespace prefix"),
        )
        .with_primary(span, label)
        .with_note(
            "each segment of a prefix is a `PascalCase` name, or one `{..}` naming a directory; \
             there is no wildcard, and `autoload discover` is the form that maps many directories",
        )
    };

    let braced = |written: &str| {
        written
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'))
            .map(str::to_owned)
    };
    if prefix.split('\\').filter_map(braced).count() > 1 {
        return Err(refuse("only one segment may name a directory".to_owned()));
    }

    let mut segments = Vec::new();
    for written in prefix.split('\\') {
        let Some(steps) = braced(written) else {
            if !is_namespace_segment(written) {
                return Err(refuse(format!("`{written}` is not a namespace segment")));
            }
            segments.push(written.to_owned());
            continue;
        };
        if steps
            .split(['/', '\\'])
            .any(|step| step != "." && step != "..")
        {
            return Err(refuse(format!(
                "`{{{steps}}}` holds something other than `.` and `..` steps"
            )));
        }
        let reached = crate::requires::canonicalize(&base_dir.join(&steps));
        let name = reached
            .as_deref()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned());
        match name {
            Some(name) if is_namespace_segment(&name) => segments.push(name),
            Some(name) => {
                return Err(refuse(format!(
                    "`{{{steps}}}` names the directory `{name}`, which is not a namespace segment"
                )));
            }
            None => {
                return Err(refuse(format!("`{{{steps}}}` reaches no named directory")));
            }
        }
    }
    Ok(segments)
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
/// exactly one top-level declaration, whose qualified name is the one the map
/// found the file by, and nothing else but `namespace`/`use`.
///
/// `expected` is that name: the prefix, the directories under the root, and
/// the file's base name. The namespace the file writes is compared too, and
/// exactly, because a file declaring some other name would put that name in
/// the program only when something probed or scanned this path. Files reached
/// by `require` are deliberately not subject to any of this and may declare
/// anything.
pub fn check_file_shape(
    stmts: &[Stmt],
    src: &SourceFile,
    expected: &QName,
    span: Span,
    diags: &mut Diagnostics,
) {
    let mut declared: Vec<(QName, Span)> = Vec::new();
    let mut stray: Option<Span> = None;
    scan_shape(stmts, src, &[], &mut declared, &mut stray);

    let file_name = expected.short_name();
    let complaint = if declared.is_empty() {
        Some((span, format!("nothing here declares `{file_name}`")))
    } else if declared.len() > 1 {
        Some((declared[1].1, "a second declaration".to_owned()))
    } else if let Some(at) = stray {
        Some((
            at,
            "only declarations belong in an autoloaded file".to_owned(),
        ))
    } else if declared[0].0 == *expected {
        None
    } else if declared[0].0.short_name() != file_name {
        Some((
            declared[0].1,
            format!(
                "this declares `{}`, but the file is named `{file_name}`",
                declared[0].0.short_name()
            ),
        ))
    } else {
        Some((
            declared[0].1,
            format!(
                "this declares `{}`, but the autoload map finds this file as `{expected}`",
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

/// Collects every top-level declaration under the namespace in force where
/// it stands. The statement form `namespace A;` sets the namespace for the
/// statements after it; the bracketed form sets it for its own block.
fn scan_shape(
    stmts: &[Stmt],
    src: &SourceFile,
    outer: &[String],
    declared: &mut Vec<(QName, Span)>,
    stray: &mut Option<Span>,
) {
    let mut namespace = outer.to_vec();
    for stmt in stmts {
        let span = match &stmt.kind {
            StmtKind::ClassDecl(d) => d.name.span,
            StmtKind::InterfaceDecl(d) => d.name.span,
            StmtKind::EnumDecl(d) => d.name.span,
            StmtKind::TypeAliasDecl(d) => d.name.span,
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let named = name.as_ref().map_or_else(Vec::new, |name| {
                    let text = src.span_text(name.span).unwrap_or_default();
                    QName::parse(text).segments().to_vec()
                });
                match body {
                    Some(block) => scan_shape(&block.stmts, src, &named, declared, stray),
                    None => namespace = named,
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
        let text = src.span_text(span).unwrap_or_default();
        declared.push((QName::join(&namespace, text), span));
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
