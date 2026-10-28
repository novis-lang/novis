//! The package client's reading of `package.toml`
//! (`rule:packaging/package-toml-is-a-tool-input`, `rule:packaging/a-package-name-is-vendor-slash-name`).
//!
//! `nvs pkg diff` reads each side's `package.toml`, so a file the reader refuses stops it with
//! `E0659` and the line, and a valid one gives its version. Each refusal of the reader is a unit
//! test in `src/package/manifest.rs`; these tests show the refusal reaches the user.

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
