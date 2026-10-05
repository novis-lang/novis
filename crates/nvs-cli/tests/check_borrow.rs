//! `nvs check` on a file a program autoloads checks it through that program's
//! `autoload` map (`rule:ide/an-autoloaded-file-borrows-its-programs-map`).
//!
//! A class file under an `autoload` root may not declare `autoload` itself, so
//! checked as its own entry point every name its program autoloads would be
//! undeclared. These cases hold that the command finds the program that lends
//! the map, that a real mistake in the class file is still reported, that the
//! first program in path order lends, that a program naming the file's root
//! outranks one that only holds it by directory, that a file no program lends
//! to is checked as before, and that a file declaring its own map never
//! borrows.
//!
//! Through the built binary, because the survey's root is the working
//! directory the command runs in.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A project directory of the case's own under the target's scratch, removed
/// when the case ends.
struct Project {
    dir: PathBuf,
}

impl Project {
    fn new(case: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("check-borrow")
            .join(case);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("the target's scratch directory is writable");
        Self { dir }
    }

    fn write(&self, name: &str, text: &str) {
        let path = self.dir.join(name);
        fs::create_dir_all(path.parent().expect("a fixture sits in a directory"))
            .expect("a scratch directory");
        fs::write(path, text).expect("a writable scratch file");
    }

    /// Runs `nvs check` with `args` from the project directory.
    fn check(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("check")
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("the `nvs` binary this test was built beside runs")
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The program: it maps `App` to `site/app` and uses one class from it.
const MAIN: &str = "<?nvs\nautoload 'App' from './app';\n\nuse App\\User;\n\nnew User();\n";

/// A class the program autoloads, naming another one it autoloads.
const USER: &str = "<?nvs\nnamespace App;\n\nuse App\\Mailer;\n\nclass User {\n    \
                    public function send(): int {\n        return Mailer::count();\n    }\n}\n";

const MAILER: &str = "<?nvs\nnamespace App;\n\nclass Mailer {\n    \
                      public static function count(): int {\n        return 1;\n    }\n}\n";

/// `site/` holds the program and the classes it autoloads.
fn site(project: &Project) {
    project.write("site/main.nvs", MAIN);
    project.write("site/app/User.nvs", USER);
    project.write("site/app/Mailer.nvs", MAILER);
}

/// A class file checks clean through the map of the program that autoloads
/// it, in both renderings, exactly as the program does.
// covers: tools:cli/nvs-check
#[test]
fn a_class_file_checks_through_the_map_of_the_program_that_autoloads_it() {
    let project = Project::new("lent");
    site(&project);

    let program = project.check(&["site/main.nvs"]);
    assert_eq!(program.status.code(), Some(0), "{}", stderr(&program));

    let class = project.check(&["site/app/User.nvs"]);
    assert_eq!(class.status.code(), Some(0), "{}", stderr(&class));
    assert_eq!(String::from_utf8_lossy(&class.stdout), "no errors\n");

    let json = project.check(&["--json", "site/app/User.nvs"]);
    assert_eq!(json.status.code(), Some(0), "{}", stderr(&json));
    let document: serde_json::Value =
        serde_json::from_slice(&json.stdout).expect("`--json` writes one document");
    assert_eq!(document["diagnostics"], serde_json::json!([]));
}

/// A mistake in the class file is reported, and the names the program
/// autoloads are not.
#[test]
fn a_mistake_in_a_lent_class_file_is_still_reported() {
    let project = Project::new("mistake");
    site(&project);
    project.write(
        "site/app/Broken.nvs",
        "<?nvs\nnamespace App;\n\nuse App\\Mailer;\n\nclass Broken {\n    \
         public function send(): int {\n        string $n = Mailer::count();\n        \
         return 0;\n    }\n}\n",
    );

    let output = project.check(&["site/app/Broken.nvs"]);
    let stderr = stderr(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("error[E0401]"), "{stderr}");
    assert!(
        !stderr.contains("E0303") && !stderr.contains("E0306"),
        "{stderr}"
    );
}

/// A file no program lends to reports its undeclared names as before.
#[test]
fn a_file_no_program_lends_to_is_checked_as_before() {
    let project = Project::new("unlent");
    site(&project);
    project.write(
        "other/Lone.nvs",
        "<?nvs\nnamespace Other;\n\nuse App\\User;\n\nclass Lone {\n    \
         public function make(): User {\n        return new User();\n    }\n}\n",
    );

    let output = project.check(&["other/Lone.nvs"]);
    let stderr = stderr(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("E0306") || stderr.contains("E0303"),
        "{stderr}"
    );
}

/// Two programs map one source tree and give `Lib` different roots. The
/// first by entry path lends, and its map alone.
#[test]
fn the_first_program_by_entry_path_lends() {
    let project = Project::new("two-lenders");
    project.write(
        "one.nvs",
        "<?nvs\nautoload 'App' from './shared';\nautoload 'Lib' from './lib_one';\n",
    );
    project.write(
        "two.nvs",
        "<?nvs\nautoload 'App' from './shared';\nautoload 'Lib' from './lib_two';\n",
    );
    project.write(
        "lib_one/Thing.nvs",
        "<?nvs\nnamespace Lib;\n\nclass Thing {\n    \
         public static function one(): int {\n        return 1;\n    }\n}\n",
    );
    project.write(
        "lib_two/Thing.nvs",
        "<?nvs\nnamespace Lib;\n\nclass Thing {\n    \
         public static function two(): int {\n        return 2;\n    }\n}\n",
    );
    project.write(
        "shared/One.nvs",
        "<?nvs\nnamespace App;\n\nuse Lib\\Thing;\n\nclass One {\n    \
         public function get(): int {\n        return Thing::one();\n    }\n}\n",
    );
    project.write(
        "shared/Two.nvs",
        "<?nvs\nnamespace App;\n\nuse Lib\\Thing;\n\nclass Two {\n    \
         public function get(): int {\n        return Thing::two();\n    }\n}\n",
    );

    let one = project.check(&["shared/One.nvs"]);
    assert_eq!(one.status.code(), Some(0), "{}", stderr(&one));
    let two = project.check(&["shared/Two.nvs"]);
    let stderr = stderr(&two);
    assert_eq!(
        two.status.code(),
        Some(1),
        "`one.nvs` lends, and its `Thing` has no `two`: {stderr}"
    );
    assert!(
        !stderr.contains("E0303") && !stderr.contains("E0306"),
        "{stderr}"
    );
}

/// A program at the project's root holds every plain file in the tree by the
/// directory test, and here it sorts first. The program whose declaration
/// names the class file's root owns the file, so its map lends.
#[test]
fn a_program_that_names_the_files_root_outranks_one_at_the_root() {
    let project = Project::new("owner-first");
    project.write(
        "app.nvs",
        "<?nvs\nautoload 'Demo' from './demo';\n\necho 1;\n",
    );
    project.write("bootstrap.nvs", "<?nvs\nautoload 'Lib' from './src';\n");
    project.write(
        "src/Point.nvs",
        "<?nvs\nnamespace Lib;\n\nclass Point {\n    \
         public function value(): int {\n        return 1;\n    }\n}\n",
    );
    project.write(
        "src/GroupedPoint.nvs",
        "<?nvs\nnamespace Lib;\n\nclass GroupedPoint extends Point {\n    \
         public function group(): Point {\n        return new Point();\n    }\n}\n",
    );

    let checked = project.check(&["src/GroupedPoint.nvs"]);
    assert_eq!(checked.status.code(), Some(0), "{}", stderr(&checked));
}

/// A file that declares its own `autoload` is a program, and never borrows
/// another program's map.
#[test]
fn a_file_with_its_own_autoload_never_borrows() {
    let project = Project::new("own-map");
    site(&project);
    project.write("site/lib/.keep", "");
    project.write(
        "site/tool.nvs",
        "<?nvs\nautoload 'Lib' from './lib';\n\nuse App\\User;\n\nnew User();\n",
    );

    let output = project.check(&["site/tool.nvs"]);
    let stderr = stderr(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("E0306") || stderr.contains("E0303"),
        "{stderr}"
    );
}
