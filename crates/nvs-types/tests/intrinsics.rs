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

use common::{check_src, check_src_table};
use nvs_diagnostics::{Code, Diagnostics, code};
use nvs_stdlib::regex::Tier;
use nvs_types::expr_table::ExprTypeTable;

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

/// [`check_call`] for the one assertion that is about what the fold *recorded*
/// rather than about what it refused — the table is `nvs-ir`'s half of ADR
/// 0056 § 3 and a `Diagnostics` cannot see it.
fn check_call_table(body: &str) -> (Diagnostics, ExprTypeTable) {
    check_src_table(&format!(
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
fn a_literal_patterns_tier_is_settled_while_checking() {
    // ADR 0056 § 3's second effect, and the half the test above deliberately
    // stops short of: *prepared* is not enough on its own, because a fold that
    // throws the automaton away leaves the first call to re-decide which
    // engine runs — and the tier is what decides whether § 2's step budget is
    // in play at all. So the fold writes it down and `nvs-ir` reads it back.
    //
    // The three patterns are the previous test's, on purpose: the same source
    // that must produce no diagnostic must produce exactly these tiers, so a
    // fold that went quiet by not looking fails here while still looking right
    // there.
    let (diags, exprs) = check_call_table(
        "    string $t = '[a-z]+';\n    \
         Core\\Regex\\Pattern $linear = Core\\Regex::compile('[a-z]+\\d{2,3}');\n    \
         Core\\Regex\\Pattern $fancy = Core\\Regex::compile('(?<=USD )\\d+');\n    \
         Core\\Regex\\Pattern $built = Core\\Regex::compile($t);\n",
    );
    assert!(
        !diags.has_errors(),
        "a pattern one of the two engines compiles was refused: {diags:?}"
    );

    // One of each, and — the third pattern — nothing at all for the computed
    // one. § 2's rule that nothing is refused for being dynamic is also a rule
    // about what is *recorded*: a site whose pattern the fold never read has
    // no tier to offer, rather than a guessed one.
    let linear = exprs.regex_tiers().filter(|t| *t == Tier::Linear).count();
    let backtracking = exprs
        .regex_tiers()
        .filter(|t| *t == Tier::Backtracking)
        .count();
    assert_eq!(
        (linear, backtracking),
        (1, 1),
        "the two literal patterns did not settle one tier each"
    );

    // And the routing rule itself, at the boundary the tier turns on: a
    // lookbehind is a construct finite automata cannot express, and a
    // character class is one they can. Asserting them together is what makes
    // this a rule rather than two independent observations.
    assert_eq!(
        nvs_stdlib::regex::validate("[a-z]+"),
        Ok(Tier::Linear),
        "a pattern the linear engine expresses did not stay on it"
    );
    assert_eq!(
        nvs_stdlib::regex::validate("(?<=USD )[0-9]+"),
        Ok(Tier::Backtracking),
        "a lookbehind did not reach the second tier"
    );
}

#[test]
fn a_regex_pattern_argument_refuses_a_tainted_operand() {
    // ADR 0056 § 4 over ADR 0024 § 3. A pattern is a sink because an
    // attacker-authored one is two vectors at once: a denial-of-service one,
    // and a logic-injection one that turns a validation check into an
    // approval by matching everything. § 4 states outright that there is no
    // laundering function for an arbitrary pattern, so the refusal is the
    // whole of the rule and there is nothing to reach for after it.
    //
    // `compile`'s row says so with `Qual::Sink`; every other member takes the
    // pattern as a `Pattern`-or-`string` union carrying no mark at all, which
    // refuses the same operand at the same position. Both are asserted here
    // because a rule stated on `compile` alone would leave the seven members
    // that accept a pattern string open.
    let compiled = check_call(
        "    tainted string $t = \"[a-z]+\" as tainted string;\n    \
         Core\\Regex\\Pattern $p = Core\\Regex::compile($t);\n",
    );
    assert!(
        reported(&compiled, code::E_TYPE_MISMATCH),
        "a tainted pattern reached Core\\Regex::compile: {compiled:?}"
    );

    let inline = check_call(
        "    tainted string $t = \"[a-z]+\" as tainted string;\n    \
         bool $b = Core\\Regex::matches(\"subject\", $t);\n",
    );
    assert!(
        reported(&inline, code::E_TYPE_MISMATCH),
        "a tainted pattern reached Core\\Regex::matches: {inline:?}"
    );

    // § 4's second sentence, and the half a refusal alone would get wrong: the
    // *subject* may be tainted. `matches` answers a `bool`, which carries no
    // byte of its subject, while `replace` is contagious and hands the
    // qualifier on — ADR 0024 § 2's rule that a substring matched out of a
    // tainted subject is tainted, spelled as the mark on the row.
    let subject = check_call(
        "    tainted string $s = \"input\" as tainted string;\n    \
         bool $b = Core\\Regex::matches($s, '[a-z]+');\n    \
         tainted string $r = Core\\Regex::replace($s, '[a-z]+', \"x\");\n",
    );
    assert!(
        !subject.has_errors(),
        "a tainted subject was refused: {subject:?}"
    );

    // And the one route that does exist, which is not an exception to § 4:
    // `quote` escapes every metacharacter, so what comes back is a pattern
    // matching the tainted text *literally* rather than a pattern the tainted
    // text authored. That is ADR 0024 § 3's sink-named launderer exactly, and
    // it is why `Core\Taint::assertTrusted` is not the only way out.
    let quoted = check_call(
        "    tainted string $t = \"a.b\" as tainted string;\n    \
         string $q = Core\\Regex::quote($t);\n",
    );
    assert!(
        !quoted.has_errors(),
        "Core\\Regex::quote did not launder its literal: {quoted:?}"
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
fn a_literal_duration_is_validated_while_checking() {
    // § 1's row 4, and the one grammar whose parser was already shared by
    // three callers before this pass was a fourth: ADR 0070 § 5 puts it in
    // `nvs-syntax` so the lexer's `1h30m`, `Core\Time\Duration::parse` and an
    // `nvs.toml` directive cannot drift apart. So each refusal below is
    // literally the diagnostic the *lexer* gives the same text.
    let backwards =
        check_call("    Core\\Time\\Duration $d = Core\\Time\\Duration::parse(\"30m1h\");\n");
    assert!(
        reported(&backwards, code::E_INTRINSIC_LITERAL_MALFORMED),
        "units that do not descend: {backwards:?}"
    );

    let mis_cased =
        check_call("    Core\\Time\\Duration $d = Core\\Time\\Duration::parse(\"30S\");\n");
    assert!(
        reported(&mis_cased, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a unit in the wrong case: {mis_cased:?}"
    );

    let unfinished =
        check_call("    Core\\Time\\Duration $d = Core\\Time\\Duration::parse(\"1h30\");\n");
    assert!(
        reported(&unfinished, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a count with no unit after it: {unfinished:?}"
    );

    // § 3's range, which is a refusal about the *value* rather than the
    // shape — a folded literal wider than `Duration` holds is the compile
    // error that ADR names, not a wrap.
    let wide = check_call(
        "    Core\\Time\\Duration $d = Core\\Time\\Duration::parse(\"99999999999w\");\n",
    );
    assert!(
        reported(&wide, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a duration wider than the type holds: {wide:?}"
    );

    // And what stays silent: the two spellings § 1 of ADR 0070 opens with, the
    // sub-second units, and a computed text, which is § 2's rule.
    let fine = check_call(
        "    string $t = \"30m1h\";\n    \
         Core\\Time\\Duration $simple = Core\\Time\\Duration::parse(\"30s\");\n    \
         Core\\Time\\Duration $compound = Core\\Time\\Duration::parse(\"1h30m\");\n    \
         Core\\Time\\Duration $small = Core\\Time\\Duration::parse(\"500ms\");\n    \
         Core\\Time\\Duration $built = Core\\Time\\Duration::parse($t);\n",
    );
    assert!(
        !fine.has_errors(),
        "a duration the grammar admits was refused: {fine:?}"
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
