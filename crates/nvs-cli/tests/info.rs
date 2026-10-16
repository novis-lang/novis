//! `nvs info`, driven through the built binary: the unit tests in
//! `src/info.rs` assert on the report string, and these assert on what a user
//! typing the command gets — the exit status, the streams, and the spellings
//! `rule:packaging/nvs-info-is-the-one-call` says do not reach it.

use std::process::Command;

/// `nvs <args...>`, as `(stdout, stderr, exit code)`.
fn nvs(args: &[&str]) -> (String, String, Option<i32>) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(args)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the report is UTF-8"),
        String::from_utf8(out.stderr).expect("the diagnostics are UTF-8"),
        out.status.code(),
    )
}

/// The summary is the three sections and the component table, on standard
/// output and nothing on standard error; `--licenses` is the same report with
/// every text after it, so the summary is a prefix of it up to the table.
// covers: tools:cli/nvs-info
#[test]
fn info_prints_the_three_sections_and_licenses_appends_the_texts() {
    let (brief, err, code) = nvs(&["info"]);
    assert_eq!(code, Some(0), "`nvs info` succeeds: {err}");
    assert!(err.is_empty(), "the report goes to standard output: {err}");
    assert!(
        brief.starts_with("Novis "),
        "the title comes first: {brief}"
    );
    for section in ["\nBuild\n", "\nHost\n", "\nLicensing\n", "\nCOMPONENTS ("] {
        assert!(
            brief.contains(section),
            "`{}` is printed: {brief}",
            section.trim()
        );
    }
    assert!(
        brief.contains(&format!(
            "version             {}",
            env!("CARGO_PKG_VERSION")
        )),
        "the version row is this crate's version: {brief}"
    );
    assert!(
        !brief.contains("LICENSE TEXTS ("),
        "the summary prints no license text"
    );

    let (full, err, code) = nvs(&["info", "--licenses"]);
    assert_eq!(code, Some(0), "`nvs info --licenses` succeeds: {err}");
    assert!(
        full.contains("NOVIS'S OWN LICENSE") && full.contains("LICENSE TEXTS ("),
        "`--licenses` appends both texts"
    );
    // The `license texts` row is the one line the flag changes above the table.
    let strip = |report: &str| {
        let table = report
            .find("\nCOMPONENTS (")
            .expect("the report has a component table");
        report[..table]
            .lines()
            .filter(|line| !line.trim_start().starts_with("license texts"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        strip(&full),
        strip(&brief),
        "`--licenses` changes nothing above the table but its own row"
    );
}

/// `rule:packaging/nvs-info-is-the-one-call`: the command is reachable one
/// way. PHP's `-i`, a derived `--info` and a top-level `--licenses` are each
/// an argument error with status 2 and no report on standard output.
// covers: tools:cli/nvs-info
#[test]
fn info_is_reachable_only_as_the_subcommand() {
    for args in [
        &["-i"][..],
        &["--info"],
        &["--licenses"],
        &["-i", "--licenses"],
    ] {
        let (out, err, code) = nvs(args);
        assert_eq!(
            code,
            Some(2),
            "`nvs {}` is an argument error: {err}",
            args.join(" ")
        );
        assert!(
            out.is_empty(),
            "`nvs {}` printed a report: {out}",
            args.join(" ")
        );
        assert!(
            err.contains("unexpected argument"),
            "`nvs {}` names the argument it did not expect: {err}",
            args.join(" ")
        );
    }
}
