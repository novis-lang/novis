//! A method that declares a return type has to produce one on every path —
//! `E0739`, and the shapes `nvs_types::returns` must not report.
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

fn refuses(diags: &Diagnostics) -> bool {
    diags.iter().any(|d| d.code == Some(code::E_MISSING_RETURN))
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

/// A `finally` that returns is the exit of the whole `try`, whatever the body
/// and the `catch`es did.
#[test]
fn a_finally_that_returns_covers_the_whole_try() {
    let diags = check_src(&method(
        "): int",
        "try {\n      echo \"x\";\n    } finally {\n      return 1;\n    }",
    ));
    assert!(!refuses(&diags), "{diags:?}");
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

/// ADR 0053 § 5: a generator's body produces no return value at all, so the
/// `Iterator<int>` its declaration names is not a value any path owes.
#[test]
fn a_generator_body_owes_no_return() {
    let diags = check_src(&method("): Iterator<int>", "yield 1;"));
    assert!(!refuses(&diags), "{diags:?}");
}
