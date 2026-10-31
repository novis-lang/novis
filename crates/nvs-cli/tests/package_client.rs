//! The package client's reading of `package.toml`
//! (`rule:packaging/package-toml-is-a-tool-input`, `rule:packaging/a-package-name-is-vendor-slash-name`),
//! and `nvs pkg pack`'s archive (`rule:packaging/a-package-is-its-digest`,
//! `rule:packaging/a-package-carries-only-its-source-and-what-it-lists`).
//!
//! `nvs pkg diff` reads each side's `package.toml`, so a file the reader refuses stops it with
//! `E0659` and the line, and a valid one gives its version. Each refusal of the reader is a unit
//! test in `src/package/manifest.rs`; these tests show the refusal reaches the user.
//!
//! The pack tests pin one fixture's digest, show the bytes do not depend on file times or on the
//! order the files were written in, and show each file-set refusal through the binary. Each path
//! rule and both limits are unit tests in `src/package/pack.rs`, which need no file on disk.
//! An extension package packs the fixture extension beside its source, and two extensions or an
//! extension whose class is outside the package's namespace are refused.
//!
//! The log tests show `nvs pkg fetch` reads an archive only once the registry's log proves it
//! (`rule:packaging/the-registry-keeps-an-append-only-log`): an index line whose record publishes
//! another digest is refused, a log that does not continue the checkpoint the data folder kept is
//! refused as forked, and a log that only grew is accepted. Each registry's log is signed by
//! `nvs registry log append` under a key `nvs registry key` makes, and its files by
//! `nvs registry index sign`. An unsigned index file is refused, and one signed eight days ago is
//! refused by `nvs pkg add` and read by `nvs pkg fetch --locked`. The proofs themselves are unit
//! tests in `src/package/log.rs`, and each refusal of a signed file is a unit test in
//! `src/package/index.rs`; these tests show the binary refuses what they do not prove.

use std::cell::{Cell, RefCell};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// A package directory under a fresh scratch folder, with `manifest` as its `package.toml` and one
/// class under `src`.
fn package(name: &str, manifest: &str) -> (nvs_repo::Scratch, PathBuf) {
    let root = nvs_repo::scratch(name);
    let dir = root.join("pkg");
    std::fs::create_dir_all(dir.join("src")).expect("the test makes its directories");
    std::fs::write(dir.join("package.toml"), manifest).expect("the test writes the manifest");
    std::fs::write(
        dir.join("src/Client.nvs"),
        "<?nvs\nnamespace Acme\\Http;\n\nfinal class Client\n{\n    public function get(): int { return 1; }\n}\n",
    )
    .expect("the test writes the class");
    (root, dir)
}

/// `nvs pkg diff <dir> <dir> --json`, as `(stdout, stderr, success)`.
fn diff(dir: &Path) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["pkg", "diff"])
        .arg(dir)
        .arg(dir)
        .arg("--json")
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// A whole package file reads: its version is the one both sides of the diff print.
// covers: tools:packages/package-toml
#[test]
fn a_full_package_toml_is_read() {
    let (_root, dir) = package(
        "package-client-full",
        "[package]\nname = 'Acme\\Http'\nversion = \"1.4.0\"\n\n\
         [dependencies]\n'Acme\\Log' = \"2.1.0\"\n\n\
         [registries]\nAcme = { url = \"file:registry\" }\n\n\
         [capabilities]\nrequired = [\"net.connect\"]\n",
    );
    let (stdout, stderr, ok) = diff(&dir);
    assert!(ok, "{stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is JSON");
    assert_eq!(report["from"], "1.4.0", "{stdout}");
}

/// An unknown key stops the command with `E0659`, naming the key.
// covers: tools:packages/package-toml
#[test]
fn an_unknown_key_is_refused() {
    let (_root, dir) = package(
        "package-client-unknown",
        "[package]\nname = 'Acme\\Http'\nversion = \"1.4.0\"\nhomepage = \"https://example.com\"\n",
    );
    let (_, stderr, ok) = diff(&dir);
    assert!(!ok, "accepted an unknown key");
    assert!(stderr.contains("E0659"), "{stderr}");
    assert!(stderr.contains("unknown field `homepage`"), "{stderr}");
}

/// A package cannot take a built-in component's namespace, in any case.
// covers: tools:packages/package-toml
#[test]
fn a_built_in_components_name_is_refused() {
    let (_root, dir) = package(
        "package-client-built-in",
        "[package]\nname = 'Novis\\IMAGE'\nversion = \"1.0.0\"\n",
    );
    let (_, stderr, ok) = diff(&dir);
    assert!(!ok, "accepted a built-in component's name");
    assert!(
        stderr.contains("a package cannot be named `Novis\\IMAGE`"),
        "{stderr}"
    );
}

/// A git dependency on a branch is refused: only an exact commit is allowed
/// (`rule:packaging/a-git-dependency-is-root-only`).
// covers: tools:packages/package-toml
#[test]
fn a_git_dependency_on_a_branch_is_refused() {
    let (_root, dir) = package(
        "package-client-branch",
        "[package]\nname = 'Acme\\Http'\nversion = \"1.0.0\"\n\n\
         [dependencies]\n'Acme\\Fork' = { git = \"https://git.example.com/fork.git\", rev = \"main\" }\n",
    );
    let (_, stderr, ok) = diff(&dir);
    assert!(!ok, "accepted a branch");
    assert!(stderr.contains("`main` is not a commit"), "{stderr}");
}

/// The manifest of the pack tests' fixture, with `files` as its `files` list.
fn pack_manifest(files: &str) -> String {
    format!("[package]\nname = 'Acme\\Http'\nversion = \"1.4.0\"\nfiles = [{files}]\n")
}

/// The pack tests' fixture: [`package`] with a license, a readme, a second class, a listed
/// template, and a test and a guide that stay out of the archive. `reversed` writes the same
/// files in the other order.
fn pack_fixture(name: &str, reversed: bool) -> (nvs_repo::Scratch, PathBuf) {
    let (root, dir) = package(name, &pack_manifest("\"templates/error.html\""));
    let mut files = vec![
        ("LICENSE", "MIT License\n"),
        ("README.md", "# Acme Http\n"),
        (
            "src/Sub/Retry.nvs",
            "<?nvs\nnamespace Acme\\Http\\Sub;\n\nfinal class Retry {}\n",
        ),
        ("templates/error.html", "<p>error</p>\n"),
        ("tests/ClientTest.nvs", "<?nvs\n"),
        ("docs/guide.md", "# Guide\n"),
    ];
    if reversed {
        files.reverse();
    }
    for (path, text) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().expect("a fixture file is in a folder"))
            .expect("the test makes its directories");
        std::fs::write(&path, text).expect("the test writes its files");
    }
    (root, dir)
}

/// `nvs pkg pack <dir> --out <out>`, as `(stdout, stderr, success)`.
fn pack(dir: &Path, out: &Path) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["pkg", "pack"])
        .arg(dir)
        .arg("--out")
        .arg(out)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// The fixture's digest. It changes only when the archive format changes, and that changes the
/// identity of every published package.
const FIXTURE_DIGEST: &str =
    "blake3:99c5249f351c6974ee2bc266f5e0765c853a9ec40aa710ca197ebc299d15ac69";

/// The fixture packs to the pinned digest, with every packed file printed, and the test and the
/// guide left out.
// covers: tools:packages/nvs-pkg-pack
#[test]
fn a_package_packs_to_its_pinned_digest() {
    let (root, dir) = pack_fixture("package-client-pack", false);
    let out = root.join("acme-http.tar");
    let (stdout, stderr, ok) = pack(&dir, &out);
    assert!(ok, "{stderr}");
    let listed: Vec<&str> = stdout
        .lines()
        .take(6)
        .filter_map(|line| line.split_whitespace().nth(1))
        .collect();
    assert_eq!(
        listed,
        [
            "LICENSE",
            "README.md",
            "package.toml",
            "src/Client.nvs",
            "src/Sub/Retry.nvs",
            "templates/error.html"
        ],
        "{stdout}"
    );
    assert!(stdout.contains("  total, 6 files"), "{stdout}");
    assert!(
        stdout.contains(&format!("acme/http 1.4.0  {FIXTURE_DIGEST}")),
        "{stdout}"
    );
    let bytes = std::fs::read(&out).expect("the archive was written");
    assert_eq!(
        format!("blake3:{}", blake3::hash(&bytes).to_hex()),
        FIXTURE_DIGEST
    );
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("ClientTest"), "a test was packed");
    assert!(!text.contains("# Guide"), "a guide was packed");
}

/// File times and the order the files were written in do not reach the bytes.
// covers: tools:packages/nvs-pkg-pack
#[test]
fn the_archive_does_not_depend_on_times_or_order() {
    let (root, first) = pack_fixture("package-client-pack-a", false);
    let (_other, second) = pack_fixture("package-client-pack-b", true);
    let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(86_400);
    std::fs::File::options()
        .write(true)
        .open(second.join("src/Client.nvs"))
        .and_then(|file| file.set_modified(old))
        .expect("the test sets a file's time");
    let (a, b) = (root.join("a.tar"), root.join("b.tar"));
    let (_, stderr, ok) = pack(&first, &a);
    assert!(ok, "{stderr}");
    let (_, stderr, ok) = pack(&second, &b);
    assert!(ok, "{stderr}");
    assert_eq!(
        std::fs::read(&a).expect("the first archive was written"),
        std::fs::read(&b).expect("the second archive was written"),
    );
}

/// The fixture with `manifest` as its `package.toml`, after `change` ran on its directory, does
/// not pack: the message contains `want`, and no archive is written.
fn pack_refuses(name: &str, manifest: &str, change: impl FnOnce(&Path), want: &str) {
    let (root, dir) = pack_fixture(name, false);
    std::fs::write(dir.join("package.toml"), manifest).expect("the test writes the manifest");
    change(&dir);
    let out = root.join("refused.tar");
    let (_, stderr, ok) = pack(&dir, &out);
    assert!(!ok, "packed: {want}");
    assert!(stderr.contains(want), "{stderr}");
    assert!(!out.exists(), "a refused pack wrote an archive");
}

#[test]
fn a_package_without_a_license_is_refused() {
    pack_refuses(
        "package-client-no-license",
        &pack_manifest("\"templates/error.html\""),
        |dir| std::fs::remove_file(dir.join("LICENSE")).expect("the test removes the license"),
        "the package has no license file",
    );
}

#[test]
fn a_missing_listed_file_is_refused() {
    pack_refuses(
        "package-client-missing",
        &pack_manifest("\"templates/missing.html\""),
        |_| {},
        "`templates/missing.html` does not exist.",
    );
}

#[test]
fn a_listed_nvs_file_is_refused() {
    pack_refuses(
        "package-client-listed-nvs",
        &pack_manifest("\"tests/ClientTest.nvs\""),
        |_| {},
        "`files` lists `tests/ClientTest.nvs`, a `.nvs` file.",
    );
}

#[test]
fn a_listed_default_file_is_refused() {
    pack_refuses(
        "package-client-listed-default",
        &pack_manifest("\"readme.md\""),
        |_| {},
        "`files` lists `readme.md`, which is packed without being listed.",
    );
}

#[test]
fn a_listed_nvs_toml_is_refused() {
    pack_refuses(
        "package-client-nvs-toml",
        &pack_manifest("\"config/nvs.toml\""),
        |dir| {
            std::fs::create_dir_all(dir.join("config")).expect("the test makes its directories");
            std::fs::write(dir.join("config/nvs.toml"), "").expect("the test writes the file");
        },
        "`config/nvs.toml` is a server configuration file.",
    );
}

/// A path rule is checked before the file is looked for, so a name Windows cannot create is
/// refused on every system.
#[test]
fn a_listed_path_windows_reserves_is_refused() {
    pack_refuses(
        "package-client-reserved",
        &pack_manifest("\"assets/aux.txt\""),
        |_| {},
        "a name Windows reserves",
    );
}

#[test]
fn an_application_is_not_packed() {
    pack_refuses(
        "package-client-application",
        "[grants]\n",
        |_| {},
        "Only a package can be packed.",
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_is_refused() {
    pack_refuses(
        "package-client-symlink",
        &pack_manifest("\"templates/error.html\""),
        |dir| {
            std::os::unix::fs::symlink("Client.nvs", dir.join("src/Link.nvs"))
                .expect("the test makes a symlink");
        },
        "`src/Link.nvs` is a symbolic link.",
    );
}

/// The fixture extension: the class `Shop\Ledger`, with the source file `Shop\Ledger\Receipt`.
fn ledger() -> Vec<u8> {
    fs::read(nvs_repo::path("tests/conformance/ext/fixtures/ledger.nvsx"))
        .expect("the fixture reads")
}

/// A package `namespace` 1.0.0 with a license, a class under `src` that calls the extension,
/// and the fixture extension at each path in `components`, each listed in `files`.
fn extension_package(
    name: &str,
    namespace: &str,
    components: &[&str],
) -> (nvs_repo::Scratch, PathBuf) {
    let root = nvs_repo::scratch(name);
    let dir = root.join("pkg");
    fs::create_dir_all(dir.join("src")).expect("the test makes its directories");
    fs::write(
        dir.join("package.toml"),
        format!("[package]\nname = '{namespace}'\nversion = \"1.0.0\"\nfiles = {components:?}\n"),
    )
    .expect("the test writes the manifest");
    fs::write(dir.join("LICENSE"), "MIT\n").expect("the test writes the license");
    fs::write(
        dir.join("src/Report.nvs"),
        format!(
            "<?nvs\nnamespace {namespace};\n\nfinal class Report\n{{\n    \
             public static function total(int $cents): int\n    {{\n        \
             return Shop\\Ledger::echoInt($cents);\n    }}\n}}\n"
        ),
    )
    .expect("the test writes the class");
    for component in components {
        let path = dir.join(component);
        fs::create_dir_all(path.parent().expect("a component is in a folder"))
            .expect("the test makes its directories");
        fs::write(path, ledger()).expect("the test copies the extension");
    }
    (root, dir)
}

/// A package whose `files` list names one `.nvsx` packs it beside its source, and the archive
/// has both.
// covers: tools:packages/nvs-pkg-pack
#[test]
fn an_extension_package_packs_its_component_and_its_source() {
    let (root, dir) =
        extension_package("package-client-extension", "Shop\\Ledger", &["ledger.nvsx"]);
    let out = root.join("shop-ledger.tar");
    let (stdout, stderr, ok) = pack(&dir, &out);
    assert!(ok, "{stderr}");
    let listed: Vec<&str> = stdout
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .collect();
    for path in ["ledger.nvsx", "src/Report.nvs", "package.toml"] {
        assert!(listed.contains(&path), "`{path}` is not packed: {stdout}");
    }
    let size = format!("{}  ledger.nvsx", ledger().len());
    assert!(
        stdout.lines().any(|line| line.trim_start() == size),
        "the extension is packed whole: {stdout}"
    );
}

// covers: tools:packages/nvs-pkg-pack
#[test]
fn a_package_with_two_components_is_refused() {
    let (root, dir) = extension_package(
        "package-client-two-extensions",
        "Shop\\Ledger",
        &["ledger.nvsx", "extra/second.nvsx"],
    );
    let out = root.join("refused.tar");
    let (_, stderr, ok) = pack(&dir, &out);
    assert!(!ok, "a package with two extensions packed");
    assert!(
        stderr
            .contains("the package has two extension files, `ledger.nvsx` and `extra/second.nvsx`"),
        "{stderr}"
    );
    assert!(!out.exists(), "a refused pack wrote an archive");
}

// covers: tools:packages/nvs-pkg-pack
#[test]
fn a_component_whose_class_is_outside_the_package_namespace_is_refused() {
    let (root, dir) = extension_package(
        "package-client-extension-outside",
        "Acme\\Ledger",
        &["ledger.nvsx"],
    );
    let out = root.join("refused.tar");
    let (_, stderr, ok) = pack(&dir, &out);
    assert!(!ok, "an extension outside its package's namespace packed");
    assert!(
        stderr.contains(
            "the extension `ledger.nvsx` declares the class `Shop\\Ledger`, which is outside \
             the package's namespace `Acme\\Ledger`"
        ),
        "{stderr}"
    );
    assert!(!out.exists(), "a refused pack wrote an archive");
}

/// The name of the log key of [`Logged`], which is also its log's origin.
const KEY_NAME: &str = "registry.example.com";

/// The repository every publish record of [`Logged`] names.
const REPOSITORY: &str = "https://git.example.com/acme/http";

/// A `file:` registry of `Acme\Http` whose log the binary signs, applications beside it, and the
/// private data folder every application's commands share.
struct Logged {
    root: nvs_repo::Scratch,
    data: nvs_repo::Scratch,
    /// The private folder of the log's signing key.
    keys: nvs_repo::Scratch,
    /// The verifier key every application's registry line pins.
    key: String,
    /// The number of records in the registry's log.
    records: Cell<u64>,
    /// The index lines of `Acme\Http`.
    versions: RefCell<Vec<serde_json::Value>>,
}

impl Logged {
    /// An empty registry with a new log key.
    fn new(name: &str) -> Self {
        let keys = nvs_repo::scratch_private(&format!("{name}-key"));
        let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["registry", "key"])
            .arg(keys.join("log.key"))
            .args(["--name", KEY_NAME])
            .output()
            .expect("the `nvs` binary this test was built beside runs");
        assert!(out.status.success(), "{}", shown(&out));
        let logged = Self {
            root: nvs_repo::scratch(name),
            data: nvs_repo::scratch_private(&format!("{name}-data")),
            key: String::from_utf8_lossy(&out.stdout).trim().to_owned(),
            keys,
            records: Cell::new(0),
            versions: RefCell::default(),
        };
        fs::create_dir_all(logged.registry()).expect("the test makes the registry");
        fs::write(
            logged.registry().join("registry.json"),
            r#"{ "schema": 1, "hosts": [] }"#,
        )
        .expect("the test writes the root file");
        logged
    }

    fn registry(&self) -> PathBuf {
        self.root.join("registry")
    }

    /// The registry's URL. It is absolute, so every application names the registry the same
    /// way, and they share the checkpoint the data folder keeps for it.
    fn url(&self) -> String {
        format!("file:{}", self.registry().display())
    }

    /// A new application folder `name` beside the registry, which needs `Acme\Http` at `minimum`.
    fn app(&self, name: &str, minimum: &str) -> PathBuf {
        let app = self.root.join(name);
        fs::create_dir_all(&app).expect("the test makes the application");
        fs::write(
            app.join("package.toml"),
            format!(
                "[dependencies]\n'Acme\\Http' = \"{minimum}\"\n\n\
                 [registries]\nAcme = {{ url = '{}', key = \"{}\" }}\n",
                self.url(),
                self.key
            ),
        )
        .expect("the test writes the manifest");
        app
    }

    /// `Acme\Http` at `version`, packed by the binary into the registry's archive folder. Its
    /// digest in hex.
    fn pack(&self, version: &str) -> String {
        let dir = self.root.join(format!("packages/acme-http-{version}"));
        fs::create_dir_all(dir.join("src")).expect("the test makes the package");
        fs::write(
            dir.join("package.toml"),
            format!("[package]\nname = 'Acme\\Http'\nversion = \"{version}\"\n"),
        )
        .expect("the test writes the package's manifest");
        fs::write(dir.join("LICENSE"), "MIT\n").expect("the test writes the license");
        fs::write(
            dir.join("src/Main.nvs"),
            "<?nvs\nnamespace Acme\\Http;\n\nfinal class Main\n{\n}\n",
        )
        .expect("the test writes the class");
        let tar = self.root.join(format!("packages/acme-http-{version}.tar"));
        let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["pkg", "pack"])
            .arg(&dir)
            .arg("--out")
            .arg(&tar)
            .output()
            .expect("the `nvs` binary this test was built beside runs");
        assert!(out.status.success(), "{}", shown(&out));
        let bytes = fs::read(&tar).expect("the archive was written");
        let digest = blake3::hash(&bytes).to_hex().to_string();
        let archive = self.registry().join("archive");
        fs::create_dir_all(&archive).expect("the test makes the archive folder");
        fs::write(archive.join(format!("{digest}.tar")), &bytes).expect("the test copies it");
        digest
    }

    /// A publish record of `package` at `version` with `digest`, appended to the registry's log
    /// by the binary. Its record number.
    fn append(&self, package: &str, version: &str, digest: &str) -> u64 {
        let entry = serde_json::json!({
            "v": 1,
            "kind": "publish",
            "package": package,
            "version": version,
            "digest": digest,
            "repository": REPOSITORY,
            "commit": "0".repeat(40),
        });
        let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(["registry", "log", "append"])
            .arg(self.registry().join("log"))
            .arg("--key")
            .arg(self.keys.join("log.key"))
            .args(["--origin", KEY_NAME])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the `nvs` binary this test was built beside runs");
        child
            .stdin
            .take()
            .expect("the child's input is piped")
            .write_all(format!("{entry}\n").as_bytes())
            .expect("the test writes the entry");
        let out = child.wait_with_output().expect("the append ends");
        assert!(out.status.success(), "{}", shown(&out));
        let n = self.records.get();
        self.records.set(n + 1);
        n
    }

    /// The index of `Acme\Http` with a released line for `version` added, its archive `digest`
    /// and its log record `record`.
    fn list(&self, version: &str, digest: &str, record: u64) {
        let mut versions = self.versions.borrow_mut();
        versions.push(serde_json::json!({
            "version": version,
            "digest": digest,
            "state": "released",
            "dependencies": {},
            "capabilities": { "required": [], "optional": [] },
            "repository": REPOSITORY,
            "commit": "0".repeat(40),
            "log": record,
        }));
        let index = serde_json::json!({
            "schema": 1,
            "name": "Acme\\Http",
            "owners": ["owner"],
            "versions": *versions,
        });
        let path = self.registry().join("index/acme/http.json");
        fs::create_dir_all(path.parent().expect("an index file has a folder"))
            .expect("the test makes the index folder");
        fs::write(path, index.to_string()).expect("the test writes the index");
        self.sign(None);
    }

    /// The root file and the index signed by the binary with the log's key, at the Unix time
    /// `at`, or now.
    fn sign(&self, at: Option<u64>) {
        let mut sign = Command::new(env!("CARGO_BIN_EXE_nvs"));
        sign.args(["registry", "index", "sign"])
            .arg(self.registry())
            .arg("--key")
            .arg(self.keys.join("log.key"));
        if let Some(at) = at {
            sign.args(["--at", &at.to_string()]);
        }
        let out = sign
            .output()
            .expect("the `nvs` binary this test was built beside runs");
        assert!(out.status.success(), "{}", shown(&out));
    }

    /// `Acme\Http` at `version`, packed, published in the log and listed in the index.
    fn publish(&self, version: &str) {
        let digest = self.pack(version);
        let record = self.append("acme/http", version, &digest);
        self.list(version, &digest, record);
    }

    /// The log and the index emptied, so the next records start a new history under the same
    /// key.
    fn forget(&self) {
        fs::remove_dir_all(self.registry().join("log")).expect("the test removes the log");
        self.records.set(0);
        self.versions.borrow_mut().clear();
    }

    /// `nvs <args>` in `app`, with the shared data folder.
    fn nvs(&self, app: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--data")
            .arg(&*self.data)
            .arg("--no-init")
            .args(args)
            .current_dir(app)
            .output()
            .expect("the `nvs` binary this test was built beside runs")
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

/// An index line whose log record publishes another digest is not fetched, even though its
/// archive is on the host and matches the index.
// covers: tools:packages/the-transparency-log
#[test]
fn an_archive_absent_from_the_log_is_refused() {
    let logged = Logged::new("package-client-log-absent");
    let digest = logged.pack("1.0.0");
    let record = logged.append("acme/http", "1.0.0", &"ab".repeat(32));
    logged.list("1.0.0", &digest, record);
    let app = logged.app("app", "1.0.0");

    let out = logged.nvs(&app, &["pkg", "fetch"]);
    assert!(
        !out.status.success(),
        "fetched an archive the log does not publish"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("is not in the log of the registry"),
        "{stderr}"
    );
    assert!(
        !app.join("vendor/acme/http").exists(),
        "the package was written"
    );
}

/// A registry whose log no longer continues the checkpoint this computer kept is refused as
/// forked, and nothing is fetched from it.
// covers: tools:packages/the-transparency-log
#[test]
fn a_forked_log_is_refused() {
    let logged = Logged::new("package-client-log-forked");
    logged.publish("1.0.0");
    let first = logged.app("first", "1.0.0");
    let out = logged.nvs(&first, &["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));

    logged.forget();
    logged.append("acme/other", "1.0.0", &"cd".repeat(32));
    logged.publish("1.0.0");
    let second = logged.app("second", "1.0.0");
    let out = logged.nvs(&second, &["pkg", "fetch"]);
    assert!(!out.status.success(), "fetched from a forked log");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("was forked"), "{stderr}");
    assert!(stderr.contains(&logged.url()), "{stderr}");
    assert!(
        !second.join("vendor/acme/http").exists(),
        "the package was written"
    );
}

/// An index file signed eight days ago is not used by `nvs pkg add`, which names the registry
/// and the time, and `nvs pkg fetch --locked` still builds from the lock with it.
// covers: tools:packages/the-transparency-log
#[test]
fn an_old_index_file_is_refused_for_add_and_read_for_a_locked_fetch() {
    let logged = Logged::new("package-client-index-old");
    logged.publish("1.0.0");
    let app = logged.app("app", "1.0.0");
    let out = logged.nvs(&app, &["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is past 1970")
        .as_secs();
    logged.sign(Some(now - 8 * 24 * 60 * 60));
    let out = logged.nvs(&app, &["pkg", "add", "acme/http"]);
    assert!(!out.status.success(), "added from an old index file");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("more than seven days ago"), "{stderr}");
    assert!(stderr.contains(&logged.url()), "{stderr}");
    assert!(stderr.contains(" UTC"), "{stderr}");

    fs::remove_dir_all(app.join("vendor")).expect("the test removes the tree");
    let out = logged.nvs(&app, &["pkg", "fetch", "--locked"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert!(app.join("vendor/acme/http/src/Main.nvs").is_file());
}

/// An index file with no signature is not used, and nothing is fetched.
// covers: tools:packages/the-transparency-log
#[test]
fn an_unsigned_index_file_is_refused() {
    let logged = Logged::new("package-client-index-unsigned");
    logged.publish("1.0.0");
    let path = logged.registry().join("index/acme/http.json");
    let note = fs::read_to_string(&path).expect("the index was signed");
    let text = note
        .split("\n\n")
        .next()
        .expect("a note has text")
        .to_owned();
    fs::write(&path, text).expect("the test removes the signature");
    let app = logged.app("app", "1.0.0");

    let out = logged.nvs(&app, &["pkg", "fetch"]);
    assert!(!out.status.success(), "fetched with an unsigned index");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("signature"), "{stderr}");
    assert!(
        !app.join("vendor/acme/http").exists(),
        "the package was written"
    );
}

/// A log that only grew since the kept checkpoint is accepted, and its new version is fetched.
// covers: tools:packages/the-transparency-log
#[test]
fn a_log_that_grew_is_accepted() {
    let logged = Logged::new("package-client-log-grew");
    logged.publish("1.0.0");
    let first = logged.app("first", "1.0.0");
    let out = logged.nvs(&first, &["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));

    logged.publish("1.1.0");
    let second = logged.app("second", "1.1.0");
    let out = logged.nvs(&second, &["pkg", "fetch"]);
    assert!(out.status.success(), "{}", shown(&out));
    assert!(second.join("vendor/acme/http/src/Main.nvs").is_file());
    assert!(
        fs::read_to_string(second.join("package.lock"))
            .unwrap_or_default()
            .contains("1.1.0"),
        "the lock does not name 1.1.0"
    );
}
