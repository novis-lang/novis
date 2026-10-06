//! `nvs check`: `no errors` and status `0` for a file that type-checks, every
//! diagnostic and status `1` for one that does not, and nothing run either
//! way, as `docs/reference/tools/10-cli.md` § *nvs check* states it.
//!
//! Through the built binary, because what is asserted is what an editor, a
//! pre-commit hook or CI reads off the command: its streams and its status.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A directory of the case's own under the target's scratch, emptied first,
/// so two cases running in parallel never share a fixture.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("check-command")
        .join(case);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
    dir
}

/// Runs `nvs check` with `args` from `dir`.
fn check_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("check")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

const ORDER: &str = "<?nvs
class Order {
    public int $total = 0;
    public function add(int $n): void {
        $this->total = $this->total + $n;
    }
}
echo \"ran\\n\";
Order $o = new Order();
";

/// A file that type-checks prints `no errors` and nothing of its own; one
/// with two mistakes reports both, in source order, and the count, on
/// standard error.
// covers: tools:cli/nvs-check
#[test]
fn nvs_check_reports_every_diagnostic_and_runs_nothing() {
    let dir = scratch("good-and-bad");
    fs::write(dir.join("good.nvs"), format!("{ORDER}$o->add(7);\n")).unwrap();
    fs::write(
        dir.join("bad.nvs"),
        format!("{ORDER}$o->add(\"seven\");\necho $o->totl, \"\\n\";\n"),
    )
    .unwrap();

    let good = check_in(&dir, &["good.nvs"]);
    let stderr = String::from_utf8_lossy(&good.stderr);
    assert_eq!(good.status.code(), Some(0), "{stderr}");
    assert_eq!(
        String::from_utf8_lossy(&good.stdout),
        "no errors\n",
        "the program's own `echo` never ran"
    );

    let bad = check_in(&dir, &["bad.nvs"]);
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert_eq!(bad.status.code(), Some(1), "{stderr}");
    assert!(
        bad.stdout.is_empty(),
        "neither `no errors` nor the program's output"
    );
    let first = stderr
        .find("error[E0401]")
        .expect("the argument of the wrong type");
    let second = stderr
        .find("error[E0405]")
        .expect("the property that does not exist");
    assert!(first < second, "diagnostics come in source order: {stderr}");
    assert!(
        stderr.contains("bad.nvs:10:9"),
        "the location is file:line:column: {stderr}"
    );
    assert!(stderr.contains("aborting due to 2 errors"), "{stderr}");
}

/// A file that is not UTF-8 is `E0006` at its first byte that is not, from
/// each command that reads a source file, and a file that is not there is
/// still the uncoded `could not read`.
#[test]
fn a_source_file_that_is_not_utf_8_is_e0006_at_its_first_bad_byte() {
    let dir = scratch("not-utf-8");
    // `0xE9` is `é` in Latin-1, and on its own it is not UTF-8.
    let mut bytes = b"<?nvs\necho 1;\n// caf".to_vec();
    bytes.extend_from_slice(&[0xE9, b'\n']);
    fs::write(dir.join("latin1.nvs"), bytes).unwrap();

    for command in [&["check"][..], &["ast"], &["fmt", "--check"]] {
        let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .args(command)
            .arg("latin1.nvs")
            .current_dir(&dir)
            .output()
            .expect("the `nvs` binary this test was built beside runs");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{command:?}: {stderr}");
        assert!(stderr.contains("error[E0006]"), "{command:?}: {stderr}");
        assert!(
            stderr.contains("latin1.nvs:3:7"),
            "{command:?} points at the byte: {stderr}"
        );
    }

    let missing = check_in(&dir, &["missing.nvs"]);
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert_eq!(missing.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("could not read"), "{stderr}");
    assert!(!stderr.contains("E0006"), "{stderr}");
}

/// A tree whose `nvs.toml` pins one extension: the `ledger` fixture's manifest and source over
/// the component `wat`, which exports none of the manifest's methods, plus a `good.nvs` that
/// calls it rightly and a `bad.nvs` that passes a string where it takes an `int`.
fn extension_tree(case: &str, wat: &str) -> PathBuf {
    let dir = scratch(case);
    let fixture = nvs_repo::path("tests/conformance/ext/fixtures/ledger");
    let manifest = fs::read(fixture.join("manifest.json")).expect("the fixture's manifest");
    let receipt = fs::read_to_string(fixture.join("source/Ledger/Receipt.nvs.src"))
        .expect("the fixture's source file");
    let source = format!(
        "{{\"source\": 1, \"files\": [{{\"path\": \"Ledger/Receipt.nvs\", \"text\": {receipt:?}}}]}}"
    );
    let component = wat::parse_str(wat).expect("the component's text parses");
    let nvsx = nvs_ext::pack::append_section(component, nvs_ext::section::MANIFEST, &manifest);
    let nvsx = nvs_ext::pack::append_section(nvsx, nvs_ext::section::SOURCE, source.as_bytes());
    fs::write(dir.join("shop.nvsx"), &nvsx).unwrap();
    fs::write(
        dir.join("nvs.toml"),
        format!(
            "[[extension]]\npath = 'shop.nvsx'\nsha256 = \"{}\"\n",
            nvs_ext::load::pin(&nvsx)
        ),
    )
    .unwrap();
    fs::write(
        dir.join("good.nvs"),
        "<?nvs\necho Shop\\Ledger::echoInt(7), \"\\n\";\necho Shop\\Ledger\\Receipt::line(\"Tea\", 250), \"\\n\";\n",
    )
    .unwrap();
    fs::write(
        dir.join("bad.nvs"),
        "<?nvs\necho Shop\\Ledger::echoInt(\"seven\"), \"\\n\";\n",
    )
    .unwrap();
    dir
}

/// A call into an extension of the tree's `[[extension]]` set is typed from its manifest, and a
/// class from the extension's source section resolves with no `autoload` line
/// (`rule:packaging/extension-calls-are-statically-typed`).
#[test]
fn nvs_check_types_an_extension_call_from_the_configured_set() {
    let dir = extension_tree("extension-set", "(component)");
    let good = check_in(&dir, &["good.nvs"]);
    let stderr = String::from_utf8_lossy(&good.stderr);
    assert_eq!(good.status.code(), Some(0), "{stderr}");
    assert_eq!(String::from_utf8_lossy(&good.stdout), "no errors\n");
}

/// A wrong argument to an extension call is the `E0401` a `Core` call gives, at the call's line.
#[test]
fn nvs_check_refuses_a_wrong_argument_to_an_extension_call() {
    let dir = extension_tree("extension-wrong-argument", "(component)");
    let bad = check_in(&dir, &["bad.nvs"]);
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert_eq!(bad.status.code(), Some(1), "{stderr}");
    assert!(bad.stdout.is_empty(), "{stderr}");
    assert!(stderr.contains("error[E0401]"), "{stderr}");
    assert!(stderr.contains("bad.nvs:2:"), "{stderr}");
}

/// The component imports a function no host provides, so instantiating it fails: `nvs run`
/// cannot, and `nvs check` still types the program, because it reads only the manifest.
#[test]
fn nvs_check_never_instantiates_an_extension() {
    let dir = extension_tree(
        "extension-never-instantiated",
        "(component (import \"missing\" (func)))",
    );
    let good = check_in(&dir, &["good.nvs"]);
    let stderr = String::from_utf8_lossy(&good.stderr);
    assert_eq!(good.status.code(), Some(0), "{stderr}");
    assert_eq!(String::from_utf8_lossy(&good.stdout), "no errors\n");

    let run = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["run", "good.nvs"])
        .current_dir(&dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    assert_ne!(
        run.status.code(),
        Some(0),
        "running the program does instantiate the component, and fails: {}",
        String::from_utf8_lossy(&run.stderr)
    );
}

/// The numbers on a `compile:` line, by name, in the order printed.
fn compile_counts(stderr: &str) -> Vec<(String, u64)> {
    let line = stderr
        .lines()
        .find_map(|line| line.strip_prefix("compile: "))
        .unwrap_or_else(|| panic!("a `compile:` line on stderr: {stderr}"));
    line.split(' ')
        .map(|pair| {
            let (name, value) = pair.split_once('=').expect("`name=value`");
            (name.to_owned(), value.parse().expect("a whole number"))
        })
        .collect()
}

/// `nvs check --count` prints one `compile:` line on stderr, stdout stays
/// `no errors`, and a program with more in it has more of every count.
/// `nvs run --count` prints the same line with the IR instructions added.
#[test]
fn check_count_prints_what_each_phase_produced() {
    let dir = scratch("count");
    fs::write(dir.join("small.nvs"), format!("{ORDER}$o->add(7);\n")).unwrap();
    let more = "$o->add(1);\necho $o->total + 2, \"\\n\";\n".repeat(20);
    fs::write(
        dir.join("large.nvs"),
        format!("{ORDER}{more}class Extra {{}}\n"),
    )
    .unwrap();

    let small = check_in(&dir, &["--count", "small.nvs"]);
    let stderr = String::from_utf8_lossy(&small.stderr);
    assert_eq!(small.status.code(), Some(0), "{stderr}");
    assert_eq!(String::from_utf8_lossy(&small.stdout), "no errors\n");
    let small = compile_counts(&stderr);
    let names: Vec<&str> = small.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["tokens", "nodes", "names", "exprs"], "{stderr}");

    let large = check_in(&dir, &["--count", "large.nvs"]);
    let stderr = String::from_utf8_lossy(&large.stderr);
    assert_eq!(large.status.code(), Some(0), "{stderr}");
    let large = compile_counts(&stderr);
    for ((name, before), (_, after)) in small.iter().zip(&large) {
        assert!(after > before, "{name}: {before} then {after}");
    }

    let without = check_in(&dir, &["small.nvs"]);
    assert!(
        !String::from_utf8_lossy(&without.stderr).contains("compile:"),
        "nothing is printed without the flag"
    );

    let run = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["run", "--count", "small.nvs"])
        .current_dir(&dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(run.status.code(), Some(0), "{stderr}");
    let counts = compile_counts(&stderr);
    assert_eq!(
        counts[..4],
        small[..],
        "the same front end, the same counts"
    );
    assert_eq!(counts[4].0, "ir", "{stderr}");
    assert!(counts[4].1 > 0, "{stderr}");
    assert!(stderr.contains("count: statements="), "{stderr}");
}
