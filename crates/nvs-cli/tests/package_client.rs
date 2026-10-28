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

use std::path::{Path, PathBuf};
use std::process::Command;

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
