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
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("check-command").join(case);
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
    assert!(bad.stdout.is_empty(), "neither `no errors` nor the program's output");
    let first = stderr.find("error[E0401]").expect("the argument of the wrong type");
    let second = stderr.find("error[E0405]").expect("the property that does not exist");
    assert!(first < second, "diagnostics come in source order: {stderr}");
    assert!(stderr.contains("bad.nvs:10:9"), "the location is file:line:column: {stderr}");
    assert!(stderr.contains("aborting due to 2 errors"), "{stderr}");
}
