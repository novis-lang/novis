//! `nvs` as one executable: `--version`, a `--help` on every subcommand the
//! table lists, and no short flag of PHP's, as `docs/reference/tools/10-cli.md`
//! § *One binary* states them.
//!
//! Through the built binary, because what is asserted is its command line.

use std::process::{Command, Output};

/// Runs `nvs` with `args` and nothing else.
fn nvs(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(args)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// Every subcommand the chapter's table names.
const SUBCOMMANDS: &[&str] = &[
    "run", "check", "test", "build", "api", "init", "config", "info", "meta", "agent", "ast",
];

/// `--version` is one line, `nvs <version> (commit <hash>, <date>)`. Every
/// subcommand the table lists answers `--help` with status `0`. `-i`, `-a`,
/// `-r`, `-f` and `-v` are refused with a non-zero status, and so is
/// `convert`, which this build does not carry.
// covers: tools:cli/one-binary
#[test]
fn nvs_is_one_binary_with_a_subcommand_for_every_operation() {
    let version = nvs(&["--version"]);
    let stdout = String::from_utf8_lossy(&version.stdout);
    assert_eq!(version.status.code(), Some(0));
    let line = stdout.strip_suffix('\n').expect("one line");
    assert!(!line.contains('\n'), "one line: {stdout}");
    let rest = line
        .strip_prefix("nvs ")
        .expect("it starts with the binary's name");
    let (semver, build) = rest.split_once(" (commit ").expect("then the commit");
    assert_eq!(semver.split('.').count(), 3, "a three-part version: {line}");
    let (_hash, date) = build.split_once(", ").expect("then the date");
    assert!(
        date.ends_with(')') && date.len() == "2026-09-24)".len(),
        "{line}"
    );

    for sub in SUBCOMMANDS {
        let help = nvs(&[sub, "--help"]);
        let stderr = String::from_utf8_lossy(&help.stderr);
        assert_eq!(help.status.code(), Some(0), "`nvs {sub} --help`: {stderr}");
        assert!(
            !help.stdout.is_empty(),
            "`nvs {sub} --help` prints its usage"
        );
    }

    for flag in ["-i", "-a", "-r", "-f", "-v"] {
        let refused = nvs(&[flag]);
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert_ne!(
            refused.status.code(),
            Some(0),
            "`nvs {flag}` is not PHP's: {stderr}"
        );
        assert!(
            stderr.contains(&format!("unexpected argument '{flag}'")),
            "{stderr}"
        );
    }

    let convert = nvs(&["convert", "index.php"]);
    let stderr = String::from_utf8_lossy(&convert.stderr);
    assert_ne!(convert.status.code(), Some(0));
    assert!(
        stderr.contains("unrecognized subcommand 'convert'"),
        "{stderr}"
    );
}
