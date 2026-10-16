//! `rule:programs/relative-paths-resolve-from-their-file`: which string
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
        .written_paths()
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

/// A written relative path at a `Core\IO` path parameter becomes an absolute path
/// under the folder of the file that wrote it. `.` and `..` are removed.
// covers: lang:programs/file-paths-a-relative-path-starts-at-the-folder-of-its-file
#[test]
fn a_written_relative_path_at_a_core_path_parameter_is_joined_to_its_file() {
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
fn a_written_relative_path_naming_an_isolates_script_is_joined_to_its_file() {
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

/// `Core\Queue::push`'s script is a path position too, so the row a worker
/// reads stores the absolute path. A variable is left alone and checked when
/// the push runs.
#[test]
fn queue_push_written_script_path_resolves_from_the_file_that_wrote_it() {
    let (diags, exprs) = check_program_table(&[(
        "pushes.nvs",
        "<?nvs\nCore\\Queue::push('jobs/../jobs/report.nvs');\n\
         var $script = 'jobs/other.nvs';\n\
         Core\\Queue::push($script);\n",
    )]);
    assert!(!diags.has_errors(), "{diags:?}");
    let paths = resolved(&exprs);
    assert_eq!(paths.len(), 1, "{paths:?}");
    let path = &paths[0];
    assert!(Path::new(path).has_root(), "{path}");
    assert!(path.ends_with(&tail(&["jobs", "report.nvs"])), "{path}");
    assert!(!path.contains(".."), "{path}");
}

/// A literal in a file reached through `require` is joined to *that* file's
/// folder, not to the entry file's.
#[test]
fn a_written_path_in_a_required_file_is_joined_to_that_files_folder() {
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
fn an_absolute_written_path_and_a_variable_are_left_alone() {
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

/// A path argument that starts with a written relative path and adds a value built
/// at run time is relative when the program runs, so the call would always
/// throw. A concatenation, an interpolated string and a user `#[Core\Path]`
/// parameter each do not compile, at the literal that starts them.
#[test]
fn a_path_built_from_a_written_relative_path_does_not_compile() {
    let src = "<?nvs\nclass Store {\n  public static function load(#[Core\\Path] string $file): \
               string {\n    return $file;\n  }\n}\nstring $name = 'x';\n\
               echo Core\\IO::read('data/' . $name . '.txt');\n\
               echo Core\\IO::read(\"data/{$name}.txt\");\n\
               echo Store::load('data/' . $name);\n\
               echo Core\\IO::read(('report-' . $name) . '.txt');\n";
    let (diags, exprs) = check_program_table(&[("built.nvs", src)]);
    let found: Vec<&str> = diags
        .iter()
        .filter(|d| d.code == Some(code::E_PATH_BUILT_FROM_A_WRITTEN_RELATIVE_PATH))
        .map(|d| {
            &src[d
                .primary_span()
                .map_or(0..0, |s| s.start as usize..s.end as usize)]
        })
        .collect();
    assert_eq!(
        found,
        ["'data/'", "data/", "'data/'", "'report-'"],
        "{diags:?}"
    );
    assert!(resolved(&exprs).is_empty(), "{:?}", resolved(&exprs));
}

/// A source with no file still reports a path built from a written relative path:
/// the folder is unknown, but the path is relative at run time either way.
#[test]
fn a_path_built_from_a_written_relative_path_does_not_compile_without_a_file() {
    let (diags, _) = common::check_src_table(
        "<?nvs\nstring $name = 'x';\necho Core\\IO::read('data/' . $name);\n",
    );
    assert_eq!(
        count(&diags, code::E_PATH_BUILT_FROM_A_WRITTEN_RELATIVE_PATH),
        1,
        "{diags:?}"
    );
}

/// A path built from something other than a written relative path may be absolute
/// at run time, so it compiles: a variable first, `Core\Path::thisDir`, an
/// absolute literal, a drive, a single letter that a drive may follow, an
/// interpolated string that starts with a value, and a plain written relative path,
/// which is joined to its file. A parameter that is not a path is not checked.
#[test]
fn a_path_built_from_anything_else_compiles() {
    let (diags, exprs) = check_program_table(&[(
        "built_ok.nvs",
        "<?nvs\nstring $name = 'x';\n\
         echo Core\\IO::exists($name . '/x.txt') ? 'y' : 'n';\n\
         echo Core\\IO::exists(Core\\Path::thisDir('data') . '/' . $name) ? 'y' : 'n';\n\
         echo Core\\IO::exists('/srv/data/' . $name) ? 'y' : 'n';\n\
         echo Core\\IO::exists('C:' . $name) ? 'y' : 'n';\n\
         echo Core\\IO::exists('C' . $name) ? 'y' : 'n';\n\
         echo Core\\IO::exists(\"{$name}/x.txt\") ? 'y' : 'n';\n\
         echo Core\\IO::exists('data/x.json') ? 'y' : 'n';\n\
         echo Core\\Str::length('data/' . $name), \"\\n\";\n",
    )]);
    assert!(!diags.has_errors(), "{diags:?}");
    assert_eq!(resolved(&exprs).len(), 2, "{:?}", resolved(&exprs));
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

/// `Core\Path::thisFile()` is replaced by the absolute path of the file that
/// contains the call.
// covers: Core\Path::thisFile
#[test]
fn this_file_folds_to_the_absolute_path_of_the_file_that_wrote_it() {
    let (diags, exprs) = check_program_table(&[(
        "this_file.nvs",
        "<?nvs\necho Core\\Path::thisFile(), \"\\n\";\n",
    )]);
    assert!(!diags.has_errors(), "{diags:?}");
    let paths = resolved(&exprs);
    assert_eq!(paths.len(), 1, "{paths:?}");
    let path = &paths[0];
    assert!(Path::new(path).has_root(), "{path}");
    assert!(path.ends_with(&tail(&["this_file.nvs"])), "{path}");
    assert!(!path.starts_with(r"\\?\"), "{path}");
}

/// A call in a file reached through `require` names *that* file, not the
/// entry file.
// covers: Core\Path::thisFile
#[test]
fn this_file_in_a_required_file_names_that_file() {
    let (diags, exprs) = check_program_table(&[
        (
            "this_file_required.nvs",
            "<?nvs\nrequire 'lib/blog.nvs';\necho Blog::source(), \"\\n\";\n",
        ),
        (
            "lib/blog.nvs",
            "<?nvs\nclass Blog {\n  public static function source(): string {\n    \
             return Core\\Path::thisFile();\n  }\n}\n",
        ),
    ]);
    assert!(!diags.has_errors(), "{diags:?}");
    let paths = resolved(&exprs);
    assert_eq!(paths.len(), 1, "{paths:?}");
    assert!(paths[0].ends_with(&tail(&["lib", "blog.nvs"])), "{paths:?}");
}

/// `Core\Path::thisDir()` is the folder `thisFile()` is in, and `null` as the
/// join is the same as writing none.
// covers: Core\Path::thisDir
#[test]
fn this_dir_folds_to_the_folder_of_the_file_that_wrote_it() {
    let (diags, exprs) = check_program_table(&[
        (
            "this_dir.nvs",
            "<?nvs\nrequire 'shop/report.nvs';\necho Report::where(), \"\\n\";\n",
        ),
        (
            "shop/report.nvs",
            "<?nvs\nclass Report {\n  public static function where(): string {\n    \
             return Core\\Path::thisFile() . Core\\Path::thisDir() . Core\\Path::thisDir(null);\n  \
             }\n}\n",
        ),
    ]);
    assert!(!diags.has_errors(), "{diags:?}");
    let paths = resolved(&exprs);
    assert_eq!(paths.len(), 3, "{paths:?}");
    let (dir, file) = (&paths[0], &paths[2]);
    assert_eq!(&paths[1], dir, "{paths:?}");
    assert!(dir.ends_with(&tail(&["shop"])), "{dir}");
    assert_eq!(Path::new(file).parent(), Some(Path::new(dir)), "{paths:?}");
}

/// The entry file and a file it requires name their folders the same way. The
/// entry is loaded by the path the caller gave and the required file by its
/// canonical path, and in a temporary folder the two differ: a Windows short
/// name such as `RUNNER~1`, or macOS's `/var` link to `/private/var`.
// covers: Core\Path::thisDir
#[test]
fn this_dir_in_the_entry_and_in_a_required_file_name_one_folder_the_same_way() {
    let (diags, exprs) = check_program_table(&[
        (
            "this_dir_entry.nvs",
            "<?nvs\nrequire 'lib/blog.nvs';\necho Core\\Path::thisDir(), Blog::folder(), \"\\n\";\n",
        ),
        (
            "lib/blog.nvs",
            "<?nvs\nclass Blog {\n  public static function folder(): string {\n    \
             return Core\\Path::thisDir();\n  }\n}\n",
        ),
    ]);
    assert!(!diags.has_errors(), "{diags:?}");
    let paths = resolved(&exprs);
    assert_eq!(paths.len(), 2, "{paths:?}");
    assert_eq!(
        Path::new(&paths[1]).parent(),
        Some(Path::new(&paths[0])),
        "{paths:?}"
    );
}

/// A written relative path is joined to the folder as a written path is, with `.`
/// and `..` removed, and a named argument fills the same parameter.
// covers: Core\Path::thisDir
#[test]
fn this_dir_joins_a_written_relative_path() {
    let (diags, exprs) = check_program_table(&[(
        "this_dir_join.nvs",
        "<?nvs\necho Core\\Path::thisDir(), \"\\n\";\n\
         echo Core\\Path::thisDir('data/./rates.json'), \"\\n\";\n\
         echo Core\\Path::thisDir(join: 'data/../mail'), \"\\n\";\n",
    )]);
    assert!(!diags.has_errors(), "{diags:?}");
    let paths = resolved(&exprs);
    assert_eq!(paths.len(), 3, "{paths:?}");
    let dir = &paths[0];
    assert!(
        paths.contains(&format!("{dir}{}", tail(&["data", "rates.json"]))),
        "{paths:?}"
    );
    assert!(
        paths.contains(&format!("{dir}{}", tail(&["mail"]))),
        "{paths:?}"
    );
}

/// A join that is a variable, a constant, a concatenation or an absolute path
/// does not compile, and records nothing.
// covers: Core\Path::thisDir
#[test]
fn this_dir_with_a_join_that_is_not_a_written_relative_path_does_not_compile() {
    let (diags, exprs) = check_program_table(&[(
        "this_dir_reject.nvs",
        "<?nvs\nclass Shop {\n  public const string DATA = 'data';\n}\n\
         string $part = 'data';\n\
         echo Core\\Path::thisDir($part), \"\\n\";\n\
         echo Core\\Path::thisDir(Shop::DATA), \"\\n\";\n\
         echo Core\\Path::thisDir('da' . 'ta'), \"\\n\";\n\
         echo Core\\Path::thisDir('/srv/data'), \"\\n\";\n\
         echo Core\\Path::thisDir(''), \"\\n\";\n",
    )]);
    assert_eq!(
        count(
            &diags,
            code::E_PATH_THIS_DIR_JOIN_NOT_A_WRITTEN_RELATIVE_PATH
        ),
        5,
        "{diags:?}"
    );
    assert!(resolved(&exprs).is_empty(), "{:?}", resolved(&exprs));
}
