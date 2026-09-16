//! `rule:expressions/intrinsic-literals`'s intrinsic folding, at the call site: the closed list of `Core`
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

use common::{check_src, check_src_granted, check_src_table, check_src_table_allowing_errors};
use nvs_config::tree::{CapDb, CapQueue, Capabilities, Setting};
use nvs_diagnostics::{Code, Diagnostics, Severity, code};
use nvs_stdlib::regex::Tier;
use nvs_stdlib::registry::{CoreTy, Qual};
use nvs_types::expr_table::ExprTypeTable;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

/// § 3's exact offset. Asserted as the *text* the primary label covers rather
/// than as a column, so the case says what a reader would see underlined and
/// survives the wrapper above it changing length.
#[test]
fn a_refused_placeholder_underlines_its_own_offset() {
    for (literal, want) in [
        // An escape ahead of the placeholder, so its decoded offset and its
        // written one differ: a refusal reading the decoded offset as a column
        // lands two characters early.
        ("\"ok\\t %q here\"", "%q"),
        // The single-quoted grammar's own escape, ahead of a placeholder that
        // reads further than the arguments go. `$` reaches the template
        // grammar only in this spelling — in a double-quoted literal `%2$s`
        // is an interpolated `$s` long before this pass sees it.
        ("'it\\'s %s %2$s'", "%2$s"),
        // Nothing ahead of the offending placeholder but another placeholder:
        // the case that fails, rather than passing by accident, if what is
        // really being exercised is the fallback to the whole literal.
        ("'%1$s and %3$s'", "%3$s"),
    ] {
        let src = format!(
            "<?nvs\nclass Main {{\n  public static function main(): void {{\n    \
             echo Core\\Str::format({literal}, \"one\");\n  }}\n}}\n"
        );
        let underlined = underlined_by_the_refusal(&src);
        assert_eq!(underlined, want, "for `{literal}`");
    }
}

/// The bound the case above is stated against: a spelling whose decoded bytes
/// sit at no source offset of their own is underlined whole, and is still
/// refused. A heredoc is that spelling, because the flexible-indentation strip
/// moves every byte off the offset it was written at.
#[test]
fn a_heredoc_template_is_refused_against_the_whole_literal() {
    let src = "<?nvs\nclass Main {\n  public static function main(): void {\n    \
               echo Core\\Str::format(<<<'TPL'\n        ok %q here\n        TPL, \"one\");\n  }\n}\n";
    let underlined = underlined_by_the_refusal(src);
    assert!(
        underlined.starts_with("<<<'TPL'") && underlined.contains("%q"),
        "a heredoc is underlined whole, not in part: {underlined:?}"
    );
}

/// The text the first refusal's primary label covers — what a reader would see
/// carets under. Asserted as text rather than as a column so a case says what
/// it means and survives the wrapper around it changing length.
fn underlined_by_the_refusal(src: &str) -> &str {
    let diags = check_src(src);
    let refusal = diags
        .iter()
        .find(|d| d.severity == Severity::Error)
        .unwrap_or_else(|| panic!("nothing was refused in:\n{src}"));
    let label = refusal
        .labels
        .first()
        .unwrap_or_else(|| panic!("refused without a primary label in:\n{src}"));
    &src[label.span.start as usize..label.span.end as usize]
}

#[test]
fn a_named_argument_is_read_at_the_parameter_it_fills() {
    // `rule:core-api/parameters-are-callable-by-name`: the roster addresses a
    // parameter, and the call's own argument mapping says which written
    // argument fills it — so every refusal above is the same refusal when the
    // pattern is written by name. Asserted on two rows whose `at` differs, so
    // a mapping that only happened to agree with position 0 fails here.
    let named = check_call("    echo Core\\Str::format(template: \"%q\");\n");
    assert!(
        reported(&named, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a `template:` template was not read: {named:?}"
    );
    let parsed = check_call(
        "    Core\\Time\\DateTime $d = Core\\Time::parse(text: \"2026-08-29\", \
         format: \"yyyy-MM-dd'\", zone: Core\\Time\\Zone::UTC);\n    \
         echo $d->format(\"yyyy\");\n",
    );
    assert!(
        reported(&parsed, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a `format:` CLDR pattern was not read: {parsed:?}"
    );

    // The count against the arguments is made through the same mapping, and it
    // counts only the values the variadic tail binds: `template:` fills a
    // parameter, so a named call with nothing after it supplies none. Every
    // *value* is still positional — a name never reaches a variadic tail, and
    // a positional argument cannot follow a named one — so this is the whole
    // shape a named template has.
    let counted = check_call("    echo Core\\Str::format(template: \"%s\");\n");
    assert!(
        reported(&counted, code::E_FORMAT_TEMPLATE_MISMATCH),
        "a named template reading past its arguments: {counted:?}"
    );

    // A `...` is the shape that still says nothing about the tail — how many
    // entries it hands over is a run-time fact — while the template's own
    // grammar is read either way. The pair is what keeps "a name is read" from
    // being a claim that every argument list now is.
    let spread = check_call(
        "    array<string> $rest = [\"one\"];\n    \
         echo Core\\Str::format(\"%s %s\", ...$rest);\n",
    );
    assert!(
        !spread.has_errors(),
        "a spread tail was counted as one argument: {spread:?}"
    );
    let spread_malformed = check_call(
        "    array<string> $rest = [\"one\"];\n    \
         echo Core\\Str::format(\"%q\", ...$rest);\n",
    );
    assert!(
        reported(&spread_malformed, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a malformed template beside a spread: {spread_malformed:?}"
    );
}

#[test]
fn a_literal_date_format_is_validated_while_checking() {
    // § 1's rows 3 and 4 — the two members reading `nvs_stdlib::cldr`'s
    // letters, and the only two rows whose written argument positions differ:
    // `$d->format(…)`'s pattern is written argument 0, while `Core\Time::parse`
    // puts the text being parsed first (`rule:core-api/shape-rules` R1) and its pattern second.
    // `j` — which CLDR reserves for a skeleton's preference between `h` and
    // `H` rather than giving it a field — is a real letter this closed subset
    // does not carry, which is exactly the case the module's own refusal names.
    let rendered = check_call(
        "    Core\\Time\\DateTime $d = Core\\Time::now()->in(Core\\Time\\Zone::UTC);\n    \
         echo $d->format(\"yyyy-jj\");\n",
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

    // The row's own restriction, which is the third refusal and not a fourth
    // grammar: the pattern is one `compile` reads and one `$d->format(…)`
    // accepts, and it is wrong only for the member that takes a `Zone` as its
    // third argument. The pair below is what says so — the same letters,
    // refused at `parse` and silent at `format`.
    let zonal = check_call(
        "    Core\\Time\\DateTime $z = \
         Core\\Time::parse(\"2026-08-29 UTC\", \"yyyy-MM-dd VV\", Core\\Time\\Zone::UTC);\n    \
         echo $z->format(\"yyyy-MM-dd\");\n",
    );
    assert!(
        reported(&zonal, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a zonal field in a parse pattern: {zonal:?}"
    );

    // And what stays silent: the two spec § 4 patterns, the zonal one at the
    // member that has no zone argument to disagree with, and a computed
    // pattern, which is § 2's rule again.
    let fine = check_call(
        "    string $p = \"yyyy-YY\";\n    \
         Core\\Time\\DateTime $d = Core\\Time::now()->in(Core\\Time\\Zone::UTC);\n    \
         echo $d->format(\"EEEE, d MMMM yyyy\");\n    \
         echo $d->format(\"yyyy-MM-dd VV\");\n    \
         echo $d->format($p);\n    \
         Core\\Time\\DateTime $z = \
         Core\\Time::parse(\"2026-08-29 UTC\", $p, Core\\Time\\Zone::UTC);\n    \
         echo $z->format(\"yyyy-MM-dd\");\n",
    );
    assert!(
        !fine.has_errors(),
        "a pattern the grammar reads was refused: {fine:?}"
    );
}

#[test]
fn a_literal_regex_pattern_is_prepared_while_checking() {
    // § 1's row 1, over `rule:core-classes/regex-two-tiers`'s two engines — the pattern is offered to
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

    // Neither engine, which is the refusal `rule:core-classes/regex-syntax` names: the linear one
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
    // `rule:core-classes/regex-literal-tiering`'s second effect, and the half the test above deliberately
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

/// The committed record's own header, rewritten whole with it: what the file
/// is, and the one command that regenerates it.
const TIER_RECORD_HEADER: &str = "\
# Every literal pattern the regex conformance suite hands a `Core\\Regex` member, and
# the engine tier it compiles on — `refused` for the malformed ones the suite writes on
# purpose. A tier is a property of the pattern text alone
# (`rule:core-classes/regex-literal-tiering`), so a routing change that moves a pattern
# between the two engines changes a line here instead of quietly changing what a request
# runs on.
#
# Kept by `every_literal_regex_pattern_in_the_suite_has_its_tier_recorded` in
# crates/nvs-types/tests/intrinsics.rs, and regenerated by that test under
# NVS_RECORD_REGEX_TIERS=1. Read the diff it writes rather than committing it unread.

";

/// Where the record sits: inside the suite it describes, so the two move in one
/// commit and a reader of either finds the other.
const TIER_RECORD: &str = "regex-literal-tiers.txt";

/// The regex conformance suite, from this crate's own directory.
fn regex_suite() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/core")
}

/// Whether `path` is one of that suite's cases. The conformance tree groups a
/// class's cases by a filename prefix, so the prefix is the whole membership
/// test and a case added under it is picked up without anything being listed
/// twice.
fn is_regex_case(path: &Path) -> bool {
    path.extension().is_some_and(|kind| kind == "nvst")
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("regex-"))
}

/// The `--FILE--` body of a `.nvst` case, its line endings normalised so a span
/// recorded against it indexes the same bytes wherever the tree was checked
/// out. The format's home is `crates/nvs-test`'s module doc; this reads the one
/// section it needs rather than taking the harness as a dependency.
///
/// # Panics
/// Panics if the case is unreadable or carries no `--FILE--` body — both mean
/// the suite and this test have drifted apart.
fn file_section(case: &Path) -> String {
    let text = std::fs::read_to_string(case)
        .unwrap_or_else(|err| panic!("{} is not readable: {err}", case.display()));
    let mut body = String::new();
    let mut inside = false;
    for line in text.lines() {
        if line.len() >= 4 && line.starts_with("--") && line.ends_with("--") {
            if inside {
                break;
            }
            inside = line == "--FILE--";
            continue;
        }
        if inside {
            body.push_str(line);
            body.push('\n');
        }
    }
    assert!(!body.is_empty(), "{} has no --FILE-- body", case.display());
    body
}

/// `Core\Regex\Pattern`'s own name, which is what marks a parameter as the one
/// a pattern is written at.
const PATTERN_CLASS: &str = r"Core\Regex\Pattern";

/// The span's own text.
fn slice(body: &str, span: nvs_diagnostics::Span) -> &str {
    &body[span.start as usize..span.end as usize]
}

/// Which argument of `Core\Regex::<member>` is the pattern, read off the class's
/// registry rows so this test carries no second copy of the signatures: the
/// pattern is the `Pattern`-or-`string` union everywhere except at `compile`,
/// whose argument is the sink itself (`rule:security/regex-pattern-is-a-sink`).
/// A member taking neither — `quote`, whose argument is text to be escaped — has
/// no pattern position, and so contributes nothing here.
fn pattern_position(member: &str) -> Option<usize> {
    let method = nvs_stdlib::regex::CLASS
        .methods
        .iter()
        .find(|row| row.name == member)?;
    method.params.iter().position(|param| match param {
        CoreTy::Text(Qual::Sink) => true,
        CoreTy::Union(members) => members
            .iter()
            .any(|one| matches!(one, CoreTy::Instance(name) if *name == PATTERN_CLASS)),
        _ => false,
    })
}

/// The tier `literal` lands in, as the compiler settles it: the literal is
/// handed to `Core\Regex::compile` and the fold's own answer is read back.
///
/// Going through the fold rather than calling `nvs_stdlib::regex::validate` a
/// second time is `rule:expressions/preparation-preserves-behaviour` applied to
/// the record — a tier taken from anywhere else would be a second opinion
/// rather than a record of what the compiler settled — and it is also what
/// decodes the literal, using the lexer's escapes rather than this file's idea
/// of them.
fn tier_of(literal: &str) -> &'static str {
    let (diags, exprs) = check_src_table_allowing_errors(&format!(
        "<?nvs\nvar $p = Core\\Regex::compile({literal});\n"
    ));
    match exprs.regex_tiers().next() {
        Some(Tier::Linear) => "linear",
        Some(Tier::Backtracking) => "backtracking",
        None => {
            assert!(
                diags.has_errors(),
                "{literal} settled no tier and was not refused either"
            );
            "refused"
        }
    }
}

/// Every literal pattern `body` writes, collected into `into`, and the number of
/// pattern-taking `Core\Regex` calls it was read out of.
///
/// The calls are found through `nvs_syntax::walk`, so the argument the pattern
/// sits at is the parser's own and not a guess made by scanning text: a
/// `StaticCall`'s children are its class expression and then its arguments in
/// order, a written member name being a name rather than a node. An argument
/// that is not a plain `Str` is left alone — § 2 of
/// `rule:core-classes/regex-literal-tiering` refuses nothing for being dynamic,
/// and a pattern assembled at run time has no tier to record.
fn patterns_of(body: &str, case: &Path, into: &mut BTreeMap<String, &'static str>) -> usize {
    let mut map = nvs_diagnostics::SourceMap::new();
    let file = map.add(case.display().to_string(), body);
    let mut diags = Diagnostics::new();
    let stmts = nvs_syntax::parse_file(map.file(file), &mut diags);
    assert!(
        !diags.has_errors(),
        "{} no longer parses: {diags:?}",
        case.display()
    );

    let mut sites = 0;
    for root in &nvs_syntax::walk::of_stmts(&stmts) {
        for node in std::iter::once(root).chain(root.descendants()) {
            if node.kind != "StaticCall" {
                continue;
            }
            let (Some(name), Some(class)) = (node.name, node.children.first()) else {
                continue;
            };
            if slice(body, class.span) != r"Core\Regex" {
                continue;
            }
            let Some(at) = pattern_position(slice(body, name)) else {
                continue;
            };
            sites += 1;
            let Some(pattern) = node.children.get(1 + at) else {
                continue;
            };
            if pattern.kind != "Str" {
                continue;
            }
            let literal = slice(body, pattern.span);
            assert!(
                !literal.contains('\n'),
                "{} writes a pattern across lines, which the record cannot hold one per line",
                case.display()
            );
            let tier = tier_of(literal);
            if let Some(earlier) = into.insert(literal.to_owned(), tier) {
                assert_eq!(
                    earlier, tier,
                    "`{literal}` settled two tiers in one suite, which the pattern text alone \
                     cannot do"
                );
            }
        }
    }
    sites
}

/// How many pattern-taking `Core\Regex` calls the case's own text shows, code
/// and comments alike minus the line comments — what the walk's count is held
/// against, so a call the walk quietly failed to reach is a failure rather than
/// a record that merely looks complete.
fn pattern_calls_in_text(body: &str) -> usize {
    let mut count = 0;
    for line in body.lines() {
        let code = line.split_once("//").map_or(line, |(before, _)| before);
        let mut rest = code;
        while let Some(at) = rest.find(r"Core\Regex::") {
            rest = &rest[at + r"Core\Regex::".len()..];
            let member: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if pattern_position(&member).is_some() {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn every_literal_regex_pattern_in_the_suite_has_its_tier_recorded() {
    // ADR 0056's M8 line, and `rule:core-classes/regex-literal-tiering`'s second
    // paragraph is what makes it possible: a tier is settled by the pattern text
    // alone, so writing it down beside the pattern is a claim a later run either
    // reproduces or has changed the engines under. The record is committed for
    // that second case — a routing change that moves a pattern arrives as a line
    // in a diff, rather than as a request that quietly started backtracking
    // under a budget it had never been subject to.
    //
    // The suite is read whole rather than through the checker's own fold,
    // because the fold's roster carries `compile` alone: a literal handed
    // straight to `matches` or `split` is a pattern the suite depends on and is
    // not one the compiler tiers, and a record covering only the folded eighth
    // of the suite would be a guard over almost nothing. What each pattern's
    // tier comes from is still the fold — `tier_of` hands the literal to
    // `compile` — so the record is one implementation's answer throughout.
    let suite = regex_suite();
    let mut cases: Vec<PathBuf> = std::fs::read_dir(&suite)
        .unwrap_or_else(|err| panic!("{} is not readable: {err}", suite.display()))
        .map(|entry| entry.expect("read one suite entry").path())
        .filter(|path| is_regex_case(path))
        .collect();
    cases.sort();
    assert!(
        !cases.is_empty(),
        "no regex case was found under {} — the suite moved, and this record is now over nothing",
        suite.display()
    );

    let mut tiers: BTreeMap<String, &'static str> = BTreeMap::new();
    for case in &cases {
        let body = file_section(case);
        assert_eq!(
            patterns_of(&body, case, &mut tiers),
            pattern_calls_in_text(&body),
            "{} writes more pattern-taking calls than the walk reached, so the record is short \
             of what the case exercises",
            case.display()
        );

        // And the half the compiler does settle agrees with the record, which is
        // what keeps the two from drifting: every tier the fold wrote down while
        // checking the case is the tier this record already holds for that
        // literal.
        let (diags, exprs) = check_src_table(&body);
        assert!(
            !diags.has_errors(),
            "{} no longer checks, so its patterns were never folded: {diags:?}",
            case.display()
        );
        for (span, tier) in exprs.regex_tier_sites() {
            let literal = slice(&body, span);
            let folded = match tier {
                Tier::Linear => "linear",
                Tier::Backtracking => "backtracking",
            };
            assert_eq!(
                tiers.get(literal).copied(),
                Some(folded),
                "the fold settled `{literal}` in {case:?} at a tier the record does not hold"
            );
        }
    }

    let mut record = String::from(TIER_RECORD_HEADER);
    for (literal, tier) in &tiers {
        record.push_str(&format!("{tier:<12}  {literal}\n"));
    }

    let path = suite.join(TIER_RECORD);
    if std::env::var_os("NVS_RECORD_REGEX_TIERS").is_some() {
        std::fs::write(&path, &record)
            .unwrap_or_else(|err| panic!("{} is not writable: {err}", path.display()));
    }
    let on_disk = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        on_disk, record,
        "the recorded tiers and the suite have drifted apart. Regenerate with \
         `NVS_RECORD_REGEX_TIERS=1 cargo test -p nvs-types --test intrinsics` and read what \
         changed: a line whose tier moved is an engine routing change and wants a decision, \
         while a line added or removed is only a pattern the suite gained or lost."
    );
}

#[test]
fn a_regex_pattern_argument_refuses_a_tainted_operand() {
    // `rule:security/regex-pattern-is-a-sink` over `rule:security/launderers-are-sink-named`. A pattern is a sink because an
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
    // qualifier on — `rule:security/taint-propagation`'s rule that a substring matched out of a
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
    // text authored. That is `rule:security/launderers-are-sink-named`'s sink-named launderer exactly, and
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
    // three callers before this pass was a fourth: `rule:types/duration-literal` puts it in
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

    // And what stays silent: the two spellings § 1 of `rule:types/duration-literal` opens with, the
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
    // `rule:expressions/preparation-preserves-behaviour` as a test: preparation produces an earlier answer and never
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

#[test]
fn a_tainted_value_at_a_query_text_parameter_is_a_diagnostic() {
    // `rule:security/sink-predicate` is the whole injection story for `rule:core-classes/db-one-api`, and this is its
    // first half: the statement text is the sink, so a `tainted` value cannot
    // reach it at all. There is no escaping function to reach for after the
    // refusal — § 1 says an escaper would be a second, weaker answer to a
    // question the bound parameters already answer.
    let queried = check_call(
        "    Core\\Db\\Connection $db = Core\\Db::connect(\"main\");\n    \
         tainted string $t = \"select id from t\" as tainted string;\n    \
         Core\\Db\\Rows<Core\\Db\\Row> $r = $db->query($t, []);\n",
    );
    assert!(
        reported(&queried, code::E_TYPE_MISMATCH),
        "a tainted statement text reached query: {queried:?}"
    );

    // `execute`'s row carries the same mark, because a rule stated on `query`
    // alone would leave the member that writes wide open.
    let executed = check_call(
        "    Core\\Db\\Connection $db = Core\\Db::connect(\"main\");\n    \
         tainted string $t = \"delete from t\" as tainted string;\n    \
         Core\\Db\\Write $w = $db->execute($t, []);\n",
    );
    assert!(
        reported(&executed, code::E_TYPE_MISMATCH),
        "a tainted statement text reached execute: {executed:?}"
    );
}

#[test]
fn the_same_tainted_value_at_a_bound_parameter_compiles() {
    // § 4's second half, and the one a refusal alone would get wrong: a bound
    // parameter accepts `tainted` **freely**. It is the same value at the same
    // call — the pair is what makes the rule a boundary rather than a ban, and
    // it is why the API needs no `escape`.
    let diags = check_call(
        "    Core\\Db\\Connection $db = Core\\Db::connect(\"main\");\n    \
         tainted string $t = \"ada\" as tainted string;\n    \
         Core\\Db\\Rows<Core\\Db\\Row> $r = \
         $db->query(\"select id from t where name = ?\", [$t]);\n",
    );
    assert!(
        !diags.has_errors(),
        "a tainted value was refused at a bound parameter: {diags:?}"
    );
}

#[test]
fn a_tainted_settings_host_is_a_diagnostic_naming_assert_trusted() {
    // `rule:core-classes/db-capabilities`'s other sink, and the one whose refusal is not the end of
    // the story: `Settings.host` refuses `tainted` and **has no launderer**,
    // because a malicious server answers any query with a `LOCAL INFILE`
    // request and no string check can establish that an address is safe to
    // send a credential to. The refusal itself is the ordinary `E0401` — the
    // key is declared a plain `string` and the value is not one.
    let refused = check_call(
        "    tainted string $h = \"db.example.test\" as tainted string;\n    \
         var $c = Core\\Db::open({driver: Core\\Db\\Driver::Postgres, host: $h, \
         database: \"shop\", user: \"app\", password: \"hunter2\"});\n",
    );
    assert!(
        reported(&refused, code::E_TYPE_MISMATCH),
        "a tainted host reached `open`'s settings: {refused:?}"
    );

    // And the half `check_shape_field` exists for. A sink with a launderer
    // leaves a reader somewhere to go and this one does not, so the diagnostic
    // names § 3's only way through, in the spelling that would compile.
    assert!(
        refused.iter().any(|diag| diag
            .notes
            .iter()
            .any(|note| note.contains("Core\\Taint::assertTrusted"))),
        "the refusal named no way through: {refused:?}"
    );

    // The pair that makes it a boundary rather than a ban, and § 3's own
    // sentence: `database` and `user` are length-prefixed protocol fields
    // rather than parsed text, so they accept `tainted` freely. Same value,
    // one key over.
    let accepted = check_call(
        "    tainted string $t = \"shop\" as tainted string;\n    \
         var $c = Core\\Db::open({driver: Core\\Db\\Driver::Postgres, \
         host: \"db.example.test\", database: $t, user: $t, \
         password: \"hunter2\"});\n",
    );
    assert!(
        !accepted.has_errors(),
        "a tainted value was refused at `database`: {accepted:?}"
    );
}

/// A connection to write a literal query against. `connect` is § 18's entry
/// point and its return type is what puts `Core\Db\Connection` on `$db`, which
/// is the class § 1's rows are matched nominally against.
fn query(sql: &str, params: &str) -> Diagnostics {
    check_call(&format!(
        "    Core\\Db\\Connection $db = Core\\Db::connect(\"main\");\n    \
         $db->query({sql}, {params});\n"
    ))
}

#[test]
fn a_placeholder_count_mismatch_on_a_literal_is_a_diagnostic() {
    // `rule:core-classes/db-literal-query-checking`'s first clause, in both directions. The refusal is the
    // rewriter's own — `nvs_stdlib::db::check_literal_query` runs the one the
    // request would have run — so this is the `LogicError` the first call would
    // have thrown, moved to `nvs check`.
    let short = query("\"select id from t where a = ? and b = ?\"", "[1]");
    assert!(
        reported(&short, code::E_FORMAT_TEMPLATE_MISMATCH),
        "two placeholders against one argument: {short:?}"
    );
    let long = query("\"select id from t where a = ?\"", "[1, 2]");
    assert!(
        reported(&long, code::E_FORMAT_TEMPLATE_MISMATCH),
        "one placeholder against two arguments: {long:?}"
    );

    // The shapes that must stay silent. A count that agrees is a call the
    // runtime accepts, and a params array this pass cannot read whole is § 2's
    // rule: nothing is refused for being dynamic.
    let fine = query("\"select id from t where a = ? and b = ?\"", "[1, 2]");
    assert!(
        !fine.has_errors(),
        "a query whose count agrees was refused: {fine:?}"
    );
    let dynamic = check_call(
        "    Core\\Db\\Connection $db = Core\\Db::connect(\"main\");\n    \
         array<mixed> $p = [1];\n    \
         $db->query(\"select id from t where a = ? and b = ?\", $p);\n",
    );
    assert!(
        !dynamic.has_errors(),
        "a computed params array was read anyway: {dynamic:?}"
    );

    // § 5's `??` escape, which is the one piece of syntax the rewriter adds to
    // SQL: a literal question mark is not a placeholder, so this binds one
    // argument and not two. It is here because it is exactly the shape a second
    // reader of the same grammar would get wrong.
    let escaped = query("\"select a ?? b from t where c = ?\"", "[1]");
    assert!(
        !escaped.has_errors(),
        "`??` was counted as a placeholder: {escaped:?}"
    );
}

#[test]
fn an_unterminated_literal_in_a_query_is_a_diagnostic() {
    // `rule:core-classes/db-literal-query-checking`'s "an unterminated string
    // literal". The rewriter refuses the same text when a request runs it, so
    // this is that `LogicError` moved to `nvs check`.
    let open = query("\"select id from t where a = 'x\"", "[]");
    assert!(
        reported(&open, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a literal with no closing quote: {open:?}"
    );

    // A fact about the text alone, so a params array this pass cannot read
    // whole does not buy silence the way § 2 buys it for the pairing.
    let dynamic = check_call(
        "    Core\\Db\\Connection $db = Core\\Db::connect(\"main\");\n    \
         array<mixed> $p = [1];\n    \
         $db->query(\"select id from t where a = 'x and b = ?\", $p);\n",
    );
    assert!(
        reported(&dynamic, code::E_INTRINSIC_LITERAL_MALFORMED),
        "an unterminated literal beside a computed params array: {dynamic:?}"
    );

    // And the shape this must not mistake for one: a closed literal holding
    // the bytes that open one, which is the whole reason the scan tracks
    // regions rather than counting quotes.
    let fine = query("\"select id from t where a = 'x?y' and b = ?\"", "[1]");
    assert!(
        !fine.has_errors(),
        "a closed literal was read as an open one: {fine:?}"
    );
}

#[test]
fn a_two_statement_literal_query_is_a_diagnostic() {
    // § 1's "every statement is prepared", read as the one question about
    // statement count that needs no vendor's grammar: a prepared statement is
    // one command on every backend, so a text holding two never runs.
    let two = query("\"update t set a = 1; drop table t\"", "[]");
    assert!(
        reported(&two, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a second statement was accepted: {two:?}"
    );

    // The *other* code, because this is a fact about the literal alone: it
    // holds however the params array reads, and is reported before the pairing
    // is looked at.
    let with_params = query("\"update t set a = ?; drop table t\"", "[1]");
    assert!(
        reported(&with_params, code::E_INTRINSIC_LITERAL_MALFORMED),
        "a second statement beside a params array that agrees: {with_params:?}"
    );

    // And the three spellings of a `;` that is not a separator — the ones a
    // scan that only looked for the byte would refuse. A terminator ends the
    // one statement, a comment after it is still that statement, and a `;`
    // inside a string literal is text.
    for sql in [
        "\"update t set a = 1;\"",
        "\"update t set a = 1; -- and nothing after it\"",
        "\"update t set a = ';'\"",
    ] {
        let single = query(sql, "[]");
        assert!(
            !single.has_errors(),
            "a `;` that separates nothing was refused: {sql} {single:?}"
        );
    }
}

#[test]
fn mixed_placeholder_styles_on_a_literal_are_a_diagnostic() {
    // § 5's two spellings, and § 10's "positional-vs-named consistency": one
    // statement uses one of them. Both directions, because the rewriter refuses
    // at whichever marker disagrees with the array it was handed.
    let named_in_a_list = query("\"select id from t where a = ? and b = :b\"", "[1, 2]");
    assert!(
        reported(&named_in_a_list, code::E_FORMAT_TEMPLATE_MISMATCH),
        "a `:name` against a list-keyed array: {named_in_a_list:?}"
    );
    let positional_in_names = query(
        "\"select id from t where a = :a and b = ?\"",
        "[\"a\" => 1, \"b\" => 2]",
    );
    assert!(
        reported(&positional_in_names, code::E_FORMAT_TEMPLATE_MISMATCH),
        "a `?` against a string-keyed array: {positional_in_names:?}"
    );

    // A name no argument binds is the same clause read the other way, and a
    // repeated `:name` is § 5's one value bound once — the thing the positional
    // form cannot express, so it must not be counted twice.
    let unbound = query(
        "\"select id from t where a = :a and b = :c\"",
        "[\"a\" => 1, \"b\" => 2]",
    );
    assert!(
        reported(&unbound, code::E_FORMAT_TEMPLATE_MISMATCH),
        "a `:name` naming no argument: {unbound:?}"
    );
    let repeated = query(
        "\"select id from t where a = :a or b = :a\"",
        "[\"a\" => 1]",
    );
    assert!(
        !repeated.has_errors(),
        "a `:name` used twice was counted twice: {repeated:?}"
    );

    // And PostgreSQL's `::` cast, which is why a bare `:` does not start a
    // name. The refusal has to hold on every dialect, and this one holds on
    // none.
    let cast = query("\"select a::text from t where b = ?\"", "[1]");
    assert!(
        !cast.has_errors(),
        "a `::` cast was read as a `:name`: {cast:?}"
    );
}

/// A deployment granting exactly `db.granted.test` under `db.open`, and
/// nothing else at all — `rule:core-classes/db-capabilities`'s block as the compiling machine reads
/// it.
fn granting(host: &str) -> Capabilities {
    Capabilities {
        db: Some(CapDb {
            connect: None,
            open: Some(Setting::List(vec![host.to_owned()])),
            schema: None,
        }),
        ..Capabilities::default()
    }
}

/// `Core\Db::open` with `host` written into its settings literal, checked
/// against `grants` — § 18's call, and the only one whose literal this pass
/// reads out of a *field* rather than out of an argument.
fn open(host: &str, grants: Option<&Capabilities>) -> Diagnostics {
    check_src_granted(
        &format!(
            "<?nvs\nclass Main {{\n  public static function main(): void {{\n    \
             var $c = Core\\Db::open({{driver: Core\\Db\\Driver::Postgres, host: {host}, \
             database: \"shop\", user: \"app\", password: \"hunter2\"}});\n  }}\n}}\n"
        ),
        grants,
    )
}

#[test]
fn an_open_host_matching_no_grant_is_a_diagnostic() {
    // `rule:core-classes/db-literal-query-checking`'s second sentence. The host is a literal and the grant is
    // this machine's, so both halves of `db.open`'s question are facts before
    // the program runs — and the answer is the one
    // `nvs_runtime::capability::require` would have given, moved earlier per
    // `rule:expressions/preparation-preserves-behaviour` rather than made stricter.
    let caps = granting("db.granted.test");
    let ungranted = open("\"db.example.test\"", Some(&caps));
    assert!(
        reported(&ungranted, code::E_UNGRANTED_HOST),
        "an ungranted literal host compiled: {ungranted:?}"
    );

    // The pair that makes it a boundary rather than a ban, and the reason the
    // grant list is walked instead of merely being present: the same call,
    // one host over, is fine.
    let granted = open("\"db.granted.test\"", Some(&caps));
    assert!(
        !granted.has_errors(),
        "a granted literal host was refused: {granted:?}"
    );
}

#[test]
fn an_open_host_says_nothing_where_a_half_of_the_question_is_missing() {
    // Two ways for § 10's question to be unanswerable at check time, and both
    // have to leave the call alone rather than deny it — a refusal here is one
    // the runtime would not have made, which is what `rule:expressions/preparation-preserves-behaviour` forbids.
    //
    // First: no configuration was read at all, which is every other fixture in
    // this file and every `nvs check` outside a project root. Absent is not
    // empty; see `nvs_types::check_program_granted`.
    let unconfigured = open("\"db.example.test\"", None);
    assert!(
        !unconfigured.has_errors(),
        "an unconfigured check denied a host: {unconfigured:?}"
    );

    // Second: the host is not a literal. § 2's rule, unchanged by the field
    // address — nothing is refused for being dynamic, and this one is decided
    // at the door with the value in hand.
    let caps = granting("db.granted.test");
    let computed = check_src_granted(
        "<?nvs\nclass Main {\n  public static function main(): void {\n    \
         string $h = Core\\Str::lower(\"DB.EXAMPLE.TEST\");\n    \
         var $c = Core\\Db::open({driver: Core\\Db\\Driver::Postgres, host: $h, \
         database: \"shop\", user: \"app\", password: \"hunter2\"});\n  }\n}\n",
        Some(&caps),
    );
    assert!(
        !computed.has_errors(),
        "a computed host was refused while checking: {computed:?}"
    );
}

/// A deployment granting exactly one queue under `queue.purge`, and nothing
/// else at all — `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s
/// block as the compiling machine reads it, which is
/// [`granting`]'s fixture one capability over.
fn sweeping(queue: &str) -> Capabilities {
    Capabilities {
        queue: Some(CapQueue {
            purge: Some(Setting::List(vec![queue.to_owned()])),
        }),
        ..Capabilities::default()
    }
}

/// `Core\Queue::purge` with `queue` written as its subject, checked against
/// `grants` — [`open`]'s call one member over, and the simpler address of the
/// two: the name is the argument itself rather than a field inside a shape.
fn purge(queue: &str, grants: Option<&Capabilities>) -> Diagnostics {
    check_src_granted(
        &format!(
            "<?nvs\nclass Main {{\n  public static function main(): void {{\n    \
             uint $swept = Core\\Queue::purge({queue}, {{limit: 100}});\n  }}\n}}\n"
        ),
        grants,
    )
}

#[test]
fn a_written_purge_queue_name_outside_the_grant_is_e0637() {
    // `rule:concurrency/queue-deletion-is-explicit-and-bounded`'s grant, read
    // before the program runs. Both halves are facts here — the name is a
    // literal and the grant is this machine's — so the answer is the one
    // `nvs_runtime::capability::require` would have given at the door, moved
    // earlier per `rule:expressions/preparation-preserves-behaviour` rather
    // than made stricter.
    let caps = sweeping("retention");
    let ungranted = purge("\"email\"", Some(&caps));
    assert!(
        reported(&ungranted, code::E_UNGRANTED_QUEUE),
        "an ungranted literal queue compiled: {ungranted:?}"
    );

    // The pair that makes it a boundary rather than a ban, and the reason the
    // list is walked instead of merely being present: the granted queue, in
    // the same call, is fine.
    let granted = purge("\"retention\"", Some(&caps));
    assert!(
        !granted.has_errors(),
        "a granted literal queue was refused: {granted:?}"
    );
}

#[test]
fn a_computed_purge_queue_name_is_not_refused_before_it_runs() {
    // Nothing is refused for being dynamic. A name this pass cannot fold is
    // decided at the door with the value in hand, which is the same refusal
    // one moment later and never a different one.
    let caps = sweeping("retention");
    let computed = check_src_granted(
        "<?nvs\nclass Main {\n  public static function main(): void {\n    \
         string $q = Core\\Str::lower(\"EMAIL\");\n    \
         uint $swept = Core\\Queue::purge($q, {limit: 100});\n  }\n}\n",
        Some(&caps),
    );
    assert!(
        !computed.has_errors(),
        "a computed queue name was refused while checking: {computed:?}"
    );
}

#[test]
fn e0637_is_not_asked_when_the_checking_machine_read_no_configuration() {
    // An `nvs check` outside a project root read no grants at all, and a
    // refusal there would be one no deployment made — see
    // `nvs_types::check_program_granted`.
    let unconfigured = purge("\"email\"", None);
    assert!(
        !unconfigured.has_errors(),
        "an unconfigured check denied a queue: {unconfigured:?}"
    );

    // Absent is not empty, which is the other side of that bound: a
    // configuration this machine did read and that grants no queue at all is
    // the denial `rule:security/capability-roster-is-closed`'s deny-by-default
    // reading gives it.
    let nothing = Capabilities::default();
    let denied = purge("\"email\"", Some(&nothing));
    assert!(
        reported(&denied, code::E_UNGRANTED_QUEUE),
        "an absent `queue` block granted a queue: {denied:?}"
    );
}

#[test]
fn delete_has_no_static_half_and_is_refused_at_the_door() {
    // `delete`'s queue arrives inside the `Queue\Id` a receipt carries, so
    // there is no written name for this pass to read however literal the rest
    // of the call is — the grant is asked about at the door, where the value
    // is in hand.
    let caps = sweeping("retention");
    let deleting = check_src_granted(
        "<?nvs\nclass Main {\n  public static function main(): void {\n    \
         var $receipt = Core\\Queue::push(\"jobs/send-receipt.nvs\", {queue: \"email\"});\n    \
         bool $gone = Core\\Queue::delete($receipt);\n  }\n}\n",
        Some(&caps),
    );
    assert!(
        !deleting.has_errors(),
        "a `delete` was refused while checking: {deleting:?}"
    );

    // And the grant those same `caps` hold is live, so the silence above is
    // `delete`'s shape rather than a fixture that grants everything: the
    // queue that call names is one a written `purge` is refused for.
    let swept = purge("\"email\"", Some(&caps));
    assert!(
        reported(&swept, code::E_UNGRANTED_QUEUE),
        "the same grant let a written `purge` through: {swept:?}"
    );
}

#[test]
fn a_literal_metrics_name_outside_the_grammar_is_a_compile_error() {
    // `rule:observability/metrics-three-members`'s `[a-z][a-z0-9_]*`, read out
    // of the literal that wrote it. Asked of all three verbs, because the
    // grammar belongs to the series: a row missing from § 1's table leaves one
    // verb accepting the name the other two refuse, which is invisible from any
    // single line. Each name is refused for a different clause — a capital, a
    // separator the grammar has no place for, and a leading `_`.
    for call in [
        r#"Core\Metrics::increment("HttpRequests");"#,
        r#"Core\Metrics::observe("http.request.seconds", 0.5);"#,
        r#"Core\Metrics::gauge("_queue_depth", 3.0);"#,
    ] {
        let diags = check_call(&format!("    {call}\n"));
        assert!(
            reported(&diags, code::E_INTRINSIC_LITERAL_MALFORMED),
            "a name outside the grammar compiled: {call} {diags:?}"
        );
    }

    // The other side of the bound, and what makes this a grammar rather than a
    // ban on punctuation: `_` and a digit are admitted everywhere after the
    // first character.
    let admitted = check_call(
        "    Core\\Metrics::increment(\"http_requests_total\");\n    \
         Core\\Metrics::observe(\"db_query_seconds\", 0.5);\n    \
         Core\\Metrics::gauge(\"queue_depth_v2\", 3.0);\n",
    );
    assert!(
        !admitted.has_errors(),
        "a name the grammar admits was refused: {admitted:?}"
    );

    // Nothing is refused for being dynamic — and here that is not merely this
    // pass declining to guess: the registry fixes whatever it is handed, so a
    // computed name is never refused at all. `nvs_types::intrinsics`' fourth
    // bullet owns why.
    let computed = check_call(
        "    string $name = Core\\Str::upper(\"http_requests\");\n    \
         Core\\Metrics::increment($name);\n",
    );
    assert!(
        !computed.has_errors(),
        "a computed metric name was refused while checking: {computed:?}"
    );
}
