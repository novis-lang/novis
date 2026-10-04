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

/// `rule:statements/no-host-populated-variables`: the help under a refused
/// superglobal names the `Core` member that replaces it, and every name it
/// writes is a row of the registry, so the help never sends its reader to a
/// second compile error. The parser owns the text and cannot reach the
/// registry; this is the crate that sees both.
// covers: tools:php-differences/two-spellings-side-by-side
#[test]
fn every_core_member_a_refused_superglobal_names_is_registered() {
    for name in [
        "$_GET",
        "$_POST",
        "$_COOKIE",
        "$_FILES",
        "$_SERVER",
        "$_SESSION",
        "$_ENV",
        "$argv",
        "$argc",
        "$_ARGS",
    ] {
        let diags = check_src_allowing_parse_errors(&format!("<?nvs\necho {name};\n"));
        let refusal = diags
            .iter()
            .find(|d| d.code == Some(code::E_SUPERGLOBAL_UNSUPPORTED))
            .unwrap_or_else(|| panic!("`{name}` is not refused: {diags:?}"));
        let named: Vec<&str> = refusal
            .notes
            .iter()
            .flat_map(|note| note.split('`'))
            .filter(|piece| piece.starts_with("Core\\"))
            .collect();
        assert!(
            !named.is_empty(),
            "the help for `{name}` names no `Core` member: {refusal:?}"
        );
        for symbol in named {
            let (class, member) = symbol.split_once("::").unwrap_or((symbol, ""));
            let row = nvs_stdlib::registry::class(class).unwrap_or_else(|| {
                panic!("the help for `{name}` names `{class}`, which is not registered")
            });
            assert!(
                member.is_empty() || row.members().any(|candidate| candidate.name == member),
                "the help for `{name}` names `{symbol}`, and `{class}` has no such member"
            );
        }
    }
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

/// `Core\Arr::map(array<T> $a, callable(T, string): U $fn): array<U>` — the
/// `U` binds from the signature the `fn` literal reports, so the result is
/// `array<string>` and reaches `Core\Str::join`, which takes exactly that.
/// `crate::generics` owns the rule; this is it end to end through the
/// checker.
///
/// The unannotated spelling is the one the written signature buys
/// (`rule:types/anonymous-function-parameter-inference`): `$n` takes `int` from the
/// substituted parameter, which is only substituted because the literal is
/// checked after the subject bound `T`.
#[test]
fn map_binds_its_result_from_a_written_fn_literal() {
    let inferred = check_in_method(
        "array<int> $a = [1, 2];\n\
         array<string> $out = Core\\Arr::map($a, fn(int $n) => \"n\" . $n);\n\
         echo Core\\Str::join($out, \",\");\n",
    );
    assert!(!inferred.has_errors(), "{inferred:?}");

    let unannotated = check_in_method(
        "array<int> $a = [1, 2];\n\
         array<string> $out = Core\\Arr::map($a, fn($n) => \"n\" . $n);\n\
         echo Core\\Str::join($out, \",\");\n",
    );
    assert!(!unannotated.has_errors(), "{unannotated:?}");

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

/// The callback need not be written at the call site: a variable carrying a
/// `rule:types/callable-signature` signature binds `U` structurally out of its
/// own type, where the binding used to need the literal's recorded return type
/// and so needed the literal.
///
/// Bare `callable` is what is left of that gap, and it is a refusal rather
/// than an `array<mixed>`: the top of the lattice promises no return type, so
/// it cannot fill a position that names one, and the annotation the variable
/// declares is what threw the signature away.
#[test]
fn map_binds_its_result_from_a_callable_typed_variable() {
    let bound = check_in_method(
        "array<int> $a = [1, 2];\n\
         var $fn = fn(int $n): string => \"n\" . $n;\n\
         array<string> $out = Core\\Arr::map($a, $fn);\n\
         echo Core\\Str::join($out, \",\");\n",
    );
    assert!(!bound.has_errors(), "{bound:?}");

    let bare = check_in_method(
        "array<int> $a = [1, 2];\n\
         callable $fn = fn(int $n): string => \"n\" . $n;\n\
         array<string> $out = Core\\Arr::map($a, $fn);\n",
    );
    assert!(
        bare.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{bare:?}"
    );

    // And what was never a callable at all is refused where it is written, by
    // the same one comparison.
    let refused = check_in_method("array<int> $a = [1];\nvar $out = Core\\Arr::map($a, 7);\n");
    assert!(refused.has_errors(), "{refused:?}");
}

/// `rule:expressions/method-reference`'s reference carries the
/// member's own signature, so it binds `U` from `Core\Str::length`'s declared
/// `uint` and satisfies the parameter for the same reason a literal wrapping
/// the same call would. The reference is the spelling that has no body to read
/// a return type out of, which is why it needed the signature to be a *type*.
#[test]
fn map_binds_its_result_from_a_first_class_callable_reference() {
    let bound = check_in_method(
        "array<string> $a = [\"ab\", \"c\"];\n\
         array<uint> $out = Core\\Arr::map($a, Core\\Str::length(...));\n\
         echo Core\\Arr::count($out);\n",
    );
    assert!(!bound.has_errors(), "{bound:?}");

    // A real binding, not a widening: the element type is the member's return
    // type and nothing else will hold it.
    let wrong = check_in_method(
        "array<string> $a = [\"ab\"];\n\
         array<string> $out = Core\\Arr::map($a, Core\\Str::length(...));\n",
    );
    assert!(
        wrong.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{wrong:?}"
    );
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

/// `rule:concurrency/all-answers-a-typed-shape` where every field carries its
/// own written signature: a `fn` literal and a first-class callable are the
/// two spellings that declare one on the spot, and each field is typed by what
/// that spelling returns rather than by what the shape as a whole holds.
#[test]
fn task_all_binds_its_result_shape_from_written_literals() {
    let diags = check_in_method(
        "var $page = Core\\Task::all({\n\
         rows:  fn(): array<int> => [1, 2],\n\
         label: fn(): string     => \"a\",\n\
         });\n\
         array<int> $rows = $page->rows;\n\
         string $label = $page->label;\n\
         echo $label, Core\\Arr::count($rows);\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The same binding with nothing written at the call.
/// `rule:types/callable-signature` put a callable's result on its *type*, so a
/// variable holding one field, and a variable holding the whole shape, bind
/// exactly as a literal does — which is
/// `rule:concurrency/an-all-field-answers-what-its-callable-declares` and the
/// restriction it replaced.
#[test]
fn task_all_binds_its_result_shape_from_callable_typed_variables() {
    let field = check_in_method(
        "var $loader = fn(): array<int> => [1];\n\
         var $page = Core\\Task::all({rows: $loader, label: fn(): string => \"a\"});\n\
         array<int> $rows = $page->rows;\n\
         string $label = $page->label;\n\
         echo $label, Core\\Arr::count($rows);\n",
    );
    assert!(!field.has_errors(), "{field:?}");

    let whole = check_in_method(
        "var $set = {user: fn(): int => 1};\n\
         var $page = Core\\Task::all($set);\n\
         int $user = $page->user;\n\
         echo $user;\n",
    );
    assert!(!whole.has_errors(), "{whole:?}");

    // A real binding, not a widening: the field's type is that callable's own
    // return type and nothing else will hold it.
    let crossed = check_in_method(
        "var $loader = fn(): array<int> => [1];\n\
         var $page = Core\\Task::all({rows: $loader});\n\
         string $rows = $page->rows;\n",
    );
    assert!(
        crossed
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{crossed:?}"
    );
}

/// The restriction that is gone, and the two refusals that are not.
///
/// A field declaring bare `callable` — the top of the lattice — has no result
/// to read, so it answers `mixed` for itself alone while every sibling stays
/// precise. That is what every other position pays for the same omission, and
/// it replaces a compile error that cost a framework holding its closures in a
/// variable the whole member. What the parameter still refuses is a field that
/// is no callable at all, and an argument that is no shape at all.
#[test]
fn task_all_no_longer_refuses_a_field_that_is_not_a_literal() {
    let bare = check_in_method(
        "callable $loader = fn(): int => 1;\n\
         var $page = Core\\Task::all({user: $loader, rows: fn(): array<int> => [1]});\n\
         array<int> $rows = $page->rows;\n\
         echo Core\\Arr::count($rows);\n",
    );
    assert!(!bare.has_errors(), "{bare:?}");

    let field = check_in_method("var $page = Core\\Task::all({user: 1});\n");
    assert!(
        field.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{field:?}"
    );

    let subject =
        check_in_method("callable $one = fn(): int => 1;\nvar $page = Core\\Task::all($one);\n");
    assert!(
        subject
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{subject:?}"
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
