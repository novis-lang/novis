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
    // M4's acceptance sentence, exactly: `new static()` reached through two
    // levels of inheritance returns the called class. Proved by asking the
    // fresh instance what it is — `instanceof` reads the descriptor the
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
