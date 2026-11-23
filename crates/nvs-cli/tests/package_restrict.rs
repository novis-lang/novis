//! `rule:packaging/a-restriction-limits-where-a-declaration-is-used`, on module layouts each test
//! builds by hand under a `modules/` folder:
//!
//! - `modules/package.toml` is the application's file, for every module.
//! - `modules/Framework/package.toml` names the package `Framework\Core` and restricts
//!   `Core\Str::upper` to `Framework\Core\Signing\*`. Its `src/Signing/Signer.nvs` calls it.
//! - `modules/Blog/public/` and `modules/Shop/public/` hold the entries. Each autoloads
//!   `Framework\Core` from `../../Framework/src` and its own `Framework\{..}` from `../src`.
//!
//! Each refusal has a positive control beside it, the same layout with the line that breaks it
//! left out, so a refusal that fires on every use cannot pass.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The framework's manifest: the package's name and one restriction.
const FRAMEWORK: &str = "[package]\nname = 'Framework\\Core'\nversion = \"1.0.0\"\n\n\
                         [[restrict]]\nuse   = 'Core\\Str::upper'\nallow = ['Framework\\Core\\Signing\\*']\n";

/// The application's file, with no restriction of its own.
const APPLICATION: &str = "[grants]\n";

/// The framework's own code, allowed to call the restricted method.
const SIGNER: &str = r"<?nvs
namespace Framework\Core\Signing;

class Signer
{
    public static function sign(string $text): string
    {
        return Core\Str::upper($text);
    }
}
";

/// A module's file that declares the framework's allowed namespace from outside its folder.
const SHIM: &str = r"<?nvs
namespace Framework\Core\Signing;

class Shim
{
    public static function sign(string $text): string
    {
        return Core\Str::upper($text);
    }
}
";

/// A call through the framework's allowed code, which also loads the package.
const SIGN: &str = "echo Framework\\Core\\Signing\\Signer::sign('a'), \"\\n\";\n";

/// A direct call of the restricted method, on line 6 of a [`module_entry`].
const UPPER: &str = "echo Core\\Str::upper('b'), \"\\n\";\n";

/// A module's entry: the two `autoload` lines, a blank line, then `body` from line 5.
fn module_entry(body: &str) -> String {
    format!(
        "<?nvs\nautoload 'Framework\\Core' from '../../Framework/src';\n\
         autoload 'Framework\\{{..}}' from '../src';\n\n{body}"
    )
}

/// A fresh `modules` directory with `files` written under it, each `(path, text)`.
fn layout(files: &[(&str, &str)]) -> (nvs_repo::Scratch, PathBuf) {
    let root = nvs_repo::scratch("package-restrict");
    let modules = root.join("modules");
    for (path, text) in files {
        let path = modules.join(path);
        std::fs::create_dir_all(path.parent().expect("every file is in a directory"))
            .expect("the test makes its directories");
        std::fs::write(&path, text).expect("the test writes its files");
    }
    (root, modules)
}

/// `nvs check <entry>`, as `(stderr, success)`.
fn check(entry: &Path) -> (String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("check")
        .arg(entry)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// The 1-based line of the first line of `text` that starts with `prefix`.
fn line_of(text: &str, prefix: &str) -> usize {
    text.lines()
        .position(|line| line.starts_with(prefix))
        .expect("the text has the line")
        + 1
}

/// Whether `stderr` is one refused use of `Core\Str::upper`, allowed only in `allowed`.
fn refused(stderr: &str, allowed: &str) -> bool {
    stderr.contains("error[E0664]")
        && stderr.contains("`Core\\Str::upper` cannot be used here")
        && stderr.contains(&format!("It is allowed only in `{allowed}`."))
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn a_framework_folder_with_a_manifest_restricts_every_module_that_loads_it() {
    let direct = module_entry(&format!("{SIGN}{UPPER}"));
    let signed = module_entry(SIGN);
    let (_root, modules) = layout(&[
        ("package.toml", APPLICATION),
        ("Framework/package.toml", FRAMEWORK),
        ("Framework/src/Signing/Signer.nvs", SIGNER),
        ("Blog/public/index.nvs", &direct),
        ("Blog/public/signed.nvs", &signed),
        ("Shop/public/index.nvs", &direct),
        ("Shop/public/signed.nvs", &signed),
    ]);

    for module in ["Blog", "Shop"] {
        let (stderr, ok) = check(&modules.join(module).join("public/signed.nvs"));
        assert!(
            ok,
            "{module}: the framework's own `Signing` code compiles:\n{stderr}"
        );

        let (stderr, ok) = check(&modules.join(module).join("public/index.nvs"));
        assert!(!ok, "{module}: a direct call does not compile");
        assert!(
            refused(&stderr, "Framework\\Core\\Signing\\*") && stderr.contains("index.nvs:6:6"),
            "{module}: the error is at the entry's direct call:\n{stderr}"
        );
        assert!(
            !stderr.contains("Signer.nvs"),
            "{module}: the framework's own call is not refused:\n{stderr}"
        );
    }
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn a_module_file_that_declares_the_framework_namespace_is_still_refused() {
    // The Signing namespace has two roots: Blog's folder first, then the framework's.
    let head = "<?nvs\n\
                autoload 'Framework\\Core\\Signing' from '../src/Signing', '../../Framework/src/Signing';\n\
                autoload 'Framework\\Core' from '../../Framework/src';\n\n";
    let with_shim =
        format!("{head}{SIGN}echo Framework\\Core\\Signing\\Shim::sign('b'), \"\\n\";\n");
    let without_shim = format!("{head}{SIGN}");
    let (_root, modules) = layout(&[
        ("package.toml", APPLICATION),
        ("Framework/package.toml", FRAMEWORK),
        ("Framework/src/Signing/Signer.nvs", SIGNER),
        ("Blog/src/Signing/Shim.nvs", SHIM),
        ("Blog/public/index.nvs", &with_shim),
        ("Blog/public/signed.nvs", &without_shim),
    ]);

    let (stderr, ok) = check(&modules.join("Blog/public/signed.nvs"));
    assert!(
        ok,
        "the same namespace inside the framework's folder compiles:\n{stderr}"
    );

    let (stderr, ok) = check(&modules.join("Blog/public/index.nvs"));
    assert!(
        !ok,
        "the module's file in `Framework\\Core\\Signing` does not compile"
    );
    assert!(
        refused(&stderr, "Framework\\Core\\Signing\\*") && stderr.contains("Shim.nvs:8:16"),
        "the error is at the module file's call:\n{stderr}"
    );
    assert!(
        !stderr.contains("Signer.nvs"),
        "the framework's own call is not refused:\n{stderr}"
    );
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn the_frameworks_own_tests_read_its_manifest_as_the_application_file() {
    // The entry is inside `Framework/`, so the walk up from `Framework/tests` finds
    // `Framework/package.toml` as the application's file, and its entry matches by namespace.
    let head = "<?nvs\nautoload 'Framework\\Core' from '../src';\n\n";
    let direct = format!("{head}{SIGN}{UPPER}");
    let signed = format!("{head}{SIGN}");
    let borrowed = "<?nvs\n\
                    autoload 'Framework\\Core\\Signing' from '../../Blog/src/Signing', '../src/Signing';\n\
                    autoload 'Framework\\Core' from '../src';\n\n\
                    echo Framework\\Core\\Signing\\Shim::sign('b'), \"\\n\";\n";
    let (_root, modules) = layout(&[
        ("package.toml", APPLICATION),
        ("Framework/package.toml", FRAMEWORK),
        ("Framework/src/Signing/Signer.nvs", SIGNER),
        ("Framework/tests/main.nvs", &direct),
        ("Framework/tests/signed.nvs", &signed),
        ("Framework/tests/borrowed.nvs", borrowed),
        ("Blog/src/Signing/Shim.nvs", SHIM),
    ]);

    let (stderr, ok) = check(&modules.join("Framework/tests/signed.nvs"));
    assert!(ok, "the framework's own `Signing` code compiles:\n{stderr}");

    let (stderr, ok) = check(&modules.join("Framework/tests/main.nvs"));
    assert!(
        !ok,
        "a direct call from the framework's tests does not compile"
    );
    assert!(
        refused(&stderr, "Framework\\Core\\Signing\\*") && stderr.contains("main.nvs:5:6"),
        "the error is at the test's direct call:\n{stderr}"
    );

    let (stderr, ok) = check(&modules.join("Framework/tests/borrowed.nvs"));
    assert!(
        ok,
        "read as the application's file, the entry matches by namespace alone, \
         so a file outside the folder in `Framework\\Core\\Signing` compiles:\n{stderr}"
    );
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn a_restriction_of_a_package_the_program_never_loads_restricts_nothing() {
    let mail = "[package]\nname = 'Framework\\Mail'\nversion = \"1.0.0\"\n\n\
                [[restrict]]\nuse   = 'Core\\Str::upper'\nallow = ['Framework\\Mail\\*']\n";
    let sender = r"<?nvs
namespace Framework\Mail;

class Sender
{
    public static function subject(string $text): string
    {
        return Core\Str::upper($text);
    }
}
";
    let plain = format!("<?nvs\nautoload 'Framework\\{{..}}' from '../src';\n\n{UPPER}");
    let loaded = format!(
        "<?nvs\nautoload 'Framework\\Mail' from '../../Mail/src';\n\
         autoload 'Framework\\{{..}}' from '../src';\n\n\
         echo Framework\\Mail\\Sender::subject('a'), \"\\n\";\n{UPPER}"
    );
    let (_root, modules) = layout(&[
        ("package.toml", APPLICATION),
        ("Mail/package.toml", mail),
        ("Mail/src/Sender.nvs", sender),
        ("Blog/public/index.nvs", &plain),
        ("Blog/public/mail.nvs", &loaded),
    ]);

    let (stderr, ok) = check(&modules.join("Blog/public/index.nvs"));
    assert!(
        ok,
        "a package the entry never loads restricts nothing:\n{stderr}"
    );

    let (stderr, ok) = check(&modules.join("Blog/public/mail.nvs"));
    assert!(
        !ok && refused(&stderr, "Framework\\Mail\\*"),
        "once the entry loads the package, its restriction holds:\n{stderr}"
    );
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn every_restriction_on_a_declaration_must_hold() {
    let page = |namespace: &str| {
        format!(
            "<?nvs\nnamespace {namespace};\n\nclass Page\n{{\n    \
             public static function title(string $text): string\n    {{\n        \
             return Core\\Str::upper($text);\n    }}\n}}\n"
        )
    };
    let app_page = page("App");
    let admin_page = page("App\\Admin");

    // The application allows `App\*`, and the framework's own entry still holds.
    let application = "[[restrict]]\nuse   = 'Core\\Str::upper'\n\
                       allow = ['App\\*', 'Framework\\Core\\Signing\\*']\n";
    let alone = "<?nvs\nautoload 'App' from '../app';\n\necho App\\Page::title('a'), \"\\n\";\n";
    let with_framework = format!(
        "<?nvs\nautoload 'App' from '../app';\n\
         autoload 'Framework\\Core' from '../../Framework/src';\n\n\
         {SIGN}echo App\\Page::title('a'), \"\\n\";\n"
    );
    let (_root, modules) = layout(&[
        ("package.toml", application),
        ("Framework/package.toml", FRAMEWORK),
        ("Framework/src/Signing/Signer.nvs", SIGNER),
        ("Blog/app/Page.nvs", &app_page),
        ("Blog/public/index.nvs", &with_framework),
        ("Blog/public/alone.nvs", alone),
    ]);

    let (stderr, ok) = check(&modules.join("Blog/public/alone.nvs"));
    assert!(
        ok,
        "without the framework, the application's entry allows `App`:\n{stderr}"
    );

    let (stderr, ok) = check(&modules.join("Blog/public/index.nvs"));
    assert!(
        !ok && refused(&stderr, "Framework\\Core\\Signing\\*") && stderr.contains("Page.nvs:8:16"),
        "the application's allow does not lift the framework's entry:\n{stderr}"
    );
    assert!(
        !stderr.contains("Signer.nvs"),
        "the framework's own call passes both entries:\n{stderr}"
    );

    // Two entries of the application's file on one method: a use passes both or neither.
    let application = "[[restrict]]\nuse   = 'Core\\Str::upper'\nallow = ['App\\*']\n\n\
                       [[restrict]]\nuse   = 'Core\\Str::upper'\nallow = ['App\\Admin\\*']\n";
    let admin = "<?nvs\nautoload 'App' from '../app';\n\n\
                 echo App\\Admin\\Page::title('a'), \"\\n\";\n";
    let (_root, modules) = layout(&[
        ("package.toml", application),
        ("Blog/app/Page.nvs", &app_page),
        ("Blog/app/Admin/Page.nvs", &admin_page),
        ("Blog/public/index.nvs", alone),
        ("Blog/public/admin.nvs", admin),
    ]);

    let (stderr, ok) = check(&modules.join("Blog/public/admin.nvs"));
    assert!(ok, "`App\\Admin` passes both entries:\n{stderr}");

    // The second entry's `use` is the line above its `allow`.
    let second_use = line_of(application, "allow = ['App\\Admin") - 1;
    let (stderr, ok) = check(&modules.join("Blog/public/index.nvs"));
    assert!(
        !ok && refused(&stderr, "App\\Admin\\*")
            && stderr.contains(&format!("package.toml`, line {second_use}.")),
        "`App` passes the first entry and not the second:\n{stderr}"
    );
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn the_error_names_the_restricting_file_and_line() {
    let direct = module_entry(&format!("{SIGN}{UPPER}"));
    let signed = module_entry(SIGN);
    let (_root, modules) = layout(&[
        ("package.toml", APPLICATION),
        ("Framework/package.toml", FRAMEWORK),
        ("Framework/src/Signing/Signer.nvs", SIGNER),
        ("Blog/public/index.nvs", &direct),
        ("Blog/public/signed.nvs", &signed),
    ]);

    let (stderr, ok) = check(&modules.join("Blog/public/signed.nvs"));
    assert!(ok, "the entry without the direct call compiles:\n{stderr}");

    let line = line_of(FRAMEWORK, "use ");
    let file = Path::new("Framework").join("package.toml");
    let (stderr, ok) = check(&modules.join("Blog/public/index.nvs"));
    assert!(!ok, "the direct call does not compile");
    assert!(
        refused(&stderr, "Framework\\Core\\Signing\\*"),
        "the error names the method and where it is allowed:\n{stderr}"
    );
    assert!(
        stderr.contains("index.nvs:6:6") && stderr.contains("this use is not allowed here"),
        "the error is at the use:\n{stderr}"
    );
    assert!(
        stderr.contains(&format!("{}:{line}:9", file.display()))
            && stderr.contains("the restriction"),
        "a second label shows the manifest's `use` line:\n{stderr}"
    );
    assert!(
        stderr.contains(&format!(
            "This rule is in `Framework/package.toml`, line {line}."
        )),
        "a note names the manifest's path, relative to the application's file, and the line of \
         its `use`:\n{stderr}"
    );
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn a_target_that_names_nothing_warns_only_in_the_application_file() {
    let names_nothing = "[[restrict]]\nuse = 'Blog\\Missing'\nallow = []\n";
    let application = format!("{APPLICATION}\n{names_nothing}");
    // A package's file may restrict only its own code or `Core`'s, so its target that names
    // nothing lies under the package's own name.
    let framework =
        format!("{FRAMEWORK}\n[[restrict]]\nuse = 'Framework\\Core\\Missing'\nallow = []\n");
    let entry = module_entry(SIGN);

    let (_root, modules) = layout(&[
        ("package.toml", &application),
        ("Framework/package.toml", FRAMEWORK),
        ("Framework/src/Signing/Signer.nvs", SIGNER),
        ("Blog/public/index.nvs", &entry),
    ]);
    let (stderr, ok) = check(&modules.join("Blog/public/index.nvs"));
    assert!(ok, "a warning does not stop the compile:\n{stderr}");
    assert!(
        stderr.contains("warning[W1031]")
            && stderr.contains("`Blog\\Missing` is not a declaration of this program"),
        "the application's file names nothing, and the compiler warns:\n{stderr}"
    );

    let (_root, modules) = layout(&[
        ("package.toml", APPLICATION),
        ("Framework/package.toml", &framework),
        ("Framework/src/Signing/Signer.nvs", SIGNER),
        ("Blog/public/index.nvs", &entry),
    ]);
    let (stderr, ok) = check(&modules.join("Blog/public/index.nvs"));
    assert!(ok, "the package's table compiles:\n{stderr}");
    assert!(
        !stderr.contains("W1031"),
        "a package's file may name code this program does not load:\n{stderr}"
    );
}
