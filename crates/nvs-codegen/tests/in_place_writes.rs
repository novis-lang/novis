//! Writes that change part of a value cost what they add, not what the value
//! already has (`rule:types/arrays`).
//!
//! The growth guards run one program at two sizes and compare the bytes each
//! run allocates. A write that copies what it writes into allocates in
//! proportion to the square of the size, so the larger run costs four times
//! the smaller. A write in place costs about twice. Each guard fails at three
//! times. No guard reads a clock (`rule:testing/bench-counters`).
//!
//! Beside each guard is a test in which another holder shares the value, and
//! the write still copies it. See `tests/common/mod.rs` for the shared
//! fixtures.

mod common;

use common::*;

/// The bytes the **run** of `source` allocates. Compilation happens outside
/// the window, since what is under test is what the compiled code spends.
///
/// **Only run in a debug build**, for `allocations_of_run`'s reason in
/// `arrays.rs`.
#[cfg(debug_assertions)]
fn bytes_of_run(source: &str) -> usize {
    let unit = compile(source).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    let before = nvs_runtime::budget::allocated_bytes();
    entry.call(&mut ctx).expect("the script ran to completion");
    nvs_runtime::budget::allocated_bytes() - before
}

/// Fails unless `program` at 4,000 iterations allocates less than three times
/// what it allocates at 2,000.
#[cfg(debug_assertions)]
fn assert_linear(name: &str, program: impl Fn(u32) -> String) {
    let small = bytes_of_run(&program(2_000));
    let large = bytes_of_run(&program(4_000));
    assert!(
        large < small * 3,
        "{name}: 2,000 iterations allocated {small} bytes and 4,000 allocated {large}, \
         which is faster than linear growth"
    );
}

#[cfg(debug_assertions)]
#[test]
fn a_nested_append_allocates_in_proportion_to_what_it_adds() {
    assert_linear("a nested append", |n| {
        format!(
            "<?nvs
array<array<int>> $g = [[]];
var $i = 0;
while ($i < {n}) {{
    $g[0][] = $i;
    $i = $i + 1;
}}
echo $g[0][{last}];
",
            last = n - 1
        )
    });
}

#[cfg(debug_assertions)]
#[test]
fn a_nested_set_allocates_in_proportion_to_what_it_writes() {
    // The row is absent before the first write, so the write builds it.
    assert_linear("a nested set", |n| {
        format!(
            "<?nvs
array<array<int>> $g = [];
var $i = 0;
while ($i < {n}) {{
    $g[\"row\"][$i] = $i;
    $i = $i + 1;
}}
echo $g[\"row\"][{last}];
",
            last = n - 1
        )
    });
}

#[cfg(debug_assertions)]
#[test]
fn a_nested_write_through_a_property_allocates_in_proportion_to_what_it_adds() {
    // The value is a method call, which runs before the property is read.
    assert_linear("a nested write through a property", |n| {
        format!(
            "<?nvs
class Grid {{
    public array<array<int>> $rows;
    public function constructor() {{ $this->rows = []; }}
    public function twice(int $i): int {{ return $i * 2; }}
    public function fill(int $n): void {{
        var $i = 0;
        while ($i < $n) {{
            $this->rows[\"a\"][] = $this->twice($i);
            $i = $i + 1;
        }}
    }}
}}
var $grid = new Grid();
$grid->fill({n});
echo $grid->rows[\"a\"][{last}];
",
            last = n - 1
        )
    });
}

#[test]
fn a_nested_write_into_a_shared_row_still_copies_it() {
    // A second variable has the row.
    assert_eq!(
        output_of(
            "<?nvs
array<array<int>> $g = [[1, 2]];
var $row = $g[0];
$g[0][0] = 9;
echo $row[0], \"/\", $g[0][0];
"
        ),
        "1/9"
    );
    // A `foreach` walks the row while the loop writes it.
    assert_eq!(
        output_of(
            "<?nvs
array<array<int>> $g = [[1, 2]];
foreach ($g[0] as var $v) {
    $g[0][1] = 7;
    echo $v;
}
echo \"/\", $g[0][1];
"
        ),
        "12/7"
    );
    // A closure captured the row.
    assert_eq!(
        output_of(
            "<?nvs
array<array<int>> $g = [[1, 2]];
var $row = $g[0];
var $f = fn() => $row[0];
$row = [];
$g[0][0] = 9;
echo $f(), \"/\", $g[0][0];
"
        ),
        "1/9"
    );
}

#[test]
fn a_nested_write_into_a_shared_array_still_copies_it() {
    // A second variable has the whole array.
    assert_eq!(
        output_of(
            "<?nvs
array<array<int>> $g = [[1, 2]];
var $h = $g;
$g[0][0] = 9;
$g[1][] = 5;
echo $h[0][0], \"/\", Core\\Arr::count($h), \"/\", $g[0][0], \"/\", $g[1][0];
"
        ),
        "1/1/9/5"
    );
    // A `foreach` walks the array while the loop writes into its rows.
    assert_eq!(
        output_of(
            "<?nvs
array<array<int>> $g = [[1], [2]];
foreach ($g as var $row) {
    $g[1][0] = 8;
    echo $row[0];
}
echo \"/\", $g[1][0];
"
        ),
        "12/8"
    );
    // A closure captured the array.
    assert_eq!(
        output_of(
            "<?nvs
array<array<int>> $g = [[1, 2]];
var $f = fn() => $g[0][0];
$g[0][0] = 9;
echo $f(), \"/\", $g[0][0];
"
        ),
        "1/9"
    );
}

#[cfg(debug_assertions)]
#[test]
fn a_nested_append_that_throws_leaves_the_array_as_it_was() {
    // The row's next index is the largest `int`, which is already a key, so
    // the append throws. Nothing was written, and nothing is left allocated.
    let source = "<?nvs
array<array<int>> $g = [[9223372036854775807 => 1], [2]];
var $h = $g;
try {
    $g[0][] = 5;
} catch (LogicError $e) {
    echo \"caught/\";
}
try {
    $h[0][] = 6;
} catch (LogicError $e) {
    echo \"caught/\";
}
echo Core\\Arr::count($g), Core\\Arr::count($g[0]), $g[0][9223372036854775807], $g[1][0];
";
    nvs_runtime::prime_empty_array();
    let unit = compile(source).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    let before = nvs_runtime::budget::live_bytes();
    entry.call(&mut ctx).expect("the script ran to completion");
    let output = ctx.take_buffered_output().expect("a buffered context");
    // The echoed bytes are the one thing the run leaves behind.
    let after = nvs_runtime::budget::live_bytes()
        - isize::try_from(output.capacity()).expect("a small buffer");
    assert_eq!(
        String::from_utf8(output).expect("the script echoed UTF-8"),
        "caught/caught/2112"
    );
    assert_eq!(after, before, "the run left bytes allocated");
}
