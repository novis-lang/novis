//! `nvs config check` and `nvs config dump`, driven through the built binary over
//! trees on disk: the example's two files, every refusal the chapter names, a
//! directory with no `nvs.toml` at all, and the hostile tree whose values are
//! written to forge a row of the listing.
//!
//! The rows themselves are `nvs_config::audit`'s, unit-tested there; these assert
//! what a user typing the command gets — the summary line, the exit status, and
//! which stream each part lands on.

use std::path::{Path, PathBuf};
use std::process::Command;

/// `nvs <args...>` in `dir`, as `(stdout, stderr, exit code)`.
fn nvs_in(dir: &Path, args: &[&str]) -> (String, String, Option<i32>) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the audit is UTF-8"),
        String::from_utf8(out.stderr).expect("the diagnostics are UTF-8"),
        out.status.code(),
    )
}

/// A tree the repository ships, by its repository-rooted path.
fn shipped(path: &str) -> PathBuf {
    nvs_repo::path(path)
}

/// An empty directory of this case's own under `CARGO_TARGET_TMPDIR`, named for
/// the case so two cases running at once never share one.
fn scratch(case: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("config-audit-{case}-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).expect("the case's directory is created");
    dir
}

/// The example's tree: `nvs.toml` includes `production.toml`, which sets
/// `limits.memory` again. `check` counts both files and the one override, and
/// `dump` lists the four keys in force with the later file's value; `--origin`
/// names that file and the one it overrode, and `--toml` is the same table as
/// one document.
// covers: tools:cli/nvs-config-check-and-nvs-config-dump
#[test]
fn check_counts_the_example_tree_and_dump_names_the_file_each_key_came_from() {
    let dir = shipped("docs/examples/tools/cli/nvs-config-check-and-nvs-config-dump");

    let (out, err, code) = nvs_in(&dir, &["config", "check", "nvs.toml"]);
    assert_eq!(code, Some(0), "the example tree is valid: {err}");
    assert_eq!(
        out,
        "ok: 2 files, 4 directives set, 1 override, 0 warnings\n"
    );
    assert!(err.is_empty(), "a clean tree prints nothing else: {err}");

    let (out, err, code) = nvs_in(&dir, &["config", "dump", "nvs.toml"]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(
        out,
        concat!(
            "include.0.path     = \"production.toml\"\n",
            "limits.hard.memory = \"512M\"\n",
            "limits.memory      = \"128M\"\n",
            "limits.wall_time   = \"30s\"\n",
        )
    );

    let (out, err, code) = nvs_in(&dir, &["config", "dump", "--origin", "nvs.toml"]);
    assert_eq!(code, Some(0), "{err}");
    let memory = out
        .lines()
        .find(|line| line.starts_with("limits.memory "))
        .unwrap_or_else(|| panic!("`limits.memory` has a row: {out}"));
    assert!(
        memory.contains("production.toml (overrides ") && memory.ends_with("nvs.toml)"),
        "the row names the file that set it and the file it overrode: {memory}"
    );

    let (out, err, code) = nvs_in(&dir, &["config", "dump", "--toml", "nvs.toml"]);
    assert_eq!(code, Some(0), "{err}");
    let table: toml::Table = out.parse().expect("`--toml` prints one TOML document");
    assert_eq!(
        table["limits"]["memory"].as_str(),
        Some("128M"),
        "the document is the resolved tree: {out}"
    );
    assert!(
        !out.contains("production.toml (overrides"),
        "and carries no origin: {out}"
    );
}

/// The configuration chapter's audit, over a tree with one `[[app]]` block: `dump`
/// numbers the block's keys `app.0.…` in dotted-key order, and the `--toml`
/// document, written out and dumped again, lists exactly the same rows, so two
/// environments compare by their documents alone.
// covers: tools:config/auditing-a-tree
#[test]
fn dump_numbers_each_app_block_and_the_toml_document_dumps_the_same_rows() {
    let dir = shipped("docs/examples/tools/config/auditing-a-tree");

    let (out, err, code) = nvs_in(&dir, &["config", "check", "nvs.toml"]);
    assert_eq!(code, Some(0), "the example tree is valid: {err}");
    assert_eq!(
        out,
        "ok: 1 file, 4 directives set, 0 overrides, 0 warnings\n"
    );

    let (dump, err, code) = nvs_in(&dir, &["config", "dump", "nvs.toml"]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(
        dump,
        concat!(
            "app.0.limits.wall_time = \"120s\"\n",
            "app.0.root             = \".\"\n",
            "limits.memory          = \"256M\"\n",
            "limits.wall_time       = \"30s\"\n",
        )
    );

    let (document, err, code) = nvs_in(&dir, &["config", "dump", "--toml", "nvs.toml"]);
    assert_eq!(code, Some(0), "{err}");
    let again = scratch("auditing-a-tree");
    std::fs::write(again.join("nvs.toml"), &document).expect("the document is written");
    let (redump, err, code) = nvs_in(&again, &["config", "dump", "nvs.toml"]);
    assert_eq!(
        code,
        Some(0),
        "the `--toml` document is itself a valid tree: {err}"
    );
    assert_eq!(
        redump, dump,
        "the document is the tree it was printed from: {document}"
    );
    drop(std::fs::remove_dir_all(&again));
}

/// Every refusal the chapter lists exits `1` with its code on standard error and
/// prints no summary line; the unchecked quantity it names passes.
// covers: tools:cli/nvs-config-check-and-nvs-config-dump
#[test]
fn every_refusal_the_chapter_names_exits_1_with_its_code() {
    let dir = scratch("refusals");
    let cases: [(&str, &str, &str); 7] = [
        ("syntax.toml", "[limits\n", "E0601"),
        ("unknown.toml", "[limits]\nmemroy = \"1M\"\n", "E0601"),
        (
            "missing.toml",
            "[[include]]\npath = \"absent.toml\"\n",
            "E0605",
        ),
        (
            "cycle.toml",
            "[[include]]\npath = \"cycle.toml\"\n",
            "E0606",
        ),
        (
            "both.toml",
            "[[app]]\nroot = \".\"\nentry = \"a.nvs\"\n",
            "E0609",
        ),
        (
            "neither.toml",
            "[[app]]\norigin = \"https://example.test\"\n",
            "E0609",
        ),
        (
            "secret.toml",
            "[db.main]\ndriver = \"postgres\"\npassword = \"x\"\npassword_file = \"p.txt\"\n",
            "E0608",
        ),
    ];
    for (file, text, want) in cases {
        std::fs::write(dir.join(file), text).expect("the case's directory takes a file");
        for command in ["check", "dump"] {
            let (out, err, code) = nvs_in(&dir, &["config", command, file]);
            assert_eq!(
                code,
                Some(1),
                "`config {command} {file}` is refused: {out}{err}"
            );
            assert!(
                err.contains(&format!("error[{want}]")),
                "`config {command} {file}` names {want}: {err}"
            );
            assert!(
                out.is_empty(),
                "a refused tree prints nothing on standard output: {out}"
            );
        }
    }

    std::fs::write(
        dir.join("bananas.toml"),
        "[limits]\nmemory = \"12 bananas\"\n",
    )
    .expect("the case's directory takes a file");
    let (out, err, code) = nvs_in(&dir, &["config", "check", "bananas.toml"]);
    assert_eq!(
        (code, out.as_str()),
        (
            Some(0),
            "ok: 1 file, 1 directive set, 0 overrides, 0 warnings\n"
        ),
        "a quantity is not checked by `config check`: {err}"
    );
    drop(std::fs::remove_dir_all(&dir));
}

/// With no file named and no `./nvs.toml`, both commands read the shipped
/// defaults: a tree of no files, which is a success and not a missing file.
// covers: tools:cli/nvs-config-check-and-nvs-config-dump
#[test]
fn a_directory_with_no_nvs_toml_is_a_tree_of_no_files() {
    let dir = scratch("no-file");
    let (out, err, code) = nvs_in(&dir, &["config", "check"]);
    assert_eq!(code, Some(0), "no file is not an error: {err}");
    assert_eq!(
        out,
        "ok: 0 files, 0 directives set, 0 overrides, 0 warnings\n"
    );

    let (out, err, code) = nvs_in(&dir, &["config", "dump"]);
    assert_eq!((code, out.as_str()), (Some(0), ""), "{err}");
    drop(std::fs::remove_dir_all(&dir));
}

/// The attack's tree: values and a database name holding a newline followed by
/// text shaped like `limits.hard.memory = "99G"`, a right-to-left override and a
/// NUL, two files that include one shared file, and a directory include. The
/// listing is one line per directive `check` counted, no line reads as a setting
/// the tree never set, and `--toml` gives every value back exactly as written.
// covers: tools:cli/nvs-config-check-and-nvs-config-dump
#[test]
fn the_hostile_tree_lists_one_row_per_directive_and_forges_none() {
    let dir = shipped("tests/hostile/tools/cli/nvs-config-check-and-nvs-config-dump");

    let (out, err, code) = nvs_in(&dir, &["config", "check", "nvs.toml"]);
    assert_eq!(code, Some(0), "the hostile tree is valid: {err}");
    assert_eq!(
        out,
        "ok: 7 files, 11 directives set, 6 overrides, 0 warnings\n"
    );

    for flags in [
        &["config", "dump", "nvs.toml"][..],
        &["config", "dump", "--origin", "nvs.toml"],
    ] {
        let (out, err, code) = nvs_in(&dir, flags);
        assert_eq!(code, Some(0), "{err}");
        assert_eq!(out.lines().count(), 11, "one line per directive: {out}");
        for line in out.lines() {
            assert!(
                ["app.0.", "db.\"main\\n", "include.", "limits.memory "]
                    .iter()
                    .any(|start| line.starts_with(start)),
                "every row is a key the tree wrote: {line}"
            );
            assert!(
                !line.contains(['\r', '\u{0}', '\u{202e}', '\u{2066}']),
                "no row carries a character a terminal acts on: {line:?}"
            );
        }
        let memory = out
            .lines()
            .find(|line| line.starts_with("limits.memory "))
            .unwrap_or_else(|| panic!("`limits.memory` has a row: {out}"));
        assert!(
            memory.contains("\"16M\""),
            "`conf.d/9-earlier.toml` is read last, because names sort as text: {memory}"
        );
    }

    let (out, err, code) = nvs_in(&dir, &["config", "dump", "--toml", "nvs.toml"]);
    assert_eq!(code, Some(0), "{err}");
    let table: toml::Table = out.parse().expect("`--toml` prints one TOML document");
    assert_eq!(
        table["app"][0]["origin"].as_str(),
        Some(
            "https://example.test\nlimits.hard.memory = \"99G\"\u{202e}\t\\ \"q\" \u{0} caf\u{e9}"
        ),
        "the value comes back exactly as written"
    );
}
