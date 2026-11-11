//! `nvs test <directory>`, driven through the built binary: a directory holding
//! `.nvs` files is one program that requires every one of them, so a test suite
//! runs without an entry that lists each file
//! (`rule:testing/a-directory-of-programs-is-one-test-program`).
//!
//! Two fixtures. `beside` is laid out the way the website teaches: an entry
//! under `public/` that declares the `autoload`, and classes and their tests
//! under `src/`, run over the project's root. `shop` keeps its tests in a
//! `tests/` directory outside the autoload root, whose `bootstrap.nvs`
//! requires the entry and whose test files require nothing.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// `nvs test <args...> <path>`, as `(stdout, stderr, success)`.
fn run_test(args: &[&str], path: &Path) -> (String, String, bool) {
    run("test", args, path)
}

/// `nvs <command> <args...> <path>`, as `(stdout, stderr, success)`.
fn run(command: &str, args: &[&str], path: &Path) -> (String, String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg(command)
        .args(args)
        .arg(path)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the report is UTF-8"),
        String::from_utf8(out.stderr).expect("the diagnostics are UTF-8"),
        out.status.success(),
    )
}

/// `tests/fixtures/test-dir/<name>`.
fn fixture(name: &str) -> PathBuf {
    [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "fixtures",
        "test-dir",
        name,
    ]
    .iter()
    .collect()
}

/// The `--format json` document `nvs test <args...> <fixture>` printed.
fn document(args: &[&str], name: &str) -> Value {
    let (out, err, ok) = run_test(args, &fixture(name));
    assert!(ok, "the run failed:\n{err}\n{out}");
    serde_json::from_str(&out)
        .unwrap_or_else(|error| panic!("`--format json` printed a document: {error}\n{out}"))
}

/// The `Class::method` of every record under `key`, sorted.
fn names(document: &Value, key: &str) -> Vec<String> {
    let mut names: Vec<String> = document[key]
        .as_array()
        .unwrap_or_else(|| panic!("`{key}` is an array: {document}"))
        .iter()
        .map(|test| {
            format!(
                "{}::{}",
                test["class"].as_str().expect("a class"),
                test["method"].as_str().expect("a method")
            )
        })
        .collect();
    names.sort();
    names
}

/// The whole directory is one program: the class under `src/` resolves through
/// the bootstrap one file in the directory requires, and a test in a
/// subdirectory runs beside one at the top.
#[test]
fn a_directory_of_test_files_runs_as_one_program() {
    let document = document(&["--format", "json"], "shop/tests");
    assert_eq!(
        names(&document, "tests"),
        ["CartTest::totalIsThree", "HelperTest::addition"]
    );
}

/// `--list` over a directory names each test's own file, not the entry the
/// runner wrote for the directory.
#[test]
fn listing_a_directory_names_each_tests_own_file() {
    let document = document(&["--list", "--format", "json"], "shop/tests");
    assert_eq!(
        names(&document, "listed"),
        ["CartTest::totalIsThree", "HelperTest::addition"]
    );
    let cart = document["listed"]
        .as_array()
        .expect("an array")
        .iter()
        .find(|test| test["class"] == "CartTest")
        .expect("the cart test is listed");
    let file = cart["file"].as_str().expect("a file");
    assert!(file.ends_with("CartTest.nvs"), "{file}");
}

/// A subdirectory is a program of its own: what needs no bootstrap runs, and
/// the tests above it are not swept in.
#[test]
fn a_subdirectory_is_run_on_its_own() {
    let document = document(&["--format", "json"], "shop/tests/unit");
    assert_eq!(names(&document, "tests"), ["HelperTest::addition"]);
}

/// The first layout the website teaches: tests under the autoload root, beside
/// the class or in a class of their own, run with `nvs test .` over the
/// project's root and no bootstrap. A test class nothing references runs too,
/// since the directory requires every file rather than only what the entry
/// reaches.
#[test]
fn a_project_root_runs_the_tests_beside_its_classes() {
    let document = document(&["--format", "json"], "beside");
    assert_eq!(
        names(&document, "tests"),
        [
            "Shop\\Cart::totalIsThree",
            "Shop\\Orders\\OrderTest::nothingReferencesThisClass"
        ]
    );
}

/// The second layout: a `tests/` folder whose bootstrap requires the entry,
/// run over the project's root beside a test under `src/`. The entry is
/// required once, through the bootstrap, so a request runs it once and its
/// body is not printed twice.
#[test]
fn a_project_root_requires_the_entry_its_bootstrap_requires_once() {
    let document = document(&["--format", "json"], "split");
    assert_eq!(
        names(&document, "tests"),
        [
            "HomeTest::theClassesResolveThroughTheBootstrap",
            "HomeTest::theEntryRunsOncePerRequest",
            "Shop\\Cart::totalIsThree"
        ]
    );
    let (out, err, ok) = run_test(&[], &fixture("split"));
    assert!(ok, "{out}{err}");
}

/// A test request runs the folder's `public/index.nvs` alone, as a served request does. The
/// script under `jobs/` throws at its top level, and it is still checked, but no request runs it.
#[test]
fn a_test_request_runs_the_served_file_and_not_the_jobs() {
    let document = document(&["--format", "json"], "jobs");
    assert_eq!(
        names(&document, "tests"),
        ["HomeTest::aRequestRunsTheServedFileAlone"]
    );
    let (out, err, ok) = run_test(&[], &fixture("jobs"));
    assert!(ok, "{out}{err}");
}

/// `nvs test` runs one program, so a second program path is refused with a
/// sentence about the count, not about the kind of file.
#[test]
fn two_programs_in_one_call_are_refused() {
    let beside = fixture("beside");
    let (_, err, ok) = run_test(&[beside.to_str().expect("a UTF-8 path")], &fixture("shop"));
    assert!(!ok);
    assert!(err.contains("runs one program at a time"), "{err}");
}

/// The two suites share no report, so a directory holding both kinds of file
/// is refused rather than run as whichever kind was found first.
#[test]
fn a_directory_mixing_programs_and_cases_is_refused() {
    let (_, err, ok) = run_test(&[], &fixture("mixed"));
    assert!(!ok);
    assert!(err.contains("run separately"), "{err}");
}

/// `packaged` holds `AppTest`, which passes, and two packages below it under
/// `vendor/acme/`. `http` is named and holds a failing test and a type error.
/// `tools` has a manifest with no name and a failing test. A walk from the
/// root goes into neither, so only `AppTest` runs.
#[test]
fn a_package_below_the_folder_is_not_walked() {
    let document = document(&["--format", "json"], "packaged");
    assert_eq!(names(&document, "tests"), ["AppTest::addition"]);
}

/// `nvs check` over the same root reads no file of either package, so the type
/// error in `vendor/acme/http/src/Broken.nvs` is not reported.
#[test]
fn a_folder_check_reads_no_file_of_a_package_below_it() {
    let (out, err, ok) = run("check", &[], &fixture("packaged"));
    assert!(ok, "{out}{err}");
    assert!(out.contains("no errors"), "{out}{err}");
}

/// A package's folder named on the command line is walked, so its own test
/// runs, and fails.
#[test]
fn a_package_folder_named_on_the_command_line_is_run() {
    let (out, err, ok) = run_test(
        &["--format", "json"],
        &fixture("packaged/vendor/acme/tools"),
    );
    assert!(!ok, "the package's failing test ran:\n{out}{err}");
    let document: Value = serde_json::from_str(&out)
        .unwrap_or_else(|error| panic!("`--format json` printed a document: {error}\n{out}"));
    assert_eq!(names(&document, "tests"), ["ToolTest::fails"]);
}

/// The `--format json` document of a run that may fail, and whether it passed.
fn report(args: &[&str], path: &Path) -> (Value, bool) {
    let (out, err, ok) = run_test(args, path);
    let document = serde_json::from_str(&out)
        .unwrap_or_else(|error| panic!("`--format json` printed a document: {error}\n{out}{err}"));
    (document, ok)
}

/// `loads-a-package/AppTest.nvs` requires `Client.nvs` of the package
/// `Acme\Http` and autoloads its `Cases` folder. The package's failing test and
/// its base class's own test do not run. `AppClientTest`, which extends the
/// package's `ClientCase`, runs the test it inherits under its own name.
#[test]
fn a_package_class_the_program_loads_runs_no_test() {
    let want = [
        "AppClientTest::inherited",
        "AppClientTest::own",
        "AppTest::usesTheClient",
    ];
    let entry = fixture("loads-a-package/AppTest.nvs");
    let (by_file, ok) = report(&["--format", "json"], &entry);
    assert!(ok, "a test of the package ran: {by_file}");
    assert_eq!(names(&by_file, "tests"), want);
    let (listed, _) = report(&["--list", "--format", "json"], &entry);
    assert_eq!(names(&listed, "listed"), want);
    let (by_folder, ok) = report(&["--format", "json"], &fixture("loads-a-package"));
    assert!(ok, "a test of the package ran: {by_folder}");
    assert_eq!(names(&by_folder, "tests"), want);
}

/// A file or the folder of the package named on the command line is the entry,
/// so the package's own tests run, and the failing one fails the run.
#[test]
fn a_package_named_on_the_command_line_runs_its_own_tests() {
    let (by_file, ok) = report(
        &["--format", "json"],
        &fixture("loads-a-package/vendor/acme/http/src/Client.nvs"),
    );
    assert!(!ok, "the package's failing test ran: {by_file}");
    assert_eq!(
        names(&by_file, "tests"),
        ["Acme\\Http\\Client::aPackageTestThatFails"]
    );
    let (by_folder, ok) = report(
        &["--format", "json"],
        &fixture("loads-a-package/vendor/acme/http"),
    );
    assert!(!ok, "the package's failing test ran: {by_folder}");
    assert_eq!(
        names(&by_folder, "tests"),
        [
            "Acme\\Http\\Cases\\ClientCase::inherited",
            "Acme\\Http\\Client::aPackageTestThatFails",
        ]
    );
}
