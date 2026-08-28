//! ADR 0079 § 1's `#[Test]` attribute: the payload check, and the table the
//! compiler builds from it.
//!
//! The table has no `.nvst` case and cannot have one yet: nothing runs a test
//! until the runner arrives, so a program can observe neither the rows nor
//! their order. It is asserted here instead, where the table itself is in
//! hand — the payload's *refusals* are pinned by a `.nvst` alongside, since
//! those a program does observe.

mod common;

use common::check_src_table;
use nvs_types::defaults::ConstArg;

/// The one shape ADR 0079 § 1's own example writes, with the `use Core\Test;`
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
        "  #[Test(skip: \"blocked on Core\\\\Db, M8\", seed: 7, server: true, retries: 2)]\n\
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
        ]
    );
}

#[test]
fn a_bare_test_carries_no_options_rather_than_defaulted_ones() {
    let (_, options) = tests_of("  #[Test]\n  public function m(): void {}\n");
    assert!(options[0].is_empty());
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
