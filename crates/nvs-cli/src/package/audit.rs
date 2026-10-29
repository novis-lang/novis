//! `nvs pkg audit`: every advisory the locked packages are under, and the smallest version that
//! clears each (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`).
//!
//! **Audit reads the lock, never the network's idea of the graph.** For each registry a locked
//! package comes from, it reads the registry's advisory feed and checks it against the whole log
//! ([`Seen::advisories`]), so a feed that hides an advisory is an error, not a clean report. Each
//! locked version under a retraction or a withhold is reported with its reason and its fix: the
//! smallest version above it that a command may choose ([`minimum::choosable`]), is under no
//! advisory, and is at least the retraction's `fixed_in`. An unlisted version is not an advisory
//! and is not reported. A git dependency comes from no registry and is not audited.
//!
//! **The second copy of the checkpoint** that a registry's root file names is compared with the
//! log the file host serves ([`Seen::compare_copy`]). Two logs that disagree, and a copy that
//! cannot be read, stop the command with an error. A registry that names no copy gets one note,
//! and audit goes on.
//!
//! The command exits non-zero while any locked version is under an advisory, so CI can run it.
//! `--json` prints the same report as one JSON object.
//!
//! Cost: per registry, one feed, one entry bundle per 256 records of its log, one checkpoint
//! copy and one consistency proof, and one index file per package under an advisory. Nothing
//! here runs on the request path.

use std::collections::BTreeMap;
use std::process::ExitCode;

use serde::Serialize;

use super::advisory::Advisory;
use super::entry::Entry;
use super::fetch::{self, App, Failed};
use super::index::{Index, fault};
use super::lock::Locked;
use super::manifest::Version;
use super::minimum;
use super::proof::Seen;
use super::resolve::line;

/// One locked version under an advisory.
#[derive(Serialize)]
struct Found {
    /// The package's namespace.
    package: String,
    version: String,
    /// `retract` or `withhold`.
    kind: &'static str,
    reason: String,
    /// The number of the advisory's record in the registry's log.
    log: u64,
    /// The version the retraction names as its fix.
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

/// The whole report, as `--json` prints it.
#[derive(Serialize)]
struct Report {
    advisories: Vec<Found>,
    registries: Vec<Checked>,
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
    let (kind, reason, fixed_in) = match &advisory.entry {
        Entry::Retract {
            reason, fixed_in, ..
        } => ("retract", reason.clone(), fixed_in.clone()),
        Entry::Withhold { reason, .. } => ("withhold", reason.clone(), None),
        _ => unreachable!("a feed lists retractions and withholds only"),
    };
    let floor = fixed_in.as_deref().and_then(|fix| fix.parse::<Version>().ok());
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
        reason,
        log: advisory.log,
        fixed_in,
        bump,
    }
}

/// `report` as text on standard output.
fn print(report: &Report) {
    for found in &report.advisories {
        let what = if found.kind == "retract" {
            "is retracted"
        } else {
            "is withheld by the registry"
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
