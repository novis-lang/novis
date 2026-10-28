//! A registry on disk, built at test time from package directories through [`pack::file_set`]
//! and [`pack::write`], so a digest in it is always the one the code packs today
//! (`crates/nvs-cli/tests/fixtures/registry/README.md` describes the layout). Each publish is
//! also a record of the file host's log, signed under a key made for the fixture.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use nvs_diagnostics::SourceMap;

use super::entry::Entry;
use super::index::{ROOT_FILE, SCHEMA};
use super::log;
use super::manifest::{self, Name};
use super::note::{Checkpoint, Signer};
use super::pack;

/// The name of the fixture's log key, which is also its log's origin.
const KEY_NAME: &str = "registry.example.com";

/// A scratch folder with a registry's URL directory, its one file host and the packages behind
/// them.
pub(crate) struct Fixture {
    scratch: nvs_repo::Scratch,
    /// Each package's index lines, by slug.
    lines: RefCell<BTreeMap<String, (String, Vec<serde_json::Value>)>>,
    /// The key the log's checkpoints are signed with.
    signer: Signer,
    /// The log's size.
    size: RefCell<u64>,
}

impl Fixture {
    /// An empty registry whose root file lists [`Self::files`] as its one file host.
    pub(crate) fn new(name: &str) -> Self {
        let (signer, _) = Signer::generate(KEY_NAME).expect("a key is made");
        let fixture = Self {
            scratch: nvs_repo::scratch(name),
            lines: RefCell::default(),
            signer,
            size: RefCell::default(),
        };
        fs::create_dir_all(fixture.registry()).expect("the test makes the registry");
        fs::create_dir_all(fixture.files().join("archive")).expect("the test makes the host");
        fixture.hosts(&[format!("file:{}", fixture.files().display())]);
        fixture
    }

    /// The scratch folder.
    pub(crate) fn root(&self) -> &Path {
        &self.scratch
    }

    /// The directory the registry's URL names.
    pub(crate) fn registry(&self) -> PathBuf {
        self.root().join("registry")
    }

    /// The file host's directory.
    pub(crate) fn files(&self) -> PathBuf {
        self.root().join("files")
    }

    /// The registry's URL.
    pub(crate) fn url(&self) -> String {
        format!("file:{}", self.registry().display())
    }

    /// The verifier key of the registry's log, as a `[registries]` line names it.
    pub(crate) fn key(&self) -> String {
        self.signer.verifier().to_string()
    }

    /// `entry` appended to the file host's log, under a newly signed checkpoint. Its record
    /// number.
    pub(crate) fn append(&self, entry: &Entry) -> u64 {
        let dir = self.files().join("log");
        let n = *self.size.borrow();
        let (size, root) = log::append(&dir, n, &[entry.canonical()]).expect("the log appends");
        let note = Checkpoint::new(KEY_NAME, size, root)
            .and_then(|checkpoint| checkpoint.sign(&self.signer))
            .expect("the checkpoint signs");
        fs::write(dir.join("checkpoint"), note).expect("the test writes the checkpoint");
        *self.size.borrow_mut() = size;
        n
    }

    /// The root file rewritten to list `hosts`, in order.
    pub(crate) fn hosts(&self, hosts: &[String]) {
        let root = serde_json::json!({ "schema": SCHEMA, "hosts": hosts });
        fs::write(self.registry().join(ROOT_FILE), root.to_string())
            .expect("the test writes the root file");
    }

    /// `namespace` at `version`, with `dependencies` as its minimums, packed onto the file host
    /// and listed in its index with `state`, a JSON state. The archive's digest.
    pub(crate) fn publish(
        &self,
        namespace: &str,
        version: &str,
        dependencies: &[(&str, &str)],
        state: &str,
    ) -> blake3::Hash {
        let name = Name::checked(namespace).expect("the test names a valid package");
        let dir = self
            .root()
            .join("packages")
            .join(format!("{}-{version}", name.slug().replace('/', "-")));
        fs::create_dir_all(dir.join("src")).expect("the test makes the package");
        let mut toml = format!("[package]\nname = '{namespace}'\nversion = \"{version}\"\n");
        if !dependencies.is_empty() {
            toml.push_str("\n[dependencies]\n");
            for (dependency, minimum) in dependencies {
                toml.push_str(&format!("'{dependency}' = \"{minimum}\"\n"));
            }
        }
        fs::write(dir.join("package.toml"), &toml).expect("the test writes the manifest");
        fs::write(dir.join("LICENSE"), "MIT\n").expect("the test writes the license");
        fs::write(
            dir.join("src/Main.nvs"),
            format!("<?nvs\nnamespace {namespace};\n\nfinal class Main\n{{\n}}\n"),
        )
        .expect("the test writes the class");
        let mut sources = SourceMap::new();
        let read = manifest::read(&dir.join("package.toml"), &mut sources)
            .expect("the test's manifest reads");
        let entries = pack::file_set(&dir, &read).expect("the test's package packs");
        let mut bytes = Vec::new();
        let digest = pack::write(&entries, &mut bytes).expect("the archive writes");
        fs::write(
            self.files()
                .join(format!("archive/{}.tar", digest.to_hex())),
            &bytes,
        )
        .expect("the test writes the archive");
        let repository = format!("https://git.example.com/{}", name.slug());
        let commit = "0".repeat(40);
        let record = self.append(&Entry::Publish {
            package: name.slug(),
            version: version.to_owned(),
            digest: digest.to_hex().to_string(),
            repository: repository.clone(),
            commit: commit.clone(),
        });
        let dependencies: BTreeMap<&str, &str> = dependencies.iter().copied().collect();
        let line = serde_json::json!({
            "version": version,
            "digest": digest.to_hex().as_str(),
            "state": serde_json::from_str::<serde_json::Value>(state).expect("the state is JSON"),
            "dependencies": dependencies,
            "capabilities": { "required": [], "optional": [] },
            "repository": repository,
            "commit": commit,
            "log": record,
        });
        let mut lines = self.lines.borrow_mut();
        let (_, versions) = lines
            .entry(name.slug())
            .or_insert_with(|| (namespace.to_owned(), Vec::new()));
        versions.push(line);
        let index = serde_json::json!({
            "schema": SCHEMA,
            "name": namespace,
            "owners": ["owner"],
            "versions": versions,
        });
        let path = self.files().join(format!("index/{}.json", name.slug()));
        fs::create_dir_all(path.parent().expect("an index file has a folder"))
            .expect("the test makes the index folder");
        fs::write(path, index.to_string()).expect("the test writes the index");
        digest
    }
}
