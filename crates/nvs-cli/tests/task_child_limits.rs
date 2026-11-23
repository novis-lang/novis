//! `rule:concurrency/one-scheduler`'s task is part of its request for every limit: a
//! `Core\Task::all` child keeps the `grants:` narrowing of the script it runs in, and its
//! allocation is refused at the request's memory ceiling before it is made.
//!
//! Both run the built `nvs` on a program written into a scratch folder, because the
//! narrowing and the ceiling are each armed by a different crate on the way from a spawn to
//! a task's stack, and only a whole run passes through all of them.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A fresh folder with `files` written under it, each `(path, text)`.
fn layout(files: &[(&str, &str)]) -> (nvs_repo::Scratch, PathBuf) {
    let root = nvs_repo::scratch("task-child-limits");
    let app = root.join("app");
    for (path, text) in files {
        let path = app.join(path);
        std::fs::create_dir_all(path.parent().expect("every file is in a folder"))
            .expect("the test makes its folders");
        std::fs::write(&path, text).expect("the test writes its files");
    }
    (root, app)
}

/// `nvs run main.nvs` in `app`, so its `nvs.toml` is the configuration, as
/// `(stdout, stderr, success)`.
fn run(app: &Path) -> (String, String, bool) {
    let data = nvs_repo::scratch_private("nvsdata");
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .current_dir(app)
        .arg("--data")
        .arg(&*data)
        .arg("--no-init")
        .arg("run")
        .arg("main.nvs")
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

/// A class that asks for `net.connect` and says whether it was granted. A loopback URL
/// is checked against the grant alone: no name is looked up and nothing is sent.
const ASK: &str = "class Ask\n{\n    public static function url(): string\n    {\n        try {\n            \
                   var $target = Core\\Http::allowUrl('http://127.0.0.1/status');\n            \
                   return 'allowed';\n        } catch (RuntimeError $refused) {\n            \
                   return 'refused';\n        }\n    }\n}\n";

// covers: Core\Task::all
#[test]
fn a_task_of_a_narrowed_script_is_refused_the_capability_the_script_lacks() {
    // The parent holds `net.connect`, so its own task is allowed: the control that shows
    // the refusal below is the narrowing's and not the configuration's.
    let main = format!(
        "<?nvs\n{ASK}\nvar $group = Core\\Task::all({{\n    inside: fn(): string => Ask::url(),\n}});\n\
         echo 'parent task: ', $group->inside, \"\\n\";\n\n\
         var $run = spawn script \"child.nvs\" with(grants: ['fs.read'], output: 'capture');\n\
         var $done = await $run;\necho $done->output;\n"
    );
    let child = format!(
        "<?nvs\n{ASK}\necho 'child: ', Ask::url(), \"\\n\";\n\
         var $group = Core\\Task::all({{\n    inside: fn(): string => Ask::url(),\n}});\n\
         echo 'child task: ', $group->inside, \"\\n\";\n"
    );
    let config = "[capabilities.script]\nspawn = true\n\n[capabilities.fs]\nread = [\".\"]\n\n\
                  [capabilities.net]\nconnect = [\"127.0.0.1\"]\n";
    let (_root, app) = layout(&[
        ("main.nvs", main.as_str()),
        ("child.nvs", child.as_str()),
        ("nvs.toml", config),
    ]);
    let (stdout, stderr, ok) = run(&app);
    assert!(ok, "the program runs to its end:\n{stdout}\n{stderr}");
    assert_eq!(
        stdout, "parent task: allowed\nchild: refused\nchild task: refused\n",
        "a task of the script spawned with `grants: ['fs.read']` reached `net.connect`:\n{stderr}"
    );
}

// covers: Core\Task::all
#[test]
fn a_tasks_allocation_over_the_ceiling_is_refused_before_it_is_made() {
    // 400 MB against a 64 MB ceiling. Refused before it is made, the error names the
    // refused size. Made and caught at the next poll, it names no refusal, and the
    // request held all 400 MB first.
    let main = "<?nvs\nclass Grow\n{\n    public static function big(): uint\n    {\n        \
                string $text = Core\\Str::repeat('x', 400000000);\n        \
                return Core\\Str::length($text);\n    }\n}\n\necho \"before\\n\";\n\
                var $group = Core\\Task::all({\n    big: fn(): uint => Grow::big(),\n});\n\
                echo 'kept ', $group->big, \"\\n\";\n";
    let (_root, app) = layout(&[
        ("main.nvs", main),
        ("nvs.toml", "[limits]\nmemory = \"64M\"\n"),
    ]);
    let (stdout, stderr, ok) = run(&app);
    assert!(!ok, "a task over the ceiling stops the program:\n{stdout}");
    assert_eq!(
        stdout, "before\n",
        "the task's allocation came back with a string"
    );
    assert!(
        stderr.contains("when an allocation of 400000000 bytes was refused"),
        "the task's allocation was made before the ceiling stopped it:\n{stderr}"
    );
}
