//! `package.lock`: every package of the resolved graph with its digest
//! (`rule:packaging/the-lockfile-holds-every-digest`).
//!
//! ```toml
//! # Written by `nvs pkg`. Commit this file, and do not edit it.
//! schema = 1
//!
//! [[package]]
//! name = 'Acme\Http'
//! version = "1.4.0"
//! source = "https://registry.novis-lang.org"
//! digest = "<64 hex digits of BLAKE3>"
//! repository = "https://git.example.com/acme/http"
//! commit = "<40 hex digits>"
//! owners = ["alice"]
//! hosts = ["fonts.example.com"]
//!
//! [package.capabilities]
//! required = ["net.connect"]
//! optional = []
//! ```
//!
//! **Every package of the graph is one entry**, direct and transitive, in slug order, so the same
//! graph writes the same bytes. Beside the name, version, source and digest, an entry records what
//! `nvs pkg update` compares: the requested capabilities, the repository and commit, the owners,
//! and the host of every URL its `[[download]]` files name
//! (`rule:packaging/an-external-file-is-pinned-by-url-sha256-and-size`), which a package without
//! one does not write. Every key is known, and an unknown one is refused. A root git
//! dependency's `source` is `git+<url>`.
//!
//! **A digest that differs from the registry's is a hard error naming the package** ([`Lock::check`]):
//! never a warning and never a fetch of the other bytes. With `--locked`, a package the lock does
//! not have, a version it does not have, or an entry the graph no longer needs is an error too,
//! so a locked run fetches nothing outside the lock and never changes it.
//!
//! Cost: the lock is read and written once per command, and held while the command runs.
//! Nothing here runs on the request path.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::download::{self, Files};
use super::index::{Capabilities, Fault, digest, fault};
use super::manifest::{Name, Version};
use super::resolve::Graph;

/// The lock's file name, beside the application's `package.toml`.
pub(crate) const FILE: &str = "package.lock";

/// The schema version of the lock this client reads and writes.
const SCHEMA: u64 = 1;

/// The comment the lock starts with.
const HEADER: &str = "# Written by `nvs pkg`. Commit this file, and do not edit it.\n";

/// One locked package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Locked {
    pub(crate) name: Name,
    pub(crate) version: Version,
    /// The URL of the registry it comes from.
    pub(crate) source: String,
    pub(crate) digest: blake3::Hash,
    pub(crate) capabilities: Capabilities,
    pub(crate) repository: String,
    pub(crate) commit: String,
    pub(crate) owners: Vec<String>,
    /// The host of every URL of its `[[download]]` files, sorted, each once.
    pub(crate) hosts: Vec<String>,
}

/// A `package.lock`: its packages in slug order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Lock {
    pub(crate) packages: Vec<Locked>,
}

/// The file, as written.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schema: u64,
    #[serde(default, rename = "package")]
    packages: Vec<Line>,
}

/// One `[[package]]`, as written.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Line {
    name: String,
    version: String,
    source: String,
    digest: String,
    repository: String,
    commit: String,
    #[serde(default)]
    owners: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hosts: Vec<String>,
    #[serde(default)]
    capabilities: Capabilities,
}

impl Lock {
    /// The lock of `graph`.
    pub(crate) fn of(graph: &Graph) -> Self {
        let mut packages: Vec<Locked> = graph
            .packages
            .iter()
            .map(|selected| Locked {
                name: selected.name.clone(),
                version: selected.release.version,
                source: selected.source.clone(),
                digest: selected.release.digest,
                capabilities: selected.release.capabilities.clone(),
                repository: selected.release.repository.clone(),
                commit: selected.release.commit.clone(),
                owners: selected.owners.clone(),
                hosts: Vec::new(),
            })
            .collect();
        packages.sort_by_key(|locked| locked.name.slug());
        Self { packages }
    }

    /// This lock, with each package's hosts read from its entries in `files`.
    pub(crate) fn with_hosts(mut self, files: &Files) -> Self {
        for locked in &mut self.packages {
            locked.hosts = files
                .get(&locked.name.slug())
                .map_or_else(Vec::new, |entries| download::hosts(entries));
        }
        self
    }

    /// Every host any package of this lock downloads a file from.
    pub(crate) fn hosts(&self) -> BTreeSet<&str> {
        self.packages
            .iter()
            .flat_map(|locked| locked.hosts.iter().map(String::as_str))
            .collect()
    }

    /// The entry for `name`, compared by slug.
    pub(crate) fn get(&self, name: &Name) -> Option<&Locked> {
        let slug = name.slug();
        self.packages
            .iter()
            .find(|locked| locked.name.slug() == slug)
    }

    /// Whether the lock has `name` at exactly `version`.
    pub(crate) fn has(&self, name: &Name, version: Version) -> bool {
        self.get(name)
            .is_some_and(|locked| locked.version == version)
    }

    /// A fault for the first package of `graph` whose digest differs from this lock's at the
    /// same version, and with `locked`, for the first difference of any kind.
    ///
    /// # Errors
    ///
    /// A digest mismatch, always. With `locked`: a package or version the lock does not have, and
    /// an entry the graph no longer needs. Each message names the package.
    pub(crate) fn check(&self, graph: &Graph, locked: bool) -> Result<(), Fault> {
        for selected in &graph.packages {
            let (name, version) = (&selected.name, selected.release.version);
            match self.get(name) {
                Some(entry) if entry.version == version => {
                    if entry.digest != selected.release.digest {
                        return Err(fault(format!(
                            "the digest of `{name}` {version} in {FILE} is {}, and the \
                             registry's is {}. The package's files are not the ones that were \
                             locked, so nothing was fetched.",
                            entry.digest.to_hex(),
                            selected.release.digest.to_hex()
                        )));
                    }
                }
                Some(entry) if locked => {
                    return Err(fault(format!(
                        "{FILE} has `{name}` {}, and the graph needs {version}. \
                         Run the command without `--locked` to update {FILE}.",
                        entry.version
                    )));
                }
                None if locked => {
                    return Err(fault(format!(
                        "`{name}` {version} is not in {FILE}. \
                         Run the command without `--locked` to add it."
                    )));
                }
                Some(_) | None => {}
            }
        }
        if locked {
            let slugs: Vec<String> = graph.packages.iter().map(|s| s.name.slug()).collect();
            if let Some(extra) = self
                .packages
                .iter()
                .find(|entry| !slugs.contains(&entry.name.slug()))
            {
                return Err(fault(format!(
                    "{FILE} has `{}`, and the graph no longer needs it. \
                     Run the command without `--locked` to remove it.",
                    extra.name
                )));
            }
        }
        Ok(())
    }

    /// The lock as its file's text.
    pub(crate) fn render(&self) -> String {
        let file = File {
            schema: SCHEMA,
            packages: self
                .packages
                .iter()
                .map(|locked| Line {
                    name: locked.name.to_string(),
                    version: locked.version.to_string(),
                    source: locked.source.clone(),
                    digest: locked.digest.to_hex().to_string(),
                    repository: locked.repository.clone(),
                    commit: locked.commit.clone(),
                    owners: locked.owners.clone(),
                    hosts: locked.hosts.clone(),
                    capabilities: locked.capabilities.clone(),
                })
                .collect(),
        };
        // Every value is a string, a list of strings or a number, which TOML always writes.
        let body = toml::to_string(&file).unwrap_or_default();
        format!("{HEADER}{body}")
    }

    /// The lock `text`, read from `path`.
    ///
    /// # Errors
    ///
    /// Text that is not a lock this client reads, with the path in the message.
    pub(crate) fn parse(path: &Path, text: &str) -> Result<Self, Fault> {
        let bad = |detail: String| fault(format!("`{}` is not valid: {detail}", path.display()));
        let file: File = toml::from_str(text).map_err(|err| bad(err.message().to_owned()))?;
        if file.schema != SCHEMA {
            return Err(bad(format!(
                "it has schema {}, and this version of `nvs` reads schema {SCHEMA}",
                file.schema
            )));
        }
        let mut packages = Vec::with_capacity(file.packages.len());
        for line in file.packages {
            let name = Name::checked(&line.name).map_err(|(message, _)| bad(message))?;
            let version = line.version.parse::<Version>().map_err(bad)?;
            let digest = digest(&line.digest).ok_or_else(|| {
                bad(format!(
                    "the digest of `{name}` is not 64 lowercase hex digits"
                ))
            })?;
            packages.push(Locked {
                name,
                version,
                source: line.source,
                digest,
                capabilities: line.capabilities,
                repository: line.repository,
                commit: line.commit,
                owners: line.owners,
                hosts: line.hosts,
            });
        }
        packages.sort_by_key(|locked| locked.name.slug());
        if let Some(pair) = packages
            .windows(2)
            .find(|pair| pair[0].name.slug() == pair[1].name.slug())
        {
            return Err(bad(format!("it has `{}` twice", pair[1].name)));
        }
        Ok(Self { packages })
    }

    /// The lock at `path`, or `None` when there is no such file.
    ///
    /// # Errors
    ///
    /// A file that cannot be read, or one [`Self::parse`] refuses.
    pub(crate) fn read(path: &Path) -> Result<Option<Self>, Fault> {
        match fs::read_to_string(path) {
            Ok(text) => Self::parse(path, &text).map(Some),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(fault(format!("cannot read `{}`: {err}", path.display()))),
        }
    }

    /// The lock written to `path` through a temporary file and a rename, so a reader never sees
    /// half of it.
    ///
    /// # Errors
    ///
    /// A failed write or rename.
    pub(crate) fn write(&self, path: &Path) -> Result<(), Fault> {
        let failed =
            |err: std::io::Error| fault(format!("cannot write `{}`: {err}", path.display()));
        let partial = path.with_extension("lock.partial");
        fs::write(&partial, self.render()).map_err(failed)?;
        fs::rename(&partial, path).map_err(failed)
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixture::Fixture;
    use super::super::resolve::{Registries, resolve};
    use super::*;

    /// The graph of `dependencies` against `fixture`.
    fn graph(fixture: &Fixture, dependencies: &[(&str, &str)]) -> Graph {
        let app = super::super::resolve::tests::application(fixture, dependencies);
        resolve(&app, &mut Registries::new(&app, None), None).expect("it resolves")
    }

    /// A lock written and read back is the same lock, and its text is the same each time.
    #[test]
    fn a_lock_reads_back() {
        let fixture = Fixture::new("lock-reads-back");
        fixture.publish("Acme\\Log", "1.0.0", &[], "\"released\"");
        fixture.publish(
            "Acme\\Http",
            "1.0.0",
            &[("Acme\\Log", "1.0.0")],
            "\"released\"",
        );
        let lock = Lock::of(&graph(&fixture, &[("Acme\\Http", "1.0.0")]));
        assert_eq!(
            lock.packages.len(),
            2,
            "the transitive package is locked too"
        );
        let path = fixture.root().join(FILE);
        lock.write(&path).expect("the lock writes");
        let read = Lock::read(&path).expect("it reads").expect("it exists");
        assert_eq!(read, lock);
        assert_eq!(read.render(), lock.render());
        assert!(lock.render().starts_with(HEADER));
    }

    /// Each package's download hosts are written once each and read back, a package with none
    /// writes no `hosts` line, and the report prints each file and only the host the old lock
    /// does not have as new.
    #[test]
    fn the_lock_records_download_hosts() {
        let fixture = Fixture::new("lock-hosts");
        fixture.publish("Acme\\Log", "1.0.0", &[], "\"released\"");
        fixture.publish(
            "Acme\\Fonts",
            "1.0.0",
            &[("Acme\\Log", "1.0.0")],
            "\"released\"",
        );
        let resolved = graph(&fixture, &[("Acme\\Fonts", "1.0.0")]);
        let entry = |path: &str, urls: &[&str]| super::super::manifest::Download {
            path: path.to_owned(),
            urls: urls.iter().map(|url| (*url).to_owned()).collect(),
            sha256: "a".repeat(64),
            size: 3 << 20,
            license: Some("OFL-1.1".to_owned()),
        };
        let old_files = Files::from([(
            "acme/fonts".to_owned(),
            vec![entry("fonts/A.otf", &["https://Old.example.com/a.otf"])],
        )]);
        let old = Lock::of(&resolved).with_hosts(&old_files);
        let files = Files::from([(
            "acme/fonts".to_owned(),
            vec![
                entry("fonts/A.otf", &["https://old.example.com/a.otf"]),
                entry(
                    "fonts/B.otf",
                    &["https://new.example.org/b", "https://old.example.com/b"],
                ),
            ],
        )]);
        let lock = Lock::of(&resolved).with_hosts(&files);
        let fonts = lock.packages.iter().find(|l| l.name.slug() == "acme/fonts");
        assert_eq!(
            fonts.expect("it is locked").hosts,
            ["new.example.org", "old.example.com"]
        );
        let path = fixture.root().join(FILE);
        lock.write(&path).expect("the lock writes");
        assert_eq!(Lock::read(&path).expect("it reads"), Some(lock.clone()));
        assert_eq!(
            lock.render().matches("hosts = ").count(),
            1,
            "{}",
            lock.render()
        );

        let text = super::super::report::report(&resolved, Some(&old), &[], &files);
        assert!(
            text.contains("`Acme\\Fonts` fonts/B.otf: 3145728 bytes (3.0 MiB) from new.example.org or old.example.com, license OFL-1.1"),
            "{text}"
        );
        assert!(
            text.contains("Total: 2 file(s), 6291456 bytes (6.0 MiB)."),
            "{text}"
        );
        assert!(text.contains("new.example.org is a new host"), "{text}");
        assert!(!text.contains("old.example.com is a new host"), "{text}");
    }

    /// With `--locked`, a package the lock does not have is an error naming it.
    #[test]
    fn a_locked_run_with_a_missing_entry_fails() {
        let fixture = Fixture::new("lock-missing");
        fixture.publish("Acme\\Log", "1.0.0", &[], "\"released\"");
        fixture.publish("Acme\\Db", "1.0.0", &[], "\"released\"");
        let lock = Lock::of(&graph(&fixture, &[("Acme\\Log", "1.0.0")]));
        let wider = graph(&fixture, &[("Acme\\Log", "1.0.0"), ("Acme\\Db", "1.0.0")]);
        assert_eq!(lock.check(&wider, false), Ok(()));
        let failed = lock.check(&wider, true).expect_err("not in the lock");
        assert!(
            failed
                .0
                .starts_with("`Acme\\Db` 1.0.0 is not in package.lock"),
            "{failed}"
        );
        let narrower = Lock::of(&wider);
        let fewer = graph(&fixture, &[("Acme\\Log", "1.0.0")]);
        let extra = narrower
            .check(&fewer, true)
            .expect_err("an entry the graph does not need");
        assert!(extra.0.contains("has `Acme\\Db`"), "{extra}");
    }

    /// An edited digest is a hard error naming the package, with or without `--locked`.
    #[test]
    fn an_edited_digest_fails_naming_the_package() {
        let fixture = Fixture::new("lock-edited");
        fixture.publish("Acme\\Log", "1.0.0", &[], "\"released\"");
        let resolved = graph(&fixture, &[("Acme\\Log", "1.0.0")]);
        let path = fixture.root().join(FILE);
        Lock::of(&resolved).write(&path).expect("the lock writes");
        let text = fs::read_to_string(&path).expect("the lock reads");
        let real = resolved.packages[0].release.digest.to_hex().to_string();
        fs::write(&path, text.replace(&real, &"0".repeat(64))).expect("the test edits it");
        let edited = Lock::read(&path).expect("it reads").expect("it exists");
        for locked in [false, true] {
            let failed = edited
                .check(&resolved, locked)
                .expect_err("the digest differs");
            assert!(
                failed.0.starts_with("the digest of `Acme\\Log` 1.0.0"),
                "{failed}"
            );
        }
    }

    /// A retracted version stays selected while the lock has it.
    #[test]
    fn a_locked_retracted_version_still_resolves() {
        let fixture = Fixture::new("lock-retracted");
        fixture.publish(
            "Acme\\Log",
            "1.0.0",
            &[],
            r#"{"retracted": {"reason": "it loses lines", "fixed_in": null}}"#,
        );
        let app = super::super::resolve::tests::application(&fixture, &[("Acme\\Log", "1.0.0")]);
        let mut registries = Registries::new(&app, None);
        assert!(resolve(&app, &mut registries, None).is_err());
        let release = registries
            .index(&Name::checked("Acme\\Log").expect("a name"))
            .expect("it answers")
            .expect("it exists")
            .releases[0]
            .clone();
        let lock = Lock {
            packages: vec![Locked {
                name: Name::checked("Acme\\Log").expect("a name"),
                version: release.version,
                source: fixture.url(),
                digest: release.digest,
                capabilities: Capabilities::default(),
                repository: release.repository,
                commit: release.commit,
                owners: Vec::new(),
                hosts: Vec::new(),
            }],
        };
        assert!(resolve(&app, &mut registries, Some(&lock)).is_ok());
    }

    /// An unknown key is refused.
    #[test]
    fn an_unknown_key_is_refused() {
        let failed = Lock::parse(Path::new(FILE), "schema = 1\nextra = 2\n").expect_err("unknown");
        assert!(failed.0.contains("extra"), "{failed}");
    }
}
