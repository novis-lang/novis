//! `rule:core-api/written-visibility`'s levels, enforced: which class a `private`/`protected` member is
//! reachable *from*.
//!
//! Every case here turns on where the access is written and never on what the
//! receiver holds — `crate::signatures::is_visible_from` is the rule, and
//! `expr::members::check_member_visibility` the one place it is applied. See
//! `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

/// The name an acceptance check in the goal records under `data/goals/` runs,
/// so it covers the whole rule in one source: both halves of a member, read
/// from a class that is neither the declaring one nor a subclass of it. The
/// cases below take the same rule apart one shape at a time.
#[test]
fn a_private_member_is_refused_outside_its_class() {
    let diags = check_src(
        "<?nvs\nclass Vault {\n  private int $balance = 0;\n  private function audit(): int { return $this->balance; }\n}\nclass Thief {\n  function take(Vault $v): int {\n    int $seen = $v->balance;\n    return $seen + $v->audit();\n  }\n}\n",
    );
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.code == Some(code::E_MEMBER_NOT_VISIBLE))
            .count(),
        2,
        "{diags:?}"
    );
}

#[test]
fn a_public_property_is_reachable_from_another_class() {
    let diags = check_src(
        "<?nvs\nclass Open {\n  public int $n = 0;\n}\nclass Other {\n  function m(Open $o): void {\n    int $x = $o->n;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_private_property_is_unreachable_from_another_class() {
    let diags = check_src(
        "<?nvs\nclass Secret {\n  private int $n = 0;\n}\nclass Other {\n  function m(Secret $s): void {\n    int $x = $s->n;\n  }\n}\n",
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
        "<?nvs\nclass Secret {\n  private int $n = 0;\n  function m(Secret $other): void {\n    int $x = $other->n;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_private_property_is_unreachable_at_file_scope() {
    let diags = check_src(
        "<?nvs\nclass Secret {\n  private int $n = 0;\n}\nSecret $s = new Secret();\nint $x = $s->n;\n",
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
        "<?nvs\nclass Base {\n  protected int $n = 0;\n}\nclass Child extends Base {\n  function m(): void {\n    int $x = $this->n;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_protected_property_is_unreachable_from_an_unrelated_class() {
    let diags = check_src(
        "<?nvs\nclass Base {\n  protected int $n = 0;\n}\nclass Other {\n  function m(Base $b): void {\n    int $x = $b->n;\n  }\n}\n",
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
        "<?nvs\nclass Base {\n  private int $n = 0;\n}\nclass Child extends Base {\n  function m(): void {\n    int $x = $this->n;\n  }\n}\n",
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
        "<?nvs\nclass Secret {\n  private int $n = 0;\n}\nclass Other {\n  function m(Secret $s): void {\n    $s->n = 1;\n  }\n}\n",
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
        "<?nvs\nclass Secret {\n  private static int $n = 0;\n}\nclass Other {\n  function m(): void {\n    int $x = Secret::$n;\n  }\n}\n",
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
        "<?nvs\nclass Secret {\n  private static int $n = 0;\n  function m(): void {\n    int $x = self::$n;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_private_method_is_unreachable_from_another_class() {
    let diags = check_src(
        "<?nvs\nclass Secret {\n  private function h(): int { return 1; }\n}\nclass Other {\n  function m(Secret $s): void {\n    int $x = $s->h();\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_MEMBER_NOT_VISIBLE)),
        "{diags:?}"
    );
}

#[test]
fn a_private_method_is_reachable_from_another_instance_of_its_own_class() {
    let diags = check_src(
        "<?nvs\nclass Secret {\n  private function h(): int { return 1; }\n  function m(Secret $other): void {\n    int $x = $other->h();\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_protected_method_is_reachable_from_a_subclass() {
    let diags = check_src(
        "<?nvs\nclass Base {\n  protected function h(): int { return 1; }\n}\nclass Child extends Base {\n  function m(): void {\n    int $x = $this->h();\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_protected_method_is_unreachable_from_an_unrelated_class() {
    let diags = check_src(
        "<?nvs\nclass Base {\n  protected function h(): int { return 1; }\n}\nclass Other {\n  function m(Base $b): void {\n    int $x = $b->h();\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_MEMBER_NOT_VISIBLE)),
        "{diags:?}"
    );
}

#[test]
fn a_private_static_method_is_unreachable_from_another_class_but_reachable_through_self() {
    let refused = check_src(
        "<?nvs\nclass Secret {\n  private static function h(): int { return 1; }\n}\nclass Other {\n  function m(): void {\n    int $x = Secret::h();\n  }\n}\n",
    );
    assert!(
        refused
            .iter()
            .any(|d| d.code == Some(code::E_MEMBER_NOT_VISIBLE)),
        "{refused:?}"
    );
    let allowed = check_src(
        "<?nvs\nclass Secret {\n  private static function h(): int { return 1; }\n  function m(): void {\n    int $x = self::h();\n  }\n}\n",
    );
    assert!(!allowed.has_errors(), "{allowed:?}");
}

#[test]
fn a_private_constructor_refuses_new_from_outside_and_allows_it_inside() {
    // PHP's singleton idiom: the whole point of writing one is that `new` is
    // refused everywhere except the class's own bodies, so `new` takes the
    // same test a call does rather than being skipped for having no member
    // name written at the site.
    let refused = check_src(
        "<?nvs\nclass Solo {\n  private function constructor() {}\n}\nSolo $s = new Solo();\n",
    );
    assert!(
        refused
            .iter()
            .any(|d| d.code == Some(code::E_MEMBER_NOT_VISIBLE)),
        "{refused:?}"
    );
    let allowed = check_src(
        "<?nvs\nclass Solo {\n  private function constructor() {}\n  static function make(): Solo {\n    return new Solo();\n  }\n}\n",
    );
    assert!(!allowed.has_errors(), "{allowed:?}");
}

#[test]
fn a_private_interface_method_is_refused_once_by_the_adr_0043_diagnostic() {
    // The two rules overlap exactly on this shape — see
    // `expr::members::check_method_visibility` for why only the more specific
    // one is reported.
    let diags = check_src(
        "<?nvs\ninterface Csv {\n  private function escape(string $f): string { return $f; }\n}\nclass Writer implements Csv {\n  function m(): void {\n    string $x = $this->escape(\"a\");\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE)),
        "{diags:?}"
    );
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_MEMBER_NOT_VISIBLE)),
        "{diags:?}"
    );
}
