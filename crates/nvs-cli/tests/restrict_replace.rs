//! `rule:packaging/a-restriction-limits-where-a-declaration-is-used`'s `replace`: it is compiled
//! where the `[[restrict]]` entry is written, and an error in it is `E0666` in `package.toml`. The
//! replacement that compiles is the fix `nvs check --fix` writes at a refused use.
//!
//! Each test builds one program: `App\Secret\Vault::open` is allowed only in `App\Allowed`, the
//! entry file calls it from outside, and the entry's `replace` is the one thing that changes.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The restricted class.
const VAULT: &str = r#"<?nvs
namespace App\Secret;

final class Vault
{
    public static function open(string $who): string
    {
        return "open " . $who;
    }
}
"#;

/// The allowed class, with a method that fits the restricted one and one that does not.
const DOOR: &str = r"<?nvs
namespace App\Allowed;

final class Door
{
    public static function open(string $who): string
    {
        return App\Secret\Vault::open($who);
    }

    public static function count(string $who): int
    {
        return 1;
    }
}
";

/// The entry: one call through the allowed class, which loads it, then one call of the restricted
/// method, outside the allowed places.
const MAIN: &str = "<?nvs\nautoload 'App' from './app';\n\necho App\\Allowed\\Door::open(\"you\"), \"\\n\";\n\
                    echo App\\Secret\\Vault::open(\"me\"), \"\\n\";\n";

/// A program whose entry restricts `App\Secret\Vault::open` with `replace` as its replacement.
fn program(replace: &str) -> (nvs_repo::Scratch, PathBuf) {
    let root = nvs_repo::scratch("restrict-replace");
    let manifest = format!(
        "[[restrict]]\nuse     = 'App\\Secret\\Vault::open'\nallow   = ['App\\Allowed']\n\
         replace = '{replace}'\n"
    );
    for (path, text) in [
        ("package.toml", manifest.as_str()),
        ("main.nvs", MAIN),
        ("app/Secret/Vault.nvs", VAULT),
        ("app/Allowed/Door.nvs", DOOR),
    ] {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("every file is in a directory"))
            .expect("the test makes its directories");
        std::fs::write(&path, text).expect("the test writes its files");
    }
    let entry = root.join("main.nvs");
    (root, entry)
}

/// `nvs check <args> <entry>`, as `(stderr, success)`.
fn check(args: &[&str], entry: &Path) -> (String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("check")
        .args(args)
        .arg(entry)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// Whether `stderr` has one `E0666` pointing into `package.toml`, with `message`.
fn bad_replacement(stderr: &str, message: &str) -> bool {
    stderr.contains("error[E0666]")
        && stderr.contains("package.toml:4:")
        && stderr.contains(message)
}

#[test]
fn a_replacement_that_does_not_compile_is_an_error_in_package_toml() {
    let (_root, entry) = program("App\\Allowed\\Door::open($whom)");
    let (stderr, ok) = check(&[], &entry);
    assert!(!ok, "{stderr}");
    assert!(
        bad_replacement(&stderr, "the replacement does not compile"),
        "{stderr}"
    );

    // The positive control: a replacement that compiles is no error.
    let (_root, entry) = program("App\\Allowed\\Door::open($who)");
    let (stderr, _) = check(&[], &entry);
    assert!(!stderr.contains("E0666"), "{stderr}");
    assert!(stderr.contains("error[E0664]"), "{stderr}");
}

#[test]
fn a_replacement_of_the_wrong_type_is_an_error() {
    let (_root, entry) = program("App\\Allowed\\Door::count($who)");
    let (stderr, ok) = check(&[], &entry);
    assert!(!ok, "{stderr}");
    assert!(
        bad_replacement(&stderr, "the replacement is `int`"),
        "{stderr}"
    );
}

#[test]
fn a_replacement_that_uses_the_restricted_declaration_is_an_error() {
    let (_root, entry) = program("App\\Secret\\Vault::open($who)");
    let (stderr, ok) = check(&[], &entry);
    assert!(!ok, "{stderr}");
    assert!(
        bad_replacement(
            &stderr,
            "the replacement uses `App\\Secret\\Vault::open`, which this entry restricts"
        ),
        "{stderr}"
    );
}

#[test]
fn the_fix_fills_in_the_replacement_at_the_use() {
    let (_root, entry) = program("App\\Allowed\\Door::open($who)");
    let (stderr, _) = check(&["--fix"], &entry);
    let fixed = std::fs::read_to_string(&entry).expect("the entry is still there");
    assert!(
        fixed.contains("echo Door::open(\"me\"), \"\\n\";")
            && fixed.contains("use App\\Allowed\\Door;"),
        "{fixed}\n{stderr}"
    );
    let (stderr, ok) = check(&[], &entry);
    assert!(ok, "the fixed program compiles: {stderr}");
}
