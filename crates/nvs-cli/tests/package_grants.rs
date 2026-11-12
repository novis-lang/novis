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
    check_from(entry, None)
}

/// `nvs check <entry>` run in `dir` where it is `Some`, so the `nvs.toml` there is the
/// configuration, as `(stderr, success)`.
fn check_from(entry: &Path, dir: Option<&Path>) -> (String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    let out = command
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

// covers: tools:packages/grants-what-each-namespace-may-do
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
    let entry =
        "<?nvs\nautoload 'App' from 'vendor/acme/http/src';\n\nvar $client = new App\\Client();\n";
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

/// An entry that requires `vendor/acme/http/src/Inner/Run.nvs`, declared in `namespace`, in the
/// package `Acme\Http` whose `src/Inner/` holds a second manifest, `inner`.
fn inner_manifest(inner: &str, namespace: &str) -> (String, bool) {
    let run = format!("<?nvs\nnamespace {namespace};\n\nclass Run {{}}\n");
    let (_root, app) = layout(&[
        (
            "main.nvs",
            "<?nvs\nrequire 'vendor/acme/http/src/Inner/Run.nvs';\n",
        ),
        ("vendor/acme/http/package.toml", MANIFEST),
        ("vendor/acme/http/src/Inner/package.toml", inner),
        ("vendor/acme/http/src/Inner/Run.nvs", run.as_str()),
    ]);
    check(&app.join("main.nvs"))
}

#[test]
fn a_second_manifest_with_no_name_does_not_end_the_namespace_check() {
    let unnamed = "# a manifest with no name\n";
    let (stderr, ok) = inner_manifest(unnamed, "Acme\\Http\\Inner");
    assert!(
        ok,
        "a file below the second manifest in the package's namespace compiles:\n{stderr}"
    );

    let (stderr, ok) = inner_manifest(unnamed, "Acme\\HTTP");
    assert!(!ok && stderr.contains("E0660"), "{stderr}");
    assert!(
        stderr.contains("`Acme\\HTTP`") && stderr.contains("the package `Acme\\Http`"),
        "the file is still checked against the outer package:\n{stderr}"
    );
}

#[test]
fn a_second_manifest_with_a_name_makes_no_second_package() {
    let named = "[package]\nname = 'Free\\Hands'\n";
    let (stderr, ok) = inner_manifest(named, "Acme\\Http\\Inner");
    assert!(
        ok,
        "a file below the second manifest in the outer package's namespace compiles:\n{stderr}"
    );

    let (stderr, ok) = inner_manifest(named, "Free\\Hands");
    assert!(!ok && stderr.contains("E0660"), "{stderr}");
    assert!(
        stderr.contains("`Free\\Hands`") && stderr.contains("the package `Acme\\Http`"),
        "the inner manifest names no package of its own:\n{stderr}"
    );
}

/// The four members that read the process environment or a deployment credential, one call each.
const ENV_READS: &str = "var $one = Core\\Env::get('HOME');\n\
                         var $key = Core\\Env::secret('HOME');\n\
                         var $all = Core\\Env::all();\n\
                         var $token = Core\\Config::secret('token');\n";

/// `ENV_READS` in a method of `Acme\Http\Client`, an entry that calls it, and `grants` as the
/// application's `package.toml` where it is `Some`.
fn reads_the_environment(grants: Option<&str>) -> (String, bool) {
    let class = format!(
        "<?nvs\nnamespace Acme\\Http;\n\nfinal class Client\n{{\n    \
         public static function read(): void\n    {{\n{ENV_READS}    }}\n}}\n"
    );
    let entry = "<?nvs\nautoload 'Acme\\Http' from 'vendor/acme/http/src';\n\n\
                 Acme\\Http\\Client::read();\n";
    let mut files = vec![
        ("main.nvs", entry),
        ("vendor/acme/http/src/Client.nvs", class.as_str()),
        (
            "vendor/acme/http/package.toml",
            "[package]\nname = 'Acme\\Http'\nversion = \"1.0.0\"\n",
        ),
    ];
    if let Some(grants) = grants {
        files.push(("package.toml", grants));
    }
    let (_root, app) = layout(&files);
    check(&app.join("main.nvs"))
}

// covers: tools:packages/grants-what-each-namespace-may-do
#[test]
fn a_package_with_an_empty_grant_line_does_not_read_the_environment() {
    let (stderr, ok) = reads_the_environment(Some("[grants]\n'Acme\\Http\\*' = []\n"));
    assert!(
        !ok,
        "a package with an empty line reads the environment:\n{stderr}"
    );
    for member in [
        "Core\\Env::get",
        "Core\\Env::secret",
        "Core\\Env::all",
        "Core\\Config::secret",
    ] {
        assert!(
            stderr.contains(&format!("`{member}` needs `env`")),
            "`{member}` is not refused for `env`:\n{stderr}"
        );
    }
    assert_eq!(stderr.matches("error[E0658]").count(), 4, "{stderr}");

    let (stderr, ok) = reads_the_environment(Some("[grants]\n'Acme\\Http\\*' = [\"env\"]\n"));
    assert!(
        ok,
        "a line that lists `env` reads the environment:\n{stderr}"
    );
}

/// `body` in a method of `Acme\Http\Client`, beside a `job.nvs` in the same folder and a static
/// `Client::job`, an entry that calls it, and the application's `package.toml` granting `line` to
/// `Acme\Http\*`.
fn starts_a_script(body: &str, line: &str) -> (String, bool) {
    let class = format!(
        "<?nvs\nnamespace Acme\\Http;\n\nfinal class Client\n{{\n    \
         public static function start(): void\n    {{\n        {body}\n    }}\n\n    \
         public static function job(): void\n    {{\n    }}\n}}\n"
    );
    let grants = format!("[grants]\n'Acme\\Http\\*' = [{line}]\n");
    let entry = "<?nvs\nautoload 'Acme\\Http' from 'vendor/acme/http/src';\n\n\
                 Acme\\Http\\Client::start();\n";
    let (_root, app) = layout(&[
        ("main.nvs", entry),
        ("vendor/acme/http/src/Client.nvs", class.as_str()),
        ("vendor/acme/http/src/job.nvs", "<?nvs\necho 'job';\n"),
        (
            "vendor/acme/http/package.toml",
            "[package]\nname = 'Acme\\Http'\nversion = \"1.0.0\"\n",
        ),
        ("package.toml", grants.as_str()),
    ]);
    check(&app.join("main.nvs"))
}

/// `body` does not compile in a package whose line is empty, and the refusal is one `E0658`
/// saying that `what` needs `script.spawn`. The same body compiles where the line lists it.
fn needs_script_spawn(body: &str, what: &str) {
    let (stderr, ok) = starts_a_script(body, "");
    assert!(
        !ok,
        "a package with an empty line compiles `{body}`:\n{stderr}"
    );
    assert!(
        stderr.contains(&format!("{what} needs `script.spawn`")),
        "`{body}` is not refused for `script.spawn`:\n{stderr}"
    );
    assert_eq!(stderr.matches("error[E0658]").count(), 1, "{stderr}");

    let (stderr, ok) = starts_a_script(body, "\"script.spawn\"");
    assert!(
        ok,
        "a line that lists `script.spawn` compiles `{body}`:\n{stderr}"
    );
}

// covers: tools:packages/grants-what-each-namespace-may-do
#[test]
fn a_package_with_an_empty_grant_line_does_not_spawn_a_script() {
    needs_script_spawn("var $job = spawn script 'job.nvs';", "`spawn script`");
}

// covers: tools:packages/grants-what-each-namespace-may-do
#[test]
fn a_package_with_an_empty_grant_line_does_not_spawn_a_method() {
    needs_script_spawn("var $job = spawn script Client::job;", "`spawn script`");
}

// covers: tools:packages/grants-what-each-namespace-may-do
#[test]
fn a_package_with_an_empty_grant_line_does_not_enqueue_a_job() {
    needs_script_spawn("Core\\Queue::push('job.nvs');", "`Core\\Queue::push`");
}

// covers: tools:packages/grants-what-each-namespace-may-do
#[test]
fn a_package_with_an_empty_grant_line_does_not_upgrade_a_connection() {
    needs_script_spawn(
        "Core\\Socket::upgrade('job.nvs');",
        "`Core\\Socket::upgrade`",
    );
    needs_script_spawn("Core\\Sse::upgrade('job.nvs');", "`Core\\Sse::upgrade`");
}

// covers: tools:packages/grants-what-each-namespace-may-do
#[test]
fn the_application_reads_the_environment_with_no_line_written() {
    let entry = format!("<?nvs\n{ENV_READS}");
    for grants in [None, Some("[grants]\n'Acme\\Http\\*' = []\n")] {
        let (_root, app) = match grants {
            Some(grants) => layout(&[("main.nvs", &entry), ("package.toml", grants)]),
            None => layout(&[("main.nvs", &entry)]),
        };
        let (stderr, ok) = check(&app.join("main.nvs"));
        assert!(
            ok,
            "the application reads the environment with no line for it ({grants:?}):\n{stderr}"
        );
    }
}

/// The manifest of the test extension `Shop\Meter`, with `requests` as its `requests` object.
fn meter_manifest(requests: &str) -> String {
    format!(
        "{{\"manifest\": 1, \"world\": \"1.0.0\", \"class\": \"Shop\\\\Meter\", \
         \"interface\": \"shop:meter/api\", \"requests\": {requests}, \
         \"methods\": [{{\"name\": \"spin\", \"params\": [{{\"name\": \"milliseconds\", \"type\": \"int\"}}]}}]}}"
    )
}

/// The `Shop\Meter` fixture's component, packed with `manifest` and the Novis source `files`, each
/// `(path, text)`.
fn meter(manifest: &str, files: &[(&str, &str)]) -> Vec<u8> {
    let dir = nvs_repo::path("tests/conformance/ext/fixtures/meter");
    let read = |name: &str| {
        std::fs::read_to_string(dir.join(name))
            .expect("the meter fixture reads")
            .replace("\r\n", "\n")
    };
    let module = wat::parse_str(read("module.wat")).expect("the meter module compiles");
    let wit = read("api.wit");
    let files: Vec<nvs_ext::source::SourceFile> = files
        .iter()
        .map(|(path, text)| nvs_ext::source::SourceFile {
            path: (*path).to_owned(),
            text: (*text).to_owned(),
        })
        .collect();
    nvs_ext::pack::pack(&nvs_ext::pack::Inputs {
        wasm: &module,
        wit: &[("api.wit", &wit)],
        manifest: manifest.as_bytes(),
        files: &files,
    })
    .expect("the meter extension packs")
}

/// `nvs check` of an application that loads `extension` as `meter.nvsx`, with `files` written
/// under it, each `(path, text)`.
fn check_with_extension(extension: &[u8], files: &[(&str, &str)]) -> (String, bool) {
    let toml = format!(
        "[[extension]]\npath = 'meter.nvsx'\nsha256 = \"{}\"\n",
        nvs_ext::load::pin(extension)
    );
    let mut files = files.to_vec();
    files.push(("nvs.toml", &toml));
    let (_root, app) = layout(&files);
    std::fs::write(app.join("meter.nvsx"), extension).expect("the test writes the extension");
    check_from(&app.join("main.nvs"), Some(&app))
}

/// A package `Acme\Http` whose `Client::run` calls `Shop\Meter::spin`, an entry that calls it,
/// and the application's `package.toml` granting `line` to `Acme\Http\*`.
fn package_calls_meter(extension: &[u8], line: &str) -> (String, bool) {
    let class = "<?nvs\nnamespace Acme\\Http;\n\nfinal class Client\n{\n    \
                 public static function run(): void\n    {\n        Shop\\Meter::spin(1);\n    }\n}\n";
    let entry = "<?nvs\nautoload 'Acme\\Http' from 'vendor/acme/http/src';\n\n\
                 Acme\\Http\\Client::run();\n";
    let grants = format!("[grants]\n'Acme\\Http\\*' = {line}\n");
    check_with_extension(
        extension,
        &[
            ("main.nvs", entry),
            ("vendor/acme/http/src/Client.nvs", class),
            (
                "vendor/acme/http/package.toml",
                "[package]\nname = 'Acme\\Http'\nversion = \"1.0.0\"\n",
            ),
            ("package.toml", &grants),
        ],
    )
}

// covers: tools:config/extension-grants-what-a-component-may-reach
#[test]
fn a_package_with_an_empty_grant_line_does_not_call_an_extension_that_requests_io() {
    let extension = meter(
        &meter_manifest(r#"{"read": ["data/"], "connect": ["tiles.example.com"]}"#),
        &[],
    );
    let (stderr, ok) = package_calls_meter(&extension, "[]");
    assert!(
        !ok,
        "a package with an empty line calls the extension:\n{stderr}"
    );
    assert!(
        stderr.contains("error[E0658]")
            && stderr.contains("`Shop\\Meter::spin` needs `fs.read`, `net.connect`"),
        "{stderr}"
    );

    let (stderr, ok) = package_calls_meter(&extension, r#"["fs.read", "net.connect"]"#);
    assert!(ok, "a line that lists both calls the extension:\n{stderr}");
}

// covers: tools:config/extension-grants-what-a-component-may-reach
#[test]
fn an_extension_that_requests_nothing_is_called_with_no_grant() {
    let extension = meter(&meter_manifest("{}"), &[]);
    let (stderr, ok) = package_calls_meter(&extension, "[]");
    assert!(
        ok,
        "an extension that requests nothing needs no grant:\n{stderr}"
    );
}

/// The source the test extension carries: `Shop\Meter\Probe::read` reads a file.
const PROBE: &str = "<?nvs\nnamespace Shop\\Meter;\n\nfinal class Probe\n{\n    \
                     public static function read(): string\n    {\n        \
                     return Core\\IO::readText('notes.txt');\n    }\n}\n";

/// An entry that calls the extension's own source.
const CALLS_PROBE: &str = "<?nvs\necho Shop\\Meter\\Probe::read();\n";

// covers: tools:config/extension-grants-what-a-component-may-reach
#[test]
fn a_third_party_extensions_source_holds_only_its_grant_line() {
    let extension = meter(&meter_manifest("{}"), &[("Meter/Probe.nvs", PROBE)]);
    let (stderr, ok) = check_with_extension(
        &extension,
        &[
            ("main.nvs", CALLS_PROBE),
            ("package.toml", "[grants]\n'Acme\\Http\\*' = []\n"),
        ],
    );
    assert!(
        !ok,
        "the extension's source reads a file with no line:\n{stderr}"
    );
    assert!(
        stderr.contains("error[E0658]")
            && stderr.contains("`Core\\IO::readText` needs `fs.read`")
            && stderr.contains("'Shop\\Meter' = [\"fs.read\"]"),
        "{stderr}"
    );

    let (stderr, ok) = check_with_extension(
        &extension,
        &[
            ("main.nvs", CALLS_PROBE),
            ("package.toml", "[grants]\n'Shop\\Meter' = [\"fs.read\"]\n"),
        ],
    );
    assert!(
        ok,
        "a line that names the source's namespace grants it:\n{stderr}"
    );
}

// covers: tools:config/extension-grants-what-a-component-may-reach
#[test]
fn a_third_party_extensions_source_is_held_where_no_grant_table_exists() {
    let extension = meter(&meter_manifest("{}"), &[("Meter/Probe.nvs", PROBE)]);
    let (stderr, ok) = check_with_extension(&extension, &[("main.nvs", CALLS_PROBE)]);
    assert!(
        !ok,
        "the extension's source reads a file where no grant table exists:\n{stderr}"
    );
    assert!(
        stderr.contains("error[E0658]") && stderr.contains("`Core\\IO::readText` needs `fs.read`"),
        "{stderr}"
    );

    let (stderr, ok) = check_with_extension(
        &meter(&meter_manifest("{}"), &[]),
        &[(
            "main.nvs",
            "<?nvs\nShop\\Meter::spin(1);\necho Core\\IO::readText('notes.txt');\n",
        )],
    );
    assert!(
        ok,
        "the application keeps its authority beside the extension:\n{stderr}"
    );
}

// covers: tools:config/extension-grants-what-a-component-may-reach
#[test]
fn a_built_in_components_source_keeps_its_authority() {
    let entry = "<?nvs\nuse Novis\\Spreadsheet\\Workbook;\n\n\
                 var $written = Workbook::fromRows([['a' => 1]])->save('out.csv');\n\
                 echo $written->size;\n";
    for grants in [None, Some("[grants]\n'Acme\\Http\\*' = []\n")] {
        let (_root, app) = match grants {
            Some(grants) => layout(&[("main.nvs", entry), ("package.toml", grants)]),
            None => layout(&[("main.nvs", entry)]),
        };
        let (stderr, ok) = check(&app.join("main.nvs"));
        assert!(
            ok,
            "a built-in component's source writes its file ({grants:?}):\n{stderr}"
        );
    }
}
