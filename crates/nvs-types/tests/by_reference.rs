//! `inout` arguments: which expressions are assignable through one, which are
//! refused, and `rule:statements/inout-is-written-at-the-call`'s marker at both ends of the same call.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// ------------------------------------------------------------------
// By-reference parameters -- the two obligations `check_inout_arg`
// adds on top of ordinary assignability. See `nvs_ir::Ty::Ref` for the
// representation those obligations exist to keep sound.
// ------------------------------------------------------------------

#[test]
fn a_by_reference_argument_that_is_a_local_is_accepted() {
    let diags = check_src(
        "<?nvs
class T {
  static function bump(inout int $s): void { $s = $s + 1; }
           function m(): void {
int $n = 1;
T::bump(inout $n);
  }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_by_reference_argument_that_is_a_property_is_accepted() {
    let diags = check_src(
        "<?nvs
class T {
  public int $hits;
           function constructor(int $hits) { $this->hits = $hits; }
           static function bump(inout int $s): void { $s = $s + 1; }
           function m(): void {
T::bump(inout $this->hits);
  }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_literal_passed_by_reference_is_diagnosed() {
    let diags = check_src(
        "<?nvs
class T {
  static function bump(inout int $s): void { $s = $s + 1; }
           function m(): void {
T::bump(inout 1);
  }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INOUT_ARG_NOT_A_PLACE)),
        "{diags:?}"
    );
}

#[test]
fn a_calls_own_result_passed_by_reference_is_diagnosed() {
    let diags = check_src(
        "<?nvs
class T {
  static function bump(inout int $s): void { $s = $s + 1; }
           static function one(): int { return 1; }
           function m(): void {
T::bump(inout T::one());
  }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INOUT_ARG_NOT_A_PLACE)),
        "{diags:?}"
    );
}

/// `rule:types/arrays`'s copy-on-write leaves an element no stable address, and
/// the staged-slot model has no write-back path for one either -- so this
/// is refused with a message that says which of the two it is, and refused
/// permanently (`check_inout_arg`'s own docs, findings.md D22).
#[test]
fn an_array_element_passed_by_reference_is_diagnosed() {
    let diags = check_src(
        "<?nvs
class T {
  static function bump(inout int $s): void { $s = $s + 1; }
           function m(): void {
array<int> $a = [1];
T::bump(inout $a[0]);
  }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INOUT_ARG_NOT_A_PLACE)),
        "{diags:?}"
    );
}

/// `rule:classes/property-observer` section 1 makes a hooked read a call and a hooked write a
/// second one, so there is no slot to hand a callee.
#[test]
fn a_hooked_property_passed_by_reference_is_diagnosed() {
    let diags = check_src(
        "<?nvs
class T {
  public int $n;
           public int $doubled { get => $this->n * 2; set(int $v) { $this->n = $v; } }
           function constructor(int $n) { $this->n = $n; }
           static function bump(inout int $s): void { $s = $s + 1; }
           function m(): void {
T::bump(inout $this->doubled);
  }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INOUT_ARG_NOT_A_PLACE)),
        "{diags:?}"
    );
}

/// A plain `string` satisfies a `tainted string` parameter by value --
/// `rule:security/tainted-qualifier` section 2's "a plain value is always a safe
/// over-approximation of may-be-tainted" -- but never by reference: the
/// callee writes a `tainted string` back, and the caller's holder is
/// declared plain. Accepting it would launder taint through an argument
/// list, which is exactly the hole `rule:security/tainted-qualifier` exists to close, and it is
/// why assignability alone is not enough at an `inout` position.
#[test]
fn a_plain_string_passed_to_a_tainted_reference_parameter_is_diagnosed() {
    let diags = check_src(
        "<?nvs
class T {
           static function fill(inout tainted string $s): void { $s = $s; }
           function m(): void {
string $t = \"x\";
T::fill(inout $t);
  }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INOUT_ARG_TYPE_NOT_EXACT)),
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
        "<?nvs
class T {
  static function bump(inout int $s): void { $s = $s + 1; }
           function m(): void {
string $t = \"x\";
T::bump(inout $t);
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
            .any(|d| d.code == Some(code::E_INOUT_ARG_TYPE_NOT_EXACT)),
        "{diags:?}"
    );
}

// ------------------------------------------------------------------
// `rule:statements/inout-is-the-by-reference-spelling` section 2 -- the call-site marker, both directions. This is
// the half a rename alone would not have bought: the write a callee
// makes to its caller's storage is visible at the point of call.
// ------------------------------------------------------------------

#[test]
fn an_inout_argument_is_required_where_the_parameter_declares_one() {
    // Positionally, and by name -- the marker is on the binding, so it
    // sits outside `name:` rather than replacing it.
    for call in ["T::bump($n)", "T::bump(s: $n)"] {
        let diags = check_src(&format!(
            "<?nvs
class T {{
  static function bump(inout int $s): void {{ $s = $s + 1; }}
           function m(): void {{
int $n = 1;
{call};
  }}
}}
"
        ));
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_INOUT_ARG_MISSING)),
            "{call}: {diags:?}"
        );
    }
}

#[test]
fn an_inout_argument_is_refused_where_the_parameter_is_by_value() {
    let diags = check_src(
        "<?nvs
class T {
  static function keep(int $s): int { return $s; }
           function m(): void {
int $n = 1;
T::keep(inout $n);
  }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INOUT_ARG_UNEXPECTED)),
        "{diags:?}"
    );

    // Through a `callable` it can never be right: `rule:types/callable-is-the-only-function-type` refuses the
    // declaration end outright, so nothing the call reaches can bind one.
    let diags = check_src(
        "<?nvs
class T {
           function m(): void {
int $n = 1;
callable $f = fn (int $x): int => $x + 1;
$f(inout $n);
  }
}
",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INOUT_ARG_UNEXPECTED)),
        "{diags:?}"
    );
}
