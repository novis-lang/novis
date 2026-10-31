//! `nvs pkg audit`: every advisory the locked packages are under, and the smallest version that
//! clears each (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`).
//!
//! **Audit reads the lock, never the network's idea of the graph.** For each registry a locked
//! package comes from, it reads the registry's advisory feed and checks it against the whole log
//! ([`Seen::advisories`]), so a feed that hides an advisory is an error, not a clean report. Each
//! locked version under a retraction, a withhold or inside an advisory's [`Range`] is reported
//! with its reason and its fix: the smallest version above it that a command may choose
//! ([`minimum::choosable`]), is under no advisory, and is at least the retraction's `fixed_in` or
//! the range's end. An advisory is reported with its `id` and its summary as the reason, and a
//! range with no end has no fix. An unlisted version is not an advisory and is not reported. A git dependency comes from no registry and is not audited.
//!
//! **The second copy of the checkpoint** that a registry's root file names is compared with the
//! log the file host serves ([`Seen::compare_copy`]). Two logs that disagree, and a copy that
//! cannot be read, stop the command with an error. A registry that names no copy gets one note,
//! and audit goes on.
//!
//! **An autoload prefix no grant line matches is a warning**
//! (`rule:security/an-unmatched-namespace-holds-the-application`). Every `.nvs` file under the
//! application's folder that starts a program is walked the way an editor finds its lenders
//! ([`Lender::walk`]), and each prefix its `autoload` declarations map, discovered ones included,
//! is looked up in the application's `[grants]`. A prefix that matches no line holds the
//! application's authority, and audit names it with its roots. Code `nvs pkg fetch` placed always
//! has a line, so what is named is code a person copied in, or the application's own source
//! tree, which has the same shape. A warning never changes the exit status.
//!
//! The command exits non-zero while any locked version is under an advisory, so CI can run it.
//! `--json` prints the same report as one JSON object.
//!
//! Cost: per registry, one feed, one entry bundle per 256 records of its log, one checkpoint
//! copy and one consistency proof, and one index file per package under an advisory. For the
//! warning, one read of every `.nvs` file under the folder and one name resolution per file that
//! starts a program. Nothing here runs on the request path.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::ExitCode;

use nvs_config::grants::Grants;
use nvs_diagnostics::SourceMap;
use nvs_hir::Lender;
use serde::Serialize;

use super::advisory::Advisory;
use super::entry::Entry;
use super::fetch::{self, App, Failed};
use super::index::{Index, fault};
use super::lock::Locked;
use super::manifest::{Range, Version};
use super::minimum;
use super::proof::Seen;
use super::resolve::line;

/// One locked version under an advisory.
#[derive(Serialize)]
struct Found {
    /// The package's namespace.
    package: String,
    version: String,
    /// `retract`, `withhold` or `advisory`.
    kind: &'static str,
    /// The advisory's id, `acme/http-1`, for kind `advisory` alone.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    reason: String,
    /// The number of the advisory's record in the registry's log.
    log: u64,
    /// The version the retraction names as its fix, or the end of the advisory's range.
    fixed_in: Option<String>,
    /// The smallest version that clears the advisory, or `None` when no version does yet.
    bump: Option<String>,
}

/// One registry the audit read.
#[derive(Serialize)]
struct Checked {
    url: String,
    /// The size of the log that was checked.
    size: u64,
    /// The URL of the second copy of the checkpoint, when the registry names one.
    copy: Option<String>,
    /// The size of the log the copy names.
    copy_size: Option<u64>,
}

/// One autoload prefix that no grant line matches.
#[derive(Serialize)]
struct Unmatched {
    /// The namespace prefix, as `Acme\Legacy`.
    prefix: String,
    /// Its roots in probe order, relative to the application's folder with `/` where they are
    /// under it.
    roots: Vec<String>,
}

/// The whole report, as `--json` prints it.
#[derive(Serialize)]
struct Report {
    advisories: Vec<Found>,
    registries: Vec<Checked>,
    /// The warnings: each autoload prefix no grant line matches, sorted.
    unmatched: Vec<Unmatched>,
}

/// `nvs pkg audit`, in the working directory. With `json`, the report is one JSON object.
pub(crate) fn run(json: bool) -> ExitCode {
    match auditing() {
        Ok(report) => {
            let clean = report.advisories.is_empty();
            if json {
                let text = serde_json::to_string_pretty(&report).expect("a report serializes");
                println!("{text}");
            } else {
                print(&report);
                warn(&report.unmatched);
            }
            if clean {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(failed) => fetch::exit(Err(failed)),
    }
}

/// The report of every locked package, returning what stopped it.
fn auditing() -> Result<Report, Failed> {
    let app = App::open()?;
    let Some(lock) = &app.old else {
        return Err(fault(format!(
            "there is no `{}` beside `{}`. Run `nvs pkg fetch` first.",
            super::lock::FILE,
            app.path.display()
        ))
        .into());
    };
    let now = minimum::now();
    let mut registries = app.registries(&app.manifest);
    let mut feeds: BTreeMap<String, Vec<Advisory>> = BTreeMap::new();
    let mut report = Report {
        advisories: Vec::new(),
        registries: Vec::new(),
        unmatched: unmatched(&app.dir, &app.manifest.grants),
    };
    for locked in &lock.packages {
        if locked.source.starts_with("git+") {
            continue;
        }
        let url = app.manifest.registry(locked.name.vendor()).to_owned();
        if !feeds.contains_key(&url) {
            let (registry, seen) = registries.with_log(&locked.name)?;
            let checked = check(registry, seen)?;
            feeds.insert(url.clone(), seen.advisories(registry)?);
            report.registries.push(checked);
        }
        let advisories = &feeds[&url];
        let slug = locked.name.slug();
        let under: Vec<&Advisory> = advisories
            .iter()
            .filter(|advisory| names(&advisory.entry, &slug, locked.version))
            .collect();
        if under.is_empty() {
            continue;
        }
        let index = registries.index(&locked.name)?;
        for advisory in under {
            report
                .advisories
                .push(found(locked, advisory, index, advisories, now));
        }
    }
    Ok(report)
}

/// The registry and its log as the report names them, once its second copy of the checkpoint is
/// compared. A registry with no copy gets one note on standard error.
fn check(registry: &super::index::Registry, seen: &Seen) -> Result<Checked, Failed> {
    let copy_size = seen.compare_copy(registry)?;
    if copy_size.is_none() {
        eprintln!(
            "note: the registry `{}` names no second copy of its checkpoint, so audit cannot \
             compare one",
            registry.url
        );
    }
    Ok(Checked {
        url: registry.url.clone(),
        size: seen.checkpoint().size(),
        copy: registry.checkpoint_copy().map(str::to_owned),
        copy_size,
    })
}

/// Whether `entry` is an advisory against `version` of the package `slug`.
fn names(entry: &Entry, slug: &str, version: Version) -> bool {
    match entry {
        Entry::Retract {
            package,
            version: named,
            ..
        }
        | Entry::Withhold {
            package,
            version: named,
            ..
        } => package == slug && named.parse::<Version>().is_ok_and(|named| named == version),
        Entry::Advisory {
            package, versions, ..
        } => {
            package == slug
                && versions
                    .parse::<Range>()
                    .is_ok_and(|range| range.contains(version))
        }
        _ => false,
    }
}

/// The report line of `advisory` against `locked`, with the smallest version of `index` that
/// clears it at the Unix time `now`. `all` is every advisory of the registry.
fn found(
    locked: &Locked,
    advisory: &Advisory,
    index: Option<&Index>,
    all: &[Advisory],
    now: u64,
) -> Found {
    let (kind, id, reason, fixed_in) = match &advisory.entry {
        Entry::Retract {
            reason, fixed_in, ..
        } => ("retract", None, reason.clone(), fixed_in.clone()),
        Entry::Withhold { reason, .. } => ("withhold", None, reason.clone(), None),
        Entry::Advisory {
            id,
            versions,
            summary,
            ..
        } => (
            "advisory",
            Some(id.clone()),
            summary.clone(),
            versions
                .parse::<Range>()
                .ok()
                .and_then(|range| range.until)
                .map(|until| until.to_string()),
        ),
        _ => unreachable!("a feed lists retractions, withholds and advisories only"),
    };
    let floor = fixed_in
        .as_deref()
        .and_then(|fix| fix.parse::<Version>().ok());
    let slug = locked.name.slug();
    let bump = index.and_then(|index| {
        index
            .releases
            .iter()
            .find(|release| {
                release.version > locked.version
                    && floor.is_none_or(|floor| release.version >= floor)
                    && minimum::choosable(release, now)
                    && !all
                        .iter()
                        .any(|other| names(&other.entry, &slug, release.version))
            })
            .map(|release| release.version.to_string())
    });
    Found {
        package: locked.name.to_string(),
        version: locked.version.to_string(),
        kind,
        id,
        reason,
        log: advisory.log,
        fixed_in,
        bump,
    }
}

/// Each autoload prefix that a program under `dir` maps and no line of `grants` matches, sorted
/// by prefix, each once.
fn unmatched(dir: &Path, grants: &Grants) -> Vec<Unmatched> {
    let base = nvs_hir::requires::canonicalize(dir).unwrap_or_else(|| dir.to_path_buf());
    let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for source in nvs_hir::lenders::sources(dir) {
        let Some(lender) = Lender::walk(SourceMap::new(), &source) else {
            continue;
        };
        for (prefix, roots) in lender.map().prefixes() {
            if grants.lookup(&prefix).is_some() {
                continue;
            }
            let shown = found.entry(prefix).or_default();
            shown.extend(roots.iter().map(|root| match root.strip_prefix(&base) {
                Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                Err(_) => nvs_diagnostics::plain_path(root).display().to_string(),
            }));
        }
    }
    found
        .into_iter()
        .map(|(prefix, roots)| Unmatched {
            prefix,
            roots: roots.into_iter().collect(),
        })
        .collect()
}

/// Each of `unmatched` as a warning on standard error.
fn warn(unmatched: &[Unmatched]) {
    for line in unmatched {
        eprintln!(
            "warning: no line in `[grants]` matches `{}`, loaded from `{}`. Its code has every \
             permission your application has.",
            line.prefix,
            line.roots.join("`, `")
        );
        eprintln!(
            "  If it is not your application's own code, add a line for `{}\\*` to `[grants]` \
             in `package.toml`, with the permissions it needs.",
            line.prefix
        );
    }
}

/// `report` as text on standard output.
fn print(report: &Report) {
    for found in &report.advisories {
        let what = match (found.kind, &found.id) {
            ("retract", _) => "is retracted".to_owned(),
            ("advisory", Some(id)) => format!("has the advisory `{id}`"),
            _ => "is withheld by the registry".to_owned(),
        };
        println!(
            "`{}` {} {what}: {}",
            found.package, found.version, found.reason
        );
        match &found.bump {
            Some(bump) => {
                let major = match (bump.parse::<Version>(), found.version.parse::<Version>()) {
                    (Ok(bump), Ok(locked)) if line(bump) != line(locked) => {
                        " This is a new major version."
                    }
                    _ => "",
                };
                println!("  Update to {bump} or newer.{major}");
            }
            None => println!("  No released version fixes it yet."),
        }
    }
    for checked in &report.registries {
        if let Some(copy) = &checked.copy {
            println!(
                "The log of `{}` matches its second copy at `{copy}`.",
                checked.url
            );
        }
    }
    match report.advisories.len() {
        0 => println!("No locked package has an advisory."),
        1 => println!("1 locked version has an advisory."),
        n => println!("{n} advisories apply to locked versions."),
    }
}
