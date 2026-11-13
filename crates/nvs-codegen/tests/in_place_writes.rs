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

#[cfg(debug_assertions)]
#[test]
fn a_property_append_allocates_in_proportion_to_what_it_adds() {
    // The piece is a method call, which runs before the property is read.
    assert_linear("a property append", |n| {
        format!(
            "<?nvs
class Page {{
    public string $out = \"\";
    public function piece(int $i): string {{ return \"<li>\" . $i . \"</li>\"; }}
    public function fill(int $n): void {{
        var $i = 0;
        while ($i < $n) {{
            $this->out .= $this->piece($i);
            $i = $i + 1;
        }}
    }}
}}
var $page = new Page();
$page->fill({n});
echo Core\\Str::length($page->out);
"
        )
    });
}

#[cfg(debug_assertions)]
#[test]
fn a_static_property_append_allocates_in_proportion_to_what_it_adds() {
    assert_linear("a static property append", |n| {
        format!(
            "<?nvs
class Log {{
    public static string $out = \"\";
    public static function piece(int $i): string {{ return \"line \" . $i . \"\\n\"; }}
}}
var $i = 0;
while ($i < {n}) {{
    Log::$out .= Log::piece($i);
    $i = $i + 1;
}}
echo Core\\Str::length(Log::$out);
"
        )
    });
}

#[cfg(debug_assertions)]
#[test]
fn an_element_append_allocates_in_proportion_to_what_it_adds() {
    // One guard per depth: a local's element, and a property's row.
    assert_linear("an element append", |n| {
        format!(
            "<?nvs
array<string> $parts = [\"a\" => \"\"];
var $i = 0;
while ($i < {n}) {{
    $parts[\"a\"] .= Core\\Str::repeat(\"x\", 8);
    $i = $i + 1;
}}
echo Core\\Str::length($parts[\"a\"]);
"
        )
    });
    assert_linear("a nested element append", |n| {
        format!(
            "<?nvs
class Page {{
    public array<array<string>> $rows;
    public function constructor() {{ $this->rows = [[\"html\" => \"\"]]; }}
    public function piece(int $i): string {{ return \"<p>\" . $i . \"</p>\"; }}
    public function fill(int $n): void {{
        var $i = 0;
        while ($i < $n) {{
            $this->rows[0][\"html\"] .= $this->piece($i);
            $i = $i + 1;
        }}
    }}
}}
var $page = new Page();
$page->fill({n});
echo Core\\Str::length($page->rows[0][\"html\"]);
"
        )
    });
}

/// How many allocation requests the **run** of `source` makes, with
/// compilation outside the window, as [`bytes_of_run`] measures bytes.
#[cfg(debug_assertions)]
fn requests_of_run(source: &str) -> usize {
    let unit = compile(source).expect("the fixture compiles");
    let mut ctx = Ctx::buffered();
    unit.install_in(&mut ctx);
    let entry = unit.script().expect("the script frame was compiled");
    let before = nvs_runtime::budget::allocations();
    entry.call(&mut ctx).expect("the script ran to completion");
    nvs_runtime::budget::allocations() - before
}

#[cfg(debug_assertions)]
#[test]
fn an_interpolated_append_allocates_only_as_the_target_grows() {
    // Each loop appends an interpolation and a `.` chain with `int` and
    // `uint` holes to a local, a property and a static property. Only the
    // targets' growth allocates, so 2,000 more steps make few more requests.
    let program = |n: u32| {
        format!(
            "<?nvs
class Page {{
    public string $out = \"\";
    public static string $log = \"\";
}}
var $page = new Page();
var $s = \"\";
uint $u = 7;
var $i = 0;
while ($i < {n}) {{
    $s .= \"<li>{{$i}}:{{$u}}</li>\";
    $s .= \"<p>\" . $i . \"/\" . $u . \"</p>\";
    $page->out .= \"<li>{{$i}}:{{$u}}</li>\";
    Page::$log .= \"line \" . $i . \"\\n\";
    $i = $i + 1;
}}
echo Core\\Str::length($s), \"/\", Core\\Str::length($page->out), \"/\", Core\\Str::length(Page::$log);
"
        )
    };
    let small = requests_of_run(&program(2_000));
    let large = requests_of_run(&program(4_000));
    assert!(
        large - small < 200,
        "2,000 steps made {small} allocation requests and 4,000 made {large}"
    );
}

#[test]
fn an_interpolated_append_that_reads_its_target_still_copies_it() {
    // A part is the target itself, in a local, a property and a static
    // property, and in an interpolation and a `.` chain.
    assert_eq!(
        output_of(
            "<?nvs
class Page {
    public string $out = \"\";
    public static string $log = \"\";
}
var $page = new Page();
var $s = \"\";
$s .= \"ab\";
$s .= \"<{$s}>\";
$s .= $s . \"!\";
$page->out .= \"cd\";
$page->out .= \"[{$page->out}]\";
Page::$log .= \"ef\";
Page::$log .= Page::$log . 1;
echo $s, \"/\", $page->out, \"/\", Page::$log;
"
        ),
        "ab<ab>ab<ab>!/cd[cd]/efef1"
    );
    // A second variable has the target, which keeps its bytes.
    assert_eq!(
        output_of(
            "<?nvs
class Page {
    public string $out = \"\";
}
var $page = new Page();
uint $u = 3;
var $s = \"\";
$s .= \"ab\";
$page->out .= \"cd\";
var $kept = $s;
var $held = $page->out;
$s .= \"{$u}-{$kept}\";
$page->out .= \"<{$u}>\";
echo $kept, \"/\", $s, \"/\", $held, \"/\", $page->out;
"
        ),
        "ab/ab3-ab/cd/cd<3>"
    );
}

#[test]
fn an_append_to_a_shared_string_still_copies_it() {
    // A second variable has the property's string.
    assert_eq!(
        output_of(
            "<?nvs
class Page {
    public string $out = \"\";
}
var $page = new Page();
$page->out .= \"ab\";
var $kept = $page->out;
$page->out .= \"cd\";
echo $kept, \"/\", $page->out;
"
        ),
        "ab/abcd"
    );
    // A second variable has the static property's string.
    assert_eq!(
        output_of(
            "<?nvs
class Log {
    public static string $out = \"\";
}
Log::$out .= \"ab\";
var $kept = Log::$out;
Log::$out .= \"cd\";
echo $kept, \"/\", Log::$out;
"
        ),
        "ab/abcd"
    );
    // A second variable has the element's string, and another has the row.
    assert_eq!(
        output_of(
            "<?nvs
array<array<string>> $g = [[\"s\" => \"\"]];
$g[0][\"s\"] .= \"ab\";
var $kept = $g[0][\"s\"];
var $row = $g[0];
$g[0][\"s\"] .= \"cd\";
echo $kept, \"/\", $row[\"s\"], \"/\", $g[0][\"s\"];
"
        ),
        "ab/ab/abcd"
    );
    // The suffix is the target itself.
    assert_eq!(
        output_of(
            "<?nvs
class Page {
    public string $out = \"ab\";
    public static string $log = \"cd\";
}
var $page = new Page();
$page->out .= $page->out;
Page::$log .= Page::$log;
array<string> $a = [\"k\" => \"ef\"];
$a[\"k\"] .= $a[\"k\"];
echo $page->out, \"/\", Page::$log, \"/\", $a[\"k\"];
"
        ),
        "abab/cdcd/efef"
    );
}

#[test]
fn an_append_to_an_absent_element_throws_what_the_read_throws() {
    let read = output_of(
        "<?nvs
array<array<string>> $g = [[\"s\" => \"\"]];
try {
    echo $g[0][\"t\"];
} catch (Throwable $e) {
    echo $e->message, \"/\";
}
try {
    echo $g[1][\"s\"];
} catch (Throwable $e) {
    echo $e->message, \"/\";
}
echo Core\\Arr::count($g), Core\\Arr::count($g[0]);
",
    );
    let append = output_of(
        "<?nvs
array<array<string>> $g = [[\"s\" => \"\"]];
try {
    $g[0][\"t\"] .= \"x\";
} catch (Throwable $e) {
    echo $e->message, \"/\";
}
try {
    $g[1][\"s\"] .= \"x\";
} catch (Throwable $e) {
    echo $e->message, \"/\";
}
echo Core\\Arr::count($g), Core\\Arr::count($g[0]);
",
    );
    assert_eq!(append, read);
    assert!(append.ends_with("/11"), "the append built a row: {append}");
}

#[test]
fn an_append_to_a_guarded_or_hooked_property_keeps_its_calls() {
    assert_eq!(
        output_of(
            "<?nvs
final class Page implements PropertyGuard, PropertyObserver {
    public string $out = \"a\";
    public array<string> $parts;
    public string $label {
        set(string $value) { echo \"hook/\"; $this->label = $value; }
    }
    public function constructor() {
        $this->parts = [\"k\" => \"a\"];
        $this->label = \"a\";
    }
    public function beforePropertySet(string $name, mixed $value, array<string> $keys): void {
        echo \"guard \", $name, \" \", Core\\Json::encode($keys), \"/\";
    }
    public function beforePropertyUnset(string $name, array<string> $keys): void {}
    public function onPropertyGet(string $name, mixed $value): void {}
    public function onPropertySet(string $name, mixed $value): void { echo \"set \", $name, \"/\"; }
}
var $page = new Page();
echo \"|\";
$page->out .= \"b\";
$page->parts[\"k\"] .= \"b\";
$page->label .= \"b\";
echo $page->out, $page->parts[\"k\"], $page->label;
"
        )
        .split_once('|')
        .map(|(_, after)| after.to_owned()),
        Some(
            "guard out []/set out/guard parts [\"k\"]/set parts/guard label []/hook/set label/ababab"
                .to_owned()
        )
    );
}

/// A class whose `add` appends to the array it is given and returns it.
const ACC: &str = "class Acc {
    public static function add(array<int> $xs, int $i): array<int> {
        $xs[] = $i;
        return $xs;
    }
    public static function boom(array<int> $xs): array<int> {
        throw new LogicError(\"boom\");
    }
    public static function fail(): int {
        throw new LogicError(\"fail\");
    }
}
";

#[cfg(debug_assertions)]
#[test]
fn an_array_passed_and_reassigned_allocates_in_proportion_to_what_it_adds() {
    // `$xs` is passed at its last use, so `add` has the only reference and
    // appends in place.
    assert_linear("an array passed and reassigned", |n| {
        format!(
            "<?nvs
{ACC}
array<int> $xs = [];
var $i = 0;
while ($i < {n}) {{
    $xs = Acc::add($xs, $i);
    $i = $i + 1;
}}
echo $xs[{last}];
",
            last = n - 1
        )
    });
}

#[cfg(debug_assertions)]
#[test]
fn a_string_passed_and_returned_allocates_in_proportion_to_what_it_adds() {
    // Each call appends to the accumulator and returns the next call's result.
    assert_linear("a string passed and returned", |n| {
        format!(
            "<?nvs
class Walk {{
    public static function walk(string $acc, int $n): string {{
        if ($n == 0) {{
            return $acc;
        }}
        $acc .= \"x\";
        return Walk::walk($acc, $n - 1);
    }}
}}
echo Core\\Str::length(Walk::walk(\"\", {n}));
"
        )
    });
}

#[test]
fn a_passed_array_another_variable_shares_is_still_copied() {
    assert_eq!(
        output_of(&format!(
            "<?nvs
{ACC}
array<int> $xs = [1, 2];
var $ys = $xs;
$xs = Acc::add($xs, 3);
echo Core\\Arr::count($ys), \"/\", Core\\Arr::count($xs), \"/\", $ys[1];
"
        )),
        "2/3/2"
    );
}

#[cfg(debug_assertions)]
#[test]
fn a_moved_argument_is_released_once_on_every_way_out() {
    // The callee throws, a later argument throws, and the call returns. Each
    // function moves `$xs` into its call, and the script catches the throw.
    let source = format!(
        "<?nvs
{ACC}
class Run {{
    public static function calleeThrows(): void {{
        array<int> $xs = [1, 2];
        $xs = Acc::boom($xs);
    }}
    public static function laterThrows(): void {{
        array<int> $xs = [1, 2];
        $xs = Acc::add($xs, Acc::fail());
    }}
    public static function returns(): uint {{
        array<int> $xs = [1, 2];
        $xs = Acc::add($xs, 3);
        return Core\\Arr::count($xs);
    }}
}}
try {{
    Run::calleeThrows();
}} catch (LogicError $e) {{
    echo \"caught/\";
}}
try {{
    Run::laterThrows();
}} catch (LogicError $e) {{
    echo \"caught/\";
}}
echo Run::returns();
"
    );
    nvs_runtime::prime_empty_array();
    let unit = compile(&source).expect("the fixture compiles");
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
        "caught/caught/3"
    );
    assert_eq!(after, before, "the run left bytes allocated");
}
