//! `nvs pkg fetch`, `nvs pkg vendor`, `nvs pkg add`, `nvs pkg update` and `nvs pkg outdated`
//! through the binary
//! (`rule:packaging/a-fetched-package-is-an-autoload-line`,
//! `rule:security/package-authority-is-granted-one-line-at-a-time`,
//! `rule:packaging/nothing-runs-before-the-program`, `rule:packaging/the-lockfile-holds-every-digest`,
//! `rule:packaging/a-package-carries-only-its-source-and-what-it-lists`).
//!
//! Each test builds a `file:` registry from package folders that `nvs pkg pack` packs, so a digest
//! is always the one the code packs today, and an application beside it that names the registry
//! for its vendor. Resolution, the lock's checks and the index are unit tests in
//! `src/package/`; these tests show what the commands write and what the compiler then reads.
//! That includes `vendor/package-files.toml`: a package reads a file it lists through a string
//! literal with an empty grant line, and a write, a `..` out of its set, an unlisted file, another
//! package's file and a link out of its folder are `E0658`.

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

    /// `namespace` 1.0.0, released and requesting `fs.read`, with `files` under its `src`, packed
    /// by the binary and listed in the registry.
    fn publish(&self, namespace: &str, files: &[(&str, &str)]) {
        self.release(namespace, "1.0.0", "released", &["fs.read"], files);
    }

    /// `namespace` at `version` in `state`, written as the index writes it, requesting
    /// `required`, with `files` under its `src`, packed by the binary and added to the package's
    /// index.
    fn release(
        &self,
        namespace: &str,
        version: &str,
        state: &str,
        required: &[&str],
        files: &[(&str, &str)],
    ) {
        self.release_listing(namespace, version, state, required, files, &[]);
    }

    /// `namespace` 1.0.0 as [`Self::publish`] makes it, which also carries `listed`, each a path
    /// beside `src` and its text, named in `[package] files`.
    fn publish_listing(&self, namespace: &str, files: &[(&str, &str)], listed: &[(&str, &str)]) {
        self.release_listing(namespace, "1.0.0", "released", &["fs.read"], files, listed);
    }

    /// [`Self::release`], with `listed` written beside `src` and named in `[package] files`.
    fn release_listing(
        &self,
        namespace: &str,
        version: &str,
        state: &str,
        required: &[&str],
        files: &[(&str, &str)],
        listed: &[(&str, &str)],
    ) {
        let slug = namespace.to_lowercase().replace('\\', "/");
        let folder = format!("{}-{version}", slug.replace('/', "-"));
        let dir = self.root.join("packages").join(&folder);
        fs::create_dir_all(dir.join("src")).expect("the test makes the package");
        let names: Vec<&str> = listed.iter().map(|(path, _)| *path).collect();
        let files_line = if names.is_empty() {
            String::new()
        } else {
            format!("files = {names:?}\n")
        };
        fs::write(
            dir.join("package.toml"),
            format!(
                "[package]\nname = '{namespace}'\nversion = \"{version}\"\n{files_line}\n\
                 [capabilities]\nrequired = {required:?}\n"
            ),
        )
        .expect("the test writes the package's manifest");
        fs::write(dir.join("LICENSE"), "MIT\n").expect("the test writes the license");
        for (path, text) in files {
            fs::write(dir.join("src").join(path), text).expect("the test writes the source");
        }
        for (path, text) in listed {
            let path = dir.join(path);
            fs::create_dir_all(path.parent().expect("a listed file has a folder"))
                .expect("the test makes the listed file's folder");
            fs::write(path, text).expect("the test writes the listed file");
        }
        let tar = self.root.join("packages").join(format!("{folder}.tar"));
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
        let state: serde_json::Value =
            serde_json::from_str(state).unwrap_or_else(|_| serde_json::json!(state));
        let line = serde_json::json!({
            "version": version,
            "digest": digest,
            "state": state,
            "dependencies": {},
            "capabilities": { "required": required, "optional": [] },
            "repository": format!("https://git.example.com/{slug}"),
            "commit": "0".repeat(40),
        });
        let path = self.registry().join(format!("index/{slug}.json"));
        let mut index = fs::read_to_string(&path).map_or_else(
            |_| serde_json::json!({ "schema": 1, "name": namespace, "owners": ["owner"], "versions": [] }),
            |text| serde_json::from_str(&text).expect("the test wrote the index"),
        );
        index["versions"]
            .as_array_mut()
            .expect("an index lists its versions")
            .push(line);
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

/// A pending version that becomes selectable long after the test runs, as an index writes it.
const PENDING: &str = r#"{ "pending": { "selectable_at": 99999999999 } }"#;

#[test]
fn add_chooses_only_a_released_version_and_takes_the_others_by_exact_version() {
    let tree = Tree::new("package-add-states", "");
    let main = &[("Main.nvs", MAIN)][..];
    tree.release("Acme\\Http", "1.0.0", "released", &["fs.read"], main);
    tree.release("Acme\\Http", "1.1.0", "unlisted", &["fs.read"], main);
    let retracted = r#"{ "retracted": { "reason": "it drops headers", "fixed_in": "1.2.1" } }"#;
    tree.release("Acme\\Http", "1.2.0", retracted, &["fs.read"], main);
    let withheld = r#"{ "withheld": { "reason": "malware" } }"#;
    tree.release("Acme\\Http", "1.3.0", withheld, &["fs.read"], main);
    tree.release("Acme\\Http", "1.4.0", PENDING, &["fs.read"], main);

    let out = tree.nvs(&["pkg", "add", "acme/http"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert!(
        tree.read("package.toml")
            .contains("'Acme\\Http' = \"1.0.0\""),
        "{}",
        tree.read("package.toml")
    );
    assert!(
        String::from_utf8_lossy(&out.stdout)
            .contains("`Acme\\Http` 1.0.0 needs 0 other package(s)"),
        "{}",
        shown(&out)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("`Acme\\Http` 1.0.0: required fs.read"),
        "{}",
        shown(&out)
    );

    for version in ["1.4.0", "1.1.0"] {
        let out = tree.nvs(&["pkg", "add", &format!("acme/http@{version}")]);
        assert!(out.status.success(), "{version}: {}", shown(&out));
        assert!(
            tree.read("package.toml")
                .contains(&format!("'Acme\\Http' = \"{version}\"")),
            "{version}: {}",
            tree.read("package.toml")
        );
        assert!(
            tree.read("package.lock")
                .contains(&format!("version = \"{version}\"")),
            "{version}: {}",
            tree.read("package.lock")
        );
        assert!(
            tree.read("vendor/acme/http/package.toml")
                .contains(&format!("version = \"{version}\"")),
            "{version} is the version under vendor/"
        );
    }

    for (version, says) in [
        (
            "1.2.0",
            "`Acme\\Http` 1.2.0 is retracted: it drops headers. Raise the minimum to 1.2.1",
        ),
        (
            "1.3.0",
            "`Acme\\Http` 1.3.0 is withheld by the registry: malware",
        ),
    ] {
        let manifest = tree.read("package.toml");
        let out = tree.nvs(&["pkg", "add", &format!("acme/http@{version}")]);
        assert!(!out.status.success(), "{version} was added");
        assert!(shown(&out).contains(says), "{version}: {}", shown(&out));
        assert_eq!(
            tree.read("package.toml"),
            manifest,
            "a failed add writes nothing"
        );
    }
}

#[test]
fn update_prints_a_capability_the_new_version_requests() {
    let tree = Tree::new("package-update-report", HTTP);
    tree.publish("Acme\\Http", &[("Main.nvs", MAIN)]);
    let out = tree.nvs(&["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));
    let main = &[("Main.nvs", MAIN)][..];
    tree.release(
        "Acme\\Http",
        "1.1.0",
        "released",
        &["fs.read", "net.connect"],
        main,
    );
    tree.release("Acme\\Http", "1.2.0", PENDING, &["fs.read"], main);

    let (manifest, lock) = (tree.read("package.toml"), tree.read("package.lock"));
    let out = tree.nvs(&["pkg", "outdated"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "`Acme\\Http` 1.0.0 -> 1.1.0",
        "the pending 1.2.0 is not offered"
    );
    assert_eq!(
        tree.read("package.toml"),
        manifest,
        "outdated writes nothing"
    );
    assert_eq!(tree.read("package.lock"), lock, "outdated writes nothing");

    let out = tree.nvs(&["pkg", "update"]);
    assert!(out.status.success(), "{}", shown(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in [
        "`Acme\\Http`: the minimum is raised from 1.0.0 to 1.1.0.",
        "`Acme\\Http` 1.1.0: required fs.read, net.connect",
        "Changes since package.lock:\n  `Acme\\Http` 1.0.0 -> 1.1.0\n    now requests net.connect (required)\n",
    ] {
        assert!(stdout.contains(line), "{line}: {}", shown(&out));
    }
    assert!(
        tree.read("package.toml")
            .contains("'Acme\\Http' = \"1.1.0\""),
        "{}",
        tree.read("package.toml")
    );
    assert!(
        tree.read("package.toml")
            .contains("'Acme\\Http\\*' = []  # required: fs.read"),
        "an update never changes a grant line: {}",
        tree.read("package.toml")
    );
    assert!(tree.read("package.lock").contains("version = \"1.1.0\""));

    let out = tree.nvs(&["pkg", "outdated"]);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("Every package is at its newest"),
        "{}",
        shown(&out)
    );
}

/// The file `Acme\Http` lists, and its text.
const GREETING: (&str, &str) = ("data/greeting.txt", "hello from the package\n");

/// A class `name` in `Acme\Http` whose constructor runs `body`.
fn class(name: &str, body: &str) -> String {
    format!(
        "<?nvs\nnamespace Acme\\Http;\n\nfinal class {name}\n{{\n    \
         public function constructor()\n    {{\n        {body}\n    }}\n}}\n"
    )
}

/// The class that reads `Acme\Http`'s own listed file, relative to `src/`.
fn reads_greeting() -> String {
    class("Main", "echo Core\\IO::readText('../data/greeting.txt');")
}

impl Tree {
    /// The application's `main.nvs`, which makes one `Acme\Http\<name>`.
    fn entry(&self, name: &str) {
        fs::write(
            self.app().join("main.nvs"),
            format!(
                "<?nvs\nrequire './vendor/packages.nvs';\n\nvar $main = new Acme\\Http\\{name}();\n"
            ),
        )
        .expect("the test writes the entry");
    }
}

#[test]
fn a_package_reads_its_own_listed_file_with_no_grant() {
    let tree = Tree::new("package-fetch-own-file", HTTP);
    tree.publish_listing(
        "Acme\\Http",
        &[("Main.nvs", &reads_greeting())],
        &[GREETING],
    );

    let out = tree.nvs(&["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert!(
        tree.read("package.toml").contains("'Acme\\Http\\*' = []"),
        "{}",
        tree.read("package.toml")
    );
    let set: toml::Table = toml::from_str(&tree.read("vendor/package-files.toml"))
        .expect("`vendor/package-files.toml` is TOML");
    let listed = set
        .get("Acme\\Http")
        .and_then(toml::Value::as_array)
        .expect("the set has a line for `Acme\\Http`");
    assert!(
        listed.iter().any(|path| path.as_str() == Some(GREETING.0)),
        "{}",
        tree.read("vendor/package-files.toml")
    );

    let out = tree.nvs(&["run", "main.nvs"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), GREETING.1);
}

/// The operator's `fs.read` scope names `data/` alone, so `vendor/` is outside it. The literal
/// read of the package's listed file passes, and a path to the same file built at run time is
/// refused by the scope, though the package's line grants `fs.read`.
#[test]
fn a_package_reads_its_own_file_where_the_operator_scope_leaves_vendor_out() {
    let tree = Tree::new("package-fetch-own-file-scoped", HTTP);
    let folder = tree.app().join("vendor/acme/http/data");
    let folder = folder.to_string_lossy().replace('\\', "/");
    let built = class(
        "Built",
        &format!("echo Core\\IO::readText('{folder}' . '/greeting.txt');"),
    );
    tree.publish_listing(
        "Acme\\Http",
        &[("Main.nvs", &reads_greeting()), ("Built.nvs", &built)],
        &[GREETING],
    );
    let out = tree.nvs(&["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));
    let manifest = tree.read("package.toml");
    assert!(manifest.contains("'Acme\\Http\\*' = []"), "{manifest}");
    fs::write(
        tree.app().join("package.toml"),
        manifest.replace("'Acme\\Http\\*' = []", "'Acme\\Http\\*' = [\"fs.read\"]"),
    )
    .expect("the test grants the package `fs.read`");
    fs::write(
        tree.app().join("nvs.toml"),
        "[capabilities.fs]\nread = [\"data\"]\n",
    )
    .expect("the test scopes `fs.read`");

    tree.entry("Main");
    let out = tree.nvs(&["run", "main.nvs"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), GREETING.1);

    tree.entry("Built");
    let out = tree.nvs(&["run", "main.nvs"]);
    assert!(!out.status.success(), "a built path read past the scope");
    assert!(shown(&out).contains("fs.read"), "{}", shown(&out));
    assert!(!shown(&out).contains(GREETING.1.trim()), "{}", shown(&out));
}

#[test]
fn a_package_needs_its_grant_for_anything_outside_its_own_set() {
    let tree = Tree::new(
        "package-fetch-own-file-outside",
        &format!("{HTTP}'Acme\\Other' = \"1.0.0\"\n"),
    );
    // From `vendor/acme/http/src/`: up four folders is the application's.
    let cases = [
        (
            "Write",
            "Core\\IO::write('../data/greeting.txt', 'changed');",
        ),
        (
            "Escape",
            "echo Core\\IO::readText('../data/../../../../secret.txt');",
        ),
        (
            "Unlisted",
            "echo Core\\IO::readText('../data/unlisted.txt');",
        ),
        (
            "Foreign",
            "echo Core\\IO::readText('../../other/data/other.txt');",
        ),
    ];
    let mut files = vec![("Main.nvs".to_owned(), reads_greeting())];
    files.extend(
        cases
            .iter()
            .map(|(name, body)| (format!("{name}.nvs"), class(name, body))),
    );
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    tree.publish_listing("Acme\\Http", &files, &[GREETING]);
    tree.publish_listing("Acme\\Other", &[], &[("data/other.txt", "other\n")]);

    let out = tree.nvs(&["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));
    fs::write(tree.app().join("secret.txt"), "secret\n").expect("the test writes the file");
    fs::write(
        tree.app().join("vendor/acme/http/data/unlisted.txt"),
        "unlisted\n",
    )
    .expect("the test places the file");
    assert!(
        tree.app()
            .join("vendor/acme/other/data/other.txt")
            .is_file()
    );

    // The listed read compiles in the same tree, so each refusal below is its case's own.
    tree.entry("Main");
    let out = tree.nvs(&["check", "main.nvs"]);
    assert!(out.status.success(), "{}", shown(&out));
    for (name, _) in cases {
        tree.entry(name);
        let out = tree.nvs(&["check", "main.nvs"]);
        assert!(!out.status.success(), "{name} compiled with no grant");
        assert!(shown(&out).contains("E0658"), "{name}: {}", shown(&out));
    }
}

#[cfg(unix)]
fn link_file(target: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn link_file(target: &std::path::Path, link: &std::path::Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, link)
}

#[test]
fn a_listed_file_that_links_out_of_its_package_needs_its_grant() {
    let tree = Tree::new("package-fetch-own-file-link", HTTP);
    tree.publish_listing(
        "Acme\\Http",
        &[("Main.nvs", &reads_greeting())],
        &[GREETING],
    );
    let out = tree.nvs(&["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));

    let secret = tree.app().join("secret.txt");
    fs::write(&secret, "secret\n").expect("the test writes the file");
    let listed = tree.app().join("vendor/acme/http").join(GREETING.0);
    fs::remove_file(&listed).expect("the test removes the listed file");
    if link_file(&secret, &listed).is_err() {
        // Windows makes a symlink only with a privilege this runner may not have.
        return;
    }
    let out = tree.nvs(&["check", "main.nvs"]);
    assert!(!out.status.success(), "a link out of the package compiled");
    assert!(shown(&out).contains("E0658"), "{}", shown(&out));
}
