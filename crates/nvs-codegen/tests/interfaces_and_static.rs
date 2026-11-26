//! `rule:classes/no-traits`'s interface default bodies and PHP's late static binding — `static::`, `new static`, `self::`.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;

#[test]
fn an_interface_default_body_calls_back_through_the_receivers_own_class() {
    // `rule:classes/interface-default-methods`'s default method, compiled: `greet()` has a body declared
    // on the *interface*, and its body calls `name()`, which the interface
    // only declares. There is no `Greets::name` function to call, so the call
    // dispatches on the receiver's runtime class — the one shape a bodiless
    // resolved target leaves no alternative for.
    let source = "<?nvs
interface Greets {
    public function name(): string;
    public function greet(): string { return \"Hello, \" . $this->name() . \"!\"; }
}
class Animal implements Greets {
    protected string $animalName;
    public function constructor(string $animalName) { $this->animalName = $animalName; }
    public function name(): string { return $this->animalName; }
}
class Dog extends Animal {
    public function name(): string { return \"dog \" . $this->animalName; }
}
var $a = new Animal(\"cat\");
var $d = new Dog(\"rex\");
echo $a->greet() . \"/\" . $d->greet();
";
    assert_eq!(output_of(source), "Hello, cat!/Hello, dog rex!");
}

/// The source of every fixture below: `Registry` declares `tag()`, `make()`
/// and `label()` once, and two levels of subclass override `tag()` only.
/// Written out here rather than in each test so what each one actually
/// asserts is the *call*, not the class tree.
const LATE_BINDING: &str = "<?nvs
class Registry {
    public static function tag(): string { return \"base\"; }
    public static function make(): static { return new static(); }
    public static function forwarded(): string { return static::tag(); }
    public function label(): string { return static::tag(); }
}
class MidRegistry extends Registry {
    public static function tag(): string { return \"mid\"; }
}
class LeafRegistry extends MidRegistry {
    public static function tag(): string { return \"leaf\"; }
}
";
#[test]
fn a_static_call_names_the_class_it_was_written_on() {
    // `LeafRegistry::forwarded()` runs `Registry::forwarded`, whose body says
    // `static::tag()`. The called class is `LeafRegistry` because that is what
    // the call site named — not `Registry`, where the code is declared. That
    // is the whole of late static binding: the called class travels in the
    // receiver slot a static method's caller already fills.
    let source = format!("{LATE_BINDING}echo LeafRegistry::forwarded();\n");
    assert_eq!(output_of(&source), "leaf");
    let source = format!("{LATE_BINDING}echo MidRegistry::forwarded();\n");
    assert_eq!(output_of(&source), "mid");
    let source = format!("{LATE_BINDING}echo Registry::forwarded();\n");
    assert_eq!(output_of(&source), "base");
}

#[test]
fn new_static_allocates_the_called_class_through_two_levels() {
    // `new static()` reached through two
    // levels of inheritance returns the called class. Proved by asking the
    // fresh instance what it is — a class test reads the descriptor the
    // allocation actually used.
    let source = format!(
        "{LATE_BINDING}var $leaf = LeafRegistry::make();
if ($leaf is LeafRegistry) {{ echo \"leaf \"; }}
if ($leaf is MidRegistry) {{ echo \"mid \"; }}
var $base = Registry::make();
if ($base is MidRegistry) {{ echo \"wrong\"; }} else {{ echo \"base\"; }}
"
    );
    assert_eq!(output_of(&source), "leaf mid base");
}

#[test]
fn an_instance_method_binds_static_to_the_receivers_own_class() {
    // No hidden argument reaches `Registry::label` at all: `static::` inside
    // an instance method reads the class word off `$this`. The receiver here
    // is a `LeafRegistry` even though its static type is whatever
    // `Registry::make` declared.
    let source = format!("{LATE_BINDING}echo LeafRegistry::make()->label();\n");
    assert_eq!(output_of(&source), "leaf");
}

#[test]
fn self_forwards_the_called_class_while_an_explicit_name_resets_it() {
    // PHP's own rule, and the reason `ResolvedCall::static_class` exists:
    // `self::forwarded()` passes the caller's called class straight through,
    // while `Registry::forwarded()` sets it to `Registry` no matter where it
    // is written.
    let source = "<?nvs
class Registry {
    public static function tag(): string { return \"base\"; }
    public static function forwarded(): string { return static::tag(); }
    public static function viaSelf(): string { return self::forwarded(); }
    public static function viaName(): string { return Registry::forwarded(); }
}
class LeafRegistry extends Registry {
    public static function tag(): string { return \"leaf\"; }
}
echo LeafRegistry::viaSelf() . \"/\" . LeafRegistry::viaName();
";
    assert_eq!(output_of(source), "leaf/base");
}

/// Calls of `Parses::format` and `Comparable::compareTo` through the interface
/// type, on `Core` receivers. Each interface declares the member with no body,
/// so the call is late-bound, and a `Core` class has no method row for it:
/// its member is native and borrows its arguments.
const CORE_THROUGH_AN_INTERFACE: &str = "<?nvs
class Through {
    public static function written(Parses $value): tainted string {
        return $value->format();
    }
    public static function ordered(Comparable $a, Comparable $b): int {
        return $a->compareTo($b);
    }
    public static function less(Comparable $a, Comparable $b): bool {
        return $a < $b;
    }
}
";

#[test]
fn a_call_through_an_interface_reaches_a_core_receivers_native_member() {
    // The floor `nvs_runtime::bodiless_floor` names for `format` and
    // `compareTo` calls the native member. Without it the call stops with the
    // `FATAL` `nvs_runtime::nvs_abstract_method` reports. The receivers are
    // read again after each call, so a release too many frees them first.
    let source = format!(
        "{CORE_THROUGH_AN_INTERFACE}
var $id = Core\\Uuid::fromBytes(Core\\Encoding::fromHex(\"00112233445566778899aabbccddeeff\"));
echo Through::written($id), \" \", $id->format(), \" \";
var $three = Core\\BigInt::of(3);
var $four = Core\\BigInt::of(4);
echo Through::ordered($three, $four), \" \", Through::ordered($four, $three), \" \";
echo Through::less($three, $four) ? \"less\" : \"not less\", \" \", $three->compareTo($four);
"
    );
    assert_eq!(
        output_of(&source),
        "00112233-4455-6677-8899-aabbccddeeff 00112233-4455-6677-8899-aabbccddeeff -1 1 less -1"
    );
}

#[test]
#[cfg(debug_assertions)]
fn a_call_through_an_interface_on_a_core_receiver_frees_what_it_was_given() {
    // The call transfers the receiver and the operand, and the floor releases
    // both after the borrowing native member returns. A missed release keeps a
    // `format` result or an operand alive on every iteration, so ten times the
    // iterations must not hold ten times the bytes.
    let held = |times: usize| {
        let source = format!(
            "{CORE_THROUGH_AN_INTERFACE}
var $id = Core\\Uuid::fromBytes(Core\\Encoding::fromHex(\"00112233445566778899aabbccddeeff\"));
var $three = Core\\BigInt::of(3);
int $sum = 0;
for (int $i = 0; $i < {times}; $i++) {{
    tainted string $text = Through::written($id);
    $sum += Through::ordered($three, Core\\BigInt::of($i));
}}
"
        );
        let unit = compile(&source).expect("the fixture compiles");
        let mut ctx = Ctx::buffered();
        unit.install_in(&mut ctx);
        let entry = unit.script().expect("the script frame was compiled");
        let before = nvs_runtime::budget::live_bytes();
        entry.call(&mut ctx).expect("the script ran to completion");
        nvs_runtime::budget::live_bytes() - before
    };
    assert_eq!(held(10), held(100));
}
