//! `Core\\Arr` end to end — a callback-bound result type, `values`, and `sort`'s whole options bag.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

/// `Core\Arr::map` end to end, and with it the `U` binding under it: the
/// result is `array<string>` because the callback's return type is, which is
/// what lets `Core\Str::join` — declared `array<string>` — take it at all.
/// `nvs_types::generics` owns that rule.
#[test]
fn a_core_member_takes_its_result_element_type_from_its_callback() {
    let source = "<?nvs
array<string> $words = [\"pear\", \"Apple\", \"fig\"];
var $upper = Core\\Arr::map($words, fn(string $w) => Core\\Str::upper($w));
echo Core\\Str::join($upper, \"|\");
";
    assert_eq!(output_of(source), "PEAR|APPLE|FIG");
}

/// Keys survive a `map`, which is PHP's own single-array `array_map`
/// behaviour and the one `Core\Arr::filter` already keeps — re-keying is
/// `mapKeys`, its own member. The callback's second parameter is the key, so
/// this also holds that `map` offers both arguments the way `filter` does.
#[test]
fn a_mapped_array_keeps_the_keys_it_was_built_with() {
    let source = "<?nvs
array<int> $a = [\"x\" => 1, \"y\" => 2];
var $labelled = Core\\Arr::map($a, fn(int $v, string $k): string => $k . ($v * 2));
echo Core\\Str::join($labelled, \";\");
if (Core\\Arr::hasKey($labelled, \"x\")) { echo \"|kept\"; } else { echo \"|lost\"; }
";
    assert_eq!(output_of(source), "x2;y4|kept");
}

/// The mapped value is the *callback's* fresh reference, stored without a
/// retain — one retain too many here is a leak per entry, and one too few a
/// double free. Ten thousand iterations over string results is what makes
/// either loud rather than theoretical.
#[test]
fn a_map_producing_strings_in_a_loop_leaks_nothing() {
    let source = "<?nvs
array<int> $nums = [1, 2, 3];
var $i = 0;
var $seen = 0;
while ($i < 10000) {
    var $tags = Core\\Arr::map($nums, fn(int $n): string => \"t\" . $n);
    $seen = $seen + Core\\Arr::count($tags) as int;
    $i = $i + 1;
}
echo $seen;
";
    assert_eq!(output_of(source), "30000");
}

/// `Core\Arr::values` renumbers from zero, and `Core\Arr::isList` reads the
/// answer back — the pair together, so a wrong key on either side shows up as
/// a `no` rather than as a passing test on each half.
#[test]
fn values_renumbers_an_associative_array_into_a_list() {
    let source = "<?nvs
array<string> $a = [\"x\" => \"a\", \"y\" => \"b\"];
if (Core\\Arr::isList($a)) { echo \"yes\"; } else { echo \"no\"; }
var $v = Core\\Arr::values($a);
if (Core\\Arr::isList($v)) { echo \"|yes\"; } else { echo \"|no\"; }
echo \"|\", Core\\Str::join($v, \",\");
";
    assert_eq!(output_of(source), "no|yes|a,b");
}

/// An `unset` leaves a hole rather than renumbering (`rule:types/arrays`), so the
/// array stops being a list until `values` rebuilds it — PHP's own answer, and
/// the reason `isList` compares key bytes rather than counting entries.
#[test]
fn a_hole_left_by_an_unset_stops_an_array_being_a_list() {
    let source = "<?nvs
array<int> $a = [10, 20, 30];
if (Core\\Arr::isList($a)) { echo \"yes\"; } else { echo \"no\"; }
unset($a[0]);
if (Core\\Arr::isList($a)) { echo \"|yes\"; } else { echo \"|no\"; }
if (Core\\Arr::isList(Core\\Arr::values($a))) { echo \"|yes\"; } else { echo \"|no\"; }
echo \"|\", Core\\Arr::count($a);
";
    assert_eq!(output_of(source), "yes|no|yes|2");
}

/// `values` copies entries out of an array that outlives the call, so every
/// one needs a reference of its own — ten thousand rounds is what makes a
/// missing retain a crash and a spare one a leak, rather than either staying
/// invisible.
#[test]
fn taking_values_in_a_loop_leaks_nothing() {
    let source = "<?nvs
var $i = 0;
var $seen = 0;
while ($i < 10000) {
    array<string> $a = [\"x\" => \"t\" . $i, \"y\" => \"u\" . $i];
    var $v = Core\\Arr::values($a);
    $seen = $seen + Core\\Arr::count($v) as int;
    $i = $i + 1;
}
echo $seen;
";
    assert_eq!(output_of(source), "20000");
}

/// `Core\Arr::sort` with nothing written: ascending, renumbered from zero.
/// PHP 8.5's own `sort(["pear", "Apple", "fig", "banana"])` answers
/// `Apple,banana,fig,pear`.
#[test]
fn sorting_with_no_options_is_ascending_and_renumbers() {
    let source = "<?nvs
array<string> $words = [\"pear\", \"Apple\", \"fig\", \"banana\"];
echo Core\\Str::join(Core\\Arr::sort($words), \",\");
";
    assert_eq!(output_of(source), "Apple,banana,fig,pear");
}

/// `{order: Core\Order::Desc}` — the first `Core`-owned enum reaching a
/// running program, and PHP's `rsort` in one option rather than a second
/// member name.
#[test]
fn a_core_enum_case_selects_the_descending_order() {
    let source = "<?nvs
array<string> $words = [\"pear\", \"Apple\", \"fig\", \"banana\"];
echo Core\\Str::join(Core\\Arr::sort($words, {order: Core\\Order::Desc}), \",\");
";
    assert_eq!(output_of(source), "pear,fig,banana,Apple");
}

/// `{by: ...}` decides *what* is compared. Sorting by the lower-cased word
/// puts `apple` first, where the bytewise order the previous test checks puts
/// `Zebra` first — so this fails if the extractor is ignored rather than
/// merely mis-ordered.
#[test]
fn a_by_extractor_decides_what_is_compared() {
    let source = "<?nvs
array<string> $words = [\"Zebra\", \"apple\"];
echo Core\\Str::join(Core\\Arr::sort($words), \",\");
echo \"|\";
echo Core\\Str::join(Core\\Arr::sort($words, {by: fn(string $w): string => Core\\Str::lower($w)}), \",\");
";
    assert_eq!(output_of(source), "Zebra,apple|apple,Zebra");
}

/// `{comparator: ...}` is `usort`'s callback, unchanged — PHP's
/// `usort($nums, fn($a, $b) => $b - $a)` answers `9|5|3|1` for the same input.
#[test]
fn a_comparator_replaces_the_natural_ordering() {
    let source = "<?nvs
array<int> $nums = [5, 3, 9, 1];
var $down = Core\\Arr::sort($nums, {comparator: fn(int $a, int $b): int => $b - $a});
echo Core\\Str::join(Core\\Arr::map($down, fn(int $n): string => $n as string), \"|\");
";
    assert_eq!(output_of(source), "9|5|3|1");
}

/// `{preserveKeys: true}` is PHP's `asort`, and with `Core\Order::Desc` its
/// `arsort` — the eight extra names that half of PHP's roster spends on this
/// one option.
#[test]
fn preserve_keys_keeps_each_entrys_own_key() {
    let source = "<?nvs
class Show {
    public static function render(array<int> $a): string {
        var $out = \"\";
        foreach ($a as string $k => int $v) {
            $out = $out . $k . \"=\" . $v . \";\";
        }
        return $out;
    }
}

array<int> $scores = [\"c\" => 3, \"a\" => 1, \"b\" => 2];
echo Show::render(Core\\Arr::sort($scores, {preserveKeys: true}));
echo \"|\";
echo Show::render(Core\\Arr::sort($scores, {preserveKeys: true, order: Core\\Order::Desc}));
echo \"|\";
echo Show::render(Core\\Arr::sort($scores));
";
    assert_eq!(output_of(source), "a=1;b=2;c=3;|c=3;b=2;a=1;|0=1;1=2;2=3;");
}

/// A comparator that throws stops the sort and propagates, rather than
/// finishing on a half-ordered array — and the `catch` runs, which is what
/// makes the hand-written merge's early return a real path rather than a
/// claim.
#[test]
fn a_comparator_that_throws_propagates_out_of_the_sort() {
    let source = "<?nvs
class Boom {
    public static function at(int $a, int $b): int {
        if ($a > 3) { throw new RuntimeError(\"nope\"); }
        return $a - $b;
    }
}

array<int> $nums = [5, 3, 9, 1];
var $cmp = fn(int $a, int $b): int => Boom::at($a, $b);
try {
    var $sorted = Core\\Arr::sort($nums, {comparator: $cmp});
    echo \"unreachable\";
} catch (RuntimeError $e) {
    echo \"caught: \", $e->message;
}
";
    assert_eq!(output_of(source), "caught: nope");
}

/// Sorting an array of heap values ten thousand times: the entries copied into
/// the result each need a reference of their own, and every `by`-extracted key
/// is one this frame owes a release for. A missing retain crashes here and a
/// spare one leaks — neither is visible from a single call.
#[test]
fn sorting_in_a_loop_leaks_nothing() {
    let source = "<?nvs
var $i = 0;
var $seen = 0;
var $by = fn(string $s): string => Core\\Str::upper($s);
while ($i < 10000) {
    array<string> $a = [\"b\" . $i, \"a\" . $i, \"c\" . $i];
    var $sorted = Core\\Arr::sort($a, {by: $by});
    $seen = $seen + Core\\Arr::count($sorted) as int;
    $i = $i + 1;
}
echo $seen;
";
    assert_eq!(output_of(source), "30000");
}
