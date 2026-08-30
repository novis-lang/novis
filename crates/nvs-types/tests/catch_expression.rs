//! The expression-level `catch` as the checker sees it — ADR 0119 §§ 3-5.
//!
//! The parser's half is `nvs-syntax`'s own tests; what is held here is the
//! part that has no syntax to point at: the result type, the arm's binding and
//! the definite-assignment state an arm is checked from.

mod common;

use common::check_src;
use nvs_diagnostics::{Diagnostics, code};

/// Wraps `body` in a method beside a `rows()` that can be guarded — a call
/// rather than a literal, so the guarded expression has a return type the
/// union is visibly made of.
fn check_with_rows(body: &str) -> Diagnostics {
    check_src(&format!(
        "<?nvs\nclass T {{\n  function rows(): int {{ return 1; }}\n\
         function m(): void {{\n{body}\n  }}\n}}\n"
    ))
}

/// Whether `diags` carries ADR 0119 § 5's advisory.
fn warns_about_discarding(diags: &Diagnostics) -> bool {
    diags
        .iter()
        .any(|d| d.code == Some(code::W_CATCH_ARM_DISCARDS_EVERY_FAILURE))
}

/// § 4: the union of the guarded expression's type and every arm's. Both sides
/// are `int` here, so the declaration on the left accepts it.
#[test]
fn an_expression_catch_types_as_the_union_of_its_sides() {
    let diags =
        check_with_rows("    int $n = $this->rows() catch (RuntimeError $e) => 0;\n    echo $n;");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The same shape with one side widened, which is what makes the acceptance
/// above a statement about the *union* rather than about the guard alone: an
/// arm answering `null` makes the whole expression `?int`, and `int` refuses
/// it with the mismatch diagnostic it already had.
#[test]
fn an_arm_that_widens_the_union_is_refused_by_the_declaration() {
    let diags = check_with_rows(
        "    int $n = $this->rows() catch (RuntimeError $e) => null;\n    echo $n;",
    );
    assert!(diags.has_errors(), "{diags:?}");
}

/// § 3: a `throw` arm is `never`, so it produces no value and contributes
/// nothing to the union — the expression is still `int`, not `int|never`.
#[test]
fn a_throw_arm_contributes_nothing_to_the_union() {
    let diags = check_with_rows(
        "    int $n = $this->rows() catch (RuntimeError $e) => throw new LogicError(\"no\");\n\
         \x20   echo $n;",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// § 1: the arm's `$e` is a local of the enclosing function for the length of
/// the arm, and no longer — only the caught value ever assigns it, and no path
/// out of the guarded expression carries one.
#[test]
fn an_arms_binding_does_not_outlive_the_arm() {
    let diags = check_with_rows(
        "    int $n = $this->rows() catch (RuntimeError $e) => 0;\n    echo $n;\n    echo $e->message;",
    );
    assert!(diags.has_errors(), "{diags:?}");
    assert!(
        diags.iter().any(|d| d.message.contains("$e")),
        "the refusal names the binding that ended with its arm: {diags:?}",
    );
}

/// § 4's pre-guard state: an arm runs precisely when the guard did not
/// complete, so a local the guard assigned is not assumed assigned after an
/// arm that did not assign it.
#[test]
fn a_local_the_guard_assigned_is_not_live_after_the_arm() {
    let diags = check_with_rows(
        "    int $seen;\n    int $n = ($seen = $this->rows()) catch (RuntimeError $e) => 0;\n\
         \x20   echo $n;\n    echo $seen;",
    );
    assert!(diags.has_errors(), "{diags:?}");
}

/// § 5: naming the root, binding nothing, over a body that is not a `throw` —
/// *discard every failure, including the ones this site never anticipated*.
#[test]
fn an_unbound_throwable_arm_warns_w1006() {
    let diags = check_with_rows("    int $n = $this->rows() catch (Throwable) => 0;\n    echo $n;");
    assert!(warns_about_discarding(&diags), "{diags:?}");
    assert!(
        !diags.has_errors(),
        "it is an advisory, not a refusal: {diags:?}"
    );
}

/// Bound, the value is carried and the site can do something with it — which
/// is one of the two spellings the warning asks for, so it does not fire.
#[test]
fn a_bound_throwable_arm_does_not_warn() {
    let diags =
        check_with_rows("    int $n = $this->rows() catch (Throwable $e) => 0;\n    echo $n;");
    assert!(!warns_about_discarding(&diags), "{diags:?}");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A `throw` body does not discard anything — the failure leaves — so an
/// unbound `Throwable` arm over one is not the shape § 5 names.
#[test]
fn an_unbound_throwable_arm_that_rethrows_does_not_warn() {
    let diags = check_with_rows(
        "    int $n = $this->rows() catch (Throwable) => throw new LogicError(\"no\");\n\
         \x20   echo $n;",
    );
    assert!(!warns_about_discarding(&diags), "{diags:?}");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// An arm below the root is a site saying which failure it anticipated, which
/// is the other spelling § 5 asks for.
#[test]
fn an_unbound_arm_naming_a_class_below_the_root_does_not_warn() {
    let diags =
        check_with_rows("    int $n = $this->rows() catch (RuntimeError) => 0;\n    echo $n;");
    assert!(!warns_about_discarding(&diags), "{diags:?}");
    assert!(!diags.has_errors(), "{diags:?}");
}
