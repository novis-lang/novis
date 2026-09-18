//! Object layout and lifetime — constructors, inherited slots, the class test, and release on every exit path.
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
fn a_class_test_sees_the_class_its_parent_and_its_interface() {
    // One `nvs_object_is_class` call per answer: the descriptor address is
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
if ($d is Dog) { echo \"dog \"; }
if ($d is Animal) { echo \"animal \"; }
if ($d is Greets) { echo \"greets \"; }
if ($d is Rock) { echo \"rock \"; }
";
    assert_eq!(output_of(source), "dog animal greets ");
}

// ---------------------------------------------------------------------------
// `rule:types/type-test` — `$x is T`, and the walks it is not allowed to
// duplicate
// ---------------------------------------------------------------------------
/// Every instruction one program lowered to, flattened and rendered.
///
/// These cases pin **which instruction the test became**, not what it
/// answered, so they read the IR rather than the output: "one descriptor
/// walk" and "not a second one" are both claims about the instruction list
/// and neither is observable from a `bool`.
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
    assert_eq!(count(&program, "ClassTest"), 0, "{program:?}");
    assert_eq!(count(&program, "ToArrayOf"), 0, "{program:?}");
    assert_eq!(output_of(hit), "1");
    // The miss is the same one comparison, which is the half a test asserting
    // only the `true` direction would let a walk creep into.
    let miss = "<?nvs\nmixed $m = 1;\necho $m is string;\n";
    assert_eq!(count(&kinds(miss), "TagIs"), 1);
    assert_eq!(output_of(miss), "");
}

#[test]
fn a_class_test_emits_one_descriptor_walk_and_nothing_else() {
    // What `rule:types/type-test` promises is the descriptor walk itself, so
    // the assertion is that the program holds exactly one `ClassTest` and
    // neither of the two walks a test can otherwise reach for.
    let subject = "<?nvs\nclass Animal {}\nclass Dog extends Animal {}\nmixed $d = new Dog();\n";
    let tested = format!("{subject}echo $d is Animal;\n");
    let program = kinds(&tested);
    assert_eq!(count(&program, "ClassTest"), 1, "{program:?}");
    assert_eq!(count(&program, "ToArrayOf"), 0, "{program:?}");
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
    assert_eq!(count(&program, "ClassTest"), 0, "{program:?}");
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
fn a_literal_test_is_one_tag_comparison_with_a_payload_compare_behind_it() {
    // Two comparisons and not one: the tag says the payload word may be read
    // at this representation, and only then does the compare say whether it
    // holds the value. Both `is` spellings of the same subject pay one tag
    // compare each and neither reaches a walk.
    let source = "<?nvs\nmixed $m = 1;\necho $m is 1, $m is 2;\n";
    let program = kinds(source);
    assert_eq!(count(&program, "TagIs"), 2, "{program:?}");
    assert_eq!(count(&program, "ClassTest"), 0, "{program:?}");
    assert_eq!(count(&program, "ToArrayOf"), 0, "{program:?}");
    assert_eq!(output_of(source), "1");
    // The tag *miss* is the half that would not merely answer wrongly: a
    // `string` literal's payload compare is `nvs_str_eq` through two
    // pointers, so running it on a value tagged `Tag::Int` dereferences a
    // `1`. Asked of both subjects in one program, so a shape that answered
    // the miss by never comparing at all fails the hit beside it.
    let crossed =
        "<?nvs\nmixed $m = 1;\nmixed $s = \"yay\";\necho $m is 'yay', $s is 'yay', $s is 'nay';\n";
    assert_eq!(output_of(crossed), "1");
    // A subject carrying exactly one tag has answered the first comparison
    // already, so the payload one stands alone — and `$n is 3` is not a row
    // the checker could have folded, an `int` and a `3` being neither
    // disjoint nor one inside the other.
    let typed = "<?nvs\nint $n = 3;\necho $n is 3, $n is 4;\n";
    let typed_program = kinds(typed);
    assert_eq!(count(&typed_program, "TagIs"), 0, "{typed_program:?}");
    assert_eq!(output_of(typed), "1");
    // The constant every `string` comparison allocates is released as soon as
    // the comparison has read it: a missing release leaks 50_000 buffers, and
    // a doubled one crashes.
    let repeated = "<?nvs\nmixed $s = \"yay\";\nvar $i = 0;\nvar $hits = 0;\nwhile ($i < 50000) {\n    if ($s is 'yay') { $hits = $hits + 1; }\n    $i = $i + 1;\n}\necho $hits;\n";
    assert_eq!(output_of(repeated), "50000");
}

#[test]
fn an_enum_case_test_is_that_same_shape_one_representation_down() {
    // `rule:enums/representation` makes a case its backing integer, so the
    // tag is that integer's and the payload compare is against the case's own
    // value. A subject whose static type is the enum has answered the tag
    // half already, and the relabel to the backing integer is
    // `InstKind::Reinterpret`, which emits no machine instruction — so both
    // tests together are two machine compares.
    let source = "<?nvs\nenum Rank {\n    Bronze,\n    Silver,\n    Gold,\n}\n\nRank $r = Rank::Silver;\necho $r is Rank::Silver, $r is Rank::Gold;\n";
    let program = kinds(source);
    assert_eq!(count(&program, "TagIs"), 0, "{program:?}");
    assert_eq!(count(&program, "ClassTest"), 0, "{program:?}");
    assert_eq!(count(&program, "ToArrayOf"), 0, "{program:?}");
    assert_eq!(output_of(source), "1");
    // A `mixed` subject pays the tag comparison, and answers the same for one
    // holding the backing integer alone. That is `rule:enums/representation`'s
    // own stated consequence — "a value that reaches `mixed` is not
    // distinguishable there from its backing integer" — and not a choice this
    // operator makes; the tag it reserves for an enum is what would separate
    // the two. Pinned rather than left implicit, because the day that tag is
    // spent this assertion is what says which answer changed.
    let erased = "<?nvs\nenum Rank {\n    Bronze,\n    Silver,\n    Gold,\n}\n\nmixed $r = Rank::Silver;\nmixed $i = 1;\nmixed $s = \"x\";\necho $r is Rank::Silver, $i is Rank::Silver, $s is Rank::Silver;\n";
    let erased_program = kinds(erased);
    assert_eq!(count(&erased_program, "TagIs"), 3, "{erased_program:?}");
    assert_eq!(output_of(erased), "11");
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

/// `rule:core-classes/reflect`'s property half, at the layer that carries it:
/// a slot's declared type is spelled where the declaration is and reaches the
/// runtime descriptor, because `nvs_runtime::ClassDesc::field_tag` — the only
/// other per-slot type fact — answers `?int`, `int` and `7` with one tag and
/// every class with another.
///
/// The types are read against the visibility bits rather than alone: the two
/// are parallel vectors over one slot order, so a roster that slipped by one
/// would still answer plausibly on its own.
#[test]
fn a_propertys_declared_type_reaches_its_descriptor_in_slot_order() {
    let unit = compile(
        "<?nvs
class Base {
    public ?int $tally = null;
}
class Item extends Base {
    public array<string> $tags;
    public function constructor(private string $note) { $this->tags = [\"a\"]; }
}
echo (new Item(\"n\"))->tags[0];
",
    )
    .expect("the fixture compiles");

    let item = unit.class_desc("Item").expect("the class is declared");
    // Ancestors first, then this class's own in declaration order — the slot
    // order `nvs_types::layout` fixes, with the promoted parameter standing
    // where its `constructor` member does.
    assert_eq!(item.field_type(0), Some("?int"));
    assert_eq!(item.field_type(1), Some("array<string>"));
    assert_eq!(item.field_type(2), Some("string"));
    assert_eq!(item.field_type(3), None, "there is no fourth slot");
    // The same three slots, read through the roster beside this one.
    assert!(item.field_is_public(1));
    assert!(
        !item.field_is_public(2),
        "the promoted parameter is private"
    );

    // The parent holds its own slot at the index it claimed, and a class
    // nothing laid out from a declaration reports the absence rather than a
    // name: the exception tree is typed in `nvs_types::error_lib` as interned
    // ids, which are not text.
    let base = unit.class_desc("Base").expect("the class is declared");
    assert_eq!(base.field_type(0), Some("?int"));
    let thrown = unit
        .class_desc("LogicError")
        .expect("the exception tree is always defined");
    assert_eq!(thrown.field_type(0), None);
}

/// `rule:core-classes/reflect`'s method half, at the layer that carries it: a
/// parameter's *name* is spelled where the declaration is and reaches the
/// runtime descriptor, because `nvs_runtime::MethodRow`'s other two parameter
/// facts count and type the parameters without naming one.
///
/// The names are read against the arity they ride with rather than alone: a
/// roster that took the receiver in would still answer plausibly on its own
/// line.
#[test]
fn a_methods_parameter_names_reach_its_descriptor_in_declaration_order() {
    let unit = compile(
        "<?nvs
class Greeter {
    public function greet(string $who, int $times): string { return $who; }
}
class Loud extends Greeter {
    public function constructor(private string $mark) {}
    public function shout(string $what): string { return $what; }
}
echo (new Loud(\"!\"))->greet(\"a\", 1);
",
    )
    .expect("the fixture compiles");

    let loud = unit.class_desc("Loud").expect("the class is declared");
    let shout = loud.method_row("shout").expect("its own method");
    assert_eq!(shout.param_names, ["what"]);
    // The receiver is excluded from both, which is the agreement that makes
    // the names readable as the arity's own list.
    assert_eq!(shout.param_names.len(), shout.arity as usize);
    // The promoted parameter is a parameter, standing where it is written.
    let constructor = loud.method_row("constructor").expect("it declares one");
    assert_eq!(constructor.param_names, ["mark"]);
    // An inherited method arrives with the spellings of the declaration it
    // came from, not of the class that answers it.
    let greet = loud.method_row("greet").expect("inherited from Greeter");
    assert_eq!(greet.param_names, ["who", "times"]);

    // A row no declaration was read for reports the absence rather than a
    // guess: the exception tree's constructor is synthesized, and
    // `nvs_types::error_lib` spells its signature in the call site's currency.
    let thrown = unit
        .class_desc("LogicError")
        .expect("the exception tree is always defined");
    assert!(
        thrown
            .method_row("constructor")
            .expect("the tree declares one")
            .param_names
            .is_empty()
    );
}

/// `rule:tooling/commands-are-compiled`'s help page asks the other parameter
/// question, and this is the layer that answers it: a parameter's declared type
/// reaches the runtime descriptor as the text the declaration wrote, because
/// `nvs_runtime::MethodRow::param_tags` types a parameter in the runtime's own
/// representation and one nibble tells neither `int` from `uint` nor one class
/// from another.
///
/// The spellings are read against the names rather than alone: two rosters that
/// disagreed in length would each still answer plausibly on its own line while
/// pairing every parameter after the first with the wrong type.
#[test]
fn a_methods_declared_parameter_types_reach_its_descriptor_beside_the_names() {
    let unit = compile(
        "<?nvs
class Sink {
    public function accept(string $who, ?int $times): string { return $who; }
}
class Louder extends Sink {
    public function constructor(private string $mark) {}
    public function retry(uint $retries): uint { return $retries; }
}
echo (new Louder(\"!\"))->accept(\"a\", 1);
",
    )
    .expect("the fixture compiles");

    let louder = unit.class_desc("Louder").expect("the class is declared");
    // A type the tag word cannot spell — `uint` and `int` share a nibble, which
    // is the whole reason the text travels.
    let retry = louder.method_row("retry").expect("its own method");
    assert_eq!(retry.param_types, ["uint"]);
    // And one the tag word cannot spell for the other reason: a nullable is the
    // same representation as what it wraps plus a case for the absence.
    let accept = louder.method_row("accept").expect("inherited from Sink");
    assert_eq!(accept.param_names, ["who", "times"]);
    assert_eq!(accept.param_types, ["string", "?int"]);
    // The two rosters are one roster read twice: same length as each other and
    // as the arity they ride with, the receiver excluded from all three.
    assert_eq!(accept.param_types.len(), accept.param_names.len());
    assert_eq!(accept.param_types.len(), accept.arity as usize);
    // A promoted parameter is typed where it is written, standing in its own
    // place.
    let constructor = louder.method_row("constructor").expect("it declares one");
    assert_eq!(constructor.param_types, ["string"]);

    // A row no declaration was read for reports the absence in both rosters
    // rather than half of one: the exception tree's constructor is synthesized.
    let thrown = unit
        .class_desc("LogicError")
        .expect("the exception tree is always defined");
    let synthesized = thrown
        .method_row("constructor")
        .expect("the tree declares one");
    assert!(synthesized.param_types.is_empty());
    assert!(synthesized.param_names.is_empty());
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
