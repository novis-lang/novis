//! `rule:iteration/concrete-generic-implements`'s one narrow generic door — a user class implementing a compiler-owned generic interface at a concrete type.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// `rule:iteration/two-interfaces` and `rule:iteration/concrete-generic-implements`: the two compiler-owned generic interfaces, and the one
// door user code has onto a type parameter.

#[test]
fn a_class_may_implement_a_compiler_owned_generic_interface_at_a_concrete_type() {
    let diags = check_src(
        "<?nvs\n\
         class Counter implements Iterable<int> {\n\
         \x20 function iterate(): Iterator<int> { return Counter::empty(); }\n\
         \x20 static function empty(): Iterator<int> { return Counter::empty(); }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_cursor_s_element_type_comes_from_the_receiver_s_type_argument() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(Iterator<int> $it): void {\n\
         \x20\x20 bool $more = $it->advance();\n\
         \x20\x20 int $n = $it->current();\n\
         \x20\x20 echo $n . $more;\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The half that proves the argument is actually *used*: the same body
/// against `Iterator<string>` must not type-check.
#[test]
fn a_cursor_s_element_type_is_not_mixed() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(Iterator<string> $it): void {\n\
         \x20\x20 int $n = $it->current();\n\
         \x20\x20 echo $n;\n\
         \x20 }\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn a_user_declared_name_written_with_type_arguments_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class Animal {}\n\
         class T {\n\
         \x20 function m(Animal<int> $a): void { echo \"x\"; }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_ARGS_NOT_GENERIC)),
        "{diags:?}"
    );
}

/// A reserved interface that takes *no* parameters is refused by the same
/// rule, in the position `rule:iteration/concrete-generic-implements` opened -- an `implements` clause.
#[test]
fn a_non_generic_reserved_interface_takes_no_type_arguments_either() {
    let diags = check_src("<?nvs\nclass M implements Comparable<int> {}\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_ARGS_NOT_GENERIC)),
        "{diags:?}"
    );
}

#[test]
fn a_generic_interface_written_bare_is_refused() {
    let diags =
        check_src("<?nvs\nclass T {\n  function m(Iterator $it): void { echo \"x\"; }\n}\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_ARG_COUNT)),
        "{diags:?}"
    );
}

#[test]
fn a_generic_interface_written_with_too_many_arguments_is_refused() {
    let diags = check_src("<?nvs\nclass C implements Iterable<int, string> {}\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_ARG_COUNT)),
        "{diags:?}"
    );
}

/// `rule:statements/nothing-gets-a-second-name` gives a `type` alias no parameters of its own, so an alias
/// written with arguments lands on the same refusal a class does -- while
/// an alias used *as* an argument still expands.
#[test]
fn a_type_alias_has_no_type_parameters_but_may_be_one() {
    let diags = check_src(
        "<?nvs\n\
         type Id = int;\n\
         class T {\n\
         \x20 function ok(Iterator<Id> $it): void { int $n = $it->current(); echo $n; }\n\
         \x20 function bad(Id<int> $x): void { echo \"x\"; }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_ARGS_NOT_GENERIC)),
        "{diags:?}"
    );
    assert!(
        !diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// A parameter whose declared type mentions no variable is known before any
/// binding, so it is checked *with* that expectation — which is what tells an
/// integer literal at a `uint` position that it is one (`rule:types/arithmetic`). Before
/// `nvs_types::expr::args::check_generic_args` did that, a `uint` parameter on a
/// generic `Core` member was unreachable from a literal, while the identical
/// parameter on a non-generic one (`Core\Str::padStart`) accepted it.
#[test]
fn an_int_literal_reaches_a_uint_parameter_of_a_generic_core_member() {
    let diags = check_src(
        "<?nvs\n\
         array<string> $a = [\"one\"];\n\
         array<string> $p = Core\\Arr::padEnd($a, 3, \"-\");\n\
         echo Core\\Arr::count($p);\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The call-site half of the same wall. `docs/spec/01-core-library.md` § 6
/// writes `decodeAs<T>` because no parameter position holds `T`; every other
/// `Core` member infers its variables from the arguments, so writing them is
/// a second spelling of a fact those arguments already settle.
#[test]
fn a_core_member_that_infers_its_variables_refuses_a_written_type_argument() {
    let diags = check_src(
        "<?nvs\n\
         array<string> $a = [\"one\"];\n\
         echo Core\\Arr::count<string>($a);\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_ARGS_NOT_GENERIC)),
        "{diags:?}"
    );
}

/// A user-declared method is the same refusal from the other side: `rule:types/declaration` parks user-declared generics, so nothing a program writes has a type
/// parameter to name.
#[test]
fn a_user_declared_method_refuses_a_written_type_argument() {
    let diags = check_src(
        "<?nvs\n\
         class Box {\n\
         \x20 static function of(int $n): int { return $n; }\n\
         }\n\
         echo Box::of<int>(1);\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_ARGS_NOT_GENERIC)),
        "{diags:?}"
    );
}

/// The refusal reports the list and stops there: the call itself is still
/// typed from the member's own signature, so one mistake yields one
/// diagnostic rather than a cascade off an unresolved return type.
#[test]
fn a_refused_type_argument_list_does_not_cascade() {
    let diags = check_src(
        "<?nvs\n\
         array<string> $a = [\"one\"];\n\
         uint $n = Core\\Arr::count<string>($a);\n\
         echo $n;\n",
    );
    assert!(
        !diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// The third position a `<...>` may be written in: between a `new` target and
/// its `(`. The grammar reads one there now (`ExprKind::New::type_args`), and
/// a user-declared class refuses it by the same rule as a type position --
/// user-declared generic classes stay deferred.
#[test]
fn a_new_target_written_with_type_arguments_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class Box {}\n\
         var $b = new Box<int>();\n\
         echo \"x\";\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_ARGS_NOT_GENERIC)),
        "{diags:?}"
    );
}

/// A class named *inside* a refused list is still resolved, so a typo there is
/// reported rather than swallowed by the refusal -- the rule
/// `check_written_type_args` already applies to a call site's own list.
#[test]
fn a_class_named_inside_a_refused_new_type_argument_list_is_still_resolved() {
    let diags = check_src(
        "<?nvs\n\
         class Box {}\n\
         var $b = new Box<Nope>();\n\
         echo \"x\";\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_CLASS)),
        "{diags:?}"
    );
}

// `nvs_stdlib::registry::GENERIC_CLASSES` — the roster that says which `new`
// target may carry a list at all, and how many arguments it takes.

/// The accepting half: spec § 9's two collections, each written with exactly
/// the arguments its roster row declares.
#[test]
fn a_core_owned_generic_class_takes_the_arguments_its_roster_row_declares() {
    let diags = check_src(
        "<?nvs\n\
         class Tag {}\n\
         var $set = new Core\\ObjectSet<Tag>();\n\
         var $map = new Core\\ObjectMap<Tag, int>();\n\
         echo \"x\";\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The case that proves the roster is read positionally rather than as a
/// yes/no: the same list is right for one class and wrong for the other, so
/// nothing but the declared arity can tell them apart.
#[test]
fn a_core_owned_generic_class_written_with_another_s_arity_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class Tag {}\n\
         var $set = new Core\\ObjectSet<Tag, int>();\n\
         var $map = new Core\\ObjectMap<Tag>();\n\
         echo \"x\";\n",
    );
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.code == Some(code::E_TYPE_ARG_COUNT))
            .count(),
        2,
        "{diags:?}"
    );
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_ARGS_NOT_GENERIC)),
        "{diags:?}"
    );
}

/// "Including none at all" — a `Core` collection infers nothing from its
/// constructor, so the bare spelling is a wrong count rather than a default.
#[test]
fn a_core_owned_generic_class_written_bare_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         var $set = new Core\\ObjectSet();\n\
         echo \"x\";\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_ARG_COUNT)),
        "{diags:?}"
    );
}

/// A class named inside an *accepted* list is resolved too, so the roster
/// does not become a hole the refusal used to cover.
#[test]
fn a_class_named_inside_an_accepted_new_type_argument_list_is_still_resolved() {
    let diags = check_src(
        "<?nvs\n\
         var $set = new Core\\ObjectSet<Nope>();\n\
         echo \"x\";\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_CLASS)),
        "{diags:?}"
    );
}
