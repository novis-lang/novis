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
fn a_literal_date_format_is_validated_while_checking() {
    // § 1's rows 3 and 4 — the two members reading `nvs_stdlib::cldr`'s
    // letters, and the only two rows whose written argument positions differ:
    // `$d->format(…)`'s pattern is written argument 0, while `Core\Time::parse`
    // puts the text being parsed first (ADR 0063 R1) and its pattern second.
    // `Q` is a real CLDR letter this closed subset does not carry, which is
    // exactly the case the module's own refusal names.
    let rendered = check_call(
        "    Core\\Time\\DateTime $d = Core\\Time::now()->in(Core\\Time\\Zone::UTC);\n    \
         echo $d->format(\"yyyy-QQ\");\n",
    );
    assert!(
        reported(&rendered, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a letter the subset does not carry: {rendered:?}"
    );

    // The second row, refused on the *other* half of the grammar: a quote that
    // opens a literal run and never closes it.
    let parsed = check_call(
        "    Core\\Time\\DateTime $d = \
         Core\\Time::parse(\"2026-08-29\", \"yyyy-MM-dd'\", Core\\Time\\Zone::UTC);\n    \
         echo $d->format(\"yyyy\");\n",
    );
    assert!(
        reported(&parsed, code::E_INTRINSIC_LITERAL_MALFORMED),
        "an unterminated quoted run: {parsed:?}"
    );

    // And what stays silent. The two spec § 4 patterns; a computed one, which
    // is § 2's rule again; and a pattern naming a *zone*, which
    // `Core\Time::parse` itself refuses at run time and this pass deliberately
    // does not — gap 3 in `intrinsics.rs` owns why, and this line moves to the
    // refusals above only when the table grows a column for it.
    let fine = check_call(
        "    string $p = \"yyyy-QQ\";\n    \
         Core\\Time\\DateTime $d = Core\\Time::now()->in(Core\\Time\\Zone::UTC);\n    \
         echo $d->format(\"EEEE, d MMMM yyyy\");\n    \
         echo $d->format($p);\n    \
         Core\\Time\\DateTime $z = \
         Core\\Time::parse(\"2026-08-29 UTC\", \"yyyy-MM-dd VV\", Core\\Time\\Zone::UTC);\n    \
         echo $z->format(\"yyyy-MM-dd\");\n",
    );
    assert!(
        !fine.has_errors(),
        "a pattern the grammar reads was refused: {fine:?}"
    );
}

#[test]
fn a_literal_regex_pattern_is_prepared_while_checking() {
    // § 1's row 1, over ADR 0056's two engines — the pattern is offered to
    // both while checking, exactly as `Core\Regex::compile`'s first call would
    // have offered it. *Prepared* is the name this check has always had and is
    // § 3's second effect; today the fold stops after validating, which is gap
    // 2 in `intrinsics.rs` and is a channel to `nvs-ir` rather than a second
    // parser.
    let unclosed = check_call("    Core\\Regex\\Pattern $p = Core\\Regex::compile(\"(a\");\n");
    assert!(
        reported(&unclosed, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a group nothing closes: {unclosed:?}"
    );

    // Neither engine, which is the refusal ADR 0056 § 5 names: the linear one
    // cannot express a backreference and the backtracking one, which can,
    // still has no group 9 to refer to.
    // The patterns below are single-quoted for the reason the format test
    // gives: a Novis `"…"` reads `\d` and `{2,3}` as its own syntax, and a
    // pattern is written the way it will be read.
    let dangling = check_call("    Core\\Regex\\Pattern $p = Core\\Regex::compile('(a)\\9');\n");
    assert!(
        reported(&dangling, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a backreference to a group that does not exist: {dangling:?}"
    );

    // And what stays silent: a pattern the linear engine takes, one only the
    // backtracking tier can express — a lookbehind — and a computed one, which
    // is § 2's rule. A pattern reaching the second tier is not a refusal
    // here, because it is not one at run time either.
    let fine = check_call(
        "    string $t = \"(a\";\n    \
         Core\\Regex\\Pattern $linear = Core\\Regex::compile('[a-z]+\\d{2,3}');\n    \
         Core\\Regex\\Pattern $fancy = Core\\Regex::compile('(?<=USD )\\d+');\n    \
         Core\\Regex\\Pattern $built = Core\\Regex::compile($t);\n",
    );
    assert!(
        !fine.has_errors(),
        "a pattern one of the two engines compiles was refused: {fine:?}"
    );
}

#[test]
fn a_literal_uri_is_validated_while_checking() {
    // § 1's row 2. Both of `Core\Uri::parse`'s throwing steps are folded, so
    // the two texts it refuses are two compile errors here: a byte RFC 3986
    // § 4.1 does not admit, and a run of digits the grammar takes for a port
    // and § 3.2.3's prose does not.
    let byte = check_call("    Core\\Uri $u = Core\\Uri::parse(\"http://exa mple.test/\");\n");
    assert!(
        reported(&byte, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a space inside a URI: {byte:?}"
    );

    let port = check_call("    Core\\Uri $u = Core\\Uri::parse(\"http://example.test:99999/\");\n");
    assert!(
        reported(&port, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a port no socket could dial: {port:?}"
    );

    // And what stays silent. A *reference* is what this member takes, so a
    // relative one with no scheme is not a refusal; neither is a percent
    // escape the grammar reads; and a computed text is § 2's rule.
    let fine = check_call(
        "    string $t = \"http://exa mple.test/\";\n    \
         Core\\Uri $absolute = Core\\Uri::parse(\"https://example.test:8443/a/b?q=1#top\");\n    \
         Core\\Uri $relative = Core\\Uri::parse(\"../sibling/page%20one\");\n    \
         Core\\Uri $built = Core\\Uri::parse($t);\n",
    );
    assert!(
        !fine.has_errors(),
        "a URI reference the grammar admits was refused: {fine:?}"
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
