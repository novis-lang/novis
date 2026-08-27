//! A `Core` member call — `nvs_stdlib::registry`'s signatures reached through the ordinary call path, including ADR 0063 R2's options bag.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

/// A `Core` member resolves through the signature table
/// `crate::core_lib` seeded, so its return type reaches the binding it is
/// assigned to -- `array<T> -> uint` with `T` bound from the argument.
#[test]
fn a_core_member_call_type_checks_against_its_registered_signature() {
    let diags =
        check_in_method("array<int> $a = [1, 2];\nuint $n = Core\\Arr::count($a);\necho $n;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The point of registering a signature at all: a `Core` call is now
/// arity-checked exactly like a user-declared one, where before every
/// `Core\...` reference was trusted unchecked.
#[test]
fn a_core_member_call_with_the_wrong_arity_is_diagnosed() {
    let diags = check_in_method("array<int> $a = [1];\nuint $n = Core\\Arr::count($a, 2);\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_ARITY_MISMATCH)),
        "{diags:?}"
    );
}

/// ADR 0063 R2's options bag at a call site: written, and omitted whole.
/// `Core\Arr::range`'s `{step?: int}` is the first one in the roster, and
/// `MethodSig::required()` has to say 2 either way — the bag is optional
/// by construction, so nothing about the arity check changed to allow it.
#[test]
fn an_options_bag_may_be_written_or_omitted() {
    let diags = check_in_method(
        "array<int> $a = Core\\Arr::range(1, 5);\n\
         array<int> $b = Core\\Arr::range(1, 5, {step: 2});\n\
         echo Core\\Arr::count($a), Core\\Arr::count($b);\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// `Core\Arr::hasKey(array<T> $a, int|string $key)` — the first `Core`
/// signature with a union parameter, checked by ADR 0007 § 6's ordinary
/// union rule with nothing added for `Core`.
#[test]
fn a_core_union_parameter_takes_either_member_and_nothing_else() {
    let diags = check_in_method(
        "array<int> $a = [\"x\" => 1];\n\
         bool $byName = Core\\Arr::hasKey($a, \"x\");\n\
         bool $byIndex = Core\\Arr::hasKey($a, 0);\n\
         echo $byName, $byIndex;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    let refused =
        check_in_method("array<int> $a = [\"x\" => 1];\nbool $b = Core\\Arr::hasKey($a, 1.5);\n");
    assert!(
        refused
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{refused:?}"
    );
}

/// `Core\Arr::map(array<T> $a, callable $fn): array<U>` — the `U` binds
/// from the `fn` literal's own return type, so the result is `array<string>`
/// and reaches `Core\Str::join`, which takes exactly that.
/// `crate::generics` owns the rule; this is it end to end through the
/// checker.
#[test]
fn a_callback_result_binds_the_members_result_element_type() {
    let inferred = check_in_method(
        "array<int> $a = [1, 2];\n\
         array<string> $out = Core\\Arr::map($a, fn(int $n) => \"n\" . $n);\n\
         echo Core\\Str::join($out, \",\");\n",
    );
    assert!(!inferred.has_errors(), "{inferred:?}");

    // The declared-return spelling of the same closure binds identically —
    // `ExprInfo::Closure`'s `return_ty` is the declared type where one is
    // written and the inferred one where it is not.
    let declared = check_in_method(
        "array<int> $a = [1, 2];\n\
         array<string> $out = Core\\Arr::map($a, fn(int $n): string => \"n\" . $n);\n\
         echo Core\\Str::join($out, \",\");\n",
    );
    assert!(!declared.has_errors(), "{declared:?}");

    // And it is a real binding, not a widening: a callback answering `int`
    // makes the result `array<int>`, which an `array<string>` binding
    // refuses.
    let wrong = check_in_method(
        "array<int> $a = [1, 2];\n\
         array<string> $out = Core\\Arr::map($a, fn(int $n): int => $n);\n",
    );
    assert!(
        wrong.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{wrong:?}"
    );
}

/// The gap `crate::generics` records: only a written `fn` literal has a
/// recorded return type to bind from, so a callable reached through a
/// variable leaves the result `array<mixed>` — honest, and diagnosed at the
/// point it is used as something narrower rather than silently accepted.
#[test]
fn a_callback_that_is_not_a_literal_leaves_the_result_unbound() {
    let diags = check_in_method(
        "array<int> $a = [1, 2];\n\
         var $fn = fn(int $n): string => \"n\" . $n;\n\
         array<string> $out = Core\\Arr::map($a, $fn);\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// A `Ty::CallableTo` parameter accepts exactly what a `callable` one
/// accepts — it is a binding site, not a constraint — and refuses what a
/// `callable` refuses.
#[test]
fn a_callback_result_parameter_still_accepts_any_callable() {
    let ok = check_in_method(
        "array<int> $a = [1, 2];\n\
         var $fn = fn(int $n): string => \"n\" . $n;\n\
         var $out = Core\\Arr::map($a, $fn);\n\
         echo Core\\Arr::count($out);\n",
    );
    assert!(!ok.has_errors(), "{ok:?}");

    let refused = check_in_method("array<int> $a = [1];\nvar $out = Core\\Arr::map($a, 7);\n");
    assert!(refused.has_errors(), "{refused:?}");
}

/// The rule that makes a bag its own type rather than an ADR 0036 shape:
/// a field the member does not declare is an error, where § 3's width
/// subtyping would have accepted it silently. The help names the real
/// options, which is the whole value of catching the typo here.
#[test]
fn an_unknown_option_is_diagnosed_and_the_real_ones_are_named() {
    let diags = check_in_method("array<int> $a = Core\\Arr::range(1, 5, {stepp: 2});\n");
    let unknown = diags
        .iter()
        .find(|d| d.code == Some(code::E_UNKNOWN_OPTION))
        .unwrap_or_else(|| panic!("{diags:?}"));
    assert!(
        unknown.notes.iter().any(|note| note.contains("step")),
        "{unknown:?}"
    );
}

/// An option's value is checked against the option's own declared type,
/// through the same assignability rule every other argument goes through.
#[test]
fn an_option_value_is_checked_against_its_declared_type() {
    let diags = check_in_method("array<int> $a = Core\\Arr::range(1, 5, {step: \"two\"});\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// A bag flattens per-option at the call site, so there is nothing to read
/// a variable's fields out of — the one restriction the design costs, and
/// it is a diagnostic rather than silence.
#[test]
fn an_options_argument_that_is_not_a_literal_is_diagnosed() {
    let diags =
        check_in_method("var $bag = {step: 2};\narray<int> $a = Core\\Arr::range(1, 5, $bag);\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_OPTIONS_NOT_A_LITERAL)),
        "{diags:?}"
    );
}

/// Flattening takes the first field of a given name, so a repeated option
/// would silently drop the second — diagnosed instead.
#[test]
fn a_repeated_option_is_diagnosed() {
    let diags = check_in_method("array<int> $a = Core\\Arr::range(1, 5, {step: 1, step: 2});\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DUPLICATE_DECLARATION)),
        "{diags:?}"
    );
}

/// The bag is not a shape *target* either: `{...}` written anywhere else
/// still means ADR 0036's anonymous object, width subtyping and all, so
/// this change is scoped to the one parameter position it describes.
#[test]
fn an_object_literal_outside_an_options_position_is_still_a_shape() {
    let diags = check_in_method("var $point = {x: 1, y: 2};\necho $point->x;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_call_may_omit_a_parameter_that_has_a_default() {
    let diags = check_src(
        "<?nvs\nclass Box {\n  static function scale(int $n, int $by = 3): int { return $n * $by; }\n\
         \n  function m(): void { echo Box::scale(5); echo Box::scale(5, 2); }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_call_that_omits_a_required_parameter_is_still_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass Box {\n  static function scale(int $n, int $by = 3): int { return $n * $by; }\n\
         \n  function m(): void { echo Box::scale(); }\n}\n",
    );
    let arity = diags
        .iter()
        .find(|d| d.code == Some(code::E_ARITY_MISMATCH))
        .unwrap_or_else(|| panic!("{diags:?}"));
    // The range, not a single number: a bare "expected 2" would be wrong
    // now that one of the two is optional.
    assert!(arity.message.contains("1 to 2"), "{:?}", arity.message);
}

#[test]
fn a_call_passing_more_arguments_than_parameters_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass Box {\n  static function scale(int $n, int $by = 3): int { return $n * $by; }\n\
         \n  function m(): void { echo Box::scale(1, 2, 3); }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_ARITY_MISMATCH)),
        "{diags:?}"
    );
}

/// An argument that cannot bind the variable at all: `T` stays unbound and
/// substitutes to `mixed`, so the parameter reads `array<mixed>` and an
/// `int` still fails it. See `crate::generics` for that rule.
#[test]
fn a_core_member_call_with_a_wrongly_typed_argument_is_diagnosed() {
    let diags = check_in_method("uint $n = Core\\Arr::count(7);\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// A type variable never survives the call site, so the assignment below
/// is checked against the concrete `uint` the signature declares.
#[test]
fn a_core_member_call_assigned_to_the_wrong_type_is_diagnosed() {
    let diags = check_in_method("array<int> $a = [1];\nstring $n = Core\\Arr::count($a);\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// A `Core` class the registry does not name stays trusted rather than
/// becoming an error -- `crate::core_lib`'s own docs own why, and name
/// removing that trust as what completing the registry buys.
#[test]
fn an_unregistered_core_reference_is_still_trusted() {
    let diags = check_in_method("Core\\Str::upper(\"x\");\n");
    assert!(!diags.has_errors(), "{diags:?}");
}
