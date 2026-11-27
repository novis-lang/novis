//! The check every refactoring's edit passes before anything is written
//! (`rule:ide/a-refactoring-checks-its-edit-before-it-writes`).
//!
//! A refactoring computes [`Rewrite`]s and file moves and hands them to
//! [`checked`]. It applies them in memory, over the open buffers, and analyses
//! every entry point whose analysis read a file the edit changes or moves
//! ([`SymbolIndex::readers`]). The same entries are analysed as they are now.
//! An error the edit adds refuses it with that error's message and position.
//! A warning never refuses it, and neither does an error the program already
//! had. Both clients call it, `nvs code` before it writes and the server
//! before it answers, so the two give one answer.
//!
//! # Decision: an error is new when its file has more errors of its code
//!
//! Errors are counted per file and per code, before and after, and the first
//! error after the edit whose count grew is the one reported. A message is not
//! compared: an error that names a renamed symbol changes its text without
//! being new. A position is not compared either, since the edit moves the
//! text. A moved file is compared at its new path.
//!
//! # Decision: the old path of a moved file stays readable
//!
//! The analysis overlays each moved file at its new path, and
//! `nvs_hir::requires::with_planned` lets an `autoload` probe find it there
//! before it is on disk. It cannot hide the
//! old one, which is still on disk. A use the edit forgot that still names the
//! old place therefore resolves and adds no error. The rewrites of
//! `crate::rename::moved` write every `require` again, so this is a name the
//! index already follows.
//!
//! # What it spends
//!
//! Nothing at runtime, since it only edits source. Per refactoring, two
//! analyses of each entry point that reads a changed file, one before the edit
//! and one after, on the caller's thread. A rename in a large project is slower
//! by those analyses. That is accepted: a wrong edit is worse than a slow one.
//! Everything is dropped with the answer.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use nvs_diagnostics::{SourceMap, canonical_key};

use crate::document::{Analysed, Entry, Reads, analyse_file, analyse_through};
use crate::rename::Rewrite;
use crate::stubs::Stubs;
use crate::{Documents, Site, SymbolIndex};

/// An edit that passed [`checked`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    /// The rewrites, unchanged.
    pub rewrites: Vec<Rewrite>,
    /// Every entry point the check analysed, at its path after the edit, in
    /// path order. Each was analysed twice: before the edit and after it.
    pub analysed: Vec<PathBuf>,
}

/// The first error an edit adds, which refuses it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Broken {
    /// Where the error is, in the file as the edit writes it, at its path
    /// after the edit.
    pub site: Site,
    /// Its line, from 1.
    pub line: usize,
    /// Its column, from 1, in characters.
    pub column: usize,
    /// The error's code, such as `E0405`, where it has one.
    pub code: Option<String>,
    /// The error's message.
    pub message: String,
}

impl Broken {
    /// The refusal both clients show, with the file's name, line and column.
    #[must_use]
    pub fn refusal(&self) -> String {
        let file = self.site.path.file_name().map_or_else(
            || self.site.path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let code = self
            .code
            .as_ref()
            .map_or_else(String::new, |code| format!("{code} "));
        format!(
            "This change is not made, because the code would not compile: {code}{} at {file}:{}:{}.",
            self.message, self.line, self.column
        )
    }
}

/// `rewrites` and `moves`, applied in memory and analysed, or the first error
/// they add.
///
/// `moves` are `(old, new)` paths, applied after the rewrites, which are keyed
/// by each file's current path. A directory moves every indexed file under
/// it. An edit that changes and moves nothing analyses nothing.
///
/// # Errors
///
/// Returns the first error the edit adds, as the module doc defines it.
pub fn checked(
    documents: &Documents,
    index: &SymbolIndex,
    rewrites: Vec<Rewrite>,
    moves: &[(PathBuf, PathBuf)],
) -> Result<Checked, Broken> {
    let mut texts = read_texts(documents, &rewrites);
    apply(&mut texts, &rewrites);
    let places = places(index, moves, &texts);

    let mut readers: BTreeSet<PathBuf> = BTreeSet::new();
    for changed in texts.keys().chain(places.keys()) {
        readers.extend(index.readers(changed));
        if index.holds(changed) {
            readers.insert(changed.clone());
        }
    }
    if readers.is_empty() {
        return Ok(Checked {
            rewrites,
            analysed: Vec::new(),
        });
    }

    let now = |path: &Path| -> PathBuf {
        let key = canonical_key(path);
        places.get(&key).cloned().unwrap_or(key)
    };
    let mut overlaid: Vec<(PathBuf, String)> = Vec::new();
    for (path, text) in &texts {
        overlaid.push((now(path), text.clone()));
    }
    for (old, new) in &places {
        if !texts.contains_key(old) {
            let Some(text) = disk_text(documents, old) else {
                continue;
            };
            overlaid.push((new.clone(), text));
        }
    }
    let after_reads = Overlaid {
        base: documents,
        texts: overlaid,
    };

    let mut analysed = Vec::new();
    let mut first: Option<Broken> = None;
    for entry in &readers {
        let before = analyse_file(documents, entry, 0)
            .map(|analysed| errors(&analysed, &now))
            .unwrap_or_default();
        let at = now(entry);
        let planned: Vec<PathBuf> = places.values().cloned().collect();
        let after =
            nvs_hir::requires::with_planned(planned, || analyse_with(documents, &after_reads, &at));
        let Some(after) = after else {
            continue;
        };
        analysed.push(at);
        if first.is_none() {
            first = added(&after, &before, &now);
        }
    }
    analysed.sort();
    match first {
        Some(broken) => Err(broken),
        None => Ok(Checked { rewrites, analysed }),
    }
}

/// Whether the entry document of `analysed`, with each `(start, end, text)`
/// of `edits` written into it, analyses with no error that `analysed` does not
/// already have, as the module doc counts them.
///
/// The check for an edit to one open document that reads nothing outside its
/// own graph: the four generators of `crate::generate` and an override
/// completion row. `analysed` is the analysis before the edit, so this costs
/// one analysis, of the rewritten text. A range outside the text is `false`.
pub(crate) fn adds_no_error_in_entry(
    documents: &Documents,
    analysed: &Analysed,
    edits: &[(usize, usize, &str)],
) -> bool {
    let mut text = analysed.map.file(analysed.entry).text().to_owned();
    let mut ordered: Vec<&(usize, usize, &str)> = edits.iter().collect();
    ordered.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    for (start, end, replacement) in ordered {
        if text.get(*start..*end).is_none() {
            return false;
        }
        text.replace_range(*start..*end, replacement);
    }
    let Some(after) = crate::document::analyse_rewritten(documents, analysed, text) else {
        return false;
    };
    let same = |path: &Path| path.to_path_buf();
    added(&after, &errors(analysed, &same), &same).is_none()
}

/// The text of each file `rewrites` edits, by `canonical_key`, as it is now.
fn read_texts(documents: &Documents, rewrites: &[Rewrite]) -> BTreeMap<PathBuf, String> {
    let mut texts = BTreeMap::new();
    for rewrite in rewrites {
        let key = canonical_key(&rewrite.site.path);
        if texts.contains_key(&key) {
            continue;
        }
        if let Some(text) = disk_text(documents, &key) {
            texts.insert(key, text);
        }
    }
    texts
}

/// The text of `path` with the open buffers overlaid, or `None` for a file
/// that cannot be read.
fn disk_text(documents: &Documents, path: &Path) -> Option<String> {
    let mut map = SourceMap::new();
    documents.overlay(&mut map);
    let id = map.load(path).ok()?;
    Some(map.file(id).text().to_owned())
}

/// Writes each rewrite into its file's text, last first, so an earlier site's
/// bytes do not move. A rewrite whose range is outside its text, or that
/// overlaps the one after it, is skipped.
fn apply(texts: &mut BTreeMap<PathBuf, String>, rewrites: &[Rewrite]) {
    let mut by_file: BTreeMap<PathBuf, Vec<&Rewrite>> = BTreeMap::new();
    for rewrite in rewrites {
        by_file
            .entry(canonical_key(&rewrite.site.path))
            .or_default()
            .push(rewrite);
    }
    for (path, mut edits) in by_file {
        let Some(text) = texts.get_mut(&path) else {
            continue;
        };
        edits.sort_by_key(|rewrite| (rewrite.site.start, rewrite.site.end));
        let mut limit = usize::MAX;
        for rewrite in edits.into_iter().rev() {
            let (Ok(start), Ok(end)) = (
                usize::try_from(rewrite.site.start),
                usize::try_from(rewrite.site.end),
            ) else {
                continue;
            };
            if end > limit || text.get(start..end).is_none() {
                continue;
            }
            text.replace_range(start..end, &rewrite.text);
            limit = start;
        }
    }
}

/// Each moved file, by `canonical_key`, and the key it has after the move.
/// A directory moves every file under it that the index holds or the edit
/// changes.
fn places(
    index: &SymbolIndex,
    moves: &[(PathBuf, PathBuf)],
    texts: &BTreeMap<PathBuf, String>,
) -> BTreeMap<PathBuf, PathBuf> {
    let mut places = BTreeMap::new();
    for (old, new) in moves {
        let old = canonical_key(old);
        let new = crate::rename::new_key(new);
        for file in index.files().chain(texts.keys().map(PathBuf::as_path)) {
            if let Ok(rest) = file.strip_prefix(&old) {
                let moved = if rest.as_os_str().is_empty() {
                    new.clone()
                } else {
                    new.join(rest)
                };
                places.insert(file.to_path_buf(), moved);
            }
        }
    }
    places
}

/// `entry` analysed through `reads`, with the caches `documents` keeps.
fn analyse_with(documents: &Documents, reads: &Overlaid<'_>, entry: &Path) -> Option<Analysed> {
    let (mut session, mut trees) = documents.caches();
    analyse_through(
        reads,
        &mut session,
        &mut trees,
        Entry {
            path: entry,
            key: &canonical_key(entry),
        },
        0,
        None,
        &|_| false,
        true,
    )
}

/// How many errors of each code each file of `analysed` has, by the file's
/// path after the edit. An error with no code is counted under its message.
fn errors(
    analysed: &Analysed,
    now: &impl Fn(&Path) -> PathBuf,
) -> BTreeMap<(PathBuf, String), usize> {
    let mut counts = BTreeMap::new();
    for (path, kind, _) in error_sites(analysed, now) {
        *counts.entry((path, kind)).or_insert(0) += 1;
    }
    counts
}

/// The first error of `after` whose file has more errors of its code than
/// `before` counted.
fn added(
    after: &Analysed,
    before: &BTreeMap<(PathBuf, String), usize>,
    now: &impl Fn(&Path) -> PathBuf,
) -> Option<Broken> {
    let mut seen: BTreeMap<(PathBuf, String), usize> = BTreeMap::new();
    for (path, kind, broken) in error_sites(after, now) {
        let count = seen.entry((path.clone(), kind.clone())).or_insert(0);
        *count += 1;
        if *count > before.get(&(path, kind)).copied().unwrap_or(0) {
            return Some(broken);
        }
    }
    None
}

/// Every error of `analysed` that has a position in a file with a path: the
/// file's path after the edit, the code or message it is counted under, and
/// the error as [`Broken`] reports it.
fn error_sites(
    analysed: &Analysed,
    now: &impl Fn(&Path) -> PathBuf,
) -> Vec<(PathBuf, String, Broken)> {
    let mut found = Vec::new();
    for diagnostic in analysed.diags.iter() {
        if !diagnostic.is_error() {
            continue;
        }
        let Some(span) = diagnostic.primary_span() else {
            continue;
        };
        let file = analysed.map.file(span.file);
        let Some(path) = file.path().map(now) else {
            continue;
        };
        let code = diagnostic.code.map(|code| code.to_string());
        let kind = code.clone().unwrap_or_else(|| diagnostic.message.clone());
        let (line, column) = file.line_col(span.start);
        found.push((
            path.clone(),
            kind,
            Broken {
                site: Site {
                    path,
                    start: span.start,
                    end: span.end,
                },
                line: line + 1,
                column: column + 1,
                code,
                message: diagnostic.message.clone(),
            },
        ));
    }
    found
}

/// The open buffers, with the edit's texts overlaid on top at their paths
/// after the edit.
struct Overlaid<'a> {
    /// What every analysis reads.
    base: &'a Documents,
    /// Each changed or moved file's text, by its `canonical_key` after the edit.
    texts: Vec<(PathBuf, String)>,
}

impl Reads for Overlaid<'_> {
    fn overlay(&self, map: &mut SourceMap) {
        self.base.overlay(map);
        for (key, text) in &self.texts {
            map.overlay_key(key.clone(), text.clone());
        }
    }

    fn lender_for(&self, key: &Path) -> Option<&nvs_hir::Lender> {
        self.base.lender_for(key)
    }

    fn stubs(&self) -> Option<&Stubs> {
        Reads::stubs(self.base)
    }

    fn open_keys(&self) -> Vec<PathBuf> {
        self.base.open_keys()
    }
}
