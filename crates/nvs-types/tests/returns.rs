//! A method that declares a return type has to produce one on every path —
//! `E0739` — and, for the one return type that is not a fixed class, what a
//! call site reads back: `rule:statements/static-is-a-member-modifier`'s `static`, `E0741`, and the two
//! substitution sites `nvs_types::signatures::MethodSig::returns_static`
//! names.
//!
//! The refusal is one line of the analysis and the acceptances are all the
//! rest of it, which is why they outnumber it here: a wrong refusal is a
//! program the compiler will not build, and the loops, `switch`es and `try`s
//! below are ordinary code. `tests/conformance/lang/a-for-body-that-always-
//! returns.nvst` and its sibling pin the same rule from the running side.

mod common;

use common::check_src;
use nvs_diagnostics::{Diagnostics, code};

/// `class T` with one method, so a fixture is one line of interest.
fn method(sig: &str, body: &str) -> String {
    format!("<?nvs\nclass T {{\n  function m({sig} {{\n    {body}\n  }}\n}}\n")
}

/// `class T` with a `never` method `fail` and one method `m` under test, so a
/// call to a `never` member is one line of the fixture.
fn with_fail(sig: &str, body: &str) -> String {
    format!(
        "<?nvs\nclass T {{\n  function fail(): never {{\n    exit(1);\n  }}\n  \
         function m({sig} {{\n    {body}\n  }}\n}}\n"
    )
}

/// A `never` body that reaches its end would come back to a caller that was
/// promised it never does, so it owes `E0739` like any other declared type.
#[test]
fn a_never_body_that_reaches_its_end_is_refused() {
    let diags = check_src(&method("): never", "$x = 1;"));
    assert!(refuses(&diags), "{diags:?}");
}

/// The exits a `never` body has: a `throw`, and a call to another `never`
/// member, which the walk counts as leaving the frame.
#[test]
fn a_never_body_that_throws_or_calls_a_never_member_is_accepted() {
    let thrown = check_src(&method("): never", "throw new Core\\Error(\"x\");"));
    assert!(!refuses(&thrown), "{thrown:?}");
    let called = check_src(&with_fail("): never", "$this->fail();"));
    assert!(!called.has_errors(), "{called:?}");
}

/// A call to a `never` member ends an `int` body's path as a `throw` would,
/// so the `if` below needs no `return` after its `else` branch.
#[test]
fn a_call_to_a_never_member_ends_a_path() {
    let src = with_fail(
        "bool $b): int",
        "if ($b) {\n      return 1;\n    } else {\n      $this->fail();\n    }",
    );
    let diags = check_src(&src);
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The same call under a branch that may not run is not an exit: the `if`
/// with no `else` still reaches the end.
#[test]
fn a_call_to_a_never_member_under_one_branch_does_not_end_the_body() {
    let src = with_fail("bool $b): int", "if ($b) {\n      $this->fail();\n    }");
    assert!(refuses(&check_src(&src)));
}

/// A subtype may override a `never` member with one that returns, because
/// nothing yet checks an override's return type. So a call that may reach an
/// override is not an exit, and the body still owes its `return`.
#[test]
fn a_call_to_an_overridden_never_member_does_not_end_the_body() {
    let src = format!(
        "{}class Leaf extends T {{\n  function fail(): int {{\n    return 1;\n  }}\n}}\n",
        with_fail("): int", "$this->fail();")
    );
    assert!(refuses(&check_src(&src)));
}

fn refuses(diags: &Diagnostics) -> bool {
    diags.iter().any(|d| d.code == Some(code::E_MISSING_RETURN))
}

/// `Base::make(): static` plus whatever `Leaf` does with it, so the four
/// late-static-binding fixtures below differ by one line each.
fn hierarchy(base_body: &str, leaf: &str) -> String {
    format!(
        "<?nvs\nclass Base {{\n  public static function make(): static {{\n    {base_body}\n  }}\n  \
         function chain(): static {{\n    return $this;\n  }}\n}}\n\
         class Leaf extends Base {{\n{leaf}\n}}\n"
    )
}

/// The rule `rule:statements/static-is-a-member-modifier` states as a dispatch, read as a type: `make` is
/// declared on `Base`, so its *declared* return is `Base` — and the site named
/// `Leaf`, so what the call answers is a `Leaf`. Nothing but
/// `MethodSig::returns_static` carries that past the declaration.
#[test]
fn a_static_return_type_resolves_to_the_called_class() {
    let diags = hierarchy(
        "return new static();",
        "  public static function grab(): Leaf {\n    return Leaf::make();\n  }",
    );
    let diags = check_src(&diags);
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The other half of the same rule, and what keeps the substitution from being
/// "always the site's own class": an explicitly named class *sets* the called
/// class, so `Base::make()` answers a `Base` and owes `Leaf` nothing.
#[test]
fn naming_the_declaring_class_still_answers_that_class() {
    let src = hierarchy(
        "return new static();",
        "  public static function grab(): Leaf {\n    return Base::make();\n  }",
    );
    let diags = check_src(&src);
    assert!(diags.has_errors(), "{diags:?}");
}

/// The instance-call site, which reads the receiver rather than a written
/// class name — `crate::expr::calls`'s other `return_ty` substitution.
#[test]
fn an_instance_call_returning_static_answers_the_receivers_class() {
    let src = hierarchy(
        "return new static();",
        "  function grab(): Leaf {\n    return $this->chain();\n  }",
    );
    let diags = check_src(&src);
    assert!(!diags.has_errors(), "{diags:?}");
}

/// What makes the two substitutions above sound rather than a hole: a body
/// promising the called class may not answer the declaring one. PHP catches
/// this at run time; Novis has nothing below the type system to catch it
/// with, so it is refused at the declaration (`E0741`, whose own doc owns
/// why refusing beats accepting).
#[test]
fn a_static_return_of_the_declaring_class_is_refused() {
    let src = hierarchy(
        "return new self();",
        "  public static function grab(): Leaf {\n    return Leaf::make();\n  }",
    );
    let diags = check_src(&src);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_STATIC_RETURN_NOT_CALLED_CLASS)),
        "{diags:?}"
    );
}

/// The whitelist's third shape, and the one a fluent interface is made of:
/// `$this` *is* the called class, so `chain()` above is accepted — asserted
/// here on its own so a narrowing of the whitelist shows up as this test
/// rather than as four.
#[test]
fn returning_this_satisfies_a_static_return_type() {
    let diags =
        check_src("<?nvs\nclass T {\n  function chain(): static {\n    return $this;\n  }\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A call forwarded through `static::` keeps the caller's called class, so a
/// body that delegates to one is as sound as one that allocates. Refusing it
/// would refuse ordinary code, which is the failure mode a whitelist has.
#[test]
fn a_forwarded_static_call_satisfies_a_static_return_type() {
    let diags = check_src(
        "<?nvs\nclass T {\n  public static function make(): static {\n    return new static();\n  }\n  \
         public static function build(): static {\n    return static::make();\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The rule itself: `nvs_ir::lower::lower_method` seals the fall-through exit
/// with `Terminator::Return(None)`, so this body hands its caller no `int` at
/// all.
#[test]
fn a_non_void_function_must_return_on_every_path() {
    let diags = check_src(&method("): int", "echo \"x\";"));
    assert!(refuses(&diags), "{diags:?}");
}

/// The path, not the presence of a `return`: this body has one, on the branch
/// that is not the problem.
#[test]
fn a_branch_without_a_return_is_the_path_that_is_reported() {
    let diags = check_src(&method(
        "int $n): int",
        "if ($n > 0) {\n      return 1;\n    }",
    ));
    assert!(refuses(&diags), "{diags:?}");
}

#[test]
fn both_branches_returning_is_every_path() {
    let diags = check_src(&method(
        "int $n): int",
        "if ($n > 0) {\n      return 1;\n    } else {\n      return 0;\n    }",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

/// `void` promises nothing, and a declaration writing no return type at all —
/// a constructor — promises nothing either.
#[test]
fn a_void_method_and_a_constructor_are_not_asked_for_a_value() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function constructor() {\n    echo \"x\";\n  }\n  \
         function m(): void {\n    echo \"y\";\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A body that throws on every path produces no value and owes none.
#[test]
fn a_body_that_always_throws_is_an_exit() {
    let diags = check_src(&method("): int", "throw new Core\\Error(\"no\");"));
    assert!(!refuses(&diags), "{diags:?}");
}

/// The idiomatic server loop: nothing leaves it, so there is no path to the
/// closing brace to report.
#[test]
fn a_loop_with_no_way_out_never_reaches_the_end_of_the_body() {
    let diags = check_src(&method(
        "): int",
        "while (true) {\n      echo \"x\";\n    }",
    ));
    assert!(!refuses(&diags), "{diags:?}");
}

/// ...and the same loop with a `break` does reach it.
#[test]
fn a_break_out_of_an_endless_loop_is_a_path_to_the_end() {
    let diags = check_src(&method("): int", "while (true) {\n      break;\n    }"));
    assert!(refuses(&diags), "{diags:?}");
}

/// A `continue` re-tests a condition that is still `true`, so it is not a way
/// out — the distinction `nvs_types::returns`' `escapes` is written around.
#[test]
fn a_continue_inside_an_endless_loop_is_not_a_way_out() {
    let diags = check_src(&method(
        "int $n): int",
        "while (true) {\n      if ($n > 0) {\n        continue;\n      }\n      echo \"x\";\n    }",
    ));
    assert!(!refuses(&diags), "{diags:?}");
}

/// A `foreach` may iterate zero times, so a `return` inside one is never the
/// reason a body exits — this is the shape the accepting-side conformance
/// cases write a trailing `return` under.
#[test]
fn a_return_inside_a_foreach_does_not_cover_the_empty_subject() {
    let diags = check_src(&method(
        "array<int> $xs): int",
        "foreach ($xs as int $v) {\n      return $v;\n    }",
    ));
    assert!(refuses(&diags), "{diags:?}");
}

/// A `switch` covers every path only with a `default` — and every earlier arm
/// may fall through into the last one rather than exiting itself.
#[test]
fn a_switch_with_a_default_and_an_exiting_last_arm_covers_every_path() {
    let diags = check_src(&method(
        "int $n): int",
        "switch ($n) {\n      case 1:\n      case 2:\n        return 2;\n      \
         default:\n        return 0;\n    }",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_switch_without_a_default_leaves_a_path_uncovered() {
    let diags = check_src(&method(
        "int $n): int",
        "switch ($n) {\n      case 1:\n        return 1;\n    }",
    ));
    assert!(refuses(&diags), "{diags:?}");
}

/// A `finally` covers no path of its own: `rule:statements/no-return-leaves-a-finally`
/// refuses the `return` that would have made it the exit of the whole `try`
/// (`E0250`, in the parser), so coverage reads the body and the `catch`es alone.
#[test]
fn a_finally_covers_no_path_because_it_never_returns() {
    let diags = check_src(&method(
        "): int",
        "try {\n      return 1;\n    } catch (Core\\Error $e) {\n      return 0;\n    } \
         finally {\n      echo \"x\";\n    }",
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    let diags = check_src(&method(
        "): int",
        "try {\n      echo \"x\";\n    } finally {\n      echo \"y\";\n    }",
    ));
    assert!(refuses(&diags), "{diags:?}");
}

/// Without one, the `try` exits only when its body and every `catch` do.
#[test]
fn a_try_exits_when_its_body_and_every_catch_do() {
    let diags = check_src(&method(
        "): int",
        "try {\n      return 1;\n    } catch (Core\\Error $e) {\n      return 0;\n    }",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_catch_that_falls_through_is_a_path_to_the_end() {
    let diags = check_src(&method(
        "): int",
        "try {\n      return 1;\n    } catch (Core\\Error $e) {\n      echo \"x\";\n    }",
    ));
    assert!(refuses(&diags), "{diags:?}");
}

/// `rule:iteration/one-way-only`: a generator's body produces no return value at all, so the
/// `Iterator<int>` its declaration names is not a value any path owes.
#[test]
fn a_generator_body_owes_no_return() {
    let diags = check_src(&method("): Iterator<int>", "yield 1;"));
    assert!(!refuses(&diags), "{diags:?}");
}
