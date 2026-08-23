//! `&$x` arguments: which expressions are assignable through one, and which are refused.
//!
//! Moved out of `mwl_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use mwl_diagnostics::code;

// ------------------------------------------------------------------
// By-reference parameters -- the two obligations `check_by_ref_arg`
// adds on top of ordinary assignability. See `mwl_ir::Ty::Ref` for the
// representation those obligations exist to keep sound.
// ------------------------------------------------------------------

#[test]
fn a_by_reference_argument_that_is_a_local_is_accepted() {
    let diags = check_src(
        "<?mwl
class T {
  static function bump(int &$s): void { $s = $s + 1; }
           function m(): void {
int $n = 1;
T::bump($n);
  }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_by_reference_argument_that_is_a_property_is_accepted() {
    let diags = check_src(
        "<?mwl
class T {
  public int $hits;
           function constructor(int $hits) { $this->hits = $hits; }
           static function bump(int &$s): void { $s = $s + 1; }
           function m(): void {
T::bump($this->hits);
  }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_literal_passed_by_reference_is_diagnosed() {
    let diags = check_src(
        "<?mwl
class T {
  static function bump(int &$s): void { $s = $s + 1; }
           function m(): void {
T::bump(1);
  }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_BY_REF_ARG_NOT_A_PLACE)),
        "{diags:?}"
    );
}

#[test]
fn a_calls_own_result_passed_by_reference_is_diagnosed() {
    let diags = check_src(
        "<?mwl
class T {
  static function bump(int &$s): void { $s = $s + 1; }
           static function one(): int { return 1; }
           function m(): void {
T::bump(T::one());
  }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_BY_REF_ARG_NOT_A_PLACE)),
        "{diags:?}"
    );
}

/// ADR 0007 § 5's copy-on-write leaves an element no stable address, and
/// the staged-slot model has no write-back path for one either -- so this
/// is refused with a message that says which of the two it is.
#[test]
fn an_array_element_passed_by_reference_is_diagnosed() {
    let diags = check_src(
        "<?mwl
class T {
  static function bump(int &$s): void { $s = $s + 1; }
           function m(): void {
array<int> $a = [1];
T::bump($a[0]);
  }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_BY_REF_ARG_NOT_A_PLACE)),
        "{diags:?}"
    );
}

/// ADR 0014 section 1 makes a hooked read a call and a hooked write a
/// second one, so there is no slot to hand a callee.
#[test]
fn a_hooked_property_passed_by_reference_is_diagnosed() {
    let diags = check_src(
        "<?mwl
class T {
  public int $n;
           public int $doubled { get => $this->n * 2; set(int $v) { $this->n = $v; } }
           function constructor(int $n) { $this->n = $n; }
           static function bump(int &$s): void { $s = $s + 1; }
           function m(): void {
T::bump($this->doubled);
  }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_BY_REF_ARG_NOT_A_PLACE)),
        "{diags:?}"
    );
}

/// A plain `string` satisfies a `tainted string` parameter by value --
/// ADR 0024 section 2's "a plain value is always a safe
/// over-approximation of may-be-tainted" -- but never by reference: the
/// callee writes a `tainted string` back, and the caller's holder is
/// declared plain. Accepting it would launder taint through an argument
/// list, which is exactly the hole ADR 0024 exists to close, and it is
/// why assignability alone is not enough at a `&` position.
#[test]
fn a_plain_string_passed_to_a_tainted_reference_parameter_is_diagnosed() {
    let diags = check_src(
        "<?mwl
class T {
           static function fill(tainted string &$s): void { $s = $s; }
           function m(): void {
string $t = \"x\";
T::fill($t);
  }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_BY_REF_ARG_TYPE_NOT_EXACT)),
        "{diags:?}"
    );
}

/// The exactness check is reported only for an argument that would
/// otherwise have been accepted -- an outright mismatch already has
/// `E_TYPE_MISMATCH` at the same span, and two diagnostics for one
/// mistake helps nobody.
#[test]
fn an_unassignable_by_reference_argument_is_diagnosed_only_once() {
    let diags = check_src(
        "<?mwl
class T {
  static function bump(int &$s): void { $s = $s + 1; }
           function m(): void {
string $t = \"x\";
T::bump($t);
  }
}
",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_BY_REF_ARG_TYPE_NOT_EXACT)),
        "{diags:?}"
    );
}
