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
fn fixture(name: &str, files: &[(&str, &str)]) -> (PathBuf, Vec<PathBuf>) {
    let dir = std::env::temp_dir().join(format!("nvs-fmt-cmd-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a private directory under the temp dir");
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

/// `nvs fmt <args...>`, as `(stdout, stderr, success)`.
fn fmt<S: AsRef<std::ffi::OsStr>>(args: &[S]) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("fmt")
        .args(args)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the text is UTF-8"),
        String::from_utf8(out.stderr).expect("the diagnostics are UTF-8"),
        out.status.success(),
    )
}

/// `nvs fmt --stdin` over `source`, as `(stdout, stderr, success)`.
fn stdin(source: &str) -> (String, String, bool) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["fmt", "--stdin"])
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
        out.status.success(),
    )
}

/// What `path` holds now.
fn text(path: &Path) -> String {
    std::fs::read_to_string(path).expect("the fixture is still on disk")
}

/// The default mode: the named files, rewritten where they sit, and a file the
/// parse reported an error in left exactly as its author last saved it.
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

    let (out, err, ok) = fmt(&paths[..2]);
    assert!(ok, "two files that parse are formatted: {err}");
    assert_eq!(out, "", "the default mode prints nothing on success");
    assert_eq!(text(&paths[0]), CANONICAL, "the first file was rewritten");
    assert_eq!(text(&paths[1]), CANONICAL, "the second file was rewritten");

    // The same call again: the formatter is a fixed point, so the second run
    // has nothing to write (`rule:tooling/fmt-is-idempotent`).
    let (_, err, ok) = fmt(&paths[..2]);
    assert!(
        ok,
        "a formatted file is formatted again without complaint: {err}"
    );
    assert_eq!(text(&paths[0]), CANONICAL, "the second run changed nothing");

    let (_, err, ok) = fmt(&paths[2..]);
    assert!(!ok, "a file that does not parse exits non-zero");
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

/// `--check`: the file list on standard output, a non-zero exit, and a disk
/// nothing touched.
#[test]
fn fmt_check_writes_nothing_and_exits_non_zero_naming_each_file() {
    let (_dir, paths) = fixture("check", &[("ragged.nvs", MANGLED), ("tidy.nvs", CANONICAL)]);

    let (out, _, ok) = fmt(&[
        std::ffi::OsStr::new("--check"),
        paths[0].as_os_str(),
        paths[1].as_os_str(),
    ]);
    assert!(!ok, "a file that would change exits non-zero");
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

    let (out, _, ok) = fmt(&[std::ffi::OsStr::new("--check"), paths[1].as_os_str()]);
    assert!(ok, "an already-formatted file exits zero");
    assert_eq!(out, "", "and is named nowhere");
}

/// `--diff`: the same verdict as `--check`, rendered as the change itself.
#[test]
fn fmt_diff_prints_the_diff_and_writes_nothing() {
    let (_dir, paths) = fixture("diff", &[("ragged.nvs", MANGLED)]);

    let (out, _, ok) = fmt(&[std::ffi::OsStr::new("--diff"), paths[0].as_os_str()]);
    assert!(!ok, "a file that would change exits non-zero");
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
#[test]
fn fmt_stdin_writes_the_formatted_file_to_stdout() {
    let (out, err, ok) = stdin(MANGLED);
    assert!(ok, "a buffer that parses is formatted: {err}");
    assert_eq!(out, CANONICAL, "the whole formatted file, and nothing else");
}

/// The safe direction: a half-written buffer comes back to its editor
/// unchanged, because nothing at all was written for it.
#[test]
fn fmt_stdin_writes_nothing_to_stdout_for_a_file_with_a_syntax_error() {
    let (out, err, ok) = stdin(BROKEN);
    assert!(!ok, "a buffer that does not parse exits non-zero");
    assert_eq!(out, "", "standard output is empty, not partial");
    assert!(
        err.contains("<stdin>"),
        "the refusal names what it refused: {err}"
    );
}

/// `rule:tooling/fmt-is-never-a-diagnostic`: layout is outside the compiler's
/// diagnostic surface entirely, so a file no formatter has ever touched checks
/// clean and is never so much as mentioned.
#[test]
fn nvs_check_never_reports_an_unformatted_file() {
    let (dir, paths) = fixture("never-a-diagnostic", &[("ragged.nvs", UNFORMATTED)]);

    // The premise, from the formatter's own side: this file *is* unformatted.
    let (_, _, formatted) = fmt(&[std::ffi::OsStr::new("--check"), paths[0].as_os_str()]);
    assert!(!formatted, "the fixture is a file `nvs fmt` would rewrite");

    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        // Run from the fixture's own directory and write no `nvs.toml` into
        // it: this asks what the *checker* says, not what a configuration
        // tree beside it says.
        .current_dir(&dir)
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
