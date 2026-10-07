//! `nvs ext`, driven through the built binary: the six subcommands
//! `rule:packaging/nvs-ext-is-the-authoring-tool` names, what each one answers, and that none of
//! them writes an `nvs.toml` into the directory it runs in.

use std::path::Path;
use std::process::Command;

/// The subcommands, in the rule's order.
const SUBCOMMANDS: [&str; 6] = ["new", "build", "inspect", "test", "verify", "pin"];

/// `nvs <args...>` run in `dir`, as `(stdout, stderr, exit code)`.
fn nvs_in(dir: &Path, args: &[&str]) -> (String, String, Option<i32>) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the output is UTF-8"),
        String::from_utf8(out.stderr).expect("the diagnostics are UTF-8"),
        out.status.code(),
    )
}

/// `nvs ext --help` lists every subcommand, and `nvs --help` lists `ext`.
#[test]
fn ext_help_lists_the_six_subcommands() {
    let dir = nvs_repo::scratch("ext-command-help");
    let (out, err, code) = nvs_in(&dir, &["ext", "--help"]);
    assert_eq!(code, Some(0), "`nvs ext --help` succeeds: {err}");
    for subcommand in SUBCOMMANDS {
        assert!(
            out.lines()
                .any(|line| line.trim_start().starts_with(&format!("{subcommand} "))),
            "`{subcommand}` is listed: {out}"
        );
    }
    let (out, err, code) = nvs_in(&dir, &["--help"]);
    assert_eq!(code, Some(0), "`nvs --help` succeeds: {err}");
    assert!(
        out.lines()
            .any(|line| line.trim_start().starts_with("ext ")),
        "`ext` is listed: {out}"
    );
}

/// A subcommand this binary does not implement says so and fails, and writes nothing into the
/// directory it runs in — no `nvs.toml` above all.
#[test]
fn an_unbuilt_subcommand_says_so_and_exits_non_zero() {
    let dir = nvs_repo::scratch("ext-command-unbuilt");
    let calls: [&[&str]; 6] = [
        &["ext", "new", "--lang", "rust", "project"],
        &["ext", "build"],
        &["ext", "inspect", "geo.nvsx"],
        &["ext", "test"],
        &["ext", "verify", "geo.nvsx"],
        &["ext", "pin", "geo.nvsx"],
    ];
    for (subcommand, args) in SUBCOMMANDS.into_iter().zip(calls) {
        let (out, err, code) = nvs_in(&dir, args);
        assert_eq!(code, Some(1), "`nvs ext {subcommand}` fails: {err}");
        assert!(out.is_empty(), "nothing goes to standard output: {out}");
        assert!(
            err.contains(&format!("`nvs ext {subcommand}` is not available")),
            "the error names the subcommand: {err}"
        );
    }
    let left: Vec<_> = std::fs::read_dir(&*dir)
        .expect("the scratch directory is readable")
        .map(|entry| entry.expect("an entry is readable").file_name())
        .collect();
    assert!(left.is_empty(), "`nvs ext` wrote nothing here: {left:?}");
}

/// `--lang` takes `rust` and `c` and nothing else, and is required.
#[test]
fn ext_new_takes_rust_or_c() {
    let dir = nvs_repo::scratch("ext-command-lang");
    let (_, err, code) = nvs_in(&dir, &["ext", "new", "--lang", "zig", "project"]);
    assert_eq!(code, Some(2), "an unknown language is a usage error: {err}");
    assert!(
        err.contains("rust") && err.contains('c'),
        "the error lists the two languages: {err}"
    );
    let (_, err, code) = nvs_in(&dir, &["ext", "new", "project"]);
    assert_eq!(code, Some(2), "`--lang` is required: {err}");
}
