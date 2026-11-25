//! `rule:statements/static-is-a-member-modifier`'s `static` among a callable
//! parameter's own parameters: `callable(static)` binds at the call to the
//! class the call names, the body reads the parameter as a plain `callable`,
//! and a direct call through it in the body is proven when each `static`
//! argument is the called class (`E1015` otherwise).
//!
//! Every fixture is a `Model` that declares the methods under test, an `Order`
//! below it, and a `Caller` whose `run` makes the calls.

mod common;

use common::{check_src, check_src_declared};
use nvs_diagnostics::{Diagnostics, code};
use nvs_types::expr_table::ExprInfo;

/// `Model` with `members`, `Order extends Model`, and `Caller::run` with `calls`.
fn fixture(members: &str, calls: &str) -> String {
    format!(
        "<?nvs\nclass Model {{\n{members}\n}}\nclass Order extends Model {{\n  public int $total = 0;\n}}\n\
         class Caller {{\n  public function run(): void {{\n{calls}\n  }}\n}}\n"
    )
}

/// A static `check` that takes a `callable(static): bool` and calls nothing.
const STATIC_CHECK: &str =
    "  public static function check(callable(static): bool $rule): bool {\n    return true;\n  }";

/// An instance `test` that calls its `callable(static): bool` with `$this`.
const INSTANCE_TEST: &str =
    "  public function test(callable(static): bool $rule): bool {\n    return $rule($this);\n  }";

fn has(diags: &Diagnostics, c: nvs_diagnostics::Code) -> bool {
    diags.iter().any(|d| d.code == Some(c))
}

#[test]
fn a_static_in_a_callable_parameter_binds_at_a_static_call() {
    let ok = check_src(&fixture(
        STATIC_CHECK,
        "    Order::check(fn (Order $o): bool => $o->total > 0);",
    ));
    assert!(!ok.has_errors(), "{ok:?}");
    // On `Model` the function must take every `Model`.
    let refused = check_src(&fixture(
        STATIC_CHECK,
        "    Model::check(fn (Order $o): bool => $o->total > 0);",
    ));
    assert!(refused.has_errors(), "{refused:?}");
}

#[test]
fn a_static_in_a_callable_parameter_binds_at_an_instance_call() {
    let ok = check_src(&fixture(
        INSTANCE_TEST,
        "    var $order = new Order();\n    $order->test(fn (Order $o): bool => $o->total > 0);",
    ));
    assert!(!ok.has_errors(), "{ok:?}");
    let refused = check_src(&fixture(
        INSTANCE_TEST,
        "    Model $model = new Order();\n    $model->test(fn (Order $o): bool => $o->total > 0);",
    ));
    assert!(refused.has_errors(), "{refused:?}");
}

#[test]
fn a_nullable_callable_parameter_binds_too() {
    for ty in ["?callable(static): bool", "(callable(static): bool)|null"] {
        let members =
            format!("  public static function maybe({ty} $rule): bool {{\n    return true;\n  }}");
        let ok = check_src(&fixture(
            &members,
            "    Order::maybe(fn (Order $o): bool => true);\n    Order::maybe(null);",
        ));
        assert!(!ok.has_errors(), "{ty}: {ok:?}");
        let refused = check_src(&fixture(
            &members,
            "    Model::maybe(fn (Order $o): bool => true);",
        ));
        assert!(refused.has_errors(), "{ty}: {refused:?}");
    }
}

#[test]
fn a_static_in_a_nested_callable_or_an_array_of_callables_does_not_bind() {
    // A nested `static` keeps the whole parameter at the declaring class.
    let deep = "  public static function deep(callable(static, callable(static): bool): bool $rule): bool {\n    return true;\n  }";
    let nested = check_src(&fixture(
        deep,
        "    Order::deep(fn (Order $o, callable(Order): bool $f): bool => true);",
    ));
    assert!(nested.has_errors(), "{nested:?}");
    let declaring = check_src(&fixture(
        deep,
        "    Order::deep(fn (Model $o, callable(Model): bool $f): bool => true);",
    ));
    assert!(!declaring.has_errors(), "{declaring:?}");
    let all = "  public static function all(array<callable(static): bool> $rules): bool {\n    return true;\n  }";
    let array = check_src(&fixture(
        all,
        "    Order::all([fn (Order $o): bool => true]);",
    ));
    assert!(array.has_errors(), "{array:?}");
    let declaring = check_src(&fixture(
        all,
        "    Order::all([fn (Model $o): bool => true]);",
    ));
    assert!(!declaring.has_errors(), "{declaring:?}");
}

#[test]
fn a_static_in_a_returned_callable_does_not_bind() {
    // Unbound, the return is `callable(Model): bool` at every call.
    let diags = check_src(&fixture(
        "  public static function rule(): callable(static): bool {\n    return fn (Model $m): bool => true;\n  }",
        "    callable(Model): bool $rule = Order::rule();",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_call_with_this_new_static_or_a_static_return_is_proven() {
    let members = "  public static function make(): static {\n    return new static();\n  }\n\
                   public function viaThis(callable(static): bool $rule): bool {\n    return $rule($this);\n  }\n\
                   public static function viaNew(callable(static): bool $rule): bool {\n    return $rule(new static());\n  }\n\
                   public static function viaMake(callable(static): bool $rule): bool {\n    return $rule(static::make());\n  }";
    let (diags, table) = check_src_declared(&fixture(members, ""));
    assert!(!diags.has_errors(), "{diags:?}");
    for call in [
        "$rule($this)",
        "$rule(new static())",
        "$rule(static::make())",
    ] {
        assert!(
            matches!(
                table.folded_at(call),
                Some(ExprInfo::CallThroughSignature { .. })
            ),
            "{call} is not proven: {:?}",
            table.folded_at(call)
        );
    }
}

#[test]
fn a_call_with_another_value_is_refused() {
    let members = "  public function other(callable(static): bool $rule): bool {\n    return $rule(new Model());\n  }";
    let diags = check_src(&fixture(members, ""));
    assert!(
        has(&diags, code::E_STATIC_CALLBACK_ARGUMENT_NOT_CALLED_CLASS),
        "{diags:?}"
    );
    let self_new = "  public static function other(callable(static): bool $rule): bool {\n    return $rule(new self());\n  }";
    let diags = check_src(&fixture(self_new, ""));
    assert!(
        has(&diags, code::E_STATIC_CALLBACK_ARGUMENT_NOT_CALLED_CLASS),
        "{diags:?}"
    );
}

#[test]
fn a_call_inside_an_anonymous_function_is_a_plain_callable_call() {
    let members = "  public function later(callable(static): bool $rule): callable {\n    return fn (): mixed => $rule(new Model());\n  }";
    let (diags, table) = check_src_declared(&fixture(members, ""));
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(table.folded_at("$rule(new Model())").is_none());
}

#[test]
fn a_parameter_the_body_assigns_to_is_a_plain_callable() {
    let members = "  public function swap(callable(static): bool $rule): bool {\n    $rule = fn (Model $m): bool => true;\n    $rule($this);\n    return true;\n  }";
    let (diags, table) = check_src_declared(&fixture(members, ""));
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(table.folded_at("$rule($this)").is_none());
}

#[test]
fn the_parameter_passed_on_is_a_plain_callable() {
    let stored = "  public function keep(callable(static): bool $rule): bool {\n    callable $kept = $rule;\n    return true;\n  }";
    let diags = check_src(&fixture(stored, ""));
    assert!(!diags.has_errors(), "{diags:?}");
    // A typed slot does not take it: the function may take only a subclass.
    let typed = "  public function pass(callable(static): bool $rule): bool {\n    callable(Model): bool $kept = $rule;\n    return true;\n  }";
    let diags = check_src(&fixture(typed, ""));
    assert!(diags.has_errors(), "{diags:?}");
}

#[test]
fn an_override_may_redeclare_a_callable_static_parameter() {
    let src = "<?nvs\nclass Model {\n  public function test(callable(static): bool $rule): bool {\n    return $rule($this);\n  }\n}\n\
               class Order extends Model {\n  public function test(callable(static): bool $rule): bool {\n    return $rule($this);\n  }\n}\n\
               class Line extends Model {\n  public function test(callable(Model): bool $rule): bool {\n    return $rule($this);\n  }\n}\n";
    let diags = check_src(src);
    assert!(!diags.has_errors(), "{diags:?}");
}
