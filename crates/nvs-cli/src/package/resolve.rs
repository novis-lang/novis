//! Resolution: every package the application's graph needs, at the highest minimum any package
//! in the graph asks for (`rule:packaging/resolution-takes-the-highest-minimum`).
//!
//! **The walk visits every version some package asked for**, starting at the application's
//! `[dependencies]`, and reads each visited version's dependencies from its index. The version
//! selected for a package is the highest one visited. There is no solver and no backtracking, so
//! resolution always ends with one answer, and the same graph gives the same answer on every run
//! and every machine. Versions are visited in name and version order, so even a fault names the
//! same package each time.
//!
//! **A major is crossed when a package is asked for at an older breaking line than the one
//! selected**, such as `1.4.0` asked for and `2.0.0` selected, or `0.3.1` and `0.4.0`
//! (`rule:packaging/below-1-0-the-breaking-slot-moves-left`). Each crossing is reported with the
//! packages that asked for the older version, and resolution goes on: a break that matters is a
//! compile error.
//!
//! **A selected version's state decides whether it can be used.** A pending or unlisted version
//! is used, because somebody named it as a minimum. A retracted one is a fault that names its
//! fix, unless the lock already has that version. A withheld one is always a fault, because its
//! bytes are not served.
//!
//! **Each vendor's packages come from one registry**, the one the application's `package.toml`
//! names for the vendor (`rule:packaging/a-vendor-resolves-from-one-registry`), and a package
//! that registry does not list is a fault. No second registry is asked.
//!
//! **A root git dependency is the version its commit declares**, cloned and packed by [`git`],
//! and that version is selected whatever any other package asks for. Its own dependencies join
//! the walk, and a git dependency among them is a fault that names it
//! (`rule:packaging/a-git-dependency-is-root-only`). Only the application's file can name one,
//! because an index lists registry dependencies alone.
//!
//! Cost: each git dependency's archive, at most [`super::pack::MAX_ARCHIVE_BYTES`], one index file read per package, held until resolution ends, and the visited set.
//! Nothing here runs on the request path.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use super::git::{self, Limits, Outbound};
use super::index::{Capabilities, Fault, Index, Registry, Release, State, fault};
use super::lock::Lock;
use super::manifest::{Manifest, Name, Source, Version};

/// Who asked for a version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Asker {
    /// The application's own `package.toml`.
    Root,
    /// A package of the graph, at the version that asked.
    Package(Name, Version),
}

impl fmt::Display for Asker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root => f.write_str("package.toml"),
            Self::Package(name, version) => write!(f, "`{name}` {version}"),
        }
    }
}

/// One package of the resolved graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Selected {
    pub(crate) name: Name,
    /// The selected version, with its digest, state and dependencies.
    pub(crate) release: Release,
    /// The URL of the registry it comes from, as `package.toml` names it.
    pub(crate) source: String,
    /// The package's owners, as its index lists them.
    pub(crate) owners: Vec<String>,
}

/// A package asked for at an older breaking line than the one selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Crossing {
    pub(crate) name: Name,
    pub(crate) selected: Version,
    pub(crate) older: Version,
    /// Every package that asked for `older`.
    pub(crate) asked_by: Vec<Asker>,
}

impl fmt::Display for Crossing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` {} is selected, and {} asks for {}",
            self.name,
            self.selected,
            self.asked_by
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" and "),
            self.older
        )
    }
}

/// The resolved graph: every package in name order, and every major crossed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Graph {
    pub(crate) packages: Vec<Selected>,
    pub(crate) crossings: Vec<Crossing>,
}

/// The registries the application's `package.toml` names, opened when first asked, and every
/// index read from them.
pub(crate) struct Registries<'a> {
    manifest: &'a Manifest,
    /// The directory a relative `file:` URL resolves against: the manifest's own.
    base: PathBuf,
    /// Where each registry's root file is kept.
    keep: Option<PathBuf>,
    open: BTreeMap<String, Registry>,
    indexes: BTreeMap<String, Option<Index>>,
    /// Where a git dependency is written out while it is packed.
    scratch: PathBuf,
    /// The limits a git dependency is cloned inside.
    limits: Limits,
    /// The addresses an `https:` git dependency may be cloned from.
    outbound: Outbound,
    /// Each git dependency's archive, by slug.
    cloned: BTreeMap<String, Vec<u8>>,
}

impl<'a> Registries<'a> {
    /// The registries of `manifest`, with each root file kept in `keep`.
    pub(crate) fn new(manifest: &'a Manifest, keep: Option<&Path>) -> Self {
        Self {
            manifest,
            base: manifest
                .path
                .parent()
                .map_or_else(PathBuf::new, Path::to_path_buf),
            keep: keep.map(Path::to_path_buf),
            open: BTreeMap::new(),
            indexes: BTreeMap::new(),
            scratch: std::env::temp_dir(),
            limits: git::LIMITS,
            outbound: Outbound::default(),
            cloned: BTreeMap::new(),
        }
    }

    /// These registries, with each `https:` git dependency cloned only where `outbound` allows.
    pub(crate) fn reaching(mut self, outbound: Outbound) -> Self {
        self.outbound = outbound;
        self
    }

    /// These registries, with each git dependency written out under `scratch` and cloned inside
    /// `limits`.
    pub(crate) fn clones_in(mut self, scratch: &Path, limits: Limits) -> Self {
        self.scratch = scratch.to_path_buf();
        self.limits = limits;
        self
    }

    /// The archive of the git dependency `name`, once [`resolve`] has cloned it.
    pub(crate) fn cloned(&self, name: &Name) -> Option<&[u8]> {
        self.cloned.get(&name.slug()).map(Vec::as_slice)
    }

    /// The registry `name`'s vendor resolves from, opened on the first call.
    pub(crate) fn registry(&mut self, name: &Name) -> Result<&Registry, Fault> {
        let url = self.manifest.registry(name.vendor()).to_owned();
        if !self.open.contains_key(&url) {
            let registry = Registry::open(&url, &self.base, self.keep.as_deref())?;
            if registry.from_kept {
                eprintln!(
                    "note: the registry `{url}` did not answer, so its file hosts are read from \
                     the copy of `{}` kept from the last run",
                    super::index::ROOT_FILE
                );
            }
            self.open.insert(url.clone(), registry);
        }
        Ok(&self.open[&url])
    }

    /// The index of `name`, read once, or `None` when its registry does not list it.
    pub(crate) fn index(&mut self, name: &Name) -> Result<Option<&Index>, Fault> {
        let slug = name.slug();
        if !self.indexes.contains_key(&slug) {
            let index = self.registry(name)?.index(name)?;
            self.indexes.insert(slug.clone(), index);
        }
        Ok(self.indexes[&slug].as_ref())
    }
}

/// The breaking line `version` is on: the leftmost part that is not zero, which is the part a
/// break moves (`rule:packaging/below-1-0-the-breaking-slot-moves-left`).
pub(crate) fn line(version: Version) -> (u64, u64, u64) {
    match version {
        Version {
            major: 0,
            minor: 0,
            patch,
        } => (0, 0, patch),
        Version {
            major: 0, minor, ..
        } => (0, minor, 0),
        Version { major, .. } => (major, 0, 0),
    }
}

/// The graph `manifest`'s dependencies need. `lock` is the application's lock, if it has one: a
/// retracted version it already names may stay selected.
///
/// # Errors
///
/// A registry that cannot be read, a package its vendor's registry does not list, a version its
/// index does not list, a git dependency that does not clone or names a git dependency itself,
/// and a selected version that is retracted or withheld.
pub(crate) fn resolve(
    manifest: &Manifest,
    registries: &mut Registries<'_>,
    lock: Option<&Lock>,
) -> Result<Graph, Fault> {
    let mut asked = Asked::new();
    let mut queue: BTreeSet<(String, Version)> = BTreeSet::new();
    let mut pinned: BTreeMap<String, Selected> = BTreeMap::new();
    for dependency in &manifest.dependencies {
        match &dependency.source {
            Source::Registry { minimum } => {
                ask(
                    &mut asked,
                    &mut queue,
                    &dependency.name,
                    *minimum,
                    Asker::Root,
                );
            }
            Source::Git { url, rev } => {
                let selected = cloned(registries, &dependency.name, url, rev)?;
                let version = selected.release.version;
                ask(
                    &mut asked,
                    &mut queue,
                    &dependency.name,
                    version,
                    Asker::Root,
                );
                let asker = Asker::Package(dependency.name.clone(), version);
                for (name, minimum) in &selected.release.dependencies {
                    ask(&mut asked, &mut queue, name, *minimum, asker.clone());
                }
                pinned.insert(dependency.name.slug(), selected);
            }
        }
    }
    while let Some((slug, version)) = queue.pop_first() {
        if pinned.contains_key(&slug) {
            continue;
        }
        let (name, versions) = &asked[&slug];
        let name = name.clone();
        let release = find(registries, &name, version, &versions[&version])?.0;
        let asker = Asker::Package(name, version);
        for (dependency, minimum) in &release.dependencies {
            ask(&mut asked, &mut queue, dependency, *minimum, asker.clone());
        }
    }
    let mut packages = Vec::with_capacity(asked.len());
    let mut crossings = Vec::new();
    for (slug, (name, versions)) in &asked {
        let pin = pinned.remove(slug);
        let selected = match &pin {
            Some(pin) => pin.release.version,
            None => match versions.last_key_value() {
                Some((&selected, _)) => selected,
                None => continue,
            },
        };
        for (&older, askers) in versions {
            if line(older) != line(selected)
                && !crossings
                    .iter()
                    .any(|c: &Crossing| c.name == *name && line(c.older) == line(older))
            {
                crossings.push(Crossing {
                    name: name.clone(),
                    selected,
                    older,
                    asked_by: askers.iter().cloned().collect(),
                });
            }
        }
        if let Some(pin) = pin {
            packages.push(pin);
            continue;
        }
        let (release, owners) = find(registries, name, selected, &versions[&selected])?;
        usable(name, &release, lock)?;
        let source = registries.registry(name)?.url.clone();
        packages.push(Selected {
            name: name.clone(),
            release,
            source,
            owners,
        });
    }
    Ok(Graph {
        packages,
        crossings,
    })
}

/// Every version asked for, by slug, with the package's name and who asked for each version.
type Asked = BTreeMap<String, (Name, BTreeMap<Version, BTreeSet<Asker>>)>;

/// `asker`'s request for `name` at `version` recorded, and the version queued to visit if no
/// package asked for it before.
fn ask(
    asked: &mut Asked,
    queue: &mut BTreeSet<(String, Version)>,
    name: &Name,
    version: Version,
    asker: Asker,
) {
    let (_, versions) = asked
        .entry(name.slug())
        .or_insert_with(|| (name.clone(), BTreeMap::new()));
    if !versions.contains_key(&version) {
        queue.insert((name.slug(), version));
    }
    versions.entry(version).or_default().insert(asker);
}

/// The root git dependency `name`, cloned from `url` at `rev` and packed, with its archive kept
/// in `registries`.
fn cloned(
    registries: &mut Registries<'_>,
    name: &Name,
    url: &str,
    rev: &str,
) -> Result<Selected, Fault> {
    let cloned = git::clone(
        url,
        rev,
        &registries.base,
        &registries.scratch,
        registries.limits,
        &registries.outbound,
    )?;
    let package = &cloned.manifest;
    if package.name.as_ref().map(Name::slug) != Some(name.slug()) {
        return Err(fault(format!(
            "`{url}` at `{rev}` is the package `{}`, and `package.toml` names it `{name}`",
            package
                .name
                .as_ref()
                .map_or_else(|| "(none)".to_owned(), ToString::to_string)
        )));
    }
    let Some(version) = package.version else {
        return Err(fault(format!(
            "`{name}` at `{url}` has no `[package] version`"
        )));
    };
    let mut dependencies = Vec::new();
    for dependency in &package.dependencies {
        match &dependency.source {
            Source::Registry { minimum } => dependencies.push((dependency.name.clone(), *minimum)),
            Source::Git { .. } => {
                return Err(fault(format!(
                    "`{name}` {version} depends on `{}` from git. Only the application's own \
                     `package.toml` may name a git dependency.",
                    dependency.name
                )));
            }
        }
    }
    let capabilities = package
        .package
        .as_ref()
        .map_or_else(Capabilities::default, |p| Capabilities {
            required: p
                .required()
                .iter()
                .map(|cap| cap.name().to_owned())
                .collect(),
            optional: p
                .optional()
                .iter()
                .map(|cap| cap.name().to_owned())
                .collect(),
        });
    registries.cloned.insert(name.slug(), cloned.archive);
    Ok(Selected {
        name: name.clone(),
        release: Release {
            version,
            digest: cloned.digest,
            state: State::Released,
            dependencies,
            capabilities,
            repository: url.to_owned(),
            commit: rev.to_owned(),
        },
        source: format!("git+{url}"),
        owners: Vec::new(),
    })
}

/// The release `name` at `version`, and the package's owners, which `askers` asked for.
fn find(
    registries: &mut Registries<'_>,
    name: &Name,
    version: Version,
    askers: &BTreeSet<Asker>,
) -> Result<(Release, Vec<String>), Fault> {
    let asked_by = askers
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" and ");
    let url = registries.registry(name)?.url.clone();
    let Some(index) = registries.index(name)? else {
        return Err(fault(format!(
            "the registry `{url}` has no package `{name}`, which {asked_by} needs"
        )));
    };
    let Some(release) = index.release(version) else {
        return Err(fault(format!(
            "the registry `{url}` has no version {version} of `{name}`, which {asked_by} needs"
        )));
    };
    Ok((release.clone(), index.owners.clone()))
}

/// A fault when the selected `release` of `name` may not be used.
fn usable(name: &Name, release: &Release, lock: Option<&Lock>) -> Result<(), Fault> {
    let version = release.version;
    match &release.state {
        State::Retracted { reason, fixed_in } => {
            if lock.is_some_and(|lock| lock.has(name, version)) {
                return Ok(());
            }
            let fix = fixed_in.map_or_else(
                || "No fixed version is named yet.".to_owned(),
                |fixed| format!("Raise the minimum to {fixed}, which fixes it."),
            );
            Err(fault(format!(
                "`{name}` {version} is retracted: {reason}. {fix}"
            )))
        }
        State::Withheld { reason } => Err(fault(format!(
            "`{name}` {version} is withheld by the registry: {reason}. Its files are not served."
        ))),
        State::Pending { .. } | State::Released | State::Unlisted => Ok(()),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use nvs_diagnostics::SourceMap;

    use super::super::fixture::Fixture;
    use super::super::manifest;
    use super::*;

    /// The application manifest with `dependencies`, whose `Acme` vendor resolves from the
    /// fixture.
    pub(crate) fn application(fixture: &Fixture, dependencies: &[(&str, &str)]) -> Manifest {
        let mut text = String::from("[dependencies]\n");
        for (name, minimum) in dependencies {
            text.push_str(&format!("'{name}' = \"{minimum}\"\n"));
        }
        text.push_str(&format!(
            "\n[registries]\nAcme = {{ url = '{}' }}\n",
            fixture.url()
        ));
        let path = fixture.root().join("app/package.toml");
        manifest::parse(&mut SourceMap::new(), &path, &text).expect("the test's manifest reads")
    }

    fn versions(graph: &Graph) -> Vec<String> {
        graph
            .packages
            .iter()
            .map(|selected| format!("{} {}", selected.name, selected.release.version))
            .collect()
    }

    /// Two packages ask for different minimums of a third, the higher is selected, and a newer
    /// version nobody asked for is not.
    #[test]
    fn the_highest_minimum_is_selected() {
        let fixture = Fixture::new("resolve-highest");
        fixture.publish("Acme\\Log", "1.2.0", &[], "\"released\"");
        fixture.publish("Acme\\Log", "1.5.0", &[], "\"released\"");
        fixture.publish("Acme\\Log", "1.9.0", &[], "\"released\"");
        fixture.publish(
            "Acme\\Http",
            "2.0.0",
            &[("Acme\\Log", "1.5.0")],
            "\"released\"",
        );
        fixture.publish(
            "Acme\\Db",
            "1.0.0",
            &[("Acme\\Log", "1.2.0")],
            "\"released\"",
        );
        let app = application(&fixture, &[("Acme\\Http", "2.0.0"), ("Acme\\Db", "1.0.0")]);
        let graph = resolve(&app, &mut Registries::new(&app, None), None).expect("it resolves");
        assert_eq!(
            versions(&graph),
            ["Acme\\Db 1.0.0", "Acme\\Http 2.0.0", "Acme\\Log 1.5.0"]
        );
        assert!(graph.crossings.is_empty());
    }

    /// The application manifest with the git dependency `Acme\Git` at `url` and `rev`, whose
    /// `Acme` vendor resolves from the fixture.
    fn with_git(fixture: &Fixture, url: &str, rev: &str) -> Manifest {
        let text = format!(
            "[dependencies]\n'Acme\\Git' = {{ git = '{url}', rev = '{rev}' }}\n\n\
             [registries]\nAcme = {{ url = '{}' }}\n",
            fixture.url()
        );
        let path = fixture.root().join("app/package.toml");
        manifest::parse(&mut SourceMap::new(), &path, &text).expect("the test's manifest reads")
    }

    /// A root git dependency is the version its commit declares, its registry dependencies join
    /// the graph, and its archive is kept.
    #[test]
    fn a_root_git_dependency_is_cloned_and_selected() {
        let fixture = Fixture::new("resolve-git");
        fixture.publish("Acme\\Log", "1.2.0", &[], "\"released\"");
        let repo = fixture.root().join("repo");
        let rev = super::super::git::tests::package(
            &repo,
            &[(
                "package.toml",
                b"[package]\nname = 'Acme\\Git'\nversion = \"1.0.0\"\n\n\
                  [dependencies]\n'Acme\\Log' = \"1.2.0\"\n",
            )],
        );
        let app = with_git(&fixture, &format!("file:{}", repo.display()), &rev);
        let mut registries = Registries::new(&app, None).clones_in(fixture.root(), git::LIMITS);
        let graph = resolve(&app, &mut registries, None).expect("it resolves");
        assert_eq!(versions(&graph), ["Acme\\Git 1.0.0", "Acme\\Log 1.2.0"]);
        let selected = &graph.packages[0];
        assert_eq!(selected.release.commit, rev);
        assert!(selected.source.starts_with("git+file:"));
        let archive = registries
            .cloned(&selected.name)
            .expect("the archive is kept");
        assert_eq!(blake3::hash(archive), selected.release.digest);
    }

    /// A git dependency of a git dependency is refused, naming the package that declared it.
    #[test]
    fn a_git_dependency_below_the_root_is_refused() {
        let fixture = Fixture::new("resolve-git-below");
        let repo = fixture.root().join("repo");
        let toml = format!(
            "[package]\nname = 'Acme\\Git'\nversion = \"1.0.0\"\n\n[dependencies]\n\
             'Acme\\Other' = {{ git = 'file:{}', rev = '{}' }}\n",
            repo.display(),
            "1".repeat(40)
        );
        let rev = super::super::git::tests::package(&repo, &[("package.toml", toml.as_bytes())]);
        let app = with_git(&fixture, &format!("file:{}", repo.display()), &rev);
        let mut registries = Registries::new(&app, None).clones_in(fixture.root(), git::LIMITS);
        let fault = resolve(&app, &mut registries, None).expect_err("it is refused");
        assert!(
            fault
                .0
                .contains("`Acme\\Git` 1.0.0 depends on `Acme\\Other` from git"),
            "{fault}"
        );
    }

    /// A graph across majors selects the higher major and reports who asked for the older one.
    #[test]
    fn a_graph_across_majors_reports_the_crossing() {
        let fixture = Fixture::new("resolve-majors");
        fixture.publish("Acme\\Log", "1.4.0", &[], "\"released\"");
        fixture.publish("Acme\\Log", "2.1.0", &[], "\"released\"");
        fixture.publish(
            "Acme\\Http",
            "1.0.0",
            &[("Acme\\Log", "1.4.0")],
            "\"released\"",
        );
        let app = application(&fixture, &[("Acme\\Http", "1.0.0"), ("Acme\\Log", "2.1.0")]);
        let graph = resolve(&app, &mut Registries::new(&app, None), None).expect("it resolves");
        assert_eq!(versions(&graph), ["Acme\\Http 1.0.0", "Acme\\Log 2.1.0"]);
        assert_eq!(graph.crossings.len(), 1);
        assert_eq!(
            graph.crossings[0].to_string(),
            "`Acme\\Log` 2.1.0 is selected, and `Acme\\Http` 1.0.0 asks for 1.4.0"
        );
    }

    /// Below 1.0 a minor is a breaking line, and a patch below 0.1.
    #[test]
    fn below_one_the_breaking_line_moves_left() {
        let v = |text: &str| line(text.parse().expect("a version"));
        assert_eq!(v("1.2.3"), v("1.9.0"));
        assert_ne!(v("0.3.1"), v("0.4.0"));
        assert_eq!(v("0.3.1"), v("0.3.9"));
        assert_ne!(v("0.0.1"), v("0.0.2"));
    }

    /// A retracted selection names its fix, a withheld one says its files are not served, and
    /// a pending one named as a minimum is used.
    #[test]
    fn a_selection_is_used_as_its_state_says() {
        let fixture = Fixture::new("resolve-states");
        fixture.publish(
            "Acme\\Log",
            "1.0.0",
            &[],
            r#"{"retracted": {"reason": "it loses lines", "fixed_in": "1.0.1"}}"#,
        );
        fixture.publish(
            "Acme\\Gone",
            "1.0.0",
            &[],
            r#"{"withheld": {"reason": "malware"}}"#,
        );
        fixture.publish(
            "Acme\\New",
            "1.0.0",
            &[],
            r#"{"pending": {"selectable_at": 1}}"#,
        );
        let run = |dependencies: &[(&str, &str)]| {
            let app = application(&fixture, dependencies);
            resolve(&app, &mut Registries::new(&app, None), None)
        };
        let retracted = run(&[("Acme\\Log", "1.0.0")]).expect_err("retracted");
        assert!(
            retracted.0.contains("Raise the minimum to 1.0.1"),
            "{retracted}"
        );
        let withheld = run(&[("Acme\\Gone", "1.0.0")]).expect_err("withheld");
        assert!(
            withheld.0.contains("withheld by the registry: malware"),
            "{withheld}"
        );
        assert!(run(&[("Acme\\New", "1.0.0")]).is_ok());
    }

    /// A version the index does not list names the package and who asked for it.
    #[test]
    fn a_missing_version_names_who_asked() {
        let fixture = Fixture::new("resolve-missing");
        fixture.publish("Acme\\Log", "1.0.0", &[], "\"released\"");
        let app = application(&fixture, &[("Acme\\Log", "3.0.0")]);
        let failed = resolve(&app, &mut Registries::new(&app, None), None).expect_err("missing");
        assert!(
            failed
                .0
                .contains("no version 3.0.0 of `Acme\\Log`, which package.toml needs"),
            "{failed}"
        );
    }
}
