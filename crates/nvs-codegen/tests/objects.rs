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
