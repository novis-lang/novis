//! The debug lend check, driven through the built binary
//! (`rule:types/a-replaced-value-is-changed-in-place`).
//!
//! The ownership pass lends a property to an owning call only where nothing
//! between the slot's read and its store can read the slot. A debug build
//! poisons the lent slot in that window, and a compiled read that finds the
//! poison stops the request naming its statement. `NOVIS_LEND_IGNORE_EFFECTS`
//! makes a debug build lend with no regard to effects, so this test can force
//! the wrong lend the check exists to catch.

use std::process::{Command, Output, Stdio};

/// `T::count` reads `$this->p` through its argument, between the slot's read
/// and the call that is lent it. Line 6 is that read.
const PROGRAM: &str = "<?nvs
final class T {
    public array<int> $p;
    public function constructor() { $this->p = [1, 2, 3]; }
    public static function count(T $t): int {
        return Core\\Arr::count($t->p) as int;
    }
    public function cut(): void {
        $this->p = Core\\Arr::slice($this->p, T::count($this) - 1);
    }
}
T $t = new T();
$t->cut();
echo Core\\Arr::count($t->p) as int, \"\\n\";
";

/// Runs `nvs run main.nvs` over [`PROGRAM`], with the effects ignored or not.
fn run(ignore_effects: bool) -> Output {
    let dir = nvs_repo::scratch("in-place-lend-trap");
    std::fs::write(dir.join("main.nvs"), PROGRAM).expect("the program is written");
    let data = nvs_repo::scratch_private("nvsdata");
    let mut command = Command::new(env!("CARGO_BIN_EXE_nvs"));
    command
        .arg("--data")
        .arg(&*data)
        .args(["run", "main.nvs"])
        .current_dir(&*dir)
        .stdin(Stdio::null())
        .env_remove("NOVIS_LEND_IGNORE_EFFECTS");
    if ignore_effects {
        command.env("NOVIS_LEND_IGNORE_EFFECTS", "1");
    }
    command
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// The pass refuses the lend on its own, so the program prints `1`. Forced
/// to lend, the read in `T::count` finds the poison, and the request stops
/// with a fatal that names `main.nvs:6`. A release build has no check.
#[test]
fn a_debug_build_traps_a_read_of_a_lent_slot() {
    let fair = run(false);
    let printed = String::from_utf8_lossy(&fair.stdout);
    assert!(
        fair.status.success() && printed.trim() == "1",
        "the pass keeps the retain and the program runs: {printed}{}",
        String::from_utf8_lossy(&fair.stderr)
    );
    if !cfg!(debug_assertions) {
        return;
    }
    let forced = run(true);
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&forced.stdout),
        String::from_utf8_lossy(&forced.stderr)
    );
    assert!(
        !forced.status.success(),
        "the wrong lend is trapped: {said}"
    );
    assert!(
        said.contains("a property was read at main.nvs:6 while a call had its value"),
        "the fatal names the reading statement: {said}"
    );
}
