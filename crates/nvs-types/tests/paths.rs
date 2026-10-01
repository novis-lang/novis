//! `rule:programs/path-literals-resolve-from-their-file`: which string
//! literals the checker resolves, against which folder, and where
//! `#[Core\Path]` may be written.

mod common;

use std::path::{MAIN_SEPARATOR_STR, Path};

use common::{check_program_table, check_src};
use nvs_diagnostics::{Diagnostics, code};
use nvs_types::ExprTypeTable;

/// Every path the checker resolved, sorted so a test can compare them.
fn resolved(exprs: &ExprTypeTable) -> Vec<String> {
    let mut paths: Vec<String> = exprs
        .path_literals()
        .map(|(_, path)| path.to_owned())
        .collect();
    paths.sort();
    paths
}

/// `parts` joined with this platform's separator, which is what the checker
/// writes between the folder and the literal.
fn tail(parts: &[&str]) -> String {
    format!("{MAIN_SEPARATOR_STR}{}", parts.join(MAIN_SEPARATOR_STR))
}

fn count(diags: &Diagnostics, want: nvs_diagnostics::Code) -> usize {
    diags.iter().filter(|d| d.code == Some(want)).count()
}

/// A relative literal at a `Core\IO` path parameter becomes an absolute path
/// under the folder of the file that wrote it. `.` and `..` are removed.
// covers: lang:programs/file-paths-a-literal-starts-at-the-folder-of-its-file
#[test]
fn a_relative_literal_at_a_core_path_parameter_is_joined_to_its_file() {
    let (diags, exprs) = check_program_table(&[(
        "joined.nvs",
        "<?nvs\necho Core\\IO::exists('./data/../data/x.json') ? 'y' : 'n';\n",
    )]);
    assert!(!diags.has_errors(), "{diags:?}");
    let paths = resolved(&exprs);
    assert_eq!(paths.len(), 1, "{paths:?}");
    let path = &paths[0];
    assert!(Path::new(path).has_root(), "{path}");
    assert!(path.ends_with(&tail(&["data", "x.json"])), "{path}");
    assert!(!path.contains(".."), "{path}");
    assert!(!path.starts_with(r"\\?\"), "{path}");
}

/// The script an isolate runs is a path position: the operand of `spawn
/// script` and the entry of both upgrades. A method entry is not a string
/// literal, so it records nothing.
#[test]
fn a_relative_literal_naming_an_isolates_script_is_joined_to_its_file() {
    let (diags, exprs) = check_program_table(&[(
        "spawns.nvs",
        "<?nvs\nclass Chat {\n  public static function run(): void {}\n}\n\
         var $job = spawn script 'jobs/child.nvs';\n\
         var $other = spawn script Chat::run(...);\n\
         Core\\Socket::upgrade('sockets/chat.nvs', null);\n\
         Core\\Sse::upgrade('streams/feed.nvs', null);\n",
    )]);
    assert!(!diags.has_errors(), "{diags:?}");
    let paths = resolved(&exprs);
    assert_eq!(paths.len(), 3, "{paths:?}");
    for (path, want) in paths.iter().zip([
        tail(&["jobs", "child.nvs"]),
        tail(&["sockets", "chat.nvs"]),
        tail(&["streams", "feed.nvs"]),
    ]) {
        assert!(Path::new(path).has_root(), "{path}");
        assert!(path.ends_with(&want), "{path} does not end with {want}");
    }
}

/// A literal in a file reached through `require` is joined to *that* file's
/// folder, not to the entry file's.
#[test]
fn a_literal_in_a_required_file_is_joined_to_that_files_folder() {
    let (diags, exprs) = check_program_table(&[
        (
            "required.nvs",
            "<?nvs\nrequire 'lib/reader.nvs';\necho Core\\IO::exists('data/x.json') ? 'y' : 'n';\n\
             echo Reader::here() ? 'y' : 'n';\n",
        ),
        (
            "lib/reader.nvs",
            "<?nvs\nclass Reader {\n  public static function here(): bool {\n    \
             return Core\\IO::exists('data/x.json');\n  }\n}\n",
        ),
    ]);
    assert!(!diags.has_errors(), "{diags:?}");
    let paths = resolved(&exprs);
    assert_eq!(paths.len(), 2, "{paths:?}");
    let in_lib = tail(&["lib", "data", "x.json"]);
    assert_eq!(
        paths.iter().filter(|path| path.ends_with(&in_lib)).count(),
        1,
        "{paths:?}"
    );
    for path in &paths {
        assert!(!path.starts_with(r"\\?\"), "{path}");
    }
}

/// An absolute literal is left as written, and a literal that is not written
/// at a path parameter is not resolved at all — including one that reaches a
/// path parameter later through a variable.
#[test]
fn an_absolute_literal_and_a_variable_are_left_alone() {
    let (diags, exprs) = check_program_table(&[(
        "absolute.nvs",
        "<?nvs\necho Core\\IO::exists('/srv/data/x.json') ? 'y' : 'n';\n\
         string $name = 'data/x.json';\necho Core\\IO::exists($name) ? 'y' : 'n';\n\
         echo Core\\Str::length('data/x.json'), \"\\n\";\n",
    )]);
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(resolved(&exprs).is_empty(), "{:?}", resolved(&exprs));
}

/// A user method marks a parameter with `#[Core\Path]`, and a literal passed
/// to it is resolved like one passed to `Core\IO`. A named argument fills the
/// same parameter.
#[test]
fn a_user_path_parameter_resolves_a_literal_written_at_the_call() {
    let (diags, exprs) = check_program_table(&[(
        "user.nvs",
        "<?nvs\nclass Store {\n  public static function load(#[Core\\Path] string $file, \
         string $label): string {\n    return $label . $file;\n  }\n}\n\
         echo Store::load('data/a.json', 'data/b.json'), \"\\n\";\n\
         echo Store::load(label: 'x', file: 'data/c.json'), \"\\n\";\n",
    )]);
    assert!(!diags.has_errors(), "{diags:?}");
    let paths = resolved(&exprs);
    assert_eq!(paths.len(), 2, "{paths:?}");
    assert!(paths[0].ends_with(&tail(&["data", "a.json"])), "{paths:?}");
    assert!(paths[1].ends_with(&tail(&["data", "c.json"])), "{paths:?}");
}

/// A source with no file behind it has no folder, so nothing is resolved and
/// the run-time check is what a relative path meets.
#[test]
fn a_source_with_no_file_resolves_nothing() {
    let (diags, exprs) =
        common::check_src_table("<?nvs\necho Core\\IO::exists('data/x.json') ? 'y' : 'n';\n");
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(resolved(&exprs).is_empty());
}

/// `#[Core\Path]` on a parameter that is not a `string` is refused, and so is
/// one on a declaration that takes no argument at all.
#[test]
fn the_marker_is_refused_away_from_a_string_parameter() {
    let diags = check_src(
        "<?nvs\n#[Core\\Path]\nclass Store {\n  \
         public static function a(#[Core\\Path] int $n): int { return $n; }\n  \
         public static function b(#[Core\\Path] ?string $p): ?string { return $p; }\n  \
         public static function c(#[Core\\Path] tainted string $p): int { return 1; }\n}\n",
    );
    assert_eq!(
        count(&diags, code::E_PATH_MARKER_NOT_ON_A_STRING),
        2,
        "{diags:?}"
    );
}
