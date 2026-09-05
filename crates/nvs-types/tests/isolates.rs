//! ADR 0006's `spawn script` and `await`: what each is typed as, and which of
//! the five `with(...)` options this compiler will accept.
//!
//! `nvs_types::expr::isolate`'s module doc is the home of the decision these
//! pin — the handle is a registered `Core` class and the result is an ADR 0036
//! shape. A shape field the checker did not name falls back to `mixed` with no
//! diagnostic (`nvs_types::expr::members`), so what a field's *type* is can
//! only be observed the way a program observes it: by binding it.
//!
//! Every case below spawns for its handle rather than declaring a `mixed` one,
//! because `mixed` no longer satisfies an `await` (ADR 0007 § 6) — which is
//! itself one of the claims here.
//!
//! The last four are the same ADR's *operand* rule at ADR 0083 § 2's second
//! site, `Core\Socket::upgrade`, which is one rule and not two: a `Core` row
//! marks its entry parameter and `nvs_types::expr::isolate` applies exactly
//! what a `spawn script` gets. They live here for that reason rather than
//! beside the member's other cases.

mod common;

use common::*;
use nvs_diagnostics::code;

/// Neither construct is refused any more — both lower — and the operands are
/// still checked, so a typo beside one is reported in the same run.
#[test]
fn spawn_script_and_await_are_accepted_and_their_operands_still_checked() {
    let diags = check_in_method(
        "var $h = spawn script \"c.nvs\";
mixed $r = await $h;
$h = $undefined;",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
        "{diags:?}"
    );
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_SPAWN_OPTION_UNSUPPORTED)),
        "{diags:?}"
    );
}

/// The two options this compiler enforces are accepted, and the three it does
/// not are refused where they are written rather than accepted and ignored —
/// `E_SPAWN_OPTION_UNSUPPORTED`'s own doc is the home of why that is the safe
/// reading and not the pedantic one.
#[test]
fn the_three_unenforced_spawn_options_are_refused_and_the_two_enforced_ones_are_not() {
    let accepted =
        check_in_method("var $h = spawn script \"c.nvs\" with(args: 7, output: \"capture\");");
    assert!(
        !accepted
            .iter()
            .any(|d| d.code == Some(code::E_SPAWN_OPTION_UNSUPPORTED)),
        "{accepted:?}"
    );
    for option in ["limits: 7", "grants: 7", "on: \"worker\""] {
        let diags = check_in_method(&format!("var $h = spawn script \"c.nvs\" with({option});"));
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_SPAWN_OPTION_UNSUPPORTED)),
            "{option}: {diags:?}"
        );
    }
}

/// A spawn answers with `Core\Script\Handle`, which a program may name in a
/// declaration like any other class — the whole point of the handle being a
/// registered class rather than an opaque `mixed`.
#[test]
fn a_spawn_answers_with_the_registered_handle_class() {
    let diags = check_in_method("Core\\Script\\Handle $h = spawn script \"c.nvs\";");
    assert!(
        !diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// The other half of the same claim: `mixed` would satisfy the binding below,
/// so the mismatch is what proves the spawn carries a type at all.
#[test]
fn a_spawn_s_handle_is_not_assignable_to_a_string() {
    let diags = check_in_method("string $h = spawn script \"c.nvs\";");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// Nothing but a spawn produces a handle, so an operand that is not one cannot
/// have come from one — and the ordinary mismatch names the class it wanted.
///
/// The operand is a variable rather than the literal `await 5`, which does not
/// parse: `await` is contextual and is read as the operator only before what
/// the production needs (`docs/spec/00-overview.md` § 2).
#[test]
fn await_refuses_an_operand_no_spawn_produced() {
    let diags = check_in_method("int $n = 5;\nvar $r = await $n;");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// `ok` is a `bool` and `output` is a `string`, which is what
/// `examples/isolate.nvs` reads off the result. Binding each to its own type
/// reports nothing beyond the two refusals themselves.
#[test]
fn an_awaited_result_is_a_shape_carrying_ok_and_output() {
    let diags = check_in_method(
        "var $h = spawn script \"c.nvs\";\nvar $r = await $h;\nbool $ok = $r->ok;\nstring $out = $r->output;",
    );
    assert!(
        !diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// The other half of the same claim: a field that answered `mixed` would
/// satisfy the binding below, so the mismatch is what proves the shape carries
/// a type at all rather than a placeholder.
#[test]
fn an_awaited_result_s_ok_is_not_assignable_to_a_string() {
    let diags = check_in_method(
        "var $h = spawn script \"c.nvs\";\nvar $r = await $h;\nstring $ok = $r->ok;",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// ADR 0083 § 2 opens a connection with "0006's operand", so the rule above
/// has a second site — and the accepted method form is the half a member call
/// could not have got for free: `Chat::run(...)` is a `callable`-typed
/// expression, and at a `string` parameter it was an ordinary mismatch.
///
/// Both spellings in one case because what is asserted is that the parameter
/// takes *two* shapes: a path alone would pass against a plain `string`
/// parameter, and the method reference alone would pass against a `callable`
/// one. `nvs_stdlib::registry::CoreTy::Entry` is the mark that says so and
/// `nvs_types::expr::isolate`'s `check_entry` is the rule it reaches.
#[test]
fn an_upgrades_entry_takes_a_path_and_a_static_method_reference_alike() {
    let diags = check_src(
        "<?nvs\nclass Chat {\n  static function run(string $room): void {}\n}\n\
         class T {\n  function m(): void {\n\
         Core\\Socket::upgrade(\"sockets/chat.nvs\");\n\
         Core\\Socket::upgrade(Chat::run(...), args: {room: \"lobby\"});\n\
         }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The half a type-directed rule could not tell from the accepted one: both are
/// `callable`, and only the written shape says whether the compiler can see the
/// function it names.
///
/// `E0802` and **not** a mismatch, exactly as at the `spawn script` site — the
/// parameter accepts two shapes, so it never reports one expected type for an
/// operand that is neither.
#[test]
fn an_upgrade_refuses_a_callable_variable_as_its_entry() {
    let diags = check_src(
        "<?nvs\nclass Chat {\n  static function run(string $room): void {}\n}\n\
         class T {\n  function m(): void {\n\
         callable $entry = Chat::run(...);\n\
         Core\\Socket::upgrade($entry);\n\
         }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_SPAWN_ENTRY_NOT_A_PATH_OR_METHOD)),
        "{diags:?}"
    );
    assert!(
        !diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// The entry interns as `mixed` so that the method form is not a mismatch
/// (`CoreTy::Entry`), which is exactly the lowering that could have dropped
/// ADR 0088 § 1's sink with it: a path whose content becomes the instruction
/// "execute this file" is the one argument a `tainted` value may never fill.
/// The comparison is `check_entry`'s own and reports the ordinary mismatch a
/// sink does.
#[test]
fn an_upgrade_refuses_a_tainted_entry_path() {
    let diags = check_in_method(
        "tainted string $path = \"sockets/chat.nvs\" as tainted string;\n\
         Core\\Socket::upgrade($path);",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// The other half of that lowering: `mixed` admits everything, so an entry that
/// is neither of the two accepted shapes has to be refused by the rule rather
/// than by the parameter — and it is refused as the `string` the spec's column
/// writes.
#[test]
fn an_upgrade_refuses_an_entry_that_is_neither_shape() {
    let diags = check_in_method("Core\\Socket::upgrade(7);");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// The other half of what a member that opens an isolate is: ADR 0033 § 4
/// refuses a `secret` **at the graph copy**, for both its callers alike, so an
/// `args:` that crosses into a connection isolate refuses one exactly where
/// `spawn script`'s `args:` does. A member with an entry parameter is what
/// says a call is that copy — `nvs_types::expr::isolate`'s
/// `check_core_isolate_call` owns why there is no second mark for it.
///
/// The bound on the other side is the line above it: the same value binds and
/// is read, so what is refused is the crossing and not the value.
#[test]
fn a_secret_fails_to_compile_through_upgrade_args() {
    let diags = check_in_method(
        "secret string $key = \"k\" as secret string;\n\
         Core\\Socket::upgrade(\"sockets/chat.nvs\", args: $key);",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_SECRET_CROSSES_A_BOUNDARY)),
        "{diags:?}"
    );
}

/// ADR 0006 § *Failure is a value, not an exception* makes `error` present
/// exactly when `ok` is false, so its type is nullable and a plain binding to
/// the failure shape is refused.
#[test]
fn an_awaited_result_s_error_is_nullable() {
    let diags = check_in_method(
        "var $h = spawn script \"c.nvs\";\nvar $r = await $h;\n({message: string}) $e = $r->error;",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}
