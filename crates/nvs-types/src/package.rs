//! Which package each file of a program is in, and the two refusals that keep a package inside
//! itself: `rule:packaging/a-package-file-stays-under-its-declared-prefix` and
//! `rule:packaging/a-package-cannot-reach-its-host`.
//!
//! **A file is in a package when a directory between it and the entry's directory holds a
//! `package.toml` with a `[package] name`.** The nearest such file wins, so a package inside
//! another package's directory is its own. The walk is over the file's canonical path, so an
//! autoloaded file, a file a `discover` glob found and a file a `require` loaded are placed by
//! one rule, and nothing has to remember how a file was reached. The walk stops at the entry's
//! directory: a `package.toml` there or above is the application's own, which `nvs_config::grants`
//! reads for `[grants]`. A file with a name-less `package.toml`, or with none, is in no package
//! and gets no file check (hand-copied code). `nvs_config::package` reads the manifest.
//!
//! **A package file declares the package's namespace or one below it.** Every `namespace`
//! declaration at the top of the file is compared to the package's name exactly, and a file
//! that declares none is in the global namespace, which no package owns. Either is
//! [`code::E_PACKAGE_NAMESPACE_OUTSIDE`]. Authority is looked up by the enclosing namespace
//! (`rule:security/authority-is-the-enclosing-namespace`), so this is what keeps a package from
//! writing `namespace App;` to hold the application's grants.
//!
//! **A path a package writes stays in the package.** Each `require` edge the walk resolved, and
//! each existing root of an `autoload` declaration written in a package file, is canonicalized
//! and then compared to the package's directory component by component
//! (`rule:security/path-scope-canonicalise-then-prefix`). `..`, an absolute path and a symlink
//! all resolve before the comparison, so each one that leaves the directory is
//! [`code::E_PACKAGE_PATH_ESCAPES`].
//!
//! Cost: one `stat` of `package.toml` per distinct directory between a loaded file and the entry,
//! each noted as missed in the `SourceMap` so a server sees a manifest created there, and one read
//! of each package's manifest. A program that loads only its entry pays nothing. All of it is
//! compile time; nothing is held per request.

use std::path::{Path, PathBuf};

use nvs_config::package::{self, Package};
use nvs_diagnostics::{Diagnostic, Diagnostics, SourceFile, SourceId, SourceMap, Span, code};
use nvs_hir::autoload::SiteKind;
use nvs_hir::{AutoloadMap, Loaded, QName};
use nvs_syntax::ast::{NamespaceDecl, Stmt, StmtKind};
use rustc_hash::FxHashMap;

/// The packages of a program, and which of them each file is in.
#[derive(Clone, Debug, Default)]
pub struct Packages {
    /// Every package a loaded file is in, each once, its directory canonical.
    packages: Vec<Package>,
    /// The index into [`Self::packages`] of each file that is in a package.
    by_file: FxHashMap<SourceId, usize>,
}

impl Packages {
    /// The package `file` is in, or `None` for a file of the application.
    #[must_use]
    pub fn of(&self, file: SourceId) -> Option<&Package> {
        self.by_file.get(&file).map(|&at| &self.packages[at])
    }

    /// Every package a loaded file is in.
    #[must_use]
    pub fn all(&self) -> &[Package] {
        &self.packages
    }
}

/// Places each file of `loaded` in its package, and reports every package file that declares a
/// namespace outside its package and every path a package writes that leaves it.
///
/// `entry` is the program's entry file. A program whose entry has no path on disk has no
/// packages.
pub fn check(
    entry: SourceId,
    loaded: &[Loaded],
    autoload: &AutoloadMap,
    map: &mut SourceMap,
    diags: &mut Diagnostics,
) -> Packages {
    let mut packages = Packages::default();
    if loaded.len() < 2 {
        return packages;
    }
    let Some(stop) = map
        .file(entry)
        .path()
        .and_then(Path::parent)
        .and_then(nvs_hir::requires::canonicalize)
    else {
        return packages;
    };
    let mut finder = Finder {
        stop,
        dirs: FxHashMap::default(),
    };
    // The entry is never in a package: its directory is where the walk stops.
    for file in loaded.iter().filter(|file| file.id != entry) {
        let Some(dir) = map.file(file.id).path().and_then(Path::parent) else {
            continue;
        };
        let dir = dir.to_path_buf();
        if let Some(at) = finder.package_of(&dir, &mut packages.packages, map, diags) {
            packages.by_file.insert(file.id, at);
        }
    }
    if packages.packages.is_empty() {
        return packages;
    }

    for file in loaded {
        let Some(package) = packages.of(file.id) else {
            continue;
        };
        check_namespaces(&file.stmts, map.file(file.id), package, diags);
        for &(span, target) in &file.requires {
            let Some(path) = map.file(target).path() else {
                continue;
            };
            if !within(package.dir(), path, nvs_hir::requires::canonicalize) {
                diags.report(escapes(span, path, package, "this `require` path"));
            }
        }
    }

    for site in autoload.sites() {
        let Some(base) = nvs_hir::requires::canonicalize(&site.base_dir) else {
            continue;
        };
        let Some(package) = packages
            .packages
            .iter()
            .filter(|package| base.starts_with(package.dir()))
            .max_by_key(|package| package.dir().components().count())
        else {
            continue;
        };
        let reached: Vec<(Span, PathBuf)> = match &site.kind {
            SiteKind::Prefix { .. } => site
                .roots()
                .into_iter()
                .filter(|root| root.exists)
                .map(|root| (root.literal, root.dir))
                .collect(),
            SiteKind::Discover { .. } => site.directories(),
        };
        for (span, dir) in reached {
            if !within(package.dir(), &dir, nvs_hir::requires::canonicalize) {
                diags.report(escapes(span, &dir, package, "this `autoload` path"));
            }
        }
    }
    packages
}

/// Whether `target` is in `dir` once `canonicalize` has resolved every `..` and symlink in it.
/// `dir` is canonical already. A path that does not resolve is not in it.
///
/// `canonicalize` is a parameter so a test can stand in for a filesystem with a symlink on it.
pub fn within(dir: &Path, target: &Path, canonicalize: impl Fn(&Path) -> Option<PathBuf>) -> bool {
    canonicalize(target).is_some_and(|real| real.starts_with(dir))
}

/// The walk up from a file's directory to the nearest package, with each directory's answer kept.
struct Finder {
    /// The entry's canonical directory. A `package.toml` here or above is the application's.
    stop: PathBuf,
    /// Each directory walked, and the package its nearest manifest names, if any.
    dirs: FxHashMap<PathBuf, Option<usize>>,
}

impl Finder {
    /// The index in `packages` of the package `dir` is in, read and pushed the first time a
    /// directory of it is walked.
    fn package_of(
        &mut self,
        dir: &Path,
        packages: &mut Vec<Package>,
        map: &mut SourceMap,
        diags: &mut Diagnostics,
    ) -> Option<usize> {
        let mut walked: Vec<PathBuf> = Vec::new();
        let mut found = None;
        for ancestor in dir.ancestors() {
            if let Some(&known) = self.dirs.get(ancestor) {
                found = known;
                break;
            }
            if self.stop.starts_with(ancestor) {
                break;
            }
            walked.push(ancestor.to_path_buf());
            let manifest = ancestor.join(nvs_config::grants::FILE);
            if !manifest.is_file() {
                map.note_missing(&manifest);
                continue;
            }
            found = match package::read(ancestor, map) {
                Ok(Some(read)) => {
                    packages.push(read);
                    Some(packages.len() - 1)
                }
                Ok(None) => None,
                Err(refused) => {
                    diags.report(refused);
                    None
                }
            };
            break;
        }
        for each in walked {
            self.dirs.insert(each, found);
        }
        found
    }
}

/// Reports each `namespace` a package file declares outside `package`, and a file that declares
/// none.
fn check_namespaces(stmts: &[Stmt], src: &SourceFile, package: &Package, diags: &mut Diagnostics) {
    let mut declared = false;
    for stmt in stmts {
        let StmtKind::NamespaceDecl(NamespaceDecl { name, span, .. }) = &stmt.kind else {
            continue;
        };
        declared = true;
        let (at, namespace) = match name {
            Some(name) => {
                let text = src.span_text(name.span).unwrap_or_default();
                (name.span, QName::parse(text).segments().join("\\"))
            }
            None => (*span, String::new()),
        };
        if !namespace.is_empty() && package.owns(&namespace) {
            continue;
        }
        diags.report(outside(at, src, &namespace, package));
    }
    if !declared && let Some(first) = stmts.first() {
        diags.report(outside(first.span, src, "", package));
    }
}

/// [`code::E_PACKAGE_NAMESPACE_OUTSIDE`] for `namespace`, empty for the global namespace.
fn outside(at: Span, src: &SourceFile, namespace: &str, package: &Package) -> Diagnostic {
    let file = src
        .path()
        .map_or_else(|| src.name().to_owned(), |path| path.display().to_string());
    let shown = if namespace.is_empty() {
        "the global namespace".to_owned()
    } else {
        format!("the namespace `{namespace}`")
    };
    Diagnostic::error(
        code::E_PACKAGE_NAMESPACE_OUTSIDE,
        format!(
            "`{file}` declares {shown}, outside the package `{}`",
            package.name()
        ),
    )
    .with_primary(at, "this namespace is not the package's")
    .with_note(format!(
        "every file of the package `{name}` declares `namespace {name};` or a namespace below it",
        name = package.name()
    ))
    .with_help(format!(
        "write `namespace {}\\...;` at the top of the file",
        package.name()
    ))
}

/// [`code::E_PACKAGE_PATH_ESCAPES`] for `target`, which `what` at `at` reached.
fn escapes(at: Span, target: &Path, package: &Package, what: &str) -> Diagnostic {
    Diagnostic::error(
        code::E_PACKAGE_PATH_ESCAPES,
        format!(
            "`{}` is outside the package `{}`",
            target.display(),
            package.name()
        ),
    )
    .with_primary(at, format!("{what} leaves the package"))
    .with_note(format!(
        "a package can only load files in its own directory, `{}`",
        package.dir().display()
    ))
    .with_help("change the path, or move the file into the package".to_owned())
}
