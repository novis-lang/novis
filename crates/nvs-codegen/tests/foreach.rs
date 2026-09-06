//! `foreach` over an array: insertion order, nesting, `break`/`continue`, `return`, and `unset`.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

#[test]
fn a_foreach_walks_every_entry_in_insertion_order() {
    // The whole cursor: `array.next_slot`, `array.key_at`, `array.value_at`,
    // and the header phi carrying the cursor across the back edge. Insertion
    // order rather than key order is `rule:types/arrays`'s guarantee, which is why
    // the keys below are deliberately not alphabetical.
    let source = "<?nvs
array<int> $a = [\"gamma\" => 3, \"alpha\" => 1];
var $out = \"\";
foreach ($a as string $k => int $v) {
    $out = $out . $k . \"=\" . $v . \";\";
}
echo $out;
";
    assert_eq!(output_of(source), "gamma=3;alpha=1;");
}

#[test]
fn a_foreach_without_a_key_binding_still_walks_the_values() {
    let source = "<?nvs
array<int> $a = [4, 5, 6];
var $sum = 0;
foreach ($a as int $v) {
    $sum = $sum + $v;
}
echo $sum;
";
    assert_eq!(output_of(source), "15");
}

#[test]
fn a_write_inside_a_foreach_separates_and_leaves_the_walk_alone() {
    // PHP's by-value `foreach`, falling out of `rule:types/arrays`'s copy-on-write
    // rather than a snapshot: the loop's own retained reference puts the array
    // above refcount one, so the body's write separates and the cursor keeps
    // walking what the loop started on. Three iterations, not an unbounded
    // walk over an array the body keeps growing.
    let source = "<?nvs
array<int> $a = [1, 2, 3];
var $n = 0;
foreach ($a as int $v) {
    $n = $n + 1;
    $a[] = 9;
}
echo $n . \"/\" . $a[\"5\"];
";
    assert_eq!(output_of(source), "3/9");
}

#[test]
fn a_nested_foreach_walks_an_array_of_arrays() {
    // Two cursors live at once, each with its own reserved `Env` names, and
    // the inner loop's subject is the outer loop's value binding — a
    // refcounted binding the outer iteration owns and releases.
    let source = "<?nvs
array<array<int>> $grid = [[1, 2], [3, 4]];
var $total = 0;
foreach ($grid as array<int> $cells) {
    foreach ($cells as int $cell) {
        $total = $total + $cell;
    }
}
echo $total;
";
    assert_eq!(output_of(source), "10");
}

#[test]
fn a_break_and_a_continue_both_end_the_iteration_they_are_in() {
    // Both back-edge shapes plus the loop's exit edge, each of which owes the
    // key/value bindings a release — see `nvs_ir::lower::LoopFrame`'s
    // `iteration_owned`. A missing one leaks; a doubled one crashes here.
    let source = "<?nvs
array<string> $a = [\"a\" => \"one\", \"b\" => \"two\", \"c\" => \"three\"];
var $out = \"\";
var $n = 0;
foreach ($a as string $k => string $v) {
    $n = $n + 1;
    if ($n == 2) {
        continue;
    }
    if ($n == 3) {
        break;
    }
    $out = $out . $k . \":\" . $v . \";\";
}
echo $out . \"|\" . $n;
";
    assert_eq!(output_of(source), "a:one;|3");
}

#[test]
fn a_foreach_over_ten_thousand_string_values_leaks_nothing() {
    // The per-iteration key/value pair, ten thousand times. A missing release
    // leaks two buffers per iteration; a doubled one crashes. The leak half is
    // what the loop goal's valgrind leg measures — this holds the crash half
    // and the arithmetic.
    let source = "<?nvs
array<string> $a = [];
var $i = 0;
while ($i < 10000) {
    $a[$i] = \"n\" . $i;
    $i = $i + 1;
}
var $seen = 0;
foreach ($a as string $k => string $v) {
    $seen = $seen + 1;
}
echo $seen;
";
    assert_eq!(output_of(source), "10000");
}

#[test]
fn a_return_out_of_a_foreach_releases_the_array_it_was_walking() {
    // The loop's own retained reference is an ordinary `Env` member, so the
    // exit sweep `nvs_ir::lower::Lowering::release_all_locals` already runs
    // finds it — no `foreach`-specific cleanup on the return path.
    let source = "<?nvs
class Pick {
    public static function first(array<string> $a): string {
        foreach ($a as string $k => string $v) {
            return $v;
        }
        return \"none\";
    }
}
echo Pick::first([\"x\" => \"hit\"]) . Pick::first([]);
";
    assert_eq!(output_of(source), "hitnone");
}

#[test]
fn an_unset_removes_one_entry_and_leaves_the_order_of_the_rest() {
    // `array.unset` on the same consume-one-yield-one protocol a write uses,
    // written back through the same holder — and a tombstoned slot the cursor
    // then steps over.
    let source = "<?nvs
array<int> $a = [\"a\" => 1, \"b\" => 2, \"c\" => 3];
unset($a[\"b\"]);
var $out = \"\";
foreach ($a as string $k => int $v) {
    $out = $out . $k . $v;
}
echo $out;
";
    assert_eq!(output_of(source), "a1c3");
}

#[test]
fn an_unset_on_a_shared_array_separates_it() {
    // Removal separates exactly the way a write does, so the other binding
    // keeps every entry.
    let source = "<?nvs
array<int> $a = [\"k\" => 1, \"j\" => 2];
var $b = $a;
unset($b[\"k\"]);
echo $a[\"k\"] . \"/\" . $b[\"j\"];
";
    assert_eq!(output_of(source), "1/2");
}

#[test]
fn an_unset_by_an_int_subscript_normalizes_the_key_and_frees_it() {
    // `rule:types/arrays`'s key normalization on the `unset` path: the converted
    // decimal string is this frame's own temporary, released once the removal
    // has read it — the mirror of the retain a *stored* key gets.
    let source = "<?nvs
array<int> $a = [7, 8, 9];
unset($a[1]);
var $out = \"\";
foreach ($a as string $k => int $v) {
    $out = $out . $k . \"=\" . $v . \";\";
}
echo $out;
";
    assert_eq!(output_of(source), "0=7;2=9;");
}
