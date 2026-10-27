//! `rule:packaging/a-package-file-stays-under-its-declared-prefix` and
//! `rule:packaging/a-package-cannot-reach-its-host`, on package layouts each test builds by hand:
//! an application whose `vendor/acme/http` holds a `package.toml` naming `Acme\Http`.
//!
//! Each refusal has a positive control beside it, the same layout with the line that breaks it
//! put right, so a refusal that fires on every package cannot pass.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The package's manifest.
const MANIFEST: &str = "[package]\nname = 'Acme\\Http'\nversion = \"1.0.0\"\n\n\
                        [capabilities]\nrequired = [\"net.connect\"]\n";

/// A fresh application directory, `app`, with `files` written under it, each `(path, text)`.
fn layout(files: &[(&str, &str)]) -> (nvs_repo::Scratch, PathBuf) {
    let root = nvs_repo::scratch("package-grants");
    let app = root.join("app");
    for (path, text) in files {
        let path = app.join(path);
        std::fs::create_dir_all(path.parent().expect("every file is in a directory"))
            .expect("the test makes its directories");
        std::fs::write(&path, text).expect("the test writes its files");
    }
    (root, app)
}

/// `nvs check <entry>`, as `(stderr, success)`.
fn check(entry: &Path) -> (String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("check")
        .arg(entry)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// An entry that autoloads `Acme\Http` from the package, and a client class declared in
/// `namespace`.
fn client_in(namespace: &str, manifest: Option<&str>) -> (String, bool) {
    let class = format!("<?nvs\nnamespace {namespace};\n\nclass Client {{}}\n");
    let entry = format!(
        "<?nvs\nautoload '{namespace}' from 'vendor/acme/http/src';\n\nvar $client = new {namespace}\\Client();\n"
    );
    let mut files = vec![
        ("main.nvs", entry.as_str()),
        ("vendor/acme/http/src/Client.nvs", class.as_str()),
    ];
    if let Some(manifest) = manifest {
        files.push(("vendor/acme/http/package.toml", manifest));
    }
    let (_root, app) = layout(&files);
    check(&app.join("main.nvs"))
}

#[test]
fn a_package_file_that_declares_a_namespace_outside_its_package_does_not_compile() {
    let (stderr, ok) = client_in("Acme\\Http", Some(MANIFEST));
    assert!(
        ok,
        "a file under the package's namespace compiles:\n{stderr}"
    );

    let (stderr, ok) = client_in("App", Some(MANIFEST));
    assert!(!ok, "a package file in `App` does not compile");
    assert!(stderr.contains("E0660"), "{stderr}");
    assert!(
        stderr.contains("Client.nvs")
            && stderr.contains("`App`")
            && stderr.contains("`Acme\\Http`"),
        "the message names the file, the namespace and the package:\n{stderr}"
    );

    let (stderr, ok) = client_in("Acme\\Httpx", Some(MANIFEST));
    assert!(
        !ok && stderr.contains("E0660"),
        "a namespace that only starts with the package's name is outside it:\n{stderr}"
    );
}

#[test]
fn a_relative_entry_path_still_places_files_in_their_package() {
    let class = "<?nvs\nnamespace App;\n\nclass Client {}\n";
    let entry = "<?nvs\nautoload 'App' from 'vendor/acme/http/src';\n\nvar $client = new App\\Client();\n";
    let (_root, app) = layout(&[
        ("main.nvs", entry),
        ("vendor/acme/http/src/Client.nvs", class),
        ("vendor/acme/http/package.toml", MANIFEST),
    ]);
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .current_dir(&app)
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("check")
        .arg("main.nvs")
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success() && stderr.contains("E0660"),
        "`nvs check main.nvs` checks the package's files as an absolute entry does:\n{stderr}"
    );
}

#[test]
fn code_with_no_package_name_gets_no_file_check() {
    let (stderr, ok) = client_in("App", None);
    assert!(ok, "hand-copied code with no manifest compiles:\n{stderr}");

    let (stderr, ok) = client_in("App", Some("[grants]\n"));
    assert!(
        ok,
        "a manifest with no `[package] name` names no package:\n{stderr}"
    );
}

#[test]
fn a_bad_package_name_is_refused() {
    let (stderr, ok) = client_in("Acme\\Http", Some("[package]\nname = 'Acme'\n"));
    assert!(!ok && stderr.contains("E0659"), "{stderr}");

    let (stderr, ok) = client_in("Acme\\Http", Some("[package]\nname = 'core\\Http'\n"));
    assert!(!ok && stderr.contains("E0659"), "{stderr}");

    let both = "[package]\nname = 'Acme\\Http'\n[capabilities]\nrequired = [\"fs.read\"]\n\
                optional = [\"fs.read\"]\n";
    let (stderr, ok) = client_in("Acme\\Http", Some(both));
    assert!(!ok && stderr.contains("E0659"), "{stderr}");
}

/// An entry that requires the package's `boot.nvs`, which is `boot`, beside `secret.nvs` and
/// `lib/` in the application.
fn boot(boot: &str) -> (String, bool) {
    let (_root, app) = layout(&[
        ("main.nvs", "<?nvs\nrequire 'vendor/acme/http/boot.nvs';\n"),
        ("secret.nvs", "<?nvs\nnamespace App;\n\nclass Secret {}\n"),
        ("lib/Note.nvs", "<?nvs\nnamespace App;\n\nclass Note {}\n"),
        ("vendor/acme/http/package.toml", MANIFEST),
        (
            "vendor/acme/http/own.nvs",
            "<?nvs\nnamespace Acme\\Http;\n\nclass Own {}\n",
        ),
        (
            "vendor/acme/http/src/Note.nvs",
            "<?nvs\nnamespace Acme\\Http;\n\nclass Note {}\n",
        ),
        ("vendor/acme/http/boot.nvs", boot),
    ]);
    let boot = boot.replace("{app}", &app.display().to_string().replace('\\', "/"));
    std::fs::write(app.join("vendor/acme/http/boot.nvs"), boot).expect("the test writes boot");
    check(&app.join("main.nvs"))
}

#[test]
fn a_require_path_that_leaves_the_package_does_not_compile() {
    let (stderr, ok) = boot("<?nvs\nnamespace Acme\\Http;\n\nrequire 'own.nvs';\n");
    assert!(ok, "a require inside the package compiles:\n{stderr}");

    let (stderr, ok) = boot("<?nvs\nnamespace Acme\\Http;\n\nrequire '../../../secret.nvs';\n");
    assert!(
        !ok && stderr.contains("E0661"),
        "`..` leaves the package:\n{stderr}"
    );
    assert!(stderr.contains("secret.nvs"), "{stderr}");

    let (stderr, ok) = boot("<?nvs\nnamespace Acme\\Http;\n\nrequire '{app}/secret.nvs';\n");
    assert!(
        !ok && stderr.contains("E0661"),
        "an absolute path leaves the package:\n{stderr}"
    );
}

#[test]
fn an_autoload_path_that_leaves_the_package_does_not_compile() {
    let (stderr, ok) = boot("<?nvs\nnamespace Acme\\Http;\n\nautoload 'Acme\\Http' from 'src';\n");
    assert!(
        ok,
        "an autoload root inside the package compiles:\n{stderr}"
    );

    let (stderr, ok) =
        boot("<?nvs\nnamespace Acme\\Http;\n\nautoload 'Acme\\Http' from '../../../lib';\n");
    assert!(
        !ok && stderr.contains("E0661"),
        "`..` leaves the package:\n{stderr}"
    );
}

#[test]
fn a_path_through_a_symlink_out_of_the_package_is_outside_it() {
    // A filesystem on which `/app/vendor/acme/http/link.nvs` is a symlink to `/app/secret.nvs`.
    // Windows creates a symlink only with a privilege a test does not have, so a canonicalizer
    // that resolves the link stands in for the filesystem.
    let package = Path::new("/app/vendor/acme/http");
    let fake = |path: &Path| {
        Some(if path == package.join("link.nvs") {
            PathBuf::from("/app/secret.nvs")
        } else {
            path.to_path_buf()
        })
    };
    assert!(nvs_types::package::within(
        package,
        &package.join("own.nvs"),
        fake
    ));
    assert!(
        !nvs_types::package::within(package, &package.join("link.nvs"), fake),
        "a symlink in the package does not carry the package to its target"
    );
    assert!(
        !nvs_types::package::within(package, Path::new("/app/vendor/acme/httpx/a.nvs"), fake),
        "a directory whose name only starts the same way is outside the package"
    );
}
