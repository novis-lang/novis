//! `rule:testing/test-attribute`'s `#[Test]` attribute and § 8's `#[Fixture]`: the payload
//! check, and the two tables the compiler builds from them.
//!
//! Neither table has a `.nvst` case for its *rows*: the `#[Test]` one is
//! reported by the runner, but a `#[Fixture]` row is observable only once §
//! 8's injection runs, so nothing a program can print says which type a
//! fixture supplies. They are asserted here instead, where the tables
//! themselves are in hand — both rosters' *refusals* are pinned by a `.nvst`
//! alongside, since those a program does observe.
//!
//! `rule:testing/doubles`' structural check is asserted here for the opposite
//! reason: its two refusals reject a program, so what a `.nvst` case can pin is
//! one of them at a time against a frozen message, while the pair of shapes
//! that must *not* be refused — a complete double, and a partial that leaves a
//! method to the real implementation — is a question about the checker rather
//! than about anything a program prints.

mod common;

use common::{check_src, check_src_declared, check_src_table};
use nvs_diagnostics::code;
use nvs_types::defaults::ConstArg;
use nvs_types::testing::Injection;

/// The one shape `rule:testing/test-attribute`'s own example writes, with the `use Core\Test;`
/// that places both the attribute and the assertions the body calls.
fn tests_of(members: &str) -> (Vec<String>, Vec<Vec<(String, ConstArg)>>) {
    let (diags, exprs) = check_src_table(&format!(
        "<?nvs\nuse Core\\Test;\nclass UserTest {{\n{members}\n}}\n"
    ));
    assert!(!diags.has_errors(), "fixture was refused: {diags:?}");
    let cases = exprs.tests("UserTest").unwrap_or_default();
    (
        cases.iter().map(|c| c.method.clone()).collect(),
        cases.iter().map(|c| c.options.clone()).collect(),
    )
}

#[test]
fn a_test_attribute_builds_a_table_of_its_cases() {
    let (methods, _) = tests_of(
        "  #[Test]\n  public function bIsSecond(): void {}\n\
         \n  public function notATest(): void {}\n\
         \n  #[Test]\n  public function aIsFirst(): void {}\n",
    );
    // Declaration order, not alphabetical and not "the ones with attributes
    // first": a runner reports in the order the class declares.
    assert_eq!(methods, ["bIsSecond", "aIsFirst"]);
}

#[test]
fn a_class_with_no_test_method_has_no_row_at_all() {
    let (diags, exprs) =
        check_src_table("<?nvs\nclass Plain {\n  public function m(): void {}\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(exprs.tests("Plain").is_none());
    assert!(exprs.test_classes().is_empty());
}

#[test]
fn each_option_is_folded_to_the_constant_a_parameter_default_folds_to() {
    let (_, options) = tests_of(
        // § 20's `because:` rides along because a `retries:` without one is
        // refused, which is `check_retries_state_a_reason`'s own rule and is
        // pinned by the `.nvst` alongside.
        "  #[Test(skip: \"blocked on Core\\\\Db, M8\", seed: 7, server: true, retries: 2, \
         because: \"real DNS\")]\n\
         \n  public function itPersists(): void {}\n",
    );
    assert_eq!(
        options[0],
        vec![
            (
                "skip".to_owned(),
                ConstArg::Str("blocked on Core\\Db, M8".to_owned())
            ),
            ("seed".to_owned(), ConstArg::Int(7)),
            ("server".to_owned(), ConstArg::Bool(true)),
            ("retries".to_owned(), ConstArg::Int(2)),
            ("because".to_owned(), ConstArg::Str("real DNS".to_owned())),
        ]
    );
}

#[test]
fn a_bare_test_carries_no_options_rather_than_defaulted_ones() {
    let (_, options) = tests_of("  #[Test]\n  public function m(): void {}\n");
    assert!(options[0].is_empty());
}

/// `rule:testing/fixtures`'s own worked example's shape, with the two imports that place
/// the marker and the assertions, and a class for each fixture to build.
fn fixture_src(members: &str) -> String {
    format!(
        "<?nvs\nuse Core\\Test;\nuse Core\\Test\\Fixture;\nuse Core\\Test\\TestWith;\n\
         class Schema {{}}\nclass Widget {{}}\n\
         class RepoTest {{\n{members}\n}}\n"
    )
}

#[test]
fn a_fixture_attribute_builds_a_roster_keyed_by_what_it_returns() {
    let (diags, declared) = check_src_declared(&fixture_src(
        "  #[Fixture]\n  public static function schema(): Schema { return new Schema(); }\n\
         \n  public static function notAFixture(): Widget { return new Widget(); }\n\
         \n  #[Fixture]\n  public static function widget(): Widget { return new Widget(); }\n\
         \n  #[Test]\n  public function itFinds(): void {}\n",
    ));
    assert!(!diags.has_errors(), "fixture was refused: {diags:?}");
    let fixtures = declared.exprs().fixtures("RepoTest").unwrap();
    // Declaration order, and only the marked members — the roster is what
    // § 8 injects from, not every static method that returns something.
    let rows: Vec<(&str, String)> = fixtures
        .iter()
        .map(|f| (f.method.as_str(), declared.interner.describe(f.ty)))
        .collect();
    assert_eq!(
        rows,
        [
            ("schema", "Schema".to_owned()),
            ("widget", "Widget".to_owned())
        ]
    );
    // The two tables are separate: a class declaring both keeps a `#[Test]`
    // row out of the roster a parameter is resolved against.
    assert_eq!(
        declared.exprs().tests("RepoTest").map(|cases| cases.len()),
        Some(1)
    );
}

#[test]
fn a_parameter_is_resolved_to_the_fixture_supplying_its_type_in_parameter_order() {
    // `rule:testing/fixtures` resolves by type, and the row records an *order* — which
    // fixture answers which parameter — so that the runner reads one rather
    // than re-deriving it. The test is written **above** the fixtures it takes
    // on purpose: resolution is a pass over the whole class, not a step in the
    // walk that collects it.
    let (diags, declared) = check_src_declared(&fixture_src(
        "  #[Test]\n  public function itTakesBoth(Widget $w, Schema $s): void {}\n\
         \n  #[Fixture]\n  public static function schema(): Schema { return new Schema(); }\n\
         \n  #[Fixture]\n  public static function widget(Schema $s): Widget { return new Widget(); }\n",
    ));
    assert!(!diags.has_errors(), "resolution was refused: {diags:?}");
    let cases = declared.exprs().tests("RepoTest").unwrap();
    assert_eq!(
        cases[0].params,
        [
            Injection::Fixture("widget".to_owned()),
            Injection::Fixture("schema".to_owned())
        ]
    );
    // No `#[TestWith]` anywhere, so the case is the one call § 1 describes.
    assert!(cases[0].rows.is_empty());
    // § 8's last sentence: a fixture's own parameters resolve the same way,
    // which is what gives the runner a build order.
    let fixtures = declared.exprs().fixtures("RepoTest").unwrap();
    assert!(fixtures[0].fixtures.is_empty());
    assert_eq!(fixtures[1].fixtures, ["schema".to_owned()]);
}

#[test]
fn each_test_with_is_a_row_folded_in_parameter_order() {
    // `rule:testing/data-rows`'s own worked example, plus the fact no line of it states:
    // a row is folded into **parameter** order rather than into the order its
    // fields happen to be written, because that is the order the call is made
    // in. The second row writes `want` first for exactly that reason.
    let (diags, exprs) = check_src_table(
        "<?nvs\nuse Core\\Test;\nuse Core\\Test\\TestWith;\n\
         class UserTest {\n\
         \x20 #[Test]\n\
         \x20 #[TestWith(input: \"  ada \", want: \"ada\")]\n\
         \x20 #[TestWith(want: \"ada\", input: \"ADA\")]\n\
         \x20 public function itNormalizes(string $input, string $want): void {}\n}\n",
    );
    assert!(!diags.has_errors(), "rows were refused: {diags:?}");
    let cases = exprs.tests("UserTest").unwrap();
    assert_eq!(cases[0].params, [Injection::Row, Injection::Row]);
    assert_eq!(
        cases[0].rows,
        [
            [
                Some(ConstArg::Str("  ada ".to_owned())),
                Some(ConstArg::Str("ada".to_owned()))
            ],
            [
                Some(ConstArg::Str("ADA".to_owned())),
                Some(ConstArg::Str("ada".to_owned()))
            ]
        ]
    );
}

#[test]
fn a_row_and_a_fixture_fill_one_parameter_list_between_them() {
    // § 9's "a mix of the two in one method is allowed, and each parameter's
    // source is unambiguous because fixtures resolve by type and rows by
    // name". The row's own position holds its value and the fixture's holds
    // `None`, so the runner walks one list rather than joining two.
    let (diags, declared) = check_src_declared(&fixture_src(
        "  #[Test]\n  #[TestWith(n: 1)]\n  #[TestWith(n: 2)]\n\
         \x20 public function itCounts(Schema $s, int $n): void {}\n\
         \n  #[Fixture]\n  public static function schema(): Schema { return new Schema(); }\n",
    ));
    assert!(!diags.has_errors(), "the mix was refused: {diags:?}");
    let cases = declared.exprs().tests("RepoTest").unwrap();
    assert_eq!(
        cases[0].params,
        [Injection::Fixture("schema".to_owned()), Injection::Row]
    );
    assert_eq!(
        cases[0].rows,
        [
            [None, Some(ConstArg::Int(1))],
            [None, Some(ConstArg::Int(2))]
        ]
    );
}

#[test]
fn a_row_wins_a_parameter_a_fixture_would_also_have_answered() {
    // The two rosters overlap here on purpose: `$n` is an `int`, which the
    // fixture supplies, *and* is named by the row. A name is the more specific
    // of the two — the row was written against this method, while the fixture
    // answers every method of the class — so the row wins, which is § 9's
    // "each parameter's source is unambiguous".
    let (diags, declared) = check_src_declared(&fixture_src(
        "  #[Test]\n  #[TestWith(n: 1)]\n\
         \x20 public function itCounts(int $n): void {}\n\
         \n  #[Fixture]\n  public static function count(): int { return 3; }\n",
    ));
    assert!(!diags.has_errors(), "the overlap was refused: {diags:?}");
    let cases = declared.exprs().tests("RepoTest").unwrap();
    assert_eq!(cases[0].params, [Injection::Row]);
}

#[test]
fn a_class_with_no_fixture_has_no_roster_at_all() {
    let (diags, declared) = check_src_declared(&fixture_src(
        "  #[Test]\n  public function itFinds(): void {}\n",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(declared.exprs().fixtures("RepoTest").is_none());
}

#[test]
fn a_fixture_is_matched_nominally_exactly_as_a_test_is() {
    // Fully qualified needs no import at all, and is the same attribute the
    // `use`d spelling above names.
    let (diags, declared) = check_src_declared(
        "<?nvs\nclass Schema {}\nclass T {\n  #[Core\\Test\\Fixture]\n  \
         public static function schema(): Schema { return new Schema(); }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    assert_eq!(declared.exprs().fixtures("T").map(<[_]>::len), Some(1));

    // A userland `Fixture` with nothing importing the `Core` one resolves to
    // a different name, so it builds no row — and is refused as the
    // undeclared name it is rather than silently marking the method.
    let (diags, declared) = check_src_declared(
        "<?nvs\nclass Schema {}\nclass T {\n  #[Fixture]\n  \
         public static function schema(): Schema { return new Schema(); }\n}\n",
    );
    assert!(diags.has_errors());
    assert!(declared.exprs().fixtures("T").is_none());
}

#[test]
fn the_match_is_nominal_so_the_name_has_to_resolve_to_core_test() {
    // Fully qualified needs no import at all.
    let (diags, exprs) =
        check_src_table("<?nvs\nclass T {\n  #[Core\\Test]\n  public function m(): void {}\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
    assert_eq!(exprs.tests("T").map(<[_]>::len), Some(1));

    // Bare, with nothing importing it, resolves to `\Test` — which is no
    // declaration at all, so it is the ordinary undeclared-name refusal
    // rather than a silently ignored attribute.
    let (diags, exprs) =
        check_src_table("<?nvs\nclass T {\n  #[Test]\n  public function m(): void {}\n}\n");
    assert!(diags.has_errors());
    assert!(exprs.tests("T").is_none());
}

// ------------------------------------------------------------------
// `rule:testing/doubles` -- the structural check between a shape of
// closures and the interface the call site wrote.
// ------------------------------------------------------------------

/// ADR 0079 § 10's worked example, at the checker: the shape answers every
/// method `Clock` declares, so the call is accepted and its `T` is the
/// interface — which is what lets the double be passed where one is taken.
#[test]
fn a_double_is_the_interface_it_names() {
    let diags = check_src(
        "<?nvs\ninterface Clock {\n  public function now(): int;\n}\n\
         final class Session {\n  private Clock $clock;\n  \
         public function constructor(Clock $clock) { $this->clock = $clock; }\n}\n\
         Session $s = new Session(Core\\Test::double<Clock>({ now: fn(): int => 1 }));\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The first of § 10's two refusals. `zone` is declared and unanswered, and
/// there is nothing the double could return for it — `rule:testing/doubles` is
/// strict for that reason rather than by preference.
#[test]
fn a_double_missing_a_method_is_refused() {
    let diags = check_src(
        "<?nvs\ninterface Clock {\n  public function now(): int;\n  \
         public function zone(): string;\n}\n\
         Clock $c = Core\\Test::double<Clock>({ now: fn(): int => 1 });\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DOUBLE_METHOD_MISSING)),
        "{diags:?}"
    );
}

/// The second, and the one that catches a renamed member: `tomorrow` stands in
/// for nothing `Clock` declares, so the double would otherwise go on answering
/// a contract no call site asks for.
#[test]
fn a_double_declaring_a_method_the_interface_lacks_is_refused() {
    let diags = check_src(
        "<?nvs\ninterface Clock {\n  public function now(): int;\n}\n\
         Clock $c = Core\\Test::double<Clock>({ now: fn(): int => 1, \
         tomorrow: fn(): int => 2 });\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DOUBLE_METHOD_UNKNOWN)),
        "{diags:?}"
    );
}

/// The shape a `double` is refused for is a complete `partial`: `zone` is left
/// to the real implementation, which is the whole of what a partial is. The
/// field half of the check still applies to one, which the second assertion is
/// — overriding a method the interface does not declare overrides nothing.
#[test]
fn a_partial_delegates_what_it_does_not_override() {
    let src = "<?nvs\ninterface Clock {\n  public function now(): int;\n  \
               public function zone(): string;\n}\n\
               final class SystemClock implements Clock {\n  \
               public function now(): int { return 1; }\n  \
               public function zone(): string { return \"utc\"; }\n}\n";
    let diags = check_src(&format!(
        "{src}Clock $c = Core\\Test::partial<Clock>(new SystemClock(), \
         {{ now: fn(): int => 2 }});\n"
    ));
    assert!(!diags.has_errors(), "{diags:?}");

    let diags = check_src(&format!(
        "{src}Clock $c = Core\\Test::partial<Clock>(new SystemClock(), \
         {{ tomorrow: fn(): int => 2 }});\n"
    ));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DOUBLE_METHOD_UNKNOWN)),
        "{diags:?}"
    );
}

/// The third refusal, and the division of labour behind it: a name that
/// resolved to a class is this walk's own `E0827`, while a type argument that
/// named no declaration at all is the written-class read's `E0447` — one
/// mistake, one diagnostic, whichever half of the question it fails.
#[test]
fn a_double_of_anything_but_an_interface_is_refused() {
    let class = "<?nvs\nfinal class SystemClock {\n  \
                 public function now(): int { return 1; }\n}\n";
    let diags = check_src(&format!(
        "{class}object $c = Core\\Test::double<SystemClock>({{ now: fn(): int => 1 }});\n"
    ));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DOUBLE_TYPE_ARG_NOT_AN_INTERFACE)),
        "{diags:?}"
    );

    // The same call written with a `partial`, whose `$real` would have been the
    // one thing that could answer the rest: it is refused for the same reason,
    // since what the shape stands in for is still not a contract.
    let diags = check_src(&format!(
        "{class}object $c = Core\\Test::partial<SystemClock>(new SystemClock(), \
         {{ now: fn(): int => 1 }});\n"
    ));
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_DOUBLE_TYPE_ARG_NOT_AN_INTERFACE)),
        "{diags:?}"
    );

    // Not a declaration at all, which the written-class read already refuses —
    // and this walk deliberately adds nothing to.
    let diags = check_src("<?nvs\nobject $c = Core\\Test::double<int>({ now: fn(): int => 1 });\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_ARG_NOT_A_CLASS)),
        "{diags:?}"
    );
    assert!(
        !diags
            .iter()
            .any(|d| d.code == Some(code::E_DOUBLE_TYPE_ARG_NOT_AN_INTERFACE)),
        "{diags:?}"
    );
}
