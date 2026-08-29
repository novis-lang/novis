//! ADR 0057's intrinsic folding, at the call site: the closed list of `Core`
//! members whose literal argument the checker reads, and what it says about
//! one it cannot.
//!
//! What is asserted here is § 1's *closure* as much as its contents — a member
//! one row away from an intrinsic folds nothing however literal its argument
//! is — and § 4's soundness rule, which is the reason a refusal here is only
//! ever one the runtime would also have made: `nvs_stdlib::format` parses the
//! template for both, so a template this file pins as a compile error is one
//! `Core\Str::format` would have thrown on.

mod common;

use common::check_src;
use nvs_diagnostics::{Code, Diagnostics, code};

/// Whether `want` was reported at all — `routes.rs`' own helper, because a
/// fixture asserting *which* refusal was made is what keeps two codes from
/// standing in for each other.
fn reported(diags: &Diagnostics, want: Code) -> bool {
    diags.iter().any(|d| d.code == Some(want))
}

/// Wraps `body` in a top-level `Main::main`, which is where a `Core` call in a
/// program is written.
fn check_call(body: &str) -> Diagnostics {
    check_src(&format!(
        "<?nvs\nclass Main {{\n  public static function main(): void {{\n{body}\n  }}\n}}\n"
    ))
}

#[test]
fn a_member_outside_the_intrinsic_list_is_never_folded() {
    // `%q` is not a conversion `Core\Str::format`'s grammar allows, so this is
    // a literal that *would* be refused had the member been on § 1's list.
    // Every member below takes a `string` at the same position and is not one,
    // which is the whole of what "closed list" means: being pattern-shaped, or
    // being spelled `format`, or living in a class one of whose siblings is an
    // intrinsic, buys nothing.
    let diags = check_call(
        "    echo Core\\Str::upper(\"%q\");\n    \
         echo Core\\Str::trim('%1$');\n    \
         echo Core\\Csv::format([['%q']]);\n",
    );
    assert!(
        !diags.has_errors(),
        "a member off the list folded its literal: {diags:?}"
    );
}

#[test]
fn a_literal_format_template_checks_its_placeholder_count_and_types() {
    // The count, in both directions — § 1 names this member's placeholder
    // check separately because it is PHP's `printf` bug family, and both
    // halves of it are the runtime's own refusals answered earlier.
    let missing = check_call("    echo Core\\Str::format(\"%s %s\", \"one\");\n");
    assert!(
        reported(&missing, code::E_FORMAT_TEMPLATE_MISMATCH),
        "a template reading past its arguments: {missing:?}"
    );
    let unread = check_call("    echo Core\\Str::format(\"%s\", \"one\", \"two\");\n");
    assert!(
        reported(&unread, code::E_FORMAT_TEMPLATE_MISMATCH),
        "an argument no placeholder reads: {unread:?}"
    );

    // The type. `%d` has no reading for a `string`, and the argument's static
    // type is what says so — this is the check that has no runtime twin at
    // all until a value exists.
    let wrong = check_call("    echo Core\\Str::format(\"%d\", \"twelve\");\n");
    assert!(
        reported(&wrong, code::E_FORMAT_TEMPLATE_MISMATCH),
        "`%d` against a `string`: {wrong:?}"
    );

    // A malformed template is the *other* code: the literal itself is what
    // this member cannot read, and no argument list would have helped.
    let malformed = check_call("    echo Core\\Str::format(\"%q\", 1);\n");
    assert!(
        reported(&malformed, code::E_INTRINSIC_LITERAL_MALFORMED),
        "an unknown conversion character: {malformed:?}"
    );

    // And the shapes that must stay silent. `%1$s` numbers its arguments, so
    // reading one twice reads them all; `%%` is not a placeholder; and a
    // computed template is § 2's "nothing is refused for being dynamic".
    //
    // The positional template is single-quoted because a Novis `"…"` would
    // interpolate `$s` — the one place this grammar and the language's own
    // string syntax collide, and `'%1$s'` is the spelling that survives it.
    let fine = check_call(
        "    echo Core\\Str::format('%1$s/%1$s %2$d', \"a\", 7);\n    \
         echo Core\\Str::format(\"100%% of %s\", \"it\");\n    \
         string $t = '%q %q %q';\n    echo Core\\Str::format($t, 1);\n",
    );
    assert!(
        !fine.has_errors(),
        "a well-formed or dynamic template was refused: {fine:?}"
    );
}

#[test]
fn a_nullable_argument_is_not_refused_against_a_numeric_conversion() {
    // ADR 0057 § 4 as a test: preparation produces an earlier answer and never
    // a different one. `Core\Str::format("%d", $n)` for a `?int` throws at run
    // time only when `$n` is actually `null`, so refusing it while checking
    // would be a *different* answer for every program whose value is not.
    let diags =
        check_call("    ?int $n = 1 > 2 ? null : 7;\n    echo Core\\Str::format(\"%d\", $n);\n");
    assert!(
        !diags.has_errors(),
        "a `?int` was refused where the runtime would have accepted it: {diags:?}"
    );
}
