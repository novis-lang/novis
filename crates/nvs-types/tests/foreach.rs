//! `rule:iteration/foreach-subjects`'s three accepted `foreach` subjects, and the element type each binds.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// `rule:iteration/foreach-subjects`: what `foreach` accepts, and what it yields.

#[test]
fn a_foreach_over_an_array_binds_the_element_type() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(array<int> $a): void {\n\
         \x20\x20 foreach ($a as int $n) { echo $n; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_foreach_binding_that_disagrees_with_the_element_type_is_diagnosed() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(array<int> $a): void {\n\
         \x20\x20 foreach ($a as string $s) { echo $s; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn a_foreach_over_a_cursor_binds_its_type_argument() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(Iterator<int> $it): void {\n\
         \x20\x20 foreach ($it as int $n) { echo $n; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The element type reaches the loop through the class's own
/// `implements Iterable<int>` clause, not through the subject's written
/// type -- which is the whole reason `ClassSignature::implements` records
/// the resolved argument.
#[test]
fn a_foreach_over_a_class_reaches_its_implements_clause_for_the_element_type() {
    let diags = check_src(
        "<?nvs\n\
         class Counter implements Iterable<int> {\n\
         \x20 function iterate(): Iterator<int> { return Counter::empty(); }\n\
         \x20 static function empty(): Iterator<int> { return Counter::empty(); }\n\
         }\n\
         class T {\n\
         \x20 function m(Counter $c): void {\n\
         \x20\x20 foreach ($c as string $s) { echo $s; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn a_foreach_over_a_class_that_implements_neither_interface_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class Bag {}\n\
         class T {\n\
         \x20 function m(Bag $b): void {\n\
         \x20\x20 foreach ($b as int $n) { echo $n; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_FOREACH_SUBJECT_NOT_ITERABLE)),
        "{diags:?}"
    );
}

#[test]
fn a_foreach_over_a_scalar_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(int $n): void {\n\
         \x20\x20 foreach ($n as int $x) { echo $x; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_FOREACH_SUBJECT_NOT_ITERABLE)),
        "{diags:?}"
    );
}

#[test]
fn a_key_binding_over_a_cursor_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(Iterator<int> $it): void {\n\
         \x20\x20 foreach ($it as int $k => int $n) { echo $k . $n; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_FOREACH_KEY_ON_CURSOR)),
        "{diags:?}"
    );
}

/// `rule:types/arrays` gives an `array<T>` one stored key type, and it is the one
/// this binding declares -- see `crate::expr::check_foreach_key`.
#[test]
fn a_string_key_binding_over_an_array_is_accepted() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(array<int> $a): void {\n\
         \x20\x20 foreach ($a as string $k => int $n) { echo $k . $n; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// Every other declared key type is always wrong rather than unprovable:
/// no array can produce one. `rule:types/arrays`.
#[test]
fn a_non_string_key_binding_over_an_array_is_refused() {
    for key_ty in ["int", "uint", "mixed", "int|string"] {
        let diags = check_src(&format!(
            "<?nvs\n\
             class T {{\n\
             \x20 function m(array<int> $a): void {{\n\
             \x20\x20 foreach ($a as {key_ty} $k => int $n) {{ echo $n; }}\n\
             \x20 }}\n\
             }}\n"
        ));
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_FOREACH_KEY_TY)),
            "{key_ty}: {diags:?}"
        );
    }
}

// `rule:iteration/keyed-interfaces`: a loop with a key binding over the keyed
// pair binds its `K` and its `V`.

/// A small class to key a map on, and a keyed cursor a user class can return.
const KEYED_FIXTURES: &str = "class Tag {}\n\
     class PairCursor implements KeyedIterator<string, int> {\n\
     \x20 public function advance(): bool { return false; }\n\
     \x20 public function key(): string { return \"a\"; }\n\
     \x20 public function current(): int { return 0; }\n\
     }\n";

/// `body` is the method body of a method taking `$map` and `$taint`.
fn check_over_maps(body: &str) -> nvs_diagnostics::Diagnostics {
    check_src(&format!(
        "<?nvs\n{KEYED_FIXTURES}\
         class T {{\n\
         \x20 public function m(Core\\ObjectMap<Tag, int> $map, \
         Core\\ObjectMap<tainted string, int> $taint): void {{\n{body}\n\
         \x20 }}\n\
         }}\n"
    ))
}

fn has(diags: &nvs_diagnostics::Diagnostics, code: nvs_diagnostics::Code) -> bool {
    diags.iter().any(|d| d.code == Some(code))
}

// covers: lang:iteration/keys-and-values-from-a-map
#[test]
fn a_key_binding_over_an_object_map_binds_the_maps_key_and_value_types() {
    let diags = check_over_maps(
        "foreach ($map as Tag $key => int $value) { Tag $t = $key; int $n = $value; }",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_var_key_over_an_object_map_takes_the_maps_key_type() {
    let diags = check_over_maps(
        "foreach ($map as var $key => var $value) { Tag $t = $key; int $n = $value; }",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let diags = check_over_maps("foreach ($map as var $key => var $value) { int $n = $key; }");
    assert!(has(&diags, code::E_TYPE_MISMATCH), "{diags:?}");
}

#[test]
fn a_var_key_over_a_tainted_key_type_is_tainted_string() {
    let diags =
        check_over_maps("foreach ($taint as var $key => var $value) { tainted string $s = $key; }");
    assert!(!diags.has_errors(), "{diags:?}");
    let diags = check_over_maps("foreach ($taint as var $key => var $value) { string $s = $key; }");
    assert!(
        diags.has_errors(),
        "a `var` key over a tainted key is tainted: {diags:?}"
    );
}

#[test]
fn a_plain_string_key_over_a_tainted_key_type_is_refused() {
    let diags = check_over_maps("foreach ($taint as string $key => int $value) { echo $value; }");
    assert!(has(&diags, code::E_UNCHECKED_TEXT_NOT_TAINTED), "{diags:?}");
    let diags =
        check_over_maps("foreach ($taint as tainted string $key => int $value) { echo $value; }");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A written key over the keyed pair is checked as a value binding is: a wider
/// type is accepted, and anything else is the ordinary mismatch.
#[test]
fn a_written_key_type_that_is_not_the_key_type_is_refused() {
    let diags = check_over_maps("foreach ($map as string $key => int $value) { echo $value; }");
    assert!(has(&diags, code::E_TYPE_MISMATCH), "{diags:?}");
    assert!(!has(&diags, code::E_FOREACH_KEY_TY), "{diags:?}");
    let diags = check_over_maps("foreach ($map as object $key => int $value) { echo $value; }");
    assert!(
        !diags.has_errors(),
        "a wider key type is accepted: {diags:?}"
    );
}

/// The value-only form keeps its meaning: a map still gives its keys.
#[test]
fn a_value_only_foreach_over_an_object_map_still_binds_its_keys() {
    let diags = check_over_maps("foreach ($map as var $key) { Tag $t = $key; }");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_value_only_foreach_over_a_keyed_only_class_binds_its_values() {
    let diags = check_src(&format!(
        "<?nvs\n{KEYED_FIXTURES}\
         class Pairs implements KeyedIterable<string, int> {{\n\
         \x20 public function iterateKeyed(): KeyedIterator<string, int> {{ return new PairCursor(); }}\n\
         }}\n\
         class T {{\n\
         \x20 public function m(Pairs $pairs): void {{\n\
         \x20\x20 foreach ($pairs as var $value) {{ int $n = $value; }}\n\
         \x20 }}\n\
         }}\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_key_binding_over_an_iterable_without_keys_is_still_refused() {
    let diags = check_src(
        "<?nvs\n\
         class Numbers implements Iterable<int> {\n\
         \x20 public function iterate(): Iterator<int> { yield 1; }\n\
         }\n\
         class T {\n\
         \x20 public function m(Numbers $numbers): void {\n\
         \x20\x20 foreach ($numbers as var $key => var $value) { echo $value; }\n\
         \x20 }\n\
         }\n",
    );
    assert!(has(&diags, code::E_FOREACH_KEY_ON_CURSOR), "{diags:?}");
}

/// `Both`'s `KeyedIterable` gives `string` keys and its `KeyedIterator` gives
/// `int` keys, so the key's type shows which one the loop drives.
#[test]
fn a_keyed_iterable_wins_over_a_keyed_iterator_when_a_class_reaches_both() {
    let diags = check_src(&format!(
        "<?nvs\n{KEYED_FIXTURES}\
         class Both implements KeyedIterable<string, int>, KeyedIterator<int, int> {{\n\
         \x20 public function iterateKeyed(): KeyedIterator<string, int> {{ return new PairCursor(); }}\n\
         \x20 public function advance(): bool {{ return false; }}\n\
         \x20 public function key(): int {{ return 0; }}\n\
         \x20 public function current(): int {{ return 0; }}\n\
         }}\n\
         class T {{\n\
         \x20 public function m(Both $both): void {{\n\
         \x20\x20 foreach ($both as var $key => var $value) {{ string $s = $key; }}\n\
         \x20 }}\n\
         }}\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A class with both pairs gives a value-only loop its `Iterable` element and
/// a loop with a key binding its `KeyedIterable` key and value.
#[test]
fn a_class_implementing_both_pairs_binds_by_the_loops_form() {
    let program = |body: &str| {
        check_src(&format!(
            "<?nvs\n{KEYED_FIXTURES}\
             class Index implements Iterable<Tag>, KeyedIterable<string, int> {{\n\
             \x20 public function iterate(): Iterator<Tag> {{ yield new Tag(); }}\n\
             \x20 public function iterateKeyed(): KeyedIterator<string, int> {{ return new PairCursor(); }}\n\
             }}\n\
             class T {{\n\
             \x20 public function m(Index $index): void {{\n{body}\n\
             \x20 }}\n\
             }}\n"
        ))
    };
    let diags = program("foreach ($index as var $value) { Tag $t = $value; }");
    assert!(!diags.has_errors(), "{diags:?}");
    let diags = program(
        "foreach ($index as var $key => var $value) { string $s = $key; int $n = $value; }",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}
