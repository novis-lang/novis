//! The tests each coverage run saw reach a line, read for the "used by" lens.
//!
//! `rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method` is the
//! specification. A coverage run's JSON report is `schemaVersion: 3`, and each
//! of its records carries `coverage`: every file the test reached, mapped to
//! the sorted lines it reached there (`rule:testing/report-formats`). A test
//! counts for a method when it reached a line inside the method.
//!
//! **The client keeps the reports, and names the directory.** The VS Code
//! client writes one report per program into a directory of its own storage,
//! and names it in `initialize`'s `initializationOptions` as `coverage.dir`,
//! the way it names `stubs.dir` ([`crate::settings::Settings::coverage`]). A
//! report replaces the one its program wrote before, so the directory holds the
//! newest report of each program. Every `.json` file in it is read, and a file
//! that is not a report is skipped. A file name the report wrote relative is
//! resolved against the workspace root; the VS Code client writes them
//! absolute.
//!
//! **A report applies to a file one of its tests reached.** Where no report
//! reached the method's file, [`Reports::tests_reaching`] says nothing and the
//! lens counts tests off the call graph, marked "static". A count read from a
//! report is as old as its run, and the next coverage run of that program
//! replaces it.
//!
//! **What it costs:** the per-test lines of every report, held for the server's
//! life and read again only when a report's size or time changes. A lens
//! request reads the directory and the time of each file in it.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use nvs_diagnostics::canonical_key;
use rustc_hash::FxHashMap;
use serde_json::Value;

/// The reports in one directory, each read once per change.
#[derive(Debug, Default)]
pub struct Reports {
    /// The directory the client names, and `None` for a client that named
    /// none, which has no reports.
    dir: Option<PathBuf>,
    /// What a relative file name in a report is resolved against.
    root: Option<PathBuf>,
    /// Each report read, by its path.
    held: BTreeMap<PathBuf, Held>,
}

/// One report, and the size and time it was read at.
#[derive(Debug)]
struct Held {
    stamp: (u64, Option<SystemTime>),
    /// Each test the report ran, as `Class::method`, and the lines it reached
    /// in each file, keyed as [`crate::index::Site::path`] is.
    tests: Vec<(String, FxHashMap<PathBuf, Vec<u32>>)>,
}

impl Reports {
    /// The reports in `dir`, nothing read yet. A relative file name is
    /// resolved against `root`.
    #[must_use]
    pub fn new(dir: Option<PathBuf>, root: Option<PathBuf>) -> Self {
        Self {
            dir,
            root,
            held: BTreeMap::new(),
        }
    }

    /// Reads each report that is new or changed since the last call, and
    /// drops each one that is gone.
    pub fn refresh(&mut self) {
        let Some(dir) = &self.dir else {
            return;
        };
        let mut present: Vec<(PathBuf, (u64, Option<SystemTime>))> = fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            .filter_map(|entry| {
                let meta = entry.metadata().ok()?;
                meta.is_file()
                    .then(|| (entry.path(), (meta.len(), meta.modified().ok())))
            })
            .collect();
        present.sort();
        self.held
            .retain(|path, _| present.iter().any(|(found, _)| found == path));
        for (path, stamp) in present {
            if self.held.get(&path).is_some_and(|held| held.stamp == stamp) {
                continue;
            }
            let tests = fs::read_to_string(&path)
                .ok()
                .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                .map(|document| self.tests_of(&document))
                .unwrap_or_default();
            self.held.insert(path, Held { stamp, tests });
        }
    }

    /// The tests `document` ran, with the lines each reached.
    fn tests_of(&self, document: &Value) -> Vec<(String, FxHashMap<PathBuf, Vec<u32>>)> {
        let Some(records) = document.get("tests").and_then(Value::as_array) else {
            return Vec::new();
        };
        records
            .iter()
            .filter_map(|record| {
                let class = record.get("class")?.as_str()?;
                let method = record.get("method")?.as_str()?;
                let reached = record.get("coverage")?.as_object()?;
                let lines = reached
                    .iter()
                    .map(|(file, lines)| {
                        let lines = lines
                            .as_array()
                            .map(|lines| {
                                lines
                                    .iter()
                                    .filter_map(Value::as_u64)
                                    .filter_map(|line| u32::try_from(line).ok())
                                    .collect()
                            })
                            .unwrap_or_default();
                        (self.key(file), lines)
                    })
                    .collect();
                Some((format!("{class}::{method}"), lines))
            })
            .collect()
    }

    /// `file` as a report wrote it, as the index keys a file.
    fn key(&self, file: &str) -> PathBuf {
        let written = Path::new(file);
        let path = match &self.root {
            Some(root) if written.is_relative() => root.join(written),
            _ => written.to_path_buf(),
        };
        canonical_key(&path)
    }

    /// The tests that reached a line of `path` from `first` to `last`, both
    /// one-based and included, sorted. `None` when no report reached `path`
    /// at all, so the count is the call graph's. `path` is under
    /// [`canonical_key`].
    #[must_use]
    pub fn tests_reaching(&self, path: &Path, first: u32, last: u32) -> Option<Vec<String>> {
        let mut applies = false;
        let mut found: Vec<String> = Vec::new();
        for (test, reached) in self.held.values().flat_map(|held| &held.tests) {
            let Some(lines) = reached.get(path) else {
                continue;
            };
            applies = true;
            if lines.iter().any(|line| (first..=last).contains(line)) {
                found.push(test.clone());
            }
        }
        found.sort();
        found.dedup();
        applies.then_some(found)
    }
}
