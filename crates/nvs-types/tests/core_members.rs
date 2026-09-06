//! A `Core` member call — `nvs_stdlib::registry`'s signatures reached through the ordinary call path, including `rule:core-api/shape-rules` R2's options bag.
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

/// `rule:core-api/shape-rules` R2's options bag at a call site: written, and omitted whole.
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
/// signature with a union parameter, checked by `rule:types/unions-and-mixed`'s ordinary
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

/// The rule that makes a bag its own type rather than an `rule:types/object-top` shape:
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

/// `rule:http-server/a-non-idempotent-retry-needs-an-idempotency-key`, asked of four calls rather than of one, because **it is the
/// verb that decides**: the same bag is a diagnostic on `post` and accepted on
/// `get`, since a repeated `GET` is a second question rather than a second
/// effect.
///
/// The refusal is reportable at all only because of `rule:core-api/shape-rules` R2 — the verb is
/// the member's own name and the bag has to be a literal — which is
/// `reject_keyless_retry`'s own subject. The two accepted retries pin the
/// halves a refusal written one condition too wide would take with it, and the
/// fourth call pins the one this rule deliberately does not reach: an omitted
/// `retryAttempts` is not a retry, so a `post` that writes none owes no key.
#[test]
fn a_post_retried_without_an_idempotency_key_is_a_compile_error() {
    let keyless = check_in_method(
        "var $r = Core\\Http\\Client::post(\"https://example.test/pay\", {retryAttempts: 3});\n",
    );
    let refused = keyless
        .iter()
        .find(|d| d.code == Some(code::E_RETRY_WITHOUT_IDEMPOTENCY_KEY))
        .unwrap_or_else(|| panic!("{keyless:?}"));
    assert!(
        format!("{refused:?}").contains("retryIdempotencyKey"),
        "the refusal names the field that is missing: {refused:?}"
    );
    for accepted in [
        "var $r = Core\\Http\\Client::post(\"https://example.test/pay\", \
         {retryAttempts: 3, retryIdempotencyKey: \"order-7\"});\n",
        "var $r = Core\\Http\\Client::get(\"https://example.test/pay\", {retryAttempts: 3});\n",
        "var $r = Core\\Http\\Client::post(\"https://example.test/pay\", {deadline: 30s});\n",
    ] {
        let diags = check_in_method(accepted);
        assert!(!diags.has_errors(), "{accepted} produced {diags:?}");
    }
}

/// The bag is not a shape *target* either: `{...}` written anywhere else
/// still means `rule:types/object-top`'s anonymous object, width subtyping and all, so
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

/// `rule:concurrency/all-answers-a-typed-shape`, and the whole reason `Core\Task::all` is worth having: the
/// answer is a shape with the argument's own field names, each field typed as
/// *that field's* closure returns.
///
/// The positive half is what a uniform result could not pass — `array<mixed>`
/// is not assignable to `array<int>` and neither is `mixed`, so a collapsed
/// answer fails the two declarations below rather than merely reading
/// differently. The negative half is the other direction: one field does not
/// acquire another's type.
#[test]
fn a_task_all_binds_each_fields_own_type() {
    let diags = check_in_method(
        "var $page = Core\\Task::all({\n\
         rows:  fn(): array<int> => [1, 2, 3],\n\
         label: fn(): string     => \"all\",\n\
         });\n\
         array<int> $rows = $page->rows;\n\
         string $label = $page->label;\n\
         echo $label, Core\\Arr::count($rows);\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    let crossed = check_in_method(
        "var $page = Core\\Task::all({rows: fn(): array<int> => [1, 2, 3]});\n\
         string $rows = $page->rows;\n",
    );
    assert!(
        crossed
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{crossed:?}"
    );
}

/// `rule:concurrency/all-answers-a-typed-shape`'s restriction, and the half of the member that no program can
/// print: a field's type binds from a *written* `fn` literal, so a field
/// holding a `callable`-typed variable is a compile error naming the field.
///
/// `rule:types/closure-literal` leaves `callable` without a signature, so there is genuinely
/// nothing to bind from — the diagnostic says which field rather than refusing
/// the call as a whole, because every other field still binds.
#[test]
fn a_task_all_field_holding_a_callable_variable_is_a_compile_error() {
    let diags = check_in_method(
        "callable $loader = fn(): int => 1;\n\
         var $page = Core\\Task::all({user: $loader, rows: fn(): array<int> => [1]});\n\
         echo Core\\Arr::count($page->rows);\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_CALLABLE_SHAPE_FIELD_NOT_A_LITERAL)),
        "{diags:?}"
    );
    assert!(
        diags.iter().any(|d| d.message.contains("`user`")),
        "the diagnostic names the offending field: {diags:?}"
    );

    // The same reasoning one level up: a variable holding the whole shape has
    // no literal at any field, so the argument itself has to be written out.
    let whole = check_in_method(
        "var $set = {user: fn(): int => 1};\n\
         var $page = Core\\Task::all($set);\n",
    );
    assert!(
        whole
            .iter()
            .any(|d| d.code == Some(code::E_CALLABLE_SHAPE_NOT_A_LITERAL)),
        "{whole:?}"
    );
}

/// `rule:concurrency/limit-and-deadline-are-the-only-bounds`: one trailing options shape carries both bounds, and there is
/// no wrapper member to reach for instead.
///
/// `timeout` is the spelling a developer arrives with, so it is asked twice —
/// as an option name, which is not one, and as a member, which does not exist.
/// `race` is asked because § 3 defers it under a *different* future spelling,
/// and a `race` that quietly existed would be the thing that decision rejected.
#[test]
fn a_limit_and_deadline_options_shape_is_the_only_spelling() {
    let written = check_in_method(
        "var $page = Core\\Task::all({rows: fn(): array<int> => [1]}, {limit: 2, deadline: 5s});\n\
         echo Core\\Arr::count($page->rows);\n",
    );
    assert!(!written.has_errors(), "{written:?}");

    let wrapped = check_in_method(
        "var $page = Core\\Task::all({rows: fn(): array<int> => [1]}, {timeout: 5s});\n",
    );
    assert!(
        wrapped
            .iter()
            .any(|d| d.code == Some(code::E_UNKNOWN_OPTION)),
        "{wrapped:?}"
    );

    for absent in ["timeout", "race"] {
        let diags = check_in_method(&format!(
            "Core\\Task::{absent}({{rows: fn(): array<int> => [1]}});\n"
        ));
        assert!(
            diags.has_errors(),
            "`Core\\Task::{absent}` resolved: {diags:?}"
        );
    }
}

/// `rule:core-api/shape-arms-are-disjoint` at the call site it was written for: `Db\Settings`'s two arms,
/// and `host` on the SQLite one. SQLite is a file and has no host to reach, so
/// its arm declares none — and the merged list the ABI flattens to *does*,
/// which is why this is the case that says arm selection happens at all. Under
/// the merged list alone the key is declared, the value is a `string`, and the
/// literal compiles.
///
/// Both arms' own literals are asserted beside it, because a refusal written
/// one condition too wide takes them with it: they are the two forms
/// `tests/conformance/core/db-open-refuses-a-tls-mode-weaker-than-verify-full.nvst`
/// runs, and neither may cost a diagnostic here.
#[test]
fn a_host_on_a_sqlite_settings_literal_is_a_compile_error() {
    let file = check_in_method(
        "var $db = Core\\Db::open({driver: Core\\Db\\Driver::Sqlite, path: \"/srv/a.db\"});\n",
    );
    assert!(!file.has_errors(), "{file:?}");

    let server = check_in_method(
        "var $db = Core\\Db::open({driver: Core\\Db\\Driver::Postgres, host: \"db.test\", \
         database: \"shop\", user: \"app\", password: \"hunter2\"});\n",
    );
    assert!(!server.has_errors(), "{server:?}");

    let hosted = check_in_method(
        "var $db = Core\\Db::open({driver: Core\\Db\\Driver::Sqlite, path: \"/srv/a.db\", \
         host: \"db.test\"});\n",
    );
    let unknown = hosted
        .iter()
        .find(|d| d.code == Some(code::E_UNKNOWN_OPTION))
        .unwrap_or_else(|| panic!("{hosted:?}"));
    assert!(
        unknown.message.contains("host"),
        "the refusal names the key that does not belong: {unknown:?}"
    );
}

/// The other half of § 2's rule, and the half a key-only reading would miss:
/// which arm a literal selects is decided by its values as well as its keys, so
/// a server-shaped literal that names the SQLite driver is refused rather than
/// being accepted by the arm whose keys it wrote.
///
/// It is the `driver` value that is reported, because that is the one field the
/// two arms declare differently — `rule:core-api/shape-arms-are-disjoint`'s disjointness, arriving at a
/// call site as an ordinary type mismatch with nothing naming a discriminant.
#[test]
fn a_sqlite_driver_on_a_server_settings_literal_is_a_compile_error() {
    let crossed = check_in_method(
        "var $db = Core\\Db::open({driver: Core\\Db\\Driver::Sqlite, host: \"db.test\", \
         database: \"shop\", user: \"app\", password: \"hunter2\"});\n",
    );
    assert!(crossed.has_errors(), "{crossed:?}");
}

/// A key the *merged* list does not declare at all is still the typo it always
/// was, and is answered by the same one diagnostic — arm selection narrows what
/// a call is held to, and must not turn one mistake into two.
#[test]
fn a_key_no_settings_arm_declares_is_one_diagnostic() {
    let typo = check_in_method(
        "var $db = Core\\Db::open({driver: Core\\Db\\Driver::Sqlite, path: \"/srv/a.db\", \
         hosts: \"db.test\"});\n",
    );
    assert_eq!(
        typo.iter()
            .filter(|d| d.code == Some(code::E_UNKNOWN_OPTION))
            .count(),
        1,
        "{typo:?}"
    );
}
