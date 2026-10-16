//! The extension's target table and the release matrix name one set of archives.
//!
//! `editors/vscode/src/install.ts` decides which archive a machine needs, and
//! `.github/workflows/release.yml`'s `build` matrix decides which archives
//! exist. Nothing links the two: a target the workflow gains is one the
//! extension never offers, and a row the extension claims that no leg builds is
//! a 404 the user meets only after asking for an install — which is the worst
//! possible moment, because it is the moment the extension promised to fix the
//! missing binary (`rule:ide/the-extension-guides-an-install-and-never-bundles-one`).
//!
//! It is Rust rather than a case in the extension's own headless suite for one
//! reason: reading the workflow means a YAML parser, and a new dependency is
//! exactly what `editors/vscode/test/contributions/contributions.test.ts`'s
//! allowlist refuses. Both files are read as text instead, which they can be —
//! the matrix is a flat list of scalars, and `install.ts` says in its own header
//! that each `TARGETS` row is one line with its fields in declaration order
//! because this test parses them that way.
//!
//! `tests/extension_reference.rs` is the same shape over a different pair.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// Read a file this test knows must exist, or fail naming it.
fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

/// How many spaces `line` opens with.
fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// One archive, as either side of this pin describes it.
#[derive(Debug, Default, PartialEq, Eq)]
struct Archive {
    /// The archive format: `tar.gz` or `zip`.
    format: String,
    /// Whether this is the musl-linked leg rather than a glibc one.
    musl: bool,
}

/// One row of `install.ts`'s `TARGETS`, read as the text it is.
#[derive(Debug)]
struct Row {
    /// The archive name the row claims, which is the matrix's `name`.
    name: String,
    /// The `process.platform` value the row answers for.
    platform: String,
    /// The `process.arch` value the row answers for.
    arch: String,
    /// The archive format the row claims.
    archive: String,
    /// The row's libc, where it has one.
    libc: Option<String>,
}

/// The lines of the `build` job's `matrix.include` list.
///
/// Scoped to that job because `docker` further down the file has an `include`
/// of its own, and the two matrices have nothing to do with each other.
fn matrix_lines(workflow: &str) -> Vec<&str> {
    let mut lines = workflow.lines();
    let job = lines.by_ref().find(|line| line.trim_end() == "  build:");
    assert!(job.is_some(), "the release workflow has no `build` job");

    let body: Vec<&str> = lines
        .take_while(|line| line.trim().is_empty() || indent(line) > 2)
        .collect();
    let mut inside = body
        .iter()
        .skip_while(|line| line.trim() != "include:")
        .copied();
    let header = inside.next().expect("the `build` job has a matrix include");
    let depth = indent(header);

    inside
        .take_while(|line| line.trim().is_empty() || indent(line) > depth)
        .filter(|line| !line.trim().starts_with('#'))
        .collect()
}

/// Every archive the release workflow builds, by name.
fn release_matrix() -> BTreeMap<String, Archive> {
    let path = nvs_repo::path(".github/workflows/release.yml");
    let workflow = read(&path);

    let mut built: BTreeMap<String, Archive> = BTreeMap::new();
    let mut current = String::new();
    for line in matrix_lines(&workflow) {
        let entry = line.trim();
        if let Some(name) = entry.strip_prefix("- name: ") {
            current = name.trim().to_owned();
            built.insert(current.clone(), Archive::default());
        } else if let Some(format) = entry.strip_prefix("archive: ") {
            let archive = built
                .get_mut(&current)
                .unwrap_or_else(|| panic!("`archive: {format}` before any `- name:`"));
            archive.format = format.trim().to_owned();
        } else if entry == "musl: true" {
            let archive = built
                .get_mut(&current)
                .unwrap_or_else(|| panic!("`musl: true` before any `- name:`"));
            archive.musl = true;
        }
    }

    assert!(
        !built.is_empty(),
        "{} parsed to no matrix rows at all",
        path.display()
    );
    built
}

/// Every row of the extension's target table, in the order it declares them.
fn target_table() -> Vec<Row> {
    let path = nvs_repo::path("editors/vscode/src/install.ts");
    let source = read(&path);

    let rows: Vec<Row> = source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("{ name: \""))
        .map(|line| {
            let fields: BTreeMap<&str, &str> = line
                .trim_start_matches('{')
                .trim_end_matches(',')
                .trim_end_matches('}')
                .split(',')
                .filter_map(|field| field.trim().split_once(": "))
                .map(|(key, value)| (key.trim(), value.trim().trim_matches('"')))
                .collect();
            let field = |key: &str| {
                (*fields
                    .get(key)
                    .unwrap_or_else(|| panic!("a TARGETS row has no `{key}`: {line}")))
                .to_owned()
            };
            Row {
                name: field("name"),
                platform: field("platform"),
                arch: field("arch"),
                archive: field("archive"),
                libc: fields.get("libc").map(|libc| (*libc).to_owned()),
            }
        })
        .collect();

    assert!(
        !rows.is_empty(),
        "{} parsed to no TARGETS rows; the row shape this test reads has changed",
        path.display()
    );
    rows
}

#[test]
fn the_extension_target_table_matches_the_release_matrix() {
    let built = release_matrix();
    let claimed: BTreeMap<String, Archive> = target_table()
        .into_iter()
        .map(|row| {
            (
                row.name,
                Archive {
                    format: row.archive,
                    musl: row.libc.as_deref() == Some("musl"),
                },
            )
        })
        .collect();

    assert_eq!(
        claimed.keys().collect::<Vec<_>>(),
        built.keys().collect::<Vec<_>>(),
        "install.ts's TARGETS and the release matrix name different archives"
    );
    for (name, archive) in &claimed {
        assert_eq!(
            *archive, built[name],
            "{name} is {archive:?} in install.ts and {:?} in the release matrix",
            built[name]
        );
    }
}

#[test]
fn every_claimed_platform_pair_resolves_to_an_archive_the_workflow_builds() {
    let built = release_matrix();
    let rows = target_table();

    let mut pairs: BTreeSet<(String, String, Option<String>)> = BTreeSet::new();
    let mut claimed: BTreeSet<String> = BTreeSet::new();
    for row in &rows {
        assert!(
            matches!(row.platform.as_str(), "linux" | "win32" | "darwin"),
            "{} is not a process.platform value: {}",
            row.platform,
            row.name
        );
        assert!(
            matches!(row.arch.as_str(), "x64" | "arm64"),
            "{} is not a process.arch value: {}",
            row.arch,
            row.name
        );
        // `currentLibc` answers only on Linux, so a libc anywhere else is a row
        // nothing can ever resolve to, and a Linux row without one is the same.
        assert_eq!(
            row.libc.is_some(),
            row.platform == "linux",
            "{}'s libc and platform cannot both be true of one machine",
            row.name
        );
        assert!(
            built.contains_key(&row.name),
            "{} {} claims {}, which the release workflow does not build",
            row.platform,
            row.arch,
            row.name
        );
        assert!(
            pairs.insert((row.platform.clone(), row.arch.clone(), row.libc.clone())),
            "two rows claim {} {} {:?}, so which archive answers depends on their order",
            row.platform,
            row.arch,
            row.libc
        );
        claimed.insert(row.name.clone());
    }

    let unclaimed: Vec<&String> = built
        .keys()
        .filter(|name| !claimed.contains(*name))
        .collect();
    assert!(
        unclaimed.is_empty(),
        "the workflow builds {unclaimed:?}, which no platform pair resolves to"
    );
}
