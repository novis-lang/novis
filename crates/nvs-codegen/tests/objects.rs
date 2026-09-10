//! Object layout and lifetime — constructors, inherited slots, `instanceof`, and release on every exit path.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Objects — M4's representation, end to end
// ---------------------------------------------------------------------------
/// A two-property class with a constructor and two readers, plus a subclass
/// that inherits both — the shape every object test below builds on.
const SHAPES: &str = "<?nvs
class Animal {
    public string $animalName;
    public int $legs;
    public function constructor(string $animalName, int $legs) {
        $this->animalName = $animalName;
        $this->legs = $legs;
    }
    public function name(): string {
        return $this->animalName;
    }
    public function legCount(): int {
        return $this->legs;
    }
}
class Dog extends Animal {
    public string $sound;
    public function constructor(string $animalName) {
        parent::constructor($animalName, 4);
        $this->sound = \"woof\";
    }
    public function speak(): string {
        return $this->sound;
    }
}
";
#[test]
fn a_constructor_writes_its_fields_and_a_reader_reads_them_back() {
    let source = format!("{SHAPES}\nvar $a = new Animal(\"cat\", 4);\necho $a->name();\n");
    assert_eq!(output_of(&source), "cat");
}

#[test]
fn an_int_field_survives_the_round_trip_through_its_value_slot() {
    // A field slot is a whole 16-byte `Value`, so an `int` field is written
    // with a tag byte the read never looks at — `nvs_runtime::object`'s own
    // docs own that decision. This is it working.
    let source = format!("{SHAPES}\nvar $a = new Animal(\"cat\", 4);\necho $a->legCount();\n");
    assert_eq!(output_of(&source), "4");
}

#[test]
fn a_subclass_reaches_both_its_own_slot_and_its_parents() {
    // `sound` is `Dog`'s own, at the slot after `Animal`'s two — the
    // ancestors-first layout rule, observed rather than asserted.
    let source = format!(
        "{SHAPES}\nvar $d = new Dog(\"rex\");\necho $d->name(), \"/\", $d->speak(), \"/\";\necho $d->legCount();\n"
    );
    assert_eq!(output_of(&source), "rex/woof/4");
}

#[test]
fn an_inherited_method_is_called_on_a_subclass_instance() {
    // `Dog` declares no `name()`, so the call resolves to `Animal::name` with
    // a `Dog` receiver — the slot index the parent computed still lands on the
    // right field.
    let source = format!("{SHAPES}\nvar $d = new Dog(\"rex\");\necho $d->name();\n");
    assert_eq!(output_of(&source), "rex");
}

#[test]
fn a_field_can_be_overwritten_after_construction() {
    let source = format!(
        "{SHAPES}\nvar $a = new Animal(\"cat\", 4);\n$a->animalName = \"lion\";\n$a->legs = 3;\necho $a->name(), \"/\", $a->legCount();\n"
    );
    assert_eq!(output_of(&source), "lion/3");
}

#[test]
fn an_object_field_keeps_its_own_object_alive() {
    let source = "<?nvs
class Leg {
    public int $length;
    public function constructor(int $length) { $this->length = $length; }
    public function length(): int { return $this->length; }
}

class Cat {
    public Leg $front;
    public function constructor(Leg $front) { $this->front = $front; }
    public function front(): Leg { return $this->front; }
}

var $cat = new Cat(new Leg(12));
echo $cat->front()->length();
";
    assert_eq!(output_of(source), "12");
}

#[test]
fn instanceof_sees_the_class_its_parent_and_its_interface() {
    // One `nvs_object_instanceof` call per answer: the descriptor address is
    // baked in, and `nvs_types::layout` gives the *interface* a descriptor
    // with no slots purely so this test can name it.
    let source = "<?nvs
interface Greets {
    public function greeting(): string;
}

class Animal implements Greets {
    public string $animalName;
    public function constructor(string $animalName) { $this->animalName = $animalName; }
    public function greeting(): string { return $this->animalName; }
}

class Dog extends Animal {
}

class Rock {
}

var $d = new Dog(\"rex\");
if ($d instanceof Dog) { echo \"dog \"; }
if ($d instanceof Animal) { echo \"animal \"; }
if ($d instanceof Greets) { echo \"greets \"; }
if ($d instanceof Rock) { echo \"rock \"; }
";
    assert_eq!(output_of(source), "dog animal greets ");
}

// ---------------------------------------------------------------------------
// `rule:types/type-test` — `$x is T`, and the walks it is not allowed to
// duplicate
// ---------------------------------------------------------------------------
/// Every instruction one program lowered to, flattened and rendered.
///
/// These four cases pin **which instruction the test became**, not what it
/// answered, so they read the IR rather than the output: "the same walk
/// `instanceof` emits" and "not a second one" are both claims about the
/// instruction list and neither is observable from a `bool`.
fn kinds(source: &str) -> Vec<String> {
    lower(source)
        .functions
        .iter()
        .flat_map(|f| f.blocks.iter())
        .flat_map(|b| b.insts.iter())
        .map(|i| format!("{:?}", i.kind))
        .collect()
}

/// How many of `program`'s instructions render with `needle` in them.
fn count(program: &[String], needle: &str) -> usize {
    program.iter().filter(|k| k.contains(needle)).count()
}

#[test]
fn a_scalar_test_emits_one_tag_comparison() {
    // A `mixed` subject carries its tag at run time, so `is int` is exactly
    // one masked compare against it — no call, no walk, no allocation.
    let hit = "<?nvs\nmixed $m = 1;\necho $m is int;\n";
    let program = kinds(hit);
    assert_eq!(count(&program, "TagIs"), 1, "{program:?}");
    assert_eq!(count(&program, "InstanceOf"), 0, "{program:?}");
    assert_eq!(count(&program, "ToArrayOf"), 0, "{program:?}");
    assert_eq!(output_of(hit), "1");
    // The miss is the same one comparison, which is the half a test asserting
    // only the `true` direction would let a walk creep into.
    let miss = "<?nvs\nmixed $m = 1;\necho $m is string;\n";
    assert_eq!(count(&kinds(miss), "TagIs"), 1);
    assert_eq!(output_of(miss), "");
}

#[test]
fn a_class_test_emits_the_same_descriptor_walk_instanceof_emits() {
    // Written as an equality between two whole programs rather than as a
    // count: what `rule:types/type-test` promises is the walk `instanceof`
    // already pays for, and a second emitter that happened to emit one
    // `InstanceOf` too would pass a count.
    let subject = "<?nvs\nclass Animal {}\nclass Dog extends Animal {}\nmixed $d = new Dog();\n";
    let tested = format!("{subject}echo $d is Animal;\n");
    let spelled = format!("{subject}echo $d instanceof Animal;\n");
    assert_eq!(kinds(&tested), kinds(&spelled));
    assert_eq!(count(&kinds(&tested), "InstanceOf"), 1);
    assert_eq!(output_of(&tested), "1");
}

#[test]
fn an_array_element_test_calls_the_same_walk_the_conversion_calls_and_not_a_second_one() {
    // The O(n) element walk is `Helper::ToArrayOfOrNull` — `as ?array<T>`'s
    // spelling of the walk `as array<T>` throws from, which is the one that
    // *answers*. Exactly one of the pair anywhere in the program is the "not
    // a second one" half.
    let source = "<?nvs\nmixed $a = [1, 2];\necho $a is array<int>, $a is array<string>;\n";
    let program = kinds(source);
    assert_eq!(count(&program, "ToArrayOfOrNull"), 2, "{program:?}");
    assert_eq!(count(&program, "ToArrayOf,"), 0, "{program:?}");
    assert_eq!(count(&program, "InstanceOf"), 0, "{program:?}");
    assert_eq!(output_of(source), "1");
}

#[test]
fn a_folded_test_emits_no_code_at_all() {
    // A test the checker settled records its constant and nothing else, so
    // the program is instruction-for-instruction the one that wrote the
    // literal — not merely one with no tag compare in it.
    let folded = "<?nvs\nint $n = 1;\necho $n is int, $n is string;\n";
    let literal = "<?nvs\nint $n = 1;\necho true, false;\n";
    assert_eq!(kinds(folded), kinds(literal));
    assert_eq!(output_of(folded), "1");
}

#[test]
fn an_inherited_constructor_is_invoked_through_its_declaring_class() {
    // `new Dog(...)` on a subclass that declares no `constructor` of its own
    // must name `Animal::constructor`, the class that actually declares it —
    // naming `Dog::constructor` compiles to a call this unit never defined.
    let source = format!(
        "{SHAPES}\nclass Puppy extends Animal {{\n}}\nvar $p = new Puppy(\"pip\", 4);\necho $p->name(), \"/\", $p->legCount();\n"
    );
    assert_eq!(output_of(&source), "pip/4");
}

#[test]
fn a_string_field_overwritten_in_a_loop_leaks_nothing() {
    // Each write releases what the slot held — `nvs_ir::lower` emits the
    // `FieldGet`/`Release` pair and this backend emits the store. A missing
    // release would leak 10_000 buffers; a doubled one would crash.
    let source = format!(
        "{SHAPES}\nvar $a = new Animal(\"cat\", 4);\nvar $i = 0;\nwhile ($i < 10000) {{\n    $a->animalName = \"n\" . $i;\n    $i = $i + 1;\n}}\necho $a->name();\n"
    );
    assert_eq!(output_of(&source), "n9999");
}

#[test]
fn a_discarded_instance_is_released_rather_than_leaked() {
    // A bare `new` used for its constructor's effect only. The instance has
    // exactly one owner and nothing binds it, so lowering releases it right
    // away — 50_000 of them must not grow the heap without bound.
    let source = format!(
        "{SHAPES}\nvar $i = 0;\nwhile ($i < 50000) {{\n    new Dog(\"rex\");\n    $i = $i + 1;\n}}\necho \"done\";\n"
    );
    assert_eq!(output_of(&source), "done");
}

#[test]
fn a_class_with_no_constructor_still_allocates() {
    let source = "<?nvs
class Marker {
    public function tag(): string { return \"marker\"; }
}

var $m = new Marker();
echo $m->tag();
";
    assert_eq!(output_of(source), "marker");
}

#[test]
fn a_throw_out_of_a_frame_holding_an_object_releases_it() {
    // The landing block's cleanup, for an object rather than a string: the
    // local goes out of scope on the error path too.
    let source = format!(
        "{SHAPES}\nclass Boom {{\n    public static function go(): void {{\n        throw new LogicError(\"boom\");\n    }}\n}}\ntry {{\n    var $d = new Dog(\"rex\");\n    Boom::go();\n    echo $d->name();\n}} catch (Throwable $e) {{\n    echo \"caught: \" . $e->message;\n}}\n"
    );
    assert_eq!(output_of(&source), "caught: boom");
}
