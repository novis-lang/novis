//! `nvs pkg fetch` and `nvs pkg vendor` through the binary
//! (`rule:packaging/a-fetched-package-is-an-autoload-line`,
//! `rule:security/package-authority-is-granted-one-line-at-a-time`,
//! `rule:packaging/nothing-runs-before-the-program`, `rule:packaging/the-lockfile-holds-every-digest`).
//!
//! Each test builds a `file:` registry from package folders that `nvs pkg pack` packs, so a digest
//! is always the one the code packs today, and an application beside it that names the registry
//! for its vendor. Resolution, the lock's checks and the index are unit tests in
//! `src/package/`; these tests show what the commands write and what the compiler then reads.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A registry, an application that resolves `Acme` from it, and a private data folder.
struct Tree {
    root: nvs_repo::Scratch,
    data: nvs_repo::Scratch,
}

impl Tree {
    /// An empty registry, and an application whose `package.toml` is `manifest` with the
    /// registry line added.
    fn new(name: &str, dependencies: &str) -> Self {
        let tree = Self {
            root: nvs_repo::scratch(name),
            data: nvs_repo::scratch_private(&format!("{name}-data")),
        };
        fs::create_dir_all(tree.registry()).expect("the test makes the registry");
        fs::write(
            tree.registry().join("registry.json"),
            r#"{ "schema": 1, "hosts": [] }"#,
        )
        .expect("the test writes the root file");
        fs::create_dir_all(tree.app()).expect("the test makes the application");
        tree.manifest(dependencies);
        fs::write(
            tree.app().join("main.nvs"),
            "<?nvs\nrequire './vendor/packages.nvs';\n\nvar $main = new Acme\\Http\\Main();\n",
        )
        .expect("the test writes the entry");
        tree
    }

    fn registry(&self) -> PathBuf {
        self.root.join("registry")
    }

    fn app(&self) -> PathBuf {
        self.root.join("app")
    }

    /// The application's `package.toml`, with `dependencies` and the registry line.
    fn manifest(&self, dependencies: &str) {
        fs::write(
            self.app().join("package.toml"),
            format!(
                "[dependencies]\n{dependencies}\n[registries]\nAcme = {{ url = \"file:../registry\" }}\n"
            ),
        )
        .expect("the test writes the manifest");
    }

    /// `namespace` 1.0.0, requesting `fs.read`, with `files` under its `src`, packed by the binary
    /// and listed in the registry.
    fn publish(&self, namespace: &str, files: &[(&str, &str)]) {
        let slug = namespace.to_lowercase().replace('\\', "/");
        let dir = self.root.join("packages").join(slug.replace('/', "-"));
        fs::create_dir_all(dir.join("src")).expect("the test makes the package");
        fs::write(
            dir.join("package.toml"),
            format!(
                "[package]\nname = '{namespace}'\nversion = \"1.0.0\"\n\n\
                 [capabilities]\nrequired = [\"fs.read\"]\n"
            ),
        )
        .expect("the test writes the package's manifest");
        fs::write(dir.join("LICENSE"), "MIT\n").expect("the test writes the license");
        for (path, text) in files {
            fs::write(dir.join("src").join(path), text).expect("the test writes the source");
        }
        let tar = self
            .root
            .join("packages")
            .join(format!("{}.tar", slug.replace('/', "-")));
        let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["pkg", "pack"])
            .arg(&dir)
            .arg("--out")
            .arg(&tar)
            .output()
            .expect("the `nvs` binary this test was built beside runs");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let bytes = fs::read(&tar).expect("the archive was written");
        let digest = blake3::hash(&bytes).to_hex().to_string();
        let archive = self.registry().join("archive");
        fs::create_dir_all(&archive).expect("the test makes the archive folder");
        fs::write(archive.join(format!("{digest}.tar")), &bytes).expect("the test copies it");
        let index = serde_json::json!({
            "schema": 1,
            "name": namespace,
            "owners": ["owner"],
            "versions": [{
                "version": "1.0.0",
                "digest": digest,
                "state": "released",
                "dependencies": {},
                "capabilities": { "required": ["fs.read"], "optional": [] },
                "repository": format!("https://git.example.com/{slug}"),
                "commit": "0".repeat(40),
            }],
        });
        let path = self.registry().join(format!("index/{slug}.json"));
        fs::create_dir_all(path.parent().expect("an index file has a folder"))
            .expect("the test makes the index folder");
        fs::write(path, index.to_string()).expect("the test writes the index");
    }

    /// `nvs <args>` in the application's folder, with the tree's own data folder.
    fn nvs(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(&*self.data)
            .arg("--no-init")
            .args(args)
            .current_dir(self.app())
            .output()
            .expect("the `nvs` binary this test was built beside runs")
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.app().join(path)).unwrap_or_default()
    }
}

/// Both streams, for a message on failure.
fn shown(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// A class whose one method calls `Core\IO::exists`, which needs `fs.read`.
const MAIN: &str = "<?nvs\nnamespace Acme\\Http;\n\nfinal class Main\n{\n    \
                    public function probe(): bool\n    {\n        \
                    return Core\\IO::exists('x');\n    }\n}\n";

const HTTP: &str = "'Acme\\Http' = \"1.0.0\"\n";

// covers: tools:packages/nvs-pkg-fetch-and-nvs-pkg-vendor
#[test]
fn a_fetched_package_compiles_only_with_its_grant_line() {
    let tree = Tree::new("package-fetch-grant", HTTP);
    tree.publish("Acme\\Http", &[("Main.nvs", MAIN)]);

    let out = tree.nvs(&["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert!(tree.app().join("vendor/acme/http/src/Main.nvs").is_file());
    assert!(
        tree.read("vendor/packages.nvs")
            .contains("autoload 'Acme\\Http' from './acme/http/src';"),
        "{}",
        tree.read("vendor/packages.nvs")
    );
    assert!(tree.read("package.lock").contains("name = 'Acme\\Http'"));
    let line = "'Acme\\Http\\*' = []  # required: fs.read";
    assert!(
        tree.read("package.toml").contains(line),
        "{}",
        tree.read("package.toml")
    );

    let out = tree.nvs(&["check", "main.nvs"]);
    assert!(
        !out.status.success(),
        "the package compiled with an empty grant line"
    );
    assert!(shown(&out).contains("E0658"), "{}", shown(&out));

    let edited = tree
        .read("package.toml")
        .replace(line, "'Acme\\Http\\*' = [\"fs.read\"]");
    fs::write(tree.app().join("package.toml"), &edited).expect("the test edits the line");
    let out = tree.nvs(&["check", "main.nvs"]);
    assert!(out.status.success(), "{}", shown(&out));

    let out = tree.nvs(&["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(
        tree.read("package.toml"),
        edited,
        "a second fetch kept the edited line"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("0 written"),
        "a folder with the locked files is kept: {}",
        shown(&out)
    );
    let out = tree.nvs(&["check", "main.nvs"]);
    assert!(out.status.success(), "{}", shown(&out));
}

#[test]
fn nothing_a_package_holds_runs_during_fetch_or_vendor() {
    let tree = Tree::new("package-fetch-runs-nothing", HTTP);
    let hook = "<?nvs\nCore\\IO::write('ran.txt', 'ran');\necho \"ran\\n\";\n";
    tree.publish("Acme\\Http", &[("Main.nvs", MAIN), ("Hook.nvs", hook)]);

    for command in ["fetch", "vendor"] {
        let out = tree.nvs(&["pkg", command]);
        assert!(out.status.success(), "{command}: {}", shown(&out));
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("1 written"),
            "{command} writes the folder: {}",
            shown(&out)
        );
        assert!(
            !String::from_utf8_lossy(&out.stdout)
                .lines()
                .any(|line| line == "ran"),
            "{command} ran the package: {}",
            shown(&out)
        );
    }
    assert!(tree.app().join("vendor/acme/http/src/Hook.nvs").is_file());
    for dir in [
        tree.app(),
        tree.app().join("vendor/acme/http"),
        tree.root.to_path_buf(),
    ] {
        assert!(
            !dir.join("ran.txt").exists(),
            "{} has `ran.txt`",
            dir.display()
        );
    }
}

/// `package.lock`'s text with the digest of its one package replaced.
fn edit_digest(lock: &str) -> String {
    lock.lines()
        .map(|line| {
            if line.starts_with("digest = ") {
                format!("digest = \"{}\"", "0".repeat(64))
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_locked_run_refuses_a_missing_entry_and_an_edited_digest() {
    let tree = Tree::new("package-fetch-locked", HTTP);
    tree.publish("Acme\\Http", &[("Main.nvs", MAIN)]);
    tree.publish("Acme\\Log", &[]);

    let out = tree.nvs(&["pkg", "fetch", "--locked"]);
    assert!(!out.status.success(), "a locked run with no lock passed");
    assert!(shown(&out).contains("--locked"), "{}", shown(&out));

    let out = tree.nvs(&["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));
    let lock = tree.read("package.lock");

    tree.manifest(&format!("{HTTP}'Acme\\Log' = \"1.0.0\"\n"));
    let out = tree.nvs(&["pkg", "fetch", "--locked"]);
    assert!(
        !out.status.success(),
        "a package missing from the lock was fetched"
    );
    assert!(
        shown(&out).contains("`Acme\\Log` 1.0.0 is not in package.lock"),
        "{}",
        shown(&out)
    );
    assert_eq!(
        tree.read("package.lock"),
        lock,
        "a locked run never writes the lock"
    );
    assert!(!tree.app().join("vendor/acme/log").exists());

    tree.manifest(HTTP);
    fs::write(tree.app().join("package.lock"), edit_digest(&lock)).expect("the test edits it");
    for args in [&["pkg", "vendor", "--locked"][..], &["pkg", "fetch"][..]] {
        let out = tree.nvs(args);
        assert!(!out.status.success(), "{args:?} passed an edited digest");
        assert!(
            shown(&out).contains("the digest of `Acme\\Http` 1.0.0 in package.lock"),
            "{args:?}: {}",
            shown(&out)
        );
    }
}
