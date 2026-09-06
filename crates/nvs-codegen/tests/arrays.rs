//! `array<T>` literals, element writes, copy-on-write separation, and truthiness.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;

#[test]
fn an_array_literal_reads_back_the_element_it_stored() {
    // The whole array path end to end: `nvs_array_new`, one `nvs_array_set`
    // per literal entry, and `nvs_array_get` reading one back.
    assert_eq!(
        output_of("<?nvs\narray<int> $a = [10, 20, 30];\necho $a[1];\n"),
        "20"
    );
    assert_eq!(
        output_of("<?nvs\narray<string> $a = [\"k\" => \"v\"];\necho $a[\"k\"];\n"),
        "v"
    );
}

#[test]
fn a_written_element_is_visible_through_the_same_local() {
    // The write-back `nvs_ir::lower::write_back_array` emits: the local is
    // re-pointed at whatever `nvs_array_set` yielded, so the read that follows
    // sees the entry. Without it the read would still name the pre-write
    // array.
    assert_eq!(
        output_of("<?nvs\narray<int> $a = [];\n$a[\"k\"] = 7;\necho $a[\"k\"];\n"),
        "7"
    );
    assert_eq!(
        output_of(
            "<?nvs\narray<int> $a = [];\n$a[] = 4;\n$a[] = 5;\necho $a[\"0\"] . $a[\"1\"];\n"
        ),
        "45"
    );
    assert_eq!(
        output_of("<?nvs\narray<int> $a = [1, 2];\n$a[0] = 9;\necho $a[0];\n"),
        "9"
    );
}

#[test]
fn a_copy_written_after_aliasing_leaves_the_original_alone() {
    // `rule:types/arrays`'s copy-on-write value semantics, which is the whole reason
    // an array write yields the array it wrote into.
    assert_eq!(
        output_of(
            "<?nvs\narray<int> $a = [\"k\" => 1];\nvar $b = $a;\n$b[\"k\"] = 99;\necho $a[\"k\"] . \"/\" . $b[\"k\"];\n"
        ),
        "1/99"
    );
}

#[test]
fn an_array_is_truthy_unless_it_is_empty() {
    // `Helper::ArrayTruthy`, which is where an array's truthiness is decided.
    assert_eq!(
        output_of("<?nvs\narray<int> $a = [];\nif ($a) {\n    echo \"full\";\n}\necho \"done\";\n"),
        "done"
    );
    assert_eq!(
        output_of("<?nvs\narray<int> $a = [1];\nif ($a) {\n    echo \"full\";\n}\n"),
        "full"
    );
}

#[test]
fn an_array_of_strings_rewritten_in_a_loop_leaks_nothing() {
    // Ten thousand writes into one solely-owned array: each one replaces a
    // stored string, which the table releases as it displaces it. A missing
    // release would leak the buffers; a doubled one would crash. This is also
    // the in-place fast path running ten thousand times without separating.
    let source = "<?nvs
array<string> $a = [];
var $i = 0;
while ($i < 10000) {
    $a[\"k\"] = \"n\" . $i;
    $i = $i + 1;
}
echo $a[\"k\"];
";
    assert_eq!(output_of(source), "n9999");
}

#[test]
fn an_array_element_written_through_a_property_survives_the_write_back() {
    // The other holder a separation is written back to: the property slot,
    // through a `field.set` of whatever the write yielded.
    let source = "<?nvs
class Bag {
    public array<int> $items;
    public function constructor() { $this->items = []; }
    public function put(string $k, int $v): void { $this->items[$k] = $v; }
    public function get(string $k): int { return $this->items[$k]; }
}

var $bag = new Bag();
$bag->put(\"a\", 1);
$bag->put(\"b\", 2);
echo $bag->get(\"a\") . $bag->get(\"b\");
";
    assert_eq!(output_of(source), "12");
}

// ============================================================================
// The packed form, reached from compiled code
// ============================================================================

/// An allocator that counts the requests made on the calling thread, so the
/// guard below measures what compiled code spends rather than reading the
/// lowering and believing it.
///
/// Compiles `source`, runs it, and answers how many allocation requests the
/// **run** made — compilation happens outside the window on purpose, since
/// what is under test is what the compiled code spends.
///
/// The counter is `nvs_runtime::budget`'s, which every build maintains because
/// the memory limit is read off it; this binary installs no allocator of its
/// own, and could not, since a `#[global_allocator]` is chosen once per binary
/// and `nvs-runtime` registers one in every `not(test)` build.
///
/// **Only run in a debug build.** What the gate keeps out is an optimized
/// build's inlining: the numbers below are pinned against what an unoptimized
/// build allocates, and nothing has measured them under `--release`.
#[cfg(debug_assertions)]
fn allocations_of_run(source: &str) -> usize {
    let unit = compile(source).expect("the fixture compiles");
    let entry = unit.script().expect("the script frame was compiled");
    let mut ctx = Ctx::buffered();
    let before = nvs_runtime::budget::allocations();
    entry.call(&mut ctx).expect("the script ran to completion");
    nvs_runtime::budget::allocations() - before
}

/// `rounds` passes over a four-element list, every element read and then
/// written through `$i` — an `int` local, which is what puts the access on
/// `nvs_array_get_index`/`nvs_array_set_index`.
#[cfg(debug_assertions)]
fn integer_subscripts(rounds: i64) -> String {
    format!(
        "<?nvs
array<int> $a = [0, 0, 0, 0];
var $r = 0;
var $i = 0;
while ($r < {rounds}) {{
    $i = 0;
    while ($i < 4) {{
        $a[$i] = $a[$i] + 1;
        $i = $i + 1;
    }}
    $r = $r + 1;
}}
"
    )
}

/// The same accesses, reached through a rendered decimal key rather than the
/// integer subscript. The control arm: it is what makes the integer form's
/// zero a measurement rather than a counter that never moved.
#[cfg(debug_assertions)]
fn rendered_subscripts(rounds: i64) -> String {
    format!(
        "<?nvs
array<int> $a = [0, 0, 0, 0];
var $r = 0;
var $i = 0;
var $k = \"\";
while ($r < {rounds}) {{
    $i = 0;
    while ($i < 4) {{
        $k = $i as string;
        $a[$k] = $a[$k] + 1;
        $i = $i + 1;
    }}
    $r = $r + 1;
}}
"
    )
}

#[cfg(debug_assertions)]
#[test]
fn an_integer_subscript_reaches_the_packed_form_from_compiled_code() {
    // `nvs_runtime::array`'s own `an_integer_subscript_allocates_no_key` holds
    // this for the primitive; what is held here is the path *compiled code*
    // takes to it. The measurement is a difference between two run lengths
    // rather than an absolute zero, because a run also allocates its array and
    // its locals once: a key rendered per access is O(accesses), so it shows up
    // in the delta and a fixed setup cost cancels out of it.
    //
    // A zero delta also pins the packed form itself, not only the ABI:
    // `nvs_array_get_index`/`nvs_array_set_index` synthesize a key whenever the
    // array is already `Hashed` (`nvs_ir::lower::expr::lower_array_key`'s doc
    // comment), so an array that degraded would allocate per access too.
    const FEW: i64 = 4;
    const MANY: i64 = 404;

    // What the accesses answer, first — and that `rule:types/arrays` is intact: the
    // key still *is* a `string`, so both spellings read the same element.
    assert_eq!(
        output_of(&format!(
            "{}echo $a[0] . \"/\" . $a[\"3\"];\n",
            integer_subscripts(3)
        )),
        "3/3"
    );

    let few = allocations_of_run(&integer_subscripts(FEW));
    let many = allocations_of_run(&integer_subscripts(MANY));
    assert_eq!(
        many, few,
        "{MANY} passes allocated {many} and {FEW} allocated {few}: an integer subscript from \
         compiled code rendered a key, or the array did not stay packed"
    );

    let few_rendered = allocations_of_run(&rendered_subscripts(FEW));
    let many_rendered = allocations_of_run(&rendered_subscripts(MANY));
    assert!(
        many_rendered > few_rendered,
        "the rendered-key spelling is supposed to build a key per access, so the counter is not \
         measuring what the arm above claims it measured"
    );
}
