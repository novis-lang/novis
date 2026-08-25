//! ADR 0094's levels, enforced: which class a `private`/`protected` member is
//! reachable *from*.
//!
//! Every case here turns on where the access is written and never on what the
//! receiver holds — `crate::signatures::is_visible_from` is the rule, and
//! `expr::members::check_member_visibility` the one place it is applied. See
//! `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use mwl_diagnostics::code;

#[test]
fn a_public_property_is_reachable_from_another_class() {
    let diags = check_src(
        "<?mwl\nclass Open {\n  public int $n = 0;\n}\nclass Other {\n  function m(Open $o): void {\n    int $x = $o->n;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_private_property_is_unreachable_from_another_class() {
    let diags = check_src(
        "<?mwl\nclass Secret {\n  private int $n = 0;\n}\nclass Other {\n  function m(Secret $s): void {\n    int $x = $s->n;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_MEMBER_NOT_VISIBLE)),
        "{diags:?}"
    );
}

#[test]
fn a_private_property_is_reachable_from_another_instance_of_its_own_class() {
    // The test is keyed on the *accessing* class, so a second instance of the
    // same class is as reachable as `$this` — PHP's rule, and the reason the
    // receiver's static type is not what decides this.
    let diags = check_src(
        "<?mwl\nclass Secret {\n  private int $n = 0;\n  function m(Secret $other): void {\n    int $x = $other->n;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_private_property_is_unreachable_at_file_scope() {
    let diags = check_src(
        "<?mwl\nclass Secret {\n  private int $n = 0;\n}\nSecret $s = new Secret();\nint $x = $s->n;\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_MEMBER_NOT_VISIBLE)),
        "{diags:?}"
    );
}

#[test]
fn a_protected_property_is_reachable_from_a_subclass() {
    let diags = check_src(
        "<?mwl\nclass Base {\n  protected int $n = 0;\n}\nclass Child extends Base {\n  function m(): void {\n    int $x = $this->n;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_protected_property_is_unreachable_from_an_unrelated_class() {
    let diags = check_src(
        "<?mwl\nclass Base {\n  protected int $n = 0;\n}\nclass Other {\n  function m(Base $b): void {\n    int $x = $b->n;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_MEMBER_NOT_VISIBLE)),
        "{diags:?}"
    );
}

#[test]
fn a_parents_private_property_is_unreachable_from_its_subclass() {
    // `resolve_property_owned` finds the declaration by walking `extends`, so
    // the name resolves — and is then refused, because `private` is scoped to
    // the declaring class and not to the chain that inherits its storage.
    let diags = check_src(
        "<?mwl\nclass Base {\n  private int $n = 0;\n}\nclass Child extends Base {\n  function m(): void {\n    int $x = $this->n;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_MEMBER_NOT_VISIBLE)),
        "{diags:?}"
    );
}

#[test]
fn a_write_to_a_private_property_from_another_class_is_refused_too() {
    // A write reaches the member through the same `PropertyAccess` expression
    // a read does (`crate::expr_table`'s docs: one span, read and write
    // alike), so the level test needs no separate assignment arm.
    let diags = check_src(
        "<?mwl\nclass Secret {\n  private int $n = 0;\n}\nclass Other {\n  function m(Secret $s): void {\n    $s->n = 1;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_MEMBER_NOT_VISIBLE)),
        "{diags:?}"
    );
}

#[test]
fn a_private_static_property_is_unreachable_from_another_class() {
    let diags = check_src(
        "<?mwl\nclass Secret {\n  private static int $n = 0;\n}\nclass Other {\n  function m(): void {\n    int $x = Secret::$n;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_MEMBER_NOT_VISIBLE)),
        "{diags:?}"
    );
}

#[test]
fn a_private_static_property_is_reachable_through_self() {
    let diags = check_src(
        "<?mwl\nclass Secret {\n  private static int $n = 0;\n  function m(): void {\n    int $x = self::$n;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}
