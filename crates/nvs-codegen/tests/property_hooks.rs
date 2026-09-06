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
