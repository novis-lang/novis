//! `array<T>` literals, element writes, copy-on-write separation, and truthiness.
//!
//! Split out of the single `compile_and_run.rs`; every test keeps its own name
//! and body. See `tests/common/mod.rs` for the shared fixtures and for why
//! these go through the real pipeline.

mod common;

use common::*;

#[test]
fn an_array_literal_reads_back_the_element_it_stored() {
    // The whole array path end to end: `mwl_array_new`, one `mwl_array_set`
    // per literal entry, and `mwl_array_get` reading one back.
    assert_eq!(
        output_of("<?mwl\narray<int> $a = [10, 20, 30];\necho $a[1];\n"),
        "20"
    );
    assert_eq!(
        output_of("<?mwl\narray<string> $a = [\"k\" => \"v\"];\necho $a[\"k\"];\n"),
        "v"
    );
}

#[test]
fn a_written_element_is_visible_through_the_same_local() {
    // The write-back `mwl_ir::lower::write_back_array` emits: the local is
    // re-pointed at whatever `mwl_array_set` yielded, so the read that follows
    // sees the entry. Without it the read would still name the pre-write
    // array.
    assert_eq!(
        output_of("<?mwl\narray<int> $a = [];\n$a[\"k\"] = 7;\necho $a[\"k\"];\n"),
        "7"
    );
    assert_eq!(
        output_of(
            "<?mwl\narray<int> $a = [];\n$a[] = 4;\n$a[] = 5;\necho $a[\"0\"] . $a[\"1\"];\n"
        ),
        "45"
    );
    assert_eq!(
        output_of("<?mwl\narray<int> $a = [1, 2];\n$a[0] = 9;\necho $a[0];\n"),
        "9"
    );
}

#[test]
fn a_copy_written_after_aliasing_leaves_the_original_alone() {
    // ADR 0007 § 5's copy-on-write value semantics, which is the whole reason
    // an array write yields the array it wrote into.
    assert_eq!(
        output_of(
            "<?mwl\narray<int> $a = [\"k\" => 1];\nvar $b = $a;\n$b[\"k\"] = 99;\necho $a[\"k\"] . \"/\" . $b[\"k\"];\n"
        ),
        "1/99"
    );
}

#[test]
fn an_array_is_truthy_unless_it_is_empty() {
    // `Helper::ArrayTruthy`, whose entry point landed with the representation.
    assert_eq!(
        output_of("<?mwl\narray<int> $a = [];\nif ($a) {\n    echo \"full\";\n}\necho \"done\";\n"),
        "done"
    );
    assert_eq!(
        output_of("<?mwl\narray<int> $a = [1];\nif ($a) {\n    echo \"full\";\n}\n"),
        "full"
    );
}

#[test]
fn an_array_of_strings_rewritten_in_a_loop_leaks_nothing() {
    // Ten thousand writes into one solely-owned array: each one replaces a
    // stored string, which the table releases as it displaces it. A missing
    // release would leak the buffers; a doubled one would crash. This is also
    // the in-place fast path running ten thousand times without separating.
    let source = "<?mwl
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
    let source = "<?mwl
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
