//! A partial check records, for each file it walks, the declarations the file
//! looked up and the names it failed to find (`nvs_hir::uses`). A whole check
//! records nothing.

mod common;

use common::Program;
use nvs_types::CheckCache;
use nvs_types::ty::TypeInterner;

/// A library with a base class, a subclass, an enum and a constant.
const LIB: &str = "<?nvs\nnamespace Framework;\n\
    enum Status: int { case Draft = 1; case Live = 2; }\n\
    class Base {\n  public int $count = 0;\n  \
    public static function greet(): string { return \"hi\"; }\n}\n\
    class Child extends Base {\n  public const LIMIT = 3;\n}\n";

/// Checks `main` beside the library, walking `main.nvs` alone, and returns the
/// sorted names it used and missed.
fn uses_of(name: &str, main: &str) -> (Vec<String>, Vec<String>) {
    let mut program = Program::new(name, &[("main.nvs", main), ("lib.nvs", LIB)]);
    let mut interner = TypeInterner::new();
    let mut cache = CheckCache::new();
    let checked = program.check("main.nvs", &mut interner, Some((&["main.nvs"], &mut cache)));
    let output = cache
        .get(checked.id("main.nvs"))
        .expect("main.nvs was walked");
    let owned = |names: Vec<&str>| names.into_iter().map(str::to_owned).collect();
    (
        owned(output.uses().used_sorted()),
        owned(output.uses().missed_sorted()),
    )
}

fn main_with(body: &str) -> String {
    format!(
        "<?nvs\nrequire 'lib.nvs';\nuse Framework\\Base;\nuse Framework\\Child;\nuse Framework\\Status;\n{body}\n"
    )
}

#[test]
fn a_resolved_static_call_is_a_use_of_its_method() {
    let (used, _) = uses_of("uses-call", &main_with("echo Base::greet();"));
    assert!(
        used.contains(&"Framework\\Base::greet".to_owned()),
        "{used:?}"
    );
    assert!(used.contains(&"Framework\\Base".to_owned()), "{used:?}");
}

#[test]
fn an_unknown_class_is_a_miss() {
    let (_, missed) = uses_of(
        "uses-unknown-class",
        &main_with("var $m = new Framework\\Missing();"),
    );
    assert!(
        missed.contains(&"Framework\\Missing".to_owned()),
        "{missed:?}"
    );
}

#[test]
fn an_unknown_member_on_a_known_class_is_a_miss_on_that_class() {
    let (_, missed) = uses_of("uses-unknown-member", &main_with("echo Base::nope();"));
    assert!(
        missed.contains(&"Framework\\Base::nope".to_owned()),
        "{missed:?}"
    );
}

#[test]
fn a_member_found_on_a_parent_is_a_use_of_the_parent_and_a_miss_on_the_child() {
    let (used, missed) = uses_of("uses-inherited", &main_with("echo Child::greet();"));
    assert!(
        used.contains(&"Framework\\Base::greet".to_owned()),
        "{used:?}"
    );
    assert!(
        missed.contains(&"Framework\\Child::greet".to_owned()),
        "{missed:?}"
    );
}

#[test]
fn a_type_annotation_is_a_use_of_the_class() {
    let main = main_with("function show(Child $c): int { return $c->count; }");
    let main = main.replace(
        "function show",
        "class Page {\n  public static function show",
    ) + "}\n";
    let (used, _) = uses_of("uses-annotation", &main);
    assert!(used.contains(&"Framework\\Child".to_owned()), "{used:?}");
    assert!(
        used.contains(&"Framework\\Base::$count".to_owned()),
        "{used:?}"
    );
}

#[test]
fn an_enum_case_is_a_use_and_an_unknown_case_is_a_miss() {
    let (used, missed) = uses_of(
        "uses-case",
        &main_with("echo Status::Draft->value;\necho Status::Gone->value;"),
    );
    assert!(
        used.contains(&"Framework\\Status::Draft".to_owned()),
        "{used:?}"
    );
    assert!(
        missed.contains(&"Framework\\Status::Gone".to_owned()),
        "{missed:?}"
    );
}

#[test]
fn every_file_a_partial_check_walks_records() {
    let mut program = Program::new(
        "uses-every",
        &[
            ("main.nvs", &main_with("echo Base::greet();")),
            ("lib.nvs", LIB),
        ],
    );
    let mut interner = TypeInterner::new();
    let mut cache = CheckCache::new();
    let checked = program.check(
        "main.nvs",
        &mut interner,
        Some((&["main.nvs", "lib.nvs"], &mut cache)),
    );
    for file in ["main.nvs", "lib.nvs"] {
        assert!(
            cache
                .get(checked.id(file))
                .is_some_and(|output| output.uses().is_on()),
            "{file}"
        );
    }
    // Recording changes nothing a check reports.
    let whole = program.check("main.nvs", &mut interner, None);
    assert_eq!(whole.diags, checked.diags);
    assert_eq!(whole.table, checked.table);
}
