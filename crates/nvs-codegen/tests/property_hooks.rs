//! `rule:classes/property-observer`'s property hooks: when each runs, how each is compiled, and a throwing one reaching a `catch`.
//!
//! See `tests/common/mod.rs` for the shared fixtures and for why these go
//! through the real pipeline.

mod common;

use common::*;

/// The whole of `rule:classes/property-hooks`,
/// end to end: a `get` hook produces the value a read yields, a `set` hook
/// commits what a write hands it, the short `=> expr;` form means "return
/// this" for `get` and "store this" for `set`, and inside a hook the property
/// is its own backing slot — which is what lets `$n`'s pair round-trip
/// without recursing.
#[test]
fn a_property_hook_runs_on_every_read_and_write_of_its_property() {
    let source = "<?nvs
class Box {
    public string $label;

    public string $shout {
        get => $this->label . \"!\";
        set(string $v) { $this->label = $v . \"?\"; }
    }

    public int $n {
        get => $this->n + 1;
        set => $value * 2;
    }

    public function constructor(string $label) {
        $this->label = $label;
        $this->n = 5;
    }
}
var $b = new Box(\"hi\");
echo $b->shout, \"/\";
$b->shout = \"yo\";
echo $b->label, \"/\", $b->n, \"/\";
$b->n = 10;
echo $b->n;
";
    assert_eq!(output_of(source), "hi!/yo?/11/21");
}

/// A hooked property is reached through the ordinary `InstKind::Call`, so it
/// is compiled under the same label `nvs_types::signatures::hook_label`
/// spells and needs no dispatch-table entry of its own — the property that
/// keeps a hooked access from costing a new calling convention.
#[test]
fn each_property_hook_is_compiled_under_its_own_label() {
    let unit = compile(
        "<?nvs
class Box {
    public int $n;
    public int $doubled { get => $this->n * 2; }
    public function constructor(int $n) { $this->n = $n; }
}
echo (new Box(2))->doubled;
",
    )
    .expect("the fixture compiles");
    assert!(unit.has_function("Box::$doubled::get"));
    assert!(!unit.has_function("Box::$doubled::set"));
}

/// Every hook a class answers reaches its **runtime descriptor**, keyed by
/// the property and the accessor and inherited the way a method is — which is
/// what lets an access through an erased receiver run the accessor a
/// statically resolved one calls directly, `rule:classes/property-hooks`
/// making every read and write of a hooked property a call to its hook at
/// every access spelling alike.
///
/// Asserted at the descriptor rather than at the label, because
/// `each_property_hook_is_compiled_under_its_own_label` already pins that the
/// function exists: what is new here is the *roster* — an override beating the
/// declaration it overrides, an accessor the subclass did not rewrite still
/// answering from the superclass's body, and a hook staying out of the method
/// table, where a `Class::name()` call and
/// `Core\Reflect\ClassInfo::methods` would otherwise find it.
#[test]
fn a_classs_hooks_reach_its_descriptor_keyed_by_property_and_accessor() {
    let unit = compile(
        "<?nvs
class Base {
    public string $label;
    public string $shout {
        get => $this->label . \"!\";
        set(string $v) { $this->label = $v; }
    }
    public function constructor(string $label) { $this->label = $label; }
}
class Loud extends Base {
    public string $shout {
        set(string $v) { $this->label = $v . \"?\"; }
    }
}
echo (new Loud(\"hi\"))->shout;
",
    )
    .expect("the fixture compiles");

    let base = unit.class_desc("Base").expect("the class is declared");
    for set in [false, true] {
        let row = base
            .hook_row("shout", set)
            .expect("the declaration wrote both accessors");
        assert!(!row.code.is_null(), "set={set}");
    }
    assert!(base.hook_row("label", false).is_none());

    // The subclass rewrote `set` and left `get` alone, so its roster answers
    // one body of its own and one of its parent's — the same precedence
    // `flatten_methods` gives an overridden method, read on the accessor
    // rather than on the name.
    let loud = unit.class_desc("Loud").expect("the class is declared");
    let own = loud.hook_row("shout", true).expect("its own `set`");
    let inherited = loud.hook_row("shout", false).expect("the inherited `get`");
    assert!(!own.code.is_null());
    assert_eq!(
        inherited.code,
        base.hook_row("shout", false)
            .expect("the parent's `get`")
            .code
    );
    assert_ne!(own.code, base.hook_row("shout", true).expect("`set`").code);

    // And neither roster is the method table: a hook is an accessor of a
    // property, so nothing reaching a class by a *method* name finds one.
    assert!(loud.method_row("shout").is_none());
    assert!(loud.method_row("Loud::$shout::set").is_none());
}

/// A hook that throws propagates through
/// `rule:errors/propagation`'s checked-return
/// path like any other call, because it *is* one — the read carries the same
/// error edge a method call does, so the throw reaches an ordinary `catch`
/// rather than escaping the expression that triggered it.
#[test]
fn a_throwing_property_hook_reaches_an_ordinary_catch() {
    let source = "<?nvs
class Guard {
    public int $n;
    public int $checked { get => $this->fail(); }
    public function constructor(int $n) { $this->n = $n; }
    public function fail(): int { throw new LogicError(\"hook said no\"); }
}
var $g = new Guard(1);
try {
    echo $g->checked;
} catch (LogicError $e) {
    echo \"caught: \", $e->message;
}
";
    assert_eq!(output_of(source), "caught: hook said no");
}
