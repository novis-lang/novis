//! A `foreach (… as inout $v)` loop walks the array it began with, and a write
//! to `$v` goes to the subject by key
//! (`rule:iteration/an-inout-loop-walks-the-array-it-began-with`).
//!
//! Each program changes the subject while the loop runs: it removes entries,
//! assigns the variable, passes it `inout`, or empties it from a nested loop.
//! None of them may reach a panic in the runtime. A write to an entry the body
//! removed throws a `LogicError` the program catches.
//!
//! See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;

#[cfg(debug_assertions)]
use nvs_runtime::budget::{allocated_bytes, live_bytes};

/// Runs `source` to completion and returns what it echoed, or the status it
/// stopped with.
fn run(source: &str) -> Result<String, i32> {
    let mut ctx = Ctx::buffered();
    let status = run_with(&mut ctx, source);
    let output = ctx
        .take_buffered_output()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    status.map(|_| output)
}

/// The message the write throws, for the key `key`.
fn removed(key: &str) -> String {
    format!(
        "array key `{key}` was removed during the loop, so the `inout` value cannot be written to it"
    )
}

/// The body removes the next entry, then writes `$v` when the loop reaches
/// it. The write throws, the program catches it, and every string the run
/// built is freed.
#[test]
fn a_write_to_a_removed_entry_throws_a_logic_error() {
    let source = r#"<?nvs
int $n = 1;
array<string> $a = ["x" => "a" . $n, "y" => "b" . $n];
try {
    foreach ($a as inout string $v) {
        unset($a["y"]);
        $v = $v . "!";
    }
} catch (LogicError $e) {
    echo $e->message, "|";
}
echo Core\Json::encode($a);
"#;
    let want = format!("{}|{{\"x\":\"a1!\"}}", removed("y"));
    assert_eq!(run(source), Ok(want));

    #[cfg(debug_assertions)]
    {
        let unit = compile(source).expect("the fixture compiles");
        let mut ctx = Ctx::buffered();
        unit.install_in(&mut ctx);
        let entry = unit.script().expect("the script frame was compiled");
        nvs_runtime::prime_empty_array();
        let before = live_bytes();
        assert!(entry.call(&mut ctx).is_ok(), "the script ran to completion");
        drop(ctx.take_buffered_output());
        assert_eq!(
            live_bytes(),
            before,
            "a write that threw left a value or an array allocated"
        );
    }
}

/// Forty entries. Each visit removes the entry before it and writes `$v`.
/// The loop visits all forty, and the last entry is the one that remains.
#[test]
fn an_inout_loop_survives_removals_that_sweep_the_array() {
    let source = r#"<?nvs
array<int> $a = [];
for (int $i = 0; $i < 40; $i++) {
    $a["k" . $i] = $i;
}
int $visits = 0;
string $prev = "";
foreach ($a as string $k => inout int $v) {
    if ($prev != "") {
        unset($a[$prev]);
    }
    $v = $v * 2;
    $prev = $k;
    $visits++;
}
echo $visits, " ", Core\Json::encode($a);
"#;
    assert_eq!(run(source), Ok("40 {\"k39\":78}".to_owned()));
}

/// The subject is assigned in the body, passed `inout` to a function that
/// empties it, and emptied from a nested loop. Each run ends normally.
#[test]
fn an_inout_loop_over_a_replaced_array_never_panics() {
    let shapes = [
        (
            "assigned",
            r#"<?nvs
array<int> $a = [1, 2, 3];
foreach ($a as inout int $v) {
    $a = [10, 20, 30, 40];
    $v = $v + 1;
}
echo Core\Json::encode($a);
"#,
            "[10,20,4,40]".to_owned(),
        ),
        (
            "passed inout",
            r#"<?nvs
final class Shrink {
    public static function clear(inout array<int> $xs): void {
        $xs = [];
    }
}
array<int> $a = [1, 2, 3];
try {
    foreach ($a as inout int $v) {
        Shrink::clear(inout $a);
        $v = 7;
    }
} catch (LogicError $e) {
    echo $e->message, "|";
}
echo Core\Json::encode($a);
"#,
            format!("{}|[]", removed("0")),
        ),
        (
            "nested",
            r#"<?nvs
array<array<int>> $grid = [[1, 2], [3, 4]];
try {
    foreach ($grid as inout array<int> $row) {
        foreach ($row as inout int $cell) {
            $grid = [];
            $cell = 9;
        }
    }
} catch (LogicError $e) {
    echo $e->message, "|";
}
echo Core\Json::encode($grid);
"#,
            format!("{}|[]", removed("0")),
        ),
    ];
    let mut failures = Vec::new();
    for (shape, source, want) in shapes {
        match run(source) {
            Ok(output) if output == want => {}
            Ok(output) => failures.push(format!("{shape}: printed {output:?}, want {want:?}")),
            Err(code) => failures.push(format!("{shape}: stopped with status {code}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A loop that only writes `$v` writes each entry in place. What the loop
/// allocates does not grow with the array: a copy of a thousand entries
/// alone would take sixteen kilobytes.
#[cfg(debug_assertions)]
#[test]
fn an_inout_loop_that_only_writes_copies_nothing() {
    /// The bytes one run of the program allocates, compile excluded.
    fn spent(size: usize, with_loop: bool) -> usize {
        let body = if with_loop {
            "foreach ($a as inout int $v) {\n    $v = $v + 1;\n}\n"
        } else {
            ""
        };
        let source = format!(
            "<?nvs\narray<int> $a = [];\nfor (int $i = 0; $i < {size}; $i++) {{\n    $a[] = $i;\n}}\n\
             {body}echo $a[0];\n"
        );
        let unit = compile(&source).expect("the fixture compiles");
        let mut ctx = Ctx::buffered();
        unit.install_in(&mut ctx);
        let entry = unit.script().expect("the script frame was compiled");
        nvs_runtime::prime_empty_array();
        let before = allocated_bytes();
        assert!(entry.call(&mut ctx).is_ok(), "the script ran to completion");
        let spent = allocated_bytes() - before;
        let output = ctx.take_buffered_output().unwrap_or_default();
        let want: &[u8] = if with_loop { b"1" } else { b"0" };
        assert_eq!(output, want, "the loop did not write the first entry");
        spent
    }

    for size in [1000, 4000] {
        let by_loop = spent(size, true).saturating_sub(spent(size, false));
        assert!(
            by_loop < 1024,
            "a loop over {size} entries that only writes `$v` allocated {by_loop} bytes"
        );
    }
}
