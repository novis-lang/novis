//! The fixed cost of one start, counted by phase: what a debug `nvs check --count` prints on its
//! `phases:` line for a one-line program, bounded per phase (`rule:testing/perf-two-mechanisms`).
//!
//! The bounds are counts, so they hold on any machine however loaded. Each is the figure
//! `bun nv start-cost` measured, plus 10%, rounded up to the next hundred. `grants` is the
//! exception: its count follows the length of the directory's path, 25 from `.agent-tmp/` and 33
//! from this test's scratch directory, so its bound is 60. A change that adds work to a start phase
//! past its bound fails here, and a change that removes work lowers the bound in the same commit.

use std::process::Command;

/// Each phase's allocation bound, in the order the line prints them. The measured figures on
/// Windows are 418, 5,726, 25, 8,109, 1,166 and 9,652.
const ALLOCATIONS: [(&str, u64); 6] = [
    ("configuration", 500),
    ("built_in", 6_300),
    ("grants", 60),
    ("core", 9_000),
    ("collection", 1_300),
    ("check", 10_700),
];

/// The figures on a `phases:` line, by name, such as `core.allocations`.
fn phase_counts(stderr: &str) -> Vec<(String, u64)> {
    let line = stderr
        .lines()
        .find_map(|line| line.strip_prefix("phases: "))
        .unwrap_or_else(|| panic!("a debug `--count` prints a `phases:` line: {stderr}"));
    line.split(' ')
        .map(|pair| {
            let (name, value) = pair.split_once('=').expect("`name=value`");
            (name.to_owned(), value.parse().expect("a whole number"))
        })
        .collect()
}

/// A one-line program's start stays within each phase's counted bound: its allocations, one
/// configuration file read, and no source file read by any phase after the entry.
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "a release `nvs` prints no `phases:` line"
)]
fn a_small_program_starts_within_its_counted_work() {
    let dir = nvs_repo::scratch_private("start-cost");
    std::fs::write(dir.join("small.nvs"), "<?nvs\necho \"hi\\n\";\n").expect("a scratch file");
    let data = nvs_repo::scratch_private("nvsdata");
    let output = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .args(["check", "--count", "small.nvs"])
        .current_dir(&*dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    let counts = phase_counts(&stderr);
    let count = |name: &str| {
        counts
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, value)| *value)
            .unwrap_or_else(|| panic!("the `phases:` line has `{name}`: {stderr}"))
    };

    for (phase, bound) in ALLOCATIONS {
        let allocations = count(&format!("{phase}.allocations"));
        assert!(
            allocations <= bound,
            "`{phase}` made {allocations} allocations, over its bound of {bound}: {stderr}"
        );
    }
    assert!(count("configuration.settings") <= 1, "{stderr}");
    for (phase, _) in ALLOCATIONS {
        assert_eq!(
            count(&format!("{phase}.read")),
            0,
            "`{phase}` read a source file: {stderr}"
        );
        if phase != "configuration" {
            assert_eq!(count(&format!("{phase}.settings")), 0, "{stderr}");
            assert_eq!(count(&format!("{phase}.listed")), 0, "{stderr}");
        }
    }
    assert!(count("collection.resolved") <= 1, "{stderr}");
}
