//! `nvs fmt`, driven through the built binary, because what
//! `rule:tooling/fmt-check-writes-nothing` promises is what the *command*
//! does to the disk: an in-place rewrite, a list, a diff, or a buffer on
//! standard output. The layout itself is `nvs-fmt`'s own suite and is never
//! re-asserted here.
//!
//! Every case works in a private directory under the temporary root, because
//! the default mode rewrites what it is given and a fixture checked into the
//! tree would be a fixture one run edits.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// A file wrong about its indentation and about nothing else, and the layout
/// `nvs fmt` writes for it. The pair `nvs-fmt`'s own
/// `indentation_is_four_spaces_per_block_depth` holds, so a failure here is
/// this command's and not the formatter's.
const MANGLED: &str = "\
<?nvs
class Tag
{
      public string $name;

  public function rename(string $name): void
    {
$this->name = $name;
    }
}
";

/// What [`MANGLED`] formats to.
const CANONICAL: &str = "\
<?nvs
class Tag
{
    public string $name;

    public function rename(string $name): void
    {
        $this->name = $name;
    }
}
";

/// A file the parser reports an error in: a refusal, never a rewrite.
const BROKEN: &str = "\
<?nvs
var $tag = ;
";

/// Valid, type-checks, and laid out the way nothing in this project lays
/// anything out.
const UNFORMATTED: &str = "\
<?nvs
     var $greeting   = 'hello';
  echo $greeting;
";

/// A private directory holding `files`, as `(directory, paths)`.
///
/// One directory per test rather than one shared: these run concurrently under
/// `cargo test`, and a file one of them is rewriting is not something another
/// should ever be able to observe.
fn fixture(name: &str, files: &[(&str, &str)]) -> (nvs_repo::Scratch, Vec<PathBuf>) {
    let dir = nvs_repo::scratch(&format!("fmt-cmd-{name}"));
    let paths = files
        .iter()
        .map(|(file, source)| {
            let path = dir.join(file);
            std::fs::write(&path, source).expect("the fixture is written");
            path
        })
        .collect();
    (dir, paths)
}

/// `nvs fmt <args...>`, as `(stdout, stderr, exit code)`.
fn fmt<S: AsRef<std::ffi::OsStr>>(args: &[S]) -> (String, String, Option<i32>) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("fmt")
        .args(args)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the text is UTF-8"),
        String::from_utf8(out.stderr).expect("the diagnostics are UTF-8"),
        out.status.code(),
    )
}

/// `nvs fmt --stdin <args...>` over `source`, as `(stdout, stderr, exit code)`.
fn stdin(source: &str, args: &[&str]) -> (String, String, Option<i32>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["fmt", "--stdin"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the `nvs` binary this test was built beside runs");
    {
        use std::io::Write as _;
        let mut pipe = child.stdin.take().expect("standard input is a pipe");
        pipe.write_all(source.as_bytes())
            .expect("the buffer reaches the command");
    }
    let out = child.wait_with_output().expect("the command exits");
    (
        String::from_utf8(out.stdout).expect("the text is UTF-8"),
        String::from_utf8(out.stderr).expect("the diagnostics are UTF-8"),
        out.status.code(),
    )
}

/// What `path` holds now.
fn text(path: &Path) -> String {
    std::fs::read_to_string(path).expect("the fixture is still on disk")
}

/// The default mode: the named files, rewritten where they sit, and a file the
/// parse reported an error in left exactly as its author last saved it.
// covers: tools:cli/nvs-fmt
#[test]
fn fmt_rewrites_each_named_file_in_place() {
    let (_dir, paths) = fixture(
        "in-place",
        &[
            ("one.nvs", MANGLED),
            ("two.nvs", MANGLED),
            ("broken.nvs", BROKEN),
        ],
    );

    let (out, err, code) = fmt(&paths[..2]);
    assert_eq!(code, Some(0), "two files that parse are formatted: {err}");
    assert_eq!(out, "", "the default mode prints nothing on success");
    assert_eq!(text(&paths[0]), CANONICAL, "the first file was rewritten");
    assert_eq!(text(&paths[1]), CANONICAL, "the second file was rewritten");

    // The same call again: the formatter is a fixed point, so the second run
    // has nothing to write (`rule:tooling/fmt-is-idempotent`).
    let (_, err, code) = fmt(&paths[..2]);
    assert_eq!(
        code,
        Some(0),
        "a formatted file is formatted again without complaint: {err}"
    );
    assert_eq!(text(&paths[0]), CANONICAL, "the second run changed nothing");

    let (_, err, code) = fmt(&paths[2..]);
    assert_eq!(code, Some(2), "a file that does not parse exits 2");
    assert!(
        err.contains("broken.nvs"),
        "the refusal names the file: {err}"
    );
    assert_eq!(
        text(&paths[2]),
        BROKEN,
        "a refused file is left exactly as it was"
    );
}

/// `--check`: the file list on standard output, exit `1`, and a disk nothing
/// touched.
// covers: tools:cli/nvs-fmt
#[test]
fn fmt_check_writes_nothing_and_exits_non_zero_naming_each_file() {
    let (_dir, paths) = fixture("check", &[("ragged.nvs", MANGLED), ("tidy.nvs", CANONICAL)]);

    let (out, _, code) = fmt(&[
        std::ffi::OsStr::new("--check"),
        paths[0].as_os_str(),
        paths[1].as_os_str(),
    ]);
    assert_eq!(code, Some(1), "a file that would change exits 1");
    assert!(
        out.contains("ragged.nvs"),
        "the file that would change is named: {out}"
    );
    assert!(
        !out.contains("tidy.nvs"),
        "the file that would not change is not: {out}"
    );
    assert_eq!(
        text(&paths[0]),
        MANGLED,
        "`--check` wrote nothing to the file it named"
    );

    let (out, _, code) = fmt(&[std::ffi::OsStr::new("--check"), paths[1].as_os_str()]);
    assert_eq!(code, Some(0), "an already-formatted file exits zero");
    assert_eq!(out, "", "and is named nowhere");
}

/// `rule:tooling/fmt-exit-status-says-what-it-found`: `2` for a refused file
/// outranks `1` for one that would change, in whichever order the walk meets
/// them, and the file after the refusal is still named.
// covers: tools:cli/nvs-fmt
#[test]
fn fmt_check_exits_two_when_a_file_is_refused_beside_one_that_would_change() {
    let (dir, paths) = fixture(
        "exit-two",
        &[("a-broken.nvs", BROKEN), ("b-ragged.nvs", MANGLED)],
    );

    let (out, err, code) = fmt(&[std::ffi::OsStr::new("--check"), dir.as_os_str()]);
    assert_eq!(code, Some(2), "a refusal outranks a change: {err}");
    assert!(
        out.contains("b-ragged.nvs"),
        "the file after the refusal is still named: {out}"
    );
    assert!(
        err.contains("a-broken.nvs"),
        "the refusal names the file: {err}"
    );

    let (_, err, code) = fmt(&[std::ffi::OsStr::new("--check"), paths[1].as_os_str()]);
    assert_eq!(code, Some(1), "the change alone exits 1: {err}");
}

/// `rule:tooling/fmt-check-writes-nothing`: a walk leaves a directory named
/// `vendor` alone, and the same directory named on the command line is
/// formatted.
// covers: tools:cli/nvs-fmt
#[test]
fn fmt_walk_skips_vendor_and_a_named_vendor_is_formatted() {
    let (dir, paths) = fixture("vendor", &[("own.nvs", MANGLED)]);
    let vendor = dir.join("vendor");
    std::fs::create_dir(&vendor).expect("the vendor directory is created");
    let pulled = vendor.join("pulled.nvs");
    std::fs::write(&pulled, MANGLED).expect("the vendored file is written");

    let (_, err, code) = fmt(&[dir.as_os_str()]);
    assert_eq!(code, Some(0), "the walk formats the project's file: {err}");
    assert_eq!(
        text(&paths[0]),
        CANONICAL,
        "the project's own file was rewritten"
    );
    assert_eq!(text(&pulled), MANGLED, "the vendored file was left alone");

    let (_, err, code) = fmt(&[vendor.as_os_str()]);
    assert_eq!(
        code,
        Some(0),
        "a named vendor directory is formatted: {err}"
    );
    assert_eq!(
        text(&pulled),
        CANONICAL,
        "the named vendored file was rewritten"
    );
}

/// `--diff`: the same verdict as `--check`, rendered as the change itself.
// covers: tools:cli/nvs-fmt
#[test]
fn fmt_diff_prints_the_diff_and_writes_nothing() {
    let (_dir, paths) = fixture("diff", &[("ragged.nvs", MANGLED)]);

    let (out, _, code) = fmt(&[std::ffi::OsStr::new("--diff"), paths[0].as_os_str()]);
    assert_eq!(code, Some(1), "a file that would change exits 1");
    assert!(
        out.starts_with("--- ") && out.contains("+++ ") && out.contains("@@ -"),
        "a unified diff, headed and hunked: {out}"
    );
    assert!(
        out.lines()
            .any(|line| line == "-      public string $name;"),
        "the line as it stands is on the minus side: {out}"
    );
    assert!(
        out.lines().any(|line| line == "+    public string $name;"),
        "the line as it would stand is on the plus side: {out}"
    );
    assert_eq!(
        text(&paths[0]),
        MANGLED,
        "`--diff` wrote nothing to the file it printed"
    );
}

/// `--stdin`: the buffer an editor holds, formatted onto standard output.
// covers: tools:cli/nvs-fmt
#[test]
fn fmt_stdin_writes_the_formatted_file_to_stdout() {
    let (out, err, code) = stdin(MANGLED, &[]);
    assert_eq!(code, Some(0), "a buffer that parses is formatted: {err}");
    assert_eq!(out, CANONICAL, "the whole formatted file, and nothing else");
}

/// `--check --stdin`: the buffer is never written back, and the exit code is
/// the one a file would get.
// covers: tools:cli/nvs-fmt
#[test]
fn fmt_check_stdin_writes_nothing_and_exits_by_what_it_found() {
    let (out, err, code) = stdin(MANGLED, &["--check"]);
    assert_eq!(code, Some(1), "a buffer that would change exits 1: {err}");
    assert_eq!(out, "<stdin>\n", "it is named, and its text is not printed");

    let (out, err, code) = stdin(CANONICAL, &["--check"]);
    assert_eq!(code, Some(0), "a formatted buffer exits 0: {err}");
    assert_eq!(out, "", "and is named nowhere");

    let (out, err, code) = stdin(BROKEN, &["--check"]);
    assert_eq!(code, Some(2), "a buffer that does not parse exits 2: {err}");
    assert_eq!(out, "", "standard output is empty");
}

/// The safe direction: a half-written buffer comes back to its editor
/// unchanged, because nothing at all was written for it.
// covers: tools:cli/nvs-fmt
#[test]
fn fmt_stdin_writes_nothing_to_stdout_for_a_file_with_a_syntax_error() {
    let (out, err, code) = stdin(BROKEN, &[]);
    assert_eq!(code, Some(2), "a buffer that does not parse exits 2");
    assert_eq!(out, "", "standard output is empty, not partial");
    assert!(
        err.contains("<stdin>"),
        "the refusal names what it refused: {err}"
    );
}

/// `rule:tooling/fmt-is-never-a-diagnostic`: layout is outside the compiler's
/// diagnostic surface entirely, so a file no formatter has ever touched checks
/// clean and is never so much as mentioned.
// covers: tools:cli/nvs-fmt
#[test]
fn nvs_check_never_reports_an_unformatted_file() {
    let (dir, paths) = fixture("never-a-diagnostic", &[("ragged.nvs", UNFORMATTED)]);

    // The premise, from the formatter's own side: this file *is* unformatted.
    let (_, _, code) = fmt(&[std::ffi::OsStr::new("--check"), paths[0].as_os_str()]);
    assert_eq!(
        code,
        Some(1),
        "the fixture is a file `nvs fmt` would rewrite"
    );

    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        // Run from the fixture's own directory with a private data folder of
        // its own and no template written into it: this asks what the
        // *checker* says, not what a configuration tree beside it says.
        .current_dir(&dir)
        .arg("--data")
        .arg(&*data)
        .args(["--no-init", "check"])
        .arg(&paths[0])
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let reported = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "an unformatted file compiles: {reported}"
    );
    assert!(
        !reported.to_lowercase().contains("format"),
        "and `nvs check` says nothing about its layout: {reported}"
    );
}

/// The attack under `tests/hostile/tools/cli/nvs-fmt/`: twelve blocks on one
/// line, strings and comments that look like code, tabs, and text that is not
/// ASCII. `--stdin` formats it, a second run over the result changes nothing
/// (`rule:tooling/fmt-is-idempotent`), and every comment and string is kept.
// covers: tools:cli/nvs-fmt
#[test]
fn fmt_formats_a_file_that_is_hard_to_format_to_a_fixed_point() {
    let path = nvs_repo::path("tests")
        .join("hostile")
        .join("tools")
        .join("cli")
        .join("nvs-fmt")
        .join("01-a-file-that-is-hard-to-format.nvs");
    let source = std::fs::read_to_string(&path).expect("the attack is on disk");

    let (once, err, code) = stdin(&source, &[]);
    assert_eq!(code, Some(0), "the attack is formatted: {err}");
    assert_ne!(once, source, "the attack is not in the canonical layout");

    let (twice, err, code) = stdin(&once, &[]);
    assert_eq!(
        code,
        Some(0),
        "the formatted attack is formatted again: {err}"
    );
    assert_eq!(twice, once, "a second run changes nothing");

    for kept in [
        "/* one */",
        "// three",
        "'café 日本 😀'",
        "'{ } ; ?> // not a comment'",
        "'/* not a comment */'",
        "} // the end of the class",
    ] {
        assert!(once.contains(kept), "`{kept}` is kept: {once}");
    }
}
